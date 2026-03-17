// ── Chimera Network Behaviour ──
//
// This combines ALL the protocols our node speaks into one struct.
// libp2p's #[derive(NetworkBehaviour)] macro does the heavy lifting —
// it automatically routes incoming messages to the right protocol handler.
//
// Think of it like: "My node can do Kademlia AND Identify AND Ping AND ..."

use libp2p::kad;
use libp2p::identify;
use libp2p::ping;
use libp2p::relay;
use libp2p::swarm::NetworkBehaviour;

use crate::network::protocol::{ChunkCodec, DagCodec};

/// Everything our node can do on the network, combined into one type.
///
/// When the swarm receives a message, it figures out which protocol
/// it belongs to and routes it to the right field here.
#[derive(NetworkBehaviour)]
pub struct ChimeraBehaviour {
    /// Kademlia DHT — peer discovery + key-value storage.
    /// Used to: find peers, resolve site names to root CIDs,
    /// find which peers have a specific chunk.
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,

    /// Identify — peers exchange basic info (PeerId, supported protocols, addresses).
    /// Runs automatically when peers connect.
    pub identify: identify::Behaviour,

    /// Ping — periodic liveness checks. If a peer stops responding, we know it's gone.
    pub ping: ping::Behaviour,

    /// Relay client — allows us to connect through the relay server
    /// when we're behind NAT (can't accept incoming connections directly).
    pub relay_client: relay::client::Behaviour,

    /// Our custom chunk transfer protocol.
    /// Peers use this to request and send chunk data.
    pub chunk_proto: ChunkCodec,

    /// Our custom DAG node transfer protocol.
    /// Peers use this to request and send Merkle DAG node info.
    pub dag_proto: DagCodec,
}
