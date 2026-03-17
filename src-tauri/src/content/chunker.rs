use sha2::{Sha256, Digest};
use std::fs;
use std::path::Path;

/// 256 KB chunk size
pub const CHUNK_SIZE: usize = 256 * 1024;

/// A piece of a file, identified by its SHA-256 hash.
pub struct Chunk {
    pub hash: Vec<u8>,      // raw 32-byte SHA-256 hash
    pub hash_hex: String,   // hex-encoded hash (for filenames, display)
    pub data: Vec<u8>,      // raw bytes of this chunk
    pub size: usize,
}

/// Read a file and split it into 256KB chunks.
/// Each chunk gets its SHA-256 hash computed.
pub fn chunk_file(path: &Path) -> Result<Vec<Chunk>, String> {
    let data = fs::read(path)
        .map_err(|e| format!("Failed to read file {}: {}", path.display(), e))?;

    let chunks = chunk_bytes(&data);
    Ok(chunks)
}

/// Split raw bytes into 256KB chunks with SHA-256 hashes.
pub fn chunk_bytes(data: &[u8]) -> Vec<Chunk> {
    let mut chunks = Vec::new();

    // if file is empty, produce one empty chunk
    if data.is_empty() {
        let hash = sha256_hash(&[]);
        let hash_hex = hex_encode(&hash);
        chunks.push(Chunk {
            hash,
            hash_hex,
            data: Vec::new(),
            size: 0,
        });
        return chunks;
    }

    for piece in data.chunks(CHUNK_SIZE) {
        let hash = sha256_hash(piece);
        let hash_hex = hex_encode(&hash);
        chunks.push(Chunk {
            hash,
            hash_hex,
            data: piece.to_vec(),
            size: piece.len(),
        });
    }

    chunks
}

/// SHA-256 hash some bytes. Returns 32 bytes.
pub fn sha256_hash(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Turn bytes into a hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_file_one_chunk() {
        let data = b"hello world";
        let chunks = chunk_bytes(data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].size, 11);
        assert_eq!(chunks[0].data, b"hello world");
        // hash should be 64 hex chars (32 bytes)
        assert_eq!(chunks[0].hash_hex.len(), 64);
    }

    #[test]
    fn test_exact_chunk_size() {
        let data = vec![0u8; CHUNK_SIZE];
        let chunks = chunk_bytes(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].size, CHUNK_SIZE);
    }

    #[test]
    fn test_multiple_chunks() {
        // 2.5 chunks worth of data
        let data = vec![42u8; CHUNK_SIZE * 2 + 1000];
        let chunks = chunk_bytes(&data);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].size, CHUNK_SIZE);
        assert_eq!(chunks[1].size, CHUNK_SIZE);
        assert_eq!(chunks[2].size, 1000);
    }

    #[test]
    fn test_empty_file() {
        let chunks = chunk_bytes(&[]);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].size, 0);
    }

    #[test]
    fn test_same_data_same_hash() {
        let chunks_a = chunk_bytes(b"same content");
        let chunks_b = chunk_bytes(b"same content");
        assert_eq!(chunks_a[0].hash_hex, chunks_b[0].hash_hex);
    }

    #[test]
    fn test_different_data_different_hash() {
        let chunks_a = chunk_bytes(b"content A");
        let chunks_b = chunk_bytes(b"content B");
        assert_ne!(chunks_a[0].hash_hex, chunks_b[0].hash_hex);
    }
}
