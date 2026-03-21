import { FaArrowLeft, FaArrowRight, FaRedo } from "react-icons/fa";
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./Browser.css";

interface WebContent {
  content_type: string;
  body: number[];
  file_path: string;
}

function Browser() {
  const [url, setUrl] = useState("");
  const [currentUrl, setCurrentUrl] = useState("");
  const [chimeraHtml, setChimeraHtml] = useState("");
  const [isChimera, setIsChimera] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function handleSearch(e: any) {
    if (e.key !== "Enter") return;

    const input = url.trim();
    if (!input) return;

    setError("");

    // chimera:// URL — fetch from P2P network
    if (input.startsWith("chimera://")) {
      setLoading(true);
      setCurrentUrl("");
      setIsChimera(true);
      setChimeraHtml("");
      try {
        const result = await invoke<WebContent | null>("navigate", { url: input });
        if (result && result.content_type.startsWith("text/html")) {
          const html = new TextDecoder().decode(new Uint8Array(result.body));
          setChimeraHtml(html);
        } else if (result) {
          setError("Not an HTML page: " + result.content_type);
        }
      } catch (err: any) {
        setError(typeof err === "string" ? err : err.message || "Failed to load site");
      } finally {
        setLoading(false);
      }
      return;
    }

    // Normal URL — load in iframe
    setIsChimera(false);
    setChimeraHtml("");
    let finalUrl = input;
    if (!input.startsWith("http")) {
      if (input.includes(".") && !input.includes(" ")) {
        finalUrl = "https://" + input;
      } else {
        finalUrl = "https://www.google.com/search?q=" + encodeURIComponent(input);
      }
    }
    setCurrentUrl(finalUrl);
  }

  function goBack() { window.history.back(); }
  function goForward() { window.history.forward(); }
  function refresh() { window.location.reload(); }

  return (
    <div className="browser">

      {/* Tabs */}
      <div className="tabs">
        <div className="tab active">New Tab</div>
        <div className="tab add">+</div>
      </div>

      {/* Toolbar */}
      <div className="bookmark-bar">
      <div className="bookmark-bar">
      <span onClick={() => { setIsChimera(false); setChimeraHtml(""); setCurrentUrl("https://www.google.com"); }}>⭐ Google</span>
      <span onClick={() => { setIsChimera(false); setChimeraHtml(""); setCurrentUrl("https://www.wikipedia.org"); }}>📚 Wikipedia</span>
      <span onClick={() => { setIsChimera(false); setChimeraHtml(""); setCurrentUrl("https://github.com"); }}>💻 GitHub</span>
      </div>
       </div>
      <div className="toolbar">
        <button className="nav-btn" onClick={goBack}><FaArrowLeft/></button>
        <button className="nav-btn" onClick={goForward}><FaArrowRight/></button>
        <button className="nav-btn" onClick={refresh}><FaRedo/></button>
        <input
          className="address-bar"
          placeholder="Search Google or type a URL"
          value={url}
          onChange={(e)=>setUrl(e.target.value)}
          onKeyDown={handleSearch}
        />
      </div>

      {/* Browser window */}
      <div className="browser-window">
        {loading ? (
          <div className="home">
            <h1>Loading...</h1>
            <p>Fetching from P2P network</p>
          </div>
        ) : error ? (
          <div className="home">
            <h1>Error</h1>
            <p style={{color:"#c0392b"}}>{error}</p>
          </div>
        ) : isChimera && chimeraHtml ? (
          <iframe
            srcDoc={chimeraHtml}
            width="100%"
            height="100%"
            style={{border:"none"}}
            sandbox="allow-scripts allow-same-origin"
          />
        ) : currentUrl ? (
          <iframe
            src={currentUrl}
            width="100%"
            height="100%"
            style={{border:"none"}}
          />
        ) : (
          <div className="home">
            <h1>Chimera Browser</h1>
            <p>Search the web securely</p>
          </div>
        )}
      </div>

    </div>
  );
}

export default Browser;
