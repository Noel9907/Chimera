use rusqlite::{Connection, params};
use std::path::Path;

/// Manages the SQLite database for metadata.
///
/// Stores info about chunks, DAG nodes, and sites.
/// The actual chunk bytes live on the filesystem (ChunkStore) —
/// this just tracks what we have and how it's organized.
pub struct Database {
    conn: Connection,
}

// ── Data structs for reading from the database ──

pub struct ChunkRecord {
    pub cid: String,
    pub size: i64,
    pub is_pinned: bool,
}

pub struct DagNodeRecord {
    pub cid: String,
    pub name: String,
    pub node_type: String,
    pub size: i64,
    pub links_json: String, // JSON string of [{cid, name, size}, ...]
}

pub struct SiteRecord {
    pub name: String,
    pub root_cid: String,
    pub total_size: i64,
    pub chunk_count: i32,
    pub file_count: i32,
    pub is_local: bool,
    pub is_pinned: bool,
    pub published_at: String,
    pub publisher_peer_id: String,
}

impl Database {
    /// Open (or create) the database at the given directory.
    /// Creates all tables if they don't exist yet.
    pub fn open(base_dir: &Path) -> Result<Self, String> {
        let db_path = base_dir.join("chimera.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;

        let db = Database { conn };
        db.create_tables()?;
        Ok(db)
    }

    /// Create all tables. Safe to call multiple times (uses IF NOT EXISTS).
    fn create_tables(&self) -> Result<(), String> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS chunks (
                cid TEXT PRIMARY KEY,
                size INTEGER NOT NULL,
                is_pinned INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS dag_nodes (
                cid TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                node_type TEXT NOT NULL,
                size INTEGER NOT NULL,
                links_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sites (
                name TEXT PRIMARY KEY,
                root_cid TEXT NOT NULL,
                total_size INTEGER NOT NULL,
                chunk_count INTEGER NOT NULL,
                file_count INTEGER NOT NULL,
                is_local INTEGER NOT NULL DEFAULT 0,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                published_at TEXT NOT NULL,
                publisher_peer_id TEXT NOT NULL
            );
        ").map_err(|e| format!("Failed to create tables: {}", e))?;

        Ok(())
    }

    // ── Chunk operations ──

    /// Record that we have a chunk. Skips if already exists.
    pub fn insert_chunk(&self, cid: &str, size: i64, is_pinned: bool) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR IGNORE INTO chunks (cid, size, is_pinned) VALUES (?1, ?2, ?3)",
            params![cid, size, is_pinned as i32],
        ).map_err(|e| format!("Failed to insert chunk: {}", e))?;
        Ok(())
    }

    /// Check if we have a chunk recorded.
    pub fn has_chunk(&self, cid: &str) -> Result<bool, String> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE cid = ?1",
            params![cid],
            |row| row.get(0),
        ).map_err(|e| format!("Failed to query chunk: {}", e))?;
        Ok(count > 0)
    }

    /// Get all chunk records.
    pub fn get_all_chunks(&self) -> Result<Vec<ChunkRecord>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT cid, size, is_pinned FROM chunks"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt.query_map([], |row| {
            Ok(ChunkRecord {
                cid: row.get(0)?,
                size: row.get(1)?,
                is_pinned: row.get::<_, i32>(2)? != 0,
            })
        }).map_err(|e| format!("Failed to query chunks: {}", e))?;

        let mut chunks = Vec::new();
        for row in rows {
            chunks.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }
        Ok(chunks)
    }

    // ── DAG node operations ──

    /// Save a DAG node. Skips if already exists.
    pub fn insert_dag_node(
        &self,
        cid: &str,
        name: &str,
        node_type: &str,
        size: i64,
        links_json: &str,
    ) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR IGNORE INTO dag_nodes (cid, name, node_type, size, links_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![cid, name, node_type, size, links_json],
        ).map_err(|e| format!("Failed to insert dag node: {}", e))?;
        Ok(())
    }

    /// Get a DAG node by CID.
    pub fn get_dag_node(&self, cid: &str) -> Result<Option<DagNodeRecord>, String> {
        let result = self.conn.query_row(
            "SELECT cid, name, node_type, size, links_json FROM dag_nodes WHERE cid = ?1",
            params![cid],
            |row| {
                Ok(DagNodeRecord {
                    cid: row.get(0)?,
                    name: row.get(1)?,
                    node_type: row.get(2)?,
                    size: row.get(3)?,
                    links_json: row.get(4)?,
                })
            },
        );

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to query dag node: {}", e)),
        }
    }

    // ── Site operations ──

    /// Record a published site.
    pub fn insert_site(
        &self,
        name: &str,
        root_cid: &str,
        total_size: i64,
        chunk_count: i32,
        file_count: i32,
        is_local: bool,
        published_at: &str,
        publisher_peer_id: &str,
    ) -> Result<(), String> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sites
             (name, root_cid, total_size, chunk_count, file_count, is_local, is_pinned, published_at, publisher_peer_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                name, root_cid, total_size, chunk_count, file_count,
                is_local as i32, is_local as i32, // local sites are auto-pinned
                published_at, publisher_peer_id
            ],
        ).map_err(|e| format!("Failed to insert site: {}", e))?;
        Ok(())
    }

    /// Get a site by name.
    pub fn get_site(&self, name: &str) -> Result<Option<SiteRecord>, String> {
        let result = self.conn.query_row(
            "SELECT name, root_cid, total_size, chunk_count, file_count,
                    is_local, is_pinned, published_at, publisher_peer_id
             FROM sites WHERE name = ?1",
            params![name],
            |row| {
                Ok(SiteRecord {
                    name: row.get(0)?,
                    root_cid: row.get(1)?,
                    total_size: row.get(2)?,
                    chunk_count: row.get(3)?,
                    file_count: row.get(4)?,
                    is_local: row.get::<_, i32>(5)? != 0,
                    is_pinned: row.get::<_, i32>(6)? != 0,
                    published_at: row.get(7)?,
                    publisher_peer_id: row.get(8)?,
                })
            },
        );

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to query site: {}", e)),
        }
    }

    /// Get all locally published sites.
    pub fn get_local_sites(&self) -> Result<Vec<SiteRecord>, String> {
        let mut stmt = self.conn.prepare(
            "SELECT name, root_cid, total_size, chunk_count, file_count,
                    is_local, is_pinned, published_at, publisher_peer_id
             FROM sites WHERE is_local = 1"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt.query_map([], |row| {
            Ok(SiteRecord {
                name: row.get(0)?,
                root_cid: row.get(1)?,
                total_size: row.get(2)?,
                chunk_count: row.get(3)?,
                file_count: row.get(4)?,
                is_local: row.get::<_, i32>(5)? != 0,
                is_pinned: row.get::<_, i32>(6)? != 0,
                published_at: row.get(7)?,
                publisher_peer_id: row.get(8)?,
            })
        }).map_err(|e| format!("Failed to query sites: {}", e))?;

        let mut sites = Vec::new();
        for row in rows {
            sites.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }
        Ok(sites)
    }

    /// Update the publisher_peer_id for a site (replaces "local" placeholder with real PeerId).
    pub fn update_site_peer_id(&self, name: &str, peer_id: &str) -> Result<(), String> {
        self.conn.execute(
            "UPDATE sites SET publisher_peer_id = ?1 WHERE name = ?2",
            params![peer_id, name],
        ).map_err(|e| format!("Failed to update site peer_id: {}", e))?;
        Ok(())
    }

    /// Delete a site by name.
    pub fn delete_site(&self, name: &str) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM sites WHERE name = ?1",
            params![name],
        ).map_err(|e| format!("Failed to delete site: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_db(name: &str) -> (Database, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("chimera_db_test_{}", name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let db = Database::open(&dir).unwrap();
        (db, dir)
    }

    #[test]
    fn test_chunk_insert_and_query() {
        let (db, dir) = test_db("chunks");

        assert!(!db.has_chunk("cid_abc").unwrap());
        db.insert_chunk("cid_abc", 1024, false).unwrap();
        assert!(db.has_chunk("cid_abc").unwrap());

        let chunks = db.get_all_chunks().unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].cid, "cid_abc");
        assert_eq!(chunks[0].size, 1024);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_chunk_deduplication() {
        let (db, dir) = test_db("chunk_dedup");

        // Inserting same CID twice should not error (INSERT OR IGNORE)
        db.insert_chunk("cid_dup", 100, false).unwrap();
        db.insert_chunk("cid_dup", 100, false).unwrap();

        let chunks = db.get_all_chunks().unwrap();
        assert_eq!(chunks.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_dag_node_insert_and_query() {
        let (db, dir) = test_db("dag");

        db.insert_dag_node("cid_node", "index.html", "file", 500, "[]").unwrap();
        let node = db.get_dag_node("cid_node").unwrap().unwrap();
        assert_eq!(node.name, "index.html");
        assert_eq!(node.node_type, "file");
        assert_eq!(node.size, 500);

        // Missing node returns None
        let missing = db.get_dag_node("cid_nope").unwrap();
        assert!(missing.is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_site_insert_and_query() {
        let (db, dir) = test_db("sites");

        db.insert_site(
            "my-site", "cid_root", 5000, 10, 3,
            true, "2026-03-14T12:00:00Z", "peer123",
        ).unwrap();

        let site = db.get_site("my-site").unwrap().unwrap();
        assert_eq!(site.root_cid, "cid_root");
        assert_eq!(site.total_size, 5000);
        assert!(site.is_local);
        assert!(site.is_pinned); // local sites are auto-pinned

        let local = db.get_local_sites().unwrap();
        assert_eq!(local.len(), 1);

        // Missing site returns None
        assert!(db.get_site("nope").unwrap().is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_site_delete() {
        let (db, dir) = test_db("site_del");

        db.insert_site(
            "del-me", "cid_root", 100, 1, 1,
            true, "2026-03-14T12:00:00Z", "peer123",
        ).unwrap();
        assert!(db.get_site("del-me").unwrap().is_some());

        db.delete_site("del-me").unwrap();
        assert!(db.get_site("del-me").unwrap().is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
