// ── Event Loop ──
//
// The heart of the P2P node. Runs forever as a background task.
//
// It does two things in a loop (via tokio::select!):
//   1. Process swarm events — "a peer connected", "got a DHT result", etc.
//   2. Process commands from IPC — "what's my peer ID?", "fetch this chunk", etc.
//
// Some operations are async (DHT lookups, chunk requests to peers).
// When we start one, we store the caller's response channel in `pending`.
// When the result comes back from the network, we look it up and send the answer.

use std::collections::HashMap;
use std::path::PathBuf;

use libp2p::futures::StreamExt;
use libp2p::swarm::SwarmEvent;
use libp2p::{kad, identify, request_response, Multiaddr, PeerId, Swarm};
use tokio::sync::{mpsc, oneshot};
use tracing::{info, debug, warn};

use super::behaviour::{ChimeraBehaviour, ChimeraBehaviourEvent};
use super::handle::NodeCommand;
use crate::network::protocol::{
    self, ChunkRequest, ChunkResponse, DagRequest, DagResponse,
    DagNodeInfo, DhtSiteRecord,
};
use crate::storage::chunk_store::ChunkStore;
use crate::storage::database::Database;

/// Tracks in-flight async operations so we can match network responses
/// back to the IPC caller that requested them.
struct Pending {
    /// DHT get_record queries: QueryId → caller waiting for the record value
    dht_lookups: HashMap<kad::QueryId, oneshot::Sender<Result<DhtSiteRecord, String>>>,

    /// DHT put_record queries: QueryId → caller waiting for confirmation
    dht_puts: HashMap<kad::QueryId, oneshot::Sender<Result<(), String>>>,

    /// Chunk requests sent to peers: RequestId → caller waiting for chunk data
    chunk_requests: HashMap<request_response::OutboundRequestId, oneshot::Sender<Result<Vec<u8>, String>>>,

    /// DAG node requests sent to peers: RequestId → caller waiting for node info
    dag_requests: HashMap<request_response::OutboundRequestId, oneshot::Sender<Result<DagNodeInfo, String>>>,
}

impl Pending {
    fn new() -> Self {
        Pending {
            dht_lookups: HashMap::new(),
            dht_puts: HashMap::new(),
            chunk_requests: HashMap::new(),
            dag_requests: HashMap::new(),
        }
    }
}

/// Run the event loop forever. Call this as a Tokio background task.
///
/// `swarm`           — the libp2p network node (takes ownership)
/// `cmd_rx`          — receives commands from NodeHandle
/// `data_dir`        — path to ~/.chimera/ for storage access
/// `bootstrap_nodes` — relay addresses, used to dial peers via relay circuit
pub async fn run_event_loop(
    mut swarm: Swarm<ChimeraBehaviour>,
    mut cmd_rx: mpsc::Receiver<NodeCommand>,
    data_dir: PathBuf,
    bootstrap_nodes: Vec<String>,
) {
    // Parse relay addresses once upfront so we can use them to dial peers via relay circuit.
    let relay_addrs: Vec<Multiaddr> = bootstrap_nodes
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    // Open storage so we can serve chunks/DAG nodes to peers.
    let chunk_store = ChunkStore::new(&data_dir).expect("Failed to open chunk store");
    let db = Database::open(&data_dir).expect("Failed to open database");

    let mut pending = Pending::new();
    let mut bootstrapped = false;

    info!("Event loop started, waiting for events...");

    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                handle_swarm_event(&mut swarm, &chunk_store, &db, &mut pending, &relay_addrs, &mut bootstrapped, event);
            }
            Some(cmd) = cmd_rx.recv() => {
                handle_command(&mut swarm, &mut pending, &relay_addrs, cmd);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Command handling — IPC requests come in here
// ═══════════════════════════════════════════════════════════════════

fn handle_command(
    swarm: &mut Swarm<ChimeraBehaviour>,
    pending: &mut Pending,
    relay_addrs: &[Multiaddr],
    cmd: NodeCommand,
) {
    match cmd {
        NodeCommand::GetNodeId { resp } => {
            let _ = resp.send(swarm.local_peer_id().to_string());
        }

        NodeCommand::GetPeerCount { resp } => {
            let _ = resp.send(swarm.connected_peers().count() as u32);
        }

        NodeCommand::AnnounceSite {
            site_name, root_cid, total_size, chunk_count, published_at, resp,
        } => {
            let peer_id = swarm.local_peer_id().to_string();
            let record_value = DhtSiteRecord {
                root_cid,
                publisher_peer_id: peer_id,
                total_size,
                chunk_count,
                published_at,
            };

            let key = kad::RecordKey::new(&format!("/chimera/site/{}", site_name));
            let value = match serde_json::to_vec(&record_value) {
                Ok(v) => v,
                Err(e) => {
                    let _ = resp.send(Err(format!("Failed to serialize: {}", e)));
                    return;
                }
            };

            let record = kad::Record { key, value, publisher: None, expires: None };
            match swarm.behaviour_mut().kademlia.put_record(record, kad::Quorum::One) {
                Ok(query_id) => {
                    pending.dht_puts.insert(query_id, resp);
                }
                Err(e) => {
                    let _ = resp.send(Err(format!("DHT put failed: {:?}", e)));
                }
            }
        }

        NodeCommand::ResolveSiteName { site_name, resp } => {
            let key = kad::RecordKey::new(&format!("/chimera/site/{}", site_name));
            let query_id = swarm.behaviour_mut().kademlia.get_record(key);
            pending.dht_lookups.insert(query_id, resp);
        }

        NodeCommand::FetchChunk { cid, peer_id, resp } => {
            let pid = match peer_id.parse::<PeerId>() {
                Ok(p) => p,
                Err(e) => {
                    let _ = resp.send(Err(format!("Invalid PeerId: {}", e)));
                    return;
                }
            };
            // If we're not connected to this peer, dial them through the relay.
            // Both peers are behind NAT — they can't reach each other directly.
            // The relay forwards traffic between them.
            ensure_connected(swarm, &pid, relay_addrs);
            let request_id = swarm
                .behaviour_mut()
                .chunk_proto
                .send_request(&pid, ChunkRequest { cid });
            pending.chunk_requests.insert(request_id, resp);
        }

        NodeCommand::FetchDagNode { cid, peer_id, resp } => {
            let pid = match peer_id.parse::<PeerId>() {
                Ok(p) => p,
                Err(e) => {
                    let _ = resp.send(Err(format!("Invalid PeerId: {}", e)));
                    return;
                }
            };
            ensure_connected(swarm, &pid, relay_addrs);
            let request_id = swarm
                .behaviour_mut()
                .dag_proto
                .send_request(&pid, DagRequest { cid });
            pending.dag_requests.insert(request_id, resp);
        }
    }
}

/// If we're not connected to a peer, add relay circuit addresses so libp2p
/// can reach them through the relay. Both peers are behind NAT, so direct
/// connections won't work — traffic must go through the relay.
fn ensure_connected(
    swarm: &mut Swarm<ChimeraBehaviour>,
    peer_id: &PeerId,
    relay_addrs: &[Multiaddr],
) {
    if swarm.is_connected(peer_id) {
        return;
    }

    // Build relay circuit address: <relay_addr>/p2p-circuit/p2p/<target_peer_id>
    // This tells libp2p: "to reach this peer, go through the relay"
    for relay_addr in relay_addrs {
        let circuit_addr: Multiaddr = relay_addr
            .clone()
            .with(libp2p::multiaddr::Protocol::P2pCircuit)
            .with(libp2p::multiaddr::Protocol::P2p(*peer_id));

        info!("Dialing peer {} via relay circuit: {}", peer_id, circuit_addr);
        if let Err(e) = swarm.dial(circuit_addr) {
            warn!("Failed to dial via relay: {}", e);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Swarm event handling — network events come in here
// ═══════════════════════════════════════════════════════════════════

fn handle_swarm_event(
    swarm: &mut Swarm<ChimeraBehaviour>,
    chunk_store: &ChunkStore,
    db: &Database,
    pending: &mut Pending,
    relay_addrs: &[Multiaddr],
    bootstrapped: &mut bool,
    event: SwarmEvent<ChimeraBehaviourEvent>,
) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => {
            info!("Listening on: {}", address);
        }
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            info!("Connected to peer: {}", peer_id);
            // Re-trigger Kademlia bootstrap after connecting to the relay.
            // The initial bootstrap() call races with the connection — it fires
            // before the relay is connected, so it fails with "No known peers".
            // This ensures bootstrap actually runs once we have a peer.
            if !*bootstrapped {
                if let Ok(_) = swarm.behaviour_mut().kademlia.bootstrap() {
                    info!("Kademlia bootstrap triggered after connecting to {}", peer_id);
                    *bootstrapped = true;
                }
            }
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            info!("Disconnected from peer: {}", peer_id);
            // If we lost the relay connection, allow re-bootstrapping on next connect
            let is_relay = relay_addrs.iter().any(|addr| {
                addr.iter().any(|proto| {
                    matches!(proto, libp2p::multiaddr::Protocol::P2p(id) if id == peer_id)
                })
            });
            if is_relay {
                *bootstrapped = false;
            }
        }
        SwarmEvent::Behaviour(event) => {
            handle_behaviour_event(swarm, chunk_store, db, pending, event);
        }
        other => {
            debug!("Swarm event: {:?}", other);
        }
    }
}

fn handle_behaviour_event(
    swarm: &mut Swarm<ChimeraBehaviour>,
    chunk_store: &ChunkStore,
    db: &Database,
    pending: &mut Pending,
    event: ChimeraBehaviourEvent,
) {
    match event {
        ChimeraBehaviourEvent::Kademlia(e) => {
            handle_kademlia_event(pending, e);
        }
        ChimeraBehaviourEvent::Identify(e) => {
            handle_identify_event(swarm, e);
        }
        ChimeraBehaviourEvent::Ping(e) => {
            debug!("Ping from {}: {:?}", e.peer, e.result);
        }
        ChimeraBehaviourEvent::ChunkProto(e) => {
            handle_chunk_event(swarm, chunk_store, pending, e);
        }
        ChimeraBehaviourEvent::DagProto(e) => {
            handle_dag_event(swarm, db, pending, e);
        }
        ChimeraBehaviourEvent::RelayClient(e) => {
            info!("Relay client event: {:?}", e);
        }
    }
}

// ── Kademlia ──

fn handle_kademlia_event(pending: &mut Pending, event: kad::Event) {
    match event {
        kad::Event::OutboundQueryProgressed { id, result, .. } => {
            match result {
                // ── DHT get_record results ──
                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(
                    kad::PeerRecord { record, .. },
                ))) => {
                    info!("DHT GET: found record for key {:?} ({} bytes)",
                        String::from_utf8_lossy(record.key.as_ref()), record.value.len());
                    if let Some(resp) = pending.dht_lookups.remove(&id) {
                        match serde_json::from_slice::<DhtSiteRecord>(&record.value) {
                            Ok(site_record) => {
                                info!("DHT GET: resolved site → root_cid={}, publisher={}",
                                    site_record.root_cid, site_record.publisher_peer_id);
                                let _ = resp.send(Ok(site_record));
                            }
                            Err(e) => {
                                warn!("DHT GET: bad record format: {}", e);
                                let _ = resp.send(Err(format!("Bad DHT record: {}", e)));
                            }
                        }
                    }
                }
                kad::QueryResult::GetRecord(Ok(
                    kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                )) => {
                    if let Some(resp) = pending.dht_lookups.remove(&id) {
                        warn!("DHT GET: site not found (no record in DHT)");
                        let _ = resp.send(Err("Site not found in DHT".to_string()));
                    }
                }
                kad::QueryResult::GetRecord(Err(e)) => {
                    if let Some(resp) = pending.dht_lookups.remove(&id) {
                        warn!("DHT GET failed: {:?}", e);
                        let _ = resp.send(Err(format!("DHT lookup failed: {:?}", e)));
                    }
                }

                // ── DHT put_record results ──
                kad::QueryResult::PutRecord(Ok(_)) => {
                    if let Some(resp) = pending.dht_puts.remove(&id) {
                        let _ = resp.send(Ok(()));
                    }
                    info!("DHT PUT: successfully stored record");
                }
                kad::QueryResult::PutRecord(Err(e)) => {
                    warn!("DHT PUT failed: {:?}", e);
                    if let Some(resp) = pending.dht_puts.remove(&id) {
                        let _ = resp.send(Err(format!("DHT put failed: {:?}", e)));
                    }
                }

                // ── Other Kademlia results (bootstrap, providers, etc.) ──
                kad::QueryResult::Bootstrap(Ok(result)) => {
                    info!("Kademlia bootstrap step: {} remaining", result.num_remaining);
                }
                kad::QueryResult::GetClosestPeers(Ok(result)) => {
                    info!("Found {} closest peers", result.peers.len());
                }
                other => {
                    debug!("Kademlia query result: {:?}", other);
                }
            }
        }
        kad::Event::RoutingUpdated { peer, .. } => {
            info!("Kademlia routing updated: added {}", peer);
        }
        kad::Event::InboundRequest { request } => {
            info!("Kademlia inbound request: {:?}", request);
        }
        other => {
            debug!("Kademlia event: {:?}", other);
        }
    }
}

// ── Identify ──

fn handle_identify_event(swarm: &mut Swarm<ChimeraBehaviour>, event: identify::Event) {
    match event {
        identify::Event::Received { peer_id, info, .. } => {
            info!("Identified peer {}: protocols={:?}", peer_id, info.protocols);
            info!("  listen_addrs: {:?}", info.listen_addrs);
            // Add the peer's addresses to Kademlia so we can reach them later
            for addr in &info.listen_addrs {
                swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
            }
        }
        identify::Event::Sent { peer_id, .. } => {
            debug!("Sent identify to {}", peer_id);
        }
        other => {
            debug!("Identify event: {:?}", other);
        }
    }
}

// ── Chunk protocol ──
// Handles both: (a) serving chunks to peers, (b) receiving responses to our requests

fn handle_chunk_event(
    swarm: &mut Swarm<ChimeraBehaviour>,
    chunk_store: &ChunkStore,
    pending: &mut Pending,
    event: request_response::Event<ChunkRequest, ChunkResponse>,
) {
    match event {
        request_response::Event::Message { peer, message } => {
            match message {
                // A peer is asking US for a chunk — serve it from local storage
                request_response::Message::Request { request, channel, .. } => {
                    info!("Peer {} requested chunk: {}", peer, request.cid);
                    let response = if chunk_store.has(&request.cid) {
                        match chunk_store.load(&request.cid) {
                            Ok(data) => ChunkResponse {
                                cid: request.cid,
                                data,
                                found: true,
                            },
                            Err(_) => ChunkResponse {
                                cid: request.cid,
                                data: vec![],
                                found: false,
                            },
                        }
                    } else {
                        ChunkResponse {
                            cid: request.cid,
                            data: vec![],
                            found: false,
                        }
                    };

                    if let Err(e) = swarm
                        .behaviour_mut()
                        .chunk_proto
                        .send_response(channel, response)
                    {
                        warn!("Failed to send chunk response: {:?}", e);
                    }
                }

                // WE asked a peer for a chunk — route the response to the caller
                request_response::Message::Response { request_id, response } => {
                    if let Some(resp) = pending.chunk_requests.remove(&request_id) {
                        if response.found {
                            let _ = resp.send(Ok(response.data));
                        } else {
                            let _ = resp.send(Err(format!(
                                "Peer doesn't have chunk {}", response.cid
                            )));
                        }
                    }
                }
            }
        }
        request_response::Event::OutboundFailure { request_id, error, .. } => {
            if let Some(resp) = pending.chunk_requests.remove(&request_id) {
                let _ = resp.send(Err(format!("Chunk request failed: {}", error)));
            }
        }
        request_response::Event::InboundFailure { peer, error, .. } => {
            warn!("Chunk response to {} failed: {}", peer, error);
        }
        _ => {}
    }
}

// ── DAG protocol ──
// Same pattern: serve local DAG nodes, and route responses for our own requests

fn handle_dag_event(
    swarm: &mut Swarm<ChimeraBehaviour>,
    db: &Database,
    pending: &mut Pending,
    event: request_response::Event<DagRequest, DagResponse>,
) {
    match event {
        request_response::Event::Message { peer, message } => {
            match message {
                // A peer is asking US for a DAG node — look it up in our database
                request_response::Message::Request { request, channel, .. } => {
                    info!("Peer {} requested DAG node: {}", peer, request.cid);
                    let response = match db.get_dag_node(&request.cid) {
                        Ok(Some(record)) => {
                            // Convert DB record → network protocol type
                            let links: Vec<protocol::DagLink> =
                                serde_json::from_str(&record.links_json).unwrap_or_default();
                            DagResponse {
                                cid: request.cid,
                                node: Some(DagNodeInfo {
                                    cid: record.cid,
                                    name: record.name,
                                    node_type: record.node_type,
                                    size: record.size as u64,
                                    links,
                                }),
                            }
                        }
                        _ => DagResponse {
                            cid: request.cid,
                            node: None,
                        },
                    };

                    if let Err(e) = swarm
                        .behaviour_mut()
                        .dag_proto
                        .send_response(channel, response)
                    {
                        warn!("Failed to send DAG response: {:?}", e);
                    }
                }

                // WE asked a peer for a DAG node — route the response to the caller
                request_response::Message::Response { request_id, response } => {
                    if let Some(resp) = pending.dag_requests.remove(&request_id) {
                        match response.node {
                            Some(node) => {
                                let _ = resp.send(Ok(node));
                            }
                            None => {
                                let _ = resp.send(Err(format!(
                                    "Peer doesn't have DAG node {}", response.cid
                                )));
                            }
                        }
                    }
                }
            }
        }
        request_response::Event::OutboundFailure { request_id, error, .. } => {
            if let Some(resp) = pending.dag_requests.remove(&request_id) {
                let _ = resp.send(Err(format!("DAG request failed: {}", error)));
            }
        }
        request_response::Event::InboundFailure { peer, error, .. } => {
            warn!("DAG response to {} failed: {}", peer, error);
        }
        _ => {}
    }
}
