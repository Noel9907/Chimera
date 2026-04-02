pub mod content;
pub mod storage;
pub mod publisher;
pub mod retriever;
pub mod network;
pub mod node;
pub mod ipc;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::Manager;
use tokio::sync::mpsc;
use node::config::NodeConfig;
use node::handle::{NodeCommand, NodeHandle};

/// Tracks the current chimera site being browsed so absolute paths
/// (like /assets/foo.js) can be resolved to the correct site.
type CurrentSite = Arc<Mutex<String>>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging so tracing::info!() etc. actually print to the console.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // ── Custom protocol for serving chimera site content ──
        //
        // When the webview loads chimera-content://localhost/site-name/file.css,
        // this handler fetches the file from the P2P layer (local cache or network).
        // Relative URLs in HTML (like <link href="style.css">) resolve naturally
        // because the browser treats it as a normal origin.
        .register_asynchronous_uri_scheme_protocol("chimera-content", |ctx, request, responder| {
            let app = ctx.app_handle().clone();

            tauri::async_runtime::spawn(async move {
                let response = serve_chimera_content(&app, request.uri().path()).await;
                responder.respond(response);
            });
        })
        .setup(|app| {
            let config = NodeConfig::default_config();
            std::fs::create_dir_all(&config.data_dir).ok();

            app.manage(CurrentSite::default());

            let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(32);
            let handle = NodeHandle::new(cmd_tx);
            app.manage(handle.clone());

            let data_dir_for_reannounce = config.data_dir.clone();
            let handle_for_reannounce = handle.clone();

            tauri::async_runtime::spawn(async move {
                match start_node(config, cmd_rx).await {
                    Ok(()) => {}
                    Err(e) => tracing::error!("Node failed to start: {}", e),
                }
            });

            // Re-announce all locally published sites after the node has time to bootstrap.
            // DHT records live in memory on the relay — they're lost when the relay restarts
            // or when we disconnect. This ensures our sites are always findable.
            tauri::async_runtime::spawn(async move {
                // Wait for connection + Kademlia bootstrap to finish
                tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                reannounce_local_sites(&handle_for_reannounce, &data_dir_for_reannounce).await;
            });

            tracing::info!("Chimera node starting...");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::commands::publish_site,
            ipc::commands::get_published_sites,
            ipc::commands::unpublish_site,
            ipc::commands::navigate,
            ipc::commands::fetch_file,
            ipc::commands::get_node_id,
            ipc::commands::get_peer_count,
            ipc::commands::browser_navigate,
            ipc::commands::browser_hide,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Serve a file from a chimera site via the custom protocol.
///
/// Path format: /site-name/path/to/file
/// e.g., /sample/index.html or /sample/css/style.css
///
/// If the first path segment isn't a known site, we assume it's a resource
/// (like /assets/foo.js) belonging to the last-browsed site.
async fn serve_chimera_content(
    app: &tauri::AppHandle,
    path: &str,
) -> tauri::http::Response<Vec<u8>> {
    let path = path.trim_start_matches('/');

    if path.is_empty() {
        return error_response(404, "No site name in URL");
    }

    // Split into first segment and rest
    let (first_segment, rest) = match path.split_once('/') {
        Some((s, r)) => (s, r),
        None => (path, ""),
    };

    let data_dir = get_data_dir();

    // Try the first segment as a site name. It might be in our local DB (cached/published)
    // or it might be a new site we need to resolve from the DHT — the retriever handles both.
    // Only fall back to "current site" context if the retriever can't find it either.
    let (site_name, file_path) = {
        let fp = if rest.is_empty() { "/index.html".to_string() } else { format!("/{}", rest) };

        // Quick check: is it a plausible site name? (lowercase, hyphens, 3+ chars)
        let looks_like_site = first_segment.len() >= 3
            && first_segment.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');

        if looks_like_site {
            // Remember as current site so relative paths (like /assets/foo.js) resolve correctly
            let current: CurrentSite = app.state::<CurrentSite>().inner().clone();
            *current.lock().unwrap() = first_segment.to_string();
            (first_segment.to_string(), fp)
        } else {
            // Not a site name — treat as a file path under the current site
            let current: CurrentSite = app.state::<CurrentSite>().inner().clone();
            let site = current.lock().unwrap().clone();
            if site.is_empty() {
                return error_response(404, "No site context for this request");
            }
            (site, format!("/{}", path))
        }
    };

    let handle: NodeHandle = app.state::<NodeHandle>().inner().clone();

    match retriever::pipeline::retrieve_file(&site_name, &file_path, &handle, &data_dir).await {
        Ok(file) => {
            tauri::http::Response::builder()
                .status(200)
                .header("Content-Type", &file.content_type)
                .body(file.body)
                .unwrap()
        }
        Err(e) => {
            tracing::warn!("Failed to serve {}{}: {}", site_name, file_path, e);
            error_response(404, &e)
        }
    }
}

fn error_response(status: u16, msg: &str) -> tauri::http::Response<Vec<u8>> {
    let body = format!(
        "<html><body><h1>Error {}</h1><p>{}</p></body></html>",
        status, msg
    );
    tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", "text/html")
        .body(body.into_bytes())
        .unwrap()
}

fn get_data_dir() -> PathBuf {
    std::env::var("CHIMERA_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".chimera")
        })
}

/// Re-announce all locally published sites to the DHT.
/// Called on startup after bootstrap so the relay always has our site records.
async fn reannounce_local_sites(handle: &NodeHandle, data_dir: &std::path::Path) {
    let db = match storage::database::Database::open(data_dir) {
        Ok(db) => db,
        Err(e) => {
            tracing::warn!("Failed to open DB for re-announce: {}", e);
            return;
        }
    };

    let sites = match db.get_local_sites() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to get local sites: {}", e);
            return;
        }
    };

    if sites.is_empty() {
        tracing::info!("No local sites to re-announce");
        return;
    }

    for site in &sites {
        tracing::info!("Re-announcing site '{}' to DHT (root_cid={})", site.name, site.root_cid);
        match handle.announce_site(
            site.name.clone(),
            site.root_cid.clone(),
            site.total_size as u64,
            site.chunk_count as u32,
            site.published_at.clone(),
        ).await {
            Ok(()) => tracing::info!("Successfully re-announced '{}'", site.name),
            Err(e) => tracing::warn!("Failed to re-announce '{}': {}", site.name, e),
        }
    }
}

async fn start_node(
    config: NodeConfig,
    cmd_rx: mpsc::Receiver<NodeCommand>,
) -> Result<(), String> {
    let mut swarm = node::swarm::create_swarm(&config)?;
    node::swarm::start_listening(&mut swarm, &config)?;

    tracing::info!("Chimera node started");

    let data_dir = config.data_dir;
    let bootstrap_nodes = config.bootstrap_nodes;
    node::event_loop::run_event_loop(swarm, cmd_rx, data_dir, bootstrap_nodes).await;
    Ok(())
}
