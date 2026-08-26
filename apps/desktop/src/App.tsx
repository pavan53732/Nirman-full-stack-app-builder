import { useEffect, useMemo, useRef, useState } from "react";
import {
  acknowledgeSubscription,
  closeSubscription,
  dispatchCommand,
  getHandshake,
  isTauriHost,
  type CommandKind,
  type CommandRequest,
  type ProjectionSnapshot,
  ProjectionStore,
  PROTOCOL_SCHEMA_VERSION,
  safeErrorMessage,
  replayEvents,
  subscribeEvents,
  subscribeToControlEvents,
  type SessionHandshake,
} from "./ipcClient";

type NavItem = "Workspace" | "Tasks" | "Files" | "Preview" | "Logs" | "Settings";
type ConnectionState = "connecting" | "connected" | "unavailable" | "error";

const navItems: Array<{ label: NavItem; icon: string }> = [
  { label: "Workspace", icon: "⌘" },
  { label: "Tasks", icon: "◌" },
  { label: "Files", icon: "⌁" },
  { label: "Preview", icon: "▣" },
  { label: "Logs", icon: "≋" },
  { label: "Settings", icon: "⚙" },
];

const files = ["app/", "src/", "package.json", "README.md"];

function makeId(prefix: string): string {
  return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`;
}

function App() {
  const [active, setActive] = useState<NavItem>("Workspace");
  const [prompt, setPrompt] = useState("");
  const [connection, setConnection] = useState<ConnectionState>("connecting");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [lastDelivery, setLastDelivery] = useState("Waiting for the host projection");
  const [snapshot, setSnapshot] = useState<ProjectionSnapshot | null>(null);
  const [handshake, setHandshake] = useState<SessionHandshake | null>(null);
  const [commandPending, setCommandPending] = useState(false);
  const storeRef = useRef(new ProjectionStore());

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let subscriptionId: string | undefined;
    let sessionForCleanup: SessionHandshake | undefined;

    async function connect() {
      if (!isTauriHost()) {
        setConnection("unavailable");
        setErrorMessage("Open Nirman through the Windows desktop host. The browser shell has no authoritative project state.");
        return;
      }
      try {
        const session = await getHandshake();
        if (disposed) return;
        setHandshake(session);
        sessionForCleanup = session;
        const bootstrap = await subscribeEvents(session);
        if (!storeRef.current.acceptBootstrap(bootstrap)) throw new Error("authoritative subscription bootstrap was rejected");
        subscriptionId = bootstrap.subscription.subscription_id;
        setSnapshot(storeRef.current.snapshot());
        unlisten = await subscribeToControlEvents((batch) => {
          if (disposed) return;
          const result = storeRef.current.acceptEventBatch(batch);
          if (result.accepted) {
            const current = storeRef.current.snapshot();
            setLastDelivery(`Delivered ${result.events.length} ordered event(s) through Tauri`);
            setSnapshot(current);
            if (current && subscriptionId) void acknowledgeSubscription(session, subscriptionId, current.last_event_sequence).catch((error) => setErrorMessage(safeErrorMessage(error)));
          } else {
            setLastDelivery(`Event batch rejected: ${result.reason}; durable snapshot preserved`);
          }
        });
        const replay = await replayEvents(bootstrap.subscription, bootstrap.snapshot.last_event_sequence);
        const replayResult = storeRef.current.acceptEventBatch(replay);
        if (replayResult.accepted) setSnapshot(storeRef.current.snapshot());
        if (subscriptionId) await acknowledgeSubscription(session, subscriptionId, storeRef.current.snapshot()?.last_event_sequence ?? 0);
        setLastDelivery(replayResult.accepted ? `Replayed ${replayResult.events.length} event(s) from SQLite` : "Projection cursor is current");
        setConnection("connected");
        setErrorMessage(null);
      } catch (error) {
        if (!disposed) {
          setConnection("error");
          setErrorMessage(safeErrorMessage(error));
        }
      }
    }

    void connect();
    return () => {
      disposed = true;
      unlisten?.();
      if (subscriptionId && sessionForCleanup) void closeSubscription(sessionForCleanup, subscriptionId).catch(() => undefined);
    };
  }, []);

  const statusCopy = useMemo(() => {
    if (!snapshot) return "No accepted host projection yet";
    if (snapshot.preview_truth === "Observed") return `Runtime observation linked to source revision ${snapshot.current_source_revision[0]}`;
    if (snapshot.preview_truth === "Requested") return "Awaiting a real control-plane runtime observation";
    if (snapshot.preview_truth === "Stale" || snapshot.preview_truth === "Invalidated") return "Preview held at the last durable projection";
    return "No runtime evidence yet";
  }, [snapshot]);

  async function sendCommand(kind: CommandKind, payload = "") {
    if (!handshake || !snapshot || commandPending) return;
    setCommandPending(true);
    setErrorMessage(null);
    const request: CommandRequest = {
      protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
      auth: handshake.auth,
      correlation_id: handshake.correlation_id,
      causation_id: null,
      deadline_epoch_seconds: null,
      command: {
        command_id: makeId("cmd"),
        project_id: snapshot.project_id,
        task_id: null,
        kind,
        payload,
        expected_projection_revision: snapshot.projection_revision,
        idempotency_key: makeId("ui"),
      },
    };
    try {
      const response = await dispatchCommand(request);
      if (storeRef.current.acceptAuthoritativeSnapshot(response.snapshot)) setSnapshot(storeRef.current.snapshot());
      setLastDelivery(`Accepted ${kind}; waiting for the ordered event notification`);
      if (kind === "SubmitInstruction") setPrompt("");
    } catch (error) {
      setErrorMessage(safeErrorMessage(error));
    } finally {
      setCommandPending(false);
    }
  }

  function submitPrompt() {
    const next = prompt.trim();
    if (next) void sendCommand("SubmitInstruction", next);
  }

  const truth = snapshot?.preview_truth ?? "Predicted";
  const taskState = snapshot?.task_state ?? "Created";
  const continuity = snapshot?.continuity_state ?? "Reconnecting";
  const hostLabel = connection === "connected" ? "Control plane online" : connection === "connecting" ? "Connecting to host" : "Host unavailable";

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup"><div className="brand-mark">N</div><div><div className="brand-name">nirman</div><div className="brand-subtitle">Android, built locally</div></div></div>
        <button className="project-switcher" type="button"><span className="project-icon">◈</span><span className="project-copy"><strong>Orbit Notes</strong><small>Android project</small></span><span className="chevron">⌄</span></button>
        <nav className="nav-list" aria-label="Primary navigation">{navItems.map((item) => <button className={`nav-item ${active === item.label ? "active" : ""}`} key={item.label} onClick={() => setActive(item.label)} type="button"><span className="nav-icon">{item.icon}</span><span>{item.label}</span>{item.label === "Tasks" && snapshot?.task_state !== "Created" && <span className="nav-badge">1</span>}</button>)}</nav>
        <div className="sidebar-spacer" />
        <div className="control-plane-card"><div className={`status-dot ${connection !== "connected" ? "waiting" : ""}`} /><div><strong>{hostLabel}</strong><small>Authenticated local session · v{snapshot ? snapshot.projection_revision[0] : "—"}</small></div><span className="signal">◒</span></div>
        <div className="profile-row"><div className="avatar">AK</div><div><strong>Local workspace</strong><small>Windows host</small></div><span className="more">•••</span></div>
      </aside>

      <section className="workspace">
        <header className="topbar"><div><span className="eyebrow">PROJECT WORKSPACE</span><h1>Orbit Notes</h1></div><div className="topbar-actions"><span className="revision-pill">Revision <strong>{snapshot ? snapshot.projection_revision[0] : "—"}</strong></span><button className="icon-button" aria-label="Notifications" type="button">♢</button><button className="build-button" disabled={!snapshot || commandPending} onClick={() => void sendCommand("SubmitInstruction", "Build the current Android project") } type="button">Build <span>⌄</span></button></div></header>

        <div className="workspace-grid">
          <section className="chat-column panel">
            <div className="panel-heading"><div><span className="eyebrow">CONVERSATION</span><h2>Build with intent</h2></div><span className="live-badge"><i /> LIVE</span></div>
            <div className="conversation">
              <div className="message user-message"><div className="message-avatar avatar">AK</div><div><span className="message-meta">You · host projection required</span><p>{snapshot?.last_event_sequence ? `Instruction accepted at event #${String(snapshot.last_event_sequence).padStart(4, "0")}` : "Create a calm, offline-first notes app with a soft dark theme."}</p></div></div>
              <div className="message agent-message"><div className="message-avatar agent-avatar">N</div><div><span className="message-meta">Nirman · control-plane status</span><p>The UI renders accepted host state only. Runtime stages appear here after durable events and observations arrive.</p><div className="plan-card"><div className="plan-title"><span className="spark">✦</span> Current projection <span className="plan-state">{taskState.toUpperCase()}</span></div><div className="plan-row"><span className="step-number done">✓</span><span>Authoritative task state</span><span className="step-state">{taskState.toUpperCase()}</span></div><div className="plan-row"><span className="step-number">2</span><span>Background continuity</span><span className="step-state muted">{continuity.toUpperCase()}</span></div><div className="plan-row"><span className="step-number">3</span><span>Runtime observation</span><span className="step-state muted">{truth.toUpperCase()}</span></div><div className="plan-row"><span className="step-number">4</span><span>APK export evidence</span><span className="step-state muted">NOT CLAIMED</span></div></div></div></div>
            </div>
            <div className="composer"><textarea aria-label="Chat instruction" value={prompt} disabled={!snapshot || commandPending} onChange={(event) => setPrompt(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); submitPrompt(); } }} placeholder={snapshot ? "Describe the next change..." : "Waiting for the Windows host..."} /><div className="composer-footer"><span className="composer-hint">{commandPending ? "Submitting to local control plane..." : "Enter to send · Shift + Enter for new line"}</span><button className="send-button" disabled={!snapshot || commandPending} onClick={submitPrompt} type="button">↑</button></div></div>
          </section>

          <section className="preview-column panel">
            <div className="panel-heading preview-heading"><div><span className="eyebrow">LIVE PREVIEW</span><h2>Android runtime</h2></div><button className="device-selector" type="button">Pixel 8 <span>⌄</span></button></div>
            <div className={`preview-stage ${truth.toLowerCase()}`}><div className="phone-frame"><div className="phone-speaker" /><div className="phone-screen"><div className="screen-status"><span>9:41</span><span>▮ ◉ ▰</span></div><div className="app-header"><span className="app-kicker">TUESDAY, APRIL 22</span><h3>Your thoughts,<br /><em>in one place.</em></h3><span className="add-note">＋</span></div><div className="note-card primary"><span className="note-tag">TODAY</span><strong>Ideas worth keeping</strong><p>Small details become<br />meaningful memories.</p><span className="note-time">09:32</span></div><div className="note-card secondary"><span className="note-tag">PERSONAL</span><strong>Walk by the river</strong><p>Remember the blue hour.</p><span className="note-time">YESTERDAY</span></div><div className="phone-nav"><span className="selected">⌂</span><span>⌕</span><span>◌</span><span>☰</span></div></div></div>{truth !== "Observed" && <div className="preview-overlay"><div className="overlay-icon">{truth === "Stale" ? "!" : "◌"}</div><strong>{truth === "Stale" ? "Preview is stale" : "Preview not observed yet"}</strong><span>{statusCopy}</span></div>}</div>
            <div className="preview-footer"><div className="truth-label"><span className={`truth-dot ${truth.toLowerCase()}`} /><strong>{truth.toUpperCase()}</strong><span>·</span><span>{statusCopy}</span></div><div className="preview-actions"><button disabled={!snapshot || commandPending || taskState === "Created"} onClick={() => void sendCommand("PauseTask")} type="button">Pause</button><button disabled={!snapshot || commandPending || taskState !== "Paused"} onClick={() => void sendCommand("ResumeTask")} type="button">Resume</button><button disabled={!snapshot || commandPending || taskState === "Cancelled" || taskState === "Completed"} onClick={() => void sendCommand("CancelTask")} type="button">Cancel</button></div></div>
          </section>

          <section className="files-column panel"><div className="panel-heading"><div><span className="eyebrow">PROJECT SURFACE</span><h2>Files & evidence</h2></div><button className="small-action" type="button">＋</button></div><div className="file-list">{files.map((file, index) => <button className="file-row" key={file} type="button"><span className={`file-icon ${index < 2 ? "folder" : "doc"}`}>{index < 2 ? "⌄" : "·"}</span><span>{file}</span>{index === 2 && <span className="file-status">host-owned</span>}</button>)}</div><div className="evidence-section"><div className="section-label">LATEST EVIDENCE</div><div className="evidence-row"><span className="evidence-icon">{snapshot?.last_event_sequence ? "✓" : "○"}</span><div><strong>Control-plane projection</strong><small>{snapshot ? `Durable event cursor #${String(snapshot.last_event_sequence).padStart(4, "0")}` : "Waiting for host snapshot"}</small></div><span className={snapshot ? "verified" : "waiting"}>{snapshot ? "VALID" : "WAITING"}</span></div><div className="evidence-row dim"><span className="evidence-icon">○</span><div><strong>Android runtime</strong><small>Waiting for local observation</small></div><span className="waiting">WAITING</span></div></div></section>
        </div>
        <footer className="bottom-status"><span><i className={`status-dot ${connection !== "connected" ? "waiting" : ""}`} /> {connection === "connected" ? "Local control plane connected" : connection === "connecting" ? "Connecting to local control plane" : "Local control plane unavailable"}</span><span>{lastDelivery}</span><span className="status-right">{errorMessage ?? "Tauri IPC · SQLite projection · Runtime implementation in progress"}</span></footer>
      </section>
    </main>
  );
}

export default App;
