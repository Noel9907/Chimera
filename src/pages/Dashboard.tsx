import "./Dashboard.css";

function Dashboard() {
  return (
    <div className="dashboard-page">
      <h1>Dashboard</h1>
      <p className="subtitle">Network stats and peer info will appear here once networking is enabled.</p>
      <div className="placeholder-cards">
        <div className="stat-card">
          <span className="stat-label">Node Status</span>
          <span className="stat-value">Offline</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Connected Peers</span>
          <span className="stat-value">0</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">DHT Size</span>
          <span className="stat-value">0</span>
        </div>
      </div>
    </div>
  );
}

export default Dashboard;
