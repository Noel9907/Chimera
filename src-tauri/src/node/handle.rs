// ── Node Handle ──
//
// The "remote control" for the P2P node.
//
// The swarm lives inside the event loop (a background Tokio task).
// Other parts of the app (IPC commands) can't touch the swarm directly.
// Instead, they send commands through a channel:
//
//   IPC command  →  NodeHandle.send(command)  →  channel  →  event loop  →  swarm
//                          ↑                                                  |
//                          └──────────── oneshot response ←───────────────────┘
//
// Each command carries a oneshot::Sender so the event loop can send back the result.

use tokio::sync::{mpsc, oneshot};

use crate::network::protocol::{DagNodeInfo, DhtSiteRecord};

/// A command sent from IPC handlers to the event loop.
///
/// Each variant carries a `resp` field — a one-time channel to send
/// the result back to whoever asked.
pub enum NodeCommand {
    /// Get this node's PeerId (our identity on the network).
    GetNodeId {
        resp: oneshot::Sender<String>,
    },

    /// Get the number of currently connected peers.
    GetPeerCount {
        resp: oneshot::Sender<u32>,
    },

    /// Announce a published site to the DHT.
    /// Stores site_name → DhtSiteRecord so other peers can find it.
    AnnounceSite {
        site_name: String,
        root_cid: String,
        total_size: u64,
        chunk_count: u32,
        published_at: String,
        resp: oneshot::Sender<Result<(), String>>,
    },

    /// Look up a site name in the DHT to find its root CID and publisher.
    /// This starts an async Kademlia query — the result comes back later.
    ResolveSiteName {
        site_name: String,
        resp: oneshot::Sender<Result<DhtSiteRecord, String>>,
    },

    /// Request a chunk from a specific peer by their PeerId string.
    FetchChunk {
        cid: String,
        peer_id: String,
        resp: oneshot::Sender<Result<Vec<u8>, String>>,
    },

    /// Request a DAG node from a specific peer by their PeerId string.
    FetchDagNode {
        cid: String,
        peer_id: String,
        resp: oneshot::Sender<Result<DagNodeInfo, String>>,
    },
}

/// A handle to the running P2P node.
///
/// This is cheap to clone and safe to share across threads.
/// Stored in Tauri's managed state so any IPC command can use it.
#[derive(Clone)]
pub struct NodeHandle {
    cmd_tx: mpsc::Sender<NodeCommand>,
}

impl NodeHandle {
    pub fn new(cmd_tx: mpsc::Sender<NodeCommand>) -> Self {
        NodeHandle { cmd_tx }
    }

    /// Send a command and wait for the response.
    /// Returns Err if the event loop has stopped.
    async fn send<T>(
        &self,
        make_cmd: impl FnOnce(oneshot::Sender<T>) -> NodeCommand,
    ) -> Result<T, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(make_cmd(tx))
            .await
            .map_err(|_| "Node is not running".to_string())?;
        rx.await
            .map_err(|_| "Node stopped responding".to_string())
    }

    pub async fn get_node_id(&self) -> Result<String, String> {
        self.send(|resp| NodeCommand::GetNodeId { resp }).await
    }

    pub async fn get_peer_count(&self) -> Result<u32, String> {
        self.send(|resp| NodeCommand::GetPeerCount { resp }).await
    }

    /// Announce a site to the DHT after publishing locally.
    pub async fn announce_site(
        &self,
        site_name: String,
        root_cid: String,
        total_size: u64,
        chunk_count: u32,
        published_at: String,
    ) -> Result<(), String> {
        self.send(|resp| NodeCommand::AnnounceSite {
            site_name, root_cid, total_size, chunk_count, published_at, resp,
        })
        .await?
    }

    /// Look up a site name in the DHT. Returns the site record if found.
    pub async fn resolve_site_name(&self, site_name: &str) -> Result<DhtSiteRecord, String> {
        self.send(|resp| NodeCommand::ResolveSiteName {
            site_name: site_name.to_string(),
            resp,
        })
        .await?
    }

    /// Fetch a chunk's raw bytes from a specific peer.
    pub async fn fetch_chunk(&self, cid: &str, peer_id: &str) -> Result<Vec<u8>, String> {
        self.send(|resp| NodeCommand::FetchChunk {
            cid: cid.to_string(),
            peer_id: peer_id.to_string(),
            resp,
        })
        .await?
    }

    /// Fetch a DAG node from a specific peer.
    pub async fn fetch_dag_node(&self, cid: &str, peer_id: &str) -> Result<DagNodeInfo, String> {
        self.send(|resp| NodeCommand::FetchDagNode {
            cid: cid.to_string(),
            peer_id: peer_id.to_string(),
            resp,
        })
        .await?
    }
}
