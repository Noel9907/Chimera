import { useState, useRef } from "react";
import { FaArrowLeft, FaArrowRight, FaRedo, FaTimes } from "react-icons/fa";
import "./Browser.css";

function Browser() {
  const [tabs, setTabs] = useState([{ id: 1, title: "New Tab", url: "", currentUrl: "" }]);
  const [activeTabId, setActiveTabId] = useState(1);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  const activeTab = tabs.find(t => t.id === activeTabId) || tabs[0];

  const updateActiveTab = (updates: any) => {
    setTabs(tabs.map(t => t.id === activeTabId ? { ...t, ...updates } : t));
  };

  const addTab = () => {
    const newId = Date.now();
    setTabs([...tabs, { id: newId, title: "New Tab", url: "", currentUrl: "" }]);
    setActiveTabId(newId);
  };

  const closeTab = (e: any, id: number) => {
    e.stopPropagation();
    if (tabs.length === 1) return;
    const newTabs = tabs.filter(t => t.id !== id);
    setTabs(newTabs);
    if (activeTabId === id) setActiveTabId(newTabs[0].id);
  };

  // NEW: Helper function to handle bookmark clicks
  const openBookmark = (name: string, destination: string) => {
    updateActiveTab({ url: destination, currentUrl: destination, title: name });
  };

  // IMPROVED Search Logic
  const handleSearch = (e: any) => {
    if (e.key === "Enter") {
      let input = activeTab.url.trim();
      let finalUrl = "";

      if (input.startsWith("http://") || input.startsWith("https://")) {
        finalUrl = input;
      } else if (input.includes(".") && !input.includes(" ")) {
        // If it looks like a file or domain (test.pdf or example.com)
        finalUrl = `http://localhost:8080/view/${input}`;
      } else {
        // If it's just words (e.g., "cat image"), search Wikipedia or Google
        finalUrl = `https://en.wikipedia.org/wiki/Special:Search?search=${encodeURIComponent(input)}`;
      }
      
      updateActiveTab({ currentUrl: finalUrl, title: input });
    }
  };

  return (
    <div className="browser">
      {/* Tabs UI */}
      <div className="tabs">
        {tabs.map((tab) => (
          <div 
            key={tab.id} 
            className={`tab ${activeTabId === tab.id ? 'active' : ''}`}
            onClick={() => setActiveTabId(tab.id)}
          >
            {tab.title.substring(0, 12) || "New Tab"}
            <FaTimes className="close-icon" onClick={(e) => closeTab(e, tab.id)} />
          </div>
        ))}
        <div className="tab add" onClick={addTab}>+</div>
      </div>

      {/* Toolbar */}
      <div className="toolbar">
        <button className="nav-btn"><FaArrowLeft /></button>
        <button className="nav-btn"><FaArrowRight /></button>
        <button className="nav-btn" onClick={() => updateActiveTab({ currentUrl: "" })}><FaRedo /></button>
        <input
          className="address-bar"
          placeholder="Search Wikipedia or enter P2P filename..."
          value={activeTab.url}
          onChange={(e) => updateActiveTab({ url: e.target.value })}
          onKeyDown={handleSearch}
        />
      </div>

      {/* Bookmark Bar */}
      <div className="bookmark-bar">
        <span onClick={() => openBookmark("Google", "https://www.google.com/search?igu=1")}>⭐ Google</span>
        <span onClick={() => openBookmark("Wikipedia", "https://en.wikipedia.org")}>📚 Wikipedia</span>
        <span onClick={() => openBookmark("Maps", "https://www.openstreetmap.org")}>🗺️ Maps</span>
      </div>

      {/* Browser Window */}
      <div className="browser-window">
        {activeTab.currentUrl ? (
          <iframe
            key={activeTab.id} 
            src={activeTab.currentUrl}
            width="100%"
            height="100%"
            style={{ border: "none" }}
            title="browser-frame"
            sandbox="allow-forms allow-scripts allow-same-origin"
          />
        ) : (
          <div className="home">
            <h1>CHIMERA</h1>
            <p>PEER-TO-PEER NETWORK BROWSER</p>
          </div>
        )}
      </div>
    </div>
  );
}

export default Browser;