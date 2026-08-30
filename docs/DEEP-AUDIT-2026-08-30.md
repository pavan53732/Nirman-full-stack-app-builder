# Nirman Deep Audit — 2026-08-30

**Scope:** all 6 repository Markdown documents read line-by-line (14,823 lines) and cross-referenced against the full Rust workspace (17 crates + Tauri host), the React frontend, the SQLite ledger schema, the test suite, and the 20 evidence files.

**Method:** every claim, schema, state machine, event, authority, milestone, ADR, and capability registry row was extracted from the documents and compared against verified code facts (greps + full-file reads of `nirman-domain`, `nirman-ipc`, `nirman-agents/loop_kernel.rs`, `nirman-preview`, `nirman-workers`, `main.rs` command surface, and the frontend).

**Verdict up front:** the runtime is materially further along than the README claims, but it is **not** "proper 100%". There are 8 critical gaps (user-visible or contract-violating), 11 major gaps, and a long tail of documented-consistent-but-unbuilt milestone scope. Documentation drift is now the single largest source of "claimed vs actual" error, followed by missing wiring between already-built subsystems.

---

## Part A — Executive summary

### A.1 What we missed to build (not built at all)

| # | Missing subsystem | Canonical owner | Status in code |
|---|---|---|---|
| B1 | `nirman-tools` and `nirman-recovery` crates are 14-line `add()` stubs | ToolBroker / RecoveryAuthority (tech-arch §44.2) | Recovery logic actually lives in `nirman-supervisor`/control-plane; the two named crates are dead weight |
| B2 | Reasoning stream (19 event types, Calm/Inspect/Developer modes) | M59–M61, ADR-096–101 | Absent; `ReasoningStreamEvent` not implemented anywhere |
| B3 | Skills runtime (discovery, scan, trust admission, invocation) | M33/M66/M112, ADR-052/120/198 | `SkillPackage` schema only in `nirman-skills`; zero runtime |
| B4 | Cost governance (budgets, reservation/settlement, exhaustion outcomes) | M111, ADR-197 | Usage records persisted; no budgets, no exhaustion behavior |
| B5 | Memory & context runtime (ConstraintRegistry, ContextAssembler, tiers) | M81, ADR-140/141/155 | Absent |
| B6 | Deliberation / reasoning runtime (DeliberationRecord, hypotheses, effort levels) | M94–M95, ADR-167–179 | Absent |
| B7 | Terminal subsystem (ConPTY + xterm.js) and CodeMirror editor | M32, ADR-112/113 | Absent (no terminal or editor UI) |
| B8 | `NirmanSupervisor.exe` process separation | ADR-111, Stage 3 | Absent (embedded in Tauri backend — allowed for first phase per ADR-117, but Stage 3 gate unmet) |
| B9 | Device manager lifecycle (reattach, invalidation) beyond in-memory registry | tech-arch §10.3 | `InMemoryDeviceManager` only; real adb flows exist in `nirman-evidence` but no session lifecycle |
| B10 | E2E scenario engine, device matrix distribution, SBOM, SigningIdentityBinding runtime | M84/M88/M87/M103 | Schemas partial or absent; no runtime |
| B11 | Provider-augmented diagnosis (model-in-the-loop repair selection) | tech-arch §76.2 | Explicitly deferred; `diagnose_failure` is deterministic-only |

### A.2 What we built but failed to wire (built, not integrated)

| # | Built subsystem | Missing wiring |
|---|---|---|
| W1 | Real device sessions: adb install/launch/logcat/screencap/uiautomator (`nirman-evidence`, `PreviewStart`) | **Frontend never invokes `PreviewStart`**; screenshots/Logcat/UI-hierarchy land on disk and are never displayed. The preview panel is a hardcoded phone mockup |
| W2 | 30-command registry with full §76.1-style metadata in code | Only 13 commands are canonically registered in build-spec §76.1; 17 commands (all `Android*`, `AgentLoopRun`, `ProviderExecute`, `Worker*`, `SubmitInstruction`, `Reconnect`, `PauseTask`) are governed by code alone |
| W3 | M108 preview-sync store (20 event types defined; 12 emitted in real flows) | 8 event types never emitted on real paths: `IntentAccepted`, `ContractValidated`, `PlanRecorded`, `CheckpointCreated`, `SourceRevisionCommitted`, `InstallRequested`, `ObservationCaptured`, `RecoveryStarted` |
| W4 | Capability registry in build-spec §5.7 (23 capabilities, TEST-*/EV-* ids) | No test or evidence file references any `TEST-*-001`/`EV-*-001` identifier — capability promotion path (PLANNED → SUPPORTED) is structurally unwired |
| W5 | `ProjectionSnapshot` | Carries only task/continuity/preview-truth. AGENTS §8 requires typed task, **worker, preview, artifact, evidence, delivery**, background-continuity projections |
| W6 | Live progress during long builds | The agent loop runs while holding the `Mutex<RuntimeState>`; no `tokio::spawn`. During a ≤15-min Gradle build: no progress events are emitted (one accepted event, then silence until completion), and `heartbeat_subscription`/`projection`/`replay_events` block on the same mutex |
| W7 | `ipcClient` helpers | `buildArtifact`, `testProvider`, `executeProvider`, `updateProviderProfile`, `heartbeatSubscription` are dead exports — no settings UI, no provider test UI |

### A.3 What the user sees vs. does not see

**Sees (real, wired end-to-end):** chat transcript; plan card running the real pipeline (SubmitInstruction → AndroidConstructionCreate → AndroidToolchainPreflight → AgentLoopRun); real Gradle build outcome; scaffold file list from the real run; APK export button performing real scan → aapt inspect → local delivery with hash verification; durable evidence rows (projection cursor, build outcome, APK delivery state); pause/resume/cancel that actually reach the control plane; connection/subscription status.

**Does not see:** the actual app preview (mockup frame; real screenshots stay on disk); device session status; Logcat or UI-hierarchy output; any evidence artifact viewer; workers/checkpoints/leases; terminals; file editor; provider settings; reasoning stream; cost/usage; recovery status detail; anything AAB.

### A.4 Is background execution complete?

**No.** Durable-command idempotency, cancellation (including killing an in-flight Gradle build via the shared flag fast-path), M2 restart-from-checkpoint, lease fencing, and M7 background runs are real and tested. But: UI close terminates the run (no supervisor process — M116's "eligible work continues without open UI" is unmet); an interrupted agent loop is not automatically resumed after restart (only its idempotent result is rebuilt); and long builds block the responsive command surface (W6), violating ADR-024's "background work must be non-blocking".

---

## Part B — Findings by the 16 audit dimensions

### 1. Internal contradictions

| ID | Location | Contradiction |
|---|---|---|
| C-1.1 | build-spec §26.14 vs §33.2 vs §5.7.2 | Three task/session lifecycle vocabularies coexist (task machine with `FAILED_RETRYABLE`/`ESCALATED`; session lifecycle `Created→…→Completed` with 6 terminal states; `ProductLifecycleState` 13 states) and are never reconciled state-for-state |
| C-1.2 | dev-plan M6 work items 9 vs 10 (L230–231) | Item 9: "ArtifactExport currently exposes only source revision and destination path". Item 10: "now exposes the six fields". Both describe the same payload's present state |
| C-1.3 | dev-plan M38 | Two titles: "Certified Android profile coverage and production acceptance" (§2 table) vs "Complete Android technology coverage" (§29 heading) |
| C-1.4 | dev-plan M5 item 4 (L201) | M5 (Stage 1/2) requires routing tool calls through "the M115 command envelope" — M115 is gated behind M100–M106 prerequisites. Circular dependency |
| C-1.5 | dev-plan L1247 vs L1240/L1266 | "each contract must have one canonical owning milestone", yet `CONTRACT.RUNTIME.SCOPE` is owned by both M11 and M96 (and TEST/EV ids shared across M11/M65/M96, M69/M82, M85/M92, M89/M90) |
| C-1.6 | tech-arch §50 vs §73.3 | Two different `PreviewRevision` definitions (11 textual fields vs 20 canonical fields); §36.1 lists it once as canonical |
| C-1.7 | tech-arch §9.1 | "at least four profiles" followed by a table of three |
| C-1.8 | tech-arch §71.4 | DECIDE branch → `SPECULATE (§65)` — `SPECULATE` is defined nowhere; §65 is the Multi-Device Scenario Coordinator |
| C-1.9 | tech-arch §58.2 vs §71.4 | Kernel loop branches {CONTINUE, VALIDATE, RECOVER, DELEGATE, REPLAN, COMPLETE} vs DECIDE branches {continue, repair, replan, delegate, branch, terminate} — overlapping, not identical |
| C-1.10 | build-spec §5.6 vs §5.7.2 | Capability status vocabulary (6 values, no `BLOCKED`) vs state-dimension enums that include `BLOCKED`/`UNKNOWN` |
| C-1.11 | build-spec §77 numbering | §77.1.1 appears before §77.1 |
| C-1.12 | build-spec §23.7 (L1287–1288) | Duplicate table row "Push changes or publish artifacts | Always ask" |
| C-1.13 | ADR-022 vs ADR-026/030/048/072 | Deferral table lists scheduled automation, external-tool protocol, long-term memory, automatic commits as open — each later accepted by another ADR; ADR-022 is also the only ADR with no status field |

### 2. Stale documentation

| ID | Location | Staleness |
|---|---|---|
| S-2.1 | README "Current status" + area table | Claims "full underlying use-case execution … Android project synthesis … runtime export implementation … remain outstanding" and "React … real Tauri event delivery and DOM/runtime validation remain outstanding". All of these shipped in commits `e5654d3`, `1fb1beb`, `6a0bcaa` (agent loop + real Gradle scaffolding + host E2E + React client driving the real pipeline). The README describes the repo as of M115-slice time, ~3 feature commits behind |
| S-2.2 | README "Windows `.exe` release | Not yet available" | Cross-compilation to `x86_64-pc-windows-msvc` and NSIS installer production were proven in this workspace (`Nirman_0.1.0_x64-setup.exe`); the row should reflect that the toolchain works and Windows-runtime validation remains |
| S-2.3 | README "13 canonical M115 commands" | Code registers **30** command kinds |
| S-2.4 | README "Android synthesis … Fixture manifests and domain boundary only" | Real synthesis → scaffold → Gradle build → diagnose/retry → validate → export now executes |
| S-2.5 | All 4 canonical docs | Mechanical splice corruption: "APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB`" is grammatically spliced into ~30+ sentences (e.g., dev-plan §34 heading, M64; build-spec §29.6/§30/§42.1/§46.1/§52.15; tech-arch §11.3/§18/§34/§52/§57.5…), in some places breaking the sentence's normative meaning |
| S-2.6 | dev-plan §19 "Immediate Next Tasks" | Reads as greenfield ("Create the source repository") while the repo is 69+ commits deep — position pointer never updated |
| S-2.7 | tech-arch References block at L4345 | Sits mid-document (between §72 and §73); reference [6] missing ([5]→[7]); sections 73–83 appended after the document was formally closed |

### 3. Implementation/spec divergence

| ID | Spec | Code | Divergence |
|---|---|---|---|
| D-3.1 | `ProductLifecycleState` = 13 states incl. `SYNTHESIZING`, `BLOCKED` (build-spec §5.7.2) | `nirman-domain` has 12: **no `Synthesizing`, no `Blocked`**, and adds **`Paused`** (not in the spec enum) | Missing states + invented state |
| D-3.2 | Export lifecycle `REQUESTED→COPYING→COPIED→UNKNOWN→RECONCILING→VERIFIED\|FAILED\|BLOCKED` (§78.2, ADR-203, M117) | `DeliveryState` = Pending/Copying/Copied/Verified/Failed/Blocked/Unknown — **no `Reconciling`** | The interrupted-copy reconciliation state does not exist; `Unknown` cannot transition to reconciliation |
| D-3.3 | `ExportVerificationRecord` 28 fields (TA §74.3) | `ApkDeliveryRecord` 18 fields | Missing: source/destination file identities, distinct source hash, `post_copy_check`, `policy_decision_id`, `checkpoint_id`, `signing_identity_binding_id`, `validation_decision_id`, `promotion_decision_id`, `reconciliation_reference`, `failure_evidence_id`, `deployment_delivery`, `evidence_id`, `verified_at` |
| D-3.4 | Worker registry: 14 canonical roles, exact names (ADR-049: "legacy role names MUST NOT appear") | `nirman-workers::WorkerRole` = 9 roles with divergent names (Architecture, Implementation, Debugging, Testing, Security, VisualQa, Performance, Release, Reconciliation) | Missing Primary Orchestrator, Repository Scout, Requirements Planner, UI Worker, Android Data and Integration Worker, Documentation Worker; `Implementation`/`Testing` are non-canonical labels |
| D-3.5 | SQLite ledger: 27 named tables (TA §57.5: projects, sessions, tasks, task_states, workers, worker_leases, approvals, policies, recovery_records, terminal_sessions, process_records, device_profiles, validation_runs, evidence_records, artifacts, toolchain_manifests, project_locks, decision_records, reasoning_stream_events, …) | 31 tables with different names/scope (events, command_results, android_*, m5/m6/m7/m8_*, agent_loop_records, projections…) | The implemented ledger is a different schema family: no projects/sessions/tasks/workers/approvals/policies/evidence_records/artifacts/reasoning_stream_events tables. Durable state exists but not where the architecture says it lives |
| D-3.6 | Spec failure classes §7.3: 10 classes (missing tool, dependency, syntax, type, runtime, visual, permission, network, provider, ambiguous requirement) | `FailureClass` = 5 build-centric classes (EnvironmentUnavailable, Timeout, BuildFailed, MissingArtifact, Cancelled) | Provider/network/permission/visual/dependency failure classes absent from the loop's diagnosis vocabulary |
| D-3.7 | Task state machine §26.14/TA §5.1: QUEUED→PLANNING→READY→RUNNING→VALIDATING→COMPLETED with WAITING_* / FAILED_RETRYABLE / ESCALATED | Control plane implements pause/resume/cancel + lifecycle states only | No QUEUED/READY/WAITING_APPROVAL/WAITING_RESOURCE/FAILED_RETRYABLE/ESCALATED transitions in the durable task model |
| D-3.8 | `ProjectionSnapshot` must carry typed task, worker, preview, artifact, evidence, delivery, background-continuity projections (AGENTS §8) | Snapshot has task_state, continuity_state, preview_truth, source revision, cursor, last-known-good only | 4 of 7 required projection families missing |

### 4. ADR conflicts

| ID | Conflict |
|---|---|
| A-4.1 | ADR-034/ADR-015 permit browser validation "for a declared non-Android surface" — impossible under the Android-only invariant (ADR-058/081/094/180). Dead carve-out or undocumented exception |
| A-4.2 | ADR-049's closed 14-role registry vs ADR-103's `BrandAssetWorker` (15th role, never amended into the registry) |
| A-4.3 | ADR-002 "background control-plane **process** will own execution" (categorical) vs ADR-111/117 (embedded in Tauri backend for first phase). Reconciliation is implicit; ADR-002 was never annotated |
| A-4.4 | ADR-022 deferral table contradicts accepted ADR-026/030/048/072 and is the only ADR without a status field |
| A-4.5 | ADR-067 ≡ ADR-162 (leases + single-use capabilities, near-verbatim) — both Accepted, no cross-reference, no supersession |
| A-4.6 | Provider gateway defined four times: ADR-019, ADR-037, ADR-069, ADR-166 — all Accepted |
| A-4.7 | Preview authority spread across six ADRs: 016, 060, 076, 182, 191, 195 |
| A-4.8 | `CONTRACT.RUNTIME.AUTHORITY` lock held by 10 different ADRs with no internal precedence |
| A-4.9 | Sandbox scope: ADR-014 (tuning space) vs ADR-022 ("high-risk profile = future") vs ADR-092 ("complete, nothing more required") |
| A-4.10 | Registry ordering scrambled: ADR-158–166 physically between 073 and 074; ADR-180 between 171 and 172. File order ≠ numeric order breaks "latest wins" reading |
| A-4.11 | Supersession mechanism defined but **never used**: all 204 ADRs simultaneously Accepted/normative; de-facto supersessions (above) are invisible to the document's own machinery |

### 5. Duplicated schemas

| ID | Duplication |
|---|---|
| DU-5.1 | **Preview truth enums ×3**: `nirman_domain::PreviewTruth` (6 states — **no `Simulated`**), `nirman_preview::PreviewEventTruth` (7), `nirman_preview::PreviewExecutionTruth` (7). The projection snapshot uses the 6-state one; the preview crate uses 7-state ones. Same concept, two vocabularies in live code |
| DU-5.2 | `PreviewRevision` defined twice in tech-arch (§50 textual, §73.3 canonical 20-field) |
| DU-5.3 | Lease/capability records defined 2–3× in tech-arch (§27.4 worker lease; §36.3 session lease + capability; §46.1/46.2 `SessionLease`/`OperationCapability` with a `nonce` only present in §46.2) |
| DU-5.4 | `AndroidRepairRegistry` (ADR-075/TA §51.1) vs `FailureModeRegistry` (ADR-085/TA §53.3) — two overlapping failure catalogues, both Accepted, both implemented as one merged `AndroidRepairRegistry` in code |
| DU-5.5 | Schedule record: TA §7.4 fields vs §16.4 `Schedule` schema (different field vocabularies for the same record) |
| DU-5.6 | Tech-arch §36.1 duplicated verbatim sentence about `IntegrationBoundaryContract` (L1789) |
| DU-5.7 | Module map duplication: TA §13 `tool-gateway/{android-device,devices}` near-duplicates; `preview/` under both `desktop-ui/` and `tool-gateway/` |

### 6. Duplicated authorities

| ID | Duplication |
|---|---|
| DA-6.1 | Lifecycle authority defined by ADR-066, ADR-159 (pure reducer), ADR-158, ADR-160, ADR-166 — five Accepted ADRs under the same lock; code implements the reducer (correct) but the decision log does not name a winner |
| DA-6.2 | Preview: `PreviewCoordinator` named sole authority by ADR-195 while ADR-016/060/076/182/191 still assign overlapping preview responsibilities |
| DA-6.3 | Worker registry dual ownership: ADR-009 (decision) + ADR-049 (list); code follows neither exactly (D-3.4) |
| DA-6.4 | `SupervisorAuthority`/`LeaseAuthority`/`DeviceAuthority`/`ProviderOperationalityAuthority` are documented as aliases-only (AGENTS §3, build-spec §77.3) — correctly not new authorities, but four alias names circulate in docs and would be easy to reify by mistake |

### 7. Missing schemas (named, never defined)

| ID | Schema | Named at |
|---|---|---|
| MS-7.1 | `UICommandEnvelope` field list | build-spec §76 canonical path; implemented in code as `CommandEnvelope` (nirman-domain) — spec never defines the fields |
| MS-7.2 | `ProjectionSnapshot` field list | build-spec §76/§77; implemented in code with 8 fields — spec never defines them |
| MS-7.3 | `GoalContract` | build-spec §5.6 names it as the completion-predicate source; only a prose table at §27.1 |
| MS-7.4 | `AndroidApplicationContract`, `VisualSpecification`, `WorkerContract`, `ArtifactRecord`, `ValidationResult`, `CertificationDecision`, `CompletionDecision`, `RecoveryRecord`, `EvidenceRecord` (full fields) | declared canonical in TA §36.1; field lists deferred to build-spec, which also does not define them |
| MS-7.5 | `ExternalToolConnection` | build-spec §23.11/§27.9; prose lifecycle only |
| MS-7.6 | `APKExportRecord` | TA §74.3 calls it a "view" over `ExportVerificationRecord`; no independent schema; code implements `ApkDeliveryRecord` instead (D-3.3) |

### 8. Missing state transitions

| ID | Gap |
|---|---|
| ST-8.1 | `ProductLifecycleState`: no `Synthesizing` transition (the agent loop literally performs synthesis as its first action — it currently reports `Implementing`/`Planning` instead); no `Blocked` state at all; `Paused` exists in code but not in the spec enum |
| ST-8.2 | `DeliveryState`: no `Unknown → Reconciling → Verified/Failed` recovery arc; `Unknown` is terminal-ish in code, so an interrupted copy cannot be reconciled (M117's core scenario) |
| ST-8.3 | Task machine: no `WAITING_APPROVAL`, `WAITING_RESOURCE`, `FAILED_RETRYABLE`, `ESCALATED`, `QUEUED`, `READY` transitions (spec §26.14) |
| ST-8.4 | Subscription lifecycle: spec §76.3 defines `GAP` and `PAUSED` statuses; code has REQUESTED/ACTIVE/PAUSED/CLOSED backpressure pause — gap-driven `GAP` state not implemented as a distinct status |
| ST-8.5 | Session lifecycle terminal states `BlockedByPolicy`, `BlockedByMissingInformation`, `ProviderUnavailable`, `EnvironmentUnrecoverable` (spec §33.2, ADR-080) have no durable representation in the control plane |

### 9. Missing events

| ID | Gap |
|---|---|
| EV-9.1 | 8 of 20 `PreviewSyncEvent` types never emitted on real paths: `IntentAccepted`, `ContractValidated`, `PlanRecorded`, `CheckpointCreated`, `SourceRevisionCommitted`, `InstallRequested`, `ObservationCaptured`, `RecoveryStarted` (the pipeline's early stages are invisible to preview sync; recovery starts never announce themselves) |
| EV-9.2 | No progress events during agent-loop execution: one `AgentLoopRun` accepted event, then silence until completion — a 15-minute Gradle build streams nothing to the UI |
| EV-9.3 | The 12 control-plane event families (TA §45.2) and 12 example events (TA §12.1: task_started, plan_created, worker_started, tool_requested, …) are not implemented; code's event `kind` strings are just `CommandKind` debug names |
| EV-9.4 | 40 lifecycle hook events (build-spec §27.4) unimplemented |
| EV-9.5 | 19 `ReasoningStream` event types (§49.2) unimplemented |
| EV-9.6 | 6 event-driven continuation triggers (§27.11: workspace_file_saved, build_completed, failure_observed, dependency_changed, promotion_or_export_requested, stream_reconnected) unimplemented — nothing continues work automatically after an event |

### 10. Missing persistence

| ID | Gap |
|---|---|
| P-10.1 | No durable `BackgroundContinuityRecord` (spec §77.1: ~31 fields incl. dimensions, resumeEligibility, fencing token, checkpoint/last-known-good refs). The continuity state shown in the UI is derived from the subscription connection, not a durable record — it cannot survive restart as spec requires |
| P-10.2 | No `evidence_records` table: evidence artifacts (device observations, screenshots, build outputs) are saved per-table (android_device_observations etc.) but there is no unified evidence ledger with dependency/invalidation tracking (§5.7.4, §37) |
| P-10.3 | No `reasoning_stream_events`, `decision_records`, `agent trust`, `cost governance`, `context cache`, `android integrity` stores (M59+/M89+/M112/M111/M113/M114) |
| P-10.4 | No `approvals`, `policies`, `projects`, `sessions`, `tasks`, `workers`, `terminal_sessions`, `process_records`, `device_profiles`, `validation_runs`, `artifacts`, `project_locks` tables (TA §57.5) — durable state lives in milestone-shaped tables instead |
| P-10.5 | Capability-evidence linkage unpersisted: no record ties a passing fixture to a capability id (TEST-GEN-001 → EV-GEN-001), so no capability can ever leave `PLANNED` by machine decision |

### 11. Missing evidence links

| ID | Gap |
|---|---|
| EL-11.1 | The 20 evidence JSON files use milestone ids (`m4_control_plane_trace.json` …) and none references the capability registry's TEST-*/EV-* identifiers — the certification chain `TEST-*-001 → EV-*-001 → SUPPORTED` has no executable instance |
| EL-11.2 | Device-session evidence (screenshots, Logcat ranges, UI hierarchy) is captured but not linked into any projection the UI can render — evidence exists, links to the user do not |
| EL-11.3 | Runtime certification evidence class (§69.10: schema/migration tests, reducer illegal-state tests, Windows process/IPC tests, failure injection, restart recovery, hidden-human-dependency fixtures) exists only partially inside `nirman-desktop` tests; not registered as the spec's evidence identity |
| EL-11.4 | FIX-DEL-01..07 deliberation fixtures and M80/M95 certification fixtures described in dev-plan have no fixture files |

### 12. Missing failure paths

| ID | Gap |
|---|---|
| FP-12.1 | Interrupted export copy: code has post-copy hash verification, but no `UNKNOWN` detection (e.g., copy error mid-stream → which state?), no `RECONCILING` state, no `reconciliation_reference`/`failure_evidence_id` bindings (ST-8.2, D-3.3) |
| FP-12.2 | Provider failure during agent loop: loop never calls providers, so provider outage has no failure path in the primary pipeline (FailureClass has no Provider variant) |
| FP-12.3 | Device loss mid-preview: no invalidation of device-bound evidence, no wait-for-new-device-session state (spec §77.2) |
| FP-12.4 | Gradle daemon/orphan handling exists (kill-on-timeout/cancel, orphan-grace readers) ✓ but no `EnvironmentUnrecoverable` classification when toolchain breaks permanently (ADR-080 terminal states) |
| FP-12.5 | Subscription gap/backpressure: pause-on-over-limit exists ✓; but GAP status + replay-request recovery arc (§76.3) is not fully distinct (ST-8.4) |

### 13. Missing recovery paths

| ID | Gap |
|---|---|
| RP-13.1 | Interrupted agent loop after host restart: durable `agent_loop_records` exist and duplicate dispatch rebuilds the result, but nothing automatically resumes an in-flight loop from its last persisted transition (M110's event-driven continuation) |
| RP-13.2 | UI-close continuation (M116): loop dies with the app; no supervisor process, no login-start resume (M34), no suspend/hibernate handling |
| RP-13.3 | Recovery ladder L2–L9 (TA §28.1): only L0 (retry with variation) and L1 (diagnostics) are implemented; no context refresh, role change, checkpoint backtracking, model escalation, delegation, isolated alternative, or decision request |
| RP-13.4 | Export `UNKNOWN → RECONCILING` reconciliation before retry (ADR-203) — absent |
| RP-13.5 | Lease expiry mid-run: M2 fencing covers supervisor restart; worker-lease expiry during agent loop not modeled (loop is not lease-scoped) |

### 14. Missing integration wiring

| ID | Gap |
|---|---|
| IW-14.1 | **PreviewStart is never invoked by the frontend** — the single biggest user-visible gap: real adb install/launch/screenshot machinery exists and is tested, but the UI shows a mock phone |
| IW-14.2 | Agent loop holds `Mutex<RuntimeState>` for its whole duration (no `tokio::spawn`): heartbeat, projection, replay, subscribe all block; violates ADR-024 non-blocking background work and starves the subscription during builds |
| IW-14.3 | Early-stage M108 events not persisted by the pipeline (contract validated, plan recorded, checkpoint created, source revision committed) → preview-sync projection cannot advance through the early stages on real runs |
| IW-14.4 | Capability promotion unwired end-to-end (EL-11.1) |
| IW-14.5 | Dead frontend exports: `buildArtifact`, `testProvider`, `executeProvider`, `updateProviderProfile`, `heartbeatSubscription` never called — no settings/provider UI |
| IW-14.6 | `ProjectionSnapshot` lacks worker/artifact/evidence/delivery projections (D-3.8) — the UI cannot render what it is required to show |
| IW-14.7 | Sidebar navigation (Workspace/Tasks/Files/Preview/Logs/Settings) switches highlight only; every panel renders the same content; project switcher and notification buttons are decorative |

### 15. Capabilities claimed in README but unsupported by source

The README is mostly conservative (it under-claims), but three claims over-reach the evidence:

| ID | README claim | Reality |
|---|---|---|
| RC-15.1 | "Documentation certification | Passing" and "Conformance mutation harness | Passing: 131/131 checks" | True for the documentation graph — but the README's phrasing sits in a table that also implies the verifier covers the current 30-command surface; the verifier's command-payload coverage check validates §76's 13 commands only |
| RC-15.2 | "The current implementation includes … typed dispatch for the 13 canonical M115 commands plus compatibility reconnect/pause operations" | Understates and misframes: the registry has 30 commands including 5 `Android*` + `AgentLoopRun` + `ProviderExecute` + 5 `Worker*` — the README's "13 plus compatibility ops" hides the real (undocumented-in-spec) surface |
| RC-15.3 | "Durable command-result records preserve idempotency across restart and conflicting request fingerprints are rejected" | True (verified by tests), but the README omits that the same durability does not yet cover continuity records or interrupted-loop resume, which the surrounding prose implies by the M2/M115 trace description |

The larger problem is the inverse (S-2.1–S-2.4): capabilities supported by source but documented as outstanding.

### 16. Source capabilities not governed by canonical contracts

| ID | Source capability | Canonical governance |
|---|---|---|
| SG-16.1 | 17 of 30 command kinds: `AndroidConstructionCreate`, `AndroidToolchainPreflight`, `AndroidRequirementEvaluate`, `AndroidSynthesisBuild`, `AndroidProjectScaffold`, `AgentLoopRun`, `ProviderExecute`, `WorkerTaskClaim`, `WorkerHandoffSubmit`, `WorkerHandoffAcknowledge`, `WorkerReconcile`, `WorkerStep`, `SubmitInstruction`, `Reconnect`, `PauseTask` (+ duplicates) | No §76.1 registry row in build-spec. Code self-governs (each carries the full 14-field registry metadata — verified), but per ADR-189 the CanonicalSchemaRegistry/build-spec must own command identity; the spec has not caught up with the implemented surface |
| SG-16.2 | `AgentLoopReducer` phase machine (9 phases, 6 terminal states, variation-enforced retries, iteration budget 8) | Matches TA §58.2's loop shape and §58.3 `AgentLoopRecord`, but the budget/phase constants and the 5-action `AgentActionType` vocabulary are defined only in code — no schema pins them |
| SG-16.3 | 31-table SQLite ledger schema | TA §57.5 specifies a different 27-table set; nothing canonical owns the implemented schema (D-3.5) |
| SG-16.4 | 12 in-crate aapt/adb/scan/deliver runtime capabilities (`nirman-android`, `nirman-evidence`, `nirman-artifacts`) | Governed only by milestone tests (m47/m48 acceptance); no capability-registry row maps to them |
| SG-16.5 | `PreviewTruth` (6-state) in domain vs 7-state truth enums in preview crate | No canonical owner resolves which vocabulary is authoritative (DU-5.1) |

---

## Part C — Consolidated remediation register (priority order)

### P0 — fix before anything else (user-visible or contract-breaking)

1. **Wire PreviewStart into the frontend** (IW-14.1): add a "Run on device" step after export; render the real screenshot (file → base64/data URL), Logcat tail, and UI-hierarchy from the persisted device observation; replace the mock phone frame with observed content and keep truth labels. This single change moves preview from PREDICTED-mockup to OBSERVED.
2. **Un-block the command surface** (IW-14.2): run `run_agent_loop` via `tokio::task::spawn_blocking` (or a dedicated thread) with the lock released; keep the cancellation fast-path; emit progress events (loop phase transitions + build stdout milestones) through the existing EventSink.
3. **Add `Reconciling` to `DeliveryState` + interrupted-copy recovery** (ST-8.2, FP-12.1, RP-13.4): detect copy errors → `Unknown`; on next export with same idempotency key → inspect destination, hash-compare, transition `Unknown → Reconciling → Verified/Failed`; persist `reconciliation_reference` + `failure_evidence_id`.
4. **Align `ProductLifecycleState` with spec** (D-3.1): add `Synthesizing` (agent loop phase 1), add `Blocked`, remove or canonize `Paused` (spec change if kept — it is genuinely useful for pause/resume).
5. **Update README + build-spec §76.1** (S-2.1–S-2.3, SG-16.1): status table to current reality; register all 30 commands in the spec registry (each already has the 14 metadata fields in code — transcribe them).
6. **Extend `ProjectionSnapshot`** (D-3.8, IW-14.6): add worker, artifact, evidence, delivery projection fields so the UI can render them.

### P1 — close the loop on built subsystems

7. Emit the 8 missing early-stage M108 events on real pipeline paths (EV-9.1, IW-14.3).
8. Unify preview truth enums (DU-5.1): make `nirman-domain::PreviewTruth` the canonical 7-state enum (add `Simulated`), reuse it in the preview crate or delete the duplicates.
9. Rename/complete `WorkerRole` to the ADR-049 14-role registry (D-3.4) — or amend ADR-049 (it is the accepted decision; the code must follow it as written).
10. Add the missing `ExportVerificationRecord` fields to `ApkDeliveryRecord` (D-3.3).
11. Auto-resume interrupted agent loops after restart from the last persisted `agent_loop_record` transition (RP-13.1).
12. Wire capability evidence: each evidence validator gains the TEST-*/EV-* id; a small registry maps capability → fixture → evidence file (EL-11.1, P-10.5, IW-14.4).
13. Document-fix pass: resolve ADR contradictions C-1.2/C-1.3/C-1.4/A-4.x (mark supersessions, fix ADR-022 status, add BrandAssetWorker to the registry or scope it as a sub-role), fix the splice-corrupted sentences, repair broken cross-refs (tech-arch §71.4 SPECULATE, build-spec §56/§59 preambles, §77 ordering, duplicate row §23.7, missing reference [6]).

### P2 — milestone scope (documented-consistent, deliberately future)

B1–B11 from Part A.1, plus: recovery ladder L2–L9, reasoning stream, skills runtime, cost governance, memory/context, terminals/editor UI, supervisor process separation (Stage 3), device-matrix/E2E/SBOM/signing runtimes, provider-augmented diagnosis.

---

## Part D — Method appendix (evidence base)

- Documents read in full: README.md (284 L), AGENTS.md (396 L), nirman-build-spec.md (5,314 L), nirman-technical-architecture.md (4,964 L), nirman-development-plan.md (1,552 L), nirman-decisions.md (2,313 L).
- Code verified: all 17 crates + Tauri host (command registration, `dispatch_request` pipeline, `run_agent_loop`, `execute_preview_device_session`, export path), frontend (App.tsx, ipcClient.ts, contract.ts), 31-table ledger DDL, 14 integration test files + 31 host tests + 20 evidence files.
- Enum-level diffs verified line-by-line: `CommandKind` (30), `ProductLifecycleState` (12 vs 13), `DeliveryState` (7 vs 8), `BackgroundContinuityState` (11 ✓ exact), `PreviewTruth` (6 vs 7), `PreviewSyncEventType` (20 ✓ exact), `KernelDecision`/`LoopContinuation` (✓ exact), `FailureClass` (5 vs 10), `WorkerRole` (9 vs 14), `CommandRegistryEntry` (15 fields ✓ superset of §76.1), `ApkDeliveryRecord` (18 vs 28), `ProjectionSnapshot` (8 fields).
