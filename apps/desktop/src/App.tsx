import { useMemo, useState } from "react";

type NavItem = "Workspace" | "Tasks" | "Files" | "Preview" | "Logs" | "Settings";

type PreviewTruth = "PREDICTED" | "REQUESTED" | "OBSERVED" | "STALE";

const navItems: Array<{ label: NavItem; icon: string }> = [
  { label: "Workspace", icon: "⌘" },
  { label: "Tasks", icon: "◌" },
  { label: "Files", icon: "⌁" },
  { label: "Preview", icon: "▣" },
  { label: "Logs", icon: "≋" },
  { label: "Settings", icon: "⚙" },
];

const files = ["app/", "src/", "package.json", "README.md"];

function App() {
  const [active, setActive] = useState<NavItem>("Workspace");
  const [prompt, setPrompt] = useState("");
  const [submittedPrompt, setSubmittedPrompt] = useState("");
  const [truth, setTruth] = useState<PreviewTruth>("PREDICTED");

  const statusCopy = useMemo(() => {
    if (truth === "OBSERVED") return "Runtime observation linked to revision 0.1";
    if (truth === "REQUESTED") return "Awaiting control-plane execution";
    if (truth === "STALE") return "Preview held at last durable projection";
    return "No runtime evidence yet";
  }, [truth]);

  function submitPrompt() {
    const next = prompt.trim();
    if (!next) return;
    setSubmittedPrompt(next);
    setPrompt("");
    setTruth("REQUESTED");
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup">
          <div className="brand-mark">N</div>
          <div>
            <div className="brand-name">nirman</div>
            <div className="brand-subtitle">Android, built locally</div>
          </div>
        </div>

        <button className="project-switcher" type="button">
          <span className="project-icon">◈</span>
          <span className="project-copy"><strong>Orbit Notes</strong><small>Android project</small></span>
          <span className="chevron">⌄</span>
        </button>

        <nav className="nav-list" aria-label="Primary navigation">
          {navItems.map((item) => (
            <button className={`nav-item ${active === item.label ? "active" : ""}`} key={item.label} onClick={() => setActive(item.label)} type="button">
              <span className="nav-icon">{item.icon}</span>
              <span>{item.label}</span>
              {item.label === "Tasks" && <span className="nav-badge">1</span>}
            </button>
          ))}
        </nav>

        <div className="sidebar-spacer" />
        <div className="control-plane-card">
          <div className="status-dot" />
          <div><strong>Control plane online</strong><small>Local supervisor · v0.1</small></div>
          <span className="signal">◒</span>
        </div>
        <div className="profile-row"><div className="avatar">AK</div><div><strong>Local workspace</strong><small>Windows host</small></div><span className="more">•••</span></div>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div><span className="eyebrow">PROJECT WORKSPACE</span><h1>Orbit Notes</h1></div>
          <div className="topbar-actions"><span className="revision-pill">Revision <strong>0.1</strong></span><button className="icon-button" aria-label="Notifications" type="button">♢</button><button className="build-button" type="button">Build <span>⌄</span></button></div>
        </header>

        <div className="workspace-grid">
          <section className="chat-column panel">
            <div className="panel-heading"><div><span className="eyebrow">CONVERSATION</span><h2>Build with intent</h2></div><span className="live-badge"><i /> LIVE</span></div>
            <div className="conversation">
              <div className="message user-message"><div className="message-avatar avatar">AK</div><div><span className="message-meta">You · just now</span><p>{submittedPrompt || "Create a calm, offline-first notes app with a soft dark theme."}</p></div></div>
              <div className="message agent-message"><div className="message-avatar agent-avatar">N</div><div><span className="message-meta">Nirman · structured response</span><p>I’ll turn this into an Android project, checkpoint the workspace, and validate the result on a local runtime. The control plane will report each real stage here.</p><div className="plan-card"><div className="plan-title"><span className="spark">✦</span> Current plan <span className="plan-state">AWAITING EXECUTION</span></div><div className="plan-row"><span className="step-number done">✓</span><span>Understand product intent</span><span className="step-state">READY</span></div><div className="plan-row"><span className="step-number">2</span><span>Create a source checkpoint</span><span className="step-state muted">QUEUED</span></div><div className="plan-row"><span className="step-number">3</span><span>Build and observe Android runtime</span><span className="step-state muted">QUEUED</span></div><div className="plan-row"><span className="step-number">4</span><span>Validate and prepare APK</span><span className="step-state muted">QUEUED</span></div></div></div></div>
            </div>
            <div className="composer"><textarea aria-label="Chat instruction" value={prompt} onChange={(event) => setPrompt(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); submitPrompt(); } }} placeholder="Describe the next change..." /><div className="composer-footer"><span className="composer-hint">Enter to send · Shift + Enter for new line</span><button className="send-button" onClick={submitPrompt} type="button">↑</button></div></div>
          </section>

          <section className="preview-column panel">
            <div className="panel-heading preview-heading"><div><span className="eyebrow">LIVE PREVIEW</span><h2>Android runtime</h2></div><button className="device-selector" type="button">Pixel 8 <span>⌄</span></button></div>
            <div className={`preview-stage ${truth.toLowerCase()}`}>
              <div className="phone-frame"><div className="phone-speaker" /><div className="phone-screen"><div className="screen-status"><span>9:41</span><span>▮ ◉ ▰</span></div><div className="app-header"><span className="app-kicker">TUESDAY, APRIL 22</span><h3>Your thoughts,<br /><em>in one place.</em></h3><span className="add-note">＋</span></div><div className="note-card primary"><span className="note-tag">TODAY</span><strong>Ideas worth keeping</strong><p>Small details become<br />meaningful memories.</p><span className="note-time">09:32</span></div><div className="note-card secondary"><span className="note-tag">PERSONAL</span><strong>Walk by the river</strong><p>Remember the blue hour.</p><span className="note-time">YESTERDAY</span></div><div className="phone-nav"><span className="selected">⌂</span><span>⌕</span><span>◌</span><span>☰</span></div></div></div>
              {truth !== "OBSERVED" && <div className="preview-overlay"><div className="overlay-icon">{truth === "STALE" ? "!" : "◌"}</div><strong>{truth === "STALE" ? "Preview is stale" : "Preview not observed yet"}</strong><span>{statusCopy}</span></div>}
            </div>
            <div className="preview-footer"><div className="truth-label"><span className={`truth-dot ${truth.toLowerCase()}`} /> <strong>{truth}</strong><span>·</span><span>{statusCopy}</span></div><div className="preview-actions"><button onClick={() => setTruth("OBSERVED")} type="button">Mark fixture observed</button><button onClick={() => setTruth("STALE")} type="button">Simulate stale</button></div></div>
          </section>

          <section className="files-column panel"><div className="panel-heading"><div><span className="eyebrow">PROJECT SURFACE</span><h2>Files & evidence</h2></div><button className="small-action" type="button">＋</button></div><div className="file-list">{files.map((file, index) => <button className="file-row" key={file} type="button"><span className={`file-icon ${index < 2 ? "folder" : "doc"}`}>{index < 2 ? "⌄" : "·"}</span><span>{file}</span>{index === 2 && <span className="file-status">edited</span>}</button>)}</div><div className="evidence-section"><div className="section-label">LATEST EVIDENCE</div><div className="evidence-row"><span className="evidence-icon">✓</span><div><strong>Intent contract</strong><small>Recorded · source event #0001</small></div><span className="verified">VALID</span></div><div className="evidence-row dim"><span className="evidence-icon">○</span><div><strong>Android runtime</strong><small>Waiting for local observation</small></div><span className="waiting">WAITING</span></div></div></section>
        </div>

        <footer className="bottom-status"><span><i className="status-dot" /> Local control plane connected</span><span>SQLite ledger ready</span><span className="status-right">Documentation certified · Runtime implementation in progress</span></footer>
      </section>
    </main>
  );
}

export default App;
