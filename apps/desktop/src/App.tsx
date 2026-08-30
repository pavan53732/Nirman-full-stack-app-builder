import { useEffect, useMemo, useRef, useState } from "react";
import {
  acknowledgeSubscription,
  closeSubscription,
  createAndroidConstructionContract,
  dispatchCommand,
  exportArtifact,
  getHandshake,
  getWorkspace,
  isTauriHost,
  type AgentLoopRunResultPayload,
  type AndroidConstructionContract,
  type ArtifactExportResultPayload,
  type CommandKind,
  type CommandRequest,
  type CommandResponse,
  type PreviewStartResultPayload,
  type ProjectionSnapshot,
  ProjectionStore,
  PROTOCOL_SCHEMA_VERSION,
  preflightAndroidToolchain,
  type ProjectId,
  readPreviewEvidence,
  runAgentLoop,
  safeErrorMessage,
  replayEvents,
  startPreview,
  subscribeEvents,
  subscribeToControlEvents,
  type SessionHandshake,
} from "./ipcClient";
import { deriveConstructionPipeline, derivePreviewRequest, exportDestination } from "./contract";

type NavItem = "Workspace" | "Tasks" | "Files" | "Preview" | "Logs" | "Settings";
type ConnectionState = "connecting" | "connected" | "unavailable" | "error";
type StepStatus = "pending" | "running" | "done" | "failed" | "cancelled";

interface PipelineStep {
  label: string;
  detail: string;
  status: StepStatus;
}

interface TranscriptEntry {
  who: "user" | "agent";
  text: string;
}

const navItems: Array<{ label: NavItem; icon: string }> = [
  { label: "Workspace", icon: "⌘" },
  { label: "Tasks", icon: "◌" },
  { label: "Files", icon: "⌁" },
  { label: "Preview", icon: "▣" },
  { label: "Logs", icon: "≋" },
  { label: "Settings", icon: "⚙" },
];

const DEFAULT_INTENT = "Create a calm, offline-first notes app with a soft dark theme.";

function makeId(prefix: string): string {
  return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`;
}

function statusIcon(status: StepStatus): string {
  switch (status) {
    case "done": return "✓";
    case "running": return "◐";
    case "failed": return "✕";
    case "cancelled": return "⊘";
    default: return "○";
  }
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
  const [workspaceRoot, setWorkspaceRoot] = useState<string | null>(null);
  const [pipeline, setPipeline] = useState<PipelineStep[]>([]);
  const [pipelineRunning, setPipelineRunning] = useState(false);
  const [loopResult, setLoopResult] = useState<AgentLoopRunResultPayload | null>(null);
  const [exportResult, setExportResult] = useState<ArtifactExportResultPayload | null>(null);
  const [previewResult, setPreviewResult] = useState<PreviewStartResultPayload | null>(null);
  const [screenshotDataUrl, setScreenshotDataUrl] = useState<string | null>(null);
  const [logcatText, setLogcatText] = useState<string | null>(null);
  const [deviceSerial, setDeviceSerial] = useState("");
  const [previewBusy, setPreviewBusy] = useState(false);
  const [transcript, setTranscript] = useState<TranscriptEntry[]>([
    { who: "agent", text: "Describe the Android app you want. Nirman derives a construction contract, scaffolds a real Gradle project, builds the APK through the agent loop, and exports the verified artifact." },
  ]);
  const storeRef = useRef(new ProjectionStore());
  const pipelineTaskRef = useRef<ProjectId | null>(null);
  const contractRef = useRef<AndroidConstructionContract | null>(null);
  const pipelineIdRef = useRef<string | null>(null);

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
        const workspace = await getWorkspace();
        if (disposed) return;
        setWorkspaceRoot(workspace.workspace_root);
        if (!workspace.workspace_root) {
          setErrorMessage("No authorized project workspace is configured on the host (NIRMAN_PROJECT_WORKSPACE). Android builds stay disabled until one is configured.");
        }
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
        task_id: pipelineTaskRef.current,
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
    } catch (error) {
      setErrorMessage(safeErrorMessage(error));
    } finally {
      setCommandPending(false);
    }
  }

  function updateStep(steps: PipelineStep[], index: number, patch: Partial<PipelineStep>): PipelineStep[] {
    return steps.map((step, position) => (position === index ? { ...step, ...patch } : step));
  }

  /** The real user path: instruction → contract → toolchain preflight →
   * AgentLoopRun (synthesis → scaffold → Gradle build → diagnosis/retry →
   * artifact validation). Every command goes through the authenticated
   * control plane; nothing is mocked. */
  async function runBuildPipeline(rawIntent: string) {
    if (!handshake || !snapshot || pipelineRunning) return;
    const intent = rawIntent.trim() || DEFAULT_INTENT;
    if (!workspaceRoot) {
      setErrorMessage("No authorized project workspace is configured on the host; the agent loop cannot build.");
      return;
    }
    const pipelineId = makeId("ui").replace("ui-", "");
    const taskId: ProjectId = [`task-${pipelineId}`];
    pipelineTaskRef.current = taskId;
    pipelineIdRef.current = pipelineId;
    setPipelineRunning(true);
    setCommandPending(true);
    setErrorMessage(null);
    setLoopResult(null);
    setExportResult(null);
    setPreviewResult(null);
    setScreenshotDataUrl(null);
    setLogcatText(null);
    setTranscript((current) => [...current, { who: "user", text: intent }]);
    const steps: PipelineStep[] = [
      { label: "Submit instruction", detail: "Durable intent event + background run", status: "pending" },
      { label: "Derive contract", detail: "AndroidConstructionCreate (validated)", status: "pending" },
      { label: "Toolchain preflight", detail: "M43 lock + environment snapshot", status: "pending" },
      { label: "Agent loop", detail: "Synthesis → scaffold → Gradle → validate", status: "pending" },
    ];
    setPipeline(steps.map((step) => ({ ...step })));

    const derive = deriveConstructionPipeline({
      projectId: snapshot.project_id[0],
      taskId: taskId[0],
      intent,
      pipelineId,
    });
    contractRef.current = derive.contract;
    try {
      // Step 1: durable instruction (also opens the task's background run).
      setPipeline((current) => updateStep(current, 0, { status: "running" }));
      const instructionRequest: CommandRequest = {
        protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
        auth: handshake.auth,
        correlation_id: handshake.correlation_id,
        causation_id: null,
        deadline_epoch_seconds: null,
        command: {
          command_id: makeId("instruction"),
          project_id: snapshot.project_id,
          task_id: taskId,
          kind: "SubmitInstruction",
          payload: intent,
          expected_projection_revision: snapshot.projection_revision,
          idempotency_key: makeId("ui"),
        },
      };
      const instruction = await dispatchCommand(instructionRequest);
      if (!storeRef.current.acceptAuthoritativeSnapshot(instruction.snapshot)) throw new Error("instruction snapshot was rejected as non-authoritative");
      setSnapshot(storeRef.current.snapshot());
      setPipeline((current) => updateStep(current, 0, { status: "done" }));

      const current = () => storeRef.current.snapshot();
      if (!current()) throw new Error("projection snapshot lost");

      // Step 2: the derived construction contract.
      setPipeline((step) => updateStep(step, 1, { status: "running" }));
      const contractResponse = await createAndroidConstructionContract(handshake, current()!, taskId, derive.contract);
      if (!storeRef.current.acceptAuthoritativeSnapshot(contractResponse.snapshot)) throw new Error("contract snapshot was rejected as non-authoritative");
      setPipeline((step) => updateStep(step, 1, { status: "done", detail: `${derive.contract.contractId} · ${derive.contract.features.length} feature(s)` }));

      // Step 3: M43 toolchain preflight + lock.
      setPipeline((step) => updateStep(step, 2, { status: "running" }));
      const preflightResponse = await preflightAndroidToolchain(handshake, current()!, taskId, derive.buildVariant);
      if (!storeRef.current.acceptAuthoritativeSnapshot(preflightResponse.snapshot)) throw new Error("preflight snapshot was rejected as non-authoritative");
      setPipeline((step) => updateStep(step, 2, { status: "done" }));

      // Step 4: the agent loop drives the real authorities.
      setPipeline((step) => updateStep(step, 3, { status: "running" }));
      const sourceRevision = Math.max(1, current()!.current_source_revision[0]);
      const loopResponse = await runAgentLoop(handshake, current()!, taskId, {
        contract: derive.contract,
        source_revision: sourceRevision,
        workspace_root: workspaceRoot,
        build_variant: derive.buildVariant,
        gradle_task: derive.gradleTask,
        iteration_budget: derive.iterationBudget,
        build_timeout_ms: derive.buildTimeoutMs,
      });
      if (!storeRef.current.acceptAuthoritativeSnapshot(loopResponse.snapshot)) throw new Error("agent loop snapshot was rejected as non-authoritative");
      const loopPayload = loopResponse.result_payload as AgentLoopRunResultPayload | null;
      if (!loopPayload) throw new Error("agent loop returned no result payload");
      setLoopResult(loopPayload);
      const artifact = loopPayload.build_observation?.artifact_path ?? null;
      if (loopPayload.outcome === "COMPLETE") {
        setPipeline((step) => updateStep(step, 3, { status: "done", detail: `COMPLETE in ${loopPayload.loop_record.iteration} action(s) · APK ${artifact ?? "produced"}` }));
        setTranscript((entries) => [...entries, { who: "agent", text: `Loop ${loopPayload.loop_record.loop_id} completed: synthesized the plan, scaffolded ${loopPayload.scaffold?.file_count ?? 0} Gradle project files (${loopPayload.scaffold?.package_name ?? "app"}), built and validated the APK, checksum ${loopPayload.build_observation?.artifact_sha256 ?? "unavailable"}.` }]);
      } else if (loopPayload.outcome === "CANCELLED") {
        setPipeline((step) => updateStep(step, 3, { status: "cancelled", detail: "Cancelled: no further actions executed" }));
        setTranscript((entries) => [...entries, { who: "agent", text: "The loop observed the task cancellation and stopped safely; the workspace was left at its last durable state." }]);
      } else {
        setPipeline((step) => updateStep(step, 3, { status: "failed", detail: `${loopPayload.outcome} after ${loopPayload.loop_record.iteration} action(s) · ${loopPayload.loop_record.retry_strategy}` }));
        setTranscript((entries) => [...entries, { who: "agent", text: `The loop ended ${loopPayload.outcome}: ${loopPayload.loop_record.retry_strategy}. Diagnosis selected variations across ${loopPayload.loop_record.variation_attempts} attempt(s); nothing was claimed beyond the durable evidence.` }]);
      }
      setPrompt("");
    } catch (error) {
      const message = safeErrorMessage(error);
      setErrorMessage(message);
      setTranscript((entries) => [...entries, { who: "agent", text: `The control plane rejected the pipeline: ${message}` }]);
      setPipeline((currentSteps) => {
        const index = currentSteps.findIndex((step) => step.status === "running");
        if (index === -1) return currentSteps;
        return updateStep(currentSteps, index, { status: "failed", detail: message });
      });
    } finally {
      setPipelineRunning(false);
      setCommandPending(false);
    }
  }

  function submitPrompt() {
    const next = prompt.trim();
    if (next) void runBuildPipeline(next);
  }

  /** Exports the loop-built APK through the M10/M11 delivery path. */
  async function exportApk() {
    if (!handshake || !snapshot || !loopResult || !workspaceRoot || commandPending) return;
    const taskId = pipelineTaskRef.current;
    const pipelineId = taskId ? taskId[0].replace("task-", "") : "current";
    const destination = exportDestination(workspaceRoot, pipelineId, loopResult.build_observation?.build_variant ?? "debug");
    setCommandPending(true);
    setErrorMessage(null);
    try {
      const response = await exportArtifact(handshake, snapshot, taskId!, {
        source_revision: loopResult.build_observation?.source_revision ?? snapshot.current_source_revision[0],
        destination_path: destination,
        packaging_profile_id: "debug-local",
        artifact_kind: "APK",
        request_fingerprint: loopResult.build_observation?.artifact_sha256 ?? loopResult.resulting_project_fingerprint ?? "unknown",
        idempotency_key: `export-${pipelineId}`,
        deployment_delivery: "REQUIRED_APK",
        destination_kind: "LOCAL_WINDOWS_FILESYSTEM",
      }, makeId("artifact-export"));
      if (storeRef.current.acceptAuthoritativeSnapshot(response.snapshot)) setSnapshot(storeRef.current.snapshot());
      const payload = response.result_payload as ArtifactExportResultPayload | null;
      if (!payload) throw new Error("artifact export returned no result payload");
      setExportResult(payload);
      setLastDelivery(`Export delivered ${payload.artifact.delivery_status} to ${payload.artifact.path}`);
    } catch (error) {
      setErrorMessage(safeErrorMessage(error));
    } finally {
      setCommandPending(false);
    }
  }

  /** Starts the real M48 preview pipeline. With an adb serial bound, the host
   * installs the exported APK on that device, launches it, and records the
   * device observation (screenshot, logcat, UI dump). Without a serial, the
   * host selects the headless smoke-test fallback — no runtime session is
   * claimed. Every result below comes from the durable control plane. */
  async function runDevicePreview() {
    if (!handshake || !snapshot || commandPending || previewBusy) return;
    const taskId = pipelineTaskRef.current;
    const contract = contractRef.current;
    const pipelineId = pipelineIdRef.current;
    if (!taskId || !contract || !pipelineId || !loopResult) {
      setErrorMessage("Run a build first; preview binds the loop's committed source revision.");
      return;
    }
    const serial = deviceSerial.trim();
    if (serial.length > 0 && !exportResult) {
      setErrorMessage("Device preview requires the exported APK; export the artifact first.");
      return;
    }
    setPreviewBusy(true);
    setErrorMessage(null);
    setScreenshotDataUrl(null);
    setLogcatText(null);
    try {
      const request = derivePreviewRequest({
        contract,
        pipelineId,
        sourceRevision: Math.max(0, snapshot.current_source_revision[0]),
        sourceFingerprint: loopResult.build_observation?.artifact_sha256 ?? loopResult.resulting_project_fingerprint ?? `source-${snapshot.current_source_revision[0]}`,
        buildVariant: loopResult.build_observation?.build_variant ?? "debug",
        deviceSerial: serial,
      });
      const response = await startPreview(handshake, snapshot, taskId, { request });
      if (!storeRef.current.acceptAuthoritativeSnapshot(response.snapshot)) throw new Error("preview snapshot was rejected as non-authoritative");
      setSnapshot(storeRef.current.snapshot());
      const payload = response.result_payload as PreviewStartResultPayload | null;
      if (!payload) throw new Error("preview start returned no result payload");
      setPreviewResult(payload);
      const observation = payload.device_observation;
      if (observation) {
        const screenshotReference = observation.screenshot_references[0];
        if (screenshotReference) {
          const evidence = await readPreviewEvidence(handshake, screenshotReference);
          if (evidence.kind === "image" && evidence.data_base64) setScreenshotDataUrl(`data:${evidence.mime};base64,${evidence.data_base64}`);
        }
        if (observation.logcat_reference) {
          const evidence = await readPreviewEvidence(handshake, observation.logcat_reference);
          if (evidence.kind === "text" && evidence.text !== null) setLogcatText(evidence.text);
        }
        setTranscript((entries) => [...entries, { who: "agent", text: `Preview observed on ${observation.device_identity}: ${observation.install_status.toLowerCase()}/install, ${observation.launch_status.toLowerCase()}/launch, ${observation.interaction_status.toLowerCase()}/interaction — evidence ${observation.observation_id} bound to source revision ${snapshot.current_source_revision[0]}.` }]);
      } else {
        setTranscript((entries) => [...entries, { who: "agent", text: `Preview fallback ${payload.selection.mode}: ${payload.selection.reason}. Runtime observation not required at this rank.` }]);
      }
      setLastDelivery(`PreviewStart ${payload.selection.mode} accepted for source revision ${snapshot.current_source_revision[0]}`);
    } catch (error) {
      setErrorMessage(safeErrorMessage(error));
      setTranscript((entries) => [...entries, { who: "agent", text: `Preview rejected by the control plane: ${safeErrorMessage(error)}` }]);
    } finally {
      setPreviewBusy(false);
    }
  }

  const truth = snapshot?.preview_truth ?? "Predicted";
  const taskState = snapshot?.task_state ?? "Created";
  const continuity = snapshot?.continuity_state ?? "Reconnecting";
  const hostLabel = connection === "connected" ? "Control plane online" : connection === "connecting" ? "Connecting to host" : "Host unavailable";
  const canExport = !pipelineRunning && loopResult?.outcome === "COMPLETE" && !!loopResult.build_observation?.artifact_path && !!workspaceRoot;

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup"><div className="brand-mark">N</div><div><div className="brand-name">nirman</div><div className="brand-subtitle">Android, built locally</div></div></div>
        <button className="project-switcher" type="button"><span className="project-icon">◈</span><span className="project-copy"><strong>Orbit Notes</strong><small>Android project</small></span><span className="chevron">⌄</span></button>
        <nav className="nav-list" aria-label="Primary navigation">{navItems.map((item) => <button className={`nav-item ${active === item.label ? "active" : ""}`} key={item.label} onClick={() => setActive(item.label)} type="button"><span className="nav-icon">{item.icon}</span><span>{item.label}</span>{item.label === "Tasks" && snapshot?.task_state !== "Created" && <span className="nav-badge">1</span>}</button>)}</nav>
        <div className="sidebar-spacer" />
        <div className="control-plane-card"><div className={`status-dot ${connection !== "connected" ? "waiting" : ""}`} /><div><strong>{hostLabel}</strong><small>Authenticated local session · v{snapshot ? snapshot.projection_revision[0] : "—"}</small></div><span className="signal">◒</span></div>
        <div className="profile-row"><div className="avatar">AK</div><div><strong>Local workspace</strong><small>{workspaceRoot ? "Authorized workspace" : "No workspace"}</small></div><span className="more">•••</span></div>
      </aside>

      <section className="workspace">
        <header className="topbar"><div><span className="eyebrow">PROJECT WORKSPACE</span><h1>Orbit Notes</h1></div><div className="topbar-actions"><span className="revision-pill">Revision <strong>{snapshot ? snapshot.projection_revision[0] : "—"}</strong></span><button className="icon-button" aria-label="Notifications" type="button">♢</button><button className="build-button" disabled={!snapshot || pipelineRunning || !workspaceRoot} onClick={() => void runBuildPipeline(prompt || DEFAULT_INTENT)} type="button">Build <span>⌄</span></button></div></header>

        <div className="workspace-grid">
          <section className="chat-column panel">
            <div className="panel-heading"><div><span className="eyebrow">CONVERSATION</span><h2>Build with intent</h2></div><span className="live-badge"><i /> LIVE</span></div>
            <div className="conversation">
              {transcript.slice(-6).map((entry, index) => (
                <div className={`message ${entry.who === "user" ? "user-message" : "agent-message"}`} key={index}>
                  <div className={`message-avatar ${entry.who === "user" ? "" : "agent-avatar"}`}>{entry.who === "user" ? "AK" : "N"}</div>
                  <div><span className="message-meta">{entry.who === "user" ? "You · durable instruction" : "Nirman · agent loop evidence"}</span><p>{entry.text}</p></div>
                </div>
              ))}
              <div className="message agent-message"><div className="message-avatar agent-avatar">N</div><div><span className="message-meta">Nirman · control-plane status</span><p>The UI renders accepted host state only. Each build runs the real pipeline behind the authenticated control plane.</p>
                <div className="plan-card">
                  <div className="plan-title"><span className="spark">✦</span> Current projection <span className="plan-state">{taskState.toUpperCase()}</span></div>
                  {pipeline.length === 0 && (
                    <>
                      <div className="plan-row"><span className="step-number">1</span><span>Authoritative task state</span><span className="step-state">{taskState.toUpperCase()}</span></div>
                      <div className="plan-row"><span className="step-number">2</span><span>Background continuity</span><span className="step-state muted">{continuity.toUpperCase()}</span></div>
                      <div className="plan-row"><span className="step-number">3</span><span>Runtime observation</span><span className="step-state muted">{truth.toUpperCase()}</span></div>
                      <div className="plan-row"><span className="step-number">4</span><span>APK export evidence</span><span className="step-state muted">NOT CLAIMED</span></div>
                    </>
                  )}
                  {pipeline.map((step, index) => (
                    <div className="plan-row" key={step.label}>
                      <span className={`step-number ${step.status === "done" ? "done" : step.status === "failed" ? "failed" : ""}`}>{statusIcon(step.status)}</span>
                      <span>{step.label}<small style={{ display: "block", opacity: 0.65 }}>{step.detail}</small></span>
                      <span className={`step-state ${step.status === "pending" ? "muted" : ""}`}>{step.status.toUpperCase()}</span>
                    </div>
                  ))}
                </div>
              </div></div>
            </div>
            <div className="composer"><textarea aria-label="Chat instruction" value={prompt} disabled={!snapshot || pipelineRunning} onChange={(event) => setPrompt(event.target.value)} onKeyDown={(event) => { if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); submitPrompt(); } }} placeholder={snapshot ? (workspaceRoot ? "Describe the Android app to build..." : "Waiting for an authorized workspace (NIRMAN_PROJECT_WORKSPACE)...") : "Waiting for the Windows host..."} /><div className="composer-footer"><span className="composer-hint">{pipelineRunning ? "Agent loop running through the local control plane..." : commandPending ? "Submitting to local control plane..." : "Enter to send · Shift + Enter for new line"}</span><button className="send-button" disabled={!snapshot || pipelineRunning || !workspaceRoot} onClick={submitPrompt} type="button">↑</button></div></div>
          </section>

          <section className="preview-column panel">
            <div className="panel-heading preview-heading"><div><span className="eyebrow">LIVE PREVIEW</span><h2>Android runtime</h2></div><input aria-label="Device serial (adb devices)" className="device-input" disabled={!snapshot || previewBusy || pipelineRunning} onChange={(event) => setDeviceSerial(event.target.value)} placeholder="adb serial (e.g. emulator-5554) · empty = headless" title="Real adb device serial — must match `adb devices`" value={deviceSerial} /></div>
            <div className={`preview-stage ${truth.toLowerCase()}`}>{screenshotDataUrl ? <div className="phone-frame"><div className="phone-speaker" /><div className="phone-screen real-capture"><img alt="Real device screenshot captured by the preview session" src={screenshotDataUrl} /></div></div> : previewResult?.device_observation ? <div className="phone-frame"><div className="phone-speaker" /><div className="phone-screen observed-placeholder"><span className="overlay-icon">◉</span><strong>Observed without a screenshot</strong><span>{previewResult.device_observation.device_identity} · {previewResult.device_observation.package_name}</span></div></div> : <div className="phone-frame"><div className="phone-speaker" /><div className="phone-screen"><div className="screen-status"><span>9:41</span><span>▮ ◉ ▰</span></div><div className="app-header"><span className="app-kicker">TUESDAY, APRIL 22</span><h3>Your thoughts,<br /><em>in one place.</em></h3><span className="add-note">＋</span></div><div className="note-card primary"><span className="note-tag">TODAY</span><strong>Ideas worth keeping</strong><p>Small details become<br />meaningful memories.</p><span className="note-time">09:32</span></div><div className="note-card secondary"><span className="note-tag">PERSONAL</span><strong>Walk by the river</strong><p>Remember the blue hour.</p><span className="note-time">YESTERDAY</span></div><div className="phone-nav"><span className="selected">⌂</span><span>⌕</span><span>◌</span><span>☰</span></div></div></div>}{truth !== "Observed" && <div className="preview-overlay"><div className="overlay-icon">{truth === "Stale" ? "!" : "◌"}</div><strong>{truth === "Stale" ? "Preview is stale" : "Preview not observed yet"}</strong><span>{statusCopy}</span></div>}</div>
            {previewResult && (
              <div className="preview-observation">
                <div className="observation-row"><span>Mode</span><strong>{previewResult.selection.mode}</strong><small>{previewResult.selection.reason}</small></div>
                {previewResult.device_observation ? (<>
                  <div className="observation-row"><span>Device</span><strong>{previewResult.device_observation.device_identity}</strong><small>{previewResult.device_observation.package_name} · APK {previewResult.device_observation.apk_sha256.slice(0, 16)}…</small></div>
                  <div className="observation-row"><span>Session</span><strong>{previewResult.device_observation.install_status} · {previewResult.device_observation.launch_status} · {previewResult.device_observation.interaction_status}</strong><small>observation {previewResult.device_observation.observation_id} · evidence {previewResult.device_observation.screenshot_references.length} screenshot(s)</small></div>
                </>) : (<div className="observation-row"><span>Revision</span><strong>{previewResult.revision.lifecycle_state}</strong><small>{previewResult.revision.preview_mode} bound to {previewResult.revision.project_revision_id}</small></div>)}
                {logcatText !== null && logcatText.trim().length > 0 && (<details className="logcat-details"><summary>Device logcat (filtered, {logcatText.split("\n").length} line(s))</summary><pre>{logcatText}</pre></details>)}
              </div>
            )}
            <div className="preview-footer"><div className="truth-label"><span className={`truth-dot ${truth.toLowerCase()}`} /><strong>{truth.toUpperCase()}</strong><span>·</span><span>{statusCopy}</span></div><div className="preview-actions"><button className="preview-run" disabled={!snapshot || commandPending || previewBusy || pipelineRunning || loopResult?.outcome !== "COMPLETE"} onClick={() => void runDevicePreview()} type="button">{previewBusy ? "Observing…" : "Run preview"}</button><button disabled={!snapshot || commandPending && !pipelineRunning || taskState === "Created"} onClick={() => void sendCommand("PauseTask")} type="button">Pause</button><button disabled={!snapshot || commandPending && !pipelineRunning || taskState !== "Paused"} onClick={() => void sendCommand("ResumeTask")} type="button">Resume</button><button disabled={!snapshot || taskState === "Cancelled" || taskState === "Completed"} onClick={() => void sendCommand("CancelTask")} type="button">Cancel</button></div></div>
          </section>

          <section className="files-column panel"><div className="panel-heading"><div><span className="eyebrow">PROJECT SURFACE</span><h2>Files & evidence</h2></div><button className="small-action" type="button">＋</button></div><div className="file-list">
            {(loopResult?.scaffold ? ["settings.gradle.kts", "app/build.gradle.kts", "app/src/main/AndroidManifest.xml", "MainActivity.kt (Compose)", loopResult.build_observation?.artifact_path?.split("/").pop() ?? "app-debug.apk"] : ["app/", "src/", "package.json", "README.md"]).map((file, index) => <button className="file-row" key={file} type="button"><span className={`file-icon ${index < 2 ? "folder" : "doc"}`}>{index < 2 ? "⌄" : "·"}</span><span>{file}</span>{!loopResult && index === 2 && <span className="file-status">host-owned</span>}</button>)}
          </div><div className="evidence-section"><div className="section-label">LATEST EVIDENCE</div><div className="evidence-row"><span className="evidence-icon">{snapshot?.last_event_sequence ? "✓" : "○"}</span><div><strong>Control-plane projection</strong><small>{snapshot ? `Durable event cursor #${String(snapshot.last_event_sequence).padStart(4, "0")}` : "Waiting for host snapshot"}</small></div><span className={snapshot ? "verified" : "waiting"}>{snapshot ? "VALID" : "WAITING"}</span></div>
            <div className="evidence-row"><span className="evidence-icon">{loopResult?.outcome === "COMPLETE" ? "✓" : "○"}</span><div><strong>Android build</strong><small>{loopResult ? `${loopResult.outcome} · iteration ${loopResult.loop_record.iteration}/${loopResult.loop_record.iteration_budget}` : "Waiting for an agent loop run"}</small></div><span className={loopResult?.outcome === "COMPLETE" ? "verified" : "waiting"}>{loopResult ? (loopResult.outcome === "COMPLETE" ? "VALID" : loopResult.outcome) : "WAITING"}</span></div>
            <div className="evidence-row dim"><span className="evidence-icon">{exportResult ? "✓" : "○"}</span><div><strong>APK delivery</strong><small>{exportResult ? `${exportResult.delivery_record.state} · ${exportResult.delivery_record.destinationPath}` : loopResult?.outcome === "COMPLETE" ? "Ready to export" : "Waiting for a validated APK"}</small></div><span className={exportResult ? "verified" : "waiting"}>{exportResult ? exportResult.delivery_record.state : "WAITING"}</span></div>
            {snapshot?.delivery_projection && <div className="evidence-row dim"><span className="evidence-icon">{snapshot.delivery_projection.post_copy_verified ? "✓" : "○"}</span><div><strong>Delivery projection</strong><small>{snapshot.delivery_projection.state} · {snapshot.delivery_projection.delivery_kind} → {snapshot.delivery_projection.destination_kind} (rev {snapshot.delivery_projection.source_revision}){snapshot.delivery_projection.copy_uncertain ? " · copy uncertain, reconciliation pending" : ""}</small></div><span className={snapshot.delivery_projection.post_copy_verified ? "verified" : "waiting"}>{snapshot.delivery_projection.post_copy_verified ? "VERIFIED" : "PENDING"}</span></div>}
            {snapshot?.evidence_projection && (snapshot.evidence_projection.m108_event_count > 0 || snapshot.evidence_projection.device_observation_count > 0) && <div className="evidence-row dim"><span className="evidence-icon">{snapshot.evidence_projection.device_observation_count > 0 ? "✓" : "○"}</span><div><strong>Evidence projection</strong><small>{snapshot.evidence_projection.m108_event_count} M108 event(s) · {snapshot.evidence_projection.m108_evidence_count} evidence record(s) · {snapshot.evidence_projection.device_observation_count} device observation(s){snapshot.evidence_projection.latest_device_identity ? ` · latest ${snapshot.evidence_projection.latest_device_identity}` : ""}</small></div><span className={snapshot.evidence_projection.device_observation_count > 0 ? "verified" : "waiting"}>{snapshot.evidence_projection.device_observation_count > 0 ? "OBSERVED" : "WAITING"}</span></div>}
            {snapshot?.worker_projection && snapshot.worker_projection.task_count > 0 && <div className="evidence-row dim"><span className="evidence-icon">{snapshot.worker_projection.open_task_ids.length === 0 ? "✓" : "○"}</span><div><strong>Worker projection</strong><small>{snapshot.worker_projection.task_count} coordination task(s) · {snapshot.worker_projection.claim_count} claim(s) · {snapshot.worker_projection.handoff_count} handoff(s) · {snapshot.worker_projection.acknowledged_handoff_count} acknowledged · roles {snapshot.worker_projection.roles.join(", ")}</small></div><span className={snapshot.worker_projection.open_task_ids.length === 0 ? "verified" : "waiting"}>{snapshot.worker_projection.open_task_ids.length === 0 ? "SETTLED" : `${snapshot.worker_projection.open_task_ids.length} OPEN`}</span></div>}
          </div>
            <button className="build-button" style={{ width: "100%", marginTop: 12 }} disabled={!canExport || commandPending} onClick={() => void exportApk()} type="button">Export APK{loopResult?.build_observation?.artifact_sha256 ? ` · ${loopResult.build_observation.artifact_sha256.slice(0, 12)}…` : ""}</button>
          </section>
        </div>
        <footer className="bottom-status"><span><i className={`status-dot ${connection !== "connected" ? "waiting" : ""}`} /> {connection === "connected" ? "Local control plane connected" : connection === "connecting" ? "Connecting to local control plane" : "Local control plane unavailable"}</span><span>{lastDelivery}</span><span className="status-right">{errorMessage ?? `Tauri IPC · SQLite projection · ${workspaceRoot ? "Agent loop armed" : "workspace missing"}`}</span></footer>
      </section>
    </main>
  );
}

export default App;
