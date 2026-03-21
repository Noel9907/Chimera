import { FaArrowLeft, FaArrowRight, FaRedo } from "react-icons/fa";
import { useState } from "react";
import "./Browser.css";

function Browser() {
  const [url, setUrl] = useState("");
  const [currentUrl, setCurrentUrl] = useState("");

  function handleSearch(e:any){
    if(e.key !== "Enter") return;

    const input = url.trim();
    if(!input) return;

    // chimera:// URL — route through custom protocol so resources resolve naturally
    if(input.startsWith("chimera://")) {
      const rest = input.replace("chimera://", "");
      // Add /index.html if no file path specified
      const path = rest.includes(".") || rest.endsWith("/") ? rest : rest + "/index.html";
      // Tauri custom protocols are accessed as http://scheme.localhost/ on Windows
      setCurrentUrl("http://chimera-content.localhost/" + path);
      return;
    }

    // Normal URL
    let finalUrl = input;
    if(!input.startsWith("http")){
      if(input.includes(".") && !input.includes(" ")){
        finalUrl = "https://" + input;
      } else {
        finalUrl = "https://www.google.com/search?q=" + encodeURIComponent(input);
      }
    }
    setCurrentUrl(finalUrl);
  }

  function goBack(){ window.history.back(); }
  function goForward(){ window.history.forward(); }
  function refresh(){ window.location.reload(); }

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
      <span onClick={() => setCurrentUrl("https://www.google.com")}>⭐ Google</span>
      <span onClick={() => setCurrentUrl("https://www.wikipedia.org")}>📚 Wikipedia</span>
      <span onClick={() => setCurrentUrl("https://github.com")}>💻 GitHub</span>
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
        {currentUrl ? (
          <iframe
            src={currentUrl}
            width="100%"
            height="100%"
            style={{border:"none"}}
          />
        ) : (
          <>
        <div className="home">
         <h1>Chimera Browser</h1>
          <p>Search the web securely</p>
         </div>
        </>
        )}
      </div>

    </div>
  );
}

export default Browser;
