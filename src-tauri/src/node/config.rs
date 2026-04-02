// ── Node Configuration ──
//
// Settings the P2P node needs to start up.
// Where to store data, what peers to connect to, what ports to use.

use std::path::PathBuf;

/// All the settings for a Chimera P2P node.
pub struct NodeConfig {
    /// Where to store everything (~/.chimera/ by default)
    pub data_dir: PathBuf,

    /// Bootstrap nodes to connect to on startup.
    /// These are addresses of known peers (like our relay server).
    /// Format: "/ip4/1.2.3.4/tcp/4001/p2p/12D3KooW..."
    pub bootstrap_nodes: Vec<String>,

    /// TCP port to listen on (0 = let the OS pick a random available port)
    pub tcp_port: u16,
}

impl NodeConfig {
    /// Create a config with sensible defaults.
    /// Data goes in ~/.chimera/, no bootstrap nodes, random port.
    pub fn default_config() -> Self {
        // dirs::home_dir() gives us the user's home folder
        // e.g., C:\Users\noel on Windows, /home/noel on Linux
        let data_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".chimera");

        NodeConfig {
            data_dir,
            bootstrap_nodes: vec![
                // EC2 relay server (bootstrap + circuit relay)
                "/ip4/44.192.53.77/tcp/4001/p2p/12D3KooWJkrcsL6Dt8fDTKiRLJFU8V143Wd9mR1PMSQw2NjgCZrJ".to_string(),
            ],
            tcp_port: 0, // random port
        }
    }

    /// Path to the keypair file (the node's persistent identity)
    pub fn keypair_path(&self) -> PathBuf {
        self.data_dir.join("identity").join("keypair.bin")
    }

    /// Path to the chunks directory
    pub fn chunks_dir(&self) -> PathBuf {
        self.data_dir.join("chunks")
    }

    /// Path to the SQLite database
    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("chimera.db")
    }
}
