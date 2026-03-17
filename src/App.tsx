import { useState } from "react";
import Browser from "./pages/Browser";
import Upload from "./pages/Upload";
import Dashboard from "./pages/Dashboard";
import "./App.css";

type Page = "browser" | "publish" | "dashboard";

function App() {
  const [currentPage, setCurrentPage] = useState<Page>("publish");

  return (
    <div className="app">
      {/* Sidebar */}
      <div className="sidebar">
        <div className="sidebar-logo">
          <h2>Chimera</h2>
        </div>
        <nav className="sidebar-nav">
          <button
            className={`sidebar-btn ${currentPage === "browser" ? "active" : ""}`}
            onClick={() => setCurrentPage("browser")}
          >
            Browser
          </button>
          <button
            className={`sidebar-btn ${currentPage === "publish" ? "active" : ""}`}
            onClick={() => setCurrentPage("publish")}
          >
            Publish
          </button>
          <button
            className={`sidebar-btn ${currentPage === "dashboard" ? "active" : ""}`}
            onClick={() => setCurrentPage("dashboard")}
          >
            Dashboard
          </button>
        </nav>
      </div>

      {/* Main content */}
      <div className="main-content">
        {currentPage === "browser" && <Browser />}
        {currentPage === "publish" && <Upload />}
        {currentPage === "dashboard" && <Dashboard />}
      </div>
    </div>
  );
}

export default App;
