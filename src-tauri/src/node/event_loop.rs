// ── Event Loop ──
//
// The swarm produces events: "a peer connected", "Kademlia found a value",
// "someone requested a chunk", etc.
//
// This loop runs forever (as a background task) processing those events.
// Think of it like an inbox — events come in, we handle each one.
//
// For now, we just log events. As we build retrieval and serving,
// we'll add actual handlers here.

use libp2p::futures::StreamExt;
use libp2p::swarm::SwarmEvent;
use libp2p::{kad, identify, request_response, Swarm};
use tracing::{info, debug, warn};

use super::behaviour::{ChimeraBehaviour, ChimeraBehaviourEvent};

/// Run the event loop forever. Call this as a Tokio background task.
///
/// `swarm` is passed by ownership — this function takes over the swarm
/// and processes events until the program shuts down.
pub async fn run_event_loop(mut swarm: Swarm<ChimeraBehaviour>) {
    info!("Event loop started, waiting for events...");

    loop {
        // swarm.select_next_some() waits for the next event.
        // It's async, so it doesn't block — other tasks can run while we wait.
        let event = swarm.select_next_some().await;

        match event {
            // ── Connection events ──
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on: {}", address);
            }

            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {}", peer_id);
            }

            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
            }

            // ── Behaviour events (from our protocols) ──
            SwarmEvent::Behaviour(event) => {
                handle_behaviour_event(&mut swarm, event);
            }

            // ── Everything else (dialing, listener errors, etc.) ──
            other => {
                debug!("Swarm event: {:?}", other);
            }
        }
    }
}

/// Handle events from our composed behaviour.
/// Each protocol (Kademlia, Identify, Ping, etc.) produces its own event type.
fn handle_behaviour_event(
    swarm: &mut Swarm<ChimeraBehaviour>,
    event: ChimeraBehaviourEvent,
) {
    match event {
        // ── Kademlia events ──
        ChimeraBehaviourEvent::Kademlia(kad_event) => {
            handle_kademlia_event(kad_event);
        }

        // ── Identify events ──
        ChimeraBehaviourEvent::Identify(identify_event) => {
            handle_identify_event(swarm, identify_event);
        }

        // ── Ping events ──
        ChimeraBehaviourEvent::Ping(ping_event) => {
            debug!(
                "Ping from {}: {:?}",
                ping_event.peer,
                ping_event.result
            );
        }

        // ── Chunk protocol events ──
        ChimeraBehaviourEvent::ChunkProto(chunk_event) => {
            handle_chunk_event(chunk_event);
        }

        // ── DAG protocol events ──
        ChimeraBehaviourEvent::DagProto(dag_event) => {
            handle_dag_event(dag_event);
        }

        // ── Relay client events ──
        ChimeraBehaviourEvent::RelayClient(relay_event) => {
            debug!("Relay event: {:?}", relay_event);
        }
    }
}

/// Handle Kademlia DHT events.
fn handle_kademlia_event(event: kad::Event) {
    match event {
        kad::Event::OutboundQueryProgressed { result, .. } => {
            match result {
                kad::QueryResult::Bootstrap(Ok(result)) => {
                    info!(
                        "Kademlia bootstrap step: {} remaining peers",
                        result.num_remaining
                    );
                }
                kad::QueryResult::GetClosestPeers(Ok(result)) => {
                    info!(
                        "Found {} closest peers",
                        result.peers.len()
                    );
                }
                kad::QueryResult::GetRecord(Ok(result)) => {
                    info!("Got DHT record (GetRecord succeeded)");
                    // We'll handle actual record data when we build retrieval
                    let _ = result;
                }
                kad::QueryResult::PutRecord(Ok(_)) => {
                    info!("Successfully stored record in DHT");
                }
                kad::QueryResult::GetProviders(Ok(result)) => {
                    match result {
                        kad::GetProvidersOk::FoundProviders { providers, .. } => {
                            info!("Found {} providers", providers.len());
                        }
                        kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. } => {
                            debug!("Provider search finished");
                        }
                    }
                }
                kad::QueryResult::StartProviding(Ok(_)) => {
                    debug!("Now providing content to DHT");
                }
                // Log failures but don't crash
                other => {
                    debug!("Kademlia query result: {:?}", other);
                }
            }
        }
        kad::Event::RoutingUpdated { peer, .. } => {
            debug!("Kademlia routing table updated: added {}", peer);
        }
        other => {
            debug!("Kademlia event: {:?}", other);
        }
    }
}

/// Handle Identify events.
/// When a peer identifies itself, we add its addresses to Kademlia
/// so we can find it again later.
fn handle_identify_event(
    swarm: &mut Swarm<ChimeraBehaviour>,
    event: identify::Event,
) {
    match event {
        identify::Event::Received { peer_id, info, .. } => {
            info!(
                "Identified peer {}: protocols={:?}",
                peer_id,
                info.protocols
            );

            // Add the peer's addresses to our Kademlia routing table.
            // This is how Kademlia learns about new peers — through Identify.
            for addr in info.listen_addrs {
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, addr);
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

/// Handle incoming chunk requests.
/// For now, just log them. We'll add actual chunk serving when we build retrieval.
fn handle_chunk_event(
    event: request_response::Event<
        crate::network::protocol::ChunkRequest,
        crate::network::protocol::ChunkResponse,
    >,
) {
    match event {
        request_response::Event::Message { peer, message } => {
            match message {
                request_response::Message::Request { request, channel, .. } => {
                    info!("Peer {} requested chunk: {}", peer, request.cid);
                    // TODO: look up chunk in store, send response via channel
                    // For now, respond with "not found"
                    let response = crate::network::protocol::ChunkResponse {
                        cid: request.cid,
                        data: vec![],
                        found: false,
                    };
                    let _ = channel;
                    // We'll wire this up when we build chunk serving
                    let _ = response;
                }
                request_response::Message::Response { response, .. } => {
                    if response.found {
                        info!("Received chunk {} ({} bytes)", response.cid, response.data.len());
                    } else {
                        warn!("Peer doesn't have chunk {}", response.cid);
                    }
                }
            }
        }
        request_response::Event::OutboundFailure { peer, error, .. } => {
            warn!("Chunk request to {} failed: {}", peer, error);
        }
        request_response::Event::InboundFailure { peer, error, .. } => {
            warn!("Chunk response to {} failed: {}", peer, error);
        }
        _ => {}
    }
}

/// Handle incoming DAG node requests.
/// Same pattern as chunk events — log for now, wire up later.
fn handle_dag_event(
    event: request_response::Event<
        crate::network::protocol::DagRequest,
        crate::network::protocol::DagResponse,
    >,
) {
    match event {
        request_response::Event::Message { peer, message } => {
            match message {
                request_response::Message::Request { request, channel, .. } => {
                    info!("Peer {} requested DAG node: {}", peer, request.cid);
                    // TODO: look up DAG node in database, send response
                    let _ = channel;
                }
                request_response::Message::Response { response, .. } => {
                    if response.node.is_some() {
                        info!("Received DAG node {}", response.cid);
                    } else {
                        warn!("Peer doesn't have DAG node {}", response.cid);
                    }
                }
            }
        }
        request_response::Event::OutboundFailure { peer, error, .. } => {
            warn!("DAG request to {} failed: {}", peer, error);
        }
        request_response::Event::InboundFailure { peer, error, .. } => {
            warn!("DAG response to {} failed: {}", peer, error);
        }
        _ => {}
    }
}
