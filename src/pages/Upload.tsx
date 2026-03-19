import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./Upload.css";

// Matches the Rust PublishResult struct
interface PublishResult {
  site_name: string;
  root_cid: string;
  total_size: number;
  chunk_count: number;
  file_count: number;
}

// Matches the Rust SiteInfo struct
interface SiteInfo {
  name: string;
  root_cid: string;
  total_size: number;
  chunk_count: number;
  file_count: number;
  published_at: string;
  is_local: boolean;
  is_pinned: boolean;
}

function Upload() {
  const [folderPath, setFolderPath] = useState("");
  const [siteName, setSiteName] = useState("");
  const [isPublishing, setIsPublishing] = useState(false);
  const [result, setResult] = useState<PublishResult | null>(null);
  const [error, setError] = useState("");
  const [publishedSites, setPublishedSites] = useState<SiteInfo[]>([]);

  // Load published sites on mount
  useEffect(() => {
    loadPublishedSites();
  }, []);

  async function loadPublishedSites() {
    try {
      const sites = await invoke<SiteInfo[]>("get_published_sites");
      setPublishedSites(sites);
    } catch (e) {
      console.error("Failed to load sites:", e);
    }
  }

  async function selectFolder() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected) {
        setFolderPath(selected as string);
        setError("");
        setResult(null);
      }
    } catch (e) {
      console.error("Folder picker error:", e);
    }
  }

  async function handlePublish() {
    if (!folderPath) {
      setError("Please select a folder first");
      return;
    }
    if (!siteName) {
      setError("Please enter a site name");
      return;
    }

    setIsPublishing(true);
    setError("");
    setResult(null);

    try {
      const res = await invoke<PublishResult>("publish_site", {
        folderPath: folderPath,
        siteName: siteName,
      });
      setResult(res);
      // Refresh the published sites list
      await loadPublishedSites();
      // Clear inputs
      setFolderPath("");
      setSiteName("");
    } catch (e) {
      setError(String(e));
    } finally {
      setIsPublishing(false);
    }
  }

  async function handleUnpublish(name: string) {
    try {
      await invoke("unpublish_site", { siteName: name });
      await loadPublishedSites();
    } catch (e) {
      setError(String(e));
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  }

  return (
    <div className="upload-page">
      <h1>Publish a Site</h1>
      <p className="subtitle">
        Select a folder with your static site (HTML/CSS/JS) and give it a name.
        Files will be chunked, hashed, and stored locally.
      </p>

      {/* Publish Form */}
      <div className="publish-form">
        {/* Folder picker */}
        <div className="form-group">
          <label>Site Folder</label>
          <div className="folder-picker">
            <input
              type="text"
              value={folderPath}
              readOnly
              placeholder="No folder selected..."
              className="folder-input"
            />
            <button onClick={selectFolder} className="btn btn-secondary">
              Browse
            </button>
          </div>
        </div>

        {/* Site name */}
        <div className="form-group">
          <label>Site Name</label>
          <input
            type="text"
            value={siteName}
            onChange={(e) => setSiteName(e.target.value.toLowerCase())}
            placeholder="my-awesome-site"
            className="name-input"
          />
          <span className="hint">
            Lowercase letters, numbers, and hyphens. 3-63 characters.
          </span>
        </div>

        {/* Publish button */}
        <button
          onClick={handlePublish}
          disabled={isPublishing || !folderPath || !siteName}
          className="btn btn-primary"
        >
          {isPublishing ? "Publishing..." : "Publish Site"}
        </button>
      </div>

      {/* Error message */}
      {error && <div className="error-msg">{error}</div>}

      {/* Success result */}
      {result && (
        <div className="success-card">
          <h3>Published Successfully!</h3>
          <div className="result-grid">
            <div className="result-item">
              <span className="result-label">Site Name</span>
              <span className="result-value">{result.site_name}</span>
            </div>
            <div className="result-item">
              <span className="result-label">Root CID</span>
              <span className="result-value cid">{result.root_cid}</span>
            </div>
            <div className="result-item">
              <span className="result-label">Files</span>
              <span className="result-value">{result.file_count}</span>
            </div>
            <div className="result-item">
              <span className="result-label">Chunks</span>
              <span className="result-value">{result.chunk_count}</span>
            </div>
            <div className="result-item">
              <span className="result-label">Total Size</span>
              <span className="result-value">{formatBytes(result.total_size)}</span>
            </div>
          </div>
          <div className="chimera-url">
            chimera://{result.site_name}
          </div>
        </div>
      )}

      {/* Published Sites List */}
      {publishedSites.length > 0 && (
        <div className="published-sites">
          <h2>Published Sites</h2>
          <div className="sites-list">
            {publishedSites.map((site) => (
              <div key={site.name} className="site-card">
                <div className="site-info">
                  <span className="site-name">{site.name}</span>
                  <span className="site-meta">
                    {site.file_count} files | {site.chunk_count} chunks |{" "}
                    {formatBytes(site.total_size)}
                  </span>
                  <span className="site-cid">{site.root_cid}</span>
                </div>
                <button
                  onClick={() => handleUnpublish(site.name)}
                  className="btn btn-danger"
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export default Upload;
