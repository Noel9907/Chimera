import { useState, useRef, useEffect } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from "recharts";

// ── tiny SVG icons ────────────────────────────────────────────────────────────
const Icon = {
  Grid: () => (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="currentColor">
      <rect x="1" y="1" width="6" height="6" rx="1" />
      <rect x="9" y="1" width="6" height="6" rx="1" />
      <rect x="1" y="9" width="6" height="6" rx="1" />
      <rect x="9" y="9" width="6" height="6" rx="1" />
    </svg>
  ),
  Upload: () => (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
      <path d="M8 12V4M5 7l3-3 3 3M2 14h12" />
    </svg>
  ),
  Files: () => (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
      <rect x="1" y="3" width="14" height="10" rx="1.5" />
      <path d="M5 7h6M5 10h4" />
    </svg>
  ),
  Peers: () => (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
      <circle cx="5" cy="8" r="2.5" />
      <circle cx="11" cy="4" r="2.5" />
      <circle cx="11" cy="12" r="2.5" />
      <path d="M7.5 7l2-2M7.5 9l2 2" />
    </svg>
  ),
  Logs: () => (
    <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5">
      <path d="M2 4h12M2 8h8M2 12h10" />
    </svg>
  ),
  Refresh: () => (
    <svg width="13" height="13" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5">
      <path d="M1 7A6 6 0 0 1 12.8 5M13 7a6 6 0 0 1-11.8 2" />
      <path d="M1 5V1M13 9v4" />
      <path d="M1 1l4 4M9 9l4 4" />
    </svg>
  ),
  Deploy: () => (
    <svg width="13" height="13" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5">
      <path d="M7 10V2M4 5l3-3 3 3M2 12h10" />
    </svg>
  ),
  ExternalLink: () => (
    <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5">
      <path d="M5 2H2a1 1 0 0 0-1 1v7a1 1 0 0 0 1 1h7a1 1 0 0 0 1-1V7M9 1h2v2M6 6l5-5" />
    </svg>
  ),
};

// ── helpers ───────────────────────────────────────────────────────────────────
const now = () => {
  const d = new Date();
  return [d.getHours(), d.getMinutes(), d.getSeconds()]
    .map((n) => String(n).padStart(2, "0"))
    .join(":");
};

const fakeCID = () =>
  "Qm" + Math.random().toString(36).slice(2, 10);

const fmtSize = (bytes) => {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / 1048576).toFixed(2) + " MB";
};

// ── styles (CSS-in-JS object map) ─────────────────────────────────────────────
const S = {
  // layout
  shell: {
    display: "flex",
    height: "100vh",
    fontFamily: "'Syne', 'Segoe UI', sans-serif",
    background: "#f4f3ef",
    overflow: "hidden",
  },
  // sidebar
  sidebar: {
    width: 220,
    background: "#ffffff",
    borderRight: "0.5px solid rgba(0,0,0,0.08)",
    display: "flex",
    flexDirection: "column",
    flexShrink: 0,
  },
  brandWrap: {
    padding: "24px 20px 20px",
    borderBottom: "0.5px solid rgba(0,0,0,0.08)",
  },
  brandMark: { display: "flex", alignItems: "center", gap: 10 },
  brandDot: {
    width: 28, height: 28, borderRadius: 6,
    background: "#0F6E56",
    display: "flex", alignItems: "center", justifyContent: "center",
  },
  brandDotInner: { width: 10, height: 10, borderRadius: "50%", background: "#fff" },
  brandName: { fontSize: 15, fontWeight: 700, color: "#1a1a18", letterSpacing: -0.3 },
  nav: { padding: "12px 10px", flex: 1 },
  navSection: {
    fontSize: 10, fontWeight: 600, color: "#a0a09a",
    letterSpacing: 1, textTransform: "uppercase",
    padding: "8px 10px 4px",
  },
  navItem: (active) => ({
    display: "flex", alignItems: "center", gap: 10,
    padding: "9px 10px", borderRadius: 8, cursor: "pointer",
    fontSize: 13, fontWeight: 500,
    color: active ? "#0F6E56" : "#6b6b65",
    background: active ? "#E1F5EE" : "transparent",
    transition: "background .15s, color .15s",
  }),
  sidebarFooter: {
    padding: "16px 10px",
    borderTop: "0.5px solid rgba(0,0,0,0.08)",
  },
  nodePill: {
    display: "flex", alignItems: "center", gap: 8,
    padding: "8px 10px", borderRadius: 8,
    background: "#f4f3ef",
  },
  statusDot: {
    width: 8, height: 8, borderRadius: "50%",
    background: "#1D9E75", flexShrink: 0,
  },
  nodeLabel: { fontSize: 12, fontWeight: 500, color: "#555" },
  nodeId: { fontSize: 10, color: "#a0a09a", fontFamily: "monospace" },
  // topbar
  topbar: {
    background: "#ffffff",
    borderBottom: "0.5px solid rgba(0,0,0,0.08)",
    padding: "0 28px", height: 56,
    display: "flex", alignItems: "center", justifyContent: "space-between",
    flexShrink: 0,
  },
  topbarLeft: { display: "flex", alignItems: "center", gap: 8 },
  pageTitle: { fontSize: 16, fontWeight: 700, color: "#1a1a18", letterSpacing: -0.3 },
  breadcrumb: { fontSize: 12, color: "#a0a09a" },
  topbarRight: { display: "flex", alignItems: "center", gap: 10 },
  btnPrimary: {
    fontFamily: "inherit",
    display: "inline-flex", alignItems: "center", gap: 6,
    padding: "8px 16px", borderRadius: 8,
    fontSize: 13, fontWeight: 600, cursor: "pointer",
    border: "none", background: "#0F6E56", color: "#fff",
  },
  btnGhost: {
    fontFamily: "inherit",
    display: "inline-flex", alignItems: "center", gap: 6,
    padding: "8px 14px", borderRadius: 8,
    fontSize: 13, fontWeight: 600, cursor: "pointer",
    border: "0.5px solid rgba(0,0,0,0.12)",
    background: "#f4f3ef", color: "#555",
  },
  // main content
  main: { flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" },
  content: { flex: 1, overflowY: "auto", padding: "24px 28px", display: "flex", flexDirection: "column", gap: 20 },
  // metric cards
  metricsGrid: { display: "grid", gridTemplateColumns: "repeat(4, minmax(0,1fr))", gap: 14 },
  metricCard: {
    background: "#fff",
    border: "0.5px solid rgba(0,0,0,0.08)",
    borderRadius: 12, padding: "16px 18px",
  },
  metricLabel: {
    fontSize: 10, fontWeight: 600, color: "#a0a09a",
    textTransform: "uppercase", letterSpacing: 0.8, marginBottom: 10,
  },
  metricValue: { fontSize: 26, fontWeight: 700, color: "#1a1a18", letterSpacing: -1, lineHeight: 1 },
  metricSub: { fontSize: 11, color: "#a0a09a", marginTop: 6, fontFamily: "monospace" },
  badgeGreen: {
    display: "inline-flex", alignItems: "center", gap: 4,
    fontSize: 11, fontWeight: 600, padding: "3px 8px",
    borderRadius: 20, marginTop: 8,
    background: "#E1F5EE", color: "#0F6E56",
  },
  badgeBlue: {
    display: "inline-flex", alignItems: "center", gap: 4,
    fontSize: 11, fontWeight: 600, padding: "3px 8px",
    borderRadius: 20, marginTop: 8,
    background: "#E6F1FB", color: "#185FA5",
  },
  // two-col
  twoCol: { display: "grid", gridTemplateColumns: "1fr 340px", gap: 16 },
  panel: {
    background: "#fff",
    border: "0.5px solid rgba(0,0,0,0.08)",
    borderRadius: 12, padding: 20,
  },
  panelHeader: { display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 16 },
  panelTitle: { fontSize: 13, fontWeight: 700, color: "#1a1a18", letterSpacing: -0.2 },
  panelAction: { fontSize: 11, color: "#0F6E56", cursor: "pointer", fontWeight: 600 },
  // peers
  peerRow: {
    display: "flex", alignItems: "center", justifyContent: "space-between",
    padding: "9px 10px", borderRadius: 8, background: "#f4f3ef", marginBottom: 4,
  },
  peerId: { fontSize: 11, fontFamily: "monospace", color: "#555" },
  peerLatency: { display: "flex", alignItems: "center", gap: 6 },
  latencyBarWrap: {
    width: 48, height: 4, background: "rgba(0,0,0,0.1)",
    borderRadius: 4, overflow: "hidden",
  },
  latencyVal: { fontSize: 11, fontFamily: "monospace", color: "#0F6E56", fontWeight: 500, minWidth: 36, textAlign: "right" },
  // bottom row
  bottomRow: { display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 },
  // table
  th: {
    fontSize: 10, fontWeight: 600, textTransform: "uppercase",
    letterSpacing: 0.7, color: "#a0a09a",
    padding: "0 8px 10px", textAlign: "left",
    borderBottom: "0.5px solid rgba(0,0,0,0.08)",
  },
  td: {
    padding: "9px 8px",
    borderBottom: "0.5px solid rgba(0,0,0,0.06)",
    color: "#1a1a18", verticalAlign: "middle", fontSize: 12,
  },
  cidCode: {
    fontFamily: "monospace", fontSize: 10, color: "#555",
    background: "#f4f3ef", padding: "2px 6px", borderRadius: 4,
  },
  pillActive: {
    display: "inline-flex", alignItems: "center", gap: 4,
    fontSize: 10, fontWeight: 600, padding: "2px 8px", borderRadius: 20,
    background: "#E1F5EE", color: "#0F6E56",
  },
  pillRep: {
    display: "inline-flex", alignItems: "center", gap: 4,
    fontSize: 10, fontWeight: 600, padding: "2px 8px", borderRadius: 20,
    background: "#FAEEDA", color: "#854F0B",
  },
  linkIcon: {
    display: "inline-flex", alignItems: "center", justifyContent: "center",
    width: 22, height: 22, borderRadius: 4, background: "#f4f3ef", cursor: "pointer",
  },
  // upload zone
  uploadZone: {
    border: "1.5px dashed rgba(0,0,0,0.15)", borderRadius: 10,
    padding: 20, textAlign: "center", cursor: "pointer",
  },
  uploadHint: { fontSize: 12, color: "#888", marginTop: 4 },
  // log
  logPanel: {
    background: "#0D1117",
    border: "0.5px solid #30363d",
    borderRadius: 12, padding: 16,
    fontFamily: "monospace", fontSize: 11,
    overflowY: "auto", height: 200,
  },
};

// ── sub-components ────────────────────────────────────────────────────────────
const NavItem = ({ icon: Ic, label, active }) => (
  <div style={S.navItem(active)}>
    <Ic />
    {label}
  </div>
);

const MetricCard = ({ label, value, sub, badge }) => (
  <div style={S.metricCard}>
    <div style={S.metricLabel}>{label}</div>
    <div style={S.metricValue}>{value}</div>
    {sub && <div style={S.metricSub}>{sub}</div>}
    {badge && <div style={badge.style}>{badge.content}</div>}
  </div>
);

const PeerRow = ({ id, latency, color = "#0F6E56" }) => {
  const pct = Math.min(100, Math.round((latency / 100) * 100));
  return (
    <div style={S.peerRow}>
      <span style={S.peerId}>{id}</span>
      <div style={S.peerLatency}>
        <div style={S.latencyBarWrap}>
          <div style={{ height: "100%", borderRadius: 4, background: color, width: `${pct}%` }} />
        </div>
        <span style={{ ...S.latencyVal, color }}>{latency} ms</span>
      </div>
    </div>
  );
};

const LogEntry = ({ time, text, type = "info" }) => {
  const colors = { info: "#7ee787", sys: "#79c0ff", warn: "#e3b341" };
  return (
    <div style={{ display: "flex", alignItems: "flex-start", gap: 8, padding: "3px 0" }}>
      <span style={{ color: "#484f58", flexShrink: 0 }}>{time}</span>
      <span style={{ color: colors[type] || colors.info }}>{text}</span>
    </div>
  );
};

// ── chart tooltip ─────────────────────────────────────────────────────────────
const ChartTooltip = ({ active, payload, label }) => {
  if (!active || !payload?.length) return null;
  return (
    <div style={{
      background: "#fff", border: "0.5px solid rgba(0,0,0,0.1)",
      borderRadius: 8, padding: "8px 12px", fontSize: 12,
    }}>
      <div style={{ color: "#a0a09a", marginBottom: 2 }}>{label}</div>
      <div style={{ fontWeight: 700, color: "#0F6E56" }}>{payload[0].value.toFixed(2)} GB</div>
    </div>
  );
};

// ── main dashboard ────────────────────────────────────────────────────────────
export default function Dashboard() {
  const [activeNav, setActiveNav] = useState("Dashboard");
  const [sites, setSites] = useState([
    { name: "blog.html", cid: "Qm123abc", size: "2 MB", status: "Active" },
    { name: "index.html", cid: "Qm456def", size: "850 KB", status: "Active" },
  ]);
  const [logs, setLogs] = useState([
    { time: "09:41:02", text: "Node started — IPFS daemon ready", type: "sys" },
    { time: "09:41:05", text: "Connected to peer 12D3KooA1", type: "info" },
    { time: "09:41:06", text: "Connected to peer 12D3KooB9", type: "info" },
    { time: "09:41:07", text: "Connected to peer 12D3KooZ2", type: "info" },
    { time: "09:42:11", text: "Replication complete — 3/3 peers", type: "sys" },
  ]);
  const [selectedFile, setSelectedFile] = useState(null);
  const fileInputRef = useRef();
  const logRef = useRef();

  const storageData = [
    { day: "Mon", value: 1.1 },
    { day: "Tue", value: 1.2 },
    { day: "Wed", value: 1.3 },
    { day: "Thu", value: 1.35 },
    { day: "Fri", value: 1.4 },
    { day: "Sat", value: 1.4 },
    { day: "Sun", value: 1.42 },
  ];

  const peers = [
    { id: "12D3KooA1", latency: 32, color: "#0F6E56" },
    { id: "12D3KooB9", latency: 40, color: "#185FA5" },
    { id: "12D3KooZ2", latency: 27, color: "#0F6E56" },
  ];

  const addLog = (entries) =>
    setLogs((prev) => [...prev, ...entries]);

  useEffect(() => {
    if (logRef.current) logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [logs]);

  const handleDeploy = () => {
    if (!selectedFile) return;
    const cid = fakeCID();
    const size = fmtSize(selectedFile.size);
    const ts = now();

    setSites((prev) => [...prev, { name: selectedFile.name, cid, size, status: "Replicating" }]);
    addLog([
      { time: ts, text: `File received: ${selectedFile.name}`, type: "warn" },
      { time: ts, text: "SHA-256 hash computed", type: "info" },
      { time: ts, text: `CID pinned: ${cid}`, type: "sys" },
      { time: ts, text: "Replicating across 3 peers…", type: "info" },
    ]);
    setSelectedFile(null);
    if (fileInputRef.current) fileInputRef.current.value = "";

    setTimeout(() => {
      setSites((prev) =>
        prev.map((s) => s.cid === cid ? { ...s, status: "Active" } : s)
      );
      addLog([{ time: now(), text: "Replication complete — 3/3 peers", type: "sys" }]);
    }, 3000);
  };

  const handleRefresh = () => {
    addLog([{ time: now(), text: "Manual refresh triggered", type: "sys" }]);
  };

  const navItems = [
    { label: "Dashboard", icon: Icon.Grid },
    { label: "Upload", icon: Icon.Upload },
    { label: "Hosted Sites", icon: Icon.Files },
    { label: "Peers", icon: Icon.Peers },
    { label: "Activity Logs", icon: Icon.Logs },
  ];

  return (
    <div style={S.shell}>
      {/* Sidebar */}
      <aside style={S.sidebar}>
        <div style={S.brandWrap}>
          <div style={S.brandMark}>
            <div style={S.brandDot}><div style={S.brandDotInner} /></div>
            <span style={S.brandName}>P2P Hosting</span>
          </div>
        </div>

        <nav style={S.nav}>
          <div style={S.navSection}>Navigation</div>
          {navItems.map(({ label, icon }) => (
            <div key={label} onClick={() => setActiveNav(label)}>
              <NavItem icon={icon} label={label} active={activeNav === label} />
            </div>
          ))}
        </nav>

        <div style={S.sidebarFooter}>
          <div style={S.nodePill}>
            <div style={S.statusDot} />
            <div>
              <div style={S.nodeLabel}>Node active</div>
              <div style={S.nodeId}>12D3Koo…X7Fa</div>
            </div>
          </div>
        </div>
      </aside>

      {/* Main */}
      <div style={S.main}>
        {/* Topbar */}
        <div style={S.topbar}>
          <div style={S.topbarLeft}>
            <span style={S.pageTitle}>Dashboard</span>
            <span style={S.breadcrumb}>/ Overview</span>
          </div>
          <div style={S.topbarRight}>
            <button style={S.btnGhost} onClick={handleRefresh}>
              <Icon.Refresh /> Refresh
            </button>
            <button style={S.btnPrimary} onClick={() => fileInputRef.current?.click()}>
              <Icon.Deploy /> Deploy
            </button>
          </div>
        </div>

        {/* Content */}
        <div style={S.content}>
          {/* Metric cards */}
          <div style={S.metricsGrid}>
            <MetricCard
              label="Node status"
              value={<span style={{ fontSize: 18, color: "#0F6E56" }}>Online</span>}
              badge={{
                style: S.badgeGreen,
                content: <><div style={{ width: 6, height: 6, borderRadius: "50%", background: "#1D9E75" }} /> Healthy</>,
              }}
            />
            <MetricCard
              label="Connected peers"
              value={peers.length}
              sub="avg 33ms latency"
            />
            <MetricCard
              label="Storage used"
              value={<>1.4<span style={{ fontSize: 14, fontWeight: 500, color: "#a0a09a" }}> GB</span></>}
              sub="of 10 GB quota"
            />
            <MetricCard
              label="Hosted sites"
              value={sites.length}
              badge={{
                style: S.badgeBlue,
                content: "All active",
              }}
            />
          </div>

          {/* Chart + Peers */}
          <div style={S.twoCol}>
            <div style={S.panel}>
              <div style={S.panelHeader}>
                <span style={S.panelTitle}>Storage usage</span>
                <span style={S.panelAction}>7 days</span>
              </div>
              <ResponsiveContainer width="100%" height={180}>
                <LineChart data={storageData} margin={{ top: 4, right: 4, bottom: 0, left: -20 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="rgba(0,0,0,0.06)" vertical={false} />
                  <XAxis dataKey="day" tick={{ fontSize: 10, fill: "#a0a09a" }} axisLine={false} tickLine={false} />
                  <YAxis
                    domain={[1.0, 1.6]}
                    tick={{ fontSize: 10, fill: "#a0a09a" }}
                    axisLine={false} tickLine={false}
                    tickFormatter={(v) => v.toFixed(1)}
                  />
                  <Tooltip content={<ChartTooltip />} />
                  <Line
                    type="monotone" dataKey="value"
                    stroke="#0F6E56" strokeWidth={2.5}
                    dot={{ fill: "#0F6E56", r: 3, strokeWidth: 1.5, stroke: "#fff" }}
                    activeDot={{ r: 5 }}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>

            <div style={S.panel}>
              <div style={S.panelHeader}>
                <span style={S.panelTitle}>Peer network</span>
                <span style={S.panelAction}>+ Add peer</span>
              </div>
              {peers.map((p) => (
                <PeerRow key={p.id} {...p} />
              ))}
              <div style={{
                marginTop: 14, paddingTop: 14,
                borderTop: "0.5px solid rgba(0,0,0,0.08)",
                display: "flex", justifyContent: "space-between", alignItems: "center",
              }}>
                <span style={{ fontSize: 11, color: "#a0a09a", textTransform: "uppercase", letterSpacing: 0.7 }}>Replication factor</span>
                <span style={{ fontSize: 13, fontWeight: 700, color: "#1a1a18", fontFamily: "monospace" }}>3×</span>
              </div>
            </div>
          </div>

          {/* Sites table + Log */}
          <div style={S.bottomRow}>
            <div style={S.panel}>
              <div style={S.panelHeader}>
                <span style={S.panelTitle}>Hosted content</span>
                <span style={S.panelAction}>View all</span>
              </div>
              <div style={{ overflowX: "auto" }}>
                <table style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr>
                      {["Name", "CID", "Size", "Status", ""].map((h) => (
                        <th key={h} style={S.th}>{h}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {sites.map((s, i) => (
                      <tr key={i}>
                        <td style={{ ...S.td, fontWeight: 500 }}>{s.name}</td>
                        <td style={S.td}><span style={S.cidCode}>{s.cid}</span></td>
                        <td style={{ ...S.td, color: "#888" }}>{s.size}</td>
                        <td style={S.td}>
                          <span style={s.status === "Active" ? S.pillActive : S.pillRep}>
                            {s.status}
                          </span>
                        </td>
                        <td style={S.td}>
                          <a
                            href={`http://localhost:8080/ipfs/${s.cid}`}
                            target="_blank"
                            rel="noreferrer"
                            style={S.linkIcon}
                          >
                            <Icon.ExternalLink />
                          </a>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>

              {/* Upload */}
              <div style={{ marginTop: 16, paddingTop: 14, borderTop: "0.5px solid rgba(0,0,0,0.08)" }}>
                <div style={{ fontSize: 11, fontWeight: 600, color: "#888", marginBottom: 8 }}>
                  Deploy new file
                </div>
                <div
                  style={S.uploadZone}
                  onClick={() => fileInputRef.current?.click()}
                >
                  <svg width="28" height="28" viewBox="0 0 32 32" fill="none" stroke="#ccc" strokeWidth="1.5" style={{ margin: "0 auto 8px", display: "block" }}>
                    <path d="M16 22V10M11 15l5-5 5 5M6 26h20" />
                  </svg>
                  <div style={S.uploadHint}>
                    <span style={{ color: "#0F6E56", fontWeight: 600 }}>Click to browse</span> or drag & drop
                  </div>
                  <div style={S.uploadHint}>HTML, CSS, JS, images</div>
                  {selectedFile && (
                    <div style={{ fontSize: 11, color: "#1D9E75", marginTop: 6, fontFamily: "monospace" }}>
                      {selectedFile.name} selected
                    </div>
                  )}
                </div>
                <input
                  ref={fileInputRef}
                  type="file"
                  style={{ display: "none" }}
                  onChange={(e) => setSelectedFile(e.target.files[0] || null)}
                />
                <button
                  style={{ ...S.btnPrimary, width: "100%", justifyContent: "center", marginTop: 10 }}
                  onClick={handleDeploy}
                >
                  <Icon.Deploy /> Deploy to network
                </button>
              </div>
            </div>

            {/* Activity log */}
            <div style={{ ...S.panel, display: "flex", flexDirection: "column" }}>
              <div style={S.panelHeader}>
                <span style={S.panelTitle}>Activity log</span>
                <span style={S.panelAction} onClick={() => setLogs([])}>Clear</span>
              </div>
              <div ref={logRef} style={S.logPanel}>
                {logs.map((l, i) => (
                  <LogEntry key={i} {...l} />
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
