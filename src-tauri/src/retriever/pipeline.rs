// ── Retriever Pipeline ──
//
// The opposite of the publisher. Given a chimera:// URL, this:
//   1. Resolves the site name → root CID (via local cache or DHT)
//   2. Fetches DAG nodes to navigate the site's directory tree
//   3. Fetches file chunks from peers
//   4. Reassembles the file and returns it
//
// Uses NodeHandle for network operations and local storage for caching.
//
// Important: Database (rusqlite) is not Send, so we never hold a &Database
// across an .await point. We open short-lived connections for sync work,
// then drop them before any async call.

use std::path::Path;

use crate::network::protocol::DagNodeInfo;
use crate::node::handle::NodeHandle;
use crate::storage::chunk_store::ChunkStore;
use crate::storage::database::Database;

/// The result of fetching a file from a chimera:// site.
pub struct RetrievedFile {
    pub content_type: String,
    pub body: Vec<u8>,
}

/// Retrieve a file from a chimera:// site.
///
/// `site_name` — the site to fetch from (e.g., "my-portfolio")
/// `file_path` — path within the site (e.g., "/about.html", defaults to "/index.html")
/// `handle`    — NodeHandle for network operations
/// `data_dir`  — path to ~/.chimera/ for local cache
pub async fn retrieve_file(
    site_name: &str,
    file_path: &str,
    handle: &NodeHandle,
    data_dir: &Path,
) -> Result<RetrievedFile, String> {
    // Default to index.html
    let path = if file_path.is_empty() || file_path == "/" {
        "index.html"
    } else {
        file_path.trim_start_matches('/')
    };

    // ── Step 1: Resolve site name to root CID ──
    let (root_cid, publisher_peer_id) = resolve_site(site_name, handle, data_dir).await?;

    // ── Step 2: Fetch root DAG node (the site's top-level directory) ──
    let root_node = fetch_dag_node(&root_cid, &publisher_peer_id, handle, data_dir).await?;

    // ── Step 3: Navigate the DAG to find the requested file ──
    // "css/style.css" → walk: root → "css" dir → "style.css" file
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let file_node = navigate_to_file(&root_node, &segments, &publisher_peer_id, handle, data_dir).await?;

    if file_node.node_type != "file" {
        return Err(format!("'{}' is a directory, not a file", path));
    }

    // ── Step 4: Fetch all chunks and reassemble the file ──
    let body = fetch_file_chunks(&file_node, &publisher_peer_id, handle, data_dir).await?;

    // ── Step 5: Determine content type from file extension ──
    let content_type = content_type_from_path(path).to_string();

    Ok(RetrievedFile { content_type, body })
}

/// Resolve a site name to its root CID and publisher peer.
/// Checks local database first (cache), then queries the DHT.
async fn resolve_site(
    site_name: &str,
    handle: &NodeHandle,
    data_dir: &Path,
) -> Result<(String, String), String> {
    // Check local cache (sync block — DB is dropped before any .await)
    {
        let db = Database::open(data_dir)?;
        if let Ok(Some(site)) = db.get_site(site_name) {
            return Ok((site.root_cid, site.publisher_peer_id));
        }
    }

    // Not cached — ask the DHT
    let record = handle.resolve_site_name(site_name).await?;

    // Cache the result so we don't hit the DHT next time
    {
        let db = Database::open(data_dir)?;
        let _ = db.insert_site(
            site_name,
            &record.root_cid,
            record.total_size as i64,
            record.chunk_count as i32,
            0, // file_count not in DHT record
            false,
            &record.published_at,
            &record.publisher_peer_id,
        );
    }

    Ok((record.root_cid, record.publisher_peer_id))
}

/// Fetch a DAG node — from local cache if we have it, otherwise from the publisher peer.
async fn fetch_dag_node(
    cid: &str,
    publisher_peer_id: &str,
    handle: &NodeHandle,
    data_dir: &Path,
) -> Result<DagNodeInfo, String> {
    // Check local cache first
    {
        let db = Database::open(data_dir)?;
        if let Ok(Some(record)) = db.get_dag_node(cid) {
            let links = serde_json::from_str(&record.links_json).unwrap_or_default();
            return Ok(DagNodeInfo {
                cid: record.cid,
                name: record.name,
                node_type: record.node_type,
                size: record.size as u64,
                links,
            });
        }
    }

    // Fetch from the publisher peer
    let node = handle.fetch_dag_node(cid, publisher_peer_id).await?;

    // Cache it locally
    {
        let db = Database::open(data_dir)?;
        let links_json = serde_json::to_string(&node.links).unwrap_or_else(|_| "[]".to_string());
        let _ = db.insert_dag_node(&node.cid, &node.name, &node.node_type, node.size as i64, &links_json);
    }

    Ok(node)
}

/// Walk the DAG tree following path segments.
/// e.g., ["css", "style.css"] → find "css" dir in root → find "style.css" in css dir.
async fn navigate_to_file(
    root: &DagNodeInfo,
    segments: &[&str],
    publisher_peer_id: &str,
    handle: &NodeHandle,
    data_dir: &Path,
) -> Result<DagNodeInfo, String> {
    let mut current = root.clone();

    for (i, segment) in segments.iter().enumerate() {
        let link = current
            .links
            .iter()
            .find(|l| l.name == *segment)
            .ok_or_else(|| format!("'{}' not found in directory '{}'", segment, current.name))?;

        let node = fetch_dag_node(&link.cid, publisher_peer_id, handle, data_dir).await?;

        // If there are more segments, this must be a directory
        if i < segments.len() - 1 && node.node_type != "directory" {
            return Err(format!("'{}' is not a directory", segment));
        }

        current = node;
    }

    Ok(current)
}

/// Fetch all chunks for a file node and concatenate them in order.
async fn fetch_file_chunks(
    file_node: &DagNodeInfo,
    publisher_peer_id: &str,
    handle: &NodeHandle,
    data_dir: &Path,
) -> Result<Vec<u8>, String> {
    let chunk_store = ChunkStore::new(data_dir)?;
    let mut body = Vec::with_capacity(file_node.size as usize);

    for link in &file_node.links {
        let chunk_data = if chunk_store.has(&link.cid) {
            // Cache hit
            chunk_store.load(&link.cid)?
        } else {
            // Fetch from peer
            let data = handle.fetch_chunk(&link.cid, publisher_peer_id).await?;
            // Store locally — this is how "browsing = seeding" works
            let _ = chunk_store.save(&link.cid, &data);
            data
        };

        body.extend_from_slice(&chunk_data);
    }

    Ok(body)
}

/// Map file extension to MIME content type.
fn content_type_from_path(path: &str) -> &str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}
