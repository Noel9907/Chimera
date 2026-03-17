use cid::Cid;
use multihash_codetable::{Code, MultihashDigest};

// Multicodec codes - these tell the CID what type of content it points to
const RAW: u64 = 0x55;       // raw bytes (used for file chunks)
const DAG_JSON: u64 = 0x0129; // JSON structure (used for DAG nodes)

/// Create a CID string for a raw chunk.
/// Takes the raw bytes of a chunk, hashes them, returns a CID string.
///
/// Example: "bafkreigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
pub fn cid_from_bytes(data: &[u8]) -> String {
    // 1. Hash the data with SHA-256 (wrapped in multihash format)
    let hash = Code::Sha2_256.digest(data);

    // 2. Create CIDv1 with codec=raw (because this is raw chunk data)
    let cid = Cid::new_v1(RAW, hash);

    // 3. Convert to string (base32lower by default)
    cid.to_string()
}

/// Create a CID string for a DAG node (JSON structure).
/// Takes the serialized JSON bytes of a DAG node, returns a CID string.
pub fn cid_from_dag_json(json_bytes: &[u8]) -> String {
    let hash = Code::Sha2_256.digest(json_bytes);
    let cid = Cid::new_v1(DAG_JSON, hash);
    cid.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_from_bytes() {
        let cid = cid_from_bytes(b"hello world");
        // Should start with "baf" (base32, CIDv1)
        assert!(cid.starts_with("baf"));
        // Should be consistent - same input = same CID
        assert_eq!(cid, cid_from_bytes(b"hello world"));
    }

    #[test]
    fn test_different_data_different_cid() {
        let cid_a = cid_from_bytes(b"hello");
        let cid_b = cid_from_bytes(b"world");
        assert_ne!(cid_a, cid_b);
    }

    #[test]
    fn test_cid_from_dag_json() {
        let json = b"{\"name\": \"index.html\"}";
        let cid = cid_from_dag_json(json);
        // CIDv1 base32 strings always start with 'b'
        assert!(cid.starts_with('b'));
    }

    #[test]
    fn test_raw_and_dag_cids_differ() {
        // Same bytes but different codecs should produce different CIDs
        let data = b"same data";
        let raw_cid = cid_from_bytes(data);
        let dag_cid = cid_from_dag_json(data);
        assert_ne!(raw_cid, dag_cid);
    }
}
