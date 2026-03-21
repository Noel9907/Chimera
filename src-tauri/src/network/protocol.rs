// ── Network Protocol Definitions ──
//
// This file defines HOW peers talk to each other.
// We have two protocols:
//   1. Chunk protocol: "give me chunk X" → "here are the bytes"
//   2. DAG protocol: "give me DAG node X" → "here's the node info"
//
// Each protocol needs:
//   - Request/Response structs (what gets sent)
//   - A Codec (how to turn structs into bytes and back)

use libp2p::request_response;
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};

// ── Protocol identifiers ──
// These strings uniquely identify our protocols on the network.
// Every Chimera peer uses the same strings, so they know they speak the same language.

pub fn chunk_protocol() -> StreamProtocol {
    StreamProtocol::new("/chimera/chunk/1.0.0")
}

pub fn dag_protocol() -> StreamProtocol {
    StreamProtocol::new("/chimera/dag/1.0.0")
}

// ── Chunk Protocol Messages ──

/// "Hey peer, do you have this chunk?"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRequest {
    pub cid: String,
}

/// "Yes, here it is" (or "no, I don't have it")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResponse {
    pub cid: String,
    pub data: Vec<u8>,   // raw chunk bytes (empty if not found)
    pub found: bool,
}

// ── DAG Protocol Messages ──

/// "Hey peer, do you have this DAG node?"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagRequest {
    pub cid: String,
}

/// A single link inside a DAG node (sent over the network).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagLink {
    pub name: String,
    pub cid: String,
    pub size: u64,
}

/// A DAG node as sent over the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNodeInfo {
    pub cid: String,
    pub name: String,
    pub node_type: String,   // "file" or "directory"
    pub size: u64,
    pub links: Vec<DagLink>,
}

/// "Here's the DAG node" (or "I don't have it")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagResponse {
    pub cid: String,
    pub node: Option<DagNodeInfo>,  // None = "I don't have it"
}

// ── DHT record ──
// This is the value stored in the Kademlia DHT for site name resolution.
// Key: "/chimera/site/{site_name}" → Value: serialized DhtSiteRecord

/// The metadata stored in the DHT so other peers can find a published site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtSiteRecord {
    pub root_cid: String,
    pub publisher_peer_id: String,
    pub total_size: u64,
    pub chunk_count: u32,
    pub published_at: String,
}

// ── Codec ──
//
// A "codec" tells libp2p how to read/write our messages from/to raw bytes.
// We use libp2p's built-in CBOR codec (like JSON but binary, smaller, faster).
// This means we just need to tell it which types to use — it handles the rest.

pub type ChunkCodec =
    request_response::cbor::Behaviour<ChunkRequest, ChunkResponse>;

pub type DagCodec =
    request_response::cbor::Behaviour<DagRequest, DagResponse>;
