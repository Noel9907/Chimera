use serde::Serialize;
use std::fs;
use std::path::Path;

use super::chunker;
use super::cid;

// ── Data Structures ──

/// A link from one DAG node to another (or to a chunk).
/// Think of it as: "this directory contains a file called X with CID Y".
#[derive(Clone, Serialize)]
pub struct DagLink {
    pub name: String,
    pub cid: String,
    pub size: u64,
}

/// A node in the Merkle DAG — either a file or a directory.
/// Files link to their chunks. Directories link to their children.
pub struct DagNode {
    pub cid: String,          // computed from the node's content
    pub name: String,         // "index.html" or "css" etc.
    pub node_type: String,    // "file" or "directory"
    pub size: u64,            // total bytes this node represents
    pub links: Vec<DagLink>,  // what this node points to
}

/// A chunk ready to be stored. Has its CID and raw data.
pub struct StoredChunk {
    pub cid: String,
    pub data: Vec<u8>,
    pub size: u64,
}

/// Everything that comes out of building a DAG from a folder.
pub struct DagBuildResult {
    pub root_cid: String,
    pub nodes: Vec<DagNode>,
    pub chunks: Vec<StoredChunk>,
    pub total_size: u64,
    pub file_count: u32,
    pub chunk_count: u32,
}

// ── This is what gets hashed to produce a DagNode's CID ──
// Same fields as DagNode, but WITHOUT the cid field.
// (You can't include the CID in the thing you're hashing to produce the CID!)
#[derive(Serialize)]
struct DagNodeContent {
    name: String,
    node_type: String,
    size: u64,
    links: Vec<DagLink>,
}

// ── Public API ──

/// Build a Merkle DAG from a folder.
/// Returns all nodes, all chunks, and the root CID.
pub fn build_dag(folder_path: &Path) -> Result<DagBuildResult, String> {
    if !folder_path.is_dir() {
        return Err(format!("{} is not a directory", folder_path.display()));
    }

    // Get the folder name (e.g., "my-site")
    let folder_name = folder_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // These collect all chunks and nodes as we recurse through the folder
    let mut all_chunks: Vec<StoredChunk> = Vec::new();
    let mut all_nodes: Vec<DagNode> = Vec::new();

    // Recursively build the DAG starting from the root folder
    let root_node = build_node(folder_path, &folder_name, &mut all_chunks, &mut all_nodes)?;

    let root_cid = root_node.cid.clone();
    let total_size = root_node.size;
    let file_count = all_nodes.iter().filter(|n| n.node_type == "file").count() as u32;
    let chunk_count = all_chunks.len() as u32;

    // Add the root node itself
    all_nodes.push(root_node);

    Ok(DagBuildResult {
        root_cid,
        nodes: all_nodes,
        chunks: all_chunks,
        total_size,
        file_count,
        chunk_count,
    })
}

// ── Internal ──

/// Recursively build a DAG node for a file or directory.
fn build_node(
    path: &Path,
    name: &str,
    all_chunks: &mut Vec<StoredChunk>,
    all_nodes: &mut Vec<DagNode>,
) -> Result<DagNode, String> {
    if path.is_file() {
        build_file_node(path, name, all_chunks)
    } else if path.is_dir() {
        build_dir_node(path, name, all_chunks, all_nodes)
    } else {
        Err(format!("{} is not a file or directory", path.display()))
    }
}

/// Build a DAG node for a single file.
/// Chunks the file, creates a CID for each chunk, then creates a file node
/// whose links point to all its chunks (in order).
fn build_file_node(
    path: &Path,
    name: &str,
    all_chunks: &mut Vec<StoredChunk>,
) -> Result<DagNode, String> {
    // Step 1: Read and chunk the file
    let raw_chunks = chunker::chunk_file(path)?;

    // Step 2: For each chunk, compute its CID and save it
    let mut links: Vec<DagLink> = Vec::new();
    let mut total_size: u64 = 0;

    for (i, chunk) in raw_chunks.into_iter().enumerate() {
        let chunk_cid = cid::cid_from_bytes(&chunk.data);
        let size = chunk.size as u64;

        links.push(DagLink {
            name: format!("chunk{}", i),
            cid: chunk_cid.clone(),
            size,
        });

        all_chunks.push(StoredChunk {
            cid: chunk_cid,
            data: chunk.data,
            size,
        });

        total_size += size;
    }

    // Step 3: Compute this file node's own CID
    // (hash of: name + type + size + links)
    let node_cid = compute_node_cid(name, "file", total_size, &links);

    Ok(DagNode {
        cid: node_cid,
        name: name.to_string(),
        node_type: "file".to_string(),
        size: total_size,
        links,
    })
}

/// Build a DAG node for a directory.
/// Recursively processes all children, then creates a directory node
/// whose links point to all child nodes.
fn build_dir_node(
    path: &Path,
    name: &str,
    all_chunks: &mut Vec<StoredChunk>,
    all_nodes: &mut Vec<DagNode>,
) -> Result<DagNode, String> {
    // Read directory entries and sort them (so the DAG is deterministic)
    let mut entries: Vec<_> = fs::read_dir(path)
        .map_err(|e| format!("Failed to read dir {}: {}", path.display(), e))?
        .filter_map(|entry| entry.ok())
        .collect();

    entries.sort_by_key(|e| e.file_name());

    // Process each child
    let mut links: Vec<DagLink> = Vec::new();
    let mut total_size: u64 = 0;

    for entry in entries {
        let child_path = entry.path();
        let child_name = entry.file_name().to_string_lossy().to_string();

        // Recursively build DAG for this child
        let child_node = build_node(&child_path, &child_name, all_chunks, all_nodes)?;

        links.push(DagLink {
            name: child_node.name.clone(),
            cid: child_node.cid.clone(),
            size: child_node.size,
        });

        total_size += child_node.size;

        // Save the child node (not the root — that gets added in build_dag)
        all_nodes.push(child_node);
    }

    // Compute this directory node's CID
    let node_cid = compute_node_cid(name, "directory", total_size, &links);

    Ok(DagNode {
        cid: node_cid,
        name: name.to_string(),
        node_type: "directory".to_string(),
        size: total_size,
        links,
    })
}

/// Compute a DAG node's CID by serializing its content to JSON and hashing it.
fn compute_node_cid(name: &str, node_type: &str, size: u64, links: &[DagLink]) -> String {
    let content = DagNodeContent {
        name: name.to_string(),
        node_type: node_type.to_string(),
        size,
        links: links.to_vec(),
    };

    // Serialize to JSON, then hash to get the CID
    let json = serde_json::to_string(&content).expect("Failed to serialize DAG node");
    cid::cid_from_dag_json(json.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper: create a temp directory with some files for testing.
    /// Each test gets a unique folder name to avoid conflicts (tests run in parallel).
    fn create_test_site(test_name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("chimera_test_{}", test_name));

        // Clean up if it exists from a previous test run
        let _ = fs::remove_dir_all(&dir);

        // Create structure:
        // test_site/
        //   index.html
        //   css/
        //     style.css
        fs::create_dir_all(dir.join("css")).unwrap();
        fs::write(dir.join("index.html"), "<h1>Hello Chimera</h1>").unwrap();
        fs::write(dir.join("css").join("style.css"), "body { color: red; }").unwrap();

        dir
    }

    #[test]
    fn test_build_dag_basic() {
        let site_dir = create_test_site("basic");
        let result = build_dag(&site_dir).unwrap();

        // Should have 2 files
        assert_eq!(result.file_count, 2);

        // Should have 2 chunks (each file is small = 1 chunk each)
        assert_eq!(result.chunk_count, 2);

        // Root CID should exist
        assert!(!result.root_cid.is_empty());

        // Total size should be the sum of file sizes
        let expected_size = "<h1>Hello Chimera</h1>".len() + "body { color: red; }".len();
        assert_eq!(result.total_size, expected_size as u64);

        // Clean up
        let _ = fs::remove_dir_all(&site_dir);
    }

    #[test]
    fn test_dag_has_correct_structure() {
        let site_dir = create_test_site("structure");
        let result = build_dag(&site_dir).unwrap();

        // Find the root node (last one added)
        let root = result.nodes.iter().find(|n| n.cid == result.root_cid).unwrap();
        assert_eq!(root.node_type, "directory");

        // Root should have 2 links: "css" (dir) and "index.html" (file)
        assert_eq!(root.links.len(), 2);

        let link_names: Vec<&str> = root.links.iter().map(|l| l.name.as_str()).collect();
        assert!(link_names.contains(&"css"));
        assert!(link_names.contains(&"index.html"));

        let _ = fs::remove_dir_all(&site_dir);
    }

    #[test]
    fn test_dag_is_deterministic() {
        let site_dir = create_test_site("deterministic");

        // Build DAG twice from the same folder
        let result1 = build_dag(&site_dir).unwrap();
        let result2 = build_dag(&site_dir).unwrap();

        // Root CID should be identical both times
        assert_eq!(result1.root_cid, result2.root_cid);

        let _ = fs::remove_dir_all(&site_dir);
    }

    #[test]
    fn test_not_a_directory() {
        let file = std::env::temp_dir().join("chimera_test_file.txt");
        fs::write(&file, "hello").unwrap();

        let result = build_dag(&file);
        assert!(result.is_err());

        let _ = fs::remove_file(&file);
    }
}
