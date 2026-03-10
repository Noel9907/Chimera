import { useState } from "react";

const styles = `
  @import url('https://fonts.googleapis.com/css2?family=Space+Mono:wght@400;700&family=Syne:wght@400;600;800&display=swap');

  * { margin: 0; padding: 0; box-sizing: border-box; }

  body {
    font-family: 'Syne', sans-serif;
    background: #0a0a0a;
    color: #e8e8e8;
    min-height: 100vh;
  }

  .app { min-height: 100vh; display: flex; flex-direction: column; }

  /* Header */
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 40px;
    border-bottom: 1px solid #1e1e1e;
    background: #0a0a0a;
    position: sticky;
    top: 0;
    z-index: 100;
  }

  .logo-text {
    font-size: 22px;
    font-weight: 800;
    letter-spacing: 3px;
    text-transform: uppercase;
    color: #00ff88;
  }

  nav { display: flex; gap: 8px; }

  .nav-btn {
    background: transparent;
    border: 1px solid #2a2a2a;
    color: #888;
    padding: 8px 20px;
    border-radius: 4px;
    font-family: 'Space Mono', monospace;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s;
    letter-spacing: 1px;
  }

  .nav-btn:hover, .nav-btn.active {
    border-color: #00ff88;
    color: #00ff88;
    background: rgba(0,255,136,0.05);
  }

  /* Home Page */
  .home {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 40px;
    text-align: center;
    position: relative;
    overflow: hidden;
  }

  .home::before {
    content: '';
    position: absolute;
    width: 600px;
    height: 600px;
    background: radial-gradient(circle, rgba(0,255,136,0.06) 0%, transparent 70%);
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    pointer-events: none;
  }

  .badge {
    display: inline-block;
    background: rgba(0,255,136,0.1);
    border: 1px solid rgba(0,255,136,0.3);
    color: #00ff88;
    font-family: 'Space Mono', monospace;
    font-size: 11px;
    padding: 6px 16px;
    border-radius: 100px;
    letter-spacing: 2px;
    margin-bottom: 32px;
  }

  .hero-title {
    font-size: clamp(42px, 8vw, 80px);
    font-weight: 800;
    line-height: 1.05;
    letter-spacing: -2px;
    margin-bottom: 24px;
    color: #fff;
  }

  .hero-title span { color: #00ff88; }

  .hero-sub {
    font-size: 16px;
    color: #666;
    max-width: 480px;
    line-height: 1.7;
    margin-bottom: 48px;
    font-weight: 400;
  }

  .cta-btn {
    background: #00ff88;
    color: #0a0a0a;
    border: none;
    padding: 16px 40px;
    font-family: 'Syne', sans-serif;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 2px;
    text-transform: uppercase;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .cta-btn:hover {
    background: #00cc6a;
    transform: translateY(-2px);
    box-shadow: 0 8px 30px rgba(0,255,136,0.3);
  }

  .stats {
    display: flex;
    gap: 60px;
    margin-top: 80px;
    padding-top: 40px;
    border-top: 1px solid #1a1a1a;
  }

  .stat { text-align: center; }

  .stat-num {
    font-family: 'Space Mono', monospace;
    font-size: 28px;
    color: #00ff88;
    font-weight: 700;
  }

  .stat-label {
    font-size: 11px;
    color: #444;
    letter-spacing: 2px;
    text-transform: uppercase;
    margin-top: 4px;
  }

  /* Upload Page */
  .upload-page {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 40px;
  }

  .page-title {
    font-size: 36px;
    font-weight: 800;
    letter-spacing: -1px;
    margin-bottom: 8px;
    color: #fff;
  }

  .page-sub {
    font-size: 14px;
    color: #555;
    margin-bottom: 48px;
    font-family: 'Space Mono', monospace;
  }

  .upload-card {
    background: #111;
    border: 1px solid #1e1e1e;
    border-radius: 12px;
    padding: 48px;
    width: 100%;
    max-width: 520px;
  }

  .drop-zone {
    border: 2px dashed #2a2a2a;
    border-radius: 8px;
    padding: 48px 32px;
    text-align: center;
    cursor: pointer;
    transition: all 0.2s;
    margin-bottom: 24px;
  }

  .drop-zone:hover, .drop-zone.dragging {
    border-color: #00ff88;
    background: rgba(0,255,136,0.03);
  }

  .drop-icon {
    font-size: 40px;
    margin-bottom: 16px;
  }

  .drop-title {
    font-size: 16px;
    font-weight: 600;
    color: #ccc;
    margin-bottom: 8px;
  }

  .drop-sub {
    font-size: 12px;
    color: #444;
    font-family: 'Space Mono', monospace;
  }

  .file-input { display: none; }

  .file-list {
    margin-bottom: 24px;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    background: #0d0d0d;
    border: 1px solid #1e1e1e;
    border-radius: 6px;
    margin-bottom: 8px;
    font-family: 'Space Mono', monospace;
    font-size: 12px;
    color: #888;
  }

  .file-dot { width: 6px; height: 6px; background: #00ff88; border-radius: 50%; }

  .upload-btn {
    width: 100%;
    background: #00ff88;
    color: #0a0a0a;
    border: none;
    padding: 16px;
    font-family: 'Syne', sans-serif;
    font-size: 14px;
    font-weight: 700;
    letter-spacing: 2px;
    text-transform: uppercase;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .upload-btn:hover:not(:disabled) {
    background: #00cc6a;
    box-shadow: 0 4px 20px rgba(0,255,136,0.25);
  }

  .upload-btn:disabled { background: #1a1a1a; color: #333; cursor: not-allowed; }

  .cid-result {
    margin-top: 24px;
    padding: 16px;
    background: rgba(0,255,136,0.05);
    border: 1px solid rgba(0,255,136,0.2);
    border-radius: 6px;
  }

  .cid-label {
    font-size: 11px;
    color: #00ff88;
    letter-spacing: 2px;
    text-transform: uppercase;
    font-family: 'Space Mono', monospace;
    margin-bottom: 8px;
  }

  .cid-value {
    font-family: 'Space Mono', monospace;
    font-size: 11px;
    color: #888;
    word-break: break-all;
    line-height: 1.6;
  }

  footer {
    text-align: center;
    padding: 20px;
    border-top: 1px solid #1a1a1a;
    font-family: 'Space Mono', monospace;
    font-size: 11px;
    color: #333;
    letter-spacing: 1px;
  }
`;

function HomePage({ onNavigate }: { onNavigate: (page: string) => void }) {
  return (
    <div className="home">
      <div className="badge">P2P NETWORK // BETA</div>
      <h2 className="hero-title">
        Host Websites<br />
        <span>Decentralized.</span>
      </h2>
      <p className="hero-sub">
        Upload and access websites through a distributed peer-to-peer network.
        No servers. No central authority. Just peers.
      </p>
      <button className="cta-btn" onClick={() => onNavigate("upload")}>
        Upload Website
      </button>
      <div className="stats">
        <div className="stat">
          <div className="stat-num">0</div>
          <div className="stat-label">Peers Connected</div>
        </div>
        <div className="stat">
          <div className="stat-num">0 MB</div>
          <div className="stat-label">Storage Used</div>
        </div>
        <div className="stat">
          <div className="stat-num">0</div>
          <div className="stat-label">Sites Hosted</div>
        </div>
      </div>
    </div>
  );
}

function UploadPage() {
  console.log("hellow")
  const [files, setFiles] = useState<File[]>([]);
  const [dragging, setDragging] = useState(false);
  const [cid, setCid] = useState("");
  const [uploading, setUploading] = useState(false);

  const handleFiles = (newFiles: FileList | null) => {
    if (newFiles) setFiles(Array.from(newFiles));
  };

  const handleUpload = async () => {
    if (!files.length) return;
    setUploading(true);
    // TODO: call Tauri IPC → publish_site(files)
    //to be dibe in rust
    await new Promise(r => setTimeout(r, 1500)); // mock delay
    setCid("bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi");
    setUploading(false);
  };

  return (
    <div className="upload-page">+
      <h2 className="page-title">Upload Website</h2>
      <p className="page-sub">// select files to publish to the network</p>
      <div className="upload-card">
        <div
          className={`drop-zone ${dragging ? "dragging" : ""}`}
          onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
          onDragLeave={() => setDragging(false)}
          onDrop={(e) => { e.preventDefault(); setDragging(false); handleFiles(e.dataTransfer.files); }}
          onClick={() => document.getElementById("fileInput")?.click()}
        >
          <div className="drop-icon">⬡</div>
          <div className="drop-title">Drop files here</div>
          <div className="drop-sub">or click to browse</div>
          <input
            id="fileInput"
            type="file"
            multiple
            className="file-input"
            onChange={(e) => handleFiles(e.target.files)}
          />
        </div>

        {files.length > 0 && (
          <div className="file-list">
            {files.map((f, i) => (
              <div className="file-item" key={i}>
                <div className="file-dot" />
                {f.name}
              </div>
            ))}
          </div>
        )}

        <button
          className="upload-btn"
          onClick={handleUpload}
          disabled={!files.length || uploading}
        >
          {uploading ? "Publishing..." : "Upload to Network"}
        </button>

        {cid && (
          <div className="cid-result">
            <div className="cid-label">✓ Site CID</div>
            <div className="cid-value">{cid}</div>
          </div>
        )}
      </div>
    </div>
  );
}

function App() {
  const [page, setPage] = useState("home");

  return (
    <>
      <style>{styles}</style>
      <div className="app">
        <header>
          <div className="logo-text">Chimera</div>
          <nav>
            <button
              className={`nav-btn ${page === "home" ? "active" : ""}`}
              onClick={() => setPage("home")}
            >
              Home
            </button>
            <button
              className={`nav-btn ${page === "upload" ? "active" : ""}`}
              onClick={() => setPage("upload")}
            >
              Upload
            </button>
          </nav>
        </header>

        {page === "home" ? <HomePage onNavigate={setPage} /> : <UploadPage />}

        <footer>© 2026 CHIMERA PROJECT // DECENTRALIZED HOSTING</footer>
      </div>
    </>
  );
}

export default App;