import { FaArrowLeft, FaArrowRight, FaRedo } from "react-icons/fa";
import { useState, useRef, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./Browser.css";

function Browser() {
  const [url, setUrl] = useState("");
  // chimeraUrl = iframe src for chimera:// sites, webviewUrl = child webview for normal sites
  const [chimeraUrl, setChimeraUrl] = useState("");
  const [webviewUrl, setWebviewUrl] = useState("");
  const browserWindowRef = useRef<HTMLDivElement>(null);

  // Position the child webview over the .browser-window div
  const navigateWebview = useCallback((targetUrl: string) => {
    const el = browserWindowRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    invoke("browser_navigate", {
      url: targetUrl,
      x: rect.left,
      y: rect.top,
      width: rect.width,
      height: rect.height,
    }).catch((e) => console.error("browser_navigate failed:", e));
  }, []);

  // Hide webview when this component unmounts (user switches page)
  useEffect(() => {
    return () => {
      invoke("browser_hide").catch(() => {});
    };
  }, []);

  // Hide webview when switching to a chimera URL or back to home
  function hideWebview() {
    invoke("browser_hide").catch(() => {});
    setWebviewUrl("");
  }

  function handleSearch(e:any){
    if(e.key !== "Enter") return;

    const input = url.trim();
    if(!input) return;

    // chimera:// URL — route through custom protocol iframe
    if(input.startsWith("chimera://")) {
      hideWebview();
      const rest = input.replace("chimera://", "");
      const path = rest.includes(".") || rest.endsWith("/") ? rest : rest + "/index.html";
      setChimeraUrl("http://chimera-content.localhost/" + path);
      return;
    }

    // Normal URL — use child webview
    setChimeraUrl("");
    let finalUrl = input;
    if(!input.startsWith("http")){
      if(input.includes(".") && !input.includes(" ")){
        finalUrl = "https://" + input;
      } else {
        finalUrl = "https://www.google.com/search?q=" + encodeURIComponent(input);
      }
    }
    setWebviewUrl(finalUrl);
    navigateWebview(finalUrl);
  }

  function handleBookmark(bookmarkUrl: string) {
    setUrl(bookmarkUrl);
    setChimeraUrl("");
    setWebviewUrl(bookmarkUrl);
    navigateWebview(bookmarkUrl);
  }

  function goBack(){ window.history.back(); }
  function goForward(){ window.history.forward(); }
  function refresh(){
    if (webviewUrl) navigateWebview(webviewUrl);
    else if (chimeraUrl) setChimeraUrl(chimeraUrl);
  }

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
      <span onClick={() => handleBookmark("https://www.google.com")}>⭐ Google</span>
      <span onClick={() => handleBookmark("https://www.wikipedia.org")}>📚 Wikipedia</span>
      <span onClick={() => handleBookmark("https://github.com")}>💻 GitHub</span>
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
      <div className="browser-window" ref={browserWindowRef}>
        {chimeraUrl ? (
          <iframe
            src={chimeraUrl}
            width="100%"
            height="100%"
            style={{border:"none"}}
          />
        ) : !webviewUrl ? (
          <div className="home">
           <h1>Chimera Browser</h1>
            <p>Search the web securely</p>
           </div>
        ) : null}
      </div>

    </div>
  );
}

export default Browser;
