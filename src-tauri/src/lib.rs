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
            app.manage(NodeHandle::new(cmd_tx));

            tauri::async_runtime::spawn(async move {
                match start_node(config, cmd_rx).await {
                    Ok(()) => {}
                    Err(e) => tracing::error!("Node failed to start: {}", e),
                }
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

    // Check if first segment is a known site (check local DB)
    let data_dir = get_data_dir();
    let is_known_site = {
        storage::database::Database::open(&data_dir)
            .ok()
            .and_then(|d| d.get_site(first_segment).ok().flatten())
            .is_some()
    };

    let (site_name, file_path) = if is_known_site {
        // Remember this as the current site
        let current: CurrentSite = app.state::<CurrentSite>().inner().clone();
        *current.lock().unwrap() = first_segment.to_string();

        let fp = if rest.is_empty() { "/index.html".to_string() } else { format!("/{}", rest) };
        (first_segment.to_string(), fp)
    } else {
        // Not a known site — treat the whole path as a file path under the current site
        let current: CurrentSite = app.state::<CurrentSite>().inner().clone();
        let site = current.lock().unwrap().clone();
        if site.is_empty() {
            return error_response(404, "No site context for this request");
        }
        (site, format!("/{}", path))
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
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".chimera")
}

async fn start_node(
    config: NodeConfig,
    cmd_rx: mpsc::Receiver<NodeCommand>,
) -> Result<(), String> {
    let mut swarm = node::swarm::create_swarm(&config)?;
    node::swarm::start_listening(&mut swarm, &config)?;

    tracing::info!("Chimera node started");

    let data_dir = config.data_dir;
    node::event_loop::run_event_loop(swarm, cmd_rx, data_dir).await;
    Ok(())
}
