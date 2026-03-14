use std::path::Path;

use crate::content::merkle;
use crate::storage::chunk_store::ChunkStore;
use crate::storage::database::Database;

/// The result of publishing a site. This is what gets returned to the frontend.
#[derive(Debug)]
pub struct PublishResult {
    pub site_name: String,
    pub root_cid: String,
    pub total_size: u64,
    pub chunk_count: u32,
    pub file_count: u32,
}

/// Publish a static site from a local folder.
///
/// This is the main function that ties everything together:
/// 1. Validates the input
/// 2. Builds the Merkle DAG (chunks all files, generates CIDs)
/// 3. Saves chunks to disk
/// 4. Saves DAG nodes and site record to SQLite
///
/// After this, the site is ready to be served to peers (once networking is added).
pub fn publish_site(
    folder_path: &str,
    site_name: &str,
    data_dir: &Path,
) -> Result<PublishResult, String> {
    let folder = Path::new(folder_path);

    // ── Step 1: Validate ──

    validate_site_name(site_name)?;

    if !folder.is_dir() {
        return Err(format!("'{}' is not a directory", folder_path));
    }

    // Check that there's at least one HTML file
    let has_html = has_html_file(folder)?;
    if !has_html {
        return Err("Folder must contain at least one .html file".to_string());
    }

    // ── Step 2: Build the Merkle DAG ──
    // This chunks every file, hashes everything, and builds the tree structure.

    let dag_result = merkle::build_dag(folder)?;

    // ── Step 3: Save chunks to disk ──

    let chunk_store = ChunkStore::new(data_dir)?;
    for chunk in &dag_result.chunks {
        chunk_store.save(&chunk.cid, &chunk.data)?;
    }

    // ── Step 4: Save metadata to SQLite ──

    let db = Database::open(data_dir)?;

    // Save each chunk record
    for chunk in &dag_result.chunks {
        db.insert_chunk(&chunk.cid, chunk.size as i64, true)?; // pinned = true (we published it)
    }

    // Save each DAG node
    for node in &dag_result.nodes {
        let links_json = serde_json::to_string(&node.links)
            .map_err(|e| format!("Failed to serialize links: {}", e))?;
        db.insert_dag_node(&node.cid, &node.name, &node.node_type, node.size as i64, &links_json)?;
    }

    // Save the site record
    let now = chrono::Local::now().to_rfc3339();
    db.insert_site(
        site_name,
        &dag_result.root_cid,
        dag_result.total_size as i64,
        dag_result.chunk_count as i32,
        dag_result.file_count as i32,
        true, // is_local = true (we're the publisher)
        &now,
        "local", // peer ID will be set once networking is added
    )?;

    // ── Done ──

    Ok(PublishResult {
        site_name: site_name.to_string(),
        root_cid: dag_result.root_cid,
        total_size: dag_result.total_size,
        chunk_count: dag_result.chunk_count,
        file_count: dag_result.file_count,
    })
}

/// Site name rules: lowercase letters, numbers, and hyphens. 3-63 chars.
/// Must start and end with a letter or number (not a hyphen).
fn validate_site_name(name: &str) -> Result<(), String> {
    if name.len() < 3 || name.len() > 63 {
        return Err("Site name must be 3-63 characters".to_string());
    }

    if name.starts_with('-') || name.ends_with('-') {
        return Err("Site name cannot start or end with a hyphen".to_string());
    }

    for ch in name.chars() {
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' {
            return Err(format!(
                "Site name can only contain lowercase letters, numbers, and hyphens. Found: '{}'",
                ch
            ));
        }
    }

    Ok(())
}

/// Check if a folder contains at least one .html file (recursively).
fn has_html_file(dir: &Path) -> Result<bool, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "html" || ext == "htm" {
                    return Ok(true);
                }
            }
        } else if path.is_dir() {
            if has_html_file(&path)? {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_site(test_name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let site_dir = std::env::temp_dir().join(format!("chimera_pub_site_{}", test_name));
        let data_dir = std::env::temp_dir().join(format!("chimera_pub_data_{}", test_name));

        let _ = fs::remove_dir_all(&site_dir);
        let _ = fs::remove_dir_all(&data_dir);

        // Create a small test site
        fs::create_dir_all(site_dir.join("css")).unwrap();
        fs::write(site_dir.join("index.html"), "<h1>Hello</h1>").unwrap();
        fs::write(site_dir.join("css").join("style.css"), "body {}").unwrap();

        fs::create_dir_all(&data_dir).unwrap();

        (site_dir, data_dir)
    }

    #[test]
    fn test_publish_site_works() {
        let (site_dir, data_dir) = create_test_site("basic");

        let result = publish_site(
            site_dir.to_str().unwrap(),
            "my-test-site",
            &data_dir,
        ).unwrap();

        // Check the result
        assert_eq!(result.site_name, "my-test-site");
        assert!(!result.root_cid.is_empty());
        assert_eq!(result.file_count, 2);
        assert_eq!(result.chunk_count, 2);
        assert!(result.total_size > 0);

        // Verify chunks are saved on disk
        let chunk_store = ChunkStore::new(&data_dir).unwrap();
        assert!(chunk_store.has(&result.root_cid) == false); // root CID is a DAG node, not a chunk
        // But the actual chunk CIDs should exist on disk

        // Verify site is in the database
        let db = Database::open(&data_dir).unwrap();
        let site = db.get_site("my-test-site").unwrap().unwrap();
        assert_eq!(site.root_cid, result.root_cid);
        assert!(site.is_local);
        assert!(site.is_pinned);

        let _ = fs::remove_dir_all(&site_dir);
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn test_publish_creates_chunks_on_disk() {
        let (site_dir, data_dir) = create_test_site("chunks_on_disk");

        let result = publish_site(
            site_dir.to_str().unwrap(),
            "disk-test",
            &data_dir,
        ).unwrap();

        // Every chunk should exist on the filesystem
        let chunk_store = ChunkStore::new(&data_dir).unwrap();
        let db = Database::open(&data_dir).unwrap();
        let chunks = db.get_all_chunks().unwrap();

        assert_eq!(chunks.len(), result.chunk_count as usize);
        for chunk in &chunks {
            assert!(chunk_store.has(&chunk.cid));
        }

        let _ = fs::remove_dir_all(&site_dir);
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn test_invalid_site_name() {
        let (site_dir, data_dir) = create_test_site("bad_name");

        // Too short
        assert!(publish_site(site_dir.to_str().unwrap(), "ab", &data_dir).is_err());

        // Uppercase
        assert!(publish_site(site_dir.to_str().unwrap(), "MyBadSite", &data_dir).is_err());

        // Starts with hyphen
        assert!(publish_site(site_dir.to_str().unwrap(), "-bad", &data_dir).is_err());

        // Contains space
        assert!(publish_site(site_dir.to_str().unwrap(), "bad site", &data_dir).is_err());

        let _ = fs::remove_dir_all(&site_dir);
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn test_folder_must_have_html() {
        let site_dir = std::env::temp_dir().join("chimera_pub_site_nohtml");
        let data_dir = std::env::temp_dir().join("chimera_pub_data_nohtml");
        let _ = fs::remove_dir_all(&site_dir);
        let _ = fs::remove_dir_all(&data_dir);

        fs::create_dir_all(&site_dir).unwrap();
        fs::write(site_dir.join("readme.txt"), "no html here").unwrap();
        fs::create_dir_all(&data_dir).unwrap();

        let result = publish_site(site_dir.to_str().unwrap(), "no-html", &data_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("html"));

        let _ = fs::remove_dir_all(&site_dir);
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn test_validate_site_name() {
        // Valid names
        assert!(validate_site_name("my-site").is_ok());
        assert!(validate_site_name("hello123").is_ok());
        assert!(validate_site_name("abc").is_ok());

        // Invalid names
        assert!(validate_site_name("ab").is_err());           // too short
        assert!(validate_site_name("AB").is_err());            // uppercase
        assert!(validate_site_name("-bad").is_err());          // starts with hyphen
        assert!(validate_site_name("bad-").is_err());          // ends with hyphen
        assert!(validate_site_name("has space").is_err());     // space
        assert!(validate_site_name("has_under").is_err());     // underscore
    }
}
