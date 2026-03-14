use std::fs;
use std::path::{Path, PathBuf};

/// Manages reading/writing chunk blobs on the filesystem.
///
/// Chunks are stored as files named by their CID, inside sharded subdirectories.
/// Example: chunk with CID "bafkreigxyz..." gets stored at:
///   <base_dir>/chunks/ba/bafkreigxyz...
///
/// The "ba" subdirectory (first 2 chars of CID) prevents any single folder
/// from having too many files, which would slow down the filesystem.
pub struct ChunkStore {
    chunks_dir: PathBuf,
}

impl ChunkStore {
    /// Create a new ChunkStore. Creates the chunks directory if it doesn't exist.
    pub fn new(base_dir: &Path) -> Result<Self, String> {
        let chunks_dir = base_dir.join("chunks");
        fs::create_dir_all(&chunks_dir)
            .map_err(|e| format!("Failed to create chunks dir: {}", e))?;

        Ok(ChunkStore { chunks_dir })
    }

    /// Save a chunk to disk. Skips if it already exists (deduplication).
    pub fn save(&self, cid: &str, data: &[u8]) -> Result<(), String> {
        let path = self.chunk_path(cid);

        // Deduplication: if this CID already exists, skip writing
        if path.exists() {
            return Ok(());
        }

        // Create the shard subdirectory (e.g., "chunks/ba/")
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create shard dir: {}", e))?;
        }

        fs::write(&path, data)
            .map_err(|e| format!("Failed to write chunk {}: {}", cid, e))?;

        Ok(())
    }

    /// Load a chunk from disk. Returns the raw bytes.
    pub fn load(&self, cid: &str) -> Result<Vec<u8>, String> {
        let path = self.chunk_path(cid);
        fs::read(&path)
            .map_err(|e| format!("Chunk {} not found: {}", cid, e))
    }

    /// Check if we have a chunk.
    pub fn has(&self, cid: &str) -> bool {
        self.chunk_path(cid).exists()
    }

    /// Delete a chunk from disk.
    pub fn delete(&self, cid: &str) -> Result<(), String> {
        let path = self.chunk_path(cid);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to delete chunk {}: {}", cid, e))?;
        }
        Ok(())
    }

    /// Get the full file path for a chunk CID.
    /// Uses first 2 chars of the CID as a shard directory.
    fn chunk_path(&self, cid: &str) -> PathBuf {
        // Take first 2 chars for sharding (e.g., "ba" from "bafkreig...")
        let shard = if cid.len() >= 2 { &cid[..2] } else { "xx" };
        self.chunks_dir.join(shard).join(cid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store(name: &str) -> (ChunkStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("chimera_chunk_test_{}", name));
        let _ = fs::remove_dir_all(&dir);
        let store = ChunkStore::new(&dir).unwrap();
        (store, dir)
    }

    #[test]
    fn test_save_and_load() {
        let (store, dir) = test_store("save_load");

        store.save("bafkreig_test_cid", b"hello chunk").unwrap();
        let data = store.load("bafkreig_test_cid").unwrap();
        assert_eq!(data, b"hello chunk");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_has() {
        let (store, dir) = test_store("has");

        assert!(!store.has("bafkreig_missing"));
        store.save("bafkreig_exists", b"data").unwrap();
        assert!(store.has("bafkreig_exists"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_deduplication() {
        let (store, dir) = test_store("dedup");

        // Saving the same CID twice should not error
        store.save("bafkreig_dup", b"data").unwrap();
        store.save("bafkreig_dup", b"data").unwrap();
        let data = store.load("bafkreig_dup").unwrap();
        assert_eq!(data, b"data");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_delete() {
        let (store, dir) = test_store("delete");

        store.save("bafkreig_del", b"data").unwrap();
        assert!(store.has("bafkreig_del"));

        store.delete("bafkreig_del").unwrap();
        assert!(!store.has("bafkreig_del"));

        // Deleting something that doesn't exist should be fine
        store.delete("bafkreig_nonexistent").unwrap();

        let _ = fs::remove_dir_all(&dir);
    }
}
