// ── Swarm Setup ──
//
// This file builds and starts the P2P node ("swarm" in libp2p terms).
//
// What happens when you call `create_swarm()`:
// 1. Load our keypair from disk (or generate a new one on first run)
// 2. Build all protocol behaviours (Kademlia, Identify, Ping, etc.)
// 3. Configure transport: TCP + Noise encryption + Yamux multiplexing
// 4. Create the swarm (combines transport + behaviour)
// 5. Start listening on a TCP port
//
// After this, you get a running Swarm that can connect to peers.

use libp2p::{
    identity, kad, identify, ping, relay,
    request_response, Multiaddr, StreamProtocol, Swarm,
};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tracing::info;

use super::behaviour::ChimeraBehaviour;
use super::config::NodeConfig;
use crate::network::protocol;

/// Build and start a Chimera P2P swarm.
///
/// Returns the running Swarm — ready to connect to peers and handle requests.
pub fn create_swarm(config: &NodeConfig) -> Result<Swarm<ChimeraBehaviour>, String> {
    // Step 1: Load or generate our identity (Ed25519 keypair)
    let keypair = load_or_generate_keypair(config)?;
    let local_peer_id = keypair.public().to_peer_id();
    info!("Local PeerId: {}", local_peer_id);

    // Step 2: Build the swarm
    // libp2p::SwarmBuilder does a lot for us:
    //   - Sets up TCP transport with Noise encryption
    //   - Adds Yamux multiplexing (multiple streams over one connection)
    //   - Wires up the behaviour
    //   - Uses Tokio as the async runtime
    let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .map_err(|e| format!("TCP setup failed: {}", e))?
        .with_relay_client(
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )
        .map_err(|e| format!("Relay client setup failed: {}", e))?
        .with_behaviour(|keypair, relay_client| {
            // Build all our protocol behaviours
            build_behaviour(keypair, relay_client, local_peer_id)
        })
        .map_err(|e| format!("Behaviour setup failed: {}", e))?
        .with_swarm_config(|cfg| {
            // Keep connections alive for a long time. The relay connection is critical —
            // if it drops, we're unreachable by other peers behind NAT.
            // 10 minutes gives plenty of time for peers to discover each other.
            cfg.with_idle_connection_timeout(Duration::from_secs(600))
        })
        .build();

    Ok(swarm)
}

/// Start the swarm listening and connect to bootstrap nodes.
///
/// Call this after create_swarm() to actually go online.
pub fn start_listening(
    swarm: &mut Swarm<ChimeraBehaviour>,
    config: &NodeConfig,
) -> Result<(), String> {
    // Listen on TCP (all interfaces, configured port)
    let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", config.tcp_port)
        .parse()
        .map_err(|e| format!("Bad listen address: {}", e))?;

    swarm
        .listen_on(listen_addr)
        .map_err(|e| format!("Failed to listen: {}", e))?;

    info!("Listening on TCP port {}", config.tcp_port);

    // Connect to bootstrap nodes (like our relay server)
    for addr_str in &config.bootstrap_nodes {
        match addr_str.parse::<Multiaddr>() {
            Ok(addr) => {
                info!("Dialing bootstrap node: {}", addr);
                if let Err(e) = swarm.dial(addr.clone()) {
                    tracing::warn!("Failed to dial {}: {}", addr, e);
                }

                // Listen through this node as a relay so peers behind NAT can reach us.
                // This makes a "relay reservation" — the relay server will forward
                // incoming connections to us. Without this, other peers can find our
                // PeerId in the DHT but can't actually connect to send us requests.
                let relay_addr: Multiaddr = format!("{}/p2p-circuit", addr)
                    .parse()
                    .expect("Valid relay circuit address");
                if let Err(e) = swarm.listen_on(relay_addr.clone()) {
                    tracing::warn!("Failed to listen on relay circuit {}: {}", relay_addr, e);
                } else {
                    info!("Listening via relay circuit: {}", relay_addr);
                }
            }
            Err(e) => {
                tracing::warn!("Invalid bootstrap address '{}': {}", addr_str, e);
            }
        }
    }

    // Tell Kademlia to bootstrap — this queries for our own PeerId
    // to find nearby peers and populate our routing table.
    if let Err(e) = swarm.behaviour_mut().kademlia.bootstrap() {
        // This fails if we have no known peers yet — that's OK on first run
        tracing::debug!("Kademlia bootstrap not started (no known peers): {}", e);
    }

    Ok(())
}

// ── Internal helpers ──

/// Build all the protocol behaviours for our node.
fn build_behaviour(
    keypair: &identity::Keypair,
    relay_client: relay::client::Behaviour,
    local_peer_id: libp2p::PeerId,
) -> ChimeraBehaviour {
    // Kademlia — the DHT for finding peers and content
    let kademlia = {
        let store = kad::store::MemoryStore::new(local_peer_id);
        let mut config = kad::Config::new(StreamProtocol::new("/chimera/kad/1.0.0"));
        // How often Kademlia republishes records to keep them alive
        config.set_replication_factor(std::num::NonZero::new(3).unwrap());
        kad::Behaviour::with_config(local_peer_id, store, config)
    };

    // Identify — peers exchange info automatically on connect
    let identify = identify::Behaviour::new(identify::Config::new(
        "/chimera/id/1.0.0".to_string(),
        keypair.public(),
    ));

    // Ping — automatic liveness checks
    let ping = ping::Behaviour::new(ping::Config::new());

    // Chunk transfer protocol — request-response pattern
    let chunk_proto = request_response::cbor::Behaviour::new(
        [(protocol::chunk_protocol(), request_response::ProtocolSupport::Full)],
        request_response::Config::default(),
    );

    // DAG node transfer protocol — same pattern
    let dag_proto = request_response::cbor::Behaviour::new(
        [(protocol::dag_protocol(), request_response::ProtocolSupport::Full)],
        request_response::Config::default(),
    );

    ChimeraBehaviour {
        kademlia,
        identify,
        ping,
        relay_client,
        chunk_proto,
        dag_proto,
    }
}

/// Load an Ed25519 keypair from disk, or generate a new one on first run.
///
/// The keypair IS our identity on the network. Same keypair = same PeerId.
/// That's why we save it to disk — so our identity persists between restarts.
fn load_or_generate_keypair(config: &NodeConfig) -> Result<identity::Keypair, String> {
    let path = config.keypair_path();

    if path.exists() {
        // Load existing keypair (we store the 32-byte secret seed)
        let bytes = fs::read(&path)
            .map_err(|e| format!("Failed to read keypair from {}: {}", path.display(), e))?;
        // Handle both old 64-byte files and new 32-byte files
        let seed: Vec<u8> = if bytes.len() == 64 { bytes[..32].to_vec() } else { bytes };
        let keypair = identity::Keypair::ed25519_from_bytes(seed)
            .map_err(|e| format!("Invalid keypair file: {}", e))?;
        info!("Loaded existing keypair from {}", path.display());
        Ok(keypair)
    } else {
        // Generate a new keypair and save it
        let keypair = identity::Keypair::generate_ed25519();

        // Make sure the directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
        }

        // Save the raw Ed25519 secret key bytes (32 bytes)
        // We need to extract the ed25519 keypair to get the raw bytes
        save_keypair(&keypair, &path)?;

        info!("Generated new keypair, saved to {}", path.display());
        Ok(keypair)
    }
}

/// Save an Ed25519 keypair to disk.
fn save_keypair(keypair: &identity::Keypair, path: &Path) -> Result<(), String> {
    // Get the Ed25519 keypair variant so we can access raw bytes
    let ed25519_kp = keypair
        .clone()
        .try_into_ed25519()
        .map_err(|e| format!("Not an Ed25519 keypair: {}", e))?;

    // Save only the 32-byte secret seed (not the full 64-byte keypair).
    // ed25519_from_bytes() expects 32 bytes when loading.
    let bytes = ed25519_kp.to_bytes();

    fs::write(path, &bytes[..32])
        .map_err(|e| format!("Failed to write keypair to {}: {}", path.display(), e))?;

    Ok(())
}
