import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./Dashboard.css";

function Dashboard() {
  const [nodeId, setNodeId] = useState("...");
  const [peerCount, setPeerCount] = useState(0);

  useEffect(() => {
    // Fetch node info on mount
    invoke<string>("get_node_id").then(setNodeId).catch(() => setNodeId("Error"));
    invoke<number>("get_peer_count").then(setPeerCount).catch(() => {});

    // Poll peer count every 5 seconds
    const interval = setInterval(() => {
      invoke<number>("get_peer_count").then(setPeerCount).catch(() => {});
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="dashboard-page">
      <h1>Dashboard</h1>
      <p className="subtitle">Your node is running on the Chimera P2P network.</p>
      <div className="placeholder-cards">
        <div className="stat-card">
          <span className="stat-label">Node Status</span>
          <span className="stat-value">Online</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Connected Peers</span>
          <span className="stat-value">{peerCount}</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Node ID</span>
          <span className="stat-value" style={{fontSize:"0.7rem", wordBreak:"break-all"}}>{nodeId}</span>
        </div>
      </div>
    </div>
  );
}

export default Dashboard;
