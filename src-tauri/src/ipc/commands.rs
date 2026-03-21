// ── Tauri IPC Commands ──
//
// These are the functions the React frontend can call via:
//   import { invoke } from "@tauri-apps/api/core";
//   const result = await invoke("publish_site", { folderPath: "...", siteName: "..." });
//
// Each #[tauri::command] function becomes available to the frontend.
// They return Result<T, String> — Ok = success data, Err = error message string.

use serde::Serialize;
use std::path::PathBuf;
use tauri::{Manager, State};

use crate::node::handle::NodeHandle;
use crate::publisher::pipeline;
use crate::retriever;
use crate::storage::database::Database;

/// Get the data directory (~/.chimera/).
/// Used by all commands that need to access storage.
fn get_data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".chimera")
}

// ── Return types ──
// These need #[derive(Serialize)] so Tauri can send them to the frontend as JSON.

#[derive(Serialize)]
pub struct PublishResult {
    pub site_name: String,
    pub root_cid: String,
    pub total_size: u64,
    pub chunk_count: u32,
    pub file_count: u32,
}

#[derive(Serialize)]
pub struct SiteInfo {
    pub name: String,
    pub root_cid: String,
    pub total_size: i64,
    pub chunk_count: i32,
    pub file_count: i32,
    pub published_at: String,
    pub is_local: bool,
    pub is_pinned: bool,
}

/// Content returned for chimera:// URLs.
#[derive(Serialize)]
pub struct WebContent {
    pub content_type: String,
    pub body: Vec<u8>,
    pub file_path: String,
}

// ═══════════════════════════════════════════════════════════════════
// Publishing commands
// ═══════════════════════════════════════════════════════════════════

/// Publish a static site from a local folder.
/// Stores files locally AND announces to the DHT so other peers can find it.
/// Frontend calls: invoke("publish_site", { folderPath: "...", siteName: "..." })
#[tauri::command]
pub async fn publish_site(
    folder_path: String,
    site_name: String,
    handle: State<'_, NodeHandle>,
) -> Result<PublishResult, String> {
    let data_dir = get_data_dir();
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    // Store files locally (chunk, build DAG, save to disk + SQLite)
    let result = pipeline::publish_site(&folder_path, &site_name, &data_dir)?;

    // Announce to the DHT so other peers can find this site.
    // Best-effort: if there are no peers yet, this fails silently.
    // The site is still published locally and browseable on this machine.
    let published_at = chrono::Utc::now().to_rfc3339();
    if let Err(e) = handle
        .announce_site(
            result.site_name.clone(),
            result.root_cid.clone(),
            result.total_size,
            result.chunk_count,
            published_at,
        )
        .await
    {
        tracing::warn!("DHT announce skipped (no peers?): {}", e);
    }

    Ok(PublishResult {
        site_name: result.site_name,
        root_cid: result.root_cid,
        total_size: result.total_size,
        chunk_count: result.chunk_count,
        file_count: result.file_count,
    })
}

/// Get all locally published sites.
/// Frontend calls: invoke("get_published_sites")
#[tauri::command]
pub fn get_published_sites() -> Result<Vec<SiteInfo>, String> {
    let data_dir = get_data_dir();
    if !data_dir.exists() {
        return Ok(Vec::new());
    }

    let db = Database::open(&data_dir)?;
    let sites = db.get_local_sites()?;

    Ok(sites
        .into_iter()
        .map(|s| SiteInfo {
            name: s.name,
            root_cid: s.root_cid,
            total_size: s.total_size,
            chunk_count: s.chunk_count,
            file_count: s.file_count,
            published_at: s.published_at,
            is_local: s.is_local,
            is_pinned: s.is_pinned,
        })
        .collect())
}

/// Delete/unpublish a site by name.
/// Frontend calls: invoke("unpublish_site", { siteName: "my-site" })
#[tauri::command]
pub fn unpublish_site(site_name: String) -> Result<(), String> {
    let data_dir = get_data_dir();
    let db = Database::open(&data_dir)?;
    db.delete_site(&site_name)?;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════
// Browser commands
// ═══════════════════════════════════════════════════════════════════

/// Navigate to a URL. For chimera:// URLs, fetches content from the P2P network.
/// For http/https URLs, returns None (frontend handles those via webview).
/// Frontend calls: invoke("navigate", { url: "chimera://my-site/about.html" })
#[tauri::command]
pub async fn navigate(
    url: String,
    handle: State<'_, NodeHandle>,
) -> Result<Option<WebContent>, String> {
    // Only handle chimera:// URLs
    let stripped = match url.strip_prefix("chimera://") {
        Some(rest) => rest,
        None => return Ok(None), // http/https — frontend handles it
    };

    // Parse: "my-site/about.html" → site_name="my-site", file_path="/about.html"
    let (site_name, file_path) = match stripped.split_once('/') {
        Some((name, path)) => (name, format!("/{}", path)),
        None => (stripped, "/index.html".to_string()),
    };

    if site_name.is_empty() {
        return Err("No site name in URL".to_string());
    }

    let data_dir = get_data_dir();
    let result = retriever::pipeline::retrieve_file(
        site_name, &file_path, &handle, &data_dir,
    ).await?;

    Ok(Some(WebContent {
        content_type: result.content_type,
        body: result.body,
        file_path,
    }))
}

/// Fetch a specific file from a chimera site (for sub-resources like CSS, JS, images).
/// Frontend calls: invoke("fetch_file", { siteName: "my-site", filePath: "/css/style.css" })
#[tauri::command]
pub async fn fetch_file(
    site_name: String,
    file_path: String,
    handle: State<'_, NodeHandle>,
) -> Result<WebContent, String> {
    let data_dir = get_data_dir();
    let result = retriever::pipeline::retrieve_file(
        &site_name, &file_path, &handle, &data_dir,
    ).await?;

    Ok(WebContent {
        content_type: result.content_type,
        body: result.body,
        file_path,
    })
}

// ═══════════════════════════════════════════════════════════════════
// Network status commands
// ═══════════════════════════════════════════════════════════════════

/// Get this node's PeerId (our unique identity on the network).
/// Frontend calls: invoke("get_node_id")
#[tauri::command]
pub async fn get_node_id(handle: State<'_, NodeHandle>) -> Result<String, String> {
    handle.get_node_id().await
}

/// Get the number of currently connected peers.
/// Frontend calls: invoke("get_peer_count")
#[tauri::command]
pub async fn get_peer_count(handle: State<'_, NodeHandle>) -> Result<u32, String> {
    handle.get_peer_count().await
}

// ═══════════════════════════════════════════════════════════════════
// Browser webview commands
// ═══════════════════════════════════════════════════════════════════

/// Navigate the browser child webview to a URL.
/// Creates the webview on first call, reuses it after.
/// Frontend sends the exact bounds of the content area so we position it correctly.
#[tauri::command]
pub async fn browser_navigate(
    app: tauri::AppHandle,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let parsed: url::Url = url.parse().map_err(|e| format!("Invalid URL: {}", e))?;

    if let Some(wv) = app.get_webview("browser-child") {
        // Already exists — navigate and reposition
        wv.navigate(parsed).map_err(|e: tauri::Error| e.to_string())?;
        wv.set_position(tauri::LogicalPosition::new(x, y))
            .map_err(|e: tauri::Error| e.to_string())?;
        wv.set_size(tauri::LogicalSize::new(width, height))
            .map_err(|e: tauri::Error| e.to_string())?;
        wv.show().map_err(|e: tauri::Error| e.to_string())?;
    } else {
        let window = app.get_window("main").ok_or("Main window not found")?;
        let builder = tauri::webview::WebviewBuilder::new(
            "browser-child",
            tauri::WebviewUrl::External(parsed),
        );
        window
            .add_child(
                builder,
                tauri::LogicalPosition::new(x, y),
                tauri::LogicalSize::new(width, height),
            )
            .map_err(|e: tauri::Error| e.to_string())?;
    }

    Ok(())
}

/// Hide the browser child webview (when switching to Publish/Dashboard pages).
#[tauri::command]
pub async fn browser_hide(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(wv) = app.get_webview("browser-child") {
        wv.hide().map_err(|e: tauri::Error| e.to_string())?;
    }
    Ok(())
}
