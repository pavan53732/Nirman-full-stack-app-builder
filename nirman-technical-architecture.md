# Nirman Technical Architecture

## Implementation Blueprint for the Windows-First Desktop Application

**Document status:** Living implementation specification — accepted architecture
**Application:** Nirman  
**Scope:** Local-first autonomous application development with configurable cloud AI providers  
**Relationship to master specification:** This architecture document explains how to implement the behavior defined by the master product specification. It contains architecture and interfaces, not production source code.

**Canonical ownership:** The Build Spec owns product contracts, invariants, and capability/contract registries. The Technical Architecture owns implementation schemas, protocols, and module boundaries; the field blocks of both documents are held in `nirman-schemas.md`, each under the section that owns it (ADR-220). The milestone document (`nirman-milestones.md`) owns sequencing, milestones, fixtures, and exit gates. The ADR document (`nirman-adrs.md`) owns accepted decisions, rationale, and supersession; `nirman-decisions.md` owns only the decision process. The README, `INDEX.md`, and `GLOSSARY.md` are explanatory only. AGENTS defines agent operating constraints only. The verifier certifies documentation and semantic checks only; it is never a runtime authority.

---

## 1. Architecture Goals

Nirman should be implemented as a **local control system for autonomous software development**. The visible desktop interface is only one client of the system. A background control plane owns task execution and persists enough state to recover from application closure, process failure, or operating-system restart.

The architecture must satisfy six goals. It must keep Android application execution local, make AI actions observable, preserve reversible project states, support specialized workers, enforce permissions at the runtime boundary, and remain extensible across Android framework profiles and device capabilities.

The architecture should prefer small, typed interfaces over implicit communication. A model may propose an action, but only the policy engine and tool gateway may authorize and execute it.

---

## 2. System Context

```text
┌─────────────────────────────────────────────────────────────┐
│                    Nirman Desktop UI                    │
│  Chat | Project Tree | Editor | Preview | Tasks | Settings   │
└─────────────────────────────┬───────────────────────────────┘
                              │ Local authenticated IPC
┌─────────────────────────────▼───────────────────────────────┐
│                    Nirman Control Plane                  │
│ Task Scheduler | Event Bus | Approval Manager | State Store  │
└───────────┬──────────┬──────────┬──────────┬────────────────┘
            │          │          │          │
      Workers      Tool Gateway  Runtime   Provider Router
            │          │          │          │
┌───────────▼──┐ ┌─────▼──────┐ ┌─▼──────┐ ┌─▼───────────────┐
│ Workspaces   │ │ Policies   │ │ Builds │ │ Cloud AI        │
│ Worktrees    │ │ Sandboxes  │ │ Preview│ │ Models          │
└──────────────┘ └────────────┘ └────────┘ └─────────────────┘
            │
      LocalDecisionEngine
            │
      Local Model Root
      (pinned Laya checkpoint)
```

The control plane should communicate with the user interface through a local authenticated IPC channel. The production transport is named pipes. A loopback HTTP or WebSocket API may be used internally for development and debugging, but it must require a per-installation secret or operating-system authenticated channel and must not be used for the production SupervisorConnection. The interface must not be able to impersonate another project or bypass task policies by modifying client-side state.

---

## 3. Process Model

Nirman is one user-facing Windows application implemented by two long-lived cooperating processes — `Nirman.exe` and `NirmanSupervisor.exe` — and the short-lived `NirmanWorker.exe` processes the supervisor spawns, one per worker lease (§3.5; ADR-222). This is an implementation boundary, not a product boundary.

```
                    ONE NIRMAN PRODUCT
                           │
              ┌────────────┴────────────┐
              │                         │
        Nirman.exe              NirmanSupervisor.exe
        visible UI                headless runtime
              │                         │
              └──── authenticated IPC ──┘
                                        │ spawns, one per worker lease
                                        ▼
                                NirmanWorker.exe × N
                                reasoning host, no authority
```

`Nirman.exe` is the visible client. `NirmanSupervisor.exe` is the durable local runtime authority. `NirmanWorker.exe` is a disposable reasoning host with no authority of its own.

The supervisor is never a separately operated application. It has no normal user workflow, no independent configuration surface, and no requirement for manual launch. The installer packages both components as one Nirman installation and maintains compatible versions together.

### 3.1 User-facing application: Nirman.exe

`Nirman.exe` owns the visible WinUI 3 experience. It may close, minimize, restart, or reconnect without transferring runtime authority away from the supervisor.

The desktop interface should be built with C#/.NET + WinUI 3. It displays state and sends user commands, but it should not directly execute arbitrary shell commands or mutate project files. All filesystem, process, provider, and build operations go through the control plane.

### 3.2 Headless runtime: NirmanSupervisor.exe

`NirmanSupervisor.exe` is a user-scoped background process. It owns autonomous execution, task state, workers, leases, persistence, recovery, policy enforcement, evidence, and runtime processes.

It must run without a normal application window or independent taskbar workflow. Nirman automatically starts or reconnects to it when required. The user must never be required to launch, configure, monitor, or terminate the supervisor manually.

When `Nirman.exe` is minimized or closed, eligible tasks continue according to their execution policy. When the UI returns, it reconnects through `SupervisorConnection` and reconstructs state from the durable ledger/event stream.

The control plane should start on user login whenever an active Goal Mode task exists, unless the user explicitly opts out for that project. A lightweight per-user startup entry should launch the stable supervisor/control-plane process without running a system service by default. If no task is active, the user may configure whether the control plane starts at login. After reboot, the supervisor must scan durable task state, reconcile process leases, and resume eligible tasks automatically without requiring the desktop UI to be opened.

### 3.3 Worker processes

Every worker runs as its own `NirmanWorker.exe` child process, one per worker lease (§3.5), with a declared role, model profile, workspace, permissions, limits, and task contract. A worker must not decide its own isolation profile or expand its own permissions.

A worker may request a model call through the supervisor's `ModelGateway` and request filesystem, process, preview, browser, or external-tool actions through the supervisor's `ToolBroker`, both over its `WorkerConnection` (§57.11). It holds no provider credential, no file handle, no socket, and no child process, and it cannot invoke the operating system directly outside those gateways.

### 3.4 Runtime processes

Development servers, test runners, package managers, emulators, browsers, and build commands are runtime processes. The process manager tracks each process tree and associates it with a task, worker, project, workspace, and resource profile.

The process manager must support cancellation of the whole process tree, not only the parent process. It must capture stdout and stderr separately, enforce output limits, and preserve the final diagnostic output when a process is terminated.

Job handles MUST be created with handle inheritance DISABLED. If a child inherits the handle, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE does not reap the tree when the parent exits, because an open handle keeps the job alive. Termination MUST NOT rely on parent-child process-tree walking alone. A grandchild assigned to its own nested job, or reparented after its parent exits, is missed. Assignment to the supervisor job at spawn is the only durable containment. Every spawned build, emulator, Android-runtime, and toolchain process MUST be assigned to the job BEFORE it is resumed. The gradlew.bat → java.exe shapes are the primary cases that require containment. A leaked Gradle daemon holds file locks and corrupts the next run; supervisor restart MUST reconcile orphaned descendants from the ledger before starting new work.

### 3.5 Process inventory and inter-process edges

Nirman's production installation consists of exactly three executables (ADR-222). Every other process Nirman runs — the emulator, Gradle and the JDK, adb, ConPTY shells, native toolchain executables — is an external tool spawned and supervised by the supervisor under §3.4, never a Nirman executable.

| Executable | Language | Instances | Started by | Holds |
|---|---|---|---|---|
| `Nirman.exe` | C#/.NET, WinUI 3 | one per interactive session | the user | presentation state only (§57.6), the `SupervisorConnection` client, and PreviewHost (§10.8), which is a surface inside this process, not a process |
| `NirmanSupervisor.exe` | Rust/Tokio | one per user (§57.4 singleton) | Windows user login or `Nirman.exe` | every authority of §57.2, the SQLite ledger, the provider credential references and `ModelGateway`, `ToolBroker`, `ContextOrchestrator`, the emulator manager and `RenderTransport`, and every Job Object |
| `NirmanWorker.exe` | Rust | one per active worker lease, at most the "Global active workers" value of §7.2 | `WorkerRuntime` in the supervisor, from a persisted launch intent (§27.3) | the reasoning engine (§71) and deliberation runtime (§72) of one worker; no authority, no credential, no file, no socket, no child process |

**Worker host.** A worker is never a thread, Tokio task, or module inside `NirmanSupervisor.exe` or `Nirman.exe`. `WorkerRuntime` spawns one `NirmanWorker.exe` per worker lease after the lease and its launch intent are committed, assigns the process to its own Job Object before it is resumed — `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, an active-process limit of 1 so the worker cannot spawn, and the worker process memory limit of build spec §80.3 (default 2 GB) — and starts it as an AppContainer process with a filtered environment: a LowBox token created with no network capability, so the kernel refuses every socket, and with a per-lease container SID, so the kernel refuses every file that carries no access-control entry for that SID — the supervisor never writes one on a workspace, the toolchain root, or the ledger, and only the versioned application directory carries the read-and-execute entry for `ALL APPLICATION PACKAGES` that lets the worker binary load. The sandbox capability report of §9.2 marks the worker host's filesystem isolation, network restriction, process limit, and memory limit active only after the M6 worker-host fixture has observed each refusal. The worker holds no provider credential, opens no network connection, opens no file in any workspace or toolchain directory, and spawns no process. Everything it knows arrives over its `WorkerConnection` (§57.11) and everything it wants leaves the same way. A `MODEL_CALL` names the purpose of the request and the context items it needs by reference — files, symbols, observations, memory entries, and its own hypotheses, rejected strategies, effort grant, and pending evidence triggers as the constraint-class content of §72.9; the supervisor's `ContextOrchestrator` assembles the `ContextPackage` (§59.6), `ModelGateway` resolves the credential and makes the provider request (§48), and the worker receives the normalized response events and the context manifest — never the assembled package, the raw provider request, or the key. A `PROPOSAL` is an `AgentProposal` (§58.3) that the kernel's AUTHORIZE step (§58.2) passes to `PolicyAuthority` and `ToolBroker`; the outcome returns as a `PROPOSAL_RESULT` carrying the decision and, for an executed action, the observation identity and content the worker cites in its next request. The declared execution profile of build spec §26.5 governs those tool executions — the workspace paths, network category, and process quota of the Gradle, adb, or shell processes the supervisor runs on the worker's behalf; the worker host process itself always runs under the fixed profile above, whatever profile its contract declares, and the pre-M7 allowance of §57.2 to host the control-plane modules inside `Nirman.exe` never extends to a worker: from M5, the first milestone that runs one, every worker is a `NirmanWorker.exe` process spawned by whichever process hosts the control plane.

**Placement rule.** A component of §58, §71, or §72 is worker-hosted when its only inputs and outputs are messages: `PrivateReasoningRuntime`, `AgentReasoningEngine`, `HypothesisManager`, `StrategySelector`, `ReflectionEngine`, `StructuredReasoningSummarizer`, and every §72.2 component except the three that grant, persist, or store. A component that opens the ledger, a file, a socket, or a process handle, or that authorizes, grants, registers, schedules, or persists, is supervisor-hosted: the whole of §58 including `AgentLoopReducer`, `WorkerRuntime`, `SwarmPlanner`, and `DelegationProtocol`, together with `CapabilityRegistry`, `CapabilityBroker`, `DelegationManager`, `SwarmGraphManager`, `ReasoningStreamFilter`, `ReasoningEffortSelector`, `DeliberationContinuationManager`, `DeliberationRecordStore`, `ContextOrchestrator`, `ModelGateway`, and `ToolBroker`. A worker-hosted component that needs one of these — a capability query, an effort grant, a model escalation chosen by `DeliberationModelRouter`, a record to persist — sends the corresponding `WorkerConnection` message and receives the supervisor's answer; the effort level and the model profile a `MODEL_CALL` names are requests the supervisor admits or reduces under the worker's unchanged permission ceiling. The kernel's OBSERVE, UNDERSTAND, PLAN, and SELECT_ACTION stages are filled by the worker's `ReasoningArtifact`s and proposed action (§71.1); AUTHORIZE through EVALUATE_PROGRESS run in the supervisor, and `AgentLoopRecord` (§58.3) is written only there. ADR-119's separation of loop state from process lifecycle state is therefore a process boundary: the loop state lives in the supervisor's ledger and survives the worker, and the worker process is disposable. This supervisor-hosting rule explicitly includes §58.17 `LocalDecisionEngine` and `LocalDecisionProposalValidator`; neither component is loaded into `NirmanWorker.exe`, despite the implementation crate name `nirman-kernel`.

**Lifecycle.** A worker process lives for one lease attempt: launched when the lease is granted, exited when the worker reaches `COMPLETED`, `FAILED`, `TIMED_OUT`, or `CANCELLED` (§5.2), never pooled or reused across leases. Its one-time launch token is delivered on its standard input, never on the command line or in the environment; it connects to the per-lease pipe named in that token, completes the `WorkerConnection` handshake, and sends a heartbeat every worker heartbeat interval (build spec §26.3, 10 seconds). The supervisor declares a worker dead only from both signals of §5.2 — the process handle and heartbeat freshness: a process exit is a worker crash (§32; §27.4: preserve the workspace, record the interruption, requeue or recover), and a live process past the stale threshold (60 seconds) is terminated through `TerminateJobObject` and follows the same path. Cancellation (§58.11) is cooperative first — a `CANCEL` message the worker acknowledges after sending its last artifact — then forced through the Job Object. Resumption never depends on a paused process being alive: every artifact a worker produced is already in the ledger, so the supervisor terminates a paused worker whenever the resource policy of build spec §26.3 needs the memory and relaunches a fresh process from durable state on resume. Workers do not outlive the supervisor — closing the supervisor closes every worker Job Object — and the recovery scan of §57.4 treats a lease whose process is gone as a worker crash. A worker that exits, hangs, leaks, or is replaced never takes another worker, the supervisor, or the UI with it, and never leaves a descendant: it has none.

#### Worker startup performance evidence

Worker isolation remains process-per-lease as required by ADR-222. The runtime MUST measure worker startup without weakening that isolation.

The startup measurement chain MUST distinguish:

- process creation;
- Job Object assignment;
- AppContainer initialization;
- launch-token delivery;
- `WorkerConnection` pipe establishment;
- protocol handshake;
- first heartbeat;
- first admitted model call; and
- worker replacement from durable state.

Each measurement MUST be bound to the worker run, lease, attempt, supervisor generation, worker profile, host environment identity, and configuration version.

Worker startup latency MUST NOT be merged with provider latency, model latency, tool latency, or task completion duration.

A worker-startup observation MAY inform resource scheduling or recovery ordering. It MUST NOT change the worker permission ceiling, isolation profile, lease semantics, or authority model. It is a §69.7 measurement and confers no authority; ADR-222 is not amended by it.

**Edges.** Every inter-process edge terminates at the supervisor; the topology is a star.

| Edge | Transport | Authentication | Carries |
|---|---|---|---|
| `Nirman.exe` ↔ supervisor | `SupervisorConnection` named pipe (§57.3) | protocol handshake, installation identity, user and project scope | `UICommandEnvelope`, `UIResponseEnvelope`, `ProjectionSnapshot`, durable events, `FrameNotice` |
| supervisor → PreviewHost | shared-memory ring announced by `FrameNotice` (§10.7) | ring mapped read-only into `Nirman.exe` | frame pixels only |
| supervisor ↔ `NirmanWorker.exe` | `WorkerConnection` named pipe, one per lease (§57.11) | one-time launch token on standard input; pipe DACL granting the invoking account and the worker's per-lease container SID only (no ALL APPLICATION PACKAGES) | `MODEL_CALL`, `PROPOSAL`, results, artifacts, records, heartbeats, cancellation |
| supervisor ↔ emulator | loopback gRPC with a per-session token, and adb (§10.7) | token held by the supervisor only | screenshot stream, input, device control |
| supervisor ↔ build, shell, and tool processes | standard streams and ConPTY under the session Job Object (§3.4; §57.7) | restricted token per execution profile | commands, output, exit codes |
| supervisor ↔ AI provider | HTTPS (§48) | credential resolved from the OS credential store at request time | `ProviderRequest` and the normalized response |

No edge exists between `Nirman.exe` and a worker, between two workers, or between a worker and the emulator, a tool process, a workspace, or the provider: all such traffic passes through the supervisor. A design, crate, or fixture that introduces such an edge violates this section and ADR-222.

---

## 4. Local State and Persistence

### 4.1 Storage layers

Nirman should use SQLite for structured metadata and ordinary files for large logs, screenshots, diffs, and build artifacts.

| Storage layer | Contents | Durability |
|---|---|---|
| SQLite database | Tasks, workers, events, approvals, checkpoints, policies, provider references | Transactional |
| Project workspace | Generated source code, configuration, dependencies, project context | User-owned |
| Task directory | Raw logs, screenshots, patch files, worker reports | Durable until cleanup policy |
| Checkpoint store | Git commits, worktree references, metadata, restore information | Durable |
| Credential store | API-key references and encrypted secrets | Operating-system keychain |
| Cache directory | Repository maps, package metadata, model capability cache | Rebuildable |

### 4.2 Database requirements

The database must use transactions for task-state transitions, worker claims, event sequence numbers, approvals, and checkpoint creation. Every event should have a monotonically increasing sequence number per task so the user interface can reconnect and request only missing events.

The system must use database migrations with explicit versions. A failed migration must prevent task execution until the database is safely upgraded or restored.

### 4.3 Recovery after restart

On control-plane startup, Nirman should run a recovery scan:

```text
Open database
  ↓
Validate schema and integrity
  ↓
Find tasks in RUNNING or WAITING states
  ↓
Check worker process and workspace liveness
  ↓
Mark missing workers as interrupted
  ↓
Verify last checkpoint and event sequence
  ↓
Determine an eligible recovery strategy
  ↓
Apply it automatically
  ↓
Expose the selected strategy or escalate at a hard gate
```

The runtime must select and apply an eligible deterministic recovery strategy using failure classification, checkpoint validity, the recovery-attempt policy, risk policy, and current evidence; there is no attended variant of this scan (ADR-226). The UI exposes the selected strategy but is not required for routine recovery. A user decision is required only when policy returns `USER_REQUIRED`, `BLOCKED`, or `ESCALATED`, or when a declared hard safety, credential, signing, destructive, or emulator gate is reached, and that decision is recorded on the affected requirement while independent requirements resume (build spec §29.4). A task should never resume from an unverified partial filesystem state. It should either continue from a validated checkpoint or create a recovery branch containing the partial state for inspection.

---

## 5. Task and Worker State Machines

### 5.1 Task state machine

This machine implements the canonical task-execution state set of build spec §26.14 (`TaskExecutionState`) with exactly its names; it adds none and omits none.

```text
QUEUED → PLANNING → READY → RUNNING → VALIDATING → RECONCILING → COMPLETED
                    │          │          │
                    │          │          ├── WAITING_APPROVAL
                    │          │          ├── WAITING_RESOURCE
                    │          │          ├── RECOVERING
                    │          │          ├── PAUSED
                    │          │          └── CANCEL_REQUESTED
                    │          │
                    │          └── FAILED_RETRYABLE → RECOVERING
                    │
                    └── ESCALATED
```

`RECONCILING` resolves unknown external effects, leases, emulator sessions, or provider responses after validation and before completion (build spec §77); `PAUSED` is entered only by a user or policy directive and resumes to `RUNNING`. Every transition should include a reason, actor, timestamp, task revision, and event ID. The transition function must reject invalid transitions, such as moving a cancelled task directly to completed without a new retry decision.

### 5.2 Worker state machine

```text
CREATED → STARTING → ACTIVE → WAITING_TOOL → ACTIVE
                      │          │
                      │          ├── WAITING_APPROVAL
                      │          ├── WAITING_DEPENDENCY
                      │          └── PAUSED
                      │
                      ├── COMPLETED
                      ├── FAILED
                      ├── TIMED_OUT
                      └── CANCELLED
```

Workers should emit heartbeats while active. The scheduler should distinguish a model request that is still processing from a dead worker process by checking both process liveness and heartbeat freshness.

The worker process lifecycle maps the disposable OS process states above to the `WorkerConnection.state` transport lifecycle (nirman-schemas.md §2.90; §57.11) and owning authorities without altering the ADR-119 separation between process lifecycle and the worker's internal 13-state reasoning loop (§71.4):

| Worker State (§5.2) | `WorkerConnection.state` (nirman-schemas.md §2.90) | Transition Trigger | Transition Authority | Persistence & Side Effect |
|---|---|---|---|---|
| `CREATED` | — | Lease granted, launch token minted | `TaskScheduler` / `WorkerRuntime` | `worker_leases` record committed; token sent via stdin (§3.5) |
| `STARTING` | `CONNECTING` | `NirmanWorker.exe` spawned in AppContainer Job Object | `WorkerRuntime` | Process ID recorded; pipe created with DACL; awaiting handshake |
| `ACTIVE` | `ACTIVE` | Handshake token digest verified | `WorkerRuntime` | `WorkerConnection` established; heartbeats start (10s interval, build spec §26.3) |
| `WAITING_TOOL` | `ACTIVE` | Tool call request submitted to `ToolBroker` | `ToolBroker` / `PolicyAuthority` | Worker awaits `PROPOSAL_RESULT`; heartbeat monitoring continues |
| `WAITING_APPROVAL` | `ACTIVE` | Policy requires interactive/user approval | `PolicyAuthority` | Awaiting approval event; worker remains connected |
| `WAITING_DEPENDENCY` | `ACTIVE` | Cross-worker dependency pending | `TaskScheduler` | Durable `AwaitCondition` registered (§58.11.1); woke on satisfaction |
| `PAUSED` | `ACTIVE` | User directive or resource backpressure | `LifecycleAuthority` | Process memory preserved or serialized; resumes to `ACTIVE` |
| `COMPLETED` | `ENDED` | Worker returns final valid result artifact | `LifecycleAuthority` / `WorkerRuntime` | Results committed; process exits normally; pipe closed |
| `FAILED` | `ENDED` | Process crash, uncaught error, or memory breach | `RecoveryAuthority` / `WorkerRuntime` | Interruption recorded in `recovery_records`; lease released |
| `TIMED_OUT` | `STALE` → `ENDED` | Heartbeat missed >60s stale threshold | `TaskScheduler` / `WorkerRuntime` | `TerminateJobObject` invoked; marked recoverable failure |
| `CANCELLED` | `ENDED` | Cancellation directive propagated (§58.11) | `CancellationPropagationManager` / `WorkerRuntime` | Cooperative `CANCEL` sent; escalates to forced kill at 60s stale bound (build spec §52.12) |


---

## 6. Inter-Worker Coordination Protocol

### 6.1 Task contracts

The orchestrator should assign each worker a task contract containing:

> **Schema projection:** `TaskContract` is defined in `nirman-schemas.md` §2.1. Owner: TA §6.1.

The worker must return a structured result matching the expected output schema. Free-form commentary may be included, but the orchestrator must not depend on parsing it to determine success.

### 6.2 Message envelope

> **Schema projection:** `WorkerMessage` is defined in `nirman-schemas.md` §1.13. Owner: BS §26.2.

Messages should be stored in the database before being delivered. This makes delivery retryable and preserves communication history for debugging.

### 6.3 Coordination rules

Workers may claim only tasks whose dependencies are complete. Claims should be atomic. A worker that crashes after claiming a task should not permanently block the task; the scheduler should return it to the queue after the stale threshold and record the previous owner.

Workers may publish findings to a topic, but only the orchestrator may change the authoritative task graph. This prevents two workers from independently declaring a blocked dependency complete.

### 6.4 Worker handoff

A handoff should contain a concise summary, files inspected, files changed, commands run, tests completed, evidence references, unresolved issues, assumptions, confidence, and recommended next action. Raw logs remain available in the task directory instead of overwhelming the main context.

---

### 6.5 Canonical worker registry and swarm decomposition

The worker registry must use one canonical role taxonomy across the product, architecture, milestones, and decision records:

| Worker role | Scope | Default mutation authority |
|---|---|---|
| Primary Orchestrator | Goal decomposition, routing, synthesis | No direct file mutation |
| Repository Scout | Repository, dependency, and environment mapping | Read-only |
| Requirements Planner | Requirements, assumptions, interfaces, acceptance criteria | Planning artifacts only |
| Architecture Worker | Architecture and integration design | Design artifacts only |
| UI Worker | Frontend screens, components, styling, interactions, branding and visual assets (§56) | Assigned workspace |
| Android Data and Integration Worker | Android data layer, persistence schemas, service integrations, business logic | Assigned workspace |
| Test and QA Worker | Tests, fixtures, regression checks | Test paths and approved commands |
| Debugging Worker | Failure diagnosis and repairs | Assigned repair paths |
| Security Worker | Security, permissions, secrets, dependencies | Read-only by default |
| Visual QA Worker | Browser/device visual and accessibility checks | Read-only |
| Performance Worker | Profiling, resource use, bottleneck and regression analysis | Read-only |
| Documentation Worker | Documentation, decisions, release notes | Documentation paths |
| Release Worker | Builds, packaging, checksums, release reports | Build and artifact paths |
| Reconciliation Worker | Conflict analysis and integration validation | No direct mutation until integration |
| Emulator Driver Worker | Install, launch, scenario execution, `ScreenGraph` exploration, device hygiene, runtime evidence capture (§10.2, §62.1, §73.12) | Device adapter operations only |
| Diagnostic Worker | Root-cause localization (§63.2) and `FailureContextPackage` production for a parent worker | Read-only; one probe child |
| Content Worker | `ContentMutation` proposals for copy, localization, and accessibility text (§85.2) | Proposal only |
| Integration Double Worker | `ContractDouble` fixtures and schema conformance (§74.1) | Double fixtures only |
| Critic Worker | Hosts `StrategyCritic` (§72.7) and independent pre-promotion review; findings and evidence requests only | Read-only |
| Android Platform Worker | Android SDK/platform APIs, Kotlin/Java interop, JNI/native modules, Gradle/plugin integration, lifecycle/process/background execution, permissions, services, device APIs (BLE/NFC/camera/sensors), widgets, OS-version compatibility, platform-specific diagnostics | Assigned workspace; platform tooling |
| Backend & Service Engineering Worker | REST/GraphQL API implementation, server-side business logic, database/server schema, authentication/authorization backend, webhooks, server-side validation, background jobs/queues, cloud functions/serverless, API versioning, backend integration tests, deployment configuration for the user's external backend | Assigned workspace; backend tooling |

The orchestrator should select swarm size from task complexity, dependency coupling, changed-file boundaries, target platforms, interface agreements, expected validation cost, and available resources. It should prefer one worker for tightly coupled work, parallel read-only workers for exploration and review, and isolated write-capable workers only when file and interface boundaries are explicit.

For coupled work, the orchestrator must create an interface agreement before parallel implementation. The agreement may contain API shapes, shared types, route contracts, database schemas, event formats, design tokens, or artifact contracts. Workers validate against it before reconciliation.

Worker nesting is limited to three levels by default (build spec §23.4; ADR-227): the Primary Orchestrator delegates to workers; a worker may request one Diagnostic Worker child; a Diagnostic Worker may request one probe child (a Repository Scout or Emulator Driver Worker instance restricted to observation actions). A probe child cannot create children; no child changes the parent contract, expands permissions, or integrates changes; every `DelegationGrant` (§71.7) carries `depth ≤ maxDepth = 3`. All worker handoffs remain attached to the parent task graph. The observation-only fan-out roles — Repository Scout, Security Worker, Visual QA Worker, Performance Worker, and Critic Worker — may inspect one revision in parallel without source/workspace mutation reservations. Requirements Planner may write only its permitted planning/design artifacts; Architecture Worker may write only its permitted design artifacts; Backend & Service Engineering Worker is write-capable and remains reservation-bound. Diagnostic Worker is restricted to its delegated diagnostic-child/probe role. These mutation distinctions are canonical across BS §22.1, BS §23.4, and TA §6.5.

## 7. Scheduler and Background Execution

### 7.1 Scheduler responsibilities

The scheduler selects runnable tasks, reserves resources, launches workers, manages dependencies, handles approvals, records heartbeats, detects stale processes, and decides whether to retry or escalate failures.

A scheduler tick should be deterministic and idempotent. Running the same scheduling cycle twice must not launch duplicate workers for the same contract.

#### Scheduler tick determinism contract

A scheduler tick MUST consume one immutable `SchedulerInputSnapshot`. The snapshot MUST contain:

- runnable task and dependency state;
- current task and project revisions;
- worker lease and fencing state;
- physical resource-integrity snapshot;
- fair-share and priority-aging state;
- validation reservations;
- starvation-age values;
- admitted policy decisions;
- historical `ResourceProfile` references;
- scheduler configuration version; and
- the input event-sequence watermark.

The tick MUST produce one `SchedulerDecisionSnapshot` containing:

- selected task identities;
- worker launch intents;
- resource reservations;
- deferred or rejected task identities;
- deterministic reason codes;
- next-wake condition; and
- output event-sequence identity.

For equal input snapshots, policy version, configuration version, authority state, and resource observations, the scheduler MUST produce an equivalent decision snapshot.

Wall-clock progression, process scheduling order, UI connection state, provider response order, and worker message arrival order MUST NOT change the decision after the input snapshot has been captured.

A task, worker, lease, or attempt that is already admitted MUST NOT be launched again by a replayed or repeated scheduler tick. A retry MUST use a new authorized attempt or lease identity.

The scheduler MUST persist the input snapshot identity, decision identity, reason codes, and event watermark needed to replay and audit the decision.

These records are implementation-facing payloads of the existing scheduler contract; no new scheduler, no new authority, and no separate decision store is introduced, and the field names above are not registered schemas (see §69.7 for measurement ownership and build spec §52 for scheduling policy).

### 7.2 Resource-aware scheduling

The scheduler MUST combine fair-share eligibility, project/task priority, dependency readiness, physical resource availability, validation urgency, historical ResourceProfile evidence, and starvation age into one deterministic scheduling decision. Historical performance is advisory routing data, never authority.

The scheduler SHOULD maintain a rolling operational measurement of queue wait, dispatch latency, worker-start latency, resource-pressure state, starvation age, recovery-caused churn, and validation reservation contention. These measurements MUST remain subordinate to Scheduler, LifecycleAuthority, and ResourceIntegrityAuthority.

The scheduler MUST reduce concurrency before violating resource-integrity limits and MUST preserve recovery/validation capacity where required. It MUST detect starvation independently of worker liveness.

Initial defaults should be configurable and conservative:

| Resource | Default |
|---|---:|
| Write-capable workers per task | 3 |
| Read-only workers per task | 5 |
| Global active workers | 8 or available-resource policy |
| Worker heartbeat | 10 seconds |
| Worker stale threshold | 60 seconds |
| Default task time policy | No autonomous-goal completion deadline. Liveness timeouts MAY exist for hung operations and process containment but MUST NOT terminate a healthy goal for elapsed time |
| Default task disk quota | 10 GB unless project policy overrides (build spec §26.3, §80.3 range 1–100 GB); emulator, build, cache, and checkpoint storage are charged against the same quota |
| Default repair strategy changes | 3 |
| Exhaustion of materially equivalent attempts MUST trigger strategy transformation, delegation, backtracking, branching, or escalation. Exhaustion MUST NOT itself terminate the goal. |

### 7.3 Background approval notifications

An approval request should be durable. If the application is minimized, the control plane may issue a Windows notification. If the application is closed, the request should appear when Nirman reopens.

Approval requests must expire under the two build spec §26.13 rules — context expiry when the bound action, task state, or policy changes, and clock expiry at the per-project §80.3 approval-expiry setting (default 24 hours, range 1–168 hours), whichever comes first. The user can approve once, approve a matching rule for the session, deny once, deny the worker, or pause the entire task. The approval record must include the exact command, path, worker, workspace, policy, and task state.

### 7.4 Scheduled tasks

Scheduled tasks should be implemented only after reliable background execution exists. A schedule record should contain a local cron-like expression or interval, project ID, task prompt, allowed mode, resource requirements, notification policy, and whether approval is required.

Scheduled tasks should never automatically publish, push, spend money, or use personal credentials. They may run local checks, update documentation, refresh dependencies in a restricted workspace, or generate reports according to user policy.

### 7.5 Reboot, sleep, and notification resilience

The stable supervisor should register a per-user startup entry for projects with active tasks. It must detect boot, login, suspend, resume, hibernate, and shutdown transitions and write those transitions to the task event ledger.

During active Goal Mode work, the runtime should request an operating-system execution power policy where supported so the machine does not enter sleep while a build, test, emulator, or provider operation is active. The user must see this setting and may disable it. If sleep or hibernation still occurs, the supervisor must mark active processes stale, revalidate provider requests, restart eligible local processes, restore ports and emulator state where possible, and resume from the last validated checkpoint.

Approval and warning events must have multiple delivery paths: in-app queue, tray badge, operating-system notification, task history, and startup summary after reboot. If notifications are suppressed, the task must not remain invisibly parked; the control plane should record the pending decision and show it on the next connection. The Autonomous-build policy should resolve routine approval states by policy, while genuine hard-gated decisions remain visible and durable.

---

## 8. Workspace Isolation and Reconciliation

**Implements:** build spec §22 and `CONTRACT.RUNTIME.WORKSPACE` (with §46; the build spec section is the authority)

### 8.1 Workspace types

| Workspace type | Purpose | Write target |
|---|---|---|
| Main workspace | User’s active project | Direct only under approved policy |
| Worker worktree | Isolated implementation task | Worker branch/worktree |
| Review copy | Non-mutating analysis | Temporary copy |
| Disposable build workspace | Untrusted dependency or build | Temporary isolated environment |
| Integration workspace | Reconcile multiple worker results | Integration branch/worktree |

### 8.2 Isolation rules

Every worker receives an absolute workspace path and an allowed-path policy. Relative paths must be resolved and checked before use. Symlinks, junctions, and path traversal must be evaluated so a permitted project path cannot unintentionally expose a protected directory.

Two write-capable workers may share a parent revision but must not share a mutable workspace. The main workspace remains untouched until reconciliation succeeds.

### 8.3 Reconciliation algorithm

The reconciliation worker should:

1. Compare each worker’s parent revision to the main integration revision.
2. Build a changed-file and changed-symbol graph.
3. Apply non-overlapping changes in deterministic order.
4. Identify overlapping files, dependency changes, route conflicts, schema conflicts, and incompatible assumptions.
5. Ask a reviewer worker to propose an integration patch.
6. Apply the proposal in the integration workspace.
7. Run formatting, linting, type checks, tests, and builds.
8. Create an integration checkpoint only after required gates pass.

If integration fails, the integration workspace remains available for inspection and the main workspace remains unchanged.

---

## 9. Sandbox and Security Architecture

### 9.1 Execution profiles

Nirman should implement the five execution profiles of build spec §26.5, which is the canonical profile set; this table restates it without adding or removing a profile:

| Profile | Characteristics |
|---|---|
| Trusted local | Fast, user process, workspace and command policies |
| Restricted process | Restricted token, process-tree control, workspace paths, environment filtering; the default for autonomous execution |
| High-risk restricted process | Strongest native boundary for untrusted repositories and risky dependencies |
| Disposable/Isolated | Temporary, fully isolated environment for untrusted code execution; destroyed after use |
| Review-only | No write access and no arbitrary process execution; diff, security, and architecture analysis |

The interface should explain when a requested operation requires a stronger profile. A worker must not be able to switch itself to a weaker profile.

### 9.2 Windows process controls

The Windows runtime should use process-tree management and resource accounting through Windows Job Objects where available. It should use restricted process tokens, controlled environment variables, explicit working directories, and deny-by-default access to protected paths.

The sandbox abstraction must not rely on a single Windows API. It should expose capabilities such as filesystem isolation, network restriction, process limits, memory limits, CPU limits, and disposable cleanup, then report which capabilities are active.

### 9.3 Network policy

Network access should be categorized as provider traffic, package-manager traffic, Android runtime traffic, Nirman-managed local Android emulator traffic, or external-tool traffic. Each category should have an independent policy.

The default autonomous build profile should allow provider requests and approved Android dependency sources only. Nirman-managed local Android emulator traffic and Android project network access should be explicitly visible. External network access should be disabled in high-risk review profiles.

### 9.4 Dependency safety

Before executing an unfamiliar dependency or install script, Nirman should record its source, version, lockfile change, requested scripts, and scan status. Unverified packages should be restricted to a disposable or explicitly approved environment.

---

## 10. Preview and Device Architecture

### 10.1 Android development preview manager

The preview manager starts the Android development server or native build process, assigns or discovers required ports, tracks the process tree, checks Nirman-managed local Android emulator readiness, installs or reloads the application, captures Logcat and runtime errors, and exposes the current emulator state to the desktop interface.

A preview instance must be associated with a project revision and checkpoint. If the revision changes, the preview reports whether it hot-reloaded or restarted. If the project is rolled back, the preview must be restarted or marked stale.

### 10.2 Android device-profile testing

A preview test can define multiple Android emulator profiles:

> **Schema projection:** `AndroidDeviceProfile` is defined in `nirman-schemas.md` §2.2. Owner: TA §10.2.

The Emulator Driver Worker (§6.5; ADR-227) should install the build, launch activities, execute synthetic interactions, capture screenshots, record Logcat and crash output, verify permissions and orientation, and return a structured visual report.

The emulator validation subsystem MUST expose an authoritative `InteractionExecutor`.

> **Schema projection:** `InteractionExecutor` is defined in `nirman-schemas.md` §2.3. Owner: TA §10.2.

The InteractionExecutor operates only against the running generated Android application through an admitted Android device adapter. It MUST NOT satisfy an interaction requirement by modifying source, invoking application internals outside the declared test interface, or asserting an expected state without observing it.

Every interaction produces an observed result that enters the normal Evidence → ValidationResult → CompletionDecision chain.

A synthetic interaction is an `InteractionExecutor` action whose target is a `ScreenModel` element identity (§74.2) and whose steps come from an `E2EScenario` — authored, or synthesized by `ScenarioSynthesizer` from the `ScreenGraph` (§62.1; ADR-225). The emulator worker does not improvise taps against pixels.

### 10.3 Android emulator manager

The Android emulator manager should provide a normalized interface for Nirman-managed Android emulators:

In Nirman's Android runtime contracts, a Device represents a Nirman-managed Android emulator session or emulator profile. Physical Android hardware is not an admissible Device implementation.

> **Schema projection:** `Device` is defined in `nirman-schemas.md` §2.4. Owner: TA §10.3.

The first release may support one active device, but the interface should not assume that limitation. Device logs, installation results, reload failures, and build artifacts should be attached to the task record.

> **Schema projection:** `DeviceHygienePolicy` is defined in `nirman-schemas.md` §2.93. Owner: TA §10.3.

> **Schema projection:** `GoldenSnapshot` is defined in `nirman-schemas.md` §2.94. Owner: TA §10.3.

Determinism is a mechanism, not an aspiration (ADR-225). After `boot()` and before any application is installed, the emulator manager applies the session's `DeviceHygienePolicy` through the device adapter — animation scales zero, keyguard disabled, stay-awake on, setup wizard skipped, locale, timezone, font scale, and density pinned, auto-rotate off — verifies each setting by reading it back, and records the result; a policy that cannot be verified leaves the device `PROVISIONED_UNVERIFIED` for validation purposes. It then takes one `GoldenSnapshot` per device session. Every scenario run, and the first exploration pass of §62.1, begins with `restore(goldenSnapshotId)` followed by a fresh install and the scenario's `seedData`; a scenario that starts from any other state is not deterministic evidence. The golden snapshot is invalidated by a system-image, hygiene-policy, or device-profile change and is retaken, never patched. The user-facing preview surface (§10.8) is never restored underneath the user without a visible `DRIVEN_BY_SCENARIO` state (build spec §29.3).

### 10.4 Screenshot and visual-specification pipeline

The input manager should accept screenshots, image sets, annotated references, and optional user assets as first-class project inputs. It should create a durable `VisualReference` record:

> **Schema projection:** `VisualReference` is defined in `nirman-schemas.md` §2.5. Owner: TA §10.4.

The Visual QA Worker (§6.5) converts references into an editable visual specification rather than directly copying pixels. The specification records screens, navigation states, layout regions, component roles, spacing, typography, colors, assets, interactions, responsive behavior across Android emulator profiles, and unresolved uncertainties. The UI Worker uses that specification to synthesize Android code, while the Visual QA Worker compares Nirman-managed local Android emulator screenshots against the reference and reports visual differences with evidence.

Screenshots sent to a cloud model must pass the project privacy policy. The system must redact or warn about sensitive text and identify the provider receiving the image. A visual reference is never treated as executable instruction; it is input data interpreted through the task contract.

A `VisualReference` establishes visual intent only. It never establishes behavior.

A reference image MAY establish: layout structure, visual hierarchy, component identity for standard components, color relationships, approximate spacing rhythm, and typographic scale.

A reference image MUST NOT be treated as establishing:

| Not establishable from an image | Must instead come from |
|---|---|
| Dynamic behavior on interaction | The instruction or a clarifying question |
| Input validation rules | The instruction or a clarifying question |
| Responsive behavior across sizes and orientations | The emulator profile and technology plan |
| Animation, transition, or gesture | The instruction; otherwise the build spec §5.4 ceiling applies |
| Semantic purpose of a non-standard component | A clarifying question |

The `interactionClues` field records hypotheses only. A hypothesis in that field MUST NOT be promoted to a requirement without confirmation from the instruction or an answered clarifying question. Every unpromoted hypothesis belongs in `uncertaintyNotes` and in the intent model's unresolved ambiguities.

Spacing, typography, and color derived from an image are estimates, not measurements. They MUST be recorded as assumptions with their derivation noted, and MUST yield to any explicitly stated value.

The primary perception channel of the autonomous loop is the `ScreenModel` derived from the running application's UI hierarchy (§74.2; ADR-225), not the screenshot: a configured vision model is optional, and its absence marks visual criteria `NOT_OBSERVED` without blocking functional completion. Visual comparison of a Nirman-managed local Android emulator screenshot against a reference is a visual observation only. It contributes evidence about appearance; it never establishes that behavior, state, or navigation is correct. Behavioral proof comes from the stateful scenarios of BS §56 (CLAUSE.EVIDENCE.CLAIM_SEPARATION applies unchanged).

### 10.5 Dynamic Android project synthesis

The project synthesizer builds a project graph from the goal contract, visual specification, existing files, assets, emulator requirements, integrations, and validation plan. It selects or composes the required Android technologies and creates the project structure, screens, navigation, state, data layer, permissions, services, tests, build configuration, and artifact profile.

The technology resolver must treat all Android implementation styles as available capabilities. It may select Java, Kotlin, Android Views, Jetpack Compose, custom NDK/CMake native modules, Gradle plugins, background services, device APIs, or a mixed native architecture (ADR-257). Its decision must be based on the requested behavior, screenshot evidence, performance needs, device APIs, offline requirements, build constraints, dependency compatibility, and validation evidence—not on a fixed user-facing template list. Because the Nirman-managed emulator runs an x86_64 system image, the resolver prefers dependencies that ship x86_64 native libraries and records any arm64-only native dependency in the `AndroidTechnologyPlan` as an environment requirement of the emulator profile (build spec §79.17).

The plan record is the build spec §80.5.1 `AndroidTechnologyPlan`, field for field; the build spec is the canonical owner and the typed definition is the single block named below:

> **Schema projection:** `AndroidTechnologyPlan` is defined in `nirman-schemas.md` §1.48. Owner: BS §80.5.1.

Internal bootstraps may provide known-good build foundations, but they are implementation details rather than product limitations. The resolver must be able to create different project shapes, combine technologies, replace an incompatible layer, and add native modules when validation proves that the current architecture cannot satisfy the goal. The user may inspect the technology plan, but should not be required to choose the stack before describing the desired application.

Project synthesis must be incremental. It should first create a buildable Android shell, then implement the visual and behavioral contract, then integrate data and device capabilities, and finally harden the project through tests, visual comparison, packaging, signing-boundary checks, and recovery. Each synthesis stage produces a checkpoint and evidence.

### 10.6 Android device and host isolation

Android validation MUST use disposable Nirman-managed local Android emulator snapshots only. Physical Android hardware is outside product scope and MUST NOT participate in preview, validation, recovery, completion, or delivery. It must not reuse personal credentials, host-side secrets, or unapproved emulator data. Test data should be synthetic by default. Emulator sessions, installed packages, permissions, logs, screenshots, and cleanup state must be attached to the task record.

### 10.7 Emulator frame transport and pipeline watchdog

The Nirman-managed local Android emulator is the canonical PreviewRuntime for the primary development workflow. It MUST run headless on the Windows host and its rendering surface MUST be projected into the WinUI 3 PreviewHost. A detached emulator window is not a valid primary preview.

No physical Android hardware is supported, required, or accepted as an alternative runtime. The Nirman-managed local Android emulator is the sole canonical Android preview and validation runtime.

**RenderPipelineWatchdog**

The supervisor owns the render pipeline watchdog and its deterministic state machine. PreviewHost does not own, start, stop, or recover the pipeline.

Responsibilities:
- detect emulator boot without frame
- detect frame stream without preview connection
- detect preview connection without current artifact
- detect frame identity mismatch
- detect frame sequence gaps
- detect frame stagnation
- detect application-process death
- detect surface-size mismatch
- detect transport generation rollover
- detect stale frame painting
- trigger deterministic recovery

State machine:
STARTING → WAITING_EMULATOR → WAITING_APP → WAITING_FRAME → CONNECTED → DEGRADED → LOST → RECOVERING → CONNECTED | FAILED

Recovery order is deterministic and supervision-owned:
re-observe → inspect process → inspect Logcat → inspect frame stream → verify artifact/session identity → refresh render transport → relaunch app → reinstall artifact → restore golden snapshot → rebuild → architecture-level diagnosis

Failure kinds the watchdog reports:
- PREVIEW_NO_FRAME
- PREVIEW_BLACK_FRAME
- PREVIEW_STALE_FRAME
- PREVIEW_FROZEN_FRAME
- PREVIEW_IDENTITY_MISMATCH
- PREVIEW_STREAM_DISCONNECTED
- PREVIEW_FRAME_GAP
- PREVIEW_SURFACE_RESIZE_MISMATCH
- PREVIEW_APP_PROCESS_DEAD
- PREVIEW_RENDER_TRANSPORT_LOST

> **Schema projection:** `RenderTransport` is defined in `nirman-schemas.md` §2.89. Owner: TA §10.7.
> **Schema projection:** `FrameNotice` is defined in `nirman-schemas.md` §2.111. Owner: TA §10.7.

> **Schema projection:** `AndroidRuntimeObservation` is defined in `nirman-schemas.md` §2.109. Owner: TA §10.7.

> **Schema projection:** `FrameQualityObservation` is defined in `nirman-schemas.md` §2.110. Owner: TA §10.7.
> **Schema projection:** `LaunchSession` is defined in `nirman-schemas.md` §1.85. Owner: TA §10.7.
> **Schema projection:** `FrameStamp` is defined in `nirman-schemas.md` §1.86. Owner: TA §10.7.

**LaunchSession and FrameStamp binding.** `LaunchSession` is the durable session identity of one application launch. It is persisted by the storage authority within the launch transaction family of §36.5 and commits with the `LaunchTransactionCommitted` stage of build spec §69.4.1 — the launch `ExternalEffectRecord` committed with `LaunchSession.committedAt` (`nirman-schemas.md` §1.85) written and `DeviceTransaction.observationState` (`nirman-schemas.md` §2.35) at `LAUNCHED`; `startedAt` is written at launch begin. It carries no status field of its own: launch state derives from `DeviceTransaction.observationState` and `PreviewRevision.previewAuthorityState` (`nirman-schemas.md` §1.35) only. After supervisor restart or an interrupted launch, its state is reconstructed from `DeviceTransaction.observationState` and `ExternalEffectTransaction.reconciliationState` (`nirman-schemas.md` §2.36) under the existing recovery/reconciliation authority; an unresolved launch outcome stays in reconciliation until the effect and device records resolve it. Cancellation and restart follow the existing lifecycle authority and the external-effect compensation path — a cancelled or fenced launch commits no session record that can advance preview. A `LaunchSession` is invalidated by a newer `previewRevisionId`, by emulator-session or device loss, or by artifact-fingerprint mismatch, and is never evidence: `AndroidRuntimeObservation` records bind to it via `launchSessionId`, and device or session loss invalidates dependent runtime evidence under the existing evidence-invalidation rules. `FrameStamp` exists per delivered frame and transport generation only — never persisted as durable state or evidence, superseded by each new frame and by transport-generation rollover. M9 owns the runtime fixture family for both identities; the documentation verifier's mutation battery owns the negative fixtures.

**Baseline and permitted transport upgrade.** The transport is a named, versioned interface. Its required baseline is the emulator's local gRPC control endpoint on loopback, using its screenshot-stream RPC. Low frame rate, minimal dependencies, sufficient for a truthful preview. The permitted upgrade is a WebRTC/video-stream path for higher frame rate and input forwarding, admitted through the SAME `PreviewPromotionGate`. Not a second authority.

Every delivered frame MUST bind `projectRevisionId`, `previewRevisionId`, `artifactFingerprint`, `deviceId`, `emulatorSessionId`, `deviceStateFingerprint`, `applicationStateFingerprint`, `runtimeObservationId`, `renderTransportGeneration`, and `interactionCausalityId`. An unbound frame MUST be labelled `STALE` and MUST NOT satisfy completion (CLAUSE.PREVIEW_SYNC.IDENTITY_MATCH, CLAUSE.PREVIEW_SYNC.EVIDENCE_BOUND). Frame capture is an `AndroidDeviceAdapter` operation carrying `adapterId`, `adapterVersion`, `technologyPlanHash`, and `deviceAdapterIdentity` (CLAUSE.PREVIEW_SYNC.ADAPTER_BOUND). Transport loss MUST invalidate the projection through the single canonical reducer (CLAUSE.PREVIEW_SYNC.SINGLE_REDUCER) and MUST NOT freeze the last frame while presenting it as live (CLAUSE.PREVIEW_SYNC.NO_LOCAL_ADVANCE). Loopback only. The transport MUST NOT bind to an external interface. Physical Android hardware has no supported capture, transport, validation, or preview path.

**Preview/launch bridge binding:** `AndroidRuntimeObservation.previewRevisionId` MUST equal `PreviewRevision.previewRevisionId`. `AndroidRuntimeObservation.artifactFingerprint` MUST equal `PreviewRevision.artifactFingerprint`. `AndroidRuntimeObservation.emulatorSessionId` MUST equal `PreviewRevision.emulatorSessionId`. `AndroidRuntimeObservation.renderTransportGeneration` MUST equal `RenderTransport.renderTransportGeneration`. `AndroidRuntimeObservation.launchSessionId` binds to the `LaunchSession` that produced this observation. `AndroidRuntimeObservation.observationSequence` is monotonic per `previewRevisionId`.

**Transport mechanics.** The `RenderTransport` is a supervisor-owned object, one per emulator session, created by the Android emulator manager (§10.3) when `AndroidDeviceAdapter.boot()` succeeds and destroyed on `release()`:

- *Emulator side.* The supervisor — never a worker, never PreviewHost — opens the single gRPC channel and subscribes to the screenshot stream (`streamScreenshot`, RGBA8888, device-native resolution, the emulator delivering a frame only when the display changes) per ADR-221. The token, the port, and the channel are never exposed on the named pipe or to any other process.
- *Frame path.* Each frame is stamped by the supervisor with `frameSequence`, `capturedAt`, `width`, `height`, `pixelFormat`, `deviceId`, `previewRevisionId`, `artifactFingerprint`, and `deviceStateFingerprint` — the binding above — and written to a per-surface shared-memory ring (`transportKind: SHARED_MEMORY_RING`) of `ringDepth` slots (default 3). PreviewHost learns of the frame through a `FrameNotice` — `previewSurfaceId`, `ringSlot`, `frameStamp` — sent on the authenticated `SupervisorConnection` (§57.3) as a volatile display message: a `FrameNotice` is not a `PreviewSyncEvent`, is never appended to the durable event log, carries no `eventSequence`, is never replayed, and is dropped without record when the reader is behind. Frame pixels and frame notices never travel through the durable event log.
- *Frames are pixels; events are meaning.* `PreviewSyncEvent`s are emitted only on a change of stream state, never per frame: `STREAM_RECONNECTED` when the first stamped frame arrives after boot or after a gap (the reducer sets `PreviewProjection.streamStatus: CONNECTED`), `STREAM_GAP` when the heartbeat fails (`STALE_STREAM`), and `OBSERVATION_CAPTURED` only when `captureScreenshot()` produces an evidence record. PreviewHost MAY paint a frame as live only when the reduced projection's `streamStatus` is `CONNECTED`, a `FrameNotice` arrived within `staleAfterMs`, and the notice's `frameStamp` binds the currently promoted PreviewRevision's: `projectRevisionId`, `previewRevisionId`, `artifactFingerprint`, `deviceId`, `emulatorSessionId`, `deviceStateFingerprint`, `applicationStateFingerprint`, `runtimeObservationId`, `renderTransportGeneration`, and `interactionCausalityId` when the frame is causally attributable to an interaction; a notice failing any identity check is discarded before painting and produces a stale/invalidated display state; PreviewHost MUST NOT repair or reinterpret identity, and the panel shows the last frame under a `STALE` label instead. Liveness therefore has two independent witnesses — durable stream state and a fresh bound frame — and neither the UI nor a lagging log can produce it alone (CLAUSE.PREVIEW_SYNC.SINGLE_REDUCER, CLAUSE.PREVIEW_SYNC.IDENTITY_MATCH, CLAUSE.PREVIEW_SYNC.NO_LOCAL_ADVANCE).
- *Rate and backpressure.* `maxFrameRate` is 30 frames per second for the baseline transport; when PreviewHost has not consumed a slot, the oldest unconsumed frame is overwritten (`backpressurePolicy: DROP_OLDEST`), never buffered without bound and never blocking the emulator. A gap of more than `staleAfterMs` (default 2000) without a new frame while the device reports the display unchanged is `IDLE` (the last frame stays painted, no event); the same gap with a failed heartbeat is `LOST`, on which the supervisor emits one durable `STREAM_GAP` and the reducer projects `STALE_STREAM` under CLAUSE.PREVIEW_SYNC.NO_LOCAL_ADVANCE.

RenderTransport MUST calculate deterministic frame-health observations from FrameStamp identity, FrameNotice drop counts, capture/transport/render timing, frame age, changed-pixel ratio, blank-pixel ratio, and freeze state. These observations are diagnostic/evidence inputs only; they MUST NOT promote preview truth.

A detected frozen, blank, stale, dropped, or presentation-path mismatch MUST trigger the existing preview recovery ladder before any new transport is selected. Transport substitution is permitted only when identity, evidence lineage, and PreviewPromotionGate semantics remain unchanged.
- *PreviewHost side.* PreviewHost maps the ring read-only, presents each frame on a WinUI 3 `SwapChainPanel` (Win2D/Direct2D), and falls back to a `WriteableBitmap` in `Image` when the GPU surface is unavailable; the fallback is recorded in `PreviewSurface.status`, never silent. Frames are scaled to the panel with aspect preserved; the pixel-to-device coordinate transform is derived from the frame `width`/`height` and applied to every `PreviewInteraction` before it leaves PreviewHost.

PreviewHost is a pure projection sink. It MUST NOT start the emulator, install APK, call ADB, call Gradle, read project files, mutate preview state, or decide currentness. It receives projection and mapped frame, validates stamp, paints, and reports surface health only.
- *Input path.* `PreviewInteraction` commands (tap, long-press, swipe, key, text, rotate) travel PreviewHost → `SupervisorConnection` (§57.3) → Supervisor → `AndroidDeviceAdapter.interact()`; the adapter injects them through the same gRPC channel (`sendTouch`, `sendKey`) and the resulting frame carries the interaction's `interactionId` in its stamp so evidence can pair an action with its observed post-state.
- *GPU.* The emulator runs with host GPU acceleration when the host GPU is usable and with the SwiftShader software renderer otherwise; both are valid render sources and the choice is recorded in `RenderTransport.gpuMode`. The hypervisor has no such fallback (build spec §79.16).
- *Upgrade.* The WebRTC path (`transportKind: WEBRTC_LOOPBACK`) replaces the ring with a loopback media stream at up to 60 frames per second under the same stamps, the same reducer, and the same `PreviewPromotionGate`; a transport upgrade never changes what counts as evidence.

### 10.8 PreviewHost

PreviewHost is the WinUI 3 presentation surface that displays the rendering projection of the Nirman-managed Android emulator.

PreviewHost does not execute Android commands and is not an authority.

Required path:

Android Emulator
→ AndroidRuntimeObservation
→ RenderTransport
→ FrameStamp
→ FrameNotice
→ PreviewHost

Runtime state transitions (durable events):

PreviewCoordinator
→ PreviewSyncEvent
→ PreviewProjectionReducer
→ PreviewHost

Input follows the reverse controlled path:

WinUI PreviewHost
→ typed PreviewInteraction command
→ Supervisor
→ AndroidDeviceAdapter
→ emulator input channel

The PreviewHost MUST NOT invoke ADB, Gradle, emulator APIs, or application internals directly.

> **Schema projection:** `PreviewSurface` is defined in `nirman-schemas.md` §2.6. Owner: TA §10.8.

> **Schema projection:** `PreviewInteraction` is defined in `nirman-schemas.md` §2.7. Owner: TA §10.8.

These are execution and projection records, not authorities.

## 11. Toolchain and Environment Management

### 11.1 Version resolution

Each project should declare required tool versions or compatible ranges. The runtime should resolve those requirements through local version managers, portable installations, or configured executable paths, then isolate each project through environment filtering, cache separation, process scopes, and toolchain bindings.

A project environment record should include:

> **Schema projection:** `EnvironmentRecord` is defined in `nirman-schemas.md` §2.8. Owner: TA §11.1.

Two projects that require different Node.js, Java, Android SDK, Rust, or package-manager versions must be able to run without silently changing global state.

### 11.2 Environment diagnostics

Diagnostics should distinguish missing, incompatible, inaccessible, unverified, and healthy tools. A failed build must name the missing executable or incompatible version and explain the next action.

### 11.3 Android runtime abstraction

The runtime should expose Android-focused interfaces for process execution, filesystem policy, environment discovery, Java/Kotlin compilation, Gradle execution, NDK/CMake native module builds, Nirman-managed local Android emulator management, Logcat, quotas, screenshots, signing-boundary checks, and APK artifacts. The Windows desktop host supplies the local process and sandbox implementation; the generated-project contract remains Android-specific and technology-neutral.

---

### 11.4 Persistent terminal session manager

The runtime should expose a `TerminalSession` abstraction instead of treating every command as a one-shot shell call.

> **Schema projection:** `TerminalSession` is defined in `nirman-schemas.md` §2.9. Owner: TA §11.4.

A worker can reuse a terminal session for commands that depend on working directory, environment variables, virtual-environment activation, package-manager state, or a long-running development server. Session environment changes must be explicit and recorded rather than inferred from arbitrary shell output.

The terminal manager must detect interactive prompts through known prompt signatures, stdin readiness, process activity, and the closed non-extensible prompt-classifier set defined in build spec §80.3 and §23.7 (not user-configurable, preventing bypass of the §23.7 gate). It should answer only declared safe prompts using a task policy; otherwise it should terminate safely, capture the prompt, and classify the task as requiring a decision. Dev servers and emulators must be registered as long-running processes rather than mistaken for hung commands.

Shell selection must be explicit on Windows. Supported profiles may include PowerShell, `cmd.exe`, Git Bash, or another approved native-Windows shell. The selected profile, executable path, version, encoding, and environment fingerprint belong in task evidence.

Each worker gets a separate terminal view in the UI. Output uses rolling files with size- and time-based rotation, searchable indexes, compressed historical segments, and preserved error excerpts. Rotation must never discard the final evidence needed to diagnose a failure.

## 12. Observability and Testing

### 12.1 Event stream

The control plane should emit events such as `task_started`, `plan_created`, `worker_started`, `tool_requested`, `approval_requested`, `tool_started`, `tool_completed`, `checkpoint_created`, `validation_completed`, `recovery_started`, `worker_failed`, and `task_completed`.

Events are persisted before being sent to clients. The UI can reconnect using the task ID and last received sequence number.

### 12.2 Health checks

Nirman should expose health checks for the control plane, database, provider connection, worker registry, process manager, workspace storage, toolchain, preview manager, and notification service.

### 12.3 Architecture tests

The engineering test suite should include database recovery tests, event replay tests, duplicate-message tests, worker heartbeat tests, quota tests, path-boundary tests, process-tree cancellation tests, reconciliation conflict tests, preview rollback tests, and toolchain isolation tests.

---

## 13. Suggested Module Boundaries

```text
nirman/
├── desktop-ui/
│   ├── chat/
│   ├── workspace/
│   ├── preview/
│   ├── tasks/
│   └── settings/
├── control-plane/
│   ├── api/
│   ├── scheduler/
│   ├── event-bus/
│   ├── approvals/
│   ├── persistence/
│   └── recovery/
├── agent-runtime/
│   ├── orchestrator/
│   ├── worker-registry/
│   ├── contracts/
│   ├── handoffs/
│   ├── reconciliation/
│   └── context-engine/
├── tool-gateway/
│   ├── filesystem/
│   ├── process/
│   ├── android-device/
│   ├── preview/
│   ├── devices/
│   └── external-tools/
├── policy-engine/
│   ├── rules/
│   ├── approvals/
│   ├── path-checker/
│   ├── command-checker/
│   └── quota-checker/
├── provider-runtime/
│   ├── adapters/
│   ├── router/
│   ├── capability-detection/
│   ├── streaming/
│   └── usage/
├── project-runtime/
│   ├── workspaces/
│   ├── git/
│   ├── toolchains/
│   ├── builds/
│   └── artifacts/
└── tests/
    ├── unit/
    ├── integration/
    ├── recovery/
    ├── security/
    └── fixtures/
```

---

## 14. Architecture Decisions Required Before Coding

The engineering team must decide the following before implementing the control plane:

| Decision | Recommended default |
|---|---|
| Local IPC | Authenticated named pipes (`SupervisorConnection`, §57.3); a loopback HTTP/WebSocket endpoint is permitted only for development and debugging behind a per-installation secret (§2) and never carries the production `SupervisorConnection` |
| Metadata storage | SQLite with migrations and WAL mode where appropriate |
| Task logs | Append-only files referenced from SQLite |
| Worktree management | Git worktrees with temporary copy fallback |
| Worker process | One `NirmanWorker.exe` child process per worker lease under the fixed worker-host profile of §3.5; the declared execution profile governs the tool executions the supervisor runs for it |
| Scheduler | Single authoritative local scheduler process |
| Event delivery | Durable event log with sequence-based replay |
| Initial sandbox | Restricted Windows process plus workspace policy |
| Strong sandbox | Restricted token, Windows Job Object, ACL-scoped workspace, process-tree supervision, resource quotas, and disposable emulator snapshot |
| Android runtime testing | Disposable Nirman-managed local Android emulator snapshot |
| Preview revision tracking | Checkpoint ID plus project revision hash |
| Secrets | OS keychain reference only |

---

## 15. Architecture Completion Criteria

The architecture implementation is ready for the next product layer when it can start a task, persist its state, launch a worker in a declared workspace, stream events, request approval, create a checkpoint, survive the UI closing, detect a worker failure, recover or escalate, reconcile an isolated change, run required checks, and show a final evidence-backed result.

A system that can generate code but cannot reconstruct what happened after a crash should not be considered autonomous-ready.

---

## 16. Goal Mode and Long-Horizon Execution

### 16.1 Goal contract

Goal Mode should be represented by a durable `GoalContract` attached to a task:

> **Schema projection:** `GoalContract` is defined in `nirman-schemas.md` §2.10. Owner: TA §16.1.

`resourceRequirements` declares the physical resources the goal needs and is evaluated by runtime resource integrity (BS §72); AI usage is telemetry and no field of the GoalContract carries an AI-usage budget.

Completion conditions should be evaluable by the validation engine, not only by the model. Examples include a successful build, a test expression returning success, a route responding without runtime errors, a screenshot meeting a visual threshold, or an artifact existing with a recorded checksum.

### 16.2 Goal evaluation loop

```text
Load goal contract
    ↓
Load current task and checkpoint state
    ↓
Run next planned action or worker handoff
    ↓
Run validation plan
    ↓
Evaluate completion conditions
    ├── All pass → complete
    ├── Some fail → plan next strategy
    ├── Physical resource unavailable → wait, reschedule, reduce concurrency, reclaim resources, checkpoint, or recover
    ├── Hard gate reached → record the decision on the affected requirement; independent requirements continue
    └── Repeated failure → backtrack or escalate
```

The goal evaluator must record each condition result and should not rely on a final model statement. A task may continue after a worker reports completion if objective validation is still incomplete.

### 16.2.1 The Autonomous-build policy and approval precedence

Nirman defines approval behavior through one explicit policy rather than through isolated UI wording or a selectable profile (build spec §23.3; ADR-226). The policy is authoritative for routine approval behavior, while safety and authority gates remain mandatory.

| Policy | Routine policy-allowed actions | Hard-gated actions |
|---|---|---|
| Autonomous-build (the only policy) | Automatically executes routine reversible actions inside the approved workspace, including local dependency installation, formatting, tests, builds, preview restarts, checkpoints, and authorized environment repair. | Protected paths, credentials, signing, destructive actions, external-emulator access, publishing, and other declared hard gates; a reached gate records a decision on the affected requirement and never pauses the goal (build spec §29.4). |

Routine approval prompts must not be required merely because the UI is disconnected or a task is running in the background. Every approval request is bound to the exact action fingerprint, policy, worker, workspace, and risk. User approval authorizes only the requested policy-bound action; it never promotes a preview or artifact without deterministic evidence.

### 16.2.2 Profile terminology namespaces

Nirman uses multiple profile concepts. Each has an explicit namespace, ID prefix, and canonical owner to prevent field collision:

| Concept | Namespace | ID prefix | Canonical owner | Purpose |
|---|---|---|---|---|
| Execution profile | `profile.execution` | `exec-profile` | PolicyAuthority | Approval behavior for routine actions; exactly one instance exists, the Autonomous-build policy (§16.2.1) |
| Sandbox profile | `profile.sandbox` | `sandbox-profile` | Sandbox/workspace authority | Process isolation and resource limits |
| Capability profile | `profile.capability` | `capability-profile` | EvidenceAuthority | Android technology composition identity |
| Device profile | `profile.device` | `device-profile` | DeviceAuthority | Android device/ emulator test matrix |
| Packaging profile | `profile.packaging` | `packaging-profile` | ArtifactAuthority | APK output configuration; optional AAB only when packaging profile requires it |
| Provider profile | `profile.provider` | `provider-profile` | ProviderOperationality | AI provider configuration and capabilities |
| Environment record | `record.environment` | `env-record` | EnvironmentCapabilityPlanner | Host/target capability classification |

Profiles are not interchangeable. A capability profile describes what technologies are available; an execution profile describes what actions are permitted; a sandbox profile describes how processes are isolated. Each profile type has its own schema, lifecycle, and authority.

### 16.3 Non-blocking background control

The control plane should manage background tasks independently from the UI event loop. The UI subscribes to task events and may disconnect and reconnect using a task ID and event sequence number.

When the user opens another project, the current task remains owned by the control plane. The scheduler must enforce per-project and global resource limits and must prevent background processes from taking keyboard or mouse focus.

### 16.4 Scheduling subsystem

A schedule record should contain:

> **Schema projection:** `Schedule` is defined in `nirman-schemas.md` §2.11. Owner: TA §16.4.

The scheduler should calculate the next run transactionally, create a new task from the goal definition, and prevent duplicate runs after a control-plane restart. A scheduled task must inherit the project’s permission policy and may not upgrade its own autonomy.

## 17. Lifecycle Hook Dispatcher

The hook dispatcher should subscribe to typed control-plane events and execute configured hook handlers in a deterministic order. Hooks should be classified as blocking or non-blocking.

| Hook group | Example event | Typical use |
|---|---|---|
| Session | `session_started` | Load project context or validate provider |
| Agent loop | `before_tool` | Policy inspection or argument redaction |
| Agent loop | `after_tool` | Update index or summarize output |
| Permissions | `approval_requested` | Notify user or record audit entry |
| Worker | `worker_failed` | Requeue, escalate, or start a debugger |
| Workspace | `checkpoint_restored` | Invalidate stale preview |
| Context | `context_capacity_reached` | Compact or switch retrieval strategy |
| Runtime | `process_failed` | Capture diagnostic and classify error |
| Configuration | `external_tool_connected` | Register tool capabilities and policies |

Blocking hooks must complete before the associated action proceeds. They require a timeout, retry policy, and failure behavior. Non-blocking hooks run asynchronously and cannot mutate the result of the completed action. Hooks must be idempotent or include a deduplication key.

## 18. Two-Tier Checkpoint and Backtracking Architecture

The checkpoint manager should maintain file-level snapshots and task-level revisions. Both tiers are stored as the canonical `Checkpoint` record of build spec §11.5 (`tier: FILE | TASK`); `FileCheckpoint` and `TaskCheckpoint` below are the per-tier projections of that record that the checkpoint manager exposes, not separate schemas, and every field they show maps onto a `Checkpoint` field (`parentRevision` and `projectRevision` are `revisionReference`; `previewRevision` is `previewRevisionId`; `validationSnapshot` is `validationSnapshotId`; `workerWorkspaces` and `metadataSnapshot` are resolved through `workspaceId` and `restoreReference`).

> **Schema projection:** `FileCheckpoint` is defined in `nirman-schemas.md` §2.12. Owner: TA §18.

> **Schema projection:** `TaskCheckpoint` is defined in `nirman-schemas.md` §2.13. Owner: TA §18.

The `Checkpoint.validity`, `knownGood`, and `retentionClass` fields are the durable form of the retention and backtracking rules below: "last known-good" always means the newest checkpoint with `knownGood = true` and `validity = VALID` for the task, never a checkpoint selected by recency or by a model's recommendation.

Checkpoint storage must use a retention policy for long-running sessions. Every task retains the initial source checkpoint, the last known-good checkpoint, all checkpoints referenced by an active recovery strategy, and a configurable number of recent task checkpoints. Older intermediate checkpoints should be compacted into content-addressed snapshots or pruned only when no active branch, preview, recovery attempt, or evidence record references them. Before deletion, the system must verify that a full restore path remains available.

Android tasks should use profile-based quotas for native compilation, NDK/CMake, emulator and combined build workflows. The quota manager must account for worktrees, dependency stores, Gradle caches, APK artifacts, emulator images, logs, screenshots, and checkpoints. It should prefer deduplicated content-addressed storage and cleanup of rebuildable caches before deleting checkpoints.

Backtracking should restore a known-good checkpoint before trying a materially different strategy. The recovery manager should keep a strategy history:

> **Schema projection:** `RecoveryAttempt` is defined in `nirman-schemas.md` §2.14. Owner: TA §18.

The recovery planner must reject a new attempt when it is substantially identical to a previous failed attempt. It may change the context mode, worker role, model profile, implementation approach, or test strategy before resuming.

## 19. Context Scaling Architecture

**Implements:** build spec §53 and `CONTRACT.RUNTIME.CONTEXT` (with §59); §19.1 implements build spec §23 and `CONTRACT.RUNTIME.SKILL`. The build spec sections are the authorities.

The context engine exposes an **Adaptive Context Architecture** operating across six provider-independent strategies:

| Mode | Operational Scope | Pipeline |
|---|---|---|
| `EXACT` | Pinned symbols, active file targets, locked decisions, and mandatory constraints | Direct deterministic lookup via URI, symbol identifier, or constraint key |
| `SEMANTIC` | Structural repository neighborhood, related interfaces, callers/callees, schema dependencies | Repository Semantic Graph traversal over symbol, type, and module dependency edges |
| `TEMPORAL` | Recent action sequences, recent test outputs, recent runtime events, recent mutations | Sliding chronological window indexed by transaction and event sequence |
| `STRUCTURED_MEMORY` | Causal execution records, verified project facts, failure signatures, architectural invariants | Query against classified memory store with mandatory source event provenance |
| `LARGE_CONTEXT` | Broad architectural synthesis, multi-module refactoring, cross-cutting reviews | Context packing up to the provider's actual context capacity with prefix and structured cache alignment |
| `COMPACTED` | Long-horizon continuity, multi-session continuation, checkpoint re-grounding | Non-destructive semantic compaction preserving causal chains and invariant proofs |

Dynamic selection is governed by the twelve criteria established in BS §19.1: task phase, model context capacity, admissible context share, dependency distance, symbol relationships, temporal recency, evidence freshness, failure relevance, unresolved uncertainty, current project revision, plan revision, and context-cache availability.

The context package records included paths, excluded paths, summaries, token estimates, redactions, selection scores, and the reason for selecting each mode. If a large-context estimate exceeds the provider's actual context capacity, the orchestrator falls back to semantic/exact retrieval rather than silently truncating critical files, and records the capacity-driven omissions.

The repository map scales incrementally via the Repository Semantic Graph (§59.2). It updates changed files and affected dependency regions instead of rebuilding the entire map after every action. Large projects use sharded indexes, symbol-level summaries, dependency fingerprints, cache invalidation, and background compaction. The map manager exposes freshness, shard size, rebuild progress, and stale-region warnings to the task runtime.

### 19.1 Skill Package Registry and Invocation

The skill registry should store:

> **Schema projection:** `SkillPackage` is defined in `nirman-schemas.md` §1.12. Owner: BS §23.11.

Skill learning follows:
validated episode → skill candidate → isolated evaluation → admission
→ canary → promotion → versioned SkillPackage → future discovery/invocation.

A promoted learned skill is never granted authority by virtue of learning.
Its required tools, capabilities, permissions, worker compatibility, trust,
scan status, and environment requirements remain subject to normal admission
and policy evaluation.

A skill is selected by the orchestrator from a task requirement, explicit user request, or matching trigger condition. Loading a skill adds instructions and schemas; it never grants permissions automatically. Skill tool calls still pass through the policy engine and are logged as ordinary tool calls.

User or shared skills must be scanned for prompt injection, unsafe commands, secret access, hidden network behavior, and dependency changes before activation. Updates must be versioned, health-checked, and reversible. Built-in runtime capabilities take precedence over skills when both provide the same function, while skills may add domain-specific workflow instructions around those capabilities.

Skills should be testable through fixture tasks and should declare the minimum tools, worker roles, and project profiles they require.

Skill admission and invocation are durable ledger records (M119), both registered in §36.1:

> **Schema projection:** `SkillAdmission` is defined in `nirman-schemas.md` §2.15. Owner: TA §19.1.

> **Schema projection:** `SkillInvocationRecord` is defined in `nirman-schemas.md` §2.16. Owner: TA §19.1.

`SkillAdmission` is written before any instruction body loads or tool call runs; a skill with no `ADMITTED` admission for the current environment fingerprint cannot be invoked. `SkillInvocationRecord` pins the version admitted for the session (ADR-154), links every tool call and evidence item the invocation produced, and is invalidated (`invalidatedBy`) when the environment fingerprint, skill version, or project revision it was bound to changes.

### 19.2 Provider Attention Capabilities and Neural Architecture Adaptation

Nirman does not implement hybrid sparse/linear attention itself. The model provider owns neural attention architecture (sparse, linear, recurrent, cached, or hybrid attention). Because such architectures recall distant literal content unevenly by position, window fill, and distractor density, a boolean "supports long context" flag carries no usable information. Nirman therefore exposes provider attention behavior as a measured, per-model `AttentionReliabilityProfile` through `ProviderProfile.attentionCapabilities` (BS §53.11 is the normative authority; this is the canonical schema):

> **Schema projection:** `AttentionReliabilityProfile` is defined in `nirman-schemas.md` §1.18. Owner: BS §53.11.

> **Schema projection:** `PositionalRecallCell` is defined in `nirman-schemas.md` §2.17. Owner: TA §19.2.

`declaredContextTokens` is the physical context capacity that `ContextCapacityPlanner` fits the package to; it is the former `maxContextTokens` and keeps that role. `reliableLiteralSpanTokens` bounds only the DENSE placement block of BS §53.11. Reasoning, tool-calling, and vision support are reported by `capabilityOverrides` and `reasoningCapabilityProfile` (§24.2), not by this profile. `confidence` is derived from sample counts, never declared.

Nirman adapts around the model by measuring its recall with deterministic probes, placing precision content inside the measured reliable span adjacent to the instruction, aligning the cache breakpoint before that block, verifying proposals against anchors and premises independently of recall, and routing queries through exact/sparse/summary memory, without coupling to any proprietary model architecture.

### Symbol Reference Optimization (optional)

Workers MAY request symbol/file references through the Supervisor retrieval API to reduce context assembly cost. The resulting context remains a derived `ContextPackage` (BS §53.3) bound to the revision and evidence ledger. This optimization must not become an independent memory/index authority or bypass the existing ContextOrchestrator contract.

## 20. External Tool Protocol Adapter

Nirman’s internal Tool Gateway remains authoritative, but an adapter may expose or consume standardized external tool servers. The adapter should translate external tool calls into Nirman policy requests before execution.

> **Schema projection:** `ExternalToolConnection` is defined in `nirman-schemas.md` §2.18. Owner: TA §20.

External tools should be capability-discovered, permission-scoped, health-checked, and auditable. A tool that requests local file access must still pass through the filesystem policy. A tool that causes an external side effect must create an approval request unless the project policy explicitly allows it.

## 21. Authority Hierarchy and Recovery Invariants

**Implements:** build spec §33 and `CONTRACT.RUNTIME.AUTHORITY` (with §27; the build spec section is the authority)

The model is a proposal generator, not the authority over the runtime. Model output remains untrusted data until deterministic runtime components validate, authorize, persist, and execute it.

| Authority | Non-delegable responsibility |
|---|---|
| Lifecycle authority | Owns process and task start, pause, resume, restart, cancellation, and termination |
| Permission authority | Evaluates every filesystem, terminal, network, provider, browser, and external-tool action |
| Sandbox authority | Enforces workspace, filesystem, process, network, and resource isolation |
| Storage authority | Commits state, events, leases, checkpoints, evidence, and recovery records transactionally |
| Evidence authority | Accepts only observable command, test, build, health, visual, security, or artifact evidence |
| Recovery authority | Chooses retry, diagnosis, repair, backtracking, delegation, degradation, or safe failure |
| Promotion authority | Controls candidates, canaries, self-updates, activation, monitoring, and rollback |

Each row is realised by the named components of the §57.12 component and authority registry, which also fixes every alias of these authorities; no other document introduces an authority name.

No model response, worker summary, skill, hook, external tool, or UI event may grant itself permissions, mark a task complete, delete recovery state, bypass a sandbox, promote a binary, suppress required evidence, or disable a mandatory control. All actions pass through the Tool Gateway, policy engine, durable state store, and evidence pipeline.

For every recoverable failure, the runtime must attempt an appropriate deterministic path: retry transient work, refresh state, repair the environment, change strategy, restore a checkpoint, delegate diagnosis, degrade an optional capability, or record a safe terminal failure. Recovery must preserve the last known-good state and must not depend on a model remembering an uncommitted action.

## 22. Additional Architecture Tests

The architecture test suite must add the following cases:

1. A goal task continues across multiple worker handoffs and completes only when objective validation passes.
2. The UI disconnects while a task continues, then reconnects and receives all missing events in order.
3. A scheduled task is created exactly once after a control-plane restart at its trigger time.
4. A blocking hook prevents an unsafe command, while a non-blocking hook cannot alter a completed result.
5. A file-level restore leaves unrelated files unchanged.
6. A task-level restore invalidates a preview running from a newer revision.
7. A repeated failed strategy triggers backtracking and a materially different recovery plan.
8. A large-context request adapts retrieval and representation when the selected provider's actual context capacity is insufficient, while preserving required EXACT context and recording capacity-driven omissions.
9. An external tool cannot bypass path, network, or approval policies.
10. A skill cannot grant itself permissions or execute undeclared tools.
11. The repository map updates only affected shards after a small file change.
12. A stale repository-map region is detected and refreshed before an edit.
13. A skill with an unsafe instruction or undeclared network behavior is rejected before activation.
14. The canonical worker registry exposes the Performance Worker and every worker has a declared contract.
15. An interface agreement is required before parallel workers modify coupled frontend/backend contracts.

## 23. Execution Surface and Evidence Model

**Implements:** build spec §37 and `CONTRACT.RUNTIME.EVIDENCE`; §23.3 is the evidence-ledger implementation that `CONTRACT.RUNTIME.INVARIANTS` (build spec §67) verifies against. The build spec sections are the authorities.

### 23.1 Durable task graph

The control plane should represent each autonomous task as a durable directed graph rather than a flat progress string. The graph contains a root goal, requirement nodes, implementation phases, worker tasks, dependency edges, validation nodes, checkpoints, approvals, recovery attempts, and final evidence.

The graph record is the build spec §80.5.4 `TaskGraph`, field for field (phases hold `TaskNode`s; `TaskNode.kind` represents the goal, requirement, worker-task, validation, checkpoint, approval, recovery-attempt, and evidence elements, `dependencies` the edges); the build spec is the canonical owner:

> **Schema projection:** `TaskGraph` is defined in `nirman-schemas.md` §1.58. Owner: BS §80.5.4.

A node may be `pending`, `ready`, `running`, `waiting_approval`, `waiting_resource`, `completed`, `failed`, `blocked`, `skipped`, or `cancelled` (`TaskNode.status`, build spec §80.5.4). A node can be marked `completed` only after its evidence requirements pass. Model summaries may explain a node, but they cannot complete it without an execution or review evidence record.

### 23.2 Nested execution tree

The UI-facing execution tree should be derived from the task graph and event ledger. It should support expandable nodes for:

```text
Goal
  ├── Requirement extraction
  ├── Planning
  ├── Implementation workstream
  │     ├── Worker handoff
  │     ├── File changes
  │     └── Commands
  ├── Validation workstream
  │     ├── Preview
  │     ├── Tests
  │     ├── Build
  │     ├── Security checks
  │     └── Visual or device QA
  ├── Recovery and backtracking
  └── Reconciliation and final evidence
```

Each displayed node should reference a durable node ID, parent ID, owner, workspace, start and end timestamps, current action, heartbeat, resource snapshot, evidence IDs, warnings, and failure fingerprint. The UI should load children lazily so large tasks remain responsive.

### 23.3 Evidence ledger

The evidence ledger records the facts that justify task status. An evidence record should contain:

> **Schema projection:** `EvidenceRecord` is defined in `nirman-schemas.md` §2.19. Owner: TA §23.3.

The first twelve fields describe the observation; the remaining sixteen are the identity and dependency fields that build spec §5.7.4 requires of every evidence node (source event, operation, session, project revision, checkpoint, artifact or preview identity, device and toolchain identity, validation-policy version, freshness interval, dependency ids, supersession, invalidation reason). `artifactId`, `previewRevisionId`, `deviceIdentity`, and `toolchainLockId` are null only when the evidence type has no such subject; a build, install, device, or preview observation without them is rejected at capture. `dependencyIds` are `EvidenceDependency` ids (§36.4); `supersedes`/`supersededBy` implement the immutability rule below, and `invalidationReason` is written only by the evidence authority when a §36.4 dependency is invalidated. An `EvidenceRecord` lacking these fields cannot participate in the `Observation → EvidenceArtifact → ValidationResult → CertificationDecision → CompletionDecision` chain.

Evidence types should include command results, test reports, build artifacts, screenshots, device results, security scans, dependency scans, review findings, user approvals, and environment diagnostics. Evidence must be immutable after capture; corrections create a new record linked to the old one.

### 23.4 Runtime telemetry

The control plane should publish task and worker telemetry as structured events. Telemetry should include elapsed time, model turns, provider requests, retries, token estimates or reported usage, estimated cost where available, CPU, memory, disk, process count, active workers, heartbeats, current action, last validated checkpoint, blocker, and next action.

Telemetry is observational and must not itself mark a task complete. It should be retained at a configurable sampling interval and summarized for long tasks to control database growth.

### 23.5 Autonomous validation pipeline

The validation coordinator should execute the applicable stages in a dependency-aware sequence:

```text
Preview or launch
    ↓
Focused checks and tests
    ↓
Build or package
    ↓
Security, dependency, and reliability checks
    ↓
Device, accessibility, and visual QA
    ↓
Failure classification
    ↓
Repair or checkpoint backtracking
    ↓
Affected tests and regression tests
    ↓
Goal evaluation
```

Not every project requires every stage. The project profile should declare required, optional, and unavailable stages. An unavailable required stage blocks completion rather than being silently treated as passed.

The validation coordinator must compute affected tests from the changed-file set, symbol/dependency graph, route ownership, fixtures, configuration changes, and recent failure fingerprints. It should run the smallest affected test set first, reuse valid cached results, shard independent regression suites, and periodically run the complete regression suite. A test result is reusable only when the source revision, environment fingerprint, dependency lock, test configuration, and relevant inputs match.

The validation pipeline must include architectural-drift checks for duplicate component or module names, circular imports, unreachable routes, dead exports, orphaned files, inconsistent interface definitions, stale generated artifacts, dependency divergence, and undocumented public-surface changes. These checks complement linting, type checking, and tests rather than assuming those tools detect architectural drift.

### 23.6 Policy-boundary approvals

The policy engine should classify actions into ordinary approved work, reviewable work, and privileged work. Ordinary reversible actions inside an approved workspace may proceed without repeated prompts. Reviewable and privileged actions create approval requests only when the action is reached.

An approval request must include the exact action, worker, workspace, path or destination, policy rule, risk explanation, requested data, predicted side effect, and choices. Approval must be bound to the request fingerprint and must expire when the action, task state, or policy changes (context expiry) or when the build spec §80.3 approval-expiry clock elapses, whichever comes first (build spec §26.13).

### 23.7 Termination coordinator

The termination coordinator evaluates whether a task may continue after every validation, recovery, resource-integrity, approval, and environment event. It must recognize these terminal classifications:

| Classification | Condition |
|---|---|
| Completed | All required completion conditions pass with evidence |
| Completed with warnings | Required conditions pass and only non-blocking warnings remain |
| Blocked | A required dependency, environment, permission, or decision is missing |
| Escalated | Recovery strategies or autonomous authority are exhausted |
| Cancelled | User or policy cancellation was requested |
| Failed | No safe recoverable path remains |

The coordinator must stop only when a required hard safety or policy limit is reached, a dangerous or unresponsive process must be terminated, the user cancels, the environment/provider is unavailable, or no safe recovery path remains. Ordinary resource thresholds must cause adaptation rather than automatic termination. It must not interpret an active model response, recent tool call, or optimistic worker summary as proof that continuing is safe. A task may continue when its next action is allowed, attached to a live goal contract, and protected by adaptive resource management.

### 23.8 Architecture tests for the execution surface

The test suite must verify that a task graph survives UI disconnection, child nodes replay in order, completed nodes require evidence, telemetry reflects worker heartbeats and resource use, unavailable required validation blocks completion, policy-boundary approvals do not interrupt ordinary safe work, and the termination coordinator stops at configured limits.

## 24. Provider Runtime and AI Settings Contract

### 24.1 Provider-neutral design

Nirman should not couple the agent runtime to one provider SDK. It should expose one internal `ModelGateway` contract and implement protocol adapters behind it. The gateway normalizes request construction, streaming, tool calls, structured output, vision input, cancellation, errors, usage, request IDs, and retry behavior.

The first implementation should support three request surfaces:

| Surface | Canonical input | Canonical output | Intended use |
|---|---|---|---|
| Chat-completion surface | Ordered role/content messages | Choice, assistant message, tool calls, finish reason, usage | Broad compatibility with OpenAI-style endpoints |
| Response-item surface | Input items and multimodal content parts | Output items, messages, function calls, tool outputs, status | Stateful or multimodal providers with richer item types |
| Generic message surface | System/developer/user/assistant/tool messages | Message content, tool calls, stop reason, usage | Providers that expose a message-oriented API |

The adapters must preserve provider-specific data in a raw-response envelope while also producing a normalized internal response. Nirman should never discard tool-call IDs, refusal information, reasoning metadata when available, streaming event types, finish reasons, request IDs, or provider error details.

The official API reference distinguishes a response-oriented surface for direct model requests, tool use, multimodal inputs, and stateful interactions from a chat-completion surface based on conversation messages.[5] Nirman should support both without assuming that every configured endpoint supports the same capabilities.

### 24.2 Provider profile

The AI Settings page should store a provider profile with the following shape (the build spec §80.5.5 record, field for field; the build spec is the canonical owner and this section adds no field):

> **Schema projection:** `ProviderProfile` is defined in `nirman-schemas.md` §1.62. Owner: BS §80.5.5.

The API key and sensitive headers must be stored only through the operating-system keychain. The profile may display a masked key fingerprint and last validation time, but never the raw key.

The `reasoningCapabilityProfile` field holds the provider's discovered reasoning capability:

> **Schema projection:** `ReasoningCapabilityProfile` is defined in `nirman-schemas.md` §1.63. Owner: BS §80.5.5.

`effortParameterMapping` is configuration metadata, not authority: it records how normalized effort levels translate into provider-specific parameters, but it can never alter granted effort, resource-integrity decisions, permission ceilings, or authority state.

`maxReasoningTokens` is provider capability metadata: the largest per-request reasoning allocation the provider accepts (a request's `ReasoningSettings.maxReasoningTokensOptional` is bounded by it). It is never a Nirman execution budget, authorization ceiling, or termination condition; it bounds what ModelGateway may request from the provider, and reasoning usage reported against it is telemetry only (BS §72).

`ProviderProfile` represents an external network-reachable AI provider only. It cannot represent a local auxiliary decision engine.

Supervisor-local auxiliary decision engines use `LocalDecisionEngineProfile` (§49.5; SCHEMAS §2.129) and do not have provider URLs, provider credentials, provider compatibility modes, or provider network state.

### 24.3 AI Settings page behavior

The settings interface should allow the user to create, duplicate, test, disable, and delete provider profiles. It should support custom base URLs and model IDs. Save is disabled until a connection Test against the configured endpoint and model returns a successful validated response per ADR-208. Any edit to key, base URL, model ID, or compatibility mode invalidates a prior pass and re-disables Save.

The connection test should discover or validate the configured endpoint, verify authentication, test the selected model, detect available features, measure a basic response, and record the provider request ID. Model discovery through a models endpoint is optional; a user must be able to enter a model ID manually when discovery is unavailable.

The page should show capability badges for text, vision, file input, tool calls, structured output, streaming, cancellation, background requests, embeddings, reasoning, supported reasoning effort levels, reasoning usage reporting, context capacity, and attention reliability. A capability badge must be based on a successful probe or explicit user override, not a provider name alone. The attention reliability badge shows the profile `source` and `reliableLiteralSpanTokens`; the connection test runs the recall probe fixture and saves the profile as `UNPROFILED` when the probe cannot complete, never as a declared value.

Reasoning capability must be displayed separately from general text generation. A model that can generate text but does not expose or support provider-native reasoning must not be presented as supporting the configured deep-reasoning capability.

### 24.4 Canonical request and response model

The model gateway should convert all supported protocols into a canonical internal request:

> **Schema projection:** `ModelRequest` is defined in `nirman-schemas.md` §2.20. Owner: TA §24.4.

`ModelRequest` is the transient provider-neutral runtime request representation. It is not the durable provider-request provenance record. Durable, non-sensitive lineage is recorded as `ProviderRequestProvenance` in the supervisor-owned SQLite execution ledger.

> **Schema projection:** `ProviderRequestProvenance` is defined in `nirman-schemas.md` §2.130. Owner: TA §24.4.

`ProviderRequestProvenance` records one logical request across one or more provider attempts. It binds the request to task, worker, session, conversation, correlation, causation, prompt class and contract, an actual runtime instruction artifact when one exists, `ContextPackage`, provider profile, model, adapter, normalized request identity, privacy/redaction policy, retention, invalidation dependencies, responses, proposals, and validation evidence. It stores identities, hashes, revisions and references rather than raw credentials, excluded content, provider-private reasoning, or an unapproved raw prompt archive.

Prompt class and template identity are separate. Worker, skill, deliberation, and review prompts whose literal templates remain owner-pending in BS §80.8 record their class and governing prompt contract but no fabricated template identity. Template identity/version/hash are populated only for a real canonical template.

Hash domains are distinct: `ContextPackage.integrityHash` identifies context; `runtimeInstructionArtifactHash` identifies the assembled internal instruction; `providerNeutralRequestHash` identifies the canonical provider-independent request after approved redaction and normalization; `retryEquivalenceFingerprint` identifies the same logical intended request across attempts. Provider-specific payload and transport hashes are optional adapter diagnostics and never replace the provider-neutral identity. Cache identity and `ExternalEffectRecord.idempotencyKey` retain their existing owners and semantics.

> **Schema projection:** `ReasoningSettings` is defined in `nirman-schemas.md` §2.21. Owner: TA §24.4.

`providerNativeParameters` may contain provider-specific reasoning controls, but those values are generated by ModelGateway from the normalized runtime request. A model response or provider cannot modify the runtime's granted effort, permission ceiling, or authority state.

`effortGrantId` must refer to the runtime-owned effort grant (§72.5) under which the request is issued. It carries the granted effort level and the provider capability constraint; it is not a usage reservation, and reasoning usage reported against it is telemetry only.

The canonical response should be event-oriented:

> **Schema projection:** `ModelEvent` is defined in `nirman-schemas.md` §2.22. Owner: TA §24.4.

The orchestrator consumes the canonical events and writes them to the task event ledger. A provider’s streaming format must never be rendered directly by the UI as the source of truth.

### 24.5 Tool-call normalization

Chat-completion tool calls, response-item function calls, and message-oriented tool calls should normalize to:

> **Schema projection:** `ToolCallRequest` is defined in `nirman-schemas.md` §2.23. Owner: TA §24.5.

Nirman validates arguments against the registered tool schema, sends the request through the policy engine, executes the tool only when allowed, and returns a normalized `ToolCallResult` with status, output reference, redactions, duration, and evidence ID. Tool results must be associated with the original call ID so the next model request can be serialized correctly for the selected protocol.

### 24.6 Streaming, cancellation, retry, and long-running requests

The provider runtime should support streaming when available and should emit partial content and tool-call deltas into the durable event stream. If a provider does not support streaming, the control plane should still emit request-started, request-completed, and usage events.

Cancellation should be cooperative first, then terminate the local request process if the provider client does not stop. A cancelled request must not be treated as a task failure unless the task cannot continue safely.

Provider retries should classify authentication errors, invalid requests, rate limits, transient network failures, provider overload, context overflow, unsupported capabilities, and content-policy responses separately. A retry must preserve the request correlation ID and must not duplicate a tool execution. When a request is too large, the context planner should compact or retrieve less context instead of silently dropping required instructions.

The provider may offer its own background-request mode, but Nirman’s local control plane remains the authoritative owner of task persistence and resumption. Provider-side background execution is an optimization, not a substitute for local recovery.

> **Schema projection:** `ProviderRequestAttempt` is defined in `nirman-schemas.md` §2.131. Owner: TA §24.6.

Each logical request has one stable `logicalRequestId` and one or more `ProviderRequestAttempt` records. A retry creates a distinct `attemptId` while preserving logical request identity and correlation. Cancellation, timeout, provider failure, and unknown outcome affect the attempt, not the logical request. Every externally issued attempt references its `ExternalEffectRecord`; that record remains authoritative for `KNOWN_SUCCESS`, `KNOWN_FAILURE`, `UNKNOWN`, `RECONCILING`, and `RESOLVED`. `ProviderRequestAttempt.modelRequestId` references `ModelRequest.requestId`, which is also carried by `ModelEvent.requestId`; `providerRequestId` is the provider-issued identifier and is reused by `UsageRecord.providerRequestId` when the provider supplies it. `ModelEvent` remains authoritative for stream/outcome events and `UsageRecord` for usage attribution. A provider background operation is an attempt under the same logical request, never a second local persistence authority. Cancellation before issuance leaves the attempt `PREPARED` and creates no external effect. Cancellation after issuance records the terminal `ModelEvent`, updates the attempt, and reconciles its `ExternalEffectRecord`; transport cancellation alone is never proof that the provider did no work. Timeout, disconnect, bridge crash, or missing terminal event transitions the attempt to `UNKNOWN` and blocks retry until reconciliation resolves whether a response or provider-side operation exists. Failover creates a new attempt with its own provider/model identity and `ExternalEffectRecord` under the same logical request only when retry-equivalence still holds; any material request change creates a new logical request. Restart reconstructs pending attempts from provenance, events, provider/background IDs, and external-effect state before issuing new network work. Fault tolerance and recovery across the request lifecycle are verified by the 10-point provider crash-order fixture matrix (defined in milestones M22, M26, and M40), covering crashes: (1) before issuance, (2) immediately after issuance, (3) after provider acknowledgement, (4) during streaming, (5) after terminal response before ModelEvent persistence, (6) after ModelEvent before UsageRecord, (7) after usage before reconciliation, (8) during cancellation, (9) during failover, and (10) while deletion is requested. In all crash scenarios, durable reconciliation ensures no orphan records, duplicate tool execution, or inconsistent transaction state.

### 24.7 Context and token policy

Nirman should record token usage when the provider reports it; token usage is telemetry with no execution-authority semantics and never a completion lock. Context is fit to the provider's actual context capacity by compaction and retrieval; concurrency reduction responds to physical resource pressure; continuation is governed by progress and evidence. A user MAY declare an explicit policy stop condition, which is a user decision evaluated by policy authority, not a runtime budget.

A provider profile may declare a context capacity or allow Nirman to learn it from probe results. Attention reliability is always learned: a declared `AttentionReliabilityProfile` is `DECLARED` metadata until the recall probe fixture of §24.8 replaces it with a `PROBED` result, and premise mismatches from the mutation broker refine it as `LEARNED` (BS §53.11). If the selected request exceeds a provider’s actual hard context limit, the gateway must return a structured context-overflow result so the context planner can reduce or reassemble the request.

### 24.8 Provider compatibility tests

The provider test suite should use protocol fixtures for:

1. Simple text completion.
2. Multi-turn role/content messages.
3. Multimodal text and image input.
4. Structured JSON output.
5. One or more tool calls and tool results.
6. Streaming text and tool-call deltas.
7. Cancellation during generation.
8. Rate-limit and transient network recovery.
9. Context overflow and reassembly.
10. Refusal or incomplete output handling.
11. Request-ID and usage capture.
12. Provider capability mismatch and fallback.
13. Positional literal recall across fill buckets (writes the `AttentionReliabilityProfile`).
14. Post-compaction constraint retention via re-projection and recall probe.

A provider adapter is not production-ready until it passes the fixtures relevant to its declared capabilities. Fixtures 13 and 14 never fail a profile save; they determine the saved profile's `source` and the placement bounds applied to it.

## 25. Optimized Self-Development Loop

### 25.1 Scope and invariants

Self-Development Mode allows Nirman to improve its own source code, tests, documentation, skills, and runtime configuration. It must operate in a separate self-development worktree and must never directly mutate the files of the currently running application.

The self-development loop must preserve these invariants:

| Invariant | Requirement |
|---|---|
| Running instance safety | The currently running Nirman process remains unchanged until promotion |
| Control-plane survival | The updater/controller can survive an application UI crash or restart |
| Reversibility | Every self-change has a parent checkpoint and rollback artifact |
| Test evidence | Promotion requires recorded static, unit, integration, smoke, and health evidence |
| Version integrity | The candidate build has a version, manifest, checksums, and compatibility metadata |
| Permission boundaries | Self-development cannot grant itself new permissions or bypass the policy engine |
| User ownership | Promotion to the active installation is explicit unless the user enables trusted auto-promotion |
| Task continuity | Active user tasks are checkpointed and either resumed or safely paused across promotion |

Self-development promotion is not a product update. The machinery of this section — the update-controller bootstrap stage, the update lock and active-version pointer, candidate staging, switching, and rollback — exists solely to activate validated self-development and self-improvement candidates (§30) inside a running Nirman that is developing itself; it never downloads, stages, installs, or replaces a distributed Nirman release, and user-enabled trusted auto-promotion never applies to the packaged product. Ordinary product updates are owned by ADR-239 — a user-initiated Check for updates from the Settings screen through the Windows App Installer MSIX flow, never automatic. The two mechanisms are disjoint: neither may be used to implement the other.

### 25.2 Two-process update architecture

Nirman should use a stable launcher/controller process and a replaceable application process. The update protocol must handle UI-supervisor version compatibility explicitly:

```text
Download → Verify → Stage → Compatibility-check → Quiesce → Switch → Restart → Health-check → Rollback
```

Required rules:
- UI updated, supervisor old: the new UI must detect the old supervisor version and either upgrade the supervisor or operate in a compatible degraded mode.
- Supervisor updated, UI old: the new supervisor must support the old UI protocol version or trigger a UI update.
- Interrupted installer: the previous version must remain runnable. The installer must use atomic file operations and validate checksums before switching.
- Failed supervisor startup after update: automatic rollback to the previous known-good version with evidence-backed task event.
- Database migration failure: the controller must never delete the only recoverable database copy. Forward migration and rollback/backup strategy required.
- Rollback compatibility: the previous version must be able to read the database schema written by the new version, or a backup must be restored.
- Evidence invalidation after controller changes: any controller update invalidates dependent evidence unless independence is proven.

```text
Stable launcher/controller
    ├── Starts current Nirman version
    ├── Owns update lock and active-version pointer
    ├── Monitors health and process liveness
    ├── Launches candidate versions in isolation
    ├── Promotes or rolls back versions atomically
    └── Restores interrupted task state

Replaceable Nirman application
    ├── Desktop UI
    ├── Control-plane client
    └── Current application features
```

The controller should not be replaced during an ordinary self-update. This gives the system a stable recovery path if the candidate application fails to start, crashes during migration, or cannot connect to the local database.

The stable launcher/controller is not a separate executable. It is the `UpdateController` bootstrap stage of `NirmanSupervisor.exe` (§57.4): the supervisor binary that Windows starts at user login owns the update lock and the active-version pointer, and the "replaceable Nirman application" is the versioned application directory it launches — `Nirman.exe` together with the supervisor's own control-plane modules loaded from that directory. The bootstrap stage is versioned and shipped separately from the application directory, is replaced only by an explicit controller-update path that ADR-039 and §25.3 escalate to a higher review level, and never loads the candidate's code before the candidate has passed compatibility checks. This keeps ADR-002A's one-product contract intact: there is no launcher the user sees, and no executable beyond the three of §3.5 — `Nirman.exe`, `NirmanSupervisor.exe`, and the supervisor-spawned `NirmanWorker.exe`, none of which is a launcher. `Nirman.exe` never performs promotion or rollback; it only shows the §25.7 status projection.

### 25.3 Self-development task contract

A self-development task should include:

> **Schema projection:** `SelfDevContract` is defined in `nirman-schemas.md` §2.24. Owner: TA §25.3.

The task must begin with a source checkpoint and an isolated worktree. Changes to the updater/controller, security policy, database migrations, provider runtime, or permission engine should automatically require a higher review level and a broader test plan.

### 25.4 Self-development state machine

```text
REQUESTED
    ↓
SOURCE_CHECKPOINTED
    ↓
ISOLATED_WORKTREE_CREATED
    ↓
REQUIREMENTS_AND_PLAN_READY
    ↓
IMPLEMENTING
    ↓
STATIC_ANALYSIS
    ↓
UNIT_AND_INTEGRATION_TESTS
    ↓
CANDIDATE_BUILD
    ↓
CANDIDATE_LAUNCHED
    ↓
HEALTH_AND_SMOKE_CHECKS
    ↓
TASK_REPLAY_AND_COMPATIBILITY_CHECKS
    ↓
PROMOTION_REVIEW
    ├── PROMOTED
    ├── REJECTED
    └── ROLLED_BACK
```

Self-development work — carried by the ordinary roles of §6.5 inside the candidate worktree, not by a distinct worker role — may continue autonomously through implementation, tests, rebuilds, candidate launches, and repair cycles. It must stop for promotion only when the promotion policy requires approval or when a hard safety, compatibility, migration, or health condition fails.

### 25.5 Candidate validation

A candidate build should be validated in layers:

1. Static formatting, type, lint, dependency, and secret checks.
2. Unit and integration tests for changed modules.
3. Control-plane recovery tests and database migration tests.
4. Provider adapter compatibility fixtures.
5. Sandbox and permission regression tests.
6. A clean candidate launch using a temporary profile and port.
7. Health checks for IPC, database, provider settings, task creation, preview, and event replay.
8. A smoke task that creates a fixture project, edits one file, runs checks, creates a checkpoint, and restores it.
9. Replay of representative end-to-end task fixtures.
10. Compatibility verification for active tasks and stored database schema.

A candidate must not be promoted merely because it compiles. The candidate must produce a machine-readable validation report with passed, failed, skipped, unavailable, and unverified checks.

### 25.6 Atomic promotion and rollback

The controller should keep at least the current version and the previous known-good version. Promotion should write a new version directory, validate its manifest and checksums, acquire an update lock, switch the active-version pointer atomically, start the candidate, and wait for health checks.

If the candidate fails to start, loses IPC, cannot open the database, fails health checks, crashes repeatedly, or causes task recovery failure, the controller should switch back to the previous version and restore the last validated task state. The rollback result must be recorded as an evidence-backed task event.

Database migrations require special handling. A self-update that changes the schema must provide a tested forward migration and a compatible rollback or backup strategy. The controller must never delete the only recoverable database copy.

### 25.7 Reload behavior

During development, Nirman may hot-reload UI assets or replace a worker module in a disposable process. Production self-updates should use a controlled candidate restart rather than mutating loaded binaries in place.

The user interface should show the candidate version, current version, validation progress, promotion status, and rollback status. A successful promotion should reopen the previous task tree and continue from its last validated checkpoint.

### 25.8 Self-development safety tests

The self-development test suite should deliberately exercise malformed builds, failed migrations, missing provider adapters, broken IPC, corrupted manifests, failed health checks, repeated crashes, interrupted promotions, locked files, disk exhaustion, and rollback during an active task. Every failure must leave the previous version runnable and the project/task state recoverable.

## 26. Architecture Completion Criteria for Provider and Self-Development Support

The architecture is ready for implementation of the advanced loop when it can configure and test a custom provider profile, execute a text request through its declared protocol, stream or emulate events, normalize a tool call, record usage and request IDs, and handle cancellation or context overflow. It must also be able to run a self-development task in an isolated worktree, build a candidate, launch it under a temporary profile, run health and smoke checks, promote it through the stable controller, and automatically roll back after an injected failure.

## 27. Complete Runtime Control Plane

**Implements:** build spec §33 and `CONTRACT.RUNTIME.AUTHORITY` (with §21; the build spec section is the authority)

### 27.1 Runtime responsibilities

The Nirman runtime is the product’s autonomous core. It must own the entire development loop rather than acting as a thin wrapper around model requests.

```text
Stable Supervisor
    ↓
Control Plane
    ├── Goal and requirement manager
    ├── Task-graph scheduler
    ├── Worker registry and lease manager
    ├── Policy and approval engine
    ├── Model Gateway
    ├── Tool Gateway
    ├── Workspace and checkpoint manager
    ├── Validation and evidence engine
    ├── Recovery and backtracking manager
    ├── Memory and context manager
    ├── Artifact and version manager
    └── Self-improvement manager
```

The desktop UI is a client of the control plane. The control plane is the authoritative owner of task state, worker leases, checkpoints, evidence, approvals, provider requests, and recovery. No model response, UI event, or worker-local file should be treated as authoritative state without being committed through the control plane.

### 27.2 Autonomous execution cycle

Every Goal Mode task should run through a durable execution cycle:

```text
Receive goal
    ↓
Normalize requirements and assumptions
    ↓
Create acceptance conditions and validation plan
    ↓
Inspect project, environment, and existing task memory
    ↓
Construct task graph and reserve initial resources
    ↓
Select workers, models, tools, and workspaces
    ↓
Execute the next highest-value ready node
    ↓
Capture events, outputs, checkpoints, and evidence
    ↓
Evaluate progress and completion conditions
    ├── Complete → finalize and report
    ├── More work → schedule next node
    ├── Failure → enter recovery ladder
    ├── Waiting policy → request only the required decision
    └── Unsafe/unrecoverable → stop safely and preserve state
```

The runtime must continue after each provider response. A provider response is one step in the loop, not the end of the task. The loop should be driven by the task graph and validation state, allowing thousands of requests and worker handoffs without losing the original goal or acceptance criteria.

### 27.3 Runtime tick

The scheduler should execute idempotent runtime ticks. A tick reads the current task snapshot, receives new events, reconciles worker heartbeats, evaluates dependencies, checks policies, selects ready work, and commits the next transition.

```text
begin tick transaction
  load task revision
  consume unprocessed events
  reconcile worker leases
  update node states
  evaluate approvals and policies
  evaluate resource health
  select next runnable nodes
  reserve workspace and provider capacity
  persist worker launch intents
commit tick transaction
launch external processes
```

Launch intents prevent duplicate workers when the supervisor restarts between database commit and process creation. Every worker must receive a unique lease and execution attempt ID.

### 27.4 Worker leases and heartbeats

A worker lease should contain worker ID, task ID, node ID, workspace, process ID, attempt ID, lease start, lease expiry, heartbeat sequence, resource snapshot, and cancellation state. Heartbeats should be persisted independently from model output.

> **Schema projection:** `WorkerLease` is defined in `nirman-schemas.md` §2.116. Owner: TA §27.4.

When a lease expires, the supervisor should inspect process liveness, preserve the worker workspace, record an interruption, and choose among resume, requeue, recovery, or escalation. A `WorkerAssignment` is assigned when the scheduler creates it and active when the worker claims it and starts on its lease. Completed and failed are terminal and immutable: requeue and recovery mint a new assignment with the next `attemptId` and never mutate the old row. Released marks a lease returned without worker terminality — cancellation, scale-down, or pre-start revocation — and resume keeps the assignment active on its lease. A lease grant MUST be durably recorded before the worker process launches; a grant without a durable record is void on recovery scan (§57.4). A replacement lease's initial `ContextPackage` MUST seed from the predecessor attempt: it MUST reference the predecessor `attemptId` and include the latest available `WorkerHandoff` or progress summary, the recorded failure fingerprint, the rejected strategies, and the currently open `REQUIRED_VALIDATION` items. `RegroundingService` assembles this seed from durable ledger state before the replacement worker receives actionable context. A worker must never keep a task permanently claimed after a crash.

## 28. Recovery Ladder and Problem-Solving Depth

### 28.1 Recovery levels

Nirman should use a graduated recovery ladder. It should not jump immediately to a new model or ask the user for help.

This table is the **canonical recovery ladder** and the single owner of recovery level numbers, and build spec §80.4.1 fixes the applicability precondition for each level without restating an ordering; every "recovery level N" reference in any document, including ADR-226 rule 4, build spec §27.10, and build spec §29.4, means a level of this table. No document may introduce a second recovery ordering (AGENTS.md: one canonical lifecycle).

| Level | Recovery action | Continue automatically? |
|---|---|---|
| 0 | Re-run a transient network or process operation once with deduplication | Yes |
| 1 | Re-read focused diagnostics and retry a minimal repair | Yes |
| 2 | Refresh repository context, project index, or environment diagnostics | Yes |
| 3 | Change implementation strategy or use a different worker role | Yes |
| 4 | Restore a known-good checkpoint and try an alternative design | Yes |
| 5 | Route to a stronger or more suitable configured model | Yes, if permitted |
| 6 | Delegate to a diagnostic, security, or architecture reviewer | Yes |
| 7 | Create an isolated alternative branch and compare solutions | Yes |
| 8 | Ask for a decision only when the requirement, permission, or external fact is genuinely missing | No, decision required |
| 9 | Preserve state and escalate when no safe recovery path remains | No |

Every recovery attempt must state what new evidence or strategy differentiates it from the previous attempt. Repeating the same command, prompt, patch, or model route does not count as a new recovery strategy.

A materially different recovery strategy MUST differ in at least one
authoritative strategy dimension: recovery level, targeted evidence,
implementation approach, worker role, model profile, context/retrieval mode,
restored checkpoint, validation method, or environment condition. Cosmetic
changes to wording, formatting, equivalent patches, or reordered equivalent
actions do not constitute a new strategy. The runtime MUST record the strategy
fingerprint used for equivalence and MUST reject an attempt classified as
materially equivalent to a previously exhausted strategy.

### 28.2 Failure fingerprints

The runtime should fingerprint failures using normalized command, exit code, error class, stack-trace structure, changed-file set, environment state, provider response class, and validation stage. Fingerprints should be stable enough to detect repeated failures but specific enough to distinguish a new cause.

The recovery manager should maintain a failure-pattern record containing the fingerprint, affected project area, attempted strategies, successful fixes, last known-good checkpoint, and confidence. This record can feed project memory and future self-improvement proposals after sensitive data is removed.

### 28.3 Progress quality

The runtime should measure whether an attempt made verified progress. Progress may include more passing tests, fewer runtime errors, a smaller conflict set, successful environment setup, a valid artifact, or a newly satisfied acceptance condition. A task that consumes requests without improving evidence should be routed into recovery rather than continuing the same loop.

## 29. Self-Observation and Self-Evaluation

### 29.1 Episode recording

Every completed, failed, cancelled, recovered, or escalated task should produce an `EpisodeRecord`:

> **Schema projection:** `EpisodeRecord` is defined in `nirman-schemas.md` §2.25. Owner: TA §29.1.

The episode record should contain structured summaries and references to raw evidence rather than copying unrestricted source code or secrets into long-term memory.

### 29.2 Runtime quality metrics

Nirman should track quality metrics by project type, task class, provider profile, worker role, and runtime version:

| Metric | Meaning |
|---|---|
| Goal completion rate | Percentage of tasks satisfying all required conditions |
| Evidence completeness | Percentage of completion claims with valid evidence |
| Regression rate | Percentage of tasks that break previously passing behavior |
| Recovery success rate | Percentage of failed attempts recovered automatically |
| Strategy diversity | Whether recovery attempts materially change approach |
| Repair efficiency | Verified progress per recovery cycle |
| Tool reliability | Success and failure rates by tool and environment |
| Provider reliability | Request success, tool-call correctness, and context-overflow rates |
| Self-update safety | Candidate pass, rollback, crash, and migration-failure rates |
| Attention reliability | Recall-probe pass rate by provider model, fill bucket, and position bucket, and `PREMISE_MISMATCH` rate per consequential step |
| Human intervention rate | Number and category of decisions required per task |

These metrics are for diagnosis and improvement. They must not be used to conceal failed tasks or to optimize only for speed at the expense of correctness.

### 29.3 Evaluation runs

An evaluation run executes a fixed fixture suite against a specific runtime version, provider configuration, model profile, prompt policy, tool registry, and project environment. The run must record the exact inputs and produce comparable results.

The evaluation engine should include ordinary feature tasks, multi-file refactors, environment failures, provider failures, merge conflicts, visual regressions, database migrations, sandbox tests, long-running continuation, self-update failures, and recovery scenarios.

### 29.4 CriticCalibrationTracker and BuildTimeMetricTracker

#### 29.4.1 CriticCalibrationTracker

`CriticCalibrationTracker` monitors calibration, precision, recall, and false-positive rates of advisory model reviews authored by `Critic Worker` (BS §23.4) against ground-truth deterministic execution outcomes:
1. *Empirical agreement measurement:* Correlates static model critique findings (e.g. predicted defects, anti-patterns, missing post-conditions) with actual scenario test execution (`CONTRACT.RUNTIME.E2E`), unit test assertions, and deterministic static analysis gates (`AndroidQualityGate`, §53.4).
2. *Critic drift detection:* Tracks temporal drift where model review rubrics become either overly permissive (vacuous approval) or overly strict (spurious rejection). Divergence exceeding policy thresholds triggers calibration warnings and prompts self-improvement fine-tuning (§30).
3. *Advisory-only integrity:* `CriticCalibrationTracker` operates strictly as a read-only calibration evaluator; it holds no decision authority over task completion or artifact promotion (`AGENTS.md §3`).

#### 29.4.2 BuildTimeMetricTracker

`BuildTimeMetricTracker` records precise execution latencies and stage timings across the construction loop:
1. *Time-to-First-Run (TTFR):* Measures the elapsed duration from initial user intent submission to the first rendered frame in the Nirman-managed local Android emulator preview window.
2. *Stage duration profiling:* Records wall-clock durations for intent formalization, technology resolution, code generation, compilation, test execution, emulator deployment, and artifact packaging.
3. *Zero-budget invariant:* Build latency metrics serve strictly as telemetry for desktop UI display and local diagnostic analysis. Nirman never bounds, throttles, terminates, or downgrades a valid task based on elapsed build time or duration quotas (ADR-218; BS §72).

## 30. Self-Improvement Manager

### 30.1 Improvement sources

The self-improvement manager may create improvement proposals from recurring failure patterns, regression clusters, provider incompatibilities, task-intervention categories, benchmark results, user corrections, stale instructions, tool failures, observed performance degradation, and validated completed complex tasks from which reusable behavior can be extracted. All sources are Nirman-internal: this loop improves Nirman's own prompts, routing, tool schemas, worker roles, and runtime code, never a generated project's behavior after publish (ADR-224).

It must not automatically convert a single unusual failure into a permanent rule. An improvement proposal should include evidence frequency, affected task classes, confidence, expected benefit, possible regressions, scope, and rollback plan.

### 30.1a Skill discovery from validated episodes

A successfully completed complex task is also an eligible skill-discovery source. Skill discovery MUST operate on the corresponding EpisodeRecord, TaskResult, validation evidence, and execution lineage rather than raw model prose. The discovery process identifies reusable behavior that generalizes beyond the originating task and produces a non-invocable skill candidate. Candidate creation, evaluation, and promotion follow the existing self-improvement pipeline described in build spec §28.4; discovery itself is described in build spec §28.4.

### 30.2 Improvement proposal

> **Schema projection:** `ImprovementProposal` is defined in `nirman-schemas.md` §2.26. Owner: TA §30.2.

Possible proposal targets include prompts, task decomposition rules, model routing, context retrieval, tool schemas, failure classifiers, UI instructions, validation rules, worker roles, skill packages, provider adapters, and runtime code.

Changes to the supervisor, policy engine, credential handling, sandbox, updater, database migration system, or evidence engine must receive the highest review level and may not be auto-promoted solely from a model-generated proposal.

### 30.3 Candidate generation loop

> **Schema projection:** `RepairExperimentationRecord` is defined in `nirman-schemas.md` §2.107. Owner: TA §30.3.

```text
Observe episodes and metrics
    ↓
Cluster recurring failures or opportunities
    ↓
Create an improvement hypothesis
    ↓
Generate a candidate patch in an isolated self-development worktree
    ↓
Run targeted tests and broad regression fixtures
    ↓
Run security, sandbox, migration, and recovery gates
    ↓
Compare candidate against the current baseline
    ↓
Run a canary task set in a disposable profile
    ↓
Promote, reject, or retain for review
    ↓
Monitor post-promotion outcomes
    ↓
Rollback if safety or quality degrades
```

The self-improvement manager should be able to run this loop in the background. It should not modify the active runtime merely because it found a possible improvement. Candidate changes must be versioned, reproducible, measurable, and reversible.

### 30.4 Candidate comparison

A candidate should be promoted only when it satisfies all mandatory safety gates and improves or preserves the agreed quality score. The score should weight correctness, evidence completeness, regression prevention, recovery success, security, compatibility, and stability. Speed and resource use are secondary objectives unless the user explicitly prioritizes them.

A candidate that improves one benchmark while causing regressions in another must be rejected or limited to the task class where it is safe. The manager should support scoped promotion for a provider, project profile, worker role, or task class instead of forcing one global behavior.

### 30.5 Promotion modes

| Mode | Behavior |
|---|---|
| Observe-only | Generate proposals and reports without changing runtime behavior |
| Candidate-only | Build and test candidates in isolation |
| Canary | Use the candidate for a small approved fixture or project class |
| Trusted auto-promotion | Promote automatically after all gates and canary checks pass |
| Manual promotion | Require user review before replacing the active version |

Trusted auto-promotion should still preserve the stable controller, safety policies, rollback artifacts, and non-bypassable credential and sandbox boundaries.

### 30.6 Post-promotion monitoring

After promotion, the runtime should compare candidate behavior with the previous baseline using task outcomes, error rates, recovery patterns, provider reliability, crash-free operation, and user corrections. A statistically meaningful degradation or safety regression should trigger automatic rollback or scoped disablement.

## 31. Runtime Memory and Learning Boundaries

**Implements:** build spec §38 and `CONTRACT.RUNTIME.MEMORY` (with §59; the build spec section is the authority)

Nirman should maintain three memory scopes:

| Memory scope | Contents | Lifetime |
|---|---|---|
| Task memory | Current goal, plan, evidence, failures, checkpoints, and active assumptions | Until task retention expires |
| Project memory | Architecture decisions, conventions, fixes, routes, dependencies, and validated preferences | Project lifetime, user-deletable |
| Runtime improvement memory | Anonymized failure patterns, evaluation results, provider compatibility, and candidate outcomes | Runtime version lifetime, user-controlled |

Memory should be written from validated events and user-confirmed decisions, not from every model statement. The user must be able to inspect, correct, export, and delete memory. Secrets, raw credentials, protected files, and unclassified private content must be excluded.

### 31.3 Project Memory Learning Service

The runtime provides a deterministic service that extracts cross-revision failure patterns from `EpisodeRecord` and `RepairPattern` data and uses them during planning.

**Responsibilities:**

- Extract causal failure signatures from episodes (task class, technology plan, failure fingerprint, recovery outcome)
- Store signatures in `ProjectMemoryEntry` records with scope, confidence, and evidence bindings
- During task initialization, query the ProjectMemory for matching signatures and surface findings to the PlanningWorker
- Update technologyPlan based on learned failure patterns (e.g., "Compose + X library → navigation bug")

**Schema:** `ProjectMemoryEntry` (SCHEMAS §2.101)

**Contract:** `CONTRACT.RUNTIME.MEMORY`

**Precedence:** Runtime intelligence, not authority - findings guide planning but cannot grant permissions, auto-complete requirements, or auto-block tasks.

> **Schema projection:** `ProjectMemoryEntry` is defined in `nirman-schemas.md` §2.101. Owner: TA §31.3.

## 32. Complete Runtime and Self-Improvement Failure Modes

The architecture must explicitly handle:

| Failure | Required response |
|---|---|
| Supervisor crash | Rehydrate task state, reconcile leases, and resume from the last validated checkpoint |
| Worker crash | Preserve workspace, record interruption, and requeue or recover |
| Provider outage | Retry with classification, use an approved fallback, or continue after service recovery |
| Context overflow | Compact, retrieve, or change provider; never silently omit critical requirements |
| In-window recall degradation or premise mismatch | Reject the proposal as `PREMISE_MISMATCH` before any transaction, re-project the DENSE block, narrow the step, or select a provider model by measured reliability; record the failure in the `AttentionReliabilityProfile`; never silently continue and never pause valid work |
| Infinite repair tendency | Detect repeated fingerprints, backtrack, change strategy, and escalate only when necessary |
| Candidate build failure | Keep the current version active and preserve candidate evidence |
| Candidate health failure | Stop promotion and retain the previous known-good version |
| Regression after promotion | Roll back or disable the candidate scope |
| Bad self-improvement rule | Revert the rule and mark its source proposal as failed |
| Database migration failure | Restore the previous version and recoverable database copy |
| Corrupted memory | Ignore invalid record, rebuild from validated events, and preserve the project |
| Disk/resource pressure | Adapt concurrency and storage, preserve checkpoints, and stop only for hard protection limits |

## 33. Strengthened Architecture Completion Criteria

The complete runtime is ready for implementation when it can receive one goal, extract requirements, create a durable task graph, run multiple workers, persist all events, execute the validation loop, recover from worker/provider/environment failures, continue across application restarts, produce evidence-backed completion, and preserve the task until the goal is complete or a genuine hard stop condition exists.

The self-improvement loop is ready when Nirman can observe task episodes, identify recurring failure patterns, create a scoped improvement proposal, build a candidate in isolation, evaluate it against deterministic fixtures and safety gates, run a canary, promote it through the stable controller, monitor post-promotion behavior, and automatically roll back without corrupting the active application or user projects.

## 34. End-to-End Autonomous Android Session

The runtime must model the user’s one-shot Android request as an `AutonomousAndroidSession`. The session owns the complete lifecycle from chat and screenshots to project synthesis, live preview, recovery, validation, and APK delivery. The session continues independently of the chat renderer and is resumable after UI closure, process restart, or host suspend/resume where the operating system permits it.

> **Schema projection:** `AutonomousAndroidSession` is defined in `nirman-schemas.md` §1.14. Owner: BS §29.2.

### 34.1 Input-fusion pipeline

The input manager combines the user instruction, screenshots, supplied assets, existing project files, emulator requirements, integrations, and delivery requirements into an application contract, editable visual specification, and technology plan. Screenshots are interpreted as visual evidence and never as executable permission. The technology resolver selects or composes the required Android implementation without requiring a user-facing framework or template choice.

### 34.2 Preview revision bridge

The preview manager and execution manager share a `projectRevisionId`, `activeBranchId`, `checkpointId`, and promotion lineage. Every emulator state records the revision, emulator identity, installation state, reload state, Logcat stream, runtime errors, screenshot, visual comparison result, and responsible task node. Preview currency additionally requires:

> **Schema projection:** `DeviceStateFingerprint` is defined in `nirman-schemas.md` §2.27. Owner: TA §34.2.

> **Schema projection:** `ApplicationStateFingerprint` is defined in `nirman-schemas.md` §2.28. Owner: TA §34.2.

> **Schema projection:** `EnvironmentStateFingerprint` is defined in `nirman-schemas.md` §2.29. Owner: TA §34.2.

If a candidate revision fails, the preview manager retains the last valid revision and marks the candidate as failed instead of presenting it as current. An identical emulator identity is not sufficient when any required device, application, or environment fingerprint differs.

### 34.3 Progress ledger, fingerprint registry, and stall detector

The runtime maintains a progress ledger containing changed files, new evidence, preview movement, test transitions, worker handoffs, strategy changes, validated requirements, and artifact transitions.

The fingerprint registry maintains three distinct fingerprint types to prevent anti-thrashing evasion:

| Fingerprint type | Purpose | Components |
|---|---|---|
| Failure fingerprint | Detect repeated failures | Normalized command, exit code, error class, stack-trace structure, changed-file set, environment state, provider response class, validation stage |
| Strategy fingerprint | Detect repeated strategies | Recovery level, action type, target component, model route, patch approach |
| Causal/root-cause fingerprint | Group related root causes | Abstracted error pattern, dependency chain, configuration state, environmental factor |

Each recovery attempt records:
- `failureFingerprint`: what went wrong
- `strategyFingerprint`: what was tried
- `causalFingerprint`: the underlying root cause hypothesis
- `progressDelta`: measured improvement (passing tests, error reduction, conflict reduction, artifact validity)
- `recoveryAttemptId`: unique identity for this attempt

The stall detector identifies repeated commands, repeated patches, repeated failure fingerprints, repeated strategy fingerprints, unchanged workspaces, absent evidence, unresponsive processes, stale emulators, and heartbeats without useful progress. A detected stall causes a controlled strategy transition: refresh context, repair the environment, change technology, delegate diagnosis, restore a checkpoint, reduce scope to a safe subtask, or construct an isolated alternative. The scheduler must reject identical retries that do not provide a new strategy fingerprint, new causal fingerprint, or positive progress delta. A stall never produces a paused task: the detector's finding is a `RecoveryAuthority` input, and the only waiting states a task may enter are `WAITING_APPROVAL` for a hard gate and `WAITING_RESOURCE` for physical capacity (build spec §26.14), both scoped to the affected requirement while independent requirements continue (build spec §29.4; ADR-226).

### 34.4 Swarm handoff and reconciliation contract

Parallel workers receive explicit contracts, isolated workspaces, allowed tools, expected outputs, and validation rules. Each handoff must include changed files, assumptions, dependencies, tests, evidence, unresolved issues, and recommended next actions. A handoff also answers what the worker proved: its `frontierDelta` lists every `EvidenceFrontier` claim it moved and the evidence that moved it, and `remainingUnproven` lists the claims of its contract still `UNRESOLVED` (build spec §52.3; ADR-225); the reconciliation worker integrates a slice whose `frontierDelta` is empty only as an unvalidated candidate, and `remainingUnproven` becomes the next cycle's frontier input. The reconciliation worker integrates only validated outputs, resolves conflicts, runs integrated Android checks, updates the preview revision, and commits the next checkpoint.

### 34.5 Autonomous validation and artifact gate

For applicable Android delivery, the validation coordinator must prove build success, APK existence, checksum, artifact scan, installation or launch, main-flow execution, visual comparison, permission behavior, and absence of unresolved fatal runtime errors. The artifact is complete only when it is linked to the project revision and evidence ledger.

Routine project-local actions are allowed under the Autonomous-build policy (build spec §23.7). The runtime may edit, install dependencies, run terminals, launch devices, build, test, capture screenshots, repair, checkpoint, delegate, reconcile, and create local artifacts without repeated approval. Protected credentials, destructive actions, publishing, signing policy, protected paths, hard safety violations, and unrecoverable blockers remain deterministic authority boundaries.

## 35. Complete Android Capability Fixture Contract

The test harness must include generated-from-instruction fixtures for Kotlin, Java, Android Views, Jetpack Compose, mixed native architectures, custom NDK/CMake native modules, background services, WorkManager, notifications, camera and media, location and sensors, Bluetooth and NFC, offline-first storage, API-heavy applications, authentication and permissions, tablet and multi-orientation layouts, device-integrated applications, and APK delivery (ADR-257). These fixtures validate AI technology selection and composition; they are not user-facing templates.

## 36. Production Runtime Contract Architecture

The production runtime is divided into deterministic authorities and model-driven proposal services. The model gateway proposes plans, edits, tool calls, recovery strategies, and improvement proposals. The supervisor, lifecycle authority, permission authority, sandbox authority, storage authority, evidence authority, recovery authority, and promotion authority decide what can execute and what counts as complete; termination is owned by the `LifecycleAuthority`.

### 36.1 Canonical runtime contracts

The following contracts are versioned and validated at the control-plane boundary:

> **Schema projection:** `CanonicalSchemaRegistry` is defined in `nirman-schemas.md` §3.1. Owner: TA §36.1.

Each contract has a schema version, owner, lifecycle status, project scope, source revision, created timestamp, updated timestamp, and audit references where applicable. Persistent records use atomic writes, file locking, migration backups, and rollback.

`APKExportRecord` is not a registered schema: it is the read-model view of `ExportVerificationRecord` for an APK deployment (§74.3, build spec §78) and carries no field of its own. `SigningState`, `DeliveryState`, `ReproducibilityLevel`, `AssuranceState`, `CapabilityMaturity`, and `ProductLifecycleState` are enumerations owned by build spec §5.7.2, not registered schemas; `ReproducibilityLevel` is the value set of the `reproducibilityLevel` field of `AndroidCapabilityProfile` and `ArtifactSet`. `PackagingProfile` is owned by build spec §5.7.3 and `SkillPackage` by build spec §23.11; §19.1 restates the latter field for field.

`CanonicalSchemaRegistry` is the sole machine-readable ownership index for these contracts. Its membership is enumerated at `nirman-schemas.md` §3.1 and is machine-checkable (ADR-241): each registered name has a field block in that document or is declared in §3.1's prose-defined identity list. Each entry records `schemaId`, `canonicalOwner`, `version`, fields, enum values, invariants, migration policy, authority, persistence location, and acceptance-fixture IDs. Repeated schema descriptions in other documents are explanatory or implementation views and must identify the registry entry they implement; they cannot silently redefine fields or enum semantics.

Schema compatibility is explicit:

> **Schema projection:** `ContractCompatibility` is defined in `nirman-schemas.md` §1.6. Owner: BS §5.7.9.

A self-development candidate or contract migration cannot be promoted until its read/write compatibility, migration, restart, replay, and rollback behavior pass the declared fixtures. `IntegrationBoundaryContract` is the common reference envelope for boundary-crossing operations; specialized contracts remain authoritative for payloads, state machines, authorities, transactions, evidence, preview, providers, skills, artifacts, signing, and completion.

Registry metadata for the content, conversation, and change-intelligence schemas is fixed as follows. The registry entry is the single canonical schema identity; the build spec section owns the normative contract shape, the architecture section owns the implementation semantics, and both cite the one field block in `nirman-schemas.md` (the `schema blocks` column), so the two cannot disagree field for field. The fixture column names the acceptance fixture that proves them.

| schemaId (normative contract; implementation schema) | canonicalOwner | version | lifecycle | persistence | authority | acceptance fixture | schema blocks |
|---|---|---|---|---|---|---|---|
| `Content`, `ContentRevision`, `ContentRevisionDraft`, `ContentMutation`, `ContentValidationResult`, `ContentPropagationPlan`, `TerminologyProfile`, `ContentEvidence`, `ContentDependency` (normative contract: BS §81.1; implementation schema: TA §85.1) | `CONTRACT.RUNTIME.CONTENT_INTELLIGENCE` | 1 | revision-bound to the parent `ConstructionTransaction`; invalidated through the `ImpactGraph` (§85.4) | `ContentStore` (SQLite; §85.3) | `ContentAuthority` admits and transitions; `EvidenceAuthority` owns `ContentEvidence` validity | `TEST-CONTENT-001` | SCHEMAS §1.66, SCHEMAS §1.67, SCHEMAS §2.76, SCHEMAS §2.77, SCHEMAS §2.78, SCHEMAS §2.79, SCHEMAS §2.80, SCHEMAS §2.81, SCHEMAS §1.68 |
| `Conversation`, `ConversationMessage`, `ConversationAttachment`, `ConversationRequirement`, `ConversationDecision`, `ConversationSuggestion`, `ConversationTaskLink`, `ConversationRequirementIndex`, `ConversationDecisionIndex`, `ConversationRebaseRecord` (normative contract: BS §82; implementation schema: TA §86.1) | `CONTRACT.RUNTIME.CONVERSATION_CONTEXT` | 1 | durable lineage; `conversationRevision` advances only when `ConversationContinuationResolver` commits new durable conversation state (BS §82.1); attachments `ACTIVE → DELETED` (§86.2) | `ConversationStore` (SQLite; §86.2) | `ConversationContinuationResolver` resolves; Memory, Context, and Task authorities remain canonical for what the indices reference (§86.4) | `TEST-CONV-001` | SCHEMAS §1.69, SCHEMAS §2.82, SCHEMAS §1.73, SCHEMAS §1.70, SCHEMAS §1.71, SCHEMAS §1.72, SCHEMAS §2.83, SCHEMAS §2.84, SCHEMAS §2.85, SCHEMAS §2.86 |
| `ChangeReportRecord`, `ChangeImpactReport` (normative contract: BS §83.1; implementation schema: TA §87.1) | `CONTRACT.RUNTIME.CHANGE_INTELLIGENCE` | 1 | record `INCOMPLETE → COMPLETE` or `INCOMPLETE → UNRESOLVED`; a COMPLETE report is immutable (§87.1) | `ChangeIntelligenceStore` (SQLite; §87.5) | `ChangeIntelligenceProjector` is read-only; `RecoveryAuthority` owns reconstruction (§87.4, §87.6) | `TEST-CHANGE-001` | SCHEMAS §1.74, SCHEMAS §1.75 |

### 36.2 Lifecycle authority

The lifecycle authority implements the state machine:

```text
Created → Understanding → Planning → EnvironmentPreparing
  → ProjectSynthesizing → Implementing → Previewing
  → Testing → Recovering → Revalidating → Packaging → Completed
```

Terminal states are `BlockedByPolicy`, `BlockedByMissingInformation`, `ProviderUnavailable`, `EnvironmentUnrecoverable`, `Cancelled`, and `SafelyFailed`. State transitions are accepted only through deterministic transition guards. A model response, worker message, skill, hook, or frontend event can request a transition but cannot commit one.

This machine is the build spec §33.2 session lifecycle; its machine-readable enum is `ProductLifecycleState` (build spec §5.7.2) and the name-by-name mapping between the two, plus the projection of per-task states (§5.1 / build spec §26.14), kernel cycle outcomes (§71.4), and completion classifications (§23.7) onto it, is fixed in build spec §33.2 and is not restated here. The lifecycle authority is `LifecycleAuthority` = the pure `SessionReducer` of §45.1 (ADR-159): one component, two names, one commit path for session and task transitions alike.

### 36.3 Renewable session leases and operation capabilities

The session supervisor maintains a renewable lease containing session ID, supervisor generation, last heartbeat, progress sequence, project revision, sandbox profile, and authority policy. Lease renewal is permitted only when the task is making validated progress or is waiting on a classified external condition.

Sensitive operations use single-use operation capabilities with an action type, session ID, worker ID, workspace ID, project revision, scope fingerprint, permission policy, issued time, expiry, and consumption state. The capability manager consumes the capability before side effects and rejects reuse, revision mismatch, scope mismatch, policy mismatch, or expired capabilities. The model cannot create or broaden capabilities.

### 36.4 Cross-entity evidence and completion contracts

The following records are canonical implementation contracts, not additional authorities. They connect the existing lifecycle, evidence, preview, artifact, policy, toolchain, device, and integration services so that no subsystem can report a stronger state than its dependencies permit.

> **Schema projection:** `EvidenceDependency` is defined in `nirman-schemas.md` §2.30. Owner: TA §36.4.

> **Schema projection:** `ArtifactSet` is defined in `nirman-schemas.md` §2.31. Owner: TA §36.4.

> **Schema projection:** `IntegrationOperationality` is defined in `nirman-schemas.md` §1.3. Owner: BS §5.7.5.

> **Schema projection:** `ExternalEffectRecord` is defined in `nirman-schemas.md` §2.32. Owner: TA §36.4.

`ExternalEffectRecord.reconciliationState` generalizes the export-only `UNKNOWN → RECONCILING` pattern (ADR-203) to every external side effect. Every adapter that performs an external effect—ADB install, Nirman-managed local Android emulator launch, provider/model request, signing operation, filesystem copy, package installation, process creation, and remote API—MUST record an `ExternalEffectRecord` and implement reconciliation against the canonical `reconciliationState` lifecycle:

- `KNOWN_SUCCESS` / `KNOWN_FAILURE`: observed and verified terminal state.
- `UNKNOWN`: the effect was issued but its outcome could not be confirmed (timeout, partial response, device/provider drop, interrupted copy, process disappearance).
- `RECONCILING`: an `UNKNOWN` outcome is being resolved by destination/identity/hash inspection or provider/device status re-check; no retry of the effect is permitted until resolution.
- `RESOLVED`: reconciliation completed and the outcome was deterministically classified as success or failure (recorded in `responseReference` / compensation state).

An `UNKNOWN` outcome MUST NOT be retried, promoted, or reported as success until it transitions to `RESOLVED`. This applies uniformly; the export copy path (M117/ADR-203) is one instance, not a special case. The `CanonicalSchemaRegistry` owns this schema; adapter-local variants are explanatory only and must not redefine the enum.

For provider/model requests, `ProviderRequestProvenance` owns logical-request identity and durable provenance, while each `ProviderRequestAttempt.externalEffectId` resolves the required `ExternalEffectRecord`. The provenance record does not replace external-outcome reconciliation, and `ExternalEffectRecord` does not duplicate prompt, context, adapter, stream, usage, proposal, or evidence lineage. `responseReference` resolves the applicable provider response or normalized-response identity recorded by the provider runtime.

> **Schema projection:** `UsageRecord` is defined in `nirman-schemas.md` §2.33. Owner: TA §36.4.

`RepositoryTrust`, `EnvironmentIdentity`, `SigningState`, `ReproducibilityLevel`, `CapabilityMaturity`, `ProductLifecycleState`, `AssuranceState`, `IntegrationOperationality`, and `DeliveryState` are separate fields. They MUST NOT be collapsed into a single status or inferred from model output.

The canonical evidence chain is:

```text
Observation → EvidenceArtifact → ValidationResult → CertificationDecision → CompletionDecision
```

A source revision, asset manifest, toolchain lock, emulator session, dependency snapshot, validation policy, or required integration change invalidates dependent evidence and completion claims unless the dependency graph proves independence. `EvidenceAuthority`, `PreviewPromotionGate`, `ArtifactAuthority`, `AndroidQualityGate`, and the completion evaluator consume the same dependency relation.

The canonical preview-current predicate is:

```text
preview_is_current(P) =
    P.activeBranchId == activeBranchId
AND P.projectRevisionId == activeProjectRevisionId
AND P.promotionLineage is the recorded lineage of activeBranchId
AND P.checkpointId == activeCheckpointId
AND P.sourceFingerprint == activeSourceFingerprint
AND P.contractVersion == activeContractVersion
AND P.technologyPlanVersion == activeTechnologyPlanVersion
AND P.assetManifestVersion == activeAssetManifestVersion
AND P.artifactFingerprint == installedArtifactFingerprint
AND P.deviceStateFingerprint == activeDeviceStateFingerprint
AND P.applicationStateFingerprint == activeApplicationStateFingerprint
AND P.environmentStateFingerprint == activeEnvironmentStateFingerprint
AND P.executionTruth in {OBSERVED, VERIFIED}
AND requiredEvidence(P) is current
AND no invalidation exists after P.observedAt
AND no policy or safety block is active
```

Every `P.` term is a field of the build spec §69.4 `PreviewRevision`; the toolchain lock and emulator session participate through `environmentStateFingerprint` and `deviceStateFingerprint` (§34.2), never as fields of their own.

Only the preview coordinator may promote a candidate through this predicate. UI, workers, models, artifact inspection, and presentation reducers may report facts but cannot independently make a preview current.

Completion requires current mandatory evidence, selected profile maturity, required integration operationality, preview and artifact gates when declared, signing policy, reproducibility policy, and no unresolved blocking condition. `COMPLETED`, `VERIFIED`, `CURRENT`, `SUPPORTED`, `DELIVERED`, `FUNCTIONAL`, and `CERTIFIED` states are rejected when their required dependencies are missing, stale, invalidated, or model-authored. The outcome is recorded as a `CompletionState` (build spec §5.7.2: `NOT_EVALUATED | NOT_COMPLETE | COMPLETED | BLOCKED | USER_REQUIRED | INVALIDATED`) on `CompletionDecision` and mirrored into `AutonomousAndroidSession.completionState` (§34); `CERTIFICATION ≠ COMPLETION` (build spec §5.7.7) means a certified artifact may carry `NOT_COMPLETE`.

### 36.5 Transaction domains and capability promotion

The runtime separates transaction domains because they have different rollback semantics:

> **Schema projection:** `LocalTransaction` is defined in `nirman-schemas.md` §2.34. Owner: TA §36.5.

> **Schema projection:** `DeviceTransaction` is defined in `nirman-schemas.md` §2.35. Owner: TA §36.5.

> **Schema projection:** `ExternalEffectTransaction` is defined in `nirman-schemas.md` §2.36. Owner: TA §36.5.

`ConstructionTransaction` governs local source and artifact preparation. Device operations produce observations and cleanup records; they are not assumed to be atomically rolled back with source changes. External effects require `ExternalEffectRecord` reconciliation and compensation semantics. A local commit never implies that a remote or device operation succeeded.

Capability promotion follows a deterministic chain:

```text
CapabilityEvidence
  → CapabilityValidation
  → CapabilityCertification
  → CapabilityPromotionAuthority
  → immutable promotion record
```

Workers and models may propose capability status changes but cannot write `SUPPORTED`, `VERIFIED`, or `CERTIFIED` directly. The promotion authority requires the matching profile, fixture, current evidence, environment identity, and policy version.

Release signing uses an immutable binding:

> **Schema projection:** `SigningIdentityBinding` is defined in `nirman-schemas.md` §1.5. Owner: BS §5.7.9.

A release-signed claim is invalid without this binding and an observed signing-inspection result.

> **Schema projection:** `SigningOperation` is defined in `nirman-schemas.md` §2.37. Owner: TA §36.5.

> **Schema projection:** `CertificateInspection` is defined in `nirman-schemas.md` §2.38. Owner: TA §36.5.

A signing request or keystore reference is not proof of signing. `CertificateInspection` must observe the packaged artifact and compare it with `SigningIdentityBinding` before a release-signed state is accepted.

## 37. Android Project Ingestion and Integrity Architecture

The ingestion service uses an Android-aware discovery pipeline:

```text
Project root
  → ignore and hard-exclusion rules
  → Android/Gradle/manifest/resource discovery
  → generated-output classification
  → secret and credential classification
  → canonical path normalization
  → scope fingerprint
  → content and metadata fingerprint
  → repository map and dependency graph
```

Generated build directories, local properties, keystores, OAuth files, environment files, and unrelated personal data are classified before any model request. Every reconciliation, preview installation, packaging operation, and self-development promotion rechecks the project revision and scope fingerprint. A content or scope mismatch forces re-ingestion and invalidates the affected operation capability.

## 38. Provider Gateway and Tool Protocol

The provider gateway has five layers:

1. **Profile registry:** validates endpoint, model ID, protocol, credentials reference, privacy policy, and capabilities.
2. **Request normalizer:** converts Chat Completions, Responses-style, and provider-native requests into a canonical internal envelope.
3. **Multimodal adapter:** handles text, screenshots, assets, and structured visual inputs.
4. **Tool protocol:** validates typed tool names, versions, arguments, requested permissions, and result schemas.
5. **Response normalizer:** converts text, structured output, tool calls, usage, cancellation, and provider errors into canonical events.

The gateway never gives a provider direct filesystem, process, emulator, or credential access. Tool calls are proposals passed back to the deterministic tool broker. The broker checks session, worker, workspace, policy, sandbox, scope, and operation capability before starting a tool.

Before any project context leaves the host, the provider gateway constructs a `ProviderContextEnvelope` containing `dataClassification`, `providerPolicyId`, `selectedContextIds`, `redactionPolicyId`, `userApprovalPolicyId`, `allowedPurpose`, `retentionPolicy`, `transmissionDecision`, and `providerRequestId`. Only the minimum context required for the declared purpose may be transmitted. Secrets, private reasoning, unrelated personal data, protected credentials, and excluded paths are withheld. Provider responses cannot broaden the envelope or authorize tools, permissions, mutations, or completion.

Provider-specific failures are classified into the eight retry classes of §24.6 — authentication errors, invalid requests, rate limits, transient network failures (including transport faults and timeouts), provider overload or unavailability, context overflow, unsupported capabilities, and content-policy responses — with user cancellation recorded as a request outcome rather than a failure class. Recovery policy chooses retry, fallback, context reduction, model change, or safe waiting per class according to the configured provider policy.

## 39. Sandbox and Process Separation

The host is divided into explicit process domains:

| Domain | Responsibility | Default access |
|---|---|---|
| Desktop shell | Chat, project navigation, preview framing, settings | IPC only; no direct project filesystem |
| Control-plane supervisor | Lifecycle, leases, task graph, permissions, persistence | Authority services and approved worker control |
| Worker process (`NirmanWorker.exe`) | Planning, coding, debugging, testing, and visual-QA reasoning for one lease | Its `WorkerConnection` only; workspace, tools, and model are reached through the supervisor (§3.5) |
| Build process | Gradle, SDK, package manager, compiler | Project workspace, toolchain, declared network |
| Emulator manager | Emulator lifecycle, install, capture, Logcat | Emulator APIs only |
| Preview application | Runs generated Android app | Disposable app/emulator profile |
| Provider transport | Model requests | Approved provider endpoints only |
| Credential service | API keys and signing material | OS-protected secret references only |

The credential authority flow is: WinUI settings → typed credential command → supervisor → OS credential store. The UI must never retrieve plaintext secrets merely to display/configure them. Only keychain references are stored in ordinary records.

Generated code and project processes cannot read personal browser data, SSH keys, unrelated directories, signing keys, or arbitrary credentials. Sandbox profiles are selected by the policy authority and cannot be relaxed by model output.

Browser validation, when enabled, is an external auxiliary surface only. The capability registry MUST mark it as non-authoritative for Android-core validation. Browser observations cannot satisfy Android build, install, launch, device, accessibility, visual, or completion requirements unless a separate non-Android surface was explicitly declared. The Nirman-managed local Android emulator remains the authoritative generated-app validation surface.

## 40. Event, Evidence, Memory, and Replay Stores

The event store records typed runtime events. The evidence store records validation proof. The memory store records only privacy-filtered, validated knowledge. The replay store records enough metadata to reproduce a task without indiscriminately retaining private source content.

```text
model claim → runtime event → validation observation → evidence record → requirement status
```

A requirement becomes complete only when its evidence record satisfies the acceptance rule. Project memory includes source, confidence, project scope, revision, retention, and deletion metadata. Credential and signing records are never eligible for semantic memory.

The replay service supports reopen, rerun validation, fork strategy, restore checkpoint, compare preview revisions, compare providers, and reproduce a failure using a sanitized fixture or approved project context.

## 41. Host Reliability and Recovery

The Windows host must initialize without a provider (`SessionProviderMode.PLANNING_ONLY`, build spec §5.7.2), enter Offline Mode (`SessionProviderMode.OFFLINE`) when network access is unavailable, preserve history and checkpoints, and resume eligible active sessions after restart or reboot. State writes use temp-file-plus-rename, file locks, versioned migrations, backups, and rollback. The installer and updater preserve user state and keep the previous version runnable if candidate startup, IPC, migration, or health checks fail.

Large projects use virtualized trees, repository-map shards, dependency fingerprints, affected-test computation, cached validation, rotating logs, content-addressed checkpoint storage, and retention policies. Resource pressure adapts concurrency and storage retention; only hard protection limits stop execution.

### 41.1 Documentation and runtime certification boundary

The contract-graph verifier certifies document structure, contract addressing, authority references, and selected semantic rules only. It is not the runtime certification authority. Runtime certification requires separate executable jobs for schema compilation, reducer transitions, transaction and lease behavior, Windows process and IPC isolation, provider fixtures, Android build and Nirman-managed local Android emulator execution, preview truth, APK inspection, failure injection, restart recovery, hidden-human-dependency handling, self-development rollback, and platform capability and cross-compilation fixtures (§84.5).

## 42. Runtime Architecture Acceptance Tests

The architecture is implementation-ready only when tests prove that:

- State transitions cannot be committed by model output or UI state.
- Operation capabilities are scoped, single-use, revision-bound, and non-reusable.
- Project and scope changes invalidate affected capabilities.
- Tool calls cannot bypass the broker or sandbox.
- Provider outputs cannot mark requirements complete without evidence.
- Generated code cannot access protected host resources.
- A failed preview candidate cannot replace the last valid revision.
- Memory excludes secrets and supports deletion.
- A failed session can be reopened, forked, resumed, or restored from checkpoint.
- Host restart, provider outage, emulator failure, build failure, and disk pressure are recoverable or safely terminal.
- Evidence dependencies invalidate completion claims after source, asset, toolchain, device, artifact, policy, or integration changes.
- An unknown external-effect response is reconciled by idempotency key or read-back before retry.
- Parent, child, shared, estimated, and unavailable resource usage remain attributable in the execution ledger.
- A required integration cannot be marked functional from build or launch evidence alone.

## 43. Architecture Completion Principle

The runtime is complete only when one Android goal can travel through planning, synthesis, tools, workers, preview, validation, recovery, packaging, and evidence without the chat interface becoming the source of truth. Deterministic authorities must remain in control throughout the entire lifecycle.

## 44. Integrated Production Runtime Architecture

This section translates the accepted Sync-AI-derived principles into Nirman’s Android-only runtime. Windows is the host operating system; Android is the sole generated application target.

### 44.1 Runtime ownership

| Component | Owns | Does not own |
|---|---|---|
| ModelGateway and AI workers | Interpretation, planning, technology proposals, repair proposals, visual analysis, decision summaries | Lifecycle, permissions, direct file writes, arbitrary process authority, completion, artifact promotion |
| Control-plane supervisor | Scheduling, leases, retries, recovery, worker lifecycle, health, event emission | Model reasoning or unvalidated mutation |
| `LifecycleAuthority` (`SessionReducer`, §45.1) | Durable session and task state transitions and replay | Side effects |
| Policy authority | Permissions, sandbox profiles, physical resource requirements, network and device policy | AI strategy |
| Transaction manager | Snapshots, revision checks, conflict detection, commit/rollback | Unvalidated model output |
| Toolchain authority | Android toolchain resolution, lock verification, environment construction | User project semantics |
| Evidence authority | Validation gates, evidence completeness, artifact eligibility | Claiming success without proof |
| Preview coordinator | Revision-bound Nirman-managed local Android emulator deployment and preview fallback | Promoting stale preview state |
| Artifact authority | APK packaging and optional AAB packaging, checksums, signing workflow, promotion | Modifying source without a transaction |

The invariant is:

> **The model proposes. Deterministic runtime authorities decide, execute, validate, recover, and promote.**

### 44.2 Runtime module graph

```text
WinUI 3 C#/.NET UI
        │ typed commands/events only
        ▼
Rust Control Plane Supervisor
├── SessionReducer
├── EventStore
├── ConstructionTransactionManager
├── LeaseManager
├── PolicyAuthority
├── ResourceGovernor
├── WorkerRegistry
├── TerminalSupervisor
├── ModelGateway
├── ToolchainAuthority
├── AndroidCodeIntelligence
├── RequirementAuthority
├── PreviewCoordinator
├── RecoveryAuthority
├── EvidenceAuthority
├── ArtifactAuthority
├── ProjectMemoryStore
└── ContextOrchestrator
        │
        ├── NirmanWorker.exe reasoning processes, one per lease (§3.5)
        ├── persistent PTY terminals
        ├── Android SDK/JDK/Gradle/ADB/emulator processes
        ├── provider bridge or direct provider adapters
        └── SQLite state and evidence database
```

No UI command may bypass the control plane to invoke a terminal, edit a file, launch an emulator, contact a provider, install a package, or promote an artifact.

This graph and the §57.2 process topology describe the same `NirmanSupervisor.exe`: §57.2 lists the authority services by their canonical names (`LifecycleAuthority` = `SessionReducer` + `EventStore`, `ToolBroker`, `TaskScheduler`, `CheckpointManager`, `AndroidWorkflowCoordinator`), while this graph additionally shows the internal modules those authorities compose (`ConstructionTransactionManager`, `LeaseManager`, `ToolchainAuthority`, `AndroidCodeIntelligence`, `RequirementAuthority`, `ProjectMemoryStore`) and the external processes they supervise. Neither graph introduces a component absent from the other's authority set; a module named in only one graph is composed by an authority named in both. Every name in either graph is a row of the §57.12 component and authority registry or is defined by a section of its own; the registry fixes each one's crate, decision rights, and committed records.

### 44.3 Human-in-the-Loop Service Layer

This section defines the components that implement the human-in-the-loop and autonomy-control contracts of build spec §4.6, BS §23.7–BS §23.9, BS §27.11, BS §28.7, and BS §29.4. Every component here is a **module or presentation service only**: none hold authority, none commit authoritative state, and none may grant permissions, mark tasks complete, promote artifacts, or bypass the sandbox. All state mutations route through the relevant authority (§57.12). The components are supervisor-hosted.

**Architectural scope boundary.** Two items from the standard HITL checklist are architecturally excluded from Nirman by sealed invariants:

- *Step-through mode* (attended, per-action approval loop): excluded by ADR-226 and BS §23.3. There is exactly one operating mode — Autonomous-build. No attended variant exists to enter, and no component may introduce one.
- *Autonomy level tuner* (user-selectable autonomy dial): excluded by ADR-226 and BS §23.3. The autonomy levels of BS §28.5 are **observed evidence states** reported by the runtime, never user-selectable options.
- *AI spend approval threshold*: the AI-monetary-budget variant is excluded by ADR-218 and BS §72 (AI usage is telemetry only, zero execution-authority semantics). Physical resource limits are covered by `ResourceGovernor` (§77) and `SwarmAdmissionController` (§58.5.1).

#### 44.3.1 Autonomy control components

`MidRunEditCoordinator` is a module in `nirman-control-plane` that receives manual file-save events while an autonomous task is active (the `workspace_file_saved` lifecycle hook of BS §27.4). It validates the edit against the active workspace lease, routes the mutation through `ConstructionTransactionManager`, and emits `workspace_file_saved` to trigger the continuation paths of BS §27.11. It does not interrupt the autonomous loop; it queues the edit as a dependency delta for the next `EVALUATE_PROGRESS` cycle. `PolicyAuthority` must admit the edit path and `ConstructionTransactionManager` must commit it; `MidRunEditCoordinator` never writes directly.

`RiskClassifier` is a sub-module of `PolicyAuthority` (crate `nirman-policy`) that classifies every incoming tool action into an ordered risk class — `ROUTINE`, `REVIEWABLE`, `PRIVILEGED`, or `HARD_GATED` — before the three-outcome allow/ask/deny decision is reached. The classification is deterministic, based on the action kind, path, worker role, network destination, and the execution profile of §16.2.2. Risk class feeds the approval card's risk explanation (§23.6) and the autonomy capability evidence reported by BS §28.5. `RiskClassifier` never makes the allow/ask/deny decision itself — that decision belongs to `PolicyAuthority`.

#### 44.3.2 Explainability and communication components

`ProgressNarrator` is a module in `nirman-control-plane` that subscribes to the `EventStore` event stream (§45.2) and converts typed task events into human-readable narrative progress strings projected to the UI via the `SupervisorConnection` (§57.3). It is a read-only projection and holds no execution state. It must not coerce event semantics — it maps events, it does not interpret or modify them.

`DiffSummarizer` is a module in `nirman-control-plane` that receives a committed `ConstructionTransaction` record (§45.3) and produces a human-readable mutation summary listing changed files, added/removed lines, affected symbols, and the requirement node that motivated the change. The summary is attached to the `ChangeReportRecord` obligation of §45.3 and projected to the UI. It is a read-only transformation; it does not alter the transaction or its evidence.

`UncertaintyCommunicator` is a module in `nirman-kernel` that subscribes to `UncertaintyRegistry` and `ContradictionDetector` (§58.12) and surfaces unresolved uncertainty findings as structured decision nodes visible in the execution tree (§23.2). It converts internal uncertainty entries into a user-facing `UncertaintyNotice` (schema: `uncertaintyId`, `source`, `affectedRequirements`, `description`, `suggestedClarification`, `severity`) and routes it to `DecisionNodeManager` (§58.12). It does not resolve uncertainty — `RecoveryAuthority` and the clarification gate of BS §69.11 own that path.

`BlockerReportGenerator` is a module in `nirman-control-plane` that monitors the task graph for requirement nodes carrying `BLOCKED` or `USER_REQUIRED` decisions (BS §27.10) and assembles a structured blocker report (schema: `blockerReportId`, `taskId`, `blockedRequirements[]`, `automatedPathsAttempted[]`, `decisionRequired`, `escalationLevel`) surfaced through the Action Center (BS §4.6) and the `PARTIALLY_BLOCKED` classification path. It is a read-only assembler; it does not set the `BLOCKED` or `USER_REQUIRED` decisions — `RecoveryAuthority` and `TaskScheduler` do.

`QuestionBatchingEngine` is a module in `nirman-kernel` that implements the clarification gate of BS §69.11. When a requirement analysis, planning step, or worker produces a clarification question, this module groups, deduplicates, and orders pending questions by dependency and priority before surfacing them as a single batched `ClarificationRequest` to the user. Answers route back through `GoalInterpreter` as requirement-level updates. The engine never bypasses the clarification gate — it optimizes the user experience of it.

`CompletionReportComposer` is a module in `nirman-control-plane` that assembles the final `TaskResult` (BS §23.9, BS §30, SCHEMAS §1.11) from the `EvidenceAuthority` ledger, `EventStore` records, and `ArtifactAuthority` promotion records when `EvidenceAuthority` issues a completion decision. The composed report includes the requested goal, changed files, checkpoints, worker activity, commands, validation evidence, tests, builds, screenshots or device results, warnings, blockers, unresolved conditions, resource usage telemetry, and the final completion classification. `CompletionReportComposer` reads authoritative records; it does not produce evidence or make completion decisions.

`OutputWalkthroughGenerator` is a module in `nirman-control-plane` that generates a human-readable post-task walkthrough artifact from the `EpisodeRecord` (BS §28.3), evidence ledger, and committed event sequence after task completion or escalation. The walkthrough describes the goal, key decisions, worker stages, notable repairs, validated evidence, and final state. It is persisted as a documentation artifact in the task directory and linked from the completion report. It is read-only; it does not create new evidence.

#### 44.3.3 Feedback ingestion pipeline

`FeedbackIngestionPipeline` is a supervisor-hosted service in `nirman-control-plane` that coordinates the ten feedback sub-components below. It receives `FeedbackRecord` items from the UI via the `SupervisorConnection` (§57.3), persists them to the `feedback_records` ledger table through `EventStore` (§45.2), and routes each item to the appropriate sub-component based on `FeedbackRecord.kind`. It holds no authority; it is a routing coordinator.

> **Schema projection:** `FeedbackRecord` is defined in `nirman-schemas.md` §2.127. Owner: TA §44.3.3.

> **Schema projection:** `RequirementDelta` is defined in `nirman-schemas.md` §2.128. Owner: TA §44.3.3.

**Sub-components:**

`ChangeRequestParser` is a module in `nirman-kernel` that parses natural-language change requests submitted mid-session (distinct from initial goal creation handled by `GoalInterpreter`) into structured `RequirementDelta` records that are admitted by `GoalInterpreter` and routed into the active `TaskGraph` through `LifecycleAuthority`. It never mutates the task graph directly. A delta targeting an existing requirement MUST carry the observed `targetRequirementRevision`; stale revisions are rejected and re-read before reconsideration. `GoalInterpreter` classifies and routes the proposal but does not mutate canonical requirements: `ConstraintRegistry` alone admits the delta, increments the target revision or creates the admitted identity, updates applicable contract revisions, and returns `admittedRequirementId`/`admittedRequirementRevision`. Admission invalidates downstream implementation, scenario, evidence, proof, completion, preview, and artifact projections whose dependency set intersects the changed requirement revision; they must be regenerated or revalidated before completion. Rejection leaves canonical state unchanged.

`ScreenshotAnnotationIngestor` is a module in `nirman-control-plane` that ingests annotated screenshots (screenshots with user-drawn regions, labels, or text overlays) into `VisualSpecification` delta records consumed by the Visual QA Worker and requirements planner. Screenshot pixels are stored in the task directory; annotation metadata is persisted through `EvidenceAuthority`.

`BugReportReproConverter` is a module in `nirman-android` that converts an externally submitted bug description (stack trace, user-reported symptom, or Logcat excerpt) into a candidate `E2EScenario` reproduction spec (§62.1) suitable for the `Debugging Worker`. The converted scenario is a proposal only; `ScenarioSynthesizer` and `ToolBroker` govern its admission and execution.

`RatingSignalCollector` is a module in `nirman-control-plane` that collects explicit user rating signals — thumbs-up/thumbs-down, star ratings, or labelled satisfaction signals — and persists them as `FeedbackRecord.kind = RATING` items in the `feedback_records` table. Rating signals are attached to `EpisodeRecord` entries for self-improvement analysis (BS §28.3–BS §28.4). Rating signals are telemetry; they do not change task state or trigger autonomous actions.

`ImplicitDissatisfactionDetector` is a module in `nirman-control-plane` that monitors `EventStore` for revert operations, repeated re-requests of the same change, consecutive checkpoint restores, and rapid consecutive edits to recently generated code — patterns that indicate implicit user dissatisfaction without explicit rating. Detected patterns produce a `FeedbackRecord.kind = IMPLICIT_DISSATISFACTION` item and an advisory signal to `RecoveryAuthority`; they never autonomously revert work or alter the task state.

`FeedbackRequirementMapper` is a module in `nirman-kernel` that maps `FeedbackRecord` items to requirement nodes in the active `TaskGraph`. It uses the requirement-node dependency graph, the changed-file set, and the feedback content to associate feedback with the most specific applicable requirement. The mapping is a proposal routed through `GoalInterpreter`; `LifecycleAuthority` owns the requirement-node update.

`UserPreferenceLearner` is a module in `nirman-context` that extracts validated user preference facts — naming conventions, visual style choices, architectural preferences, communication style — from confirmed `EpisodeRecord` decisions and `FeedbackRecord.kind = CORRECTION` items. Extracted preferences must be validated against existing `ProjectMemoryStore` entries and may not contradict confirmed facts. Validated preferences are written to `ProjectMemoryStore` through `MemoryWriter` (§59.1). This module never writes preferences inferred from model summaries alone; durable user action or explicit confirmation is required provenance.

`CodeStyleExtractor` is a module in `nirman-android` that analyzes committed source history and confirmed correction feedback to extract code style conventions — formatting rules, naming patterns, import ordering, comment style, component structure — and populates style-constraint fields within `SkillPackage` style conventions (§19.1). Extracted conventions are proposed as `SkillPackage` candidates and must pass the full skill admission process (§19.1) before becoming active constraints. `CodeStyleExtractor` never modifies existing admitted skills directly.

`CorrectionMemoryStore` is a named partition of `ProjectMemoryStore` (§59.1, crate `nirman-context`) dedicated to user-confirmed correction facts: cases where the user explicitly rejected an agent output and provided the correct alternative. Every `CorrectionMemoryStore` entry must carry its source `FeedbackRecord.feedbackId`, the `EpisodeRecord.episodeId` of the task it corrects, and the original agent output. `MemoryWriter` applies the same validation rules as the parent `ProjectMemoryStore`; model-generated statements without durable user confirmation do not qualify as corrections.

`FeedbackTriagePrioritizer` is a module in `nirman-control-plane` that prioritizes pending `FeedbackRecord` items from the `FeedbackIngestionPipeline` queue before routing them to worker dispatch. Priority is computed from feedback kind (CORRECTION > IMPLICIT_DISSATISFACTION > RATING), recency, affected-requirement criticality, and frequency of co-occurring feedback. The prioritizer produces an ordered dispatch list; `TaskScheduler` and `PolicyAuthority` govern the actual worker dispatch.

#### 44.3.3.1 Requirement revision invalidation matrix

When `ConstraintRegistry` admits a mutation or revision to a `ConstructionRequirement`, the runtime traverses `RequirementToImplementationGraph` and dependent registries to invalidate downstream projections. The table below defines the exact invalidation cascade per modified dimension:

| Modified Requirement Dimension | Affected Projections & Artifacts Invalidated | Required Re-evaluation / Remediation |
|---|---|---|
| `statement` (functional description/scope) | Implementation mappings (`RequirementToImplementationGraph`), synthesis plans, generated source units, `Scenario` specifications, test cases, validation results, `EvidenceRecord`, `ProofSynthesis`, `CompletionDecision`, promoted APK or optional AAB | Regeneration of implementation mapping, code synthesis, full test re-execution, proof re-synthesis |
| `mandatory` (promoted to mandatory or relaxed) | Task graph scheduling priority, `CompletionDecision` predicates, validation gate thresholds | Re-evaluation of completion criteria; mandatory promotion blocks completion until satisfied |
| `acceptanceCriteria` (added, removed, or edited) | Target `ScenarioStep` definitions, verification assertions, test fixtures, `ValidationResult`, `EvidenceRecord`, `ProofSynthesis`, `CompletionDecision` | Scenario and assertion update, test re-execution, evidence re-collection |
| `sourceFeatureLineage` (feature association) | Feature trace links, UI hierarchy mapping, documentation references, `ArchitectureFitnessReport` | Re-index requirement-to-feature linkage and architecture fitness evaluation |
| `applicability` (target SDK/profile/flavor constraints) | Build variant configurations, device matrix targeting (`DeviceMatrixRiskProfile`), platform capability bindings | Re-evaluation of build gates, emulator device selection, variant compatibility |
| `privacyClassification` (data sensitivity change) | Data flow analyzers, permission requirements (`permissionInventory`), storage encryption checks, cleartext traffic policy | Security scan re-execution, permission audit, manifest inspection update |
| `integrationRequirement` (service endpoint/contract change) | Integration boundary bindings (`AndroidServiceIntegration`), `ContractDoubleScenario` fixtures, network security configs, mock state | Contract double update, integration test re-run, credential binding check |
| `deviceRequirement` (hardware/sensor/API requirements) | Emulator hardware profile, `AndroidDeviceProfile`, permission bindings, runtime sensor mocks | Device compatibility verification, emulator capability check, hardware API validation |

---

## 45. Reducer, Event Store, and Transaction Manager

### 45.1 Session reducer

`SessionReducer` is a pure function over validated events. It receives the previous state and an event, validates the transition, and returns the next immutable state. Side effects are emitted as commands for supervised handlers. `SessionReducer` is the implementation of `LifecycleAuthority` (§36.2, §57.2; ADR-066, ADR-159): it is the only component that commits a session-lifecycle (build spec §33.2) or task-execution (build spec §26.14) transition. Proposed transitions from the kernel's `AgentLoopReducer` (§58.2), workers, skills, hooks, or the UI arrive as events and are accepted or rejected here.

```text
Event received
  ↓
Schema validation
  ↓
Session/task/revision/lease validation
  ↓
Reducer transition
  ↓
Atomic durable state write
  ↓
Command dispatch, if required
```

The reducer rejects stale revisions, unknown entity IDs, expired worker leases, invalid completion events, evidence-free promotion, and impossible state transitions. Every rejected event is persisted as a policy or integrity event without changing the authoritative state.

### 45.2 Event store

The event store uses append-only records with monotonic sequence numbers, schema versions, timestamps, correlation IDs, actor type, and content hashes. It stores metadata and evidence references rather than raw secrets or unnecessary model content.

Local execution-ledger retention is owned by storage authority and is project-scoped and user-controlled. Provider-request provenance uses `SESSION`, `PROJECT`, or `USER_PINNED` local retention: session records may be pruned only after no active task, retry, reconciliation, evidence, or recovery record references them; project records survive session closure and are deleted with the project unless pinned by a user-approved export or legal policy; user-pinned records require an explicit user deletion action. Deletion emits an append-only provider-provenance tombstone event, projects `deletionStatus = DELETED` in the SQLite ledger, removes eligible retained metadata, and preserves only the minimum tombstone identities required to prevent dangling replay or false freshness. The original EventStore event is never rewritten; compaction may replace deletable event content only with the tombstone projection after the dependency checks above pass. A deletion request first projects `DELETION_REQUESTED`; it cannot reach `DELETED` while any `retentionDependencyRefs` identify an active task, pending retry, unresolved external effect, recovery dependency, evidence claim, usage attribution, or audit hold. Once all dependencies close, deletion removes eligible metadata and emits the tombstone. Credentials, sensitive headers, raw private reasoning, excluded content, and unnecessary provider payloads are forbidden at ingestion and therefore have no retention exception. Hashes inherit the source data's privacy classification and are not automatically safe to retain after source deletion. `ProviderContextEnvelope.retentionPolicy` continues to govern provider transmission/retention policy; it does not silently replace this local-ledger retention rule.

Required event families include session lifecycle, task graph and worker, lease, transaction, terminal and process health, provider request, toolchain, preview and device, validation and evidence, recovery and checkpoint, artifact and signing, and decision trace events.

**Authoritative committed-transaction event (ADR-251).** The `transaction` event family contains exactly one
event class that is authoritative for committed project state: the committed-transaction event. Its contract
is normative and exhaustive:

1. **Existence.** One committed-transaction event is appended for each `ConstructionTransaction` that
   reaches the atomic commit boundary and commits. Transactions that abort, roll back, or observe a zero
   project-state witness-set delta append no committed-transaction event (TA §45.3).
2. **Atomicity with the commit.** The committed-transaction event is appended in the same atomic durable
   write as the transaction's committed state transition — never before, never after. A committed
   transaction without its committed-transaction event, or a committed-transaction event without its
   transaction, is an integrity violation, not a representable state.
3. **Payload.** The event carries at minimum `transactionId`, `projectId`, the committed
   `projectRevisionAfter`, the base revision, the committed project-state witness set, and the correlation
   id of the session and task that produced it. It carries no raw workspace content.
4. **Sequence authority.** The event's monotonic sequence number is the sole authority for
   "authoritative commit-event sequence order". Commit order is never derived from wall-clock timestamps,
   from event-store insertion order alone, or from `projectRevisionId` ordering — revision ids are opaque
   (ADR-242).
5. **Project scoping.** The event's `projectId` is the only authority for attributing a commit to a
   project. The tip projection (TA §45.3) selects over committed-transaction events by project.
6. **Recovery and compaction provenance.** Compaction, snapshotting, checkpoint restore, and recovery
   replay MUST preserve the committed-transaction event (or an exact, verifiably equivalent record of it),
   including its sequence number, payload, and ordering relative to other committed-transaction events. A
   recovery or compaction step that loses, renumbers, reorders, duplicates, or synthesizes a
   committed-transaction event is an integrity violation.

The committed-transaction event is the event-store representation of the commit; it is not a second
authority alongside the committed state. The committed state transition and its committed-transaction
event are the same fact in two representations, and neither may be updated without the other.

Replay reconstructs session state and can optionally re-run validation commands against a checkpoint without re-running model generation.

### 45.3 ConstructionTransactionManager

A `ConstructionTransaction` is the atomic unit of project mutation: it carries one base revision, one staged candidate change set, one commit-or-rollback outcome, and — when it commits — exactly one minted project revision.

The manager creates a pre-mutation checkpoint, captures the project fingerprint and base revision, validates worker scope and operation capability, stages changes in a transaction workspace, runs syntax/graph/policy/mutation-safety checks, applies the candidate revision, re-indexes affected files, runs affected tests/build/preview checks, collects evidence, and commits or rolls back atomically.

Writes are serialized per project revision. Independent read-only analysis may proceed concurrently.

**Zero-delta termination (ADR-251).** The committed project-state witness set W is defined in ADR-242 as the
workspace file tree, the toolchain lock, and the dependency snapshot. At the atomic commit boundary the
manager computes the delta between the base witness set and the candidate witness set:

1. **Nonzero W delta** — the candidate witness set differs from the base in at least one of the three
   witness components. The transaction is eligible to commit and, if it commits, mints exactly one new
   `ProjectRevisionId` and appends exactly one committed-transaction event (TA §45.2).
2. **Zero W delta** — the candidate witness set is identical to the base in all three witness components.
   The transaction is a no-op: it terminates successfully as an explicit **no-op abort**. It mints no
   `ProjectRevisionId`, appends no committed-transaction event, and changes the project tip in no way.
3. **File-only delta is not the criterion.** A transaction that rewrites files without changing W as
   defined — for example, a rewrite that leaves the file tree, toolchain lock, and dependency snapshot
   semantically identical — is a no-op under this rule. Conversely, a transaction that changes the
   toolchain lock or dependency snapshot without touching the workspace file tree has a nonzero W delta and
   is eligible to commit. The witness set, not the presence of file edits, decides.
4. **Exactly one committed revision.** A committing transaction mints at most one `ProjectRevisionId`. A
   transaction that is eligible to commit and commits therefore produces exactly one committed revision,
   and consequently exactly one `ChangeReportRecord` (`CLAUSE.CHANGE.EXACTLY_ONE_REPORT`, BS §83).
5. **No-op creates no obligations.** A no-op abort creates neither a `ProjectRevisionId` nor the committed
   report obligation that attaches to a revision. It may still be recorded for diagnostics as a
   non-committed transaction outcome, and it is never a projection source.

A no-op abort is a normal, expected termination. It is not a failure, a validation defect, or a policy
violation, and it MUST NOT be surfaced as an error to the user.

At the atomic commit boundary of every project mutation, ConstructionTransactionManager mints a new
`ProjectRevisionId` (ADR-242) only when the transaction commits and its committed project-state
witness set — workspace file tree, toolchain lock, dependency snapshot — differs from the base witness
set; a transaction whose committed witness set equals its base witness set mints nothing, and so does
any transaction that aborts or rolls back, whatever its candidate witness set. The current project tip,
exposed to readers as `Project.currentRevision` (BS §82.1), is a deterministic storage-authority
projection equal to the `projectRevisionAfter` of the latest committed-transaction event in
authoritative commit-event sequence order (TA §45.2) selected by the predicate `event.projectId ==
project.id AND event is a committed-transaction event`; aborted, rolled-back, and no-op transactions
contribute no event and are never a projection source. The selection predicate is project-scoped and
status-scoped: a committed-transaction event of another project, and any event that is not a
committed-transaction event, are excluded by construction rather than by ordering accident. No separate
StorageAuthority component is introduced: the SQLite execution ledger (§57.5) remains the storage
authority, written only through `EventStore` and `ConstructionTransactionManager` (§21 mapping).

> **Schema projection:** `TaskRevision` is defined in `nirman-schemas.md` §2.122. Owner: TA §45.3.

`TaskRevision` (D3-K1; ADR-248) is the immutable revision identity of a task's authoritative semantic contract — objective/scope, dependencies and dependency-failure semantics, required inputs/outputs/capabilities, validation/acceptance requirements, join semantics, and other contract-defining fields. It is never task lifecycle state, worker attempt or lease, heartbeat or progress, retry count, evidence state, or execution epoch, and `taskRevisionId` is opaque: authority and freshness never derive from numeric ordering. `ConstructionTransactionManager` is the sole minter of `taskRevisionId`: a committed task-contract mutation atomically advances the applicable `TaskGraph.revision` and mints the affected task's new `taskRevisionId`; unaffected tasks retain their existing `TaskRevisionId`. A replan advances only tasks whose contract actually changes; migration classes (REBASE/REPLACE), task-state transitions, heartbeats, retries, evidence updates, lease/handoff, worker replacement, and execution-epoch rollover never advance a TaskRevision. The durable `task_revisions` row carries provenance (`createdByTransactionId`, `contractFingerprint`) as storage ground truth; by the same representation-versus-identity principle that governs `ProjectRevision` (ADR-242), the row is representation, not the semantic identity.

### 45.4 Commit barrier

`CommitBarrier` prevents conflicting worker writes. A proposal declares its base revision, touched paths, semantic symbols, dependencies, requirements, and expected outputs. The barrier compares the proposal against committed and pending proposals.

| Situation | Result |
|---|---|
| Disjoint files and compatible dependencies | May be queued for commit |
| Same file, non-overlapping structured symbols | Requires semantic merge validation |
| Same file, overlapping symbols | Reconciliation worker required |
| Stale base revision | Rebase or regenerate proposal |
| Conflicting technology plan | Architecture-plan review and checkpoint boundary |
| Conflicting permissions or signing policy | Policy authority decision required |

---

## 46. Lease and Capability Runtime

**Implements:** build spec §22 and `CONTRACT.RUNTIME.WORKSPACE` (with §8; the build spec section is the authority)

### 46.1 Session leases

A long-running autonomous session uses a renewable `SessionLease` rather than a fixed short execution token. The lease contains session ID, owner supervisor ID, issued time, expiry time, last progress time, heartbeat sequence, resource reservation, and revocation state.

The lease is renewed only when the supervisor observes valid progress, such as a committed event, active provider response, running build, device operation, recovery action, or explicitly permitted waiting condition. A spinner or repeated identical event is not progress.

On lease loss, the supervisor stops new work, revokes worker capabilities, preserves the workspace and event log, and resumes from the last durable checkpoint after restart.

### 46.2 Operation capabilities

Sensitive operations use a single-use `OperationCapability` bound to session and project IDs, worker and task IDs, operation type, workspace and path scope, base project revision, scope fingerprint, capability expiry, nonce, and consumption state.

The capability is consumed before network I/O or external side effects. It is never persisted in plaintext. A capability is invalid if the project fingerprint, revision, worker lease, or policy context changes.

Examples include dependency installation, emulator access, external network requests, signing, keystore use, writing outside generated source scope, and self-development promotion.

---

## 47. Project Ingestion, Fingerprinting, and Android Code Intelligence

**Implements:** build spec §5 and `CONTRACT.RUNTIME.SCOPE` (the build spec section is the authority)

### 47.1 Project ingestion pipeline

```text
Discover workspace → normalize root and exclusions → classify Android files
→ compute hashes and project fingerprint → build lightweight index
→ resolve toolchain and identity → full semantic graph before mutation
```

The ingestion service understands Kotlin, Java, XML, manifests, Gradle files, NDK/CMake native modules, resources, assets, SQL, JSON, YAML, TOML, lockfiles, signing configuration, emulator metadata, and test sources.

**Ingestion Stages:**

1. *Discovery and exclusion filtering:* Scans the project root while enforcing mandatory exclusion rules: `.gradle/`, `build/`, `.idea/`, `captures/`, Gradle daemon logs, local keystores (`*.jks`, `*.keystore`), and user credentials (`local.properties`). Excluded paths are excluded from AI model mutation and context assembly unless a transaction explicitly authorizes a privileged operation capability.
2. *File classification by Android role:*
   - `MANIFEST`: `AndroidManifest.xml` (root and flavor-specific overlays).
   - `BUILD_CONFIG`: `build.gradle.kts`, `build.gradle`, `settings.gradle.kts`, `gradle/libs.versions.toml`.
   - `SOURCE_KOTLIN`: `src/main/kotlin/**/*.kt`, `src/main/java/**/*.kt`.
   - `SOURCE_JAVA`: `src/main/java/**/*.java`.
   - `RESOURCE_LAYOUT`: `src/main/res/layout*/**/*.xml`.
   - `RESOURCE_VALUES`: `src/main/res/values*/**/*.xml` (strings, colors, dimensions, themes, styles).
   - `RESOURCE_DRAWABLE`: `src/main/res/drawable*/**/*` (vector drawables, shape XMLs, raster assets).
   - `RESOURCE_NAVIGATION`: `src/main/res/navigation*/**/*.xml`.
   - `TEST_UNIT`: `src/test/**/*.kt`, `src/test/**/*.java`.
   - `TEST_INSTRUMENTED`: `src/androidTest/**/*.kt`, `src/androidTest/**/*.java`.
3. *Content hashing and Merkle tree derivation:* Computes SHA-256 digests per classified file and builds an in-memory Merkle DAG for $O(\log N)$ change detection during worker transactions.
4. *Lightweight symbol extraction:* Runs language-specific Tree-sitter adapters to extract top-level declarations (packages, classes, methods, Composable functions, XML identifiers).
5. *Semantic graph assembly:* Reconciles identifiers across file boundaries into `AndroidSymbolGraph` before authorizing mutation planning.

### 47.2 Fingerprint model

The project fingerprint is a canonical hash over normalized relative paths, content hashes, selected metadata, technology-plan hash, toolchain-lock hash, and relevant external state. It is recomputed before every transaction commit, preview promotion, package operation, and self-development promotion.

TOCTOU protection rejects an operation when files changed outside the transaction, a worker’s base revision is stale, the selected toolchain changed, or the preview/device revision no longer corresponds to the candidate source.

The canonical project fingerprint calculation:
$$\text{ProjectFingerprint} = \text{SHA-256}\left(\bigoplus_{i=1}^{N} \text{Hash}\left(\text{Path}_i \parallel \text{ContentDigest}_i \parallel \text{Role}_i\right) \parallel \text{ToolchainLockHash} \parallel \text{TechnologyPlanHash}\right)$$
where relative paths are strictly POSIX-normalized and sorted lexicographically.

### 47.3 Language adapter interface

> **Schema projection:** `AndroidLanguageAdapter` is defined in `nirman-schemas.md` §2.39. Owner: TA §47.3.

Adapters are selected by file type and technology plan. No single parser is mandatory for every Android project.

- `AndroidSymbolGraph` — The bi-directional code-intelligence graph connecting declarative Android resources with imperative source code.

**Multi-language Tree-sitter AST parsers.** The Rust control plane embeds lightweight, fast Tree-sitter grammars rather than executing JVM-based compiler frontends during ingestion and analysis:
- `tree-sitter-kotlin` for Kotlin source and Kotlin DSL (`build.gradle.kts`, `settings.gradle.kts`).
- `tree-sitter-java` for legacy Android Java source.
- `tree-sitter-xml` for `AndroidManifest.xml`, layout XMLs, values XMLs, and navigation graphs.

**Static AST extraction queries:**
- *Kotlin declarations & annotations:* Extracts class declarations, companion objects, function declarations, property definitions, and annotations (`@Composable`, `@Entity`, `@Dao`, Hilt `@ViewModel`, `@Inject`).
- *XML element & attribute extraction:* Statically extracts tags, `android:id` definitions (`@+id/name`), layout inclusion (`<include layout="@layout/name">`), string keys (`<string name="key">`), and theme attributes.
- *Static Gradle DSL extraction:* Queries `call_expression` AST nodes matching `plugins`, `dependencies`, `android`, `defaultConfig`, and `buildTypes` without running a JVM process or Gradle daemon:
  ```text
  (call_expression
    (identifier) @block_name (#match? @block_name "^(dependencies|plugins|android)$")
    (lambda_literal (statements) @body))
  ```
  Extracts dependencies (`implementation`, `api`, `ksp`, `testImplementation`), SDK constraints (`compileSdk`, `minSdk`, `targetSdk`), and application ID statically, safely, and instantaneously.

**Cross-language symbol linking algorithm.** The runtime maintains a bi-directional `AndroidSymbolGraph` connecting declarative Android resources with imperative source code:

1. *Node classification:*
   - `ResourceSymbolNode`: ID (`@+id/save_button`), String (`@string/app_name`), Drawable (`@drawable/ic_check`), Layout (`@layout/fragment_detail`).
   - `CodeSymbolNode`: Kotlin/Java class, function, property, ViewBinding reference (`binding.saveButton`), Compose test tag (`Modifier.testTag("save_button")`), or R-class reference (`R.id.save_button`, `R.string.app_name`).
   - `ManifestComponentNode`: Activity, Service, BroadcastReceiver, ContentProvider, Permission request (`<uses-permission>`), IntentFilter (Action, Category, Data).
2. *Edge relationships:*
   - `DECLARES_RESOURCE`: Links an XML file to the resources it creates.
   - `BINDS_VIEW`: Links Kotlin/Java ViewBinding access to the corresponding XML `@+id`.
   - `TEST_TAG_MATCH`: Links Compose `Modifier.testTag("id")` to `ScreenModel` element queries.
   - `REFERENCES_RESOURCE`: Links code expressions (`R.string.foo`, `stringResource(R.string.foo)`) to XML value definitions.
   - `DECLARES_COMPONENT`: Links `AndroidManifest.xml` `<activity android:name=".MainActivity">` to the concrete Kotlin class symbol.
   - `REQUIRES_PERMISSION`: Links API calls (e.g. FusedLocationProviderClient) to the required Android permission declaration in the manifest.

### 47.4 Impact analysis

The graph service calculates affected files, modules, resources, permissions, tests, emulator profiles, preview surfaces, and artifact outputs. The affected-test set is persisted with each transaction and evidence record, so long-horizon sessions can validate changed behavior without rebuilding unrelated areas unnecessarily.

**Bi-directional mutation impact analysis:**
1. *Forward impact propagation:* When a transaction modifies symbol node $S$, the graph service computes the reflexive transitive closure of dependent nodes along `BINDS_VIEW`, `REFERENCES_RESOURCE`, and `CALLS` edges. This identifies all files requiring recompilation validation and all Composable functions or View classes requiring preview invalidation.
2. *Targeted test set derivation:* Filters `TEST_UNIT` and `TEST_INSTRUMENTED` test cases: only tests that exercise the forward impact set of mutated symbols are scheduled for execution. Tests with zero dependency paths to modified nodes remain valid from their prior cached evidence watermark.
3. *Premise verification:* Compares the proposal's premise set against the current `AndroidSymbolGraph`. If an agent proposes a change based on a symbol signature or XML ID that changed in a preceding transaction, the mutation is immediately rejected as `PREMISE_MISMATCH` before workspace mutation opens.

- `AndroidAntiPatternDetector` — The static AST analysis service that detects prohibited Android and Jetpack Compose anti-patterns before commit.
- `AndroidApiLevelValidator` — The static AST analysis service that verifies API calls against minSdk constraints.
- `AndroidCircularDependencyDetector` — The graph analysis service that detects cycles across Gradle modules, dependency injection graphs, and database entity relationships.
- `AndroidDataFlowAnalyzer` — The static analysis service that computes intra-procedural control flow and taint flow across Android source files.
- `AndroidPatternLibrary` — The local offline repository of canonical Android Jetpack implementation patterns.
- `AndroidRefactoringPipeline` — The AST-guided refactoring service that executes dead resource elimination, deprecated API migration, and Compose state hoisting.
- `ArchitectureDriftDetector` — The static AST analysis service that detects Clean Architecture layer boundary violations between UI, ViewModel, and Data layers.
- `CodeDuplicationDetector` — The AST clone detection service that identifies duplicated Composable UI trees and business logic algorithms using structural fingerprints.
- `CrashPatternAnalyzer` — The runtime crash analysis service that correlates Logcat stack traces with source symbol anchors and episodic repair patterns.
- `DocCodeMismatchDetector` — The documentation consistency service that verifies KDoc and Javadoc tags against Tree-sitter AST declarations.
- `ProjectReadmeSynthesizer` — The documentation engine that generates complete, truthful README.md files for exported Android projects.
- `SemanticCodeFingerprintEngine` — The AST normalization service that computes syntax-invariant structural hashes for methods, classes, and source files.

**Static Android API level and minSdk validation.** To prevent fatal runtime crashes (`NoSuchMethodError`, `ClassNotFoundException`) on older devices and emulators, `AndroidApiLevelValidator` enforces static API level compliance:
1. Resolves active `minSdk` and `compileSdk` from the project's `AndroidToolchainLock`.
2. Walks the Tree-sitter AST of all Kotlin and Java methods, properties, and class declarations.
3. Cross-references invoked Android framework methods against the local SDK API level index.
4. If a method requires an API level higher than `minSdk`, verifies that the AST node is enclosed in an explicit version check (`if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.X)`) or guarded by `@RequiresApi`.
5. Any unguarded invocation is flagged with pre-compilation diagnostic `API_LEVEL_UNGUARDED_CALL` and rejected at pre-commit.

**Offline Android pattern and snippet retrieval.** The supervisor embeds `AndroidPatternLibrary` directly within `nirman-supervisor`:
- Supplies verified, canonical code templates for every component in the closed-world decision matrix of §73.2 (Room DAOs with KSP, Jetpack Compose Navigation 2.8+ type-safe routes, WorkManager periodic workers, Material 3 Scaffolds).
- Operates 100% locally on the user's Windows host, enabling workers to synthesize compliant, idiomatic Android architectures even during severed network connectivity (`SessionProviderMode.OFFLINE`).

**AST-guided Android refactoring pipeline.** When executing architectural refactoring or code cleanup under the `Android Platform Worker` role, `AndroidRefactoringPipeline` performs three deterministic AST transformations without relying on full JVM re-compilation:
1. *Dead resource & symbol pruning:* Identifies zero-in-degree nodes in `AndroidSymbolGraph` (e.g. unused `@string`, `@color`, `@drawable` XML entries, or unreferenced private Composable helper functions) and stages atomic deletion patches.
2. *Deprecated API & lifecycle migration:* Rewrites legacy Android patterns to modern Jetpack equivalents: transforms legacy `findViewById` or synthetic bindings into ViewBinding / Compose; upgrades deprecated Accompanist insets to AndroidX Window insets (`WindowInsets.safeDrawing`); and replaces unsafe `GlobalScope` / unbonded coroutines with `viewModelScope` and `repeatOnLifecycle`.
3. *Compose state hoisting normalization:* Scans `@Composable` functions containing internal `remember { mutableStateOf(...) }` declarations that violate unidirectional data flow; automatically refactors them into stateless Composables by hoisting the state value to parameters and exposing typed event lambdas (`(Value) -> Unit`) to the parent caller.

- `EpisodicRepairPatternCatalog` — The supervisor-owned catalog of validated, cross-session AST repair patterns indexed by compilation and runtime error signatures.

**Episodic repair pattern catalog.** To accelerate autonomous repair loops without recurrent model inference, `EpisodicRepairPatternCatalog` durably indexes verified AST patch transformations:
1. *Pattern representation:* Stores structured `(failureSignature, astPatchTemplate, verificationRule, targetJetpackVersion, minSdk)` tuples derived from previously successful metamorphic test episodes. In compliance with ADR-218 and BS §66, no raw chain-of-thought tokens or unverified model summaries are retained.
2. *Deterministic pre-inference lookup:* When a build compilation or emulator test fails, the `Debugging Worker` queries the catalog using Tree-sitter error classification before dispatching a model deliberation request. An exact match on AST error signature and `minSdk` compatibility applies the proven patch template directly under a speculative branch.
3. *Validation gating and quarantine:* Applied catalog patches must pass complete test and emulator validation before promotion. Any catalog pattern that causes an unexpected regression or fails validation on a target project is immediately marked `QUARANTINED` in the local ledger and removed from the active lookup index.

**Intra-procedural control flow and taint flow analysis.** Within `nirman-supervisor`, `AndroidDataFlowAnalyzer` extends Tree-sitter AST traversal with lightweight intra-procedural control flow graphs (CFGs) and data flow taint tracking:
1. *Lifecycle and coroutine flow validation:* Constructs CFGs for `@Composable` functions, Activities, and Fragments to verify that asynchronous coroutine flows, channel collections, and StateFlow emissions are strictly bound to lifecycle-aware scopes (`repeatOnLifecycle`, `collectAsStateWithLifecycle`, or `viewModelScope`). Any unconstrained collection inside an event branch or recomposition loop without lifecycle gating is flagged as a potential memory leak or background execution hazard before compilation.
2. *Local taint tracking and credential protection:* Traces data flow from sensitive sources (e.g. Android Keystore, user input text fields, biometrics) to external sinks (e.g. unencrypted SharedPreferences, cleartext HTTP loggers, intent bundles). Hardcoded API secrets, auth tokens, or private signing keys discovered in AST literals or flowing into unencrypted local persistence are rejected with pre-commit diagnostic `TAINT_SENSITIVE_LEAK`.
3. *Exception and branch exhaustiveness:* Analyzes CFG branches across sealed classes, enum switches, and Android permission request results to guarantee exhaustive handling and prevent silent branch drops.

**Semantic code fingerprinting and normalization.** To eliminate no-op edits and index structural code artifacts across worker mutations, `SemanticCodeFingerprintEngine` operates on normalized AST representations:
1. *Syntax-invariant normalization:* Normalizes Kotlin and Java AST nodes by stripping comments, formatting whitespace, import re-orderings, and local variable identifier renamings into canonical structural form ($H_{\text{ast}}$).
2. *Semantic no-op elimination:* Before staging a file mutation, compares the candidate AST fingerprint against the baseline AST fingerprint. If the semantic structural hash is identical despite whitespace or superficial formatting churn, the supervisor cancels the redundant write, avoiding superfluous build triggers and emulator reload cycles.
3. *Structural patch indexing:* Supplies deterministic method-level and class-level fingerprints to `EpisodicRepairPatternCatalog` and the supervisor's SQLite execution ledger, enabling instant retrieval of historically verified AST repair patterns across diverse projects and file organizations.

**Static anti-pattern and Compose code smell detection.** To enforce the closed-world decision matrix of §73.2 and maintain Android runtime performance, `AndroidAntiPatternDetector` statically scans Tree-sitter AST nodes prior to transaction staging:
1. *Prohibited pattern enforcement:* Detects and rejects legacy or dangerous constructs prohibited by §73.2 (`AsyncTask`, raw `Thread`/`Handler` allocations, direct `SQLiteOpenHelper` subclasses, `ProgressDialog`, and exported components without explicit `android:exported` attributes).
2. *Compose state allocation anti-patterns:* Flags calls to `mutableStateOf(...)` or expensive allocations (e.g. bitmap resource decoding, collection transformations, regex compilations) inside `@Composable` functions that are not wrapped in `remember` or `rememberSaveable`, preventing severe frame-rate degradation and infinite recomposition loops.
3. *Unstable parameter detection:* Scans Composable parameter types against Compose stability invariants, recommending immutable collection wrappers or `@Stable`/`@Immutable` annotations when unstable types cause unnecessary recompositions.

**Circular dependency detection.** To prevent cyclic build deadlocks and runtime initialization crashes, `AndroidCircularDependencyDetector` constructs directed dependency graphs across three Android architectural planes:
1. *Multi-module Gradle graph:* Builds a directed graph of project dependencies (`implementation(project(":feature"))`) across `build.gradle.kts` files and executes Tarjan's strongly connected components algorithm to identify and reject inter-module cycles before Gradle invocation.
2. *Dependency injection graph:* Traverses Dagger/Hilt `@Inject` constructors and `@Provides` module bindings to verify acyclic object graph construction, reporting pre-commit diagnostic `CIRCULAR_INJECTION_DEPENDENCY` on detected cycles.
3. *Relational schema graph:* Inspects Room `@Entity` relations and `@ForeignKey` constraints to guarantee acyclic entity dependency hierarchies, preventing cascading delete deadlocks.

**Documentation consistency and doc-code mismatch detection.** Within `nirman-supervisor`, `DocCodeMismatchDetector` ensures that codebase documentation faithfully matches actual implementation:
1. *Tag-to-signature reconciliation:* Compares KDoc and Javadoc `@param`, `@return`, and `@throws` tags against the corresponding Tree-sitter AST method signatures. If a parameter is renamed, removed, or added without updating the documentation comment, the analyzer flags a `DOC_CODE_MISMATCH` diagnostic.
2. *Type and visibility consistency:* Verifies that documented types and exception classes exist in the current project classpath and that private helper details are not exposed in public KDoc contracts.

**Project README synthesis.** During export preparation under `Documentation Worker`, `ProjectReadmeSynthesizer` generates a truthful, deterministic `README.md` at the project root:
1. *Metadata grounding:* Derives project name, package name, `minSdk`, `targetSdk`, and toolchain versions directly from `AndroidToolchainLock` and `AndroidManifest.xml`, strictly forbidding fabricated build parameters.
2. *Build and execution commands:* Emits exact local Gradle wrapper commands (`./gradlew assembleDebug`, `./gradlew test`) corresponding to the project's verified configuration.
3. *Architecture summary:* Summarizes implemented screens, Room database entities, and background workers based strictly on verified `CapabilityRegistry` evidence.

**Architecture boundary enforcement and drift detection.** Within `nirman-supervisor`, `ArchitectureDriftDetector` statically verifies that Android source code strictly adheres to Clean Architecture layer separation before staging transactions:
1. *UI to data layer isolation:* Analyzes Tree-sitter AST call expressions in `@Composable` functions and Android UI classes (Activities, Fragments, Custom Views) to guarantee they do not invoke Room DAOs, SQLite queries, or Retrofit/Ktor network interfaces directly. All data access must be mediated through a lifecycle-managed `ViewModel` exposing observable state.
2. *Context and lifecycle leak prevention:* Inspects `ViewModel` class fields and constructor parameters to verify they do not retain references to Android `Context`, `Activity`, `Fragment`, or `View` objects. Any detected UI reference is rejected with pre-commit diagnostic `ARCHITECTURE_VIEWMODEL_CONTEXT_LEAK`.
3. *Data layer immutability:* Verifies that Repository classes expose read-only `Flow` or `StateFlow` streams to consumers rather than mutable `MutableStateFlow` or `MutableLiveData` references, preventing uncontrolled state mutation across architectural layers.

**AST-guided code duplication detection.** To maintain maintainability and eliminate redundant code across autonomous synthesis runs, `CodeDuplicationDetector` operates across the project's Tree-sitter AST:
1. *Composable UI clone detection:* Compares normalized sub-tree structural fingerprints generated by `SemanticCodeFingerprintEngine` across `@Composable` functions. When duplicate layout structures (e.g. repeated Card hierarchies, form input fields, or dialog scaffolds exceeding a configured structural node threshold) are detected, the detector flags an `AST_DUPLICATE_COMPOSABLE` finding.
2. *Business logic algorithm clone detection:* Identifies identical expression and statement sub-trees across Repository and UseCase implementations, accounting for parameter renamings while asserting structural equivalence.
3. *Refactoring extraction recommendation:* Synthesizes refactoring proposals for `AndroidRefactoringPipeline` to extract duplicated sub-trees into parameterized reusable Composable functions or shared Kotlin extension utilities.

**Runtime crash pattern analysis.** Working in close coordination with `RuntimeTraceAnalyzer`, `CrashPatternAnalyzer` processes raw emulator Logcat and Android runtime crash reports:
1. *Crash dump ingestion and frame extraction:* Normalizes `FATAL EXCEPTION`, ANR traces, `NullPointerException`, `IndexOutOfBoundsException`, and unhandled coroutine exception blocks from the emulator log stream, extracting the fully qualified class names, method names, and line numbers of the failure stack frames.
2. *Symbol graph line anchor correlation:* Queries `AndroidSymbolGraph` to map the crash stack frames directly to active source file paths and AST node declarations, resolving generated class names (e.g. Composable lambdas, coroutine continuations) back to original source constructs.
3. *Episodic catalog matching:* Matches the structured crash signature and active `minSdk` against `EpisodicRepairPatternCatalog` to retrieve validated AST repair transformations, enabling zero-inference speculative repair of recurring runtime defects before invoking model deliberation.

**Placeholder residue detection.** Within `nirman-supervisor`, `PlaceholderResidueDetector` statically scans proposed source code mutations, string resources, and layout templates before transaction staging:
1. *Code placeholder pattern matching:* Walks the Tree-sitter AST to identify unexpanded stub markers, including `TODO`, `FIXME`, calls to standard library stubs (`TODO()`, `error("Not implemented")`), hollow exception throws (`throw NotImplementedError()`, `throw UnsupportedOperationException()`), and empty method bodies returning default dummy literals.
2. *Resource placeholder scanning:* Inspects XML resource files (`strings.xml`, `arrays.xml`) for filler text patterns (`Lorem ipsum`, `Sample Text`, `Placeholder`, `Title here`, `lorem_ipsum`).
3. *Pre-commit rejection:* Emits a `PLACEHOLDER_RESIDUE_DETECTED` finding and rejects transaction staging, preventing incomplete or hollow code from reaching compilation or evidence ledger records.

**Truncated file detection.** Working in close coordination with Tree-sitter AST validation, `TruncatedFileDetector` guards against incomplete or cut-off model completions:
1. *Syntactic delimiter balance:* Analyzes source code for unbalanced braces, parentheses, or brackets, unclosed triple-quoted string literals, and unclosed KDoc comment blocks.
2. *Grammar error node verification:* Scans Tree-sitter parse trees for top-level `ERROR` nodes positioned at end-of-file indicative of abrupt stream truncation.
3. *Immediate recovery trigger:* Flags a `FILE_TRUNCATION_DETECTED` defect, preventing partial file writes from corrupting the workspace, and instructs the kernel to request continuation or clean re-synthesis before attempting compilation.

**Mock and residual double detection.** Working before release packaging and final validation gates, `MockResidualDetector` protects production release integrity:
1. *Production source set isolation:* Scans `src/main/` source trees for test doubles, in-memory mock repositories, hardcoded dummy lists, and test-only bypass logic.
2. *Double binding verification:* Verifies that dependency injection modules (`@Module`, `@InstallIn(SingletonComponent::class)`) in production source sets bind to concrete Room databases, DataStore preferences, and actual network clients rather than in-memory fakes.
3. *Gated double authorization:* Rejects any mutation introducing unauthorized test doubles into production sets with `UNAUTHORIZED_MOCK_RESIDUAL`, ensuring mock doubles are strictly confined to `src/test/`, `src/androidTest/`, or explicitly declared `ContractDouble` boundaries (ADR-225).

**Room schema and migration safety analysis.** To prevent runtime SQLite crashes and data loss across app upgrades, `RoomSchemaMigrationAnalyzer` validates Room entity schemas and database evolution:
1. *Schema diffing and version consistency:* Statically parses exported Room schema JSON files (`schemas/com.example.AppDatabase/N.json`) across version increments, comparing table structures, column types, nullability, primary key constraints, and indices against the compiled `@Database(version = ...)` definition.
2. *Destructive migration detection:* Flags any unmanaged schema drift where columns or tables were altered or dropped without an explicit `AutoMigration` specification, a registered `Migration(from, to)` implementation, or an authorized destructive migration policy.
3. *Foreign key and constraint verification:* Verifies that foreign key references (`@ForeignKey`) target valid parent entity primary keys and specify explicit cascading policies (`onDelete`, `onUpdate`) compatible with SQLite constraints.
4. *Room migration path matrix:* For each supported schema version, `RoomSchemaMigrationAnalyzer` validates all migration paths across version increments (`oldVersion → every legal intermediate path → currentVersion`). The matrix checks: (a) retained rows across table alterations, (b) transformed rows and column mapping correctness, (c) default values on newly added non-null columns, (d) foreign key referential integrity across migrated tables, (e) index re-creation and coverage, (f) nullability constraint preservation, (g) transaction rollbacks during interrupted migrations, (h) detection and handling of corrupt database files, (i) rejection of unsupported database downgrades, and (j) prohibition of destructive migrations (`fallbackToDestructiveMigration`) unless explicitly authorized in the construction contract.

**Room query performance and safety analysis.** To eliminate database latency bottlenecks and runtime vulnerabilities, `QueryPerformanceAnalyzer` statically inspects Room DAO interfaces:
1. *N+1 query pattern detection:* Scans DAO method signatures and usage call-graphs in ViewModels and Repositories to detect N+1 query patterns where child entities are fetched iteratively in a loop rather than using composite Room `@Relation` embeddings or parameterized `IN (:ids)` batch queries.
2. *Index coverage and table scan prevention:* Correlates query `WHERE`, `JOIN`, and `ORDER BY` clauses against entity `@Index` definitions to flag queries that would trigger unindexed SQLite full table scans on large datasets.
3. *Injection safety and parameter binding:* Validates that dynamic raw queries (`@RawQuery`, `SupportSQLiteQuery`) use parameterized bindings (`?` or `:arg`) rather than string concatenation, rejecting unsafe SQL construction before transaction commit.

**Offline-first synchronization protocol planning.** To guarantee data consistency in disconnected or intermittent network conditions, `OfflineSyncProtocolPlanner` verifies persistence and sync architecture:
1. *Reactive single-source-of-truth verification:* Confirms that local Room DAOs and Repositories expose reactive `Flow<T>` streams or AndroidX paging data sources to UI layers, ensuring the local database acts as the single source of truth.
2. *Outbox pattern and retry queueing:* Statically verifies the presence of an Outbox persistence entity and scheduled Android Jetpack WorkManager tasks with exponential backoff retry policies for reliable background synchronization.
3. *Conflict resolution verification:* Ensures data repositories define explicit deterministic conflict resolution policies (such as Last-Write-Wins based on synchronized timestamps or server-authoritative reconciliation) for bidirectional sync operations.
4. *Offline synchronization conflict matrix:* To evaluate repository resilience under unreliable connectivity, `OfflineSyncProtocolPlanner` executes a deterministic conflict matrix verifying client handling for: (a) local update vs remote update, (b) deleted locally but modified remotely, (c) duplicate outbox event replay, (d) lost server acknowledgements, (e) clock skew between device and server timestamps, (f) authentication credential expiry during active batch sync, (g) network reconnect mid-batch, (h) idempotency-key reuse across retried requests, and (i) Android process termination during replay. The generated application's declared contract determines the conflict resolution strategy (e.g. Last-Write-Wins, field-level merge, server-authoritative, or user prompt), without mandating CRDTs or multi-master replication.

**Structural three-way AST merge.** When autonomous background generation runs concurrently with user manual edits in the built-in code editor, `ThreeWayAstMergeEngine` reconciles the changes without broad file rewrites:
1. *Three-way AST diffing:* Parses the common base revision ($Base$), the user's manual modifications ($Ours$), and the model-proposed generation ($Theirs$) into Tree-sitter ASTs, mapping edits to specific syntax nodes (declarations, imports, statements).
2. *Non-conflicting integration:* Automatically merges modifications when $Ours$ and $Theirs$ touch disjoint AST declarations or non-overlapping function/class bodies.
3. *Conflict containment:* When both sides mutate the identical AST expression or statement, isolates the conflicting region with explicit conflict markers and surfaces an Action Center decision card, preserving user edits deterministically (BS §4.5; ADR-251).

**Regeneration-safe zone protection.** To allow users to write custom native code immune to agent overwriting, `RegenerationSafeZoneMarker` statically enforces boundary invariants:
1. *Boundary parsing:* Scans Kotlin, Java, and XML source files for declared safe-zone comments (`// nirman:protected-start` ... `// nirman:protected-end`) and `@NirmanProtected` annotations.
2. *Pre-transaction violation check:* Validates every incoming `StructuredPatch` against the active protected line ranges; any patch that modifies, moves, or deletes tokens within a protected zone is rejected with `PROTECTED_ZONE_VIOLATION` before staging.

**Fast micro-loop incremental compilation and validation.** To provide the sub-5-second feedback required by BS §52.3's Tier 1 Inner Micro-Loop, `MicroLoopValidator` gates proposed mutations before transaction staging:
1. *In-memory AST validation:* Validates syntax, brace matching, and Compose annotations using Tree-sitter parsers, rejecting malformed constructs with instant syntax diagnostics.
2. *Incremental module compiler dry-run:* Invokes an isolated, non-packaging Gradle compilation check (`compileDebugKotlin` targeting strictly the affected module) to verify type correctness, import resolution, and symbol bindings without packaging APKs or touching emulator runtimes.
3. *Micro-fail-fast dispatch:* Emits a fingerprinted `MICRO_COMPILATION_ERROR` or `AST_VALIDATION_ERROR` and halts Tier 1 execution within 5 seconds on defects, handing diagnostic feedback back to the agent before disk staging or downstream invalidation occurs.

---

### 47.5 Android Code Intelligence, Architecture Reasoning, Generation Intelligence, and Data Intelligence Services

**Role:** aggregate query facade and static what-if analysis — read-only services; no authority, no AI-usage budget.

#### 47.5.1 AndroidCodeIntelligenceService

`AndroidCodeIntelligenceService` is the supervisor-owned, read-only aggregate service that exposes a typed query interface over `AndroidSymbolGraph` (§47.3), `SemanticCodeFingerprintEngine` (§47.4), `EpisodicRepairPatternCatalog` (§47.4), and the project `ImpactGraph` (BS §43.3) to the `AgentExecutionKernel` and registered IPC command handlers. It does not own any of those components; it routes queries to them under their existing authorities. It is the implementation of the code-intelligence query surface referred to in BS §43.1.

Responsibilities:
- Accepts typed code-intelligence queries (symbol lookup, cross-reference expansion, fingerprint retrieval, impact-scope expansion, repair-pattern lookup, analysis scheduling) from kernel workers and registered frontend IPC commands.
- Routes each query to the authoritative component (`AndroidSymbolGraph`, `SemanticCodeFingerprintEngine`, `EpisodicRepairPatternCatalog`, or the impact-graph traversal of §47.4) and returns a typed read-only result.
- Never writes authoritative state; all mutations pass through the `MutationBroker` and `ConstructionTransactionManager` as governed by BS §43.2.
- Analysis scheduling (code-analysis scan requests, incremental index refresh) is delegated to `TaskScheduler` (§7.1) and does not bypass it.

#### 47.5.2 AndroidArchitectureReasoningService

`AndroidArchitectureReasoningService` is the static architectural what-if analysis service. It performs bounded `AndroidSymbolGraph` traversal and `ImpactGraph` expansion to compute a hypothetical impact surface — affected files, modules, resources, tests, and evidence — for a proposed architectural change *before* any `ConstructionTransaction` opens. It is the implementation of the architectural simulation and change-impact analysis surface for Android-scoped patterns (BS §43.3, BS §66.4).

Responsibilities:
- Accepts an architectural change hypothesis (a proposed mutation surface: files, symbols, module boundaries, or layer transitions) and traverses the `AndroidSymbolGraph` and `ImpactGraph` to compute the set of affected source files, resource files, test identifiers, evidence records, and preview surfaces.
- Reports the projected impact as a read-only `ArchitecturalImpactProjection`; this projection is advisory and carries no authority over evidence or completion.
- Detects Clean Architecture layer boundary violations, circular dependency risks, and anti-pattern introduction *before* a transaction, by querying `ArchitectureDriftDetector` and `AndroidAntiPatternDetector` (§47.4) in read-only mode.
- Never opens a `ConstructionTransaction`, mutates project state, or grants permissions.

#### 47.5.3 AndroidGenerationIntelligenceService

`AndroidGenerationIntelligenceService` is the supervisor-owned, read-only aggregate query facade that unifies code generation pattern retrieval, placeholder detection, syntactic truncation verification, mock residue detection, and API level safety. It exposes a typed query surface to code generation workers (`UI Worker`, `Android Data and Integration Worker`, `Android Platform Worker`) and registered IPC command handlers.

`AndroidGenerationIntelligenceService` coordinates five deterministic analytical and pattern components:
1. *Idiomatic snippet retrieval:* Queries `AndroidPatternLibrary` (§47.4) and `AndroidDomainKnowledgeCatalog` (§73.15.5) for compliant Room DAOs, Compose layouts, and Navigation 2.8+ type-safe routes.
2. *Placeholder residue verification:* Invokes `PlaceholderResidueDetector` (§47.4) to guarantee generated source files contain no unexpanded `TODO`, `FIXME`, or `Lorem ipsum` literals.
3. *Syntactic continuity verification:* Invokes `TruncatedFileDetector` (§47.4) to ensure proposed completions are syntactically complete without premature EOF truncation.
4. *API level and anti-pattern enforcement:* Queries `AndroidApiLevelValidator` (§47.4) and `AndroidAntiPatternDetector` (§47.4) to verify `minSdk` compatibility and prevent banned Android constructs (`AsyncTask`, unremembered `mutableStateOf`).
5. *Mock residue scanning:* Queries `MockResidualDetector` (§47.4) before release packaging to prevent test doubles and fake in-memory repositories from leaking into production source sets.

`AndroidGenerationIntelligenceService` creates no second authority. It does not directly mutate project source or bypass policy; all generation proposals route through `MutationBroker` (BS §43.2) and commit via `ConstructionTransaction` (BS §42.2). `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

#### 47.5.4 AndroidDataIntelligenceService

`AndroidDataIntelligenceService` is the supervisor-owned, read-only aggregate query facade that unifies schema design validation, Room migration verification, query performance inspection, and offline-first synchronization planning. It exposes a typed query surface to data engineering workers (`Android Data and Integration Worker`, `Backend & Service Engineering Worker`) and registered IPC command handlers.

`AndroidDataIntelligenceService` coordinates four deterministic analytical and pattern components:
1. *Domain entity and schema synthesis:* Queries `AndroidDomainKnowledgeCatalog` (§73.15.5) and `AndroidPatternLibrary` (§47.4) for idiomatic Room entity models, TypeConverters, and DataStore schemas.
2. *Migration safety and schema diffing:* Invokes `RoomSchemaMigrationAnalyzer` (§47.4) to verify database schema version transitions, validate `Migration` implementations, and ensure zero unmanaged data loss.
3. *Query optimization and index analysis:* Invokes `QueryPerformanceAnalyzer` (§47.4) to eliminate N+1 query patterns, recommend composite indices, and prevent SQL injection.
4. *Offline sync and outbox verification:* Invokes `OfflineSyncProtocolPlanner` (§47.4) to validate reactive Flow repositories, WorkManager background synchronization, and conflict resolution policies.

`AndroidDataIntelligenceService` creates no second authority. It does not directly mutate project source or bypass policy; all data layer mutations route through `MutationBroker` (BS §43.2) and commit via `ConstructionTransaction` (BS §42.2). `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

---

## 48. Provider Bridge and ModelGateway

### 48.1 Provider bridge lifecycle

The provider bridge, whether implemented inside the Rust backend or as a separately supervised local process, follows this lifecycle:

```text
STARTING → HANDSHAKING → HEALTHY
                         ├── DEGRADED
                         ├── RESTARTING
                         └── OFFLINE
```

The handshake validates protocol version, session authentication, provider profile identity, model capabilities, context limit, supported input modalities, tool-call format, and response normalization.

### 48.2 Request contract

Every provider request includes session ID, task ID, worker ID, trace ID, provider profile ID, model ID, protocol family, context classification, tool policy, maximum context limit, cancellation token, and privacy classification. The bridge strips secrets from logs and rejects unknown or unapproved tool calls.

The gateway normalizes Chat Completions, Responses-style, and message-oriented providers into one internal representation containing text blocks, image blocks, tool calls, tool results, structured output, usage, finish reason, and retryability. The protocol family is resolved FROM the declared compatibility mode per ADR-208, not chosen independently by a worker or model.

### 48.3 Provider failure behavior

| Failure | Runtime behavior |
|---|---|
| Timeout | Cancel request, record evidence, retry under provider policy |
| Rate limit | Honor retry-after, reduce concurrency, preserve session lease |
| Authentication failure | Enter provider-blocked state; never loop blindly |
| Unsupported capability | Select approved alternate profile or change task strategy |
| Bridge crash | Restart bridge, re-handshake, resume from durable request boundary |
| Network unavailable | Enter offline mode while preserving local project and history |
| Malformed response | Reject as untrusted output and request a fresh structured response |

---

## 49. Android Toolchain Authority and Environment

### 49.1 Toolchain lock resolution

`ToolchainAuthority` resolves the technology plan to a verified `AndroidToolchainLock`. It checks versions, file hashes, licenses, paths, compatibility constraints, and required environment variables before any build or preview command. The lock MUST bind to the `toolchainLock` field set defined in BS §5.7.1 (AGP, Gradle wrapper, JDK vendor + major, compileSdk, targetSdk, minSdk, Build Tools, Kotlin, Compose BOM, NDK when applicable). Incompatible combinations MUST be rejected at preflight naming the violated constraint, before any build starts.

The isolated environment controls JDK, Gradle, Android SDK, build tools, platform tools, NDK, CMake, ADB, emulator, temporary directories, Gradle caches, and project-local configuration (ADR-257). Host PATH and unrelated user configuration are not trusted.

Hypervisor preflight MUST be a precondition of emulator readiness. The isolated environment MUST record firmware virtualization enabled, hypervisor platform present, and conflicting hypervisor consumers before emulator launch.

### 49.2 EnvironmentSnapshot

The environment snapshot includes toolchain lock hash, tool versions and hashes, selected emulator identity, API level and ABI, build variant, relevant environment variables, Gradle and package lock hashes, provider metadata without secrets, project fingerprint, and command policy. It is attached to build, recovery, preview, and artifact evidence.

### 49.3 Toolchain repair

Toolchain repair may install, hydrate, or repair components only through an approved operation capability. It records acquisition source, checksum, license metadata, before/after health, and rollback behavior. A repair that changes the lock requires a new checkpoint and technology-plan compatibility validation.

### 49.4 Android toolchain provisioning

Provisioning is how a fresh Windows machine — no JDK, no Android SDK, no emulator, no system image — acquires everything the Android build and the Nirman-managed local Android emulator need. It is owned by `ToolchainProvisioner`, a supervisor service under `ToolchainAuthority` (§49.1). No worker, model, skill, plugin, or UI component may download, unpack, or configure a toolchain component through any other path; provisioning is a privileged operation class (build spec §9.3) executed only by the supervisor.

**Engine identity.** The Nirman-managed local Android emulator is the Google Android Emulator as distributed through the Android SDK repository, running Google APIs system images (ADR-221). Nirman MUST NOT bundle, fork, patch, rebuild, or redistribute the emulator, a system image, or any other Android SDK component, whether inside the Nirman installer or through a Nirman-operated download server: the SDK licence is granted to the user and is non-sublicensable, and a self-built system image lacks Google Play services, which the generated-application scope of build spec §5 requires (maps, push notifications, billing, fused location, sign-in). Nirman orchestrates the emulator engine exactly as it orchestrates the JDK, Gradle, and ADB (build spec §51.1). "Prebuilt" therefore means provisioned, configured, booted once, snapshotted, and proven inside the Preview panel before the user's first project — never shipped.

**Trigger and precedence.** Provisioning starts on first launch, before any project exists, as part of the first-run flow of build spec §4.2, and re-runs when the pinned manifest changes or a health probe fails. It never waits for an AI provider: a `SessionProviderMode.PLANNING_ONLY` session provisions fully, and provider setup proceeds in parallel. Within the resolution order of build spec §26.11 (restated in §11.1 of this document), the Nirman-provisioned toolchain root is the portable installation; it is selected whenever a project declares no version manager and no explicitly configured path, which is the default for every generated project. An Android SDK, JDK, or Android Studio already present on the host is detected and recorded in `EnvironmentSnapshot` (§49.2) as detected-not-used; it is never adopted implicitly. `ANDROID_HOME`, `ANDROID_SDK_ROOT`, `JAVA_HOME`, and the user or machine `PATH` are never read as a source of truth and never written; the equivalent variables are set per spawned process only, through the environment filtering of §9.2.

**Toolchain root.** `C:\Nirman\<sid8>\tc\`, where `C:\Nirman\<sid8>\` is the per-user root of build spec §79.14 (`<sid8>` = the first eight lowercase hex characters of the SHA-256 of the Windows account SID) — never under the user profile, Desktop, or a synced folder — created by the supervisor and ACL-scoped to that account:

```text
C:\Nirman\<sid8>\tc\
  manifest\<manifestVersion>.json          pinned component list, digests, licence hashes
  jdk\<vendor>-<major>\                    LTS JDK (Temurin or Microsoft Build of OpenJDK)
  sdk\cmdline-tools\latest\                sdkmanager, avdmanager
  sdk\platform-tools\                      adb
  sdk\build-tools\<version>\
  sdk\platforms\android-<api>\
  sdk\emulator\                            emulator engine
  sdk\system-images\android-<api>\google_apis\x86_64\
  sdk\licenses\                            accepted licence hashes, written only after the user accepts
  avd\nirman-<profile>.avd\                AVDs and their quick-boot snapshots
  gradle\                                  GRADLE_USER_HOME of the provisioned lane
  downloads\                               in-flight archives; emptied after digest verification
```

**Pinned manifest.** The manifest is versioned and signed with the Nirman release and names, for every component, the SDK-repository package path or vendor download URL, the exact version, the SHA-256 digest, the byte size, the licence identifier with its hash, and the install location. It selects one baseline: the current stable API level's Google APIs x86_64 system image, the matching platform and build-tools, the current stable emulator, platform-tools, and one LTS JDK satisfying the AGP requirement of the `AndroidToolchainLock` (§49.1). Downloads come only from the manifest's sources over HTTPS; an archive whose digest does not match is discarded and recorded as `FAILED_INTEGRITY`; no component executes before its digest is verified. Further system images and device profiles (other API levels, tablet and foldable profiles) are provisioned on demand when the technology plan or the device matrix requires them, through the same manifest, consent, and evidence path. A new manifest version is the only way a component changes; it invalidates the AVD snapshot, rebuilds it, and changes the environment fingerprint so earlier evidence is invalidated per CLAUSE.PLATFORM.EVIDENCE_ENV_BINDING.

> **Schema projection:** `ToolchainProvisioningManifest` is defined in `nirman-schemas.md` §2.87. Owner: TA §49.4.

**The three user actions no software can remove.** Each is a durable `USER_REQUIRED` decision under build spec §79.11 — a single action on a screen that already shows the consequence — and never an installation guide, an external link, or a command for the user to run:

1. *Consent and licence acceptance*, once per machine. One screen lists the components, the download size, the disk requirement, and the full Android SDK License Agreement text; the single Continue action records `ToolchainProvisioningRecord.licenseAcceptance` (licence hash, manifest version, timestamp, Windows account) and only then writes `sdk\licenses\`. Nirman MUST NOT pre-accept, auto-accept, or accept the licence on the user's behalf. A declined licence leaves every Android capability `UNAVAILABLE` with that reason; the licence text is shown again only when its hash changes.
2. *Hypervisor enablement*, when preflight (§49.1; build spec §79.16) finds firmware virtualization enabled but no usable accelerator. The supervisor relaunches itself elevated — `NirmanSupervisor.exe --elevated-hypervisor-setup`, one UAC prompt whose text names the exact change — and enables Windows Hypervisor Platform when Hyper-V, VBS/HVCI, WSL2, or Windows Sandbox is active on the host, otherwise installs the Android Emulator Hypervisor Driver from the SDK repository; never both. HAXM is never provisioned. A required restart is a `USER_REQUIRED` resume condition, and provisioning resumes from durable state after the restart without user action.
3. *Firmware virtualization* disabled in UEFI/BIOS. No software can change it. Nirman names the setting for the detected firmware vendor, blocks emulator readiness on that single condition, and continues every non-emulator step under the split rule of build spec §79.4.

**Preflight.** Host architecture first: Nirman requires a 64-bit x86-64 host (`hostArchitecture = X64`). On a Windows ARM64 host, host preflight resolves the host to `HOST_OUT_OF_SCOPE` (build spec §79.17; ADR-257); the provisioner stops before acquiring or installing any toolchain component, records the environment capability as `HOST_OUT_OF_SCOPE`, and never downloads or launches tools or emulators for an unsupported host. Disk: the free space on the toolchain drive must be at least 2.5 × the manifest's total download size (archives, unpacked trees, first snapshot); otherwise `FAILED_DISK` with the exact figures as a `USER_REQUIRED` decision, never a partial install. Network: the manifest sources must be reachable; otherwise `WAITING_NETWORK` with automatic retry and backoff — a condition independent of `SessionProviderMode.OFFLINE`, which concerns the AI provider only. Downloads are resumable. The downloader honours the Windows system proxy: it resolves each manifest URL through the WinHTTP proxy configuration and the signed-in user's Internet Options (explicit proxy, PAC script, or WPAD), in that precedence, and records the path it used as `ToolchainProvisioningRecord.networkPath` (`DIRECT`, `SYSTEM_PROXY`, or `PAC`) together with the proxy host when one applied. It never reads a proxy from `HTTP_PROXY`/`HTTPS_PROXY` environment variables, never prompts for proxy credentials, and never stores them: a proxy that demands authentication the operating system does not supply, a TLS-intercepting proxy whose certificate the Windows trust store does not hold, or a captive portal is reported as `WAITING_NETWORK` with the proxy host and the observed HTTP status or TLS failure, so that the user sees which network component blocked the download, and provisioning resumes automatically when a probe succeeds. A digest is verified after every download regardless of the path, so an intercepting proxy that alters an archive yields `FAILED_INTEGRITY`, never an installed component. Real-time scanning over the toolchain root is recorded per build spec §79.15.

**Readiness.** After installation the provisioner creates the AVD from the Nirman device profile with `avdmanager` (fixed hardware profile: phone, 1080 × 2400, 420 dpi, 4 GB RAM, host GPU with SwiftShader fallback per §10.7), cold-boots it headless once, waits for `sys.boot_completed`, saves the quick-boot snapshot, and runs the readiness probe: `adb` responsive, the emulator control endpoint answering `getStatus`, and one identity-valid frame delivered through the `RenderTransport` and accepted by the supervisor-side preview readiness predicate; PreviewHost painting is an additional presentation-health observation. Readiness is proven only by that frame, recorded as `ToolchainProvisioningRecord.readinessEvidenceId` (screenshot plus the `PreviewSyncEvent` that carried it); a run that installs everything but delivers no frame is `PROVISIONED_UNVERIFIED`, never `READY`.

The provisioning state machine is `NOT_PROVISIONED → CONSENT_REQUIRED → DOWNLOADING → VERIFYING → INSTALLING → (HYPERVISOR_REQUIRED) → AVD_CREATING → FIRST_BOOT → SNAPSHOT_SAVED → READY`, with the side states `WAITING_NETWORK`, `FAILED_INTEGRITY`, `FAILED_DISK`, `PROVISIONED_UNVERIFIED`, `USER_REQUIRED`, and `UNAVAILABLE`. It maps onto the four-state vocabulary of build spec §79.1 deterministically: `READY` is `AVAILABLE`; `NOT_PROVISIONED`, `WAITING_NETWORK`, `FAILED_INTEGRITY`, `FAILED_DISK`, and `PROVISIONED_UNVERIFIED` are `REPAIRABLE`; `CONSENT_REQUIRED`, `HYPERVISOR_REQUIRED`, and a firmware block are `USER_REQUIRED`; a declined licence or an unsupported CPU is `UNAVAILABLE`. The classification is written by the provisioner from observed state (CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION); no model may set or raise it.

**Evidence and isolation.** Every run produces a `ToolchainProvisioningRecord`, attached to `EnvironmentSnapshot` (§49.2) and to `EnvironmentCapabilityRecord` (build spec §79.2), extending the §49.3 record with the manifest version, per-component source and observed digest, licence acceptance, consent figures, hypervisor action, elevation performed, restart required, detected-not-used installs, AVD and snapshot identity, readiness evidence, and the environment fingerprint after the run. Every provisioning process — downloader, `sdkmanager`, `avdmanager`, the first boot — runs through the restricted-token and Job Object path of §3.4 and §9.2 with process-scoped environment only. Uninstalling Nirman offers to remove the toolchain root; it never removes an SDK the user installed themselves.

> **Schema projection:** `ToolchainProvisioningRecord` is defined in `nirman-schemas.md` §2.88. Owner: TA §49.4.

---

### 49.5 Local auxiliary decision-engine provisioning

`LocalDecisionEngineProvisioner` is a supervisor-owned provisioning service for the bounded auxiliary decision engine permitted by ADR-252.

It uses a Nirman release-pinned manifest rather than provider configuration. The manifest entry identifies the engine version, exact model revision, immutable HTTPS source, SHA-256 digest, byte size, license identity/hash, runtime adapter version, install path, supported decision primitives, supported languages, and admission profile.

Provisioning MUST:
1. validate the Nirman release manifest;
2. validate the immutable model source identity;
3. acquire artifacts only over HTTPS;
4. verify SHA-256 before execution or model load;
5. install below the Nirman-managed local model root;
6. record environment identity and artifact identity;
7. run the local-engine self-test;
8. persist the resulting `LocalDecisionEngineProfile` and the frozen `LocalDecisionAcceptanceProfile`; and
9. admit the engine only when all required integrity and runtime checks pass.

The frozen `LocalDecisionAcceptanceProfile` used for admission and proposal validation MUST be persisted in `local_decision_acceptance_profiles` by its exact immutable `profileId`. Proposal replay MUST resolve the exact stored acceptance-profile version; a mutable or reconstructed acceptance profile is not sufficient for replay or evidence validation.

A local-engine profile MUST NOT use a mutable `main`, `latest`, floating branch, or unpinned model alias as its identity.

The first-launch bootstrap MAY provision the engine automatically. No additional user installation workflow is required. Provisioning is independent of external provider configuration and MAY proceed while `SessionProviderMode` is `PLANNING_ONLY`.

Local-engine health-state progression is:

`NOT_INSTALLED → MANIFEST_VERIFIED → PROVISIONING → READY`

with `DEGRADED`, `WAITING_NETWORK`, `FAILED_INTEGRITY`, `FAILED_RUNTIME`, and `UNAVAILABLE` side states.

`admissionState` and `healthState` are orthogonal state dimensions and MUST NOT be conflated.

`admissionState` determines whether a local-engine profile is admitted for use:
- `DISABLED` — the profile cannot be loaded or invoked;
- `EXPERIMENTAL` — the profile may execute only under the M126 experimental/shadow constraints;
- `ACTIVE` — the profile may provide normal advisory proposals when health and acceptance requirements pass;
- `QUARANTINED` — the profile is not admitted for new inference.

`healthState` determines the current operational condition of an admitted profile. `READY` permits inference. `DEGRADED`, `UNAVAILABLE`, and all failure/provisioning states are non-ready states and require the consumer's declared fallback.

`DEGRADED` is a health state, not an admission state and MUST NOT be represented as an `ACTIVE → DEGRADED` admission transition.

`QUARANTINED` is an admission decision. A quarantined profile MUST NOT load for new inference and MUST NOT produce `ACCEPTED_AS_INPUT` proposals.

Re-entry from `QUARANTINED` requires the applicable profile validation and admission gate again; a quarantined profile MUST NOT return directly to `ACTIVE` merely because the immediate failure condition disappears.

The engine MUST NOT block supervisor startup, deterministic runtime operation, toolchain provisioning, project creation, planning-only mode, or external-provider configuration.

`LocalDecisionEngineProfile.admissionState` begins as `EXPERIMENTAL`. A profile may become `ACTIVE` only after M126 evidence passes.

Changing the engine revision, runtime adapter, model digest, manifest version, or decision contract invalidates prior local decision proposals tied to the previous profile identity.

The local engine MUST be loaded and retained only when admitted by the existing resource-integrity authority. Under memory or CPU pressure it may be unloaded or marked unavailable without blocking the task.

The local engine has no authority over filesystem mutation, process execution, provider credentials, emulator control, policy, evidence promotion, artifact promotion, or completion.

> **Schema projection:** `LocalDecisionEngineProfile` is defined in `nirman-schemas.md` §2.129. Owner: TA §49.5.

---

## 50. Preview Coordinator and Android Runtime Validation

`PreviewCoordinator` selects the least expensive valid preview mode for the current change and falls back when that mode cannot prove the requested behavior.

```text
Change classification → preview mode selection → build/install/reload
→ health and revision verification → screenshot/interaction/log evidence
→ PreviewRevision commit or stale/failure event
```

The coordinator supports incremental emulator install, Compose reload, full APK reinstall, Nirman-managed local Android emulator execution, headless smoke tests, and diagnostic-only source preview. Diagnostic preview can support recovery but can never satisfy final completion.

A `PreviewRevision` includes source revision, artifact hash, device serial/profile, API level, build variant, technology-plan hash, preview mode, launch timestamp, health status, screenshot IDs, and Logcat evidence.

---

## 51. Repair Registry, Decision Trace, and Resource Governor

### 51.1 Repair registry

`AndroidRepairRegistry` maps structured failure fingerprints to repair strategies. Each pattern contains classifier, severity, likely cause, allowed scope, preconditions, operation type, recovery-attempt policy (`recoveryAttemptPolicy`), checkpoint rule, validation command, and evidence requirements.

Patterns cover JDK/Gradle/AGP/Kotlin/Compose compatibility, missing SDKs, Gradle/dependency conflicts, resource and manifest errors, DEX/R8 failures, NDK/native-module failures, emulator/ADB/install failures, runtime crashes, permission errors, visual/accessibility issues, and APK/signing failures.

A learned repair can be promoted into the trusted registry only after repeated successful validation across independent fixtures. Model suggestions remain untrusted until promoted by deterministic evidence.

> **Schema projection:** `RepairPattern` is defined in `nirman-schemas.md` §2.96. Owner: TA §51.1.

Repairs get cheaper over time (ADR-225). Each registry entry is a `RepairPattern`; the `BUILT_IN` set ships with the deterministic first-line fixes for the failure families above — missing SDK component → provision through `ToolchainProvisioner` (§49.4); AGP, Gradle, Kotlin, or JDK mismatch → re-pin from the toolchain lock; manifest merge conflict → declared resolution; duplicate class → dependency exclusion; missing `INTERNET` or runtime permission → manifest and request-flow addition; cleartext-traffic failure → network security configuration; main-thread network call → dispatcher move; R8 stripping → keep rule. When a failure fingerprint matches a `BUILT_IN` or `PROMOTED` pattern whose preconditions hold, the runtime applies the pattern before any model reasoning; the attempt counts toward `recoveryAttemptPolicy` like any other, and a pattern that fails twice on the same fingerprint is demoted to `CANDIDATE` for that project. The self-improvement manager (§30) is the only writer of new patterns, from `ImprovementProposal`s backed by repeated successful repairs, and never past `CANDIDATE` without independent-fixture evidence.

### 51.2 DecisionTrace service

The service records concise decision summaries without hidden chain-of-thought. It stores inputs, constraints, candidate actions, selected action, deterministic procedure identity (`decisionProcedureId` and `decisionProcedureVersion`), evaluation criteria record (`orderedCriteriaApplied` and `firstDiscriminatingCriterion`), policy checks, provider/model provenance, confidence, outcome, and evidence references. The UI can show why a technology, worker, repair, checkpoint, preview mode, or provider was selected.

### 51.3 ResourceGovernor

`ResourceGovernor` is the §57.2 process-topology name of `ResourceIntegrityAuthority` (§59, §77; build spec §72) — one service, one authority. The governor monitors CPU, RAM, disk, checkpoint storage, emulator memory, Gradle memory, worker/provider concurrency, context size, log volume, build duration, and device slots. It can compact context, reduce concurrency, prune safe caches, stop redundant workers, select affected tests, defer nonessential checks, or use an approved lighter provider profile whose `AttentionReliabilityProfile` satisfies the pending step's `requiredReliability` (§59.12). It cannot weaken sandbox, permission, evidence, signing, or artifact gates.

### 51.4 AndroidRepairIntelligenceService

`AndroidRepairIntelligenceService` is the supervisor-owned, read-only aggregate query facade that unifies error intelligence, failure classification, proven repair pattern lookup, and recovery guidance. It exposes a typed query interface to the kernel agent (`Debugging Worker`, `Primary Orchestrator`, `RecoveryAuthority`) and registered desktop IPC command handlers.

`AndroidRepairIntelligenceService` coordinates five deterministic analytical and storage components:
1. *Failure classification:* Maps raw execution diagnostics to the canonical six-tier failure taxonomy T1–T6 (`FailureModeRegistry`, §53.3; BS §42.4).
2. *Proven repair retrieval:* Queries `EpisodicRepairPatternCatalog` (§47.4) and `AndroidRepairRegistry` (§51.1) for pre-verified AST and configuration transformations before model deliberation.
3. *Crash-to-symbol correlation:* Correlates Logcat crash frames and fatal exceptions with production AST symbols via `CrashPatternAnalyzer` (§47.4).
4. *Error trace normalization:* Ingests heterogeneous compiler, Gradle, ADB, and runtime diagnostics through `RuntimeTraceAnalyzer` (§53.7) to produce redacted, LLM-ready failure summaries.
5. *Oscillation and thrashing prevention:* Consults `RepairOscillationDetector` (§58.1.1) to arrest cyclical patch regressions.

`AndroidRepairIntelligenceService` holds no authority and creates no second authority. It is a read-only query facade; it does not directly mutate project source or bypass policy; all repair proposals route through `MutationBroker` (BS §43.2) and commit via `ConstructionTransaction` (BS §42.2). `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

---

## 52. Technical Acceptance Tests

The architecture is accepted only when killing the supervisor during a transaction leaves a recoverable event log and checkpoint; replaying events reconstructs the same authoritative session state; stale worker proposals are rejected without changing the project; changed files or toolchain locks invalidate pending transactions through TOCTOU checks; parallel workers can analyze and propose while conflicting writes are serialized; provider bridge restart and protocol mismatch do not corrupt the session; builds use the locked Android toolchain; preview promotion rejects stale source or artifact revisions; resource pressure changes scheduling without bypassing completion gates; and an APK is not promoted without revision, checksum, environment, validation, and signing evidence.

## 53. Integrated Workflow and Quality Services

### 53.1 WorkflowCoordinator

`WorkflowCoordinator` is the single control-plane service that connects the autonomous Android session contract to execution and completion. It is the `IntegratedAndroidWorkflowCoordinator` of build spec §47.1 and ADR-082, listed as `AndroidWorkflowCoordinator` in the §57.2 process topology: one service under three spellings, and no other coordinator owns the Android construction lifecycle. It owns no side-effect implementation itself; it emits typed commands to supervised services and consumes validated events.

```text
WorkflowCoordinator
├── normalize request and screenshots
├── create/update AndroidConstructionContract
├── run PreflightService
├── select or revise AndroidTechnologyPlan
├── build/validate task graph
├── allocate workers and leases
├── submit ConstructionTransactions
├── coordinate build/preview/test cycles
├── invoke independent AndroidQualityGate
├── route RecoveryAuthority decisions
├── request packaging and artifact validation
└── promote only complete evidence bundles
```

The coordinator must be idempotent at every boundary. Replaying a scheduling or recovery command must not duplicate a worker, transaction, preview installation, artifact, or evidence record.

### 53.2 PreflightService and RiskAndFeasibilityEngine

`PreflightService` gathers deterministic host, provider, workspace, toolchain, device, dependency, requirements, and resource facts. `RiskAndFeasibilityEngine` converts those facts into a `PreflightReport`.

> **Schema projection:** `PreflightReport` is defined in `nirman-schemas.md` §1.83. Owner: TA §53.2.

Routine environment repairs may be dispatched through authorized capabilities. The report must distinguish unavailable credentials, policy restrictions, required device absence, provider limitations, and repairable local deficiencies.

### 53.3 FailureModeRegistry

`FailureModeRegistry` stores preventive and reactive rules. A record contains failure fingerprint, detection source, classification, preconditions, prevention checks, permitted repair scope, strategy alternatives, recovery-attempt policy (`recoveryAttemptPolicy`), checkpoint rule, stop condition, and required evidence.

The registry is consulted before open-ended model diagnosis. A model may propose a new pattern, but promotion into the trusted registry requires independent fixture validation and regression checks.

### 53.4 AndroidQualityGate

`AndroidQualityGate` runs independent review dimensions:

| Dimension | Examples |
|---|---|
| Contract | Requirement coverage, assumptions, unresolved drift |
| Architecture | Module boundaries, technology-plan compliance, dependency direction |
| Build | Clean build, lockfile integrity, variant completeness |
| Security | Secrets, exported components, insecure storage, dangerous permissions, network policy |
| Runtime | Crashes, ANRs, Logcat failures, lifecycle defects, permission behavior |
| UI | Screenshot comparison, navigation, state handling, orientation, responsive layouts |
| Accessibility | Content descriptions, labels, focus order, contrast, touch targets |
| Performance | Startup, frame/jank behavior, memory, CPU, battery-sensitive work, APK size |
| Tests | Acceptance traceability, affected tests, flakiness, missing coverage |
| Release | Version codes, signing, manifest, checksums, artifact provenance |

Each finding is persisted with severity, confidence, source revision, affected scope, recommendation, and evidence references. The gate returns `BLOCKING`, `WARNINGS_ONLY`, or `PASSED` only after all required dimensions report.

### 53.5 TestTraceabilityService

`TestTraceabilityService` maintains the mapping:

```text
contract requirement
  → acceptance criterion
  → test specification
  → selected device/profile
  → execution attempt
  → result
  → evidence
  → artifact revision
```

The service supports unit, integration, instrumentation, UI, visual, accessibility, permission, migration, offline, and smoke tests. It records skipped, blocked, flaky, and not-applicable states rather than treating them as passes.

### 53.5.1 AndroidTestIntelligenceService

`AndroidTestIntelligenceService` is the supervisor-owned, read-only aggregate query facade that exposes on-demand test and coverage comprehension across the project's test suite, coverage graphs, and verification artifacts to the `AgentExecutionKernel` and registered desktop IPC command handlers. It coordinates ten deterministic analytical modules to answer on-demand test intelligence queries without requiring full model deliberation or triggering unneeded emulator test cycles. It is the implementation of the on-demand test comprehension query surface referenced by BS §47.5.

Responsibilities:
- Exposes typed query endpoints for bi-directional test-to-code mapping, prioritized coverage gaps, untested CFG decision branches, semantic test intent, static assertion quality scoring, flakiness signatures, fixture dependency impact, mock boundary conformance, test pyramid balance, and redundant test elimination.
- Routes each query to the respective deterministic module (`TestToCodeMappingEngine`, `CoverageGapLocator`, `UntestedBranchDetector`, `TestIntentExtractor`, `AssertionStrengthAnalyzer`, `FlakyTestSignatureDetector`, `FixtureDependencyTracer`, `MockAndStubBoundaryAnalyzer`, `TestPyramidBalanceAnalyzer`, `RedundantTestDetector`) and returns typed, read-only analytical records.
- Operates 100% locally on the Windows host with zero token, monetary, or reasoning budgets (ADR-218; BS §72).
- Operates as a read-only query facade that holds no authority and creates no second authority. Never mutates project code, executes unauthorized test runners, or overrides deterministic quality gates; all mutation proposals pass through `MutationBroker` and `ConstructionTransactionManager` (BS §43.2), and `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

### 53.5.2 TestToCodeMappingEngine

`TestToCodeMappingEngine` provides deterministic, bi-directional symbol-to-test and test-to-symbol mapping across the project:
1. *Forward mapping (Code to Tests):* Given an AST symbol node (function, Composable, ViewModel, Repository, Room DAO), traverses `AndroidSymbolGraph` (§47.3) and `ImpactGraph` (BS §43.3) to resolve all `TEST_UNIT`, `TEST_INSTRUMENTED`, and `E2EScenario` tests and assertion identifiers that exercise that symbol.
2. *Reverse mapping (Test to Code):* Given a test class, method, or scenario assertion, parses invocation targets, `@Test` call graphs, and `composeTestRule` node lookups to resolve the exact set of production source symbols and files covered by that test.
3. *On-demand query support:* Supplies instant cross-referencing to kernel workers (`Test and QA Worker`, `Debugging Worker`) and the desktop UI without requiring a staging transaction or full impact recomputation.

### 53.5.3 CoverageGapLocator

`CoverageGapLocator` unifies and prioritizes coverage gaps across three distinct architectural layers into a single actionable report:
1. *Tri-layer synthesis:* Aggregates source-level instruction/method coverage (derived from AST instrumentation or local JaCoCo reports), state-space transition coverage (`StateSpaceCoverageModel`, §62.1), and requirement coverage (`RequirementCoverageReport`, BS §56.6).
2. *Prioritized gap enumeration:* Ranks uncovered symbols and screen transitions by risk weight (`riskFactor`, `riskWeight`), distinguishing critical domain logic and unhandled UI failure states from benign cosmetic code.
3. *Targeted test synthesis guidance:* Emits structured recommendations for `ScenarioSynthesizer` (§62.1) and `AssertionAuthor` (§64.1) identifying the highest-value missing tests required to satisfy completion evidence.

### 53.5.4 UntestedBranchDetector

`UntestedBranchDetector` combines intra-procedural control flow graph (CFG) analysis with test execution traces to identify untested decision points:
1. *CFG branch extraction:* Queries `AndroidDataFlowAnalyzer` (§47.4) to enumerate all conditional branches, `when` clauses, Elvis null-coalescing operators (`?:`), sealed class subtypes, and `try/catch` exception handlers in target Kotlin and Java ASTs.
2. *Execution trace correlation:* Cross-references enumerated branch identifiers against unit assertion outcomes and scenario runtime traces (`RuntimeTraceAnalyzer`, §53.7).
3. *Defect and gap reporting:* Flags any executable branch with zero test execution or assertion verification as an `UNTESTED_BRANCH` diagnostic, ensuring error handling and recovery branches are never left unverified.

### 53.5.5 TestIntentExtractor

`TestIntentExtractor` performs deterministic inbound parsing of test code to extract semantic behavioral intent:
1. *Structural intent harvesting:* Parses test method names (e.g. `givenEmptyCart_whenCheckoutClicked_showsError`), JUnit 5 `@DisplayName` annotations, KDoc/Javadoc comments, and Given-When-Then block comments via Tree-sitter AST traversal.
2. *Assertion target analysis:* Inspects assertion calls (`assertEquals`, `assertThat`, `assertIsDisplayed`) and MockK/Mockito verification blocks (`verify { ... }`) to extract the exact behavioral postconditions asserted by the test.
3. *Semantic intent indexing:* Normalizes extracted intents into structured `TestIntentRecord`s linked to `AndroidConstructionContract` requirement IDs, enabling on-demand semantic test search and traceability verification without model deliberation.

### 53.5.6 AssertionStrengthAnalyzer

`AssertionStrengthAnalyzer` statically evaluates the quality, specificity, and mutation-killing strength of test assertions:
1. *Assertion density and presence:* Scans test methods to detect tests with zero assertions (execution-only tests) or tests relying solely on uncaught exception absence.
2. *Superficial assertion detection:* Flags weak assertions that fail to verify state semantics, such as asserting nullness only (`assertNotNull(item)`) when field contents should be validated, or trivial collection size checks (`size > 0`) without member verification.
3. *Tautological and vacuity detection:* Detects tautological assertions (`assertTrue(true)`, variable self-comparison) and assertions on mocked return values; complements the in-loop dynamic `MutationProber` (§64.4) by providing instant pre-commit static quality feedback.
4. *Compose assertion specificity:* Verifies that Jetpack Compose UI tests assert node semantic properties (text, state, enabled/disabled, selected) rather than merely checking node existence in the UI tree.

### 53.5.7 FlakyTestSignatureDetector

`FlakyTestSignatureDetector` proactively scans test code and runtime execution traces for known non-determinism anti-patterns *before* expensive emulator executions:
1. *Static timing hazard detection:* Flags hardcoded delays (`Thread.sleep()`, `delay()`) in test bodies and coroutines, enforcing migration to coroutine test dispatchers with `advanceUntilIdle()` or explicit `wait_for` conditions (§62.4).
2. *Unseeded randomness and clock access:* Detects unseeded `Random()` instances, `UUID.randomUUID()`, `System.currentTimeMillis()`, and unmocked `Instant.now()` in test assertions, requiring deterministic seed binding via `SeedDataProvisioner` (§62.1).
3. *Concurrency and dispatcher hazards:* Flags uncoordinated asynchronous launches (`GlobalScope.launch`, unconfined coroutine dispatchers) and tests missing Compose test synchronization (`composeTestRule.waitUntil` or `IdlingResource`).
4. *Shared mutable state:* Identifies static mutable fields and singleton references that persist across test methods without `@BeforeEach` or `@AfterEach` reset lifecycle hooks.

### 53.5.8 FixtureDependencyTracer

`FixtureDependencyTracer` maps and traces relationships between test cases and their shared fixtures, seed data, and test assets:
1. *Fixture mapping:* Maps unit and instrumentation tests to their declared setup hooks (`@BeforeEach`, `@BeforeAll`), Room pre-packaged database fixtures, mock JSON assets (`src/test/resources/`), and factory objects.
2. *Fixture blast radius calculation:* Computes the downstream test invalidation set when a test asset, seed file, or fixture helper is modified, ensuring only affected tests are scheduled for re-execution under targeted test set derivation (§47.4).
3. *Fixture state leakage detection:* Detects fixtures that modify shared persistent state (SQLite tables, DataStore preferences) without registering corresponding teardown routines, preventing inter-test contamination.

### 53.5.9 MockAndStubBoundaryAnalyzer

`MockAndStubBoundaryAnalyzer` statically inspects test doubles (MockK, Mockito, fake repositories) in the generated project to guarantee contract fidelity:
1. *Stub signature and type fidelity:* Verifies that stubbed methods (`every { repo.getUser(id) } returns user`) conform to current production class and interface signatures, detecting broken stubs immediately when production APIs change.
2. *Over-mocking detection:* Flags anti-patterns where domain data classes, value objects, Room entities, or pure algorithmic utilities are mocked rather than instantiated directly.
3. *Mock boundary isolation:* Verifies that mocks do not leak across test boundaries and ensures that tests asserting end-to-end capabilities do not mock the primary subsystem under test. Complements the supervisor-level `ContractDouble` (§74.1) by enforcing in-project unit test double integrity.

### 53.5.10 TestPyramidBalanceAnalyzer

`TestPyramidBalanceAnalyzer` evaluates the structural distribution of the project's test suite against canonical Android testing pyramid principles:
1. *Tier cardinality computation:* Computes the distribution of tests across the three canonical tiers: Unit tests (fast JVM tests, ViewModels, business logic), Integration/Component tests (Robolectric, Compose UI unit tests, Room DAO tests), and E2E Scenarios (emulator-based full APK workflows, §62.1).
2. *Pyramid balance scoring:* Evaluates the unit-to-integration-to-E2E ratio against recommended balance thresholds (e.g. 70% unit, 20% integration, 10% E2E).
3. *Inversion anti-pattern detection:* Detects the "inverted pyramid" or "ice cream cone" anti-pattern where slow, brittle emulator E2E tests outnumber fast unit tests; provides structured tier-placement guidance to `Test and QA Worker` when synthesizing new tests.

### 53.5.11 RedundantTestDetector

`RedundantTestDetector` identifies duplicate, overlapping, and subsumed tests to maintain test suite efficiency:
1. *AST structural clone detection:* Leverages `SemanticCodeFingerprintEngine` (§47.4) to compare normalized test method ASTs, detecting tests with identical setup, stimulus, and assertion structures across test classes.
2. *Subsumption analysis:* Identifies test cases whose execution path, input equivalence partition, and assertion set form an exact subset of a broader parameterized or scenario test, providing zero marginal fault-detection capability.
3. *Redundancy reporting:* Emits advisory pruning proposals to `Test and QA Worker`, enabling test suite optimization without reducing verified requirement or state-space coverage.

### 53.6 ArchitectureDriftDetector and ContractDriftDetector

The detectors compare the current project graph and build outputs with the approved contract and technology plan. They identify missing features, unreachable screens, undocumented permissions, data models without migrations, untested acceptance criteria, unauthorized dependencies, stale generated files, architecture-boundary violations, and preview/artifact revision mismatch.

A drift finding cannot be dismissed by editing the contract in place. Contract changes require a new version, rationale, reconciliation event, and revalidation of affected requirements.

### 53.7 RuntimeTraceAnalyzer

`RuntimeTraceAnalyzer` normalizes Logcat, stack traces, ANRs, native crash reports, install failures, permission denials, activity/service lifecycle events, and test-runner diagnostics. It produces stable failure fingerprints that feed `FailureModeRegistry`, `RecoveryAuthority`, and affected-test computation.

The analyzer must redact secrets, tokens, personal data, and full user content before persistence or provider submission.

### 53.8 DependencyHealthService

`DependencyHealthService` evaluates Gradle, Maven, npm/pnpm/yarn when selected, native module, and lockfile dependencies for version compatibility, transitive conflicts, known vulnerabilities, license policy, provenance, size impact, duplicate classes, and upgrade risk.

Dependency changes are proposed through ConstructionTransaction and require restore, build, relevant tests, security review, and rollback evidence before commit.

#### 53.8.1 DependencyIntelligenceService

`DependencyIntelligenceService` is the supervisor-owned, read-only coordination facade that exposes a unified typed query interface over `DependencyHealthService` (§53.8), `DependencyResolver` (§70.1), `SubstitutionDetector` (§70.1), `SbomBuilder` (§70.1), and `FindingDispositionStore` (§70.1) to the `AgentExecutionKernel` and registered IPC command handlers. It does not own any of those components; it routes typed queries to them under their existing authorities. It is the dependency-intelligence query surface complement to `AndroidCodeIntelligenceService` (§47.5.1), referenced by BS §58.3.

Responsibilities:
- Accepts typed dependency-intelligence queries (dependency health status, finding enumeration, version resolution status, substitution flag status, SBOM completeness, disposition record lookup) from kernel workers and registered IPC commands.
- Routes each query to the authoritative component and returns a typed read-only result.
- Never writes authoritative state; all dependency mutations pass through the `MutationBroker` and `ConstructionTransactionManager` as governed by BS §43.2.
- Does not create a second authority; `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

### 53.9 ProjectHandbookService and ReleaseReportService

`ProjectHandbookService` generates a concise project handbook from validated state. `ReleaseReportService` generates the artifact release-intelligence report. Both are revision-bound and updated transactionally.

The release report must include source revision, technology plan, toolchain lock, dependency and permission inventory, data-handling summary, device/API results, performance findings, warnings, artifact hashes, signing status, and environment snapshot.

### 53.10 WorkerMetricsService and ValidatedPatternPromotionService

`WorkerMetricsService` tracks worker success rate, regression rate, rollback frequency, handoff completeness, time-to-evidence, affected-test precision, review false positives, and repair reuse. Metrics influence routing but cannot grant permissions.

`ValidatedPatternPromotionService` promotes reusable repairs or generation patterns only after repeated successful validation on independent fixtures, with recorded provenance and regression results.

---

## 54. Native Isolation and External Side-Effect Boundaries

Nirman uses native Windows isolation as its required execution model: restricted tokens, Windows Job Objects, ACL-scoped workspaces, environment filtering, process-tree supervision, resource quotas, toolchain isolation, and disposable Android emulator snapshots. This model is self-contained and must preserve Android emulator, GPU, and Nirman-managed local Android emulator workflows.

Remote Git pushes, pull requests, publishing, store submission, credential use, release signing, and external repository writes remain explicit operation-capability requests. The autonomous session may continue local implementation and validation while waiting for required confirmation, but it must not simulate completion of the external side effect.

### 54.1 Structured reasoning boundary

Prompt normalization, logical consistency checks, risk prediction, alternative comparison, self-critique, reflection, and strategy evaluation return bounded structured records. Nirman does not persist or display hidden chain-of-thought. A decision record contains inputs, constraints, alternatives, selected action, policy checks, model provenance, confidence, outcome, and evidence IDs.

### 54.2 Evidence-based capability claims

Nirman must not advertise a module count, mechanism count, percentage of implementation, or supported feature list as proof that a capability works. A capability is considered supported only when an acceptance fixture passes and its evidence is retained. Product documentation may describe intended capabilities, but implementation status must be derived from executable tests and health results.

### 54.3 Technical acceptance additions

The architecture contract is documentation-complete when the coordinator can run preflight before expensive work, independent quality gates can block promotion, every mandatory requirement maps to executable tests, contract and architecture drift is detected, runtime traces feed repair classification, dependency health is checked before commit, handbook and release reports are generated from validated state, worker metrics are recorded, learned repairs require independent validation, and native isolation or remote side effects cannot weaken the core authority model.

## 55. Private Reasoning and Visible ReasoningStream Architecture

### 55.1 Reasoning boundary

`PrivateReasoningRuntime` may use the configured model’s internal reasoning capabilities for planning, self-critique, hypothesis generation, alternative comparison, diagnosis, and strategy selection. It returns only a structured result to Nirman. Verbatim hidden chain-of-thought is never exposed, persisted, sent to another worker, used as evidence, or accepted as a runtime command.

```text
Model/private reasoning
        ↓
StructuredReasoningSummarizer
        ↓
ReasoningStreamFilter
        ↓
Durable filtered event store ───→ UI stream / replay / export
```

The summarizer must produce concise, decision-relevant information: objective, constraints, alternatives, selected strategy, confidence, uncertainty, expected validation, and next action. It must not reconstruct or infer a verbatim private transcript.

### 55.2 ReasoningStreamEvent

> **Schema projection:** `ReasoningStreamEvent` is defined in `nirman-schemas.md` §2.97.1. Owner: TA §55.2. This section carries event type vocabulary and runtime/producer constraints only.


Allowed event types are `UNDERSTANDING`, `CONSTRAINT`, `PLAN`, `ALTERNATIVE`, `DECISION`, `ACTION`, `OBSERVATION`, `RECOVERY`, `EVIDENCE`, `NEXT_STEP`, `WAITING`, and `COMPLETION`. Runtime events remain distinct from reasoning events. A reasoning event can explain a proposed action, but only a validated runtime event can authorize or prove that action.

### 55.3 Stream pipeline

Provider delta / worker reasoning result
→ schema validation
→ normalization
→ structured reasoning classification
→ deterministic redaction
→ causal binding
→ durable event append
→ authenticated publication
→ UI projection
→ acknowledgement/replay

Runtime execution events and reasoning events share correlation/causation identity but remain separate authority classes. Every visible reasoning event must resolve to its source model request, worker cycle, task, project revision, and associated runtime event where one exists.

The stream publisher must be back-pressure aware. If the UI is disconnected or slow, events remain durable and are replayed from the last acknowledged sequence. Stream delivery cannot block the autonomous runtime indefinitely.

### 55.4 Redaction and privacy service

`ReasoningStreamFilter` applies deterministic redaction before display, persistence, telemetry-free logging, cross-worker handoff, or export. It masks API keys, access tokens, private keys, passwords, cookies, personal data, sensitive project content, complete source files, raw provider messages, hidden system instructions, and sensitive filesystem paths.

The filter returns redaction metadata and a safe replacement summary. If a summary cannot be safely redacted, it is discarded and replaced with a generic event such as “A sensitive implementation detail was omitted; inspect the approved operation and evidence.”

### 55.5 Event persistence and replay

Filtered reasoning events are stored in the event database using the same transaction as the associated authoritative runtime event whenever possible. A reasoning event without a valid session/task/revision reference is rejected. Event payloads use schema versions and content hashes.

Replay reconstructs the visible reasoning stream from filtered durable events. Replay does not call the provider, regenerate private reasoning, rerun tools, or change the project. A replayed event is marked as historical and cannot authorize a new operation.

### 55.6 Local streaming transport

The control plane exposes an authenticated local event stream over the SupervisorConnection protocol (named pipes). Every subscription is bound to the current installation, user session, project, and requested task scope.

```text
subscribe(session_id, task_id, after_sequence)
        ↓
validate UI capability and project scope
        ↓
replay durable events after after_sequence
        ↓
stream new filtered events
        ↓
ack(sequence)
```

The server sends periodic stream heartbeats, detects stale clients, supports reconnect, and prevents one project from receiving another project’s reasoning events. The UI cannot publish forged reasoning events into the authoritative stream.

### 55.7 Provider streaming normalization

The ModelGateway may receive provider-native streamed deltas, reasoning summaries, tool calls, or final responses. It normalizes them internally, but the UI receives only approved `ReasoningStreamEvent`, tool-status, progress, observation, and evidence events. Provider-native hidden reasoning channels are never forwarded verbatim.

Partial model output must not be interpreted as a tool call or file mutation until the complete structured response passes schema, scope, policy, and transaction validation. Cancellation closes the provider stream, records a cancellation event, and leaves the project at the last valid revision.

### 55.8 UI presentation model

The UI provides:

| Presentation | Behavior |
|---|---|
| Calm | Shows the latest safe summary, current action, status, and next step |
| Inspect | Shows chronological reasoning events, workers, tasks, operations, checkpoints, and evidence links |
| Developer | Shows structured rationale, uncertainty, policy references, provider/model provenance, redaction indicators, and replay controls |

The user can pause auto-scroll without pausing execution, collapse repeated events, filter by phase/worker/type, inspect evidence, copy a safe summary, request a current status summary, and replay the session. The UI must distinguish model summary, runtime operation, policy result, and evidence.

- `CoordinationTraceGraphView` — The WinUI 3 presentation view that renders multi-worker task graphs, message timelines, and lease lifecycles.
- `ReasoningTraceGraphView` — The WinUI 3 presentation view that projects multi-pass deliberation traces as an interactive directed acyclic graph.

**Reasoning trace graph visualization.** In `Inspect` and `Developer` modes, the WinUI 3 presentation layer projects multi-pass deliberation traces as an interactive directed acyclic graph (`ReasoningTraceGraphView`):
- *Nodes:* Represent deliberation passes, candidate hypotheses ($H_1, H_2, \dots$), proposed repair strategies ($S_1, S_2, \dots$), and counterfactual audit checkpoints.
- *Edges:* Represent discriminating tests, evidentiary observations, and causal refutation links.
- *States:* Color-coded by lifecycle state: Evaluating (pulsing amber), Pruned/Refuted (muted red with clickable refuting evidence citation), Accepted/Sufficient (emerald green with verification certificate link).
- *Privacy guarantee:* Selecting any node or edge displays the structured rationale summary, uncertainty delta, and associated non-mutating evidence references. The view strictly refuses to render raw private chain-of-thought tokens, enforcing ADR-218 and BS §66.

**Coordination trace and worker timeline visualization.** In `Inspect` and `Developer` modes, the WinUI 3 presentation layer provides real-time visibility into multi-worker coordination and swarm execution dynamics (`CoordinationTraceGraphView`):
- *Concurrency lanes:* Displays horizontal Gantt-style execution lanes for active `NirmanWorker.exe` instances, plotting task start, execution, checkpoint generation, and retirement phases.
- *Message and handoff vectors:* Visualizes typed protocol messages exchanged over named pipes between the supervisor and isolated worker processes, highlighting task graph dependencies and handoff barriers.
- *Lease and fencing state badges:* Renders monotonic lease epochs, resource reservations, and lock states, color-coding active leases (green), preempted leases (amber), and fenced/invalidated leases (red).
- *Audit trace inspection:* Allows developers to inspect structured task events, worker anomaly warnings, and coordination stall resolutions chronologically without exposing raw model tokens.

### 55.9 Failure and ordering behavior

The stream must preserve per-session sequence order. If events arrive out of order, the client buffers them briefly and requests a replay gap when necessary. Duplicate events are de-duplicated by event ID and sequence.

If summarization fails, Nirman emits a safe generic progress event and continues execution. If redaction fails, the event is withheld rather than displayed. If the stream service fails, autonomous execution continues through the control plane and the UI catches up from durable history after reconnection.

### 55.10 Technical acceptance tests

1. Provider deltas become filtered structured events rather than raw hidden reasoning.
2. The stream reconnects from an acknowledged sequence without loss or duplication.
3. A forged UI event cannot enter the authoritative event store.
4. A visible decision cannot authorize a tool or mutation without a separate policy/runtime event.
5. Redaction removes secrets, source content, personal data, and hidden instructions.
6. Private reasoning is absent from logs, event exports, worker handoffs, and replay payloads.
7. Replay is deterministic and side-effect free.
8. Stream back-pressure or UI disconnection never stops the autonomous session.
9. Cancellation stops provider generation and records the last valid revision.
10. Calm, Inspect, and Developer modes change presentation only, not runtime behavior.

## 56. Brand and Asset Runtime Architecture

### 56.1 Asset execution under the canonical UI Worker

Branding and visual-asset work is executed by the canonical UI Worker (§6.5; ADR-049) inside a scoped asset transaction; no dedicated asset worker role exists (ADR-103 as amended). Within that scope the UI Worker turns user brand intent, screenshots, supplied assets, and the AndroidConstructionContract into validated Android visual assets. It may propose generated or vector assets, but the runtime validates every output before integration and promotion.

Responsibilities within the asset scope include brand-intent extraction, BrandManifest creation, asset planning, provider/image-generation requests, vector or deterministic local fallback, adaptive-icon preparation, splash integration, notification-icon preparation, density/format conversion, resource integration, content hashing, visual inspection, accessibility checks, and regeneration after a branding change.

The asset scope is bound to the asset transaction: within it the UI Worker cannot modify unrelated source, change the technology plan, grant permissions, or mark the APK complete.

### 56.2 BrandManifest and AssetManifest schemas

> **Schema projection:** `BrandManifest` is defined in `nirman-schemas.md` §2.125. Owner: TA §56.2.
>
> **Schema projection:** `AssetManifestEntry` is defined in `nirman-schemas.md` §2.126. Owner: TA §56.2.

Schemas are versioned and strict. Each asset entry is linked to the source revision and ConstructionTransaction that generated or changed it. `app_identity` is the display name of build spec §44.2 and §50.3 jointly. Provider/model metadata is jointly covered by `BrandManifest` and its entries and is stored only on `AssetManifestEntry.provider_model_metadata`. `source_prompt_hash` on `BrandManifest` hashes the brand-intent prompt; `source_prompt_hash` on an entry hashes that asset's generation-call prompt. `source_seed` is optional, recorded as an input, and never proof of identical output (§56.8; ADR-105).

### 56.3 Asset state machine

```text
ASSET_INTENT_EXTRACTED
        ↓
BRAND_MANIFEST_READY
        ↓
ASSETS_GENERATING
        ↓
ASSETS_FORMATTING
        ↓
ASSETS_INTEGRATING
        ↓
ASSETS_VALIDATING
        ↓
ASSETS_PREVIEW_VERIFIED
        ↓
ASSETS_RELEASE_READY
```

Failure states are `ASSET_PROVIDER_WAITING`, `ASSET_RETRYABLE_FAILURE`, `ASSET_FALLBACK_PENDING`, `ASSET_BLOCKED`, and `ASSET_SAFE_FAILURE`. A fallback record must state whether the fallback satisfies the user requirement. Placeholder assets are never silently treated as final branded output.

### 56.4 AssetValidator

`AssetValidator` performs:

| Validation area | Required checks |
|---|---|
| File integrity | Exists, readable, content hash, expected format |
| Android resources | Correct resource directory, naming, qualifiers, density, adaptive icon structure |
| Dimensions | Required width/height, aspect ratio, safe zones, splash constraints |
| Visual quality | Transparency, contrast, color-space, clipping, illegible details, visual consistency |
| Accessibility | Notification-icon silhouette, contrast, legibility, theme compatibility |
| Integration | Resource references resolve, manifest points to valid assets, unused requested assets are reported |
| Build packaging | Asset is present in the built APK and reachable at runtime |
| Revision | Workspace, preview, and artifact all reference the same AssetManifest version |

Validation results are evidence records and are linked to the source revision, PreviewRevision, and artifact hash.

### 56.5 Asset transaction and impact analysis

Brand changes use the normal ConstructionTransactionManager. The transaction captures the previous BrandManifest version, affected assets, resource files, manifest references, impacted screens, preview surfaces, and artifact outputs. It regenerates only the affected assets where the impact graph proves independence, invalidates stale asset evidence, refreshes the preview, and reruns the asset gate.

### 56.6 ArtifactAssetInspector

`ArtifactAssetInspector` runs after APK creation and before artifact promotion. It extracts and verifies launcher resources, adaptive and monochrome icon layers where required, splash resources, notification assets, in-app assets, theme resources, and font/illustration references. It compares extracted content hashes with AssetManifest entries and rejects an artifact with missing, stale, wrong-path, or placeholder-only requested assets.

### 56.7 Preview integration

`PreviewCoordinator` receives the current AssetManifest version and includes it in PreviewRevision. The preview must install or reload the candidate artifact, capture relevant launcher, splash, onboarding, header, empty-state, notification, light-theme, and dark-theme surfaces, and attach screenshots to the asset evidence bundle.

A preview showing an older AssetManifest is explicitly marked stale. A source-only asset check cannot satisfy preview verification.

### 56.8 Provider and fallback behavior

Asset generation requests use the configured image-capable provider profile under the normal ModelGateway policy. Provider failures may trigger retry, an approved alternate profile, cached content-addressed output, or a locally generated vector fallback. The system records the fallback and whether it meets the user’s stated requirement.

Seeds, when supported, are recorded as inputs but do not guarantee identical AI output. Output content hashes and visual validation determine reproducibility and freshness.

### 56.9 Technical acceptance tests

1. A brand request creates versioned BrandManifest and AssetManifest records.
2. The UI Worker's asset transaction cannot modify unrelated source or bypass transaction scope.
3. Adaptive, legacy, monochrome, splash, notification, in-app, and theme assets are validated according to the target Android configuration.
4. Resource references and manifest entries resolve before build.
5. APK extraction confirms requested assets are actually packaged.
6. Stale AssetManifest versions cannot satisfy PreviewRevision or artifact gates.
7. Branding changes invalidate affected evidence and regenerate only impacted assets.
8. Provider failure and fallback behavior are explicit and replayable.
9. Placeholder-only output blocks completion when branded assets were requested.

## 57. Locked Implementation Stack and Supervisor Process Architecture

### 57.1 Implementation stack

Nirman uses C#/.NET with WinUI 3 and Windows App SDK for the Windows desktop application. XAML is the presentation language and WinUI 3 Fluent Design is the initial design system. The presentation layer uses a presentation-only MVVM or equivalent state architecture.

Rust with Tokio owns the authoritative local runtime and control plane. SQLite is the execution ledger. SQLx is the preferred asynchronous access layer, with rusqlite permitted only when isolated safely from Tokio scheduling.

The host and control-plane toolchains are version-pinned. The C#/.NET host targets the .NET 10.0 LTS line (C# 14) with the Windows App SDK 2.x stable line (minimum 2.4) and the Rust control plane is built on the 2024 edition, stable channel, minimum 1.98, with Tokio 1.53 and SQLx 0.9 as the pinned library lines. These floors are the canonical implementation targets; the exact patch versions are locked, not chosen ad hoc: the M0 repository foundation commits `global.json`, `rust-toolchain.toml`, a committed `Cargo.lock`, and NuGet package lock files that pin the exact versions within these lines, and the local certification gate fails a build that bypasses them (development plan M0, "C#/.NET, WinUI 3, Windows App SDK, and Rust conventions"). Upgrading a pinned line is a specification change to this section, not an incidental dependency bump. The Android toolchain is deliberately not version-pinned here: its versions are resolved at provision time into the digest-verified `AndroidToolchainLock` (§49.4; build spec §5.7.1), which is the sole version authority for JDK, Gradle, AGP, Kotlin, SDK, build tools, and emulator components.

The Windows runtime uses native APIs including ConPTY, restricted process tokens, Windows Job Objects, ACL-scoped workspaces, environment filtering, process-tree supervision, and resource quotas.

The Android toolchain remains externally installed or managed by Nirman's toolchain authority.

The Rust side is one Cargo workspace under `crates/`. The crate boundaries follow the authority boundaries of this architecture, and `nirman-ipc` — the crate that build spec §76.1 names as the mirror of the `UICommandRegistry` — is the only crate the C#/.NET host binds to:

| Crate | Owns | May depend on |
|---|---|---|
| `nirman-domain` | Canonical schemas of §36.1 as Rust types, enumerations of build spec §5.7.2, `CanonicalSchemaRegistry` metadata | nothing internal |
| `nirman-ipc` | `UICommandEnvelope`, `UIResponseEnvelope`, `UIErrorEnvelope`, `EventSubscription`, `command_registry()` mirroring build spec §76.1, the named-pipe `SupervisorConnection` protocol (§57.3) | `nirman-domain` |
| `nirman-policy` | `PolicyAuthority`, permission profiles (build spec §26.5), operation capabilities | `nirman-domain` |
| `nirman-control-plane` | `LifecycleAuthority` (`SessionReducer` + `EventStore`), `TaskScheduler`, `WorkerRegistry`, `RecoveryAuthority`, `ConstructionTransactionManager` and the `CommitBarrier` (§45.3, §45.4), `LeaseManager` (§46), `CheckpointManager` (§18), `ToolBroker` (§57.8), `TerminalSupervisor` (§57.7), `UpdateController` (§57.4), `ResourceIntegrityAuthority` (§51.3), `ConversationContinuationResolver` (§86.2), the SQLite execution ledger (§57.5), use-case handlers reached from `nirman-ipc` | `nirman-domain`, `nirman-ipc`, `nirman-policy`, `nirman-evidence` |
| `nirman-evidence` | `EvidenceAuthority`, evidence dependency graph, `ExportVerificationRecord` verification, `CapabilityPromotionAuthority` (§36.5) | `nirman-domain` |
| `nirman-provider` | `ModelGateway` (§48), `ProviderAdapter` implementations (§57.8.1), provider bridge lifecycle and failure behaviour (§48.1, §48.3), `UsageRecord` telemetry (§36.4) | `nirman-domain`, `nirman-policy` |
| `nirman-context` | `ContextOrchestrator` and the §59.1 components, `MemoryStore` and `MemoryWriter` with `ProjectMemoryStore` (§59.5, §31), `ContextPackage` assembly (§59.6) | `nirman-domain`, `nirman-control-plane`, `nirman-evidence` |
| `nirman-worker-ipc` | The `WorkerConnection` protocol (§57.11; §3.5): launch-token handshake, heartbeat, and the worker and supervisor message kinds | `nirman-domain` |
| `nirman-kernel` | `AgentExecutionKernel` (§58): the AUTHORIZE through EVALUATE_PROGRESS stages, `AgentLoopReducer`, `WorkerRuntime` (spawns and supervises one `NirmanWorker.exe` per lease), `SwarmPlanner`, `DelegationProtocol`, `CapabilityBroker`, `GoalInterpreter`, `TaskGraphCompiler`, `ProgressEvaluator`, and the other §58.1 modules, the supervisor end of `WorkerConnection` | `nirman-domain`, `nirman-policy`, `nirman-control-plane`, `nirman-worker-ipc`, `nirman-provider`, `nirman-context` |
| `nirman-agents` | `AgentReasoningEngine` (§71), `DeepDeliberationRuntime` (§72), `PrivateReasoningRuntime`, `StructuredReasoningSummarizer` (§55.1), worker-role reasoning profiles, the worker end of `WorkerConnection`; linked by `NirmanWorker.exe` only | `nirman-domain`, `nirman-worker-ipc` |
| `nirman-android` | `AndroidWorkflowCoordinator`, `AndroidTechnologyResolver` (§73.2) and the technology adapters (§73.10), build and device adapters, `ToolchainAuthority` with `ToolchainProvisioner` (§49), `RequirementAuthority` with `AndroidRepairRegistry` (§51.1) | `nirman-domain`, `nirman-policy`, `nirman-evidence` |
| `nirman-preview` | `PreviewCoordinator`, `PreviewPromotionGate` (§73.5.1), `PreviewProjectionReducer`, `PreviewRequest`, `RenderTransport` (§10.7) | `nirman-domain`, `nirman-android`, `nirman-evidence` |
| `nirman-artifacts` | `ArtifactAuthority`, `PackagingProfile` admission, local export handler (§83) | `nirman-domain`, `nirman-evidence`, `nirman-policy` |
| `nirman-skills` | Skill registry, `SkillAdmission`/`SkillInvocationRecord` persistence (§19.1), built-in bodies and manifests under `skills/` | `nirman-domain`, `nirman-policy` |
| `nirman-control-plane` | `LocalDecisionEngineProvisioner` (§49.5) | `nirman-domain`, `nirman-ipc`, `nirman-policy`, `nirman-evidence` |
| `nirman-kernel` | `LocalDecisionEngine` (§58.17) | `nirman-domain`, `nirman-control-plane`, `nirman-evidence` |

`NirmanSupervisor.exe` links every crate of this table except `nirman-agents`; `NirmanWorker.exe` links `nirman-domain`, `nirman-worker-ipc`, and `nirman-agents` and nothing else — no ledger, no policy engine, no adapter, no provider client (§3.5); `Nirman.exe` links only the generated `nirman-ipc` client bindings. A crate that reaches across this table (for example `nirman-preview` writing the ledger directly, `nirman-ipc` containing domain logic, or `nirman-agents` depending on `nirman-control-plane`) or a binary that links outside its row violates §57.2 and §3.5 and is rejected at code review by the M0 module-boundary check (development plan M0, "Repository layout").

### 57.2 Process topology

```text
Nirman.exe
└── C#/.NET + WinUI 3 + Windows App SDK
    ├── Chat
    ├── Project navigation
    ├── Native editor surface
    ├── Native terminal surface
    ├── Android preview presentation
    ├── Task graph and reasoning stream
    ├── Settings and user controls
    └── SupervisorConnection client
              │ authenticated named-pipe protocol
              ▼
NirmanSupervisor.exe
├── LifecycleAuthority (SessionReducer + EventStore, §45)
├── TaskScheduler
├── WorkerRegistry
├── PolicyAuthority
├── ToolBroker
├── ModelGateway
├── RecoveryAuthority
├── EvidenceAuthority
├── ArtifactAuthority
├── CheckpointManager
├── ResourceGovernor
├── TerminalSupervisor
├── AndroidWorkflowCoordinator
├── PreviewCoordinator
├── ContextOrchestrator
├── WorkerRuntime
└── SQLite execution ledger
              │ WorkerConnection: one named pipe per worker lease (§57.11)
              ▼
NirmanWorker.exe × N   (one per active worker lease; restricted token, own Job Object, no network, no workspace ACL — §3.5)
├── AgentReasoningEngine (§71)
├── DeepDeliberationRuntime (§72)
└── PrivateReasoningRuntime
```

The first implementation may host the Rust control-plane modules in-process with the WinUI 3 application to reduce initial process complexity. This allowance is bounded: it applies only to the pre-M7 vertical slice (M1–M6), every UI call MUST still cross the `SupervisorConnection` protocol boundary (ADR-117) so that extraction changes the transport and nothing else, and from M7 onward `Nirman.exe` and `NirmanSupervisor.exe` MUST be distinct processes. An in-process build MUST NOT claim the M7 exit gate, `CAP.ANDROID.BACKGROUND_CONTINUITY`, or `CLAUSE.CONTINUITY.NO_UI_DEPENDENCY`. The allowance never extends to workers: from M5 onward every worker is a `NirmanWorker.exe` process (§3.5), spawned by whichever process hosts the control-plane modules. The production durable-autonomy architecture separates Nirman.exe from NirmanSupervisor.exe.

### 57.3 SupervisorConnection

> **Schema projection:** `SupervisorConnection` is defined in `nirman-schemas.md` §1.84. Owner: TA §57.3.

The connection performs a protocol/version handshake, authenticates the UI instance, validates project scope, subscribes to durable events after a supplied sequence, reports supervisor health, and handles reconnect after UI crash, UI restart, supervisor restart, Windows reboot, and sleep/resume. A UI connection cannot impersonate another project, publish forged events, or invoke a command outside its capability scope.

The canonical `UICommandEnvelope`, `ProjectionSnapshot`, `UIResponseEnvelope`, `UIErrorEnvelope`, and `EventSubscription` schemas, command registry, transaction ownership, and replay rules are defined by technical architecture §81. `SupervisorConnection` carries the authenticated transport and cursor required by that contract.

### 57.4 SupervisorLifecycle, singleton invariant, and recovery scan

One user + one installation → one authoritative supervisor instance. The supervisor is a per-user singleton.

#### Internal Partitioning Optimization (optional)

To support scale, the Supervisor MAY use internal scheduler partitioning, task/resource namespaces, queue sharding, and resource-domain partitioning. These are internal scheduler/resource-domain concepts only. No partition may establish a new authority, process, or supervisor hierarchy. `NirmanSupervisor.exe` remains the single control-plane authority singleton.

Singleton enforcement:
- Mutex/lock ownership: the supervisor acquires a named Windows mutex on startup. A second instance detects the existing mutex, refuses to start, and exits.
- Stale supervisor detection: if the mutex exists but the owning process is dead, the new instance takes ownership after verifying no active leases or tasks are in flight.
- Split-brain prevention: only one supervisor may hold the installation lease and write to the SQLite ledger at a time. The lease is fenced by a monotonic token.
- Second-instance refusal: any additional supervisor process beyond the singleton must terminate immediately without acquiring leases or opening the ledger.
- Supervisor takeover after crash: on crash recovery, the new supervisor fences abandoned leases, reconciles unknown outcomes, and resumes only eligible operations.

The supervisor validates the local auxiliary decision-engine manifest and profile records. Invalid profiles are quarantined and do not prevent supervisor startup. An admitted `ACTIVE` or `EXPERIMENTAL` profile may be loaded once into the resident local decision runtime when resource-integrity admission succeeds. The supervisor then continues the normal session/task recovery scan.

`NirmanSupervisor.exe` starts at Windows user login when an eligible session or scheduled task exists, owns all long-running process trees, and records graceful or abnormal shutdown. Its first bootstrap stage is the stable `UpdateController` of §25.2 (ADR-039): it reads the active-version pointer, verifies the active application directory, and only then loads the control-plane modules of that version; a failed health check after a self-update rolls the pointer back to the previous known-good directory before the control plane starts. On startup it validates SQLite integrity, migrations, leases, checkpoints, project fingerprints, process records, terminal sessions, preview revisions, and pending provider requests.

```text
Supervisor start
  ↓
Validate installation and protocol
  ↓
Open and migrate SQLite
  ↓
Scan active sessions and leases
  ↓
Reconcile workers, terminals, devices, and previews
  ↓
Restore eligible checkpoints and leases
  ↓
Emit recovery stream events
  ↓
Accept UI connections
```

The supervisor must remain useful when the UI is closed. The UI reconnects to the existing authoritative state rather than recreating tasks from client memory.

**Loop liveness scan (ADR-226).** On the worker-staleness schedule of §7.2, `SupervisorLifecycle` also reads the newest `LoopHeartbeat` (build spec §29.4; SCHEMAS §1.78) of every `RUNNING` task. A task whose newest heartbeat is older than the stall detection window is a hung loop, whatever its worker heartbeat says: the supervisor retires the lease through `WorkerRuntime`, records fingerprint `LOOP_HUNG` with the last state entered, and forces `RECOVER` on a fresh lease. `WorkerRuntime` applies the same retirement to a worker whose proposals are rejected `EVIDENCE_NOT_ACQUIRED` for the configured consecutive count (§80.3 of the build spec), attaching the failure fingerprint and the rejected proposals to the new lease. Neither rule counts tokens, requests, or elapsed goal time; both count the absence of a transition.

### 57.5 SQLite execution ledger

SQLite stores transactional execution metadata, not merely settings:

```text
projects, sessions, tasks, task_revisions, task_states,
workers, worker_contracts, worker_leases, handoffs,
events, event_sequences, approvals, policies,
checkpoints, recovery_records, provider_profiles,
provider_capabilities, provider_request_provenance, provider_request_attempts, terminal_sessions, process_records,
preview_revisions, device_profiles, validation_runs,
evidence_records, artifacts, toolchain_manifests,
project_locks, constraint_records, decision_records, reasoning_stream_events, coordination_stall_records, execution_epochs, await_conditions,
join_barrier_states, premise_invalidations, trajectory_assessments,
brand_manifests, asset_manifest_entries,
construction_transactions, change_report_records, conversations,
conversation_messages, conversation_rebase_records, content_revisions,
export_verification_records, environment_capability_records,
local_decision_engine_profiles, local_decision_acceptance_profiles,
local_decision_proposals,
build_gate_records, skill_admissions, skill_invocation_records,
resource_integrity_records, background_continuity_records
```

The added table groups persist, respectively, the §36.1 records of the construction/change-intelligence (§87), conversation (§86), content (§85), export (§83), platform capability (§84), skill (§19.1), resource integrity (§77), and background continuity (§82) contracts; a registered record with no ledger table is a defect of this section. `AssetManifest` is the versioned collection of `asset_manifest_entries` for a `BrandManifest` version and is not a separate ledger table.

Large logs, screenshots, diffs, patches, crash dumps, build output, and APK files remain in the filesystem artifact store with content hashes, revision references, and retention metadata. All durable records use migrations, atomic writes, schema versions, and integrity checks.

### 57.6 UIProjectionState

The C#/.NET WinUI 3 client maintains only presentation state: selected project, open tabs, expanded task nodes, filters, scroll position, optimistic form values, and the last acknowledged event sequence. It receives authoritative task, worker, preview, reasoning, evidence, and health state from the supervisor.

On reconnect, the UI discards stale projections and rebuilds them from the supervisor snapshot plus durable events. No client-side state can mark a task complete, authorize a command, promote an artifact, or change a policy.

### 57.7 Terminal architecture

```text
WinUI terminal surface
      ↓ SupervisorConnection (named pipes)
Supervisor TerminalSupervisor
      ↓
Windows ConPTY
      ↓
PowerShell / cmd.exe / Git Bash / approved shell
```

Rust owns working directory, environment snapshot, shell profile, process group, input policy, output limits, searchable rolling logs, cancellation, tree termination, heartbeat, and recovery. The WinUI terminal surface renders output and sends user input through policy-checked commands; it never owns the process.

### 57.8 Provider authority chain

### 57.8.1 ProviderAdapter interface

> **Schema projection:** `ProviderAdapter` is defined in `nirman-schemas.md` §2.40. Owner: TA §57.8.1.

```text
ProviderProfile
      ↓
ModelGateway
      ↓
ProviderAdapter
      ↓
Structured model proposal
      ↓
ToolBroker
      ↓
PolicyAuthority
      ↓
Runtime authority
      ↓
Filesystem / terminal / emulator / build / artifact
```

Provider adapters normalize configured Chat Completions, Responses-style, and message-oriented external providers together with their declared compatibility mode, capability, streaming, cancellation, and retry behavior. Partial provider output never executes. Complete structured proposals still require scope, schema, policy, revision, capability, and transaction validation.

### 57.8.2 Canonical AI request/stream lifecycle

Every external-provider-backed model invocation follows:

ContextPackage
→ ModelRequest
→ ProviderRequest
→ provider stream
→ StreamEvent accumulation
→ schema/response validation
→ NormalizedResponse
→ ReasoningArtifact / AgentProposal
→ WorkerConnection
→ AgentExecutionKernel

Streaming is incremental transport only. No delta is authoritative.
A completed normalized response is required before proposal execution.
Cancellation terminates the provider stream and returns control to the
kernel without committing a partial proposal.
Provider failure, stream truncation, schema failure, or disconnect creates
a typed runtime outcome and enters the existing recovery path.

Supervisor-local auxiliary decision invocations follow the separate local-engine lifecycle of §58.17 and MUST NOT be represented as `ProviderRequest`, `ProviderResponse`, or provider-stream events.

Provider interruption behavior follows ADR-245: the route's `providerCircuitState` transitions CLOSED → OPEN → HALF_OPEN → CLOSED as defined in build spec §5.7.5; stream resumption requires a provider-supplied resume identity, otherwise the durable logical request is retried only through reconciliation/idempotency rules.

### 57.9 Git and worktree subsystem

Git is a first-class subsystem for checkpoints, rollback, worker isolation, reconciliation, diffs, revision identity, recovery branches, and artifact provenance. Parallel workers use isolated worktrees or copy-on-write fallback. Reconciliation produces an integration revision only after conflict, dependency, requirement, and test-impact checks pass.

### 57.10 Architecture acceptance criteria

The architecture acceptance criteria are the conditions that must be met for the architecture to be considered correct. They are verified by runtime fixtures and evidence, not by documentation certification alone.

The architecture acceptance criteria are satisfied when the UI can restart while the supervisor continues a task; the supervisor can start after Windows reboot and recover eligible sessions; SQLite reconstructs the same state after event replay; ConPTY terminals survive reconnect; stale UI projections cannot mutate authority; provider proposals cannot bypass ToolBroker or PolicyAuthority; the native WinUI editor and terminal surfaces remain presentation components; Android toolchains are supervised locally; and the final APK remains bound to source revision, toolchain lock, preview, evidence, and artifact checksums.

### 57.11 WorkerConnection

> **Schema projection:** `WorkerConnection` is defined in `nirman-schemas.md` §2.90. Owner: TA §57.11.

The supervisor end is `WorkerRuntime` (§58.1); the worker end is the only input and output `NirmanWorker.exe` has (§3.5). Every inter-process edge terminates at `NirmanSupervisor.exe`. The supervisor creates one pipe per lease with a security descriptor whose DACL contains exactly two access-allowed ACEs: one granting full control to the invoking user account (`CURRENT_USER_SID`) and one granting `FILE_GENERIC_READ | FILE_GENERIC_WRITE` specifically to the worker's unique per-lease AppContainer SID (`workerContainerSid`), with no ACE for `ALL APPLICATION PACKAGES` (`S-1-15-2-1`), ensuring that peer AppContainers and other workers cannot open the pipe. The worker authenticates with the one-time launch token it read from standard input, and the handshake binds protocol version, worker ID, lease ID, attempt ID, role, declared execution profile, model profile ID, and limits. A token presented twice, a lease that is not active, or a mismatched attempt closes the pipe. Worker-to-supervisor kinds are `HELLO`, `HEARTBEAT`, `MODEL_CALL`, `PROPOSAL`, `CAPABILITY_QUERY`, `REASONING_ARTIFACT`, `DELIBERATION_RECORD`, `CANCEL_ACK`, and `EXIT`; supervisor-to-worker kinds are `WELCOME`, `CYCLE_INPUT`, `MODEL_EVENT`, `PROPOSAL_RESULT`, `CAPABILITY_ANSWER`, `DECISION`, `PAUSE`, `RESUME`, `CANCEL`, and `CLOSE`. Every worker-originated message carries the lease ID and attempt ID and is rejected once the lease is fenced (§46); nothing a worker sends is authoritative until an authority commits it (§27.1). Pipe traffic is not a durable event: the durable record of a worker's work is what the supervisor commits — `ReasoningArtifact`s, `DeliberationRecord`s, `AgentProposal`s, `AgentLoopRecord`s, and evidence — and the worker keeps no local file.

Control messages (`HEARTBEAT`, `CANCEL`, and lifecycle fencing messages) use `CONTROL` priority and MUST NOT be starved behind bulk artifact/reasoning payloads. The implementation may use bounded per-connection queues; if a lower-priority queue saturates, bulk traffic is delayed/dropped according to policy while control traffic remains deliverable. This is transport QoS only and does not make pipe traffic durable.

**Framed multiplexing and backpressure protocol.** To prevent large AST or proposal payloads from blocking critical lifecycle signals, `WorkerConnection` transmits all frames across an explicit channel envelope:
- *Frame layout:* `[4-byte big-endian uint32 payload_length][1-byte uint8 channel_id][payload_bytes]`.
- *Channel `0x00` (Control lane):* Carries `HEARTBEAT`, `CANCEL`, `CANCEL_ACK`, `PAUSE`, `RESUME`, `FENCE`, and `CLOSE`. Bounded input buffer with immediate unblocking. This list is the published connection-control subset of lane `0x00`; the lane's canonical kind enum is `controlMessageKinds` (`nirman-schemas.md` §2.90).
- *Channel `0x01` (Bulk Data lane):* Carries `REASONING_ARTIFACT`, `MODEL_CALL`, `PROPOSAL`, and `DELIBERATION_RECORD`.
- *Framing bounds:* Maximum frame length is strictly 16 MB (`MAX_FRAME_SIZE = 16 * 1024 * 1024`). Frames exceeding this limit are rejected with an immediate pipe protocol violation error.
- *High-water mark backpressure:* The supervisor maintains a 64 MB high-water mark buffer per connection. If bulk data queues exceed 64 MB, the supervisor pauses emitting `CYCLE_INPUT` and signals worker backpressure until the queue drains below the 16 MB low-water mark.

### 57.11.1 Supervisor Ephemeral Coordination Cache (optional optimization)

The Supervisor MAY maintain an ephemeral, supervisor-owned coordination cache for low-latency subscription to dependency, reservation, or conflict events. This cache is bounded, revision-aware, and non-authoritative; it may accelerate subscription/notification delivery but cannot establish observation, evidence, task state, lease state, or recovery state. All durable state and truth remain in the SQLite ledger and event stream. Cache miss, eviction, or supervisor restart must fall back deterministically to the SQLite ledger/event stream. Durable `WorkerMessage` semantics remain unchanged.

### 57.11.2 Durable Coordination Fabric

All logical worker/swarm messaging physically crosses the Supervisor's durable coordination fabric; there is no peer-to-peer worker transport (§58.16 rule 13). Every application-critical message follows persist → dispatch → receive → accept → apply → durable-ack with independent transport (`deliveryState`) and application (`processingState`) states (`nirman-schemas.md` §1.13). Receiving a message (`deliveryState: ACKED`) never means its state transition was applied; `APPLIED` means the authoritative transition or result was durably committed. `REJECTED` and `DEFERRED` retain a durable `failureCode` and reference so recovery can distinguish them from transport failure (ADR-246). A message transitions to terminal `deliveryState: DEAD_LETTERED` when the envelope fails deserialization or digest verification against protocol schemas, or when delivery attempts exhaust the node's declared `deliveryAttemptPolicy` (governed by `recoveryAttemptPolicy`, build spec §26.3) without receiving `ACKED`. Dead-lettered messages are quarantined with their `failureCode`, error reason, and raw payload for operator inspection and recovery analysis; they are never silently dropped or allowed to block subsequent stream messages.

Delivery is at-least-once with idempotent authoritative application; the fabric makes no end-to-end exactly-once claim. Durable inbox/outbox precede dispatch, so after a crash: `PENDING` messages redispatch, in-flight messages reconcile, `APPLIED` messages never reapply, and unknown states reconcile before new dispatch. `ExecutionEpoch` (`nirman-schemas.md` §2.119) captures in-flight messages and mailbox/order watermarks, not only pending IDs.

Ordering is explicit per coordination stream and never a global serialization of the swarm: a message applies only when its sequence equals the stream's expected sequence; behind-expected and ahead-of-expected messages are held; duplicates are no-ops; a persistent gap triggers reconciliation (§58.11.3). `CANCEL`, `FENCE`, `REPLACE`, `PLAN_SUPERSEDED`, `RECONCILE`, and `RECOVER` cross the reserved-capacity control lane (§58.11.2).

Hierarchical supervision remains logical scoping — Supervisor → coordination scope → swarm → agent → sub-agent → worker: these are scopes in the existing Supervisor, not new executables, and supervisors monitor and restart workers without becoming authorities.


### 57.12 Component and authority registry

This table is the single inventory of Nirman's authorities and of every component name that the build spec or this document uses as an identifier without a heading, component table, or list definition of its own (ADR-223). A PascalCase component name that appears in backticks, inside a fenced diagram, or in a table row of either document is defined at exactly one of: a heading whose words spell it, the first cell of a component table, a list entry that opens with the backticked name, a paragraph that opens with it, a `nirman-schemas.md` heading, or a row below; a name with none of these fails documentation certification. Alias rows carry no crate: an alias is a label for the owner named in its row and never a second authority. `WorkManager` and `DataStore` are Android Jetpack library names that occur only in generated-application content and are not Nirman components. The crate column agrees with the §57.1 table; the non-delegable authorities are those of §58.16, and the §21 hierarchy maps onto them as stated below the table.

| Name | Kind | Crate | Owns | Commits | Defined in |
|---|---|---|---|---|---|
| `LifecycleAuthority` | authority | `nirman-control-plane` | Session and task lifecycle transitions of §36.2 and build spec §33.2; implemented by `SessionReducer` (§45.1); the only committer of lifecycle state (§58.16 rule 1) | `sessions`, `tasks`, `task_states`, `events` (through `EventStore`) | §36.2, §44.1, §45.1 |
| `EventStore` | service | `nirman-control-plane` | Append-only durable event log with monotonic sequence numbers and replay (§45.2); the storage side of `LifecycleAuthority` | `events`, `event_sequences` | §45.2 |
| `ConstructionTransactionManager` | authority | `nirman-control-plane` | Every project mutation (§58.16 rule 4): pre-mutation checkpoint, staging, checks, commit or rollback (§45.3), the `CommitBarrier` (§45.4), and the transaction domains of §36.5 | `construction_transactions`, `task_revisions`, the `ChangeReportRecord` obligation (§87.5) | §45.3 |
| `PolicyAuthority` | authority | `nirman-policy` | Allow, ask, or deny for every filesystem, terminal, network, provider, device, and external-tool action; execution, autonomy, and sandbox profiles (§16.2.2; build spec §26.5); operation-capability grants (§46.2; §58.16 rule 3) | `approvals`, `policies` | §21 "Permission authority", §44.1 "Policy authority" |
| `ToolBroker` | service | `nirman-control-plane` | The only executor of tools (§58.16 rule 2): dispatches a `PolicyAuthority`-admitted `ToolCallRequest` (§24.5) to the filesystem, `TerminalSupervisor`, process, preview, browser, or external-tool adapter (§57.8) through adapter traits the supervisor binary registers at startup | `ActionRecord`s (build spec §11.4), `process_records` | §57.8, §58.16 |
| `EvidenceAuthority` | authority | `nirman-evidence` | Evidence admission and the evidence ledger (§23.3), the dependency graph and cascading invalidation (§36.4; build spec §5.7.4), validation gates and `CertificationDecision`, the sole completion evaluator of build spec §5.7.7 (§58.16 rule 5), and shared-memory drift and poisoning defence — re-evaluating a `KnowledgeArtifact` on evidence invalidation and committing its supersession (§58.6) | `evidence_records`, `validation_runs` | §21 "Evidence authority", §23.3, §44.1 |
| `RecoveryAuthority` | authority | `nirman-control-plane` | The choice of retry, diagnosis, repair, backtracking, delegation, degradation, or safe failure (§21); reconciliation of `UNKNOWN` external outcomes (§36.4) and of continuity state (build spec §77.4); ledger reconstruction (§87.6) | `recovery_records` | §21 "Recovery authority", §44.1 |
| `ArtifactAuthority` | authority | `nirman-artifacts` | APK and optional AAB packaging, checksums, the signing workflow, packaging-profile admission, artifact promotion (§44.1; §58.16 rule 6), and local export (§83) | `artifacts`, `export_verification_records` | §44.1 "Artifact authority", §83 |
| `AndroidArtifactInspector` | service | `nirman-artifacts` | Structural and content inspection of candidate APK and AAB binaries before preview promotion or local deployment export (§74.3; ADR-259) | `AndroidArtifactInspectionRecord` committed to `EvidenceAuthority` | §74.3 |
| `CapabilityPromotionAuthority` | authority | `nirman-evidence` | The last step of the promotion chain (§36.5; build spec §5.7.9): the only writer of `CapabilityMaturity` (build spec §5.6) | immutable capability promotion records | §36.5, build spec §5.7.9 |
| `PreviewCoordinator` | service | `nirman-preview` | Revision-bound emulator deployment, preview-mode resolution, and staleness (§44.1, §73.3) | `preview_revisions` | §50, §73.3 |
| `PreviewPromotionGate` | service | `nirman-preview` | The preview promotion decision (§73.5.1) | `PreviewRevision` promotion state | §73.5.1 |
| `ToolchainAuthority` | authority | `nirman-android` | Toolchain lock resolution, isolated environment construction, and repair (§49.1–§49.3); hosts `ToolchainProvisioner` | `toolchain_manifests` | §49 |
| `ToolchainProvisioner` | service | `nirman-android` | First-launch and repair provisioning of the JDK, SDK, emulator, and system images under `ToolchainAuthority` (§49.4; build spec §4.2) | `ToolchainProvisioningRecord` (SCHEMAS §2.88) | §49.4 |
| `RequirementAuthority` | service | `nirman-android` | Android requirement evaluation: capability and permission inference, manifest and resource validation, and repair selection from `AndroidRepairRegistry` (§51.1) — the "Android requirement authority" of build spec §76.1 (`android.requirements.evaluate`, M47) | requirement evaluation and repair-selection events; the repair itself runs through `RecoveryAuthority` and `ConstructionTransactionManager` | §44.2, §51.1 |
| `AndroidTechnologyResolver` | service | `nirman-android` | Selection of the `AndroidTechnologyPlan` from requirements and evidence, never from a template (§73.2, §73.10) | proposes the plan; `ConstructionTransactionManager` commits it with the revision | §73.2 |
| `NativeAndroidAdapter` | module | `nirman-android` | The Kotlin or Java, Views or Compose, Gradle implementation family of `AndroidTechnologyAdapter` (§73.10; M108) | resolutions only | §73.10 |
| `MixedAndroidAdapter` | module | `nirman-android` | The mixed Kotlin-plus-Java-plus-NDK/CMake native implementation family of `AndroidTechnologyAdapter` (§73.10; M108) | resolutions only | §73.10 |
| `TaskScheduler` | service | `nirman-control-plane` | Runnable-task selection, resource reservation, worker launch requests, heartbeat and stale-process detection, fair share (§7.1, §7.2), and schedule firing (§16.4) | `tasks` claims, `handoffs`, schedule runs | §7.1, §16.4 |
| `WorkerRegistry` | service | `nirman-control-plane` | The canonical worker roles of §6.5 and each worker's `WorkerContract` (build spec §23.4) | `workers`, `worker_contracts` | §6.5 |
| `WorkerRuntime` | service | `nirman-kernel` | Spawns and supervises one `NirmanWorker.exe` per lease and holds the supervisor end of `WorkerConnection` (§3.5, §57.11) | `process_records` for worker processes | §3.5, §57.11 |
| `LeaseManager` | service | `nirman-control-plane` | Session leases (§46.1; §36.3) and single-use operation capabilities (§46.2) — the capability manager of §36.3; workspace leases belong to `WorkspaceLeaseManager` (§58.7) | `worker_leases`, capability issue and consumption events | §36.3, §46 |
| `CheckpointManager` | service | `nirman-control-plane` | File- and task-tier checkpoints, retention, and restore (§18; build spec §11.5 and §27.6) | `checkpoints` | §18 |
| `TerminalSupervisor` | service | `nirman-control-plane` | ConPTY terminal sessions, prompt classification, and output rotation (§57.7; §11.4) | `terminal_sessions`, `process_records` | §57.7, §11.4 |
| `ModelGateway` | service | `nirman-provider` | Provider bridge lifecycle, request normalization, credential resolution, and every provider request (§48; §57.8; §3.5) | provider request events, `UsageRecord`s (§36.4) | §48 |
| `ProjectMemoryStore` | module | `nirman-context` | The project-scope partition of `MemoryStore` (§59.1): `MemoryRecord`s with `scope: project` (§59.5; §31) | project-scope `MemoryRecord`s written by `MemoryWriter` | §31, §59.1 |
| `UpdateController` | service | `nirman-control-plane` | The first bootstrap stage of `NirmanSupervisor.exe` (§57.4): active-version pointer, update lock, health check, and rollback (§25.2) | the active-version pointer and update events | §25.2, §57.4 |
| `ConversationContinuationResolver` | service | `nirman-control-plane` | Reconstruction of Continue state from durable conversation records (§86.2, §86.5) | `conversations`, `conversation_messages`, `conversation_rebase_records` | §86.2 |
| `GoalInterpreter` | module | `nirman-kernel` | Turns the user request and the `AndroidConstructionContract` into the `GoalContract` (§16.1) and its acceptance conditions | proposals only; the contract is committed as a `LifecycleAuthority` event | §58.1 |
| `TaskGraphCompiler` | module | `nirman-kernel` | Compiles the `GoalContract` into the `TaskGraph` (build spec §33.1; SCHEMAS §1.58) inside the admitted capability set (build spec §79.4) | proposals only; graph revisions are `LifecycleAuthority` events | §58.1, build spec §79.4 |
| `ProgressEvaluator` | module | `nirman-kernel` | The EVALUATE_PROGRESS stage (§58.2): classifies a cycle as CONTINUE, RECOVER, DELEGATE, REPLAN, or COMPLETE; not the completion evaluator of build spec §5.7.7 | `AgentLoopRecord.progress_status` through `AgentLoopReducer` | §58.1, §58.2 |
| `PlanCompiler` | module | `nirman-kernel` | With `Replanner`, plan revisions when evidence invalidates the plan (§58.12; build spec §52.13) | plan revision records (`planRevision`, `supersedesPlan`) | §58.12 |
| `ContradictionDetector` | module | `nirman-kernel` | A controlled decision revision when `UncertaintyRegistry` facts contradict (§58.12) | `DecisionNode` proposals only | §58.12 |
| `SwarmAdmissionController` | module | `nirman-control-plane` | Swarm admission evaluation over physical resource, queue, child-concurrency, emulator-slot, provider-concurrency, and recovery/validation-reserve signals (§58.5.1); a scheduler component, never an authority | none | §58.5.1 |
| `PlanAssignmentMigrator` | module | `nirman-kernel` | Classification of every active assignment after plan supersession as RETAIN, REBASE, QUIESCE, CANCEL, or REPLACE (§58.12); never grants authority | none | §58.12 |
| `CoordinationProgressMonitor` | module | `nirman-kernel` | Records `CoordinationStallRecord` and evaluates coordination progress distinct from process liveness (§58.13); routes through RecoveryAuthority | none | §58.13 |
| `MutationRegressionAnalyzer` | module | `nirman-kernel` | Predicts affected behaviour and expands the `ValidationPlan` (§58.9) | proposals only | §58.9 |
| `StructuredReasoningSummarizer` | module | `nirman-agents` | Reduces private reasoning to the structured summary of §55.1; worker-hosted (§3.5) | none — its output leaves the worker as a `REASONING_ARTIFACT` message | §55.1 |
| `AndroidWorkflowCoordinator` | alias | — | alias of `WorkflowCoordinator` (§53.1), the `IntegratedAndroidWorkflowCoordinator` of build spec §47.1, as listed in the §57.2 topology | — | §53.1 |
| `ConversationResolver` | alias | — | alias of `ConversationContinuationResolver` in build spec §82.1 (§86.5) | — | §86.5 |
| `ValidationAuthority` | alias | — | alias of the acceptance-policy interpretation step of `EvidenceAuthority` (§80.2, §83) | — | §80.2 |
| `SigningAuthority` | alias | — | alias of the signing-identity policy of `ArtifactAuthority` (build spec §5.7.9; §74.3) in §83 | — | §83 |
| `PromotionAuthority` | alias | — | alias of artifact promotion by `ArtifactAuthority` and preview promotion by `PreviewPromotionGate` in §83 | — | §83 |
| `ExternalEffectCoordinator` | alias | — | alias of the external-effect transaction domain of `ConstructionTransactionManager` (§36.5) with reconciliation by `RecoveryAuthority` (§36.4) in §83 | — | §83 |
| `SupervisorAuthority` | alias | — | alias of the process-supervision function of `NirmanSupervisor.exe` (`SupervisorLifecycle`, §57.4; §7.1), owner of `hostState` (build spec §77.4) | — | build spec §77.4 |
| `LeaseAuthority` | alias | — | alias of `WorkspaceLeaseManager` (§58.7) and `LeaseManager` lease and fencing control, owners of `leaseState` (build spec §77.4) | — | build spec §77.4 |
| `DeviceAuthority` | alias | — | alias of the device-session authority — the emulator manager of §10.3 and `AndroidDeviceAdapter` (§73.12) — owner of `deviceAvailabilityState` and of the device-profile namespace (§16.2.2; build spec §77.4) | — | build spec §77.4 |
| `ProviderOperationalityAuthority` | alias | — | alias of provider operationality as observed by `ModelGateway` (§48.1, §48.3) and recorded as `IntegrationOperationality` (build spec §5.7.5), owner of `providerAvailabilityState` (build spec §77.4) | — | build spec §77.4 |
| `CrossCompilationAuthority` | decision point | `nirman-policy` | The cross-build admission decision inside `ToolBroker`/`PolicyAuthority`, fed by the `EnvironmentCapabilityPlanner` classification (§84.3) | `BuildGateRecord` (§84.1) | §84.3 |
| `NativeRuntimeValidationAuthority` | decision point | `nirman-evidence` | The native-runtime validation gate inside `EvidenceAuthority` and the completion evaluator (§84.3) | gate closure on `BuildGateRecord` | §84.3 |
| `BrandAssetCompletionGate` | decision point | `nirman-evidence` | The asset completion rules of build spec §50.4 applied by `EvidenceAuthority` to the built APK through `ArtifactAssetInspector` (§56.6): a requested asset that is missing, unpackaged, stale, failing format checks, or unverified in the preview blocks completion | asset gate evidence | build spec §50.2, §56.6 |
| `ScreenGraphExplorer` | module | `nirman-android` | Bounded exploration of the installed application into a `ScreenGraph` on a golden-snapshot device (§62.1; ADR-225) | `ScreenGraph` | §62.1 |
| `ScenarioSynthesizer` | module | `nirman-android` | Derivation of `E2EScenario` steps and assertions for acceptance criteria and the build spec §56.3 classes from the `ScreenGraph`; coverage bookkeeping (§62.1; ADR-225) | `E2EScenario` via `ScenarioRegistry` | §62.1 |
| `AndroidCodeIntelligenceService` | service | `nirman-android` | Read-only aggregate query facade over `AndroidSymbolGraph`, `SemanticCodeFingerprintEngine`, `EpisodicRepairPatternCatalog`, and the project `ImpactGraph`; routes typed code-intelligence queries from kernel workers and IPC command handlers to the authoritative component without creating a second authority (§47.5.1; BS §43.1) | none — read-only; proposals routed through `MutationBroker` | §47.5.1 |
| `AndroidArchitectureReasoningService` | module | `nirman-android` | Static what-if architectural impact analysis via bounded `AndroidSymbolGraph` and `ImpactGraph` traversal before any `ConstructionTransaction` opens; produces advisory `ArchitecturalImpactProjection` only (§47.5.2; BS §43.3) | none — read-only; no authority, no AI-usage budget | §47.5.2 |
| `AndroidSecurityIntelligenceService` | service | `nirman-android` | Named service grouping `AppSecurityScanner`, `SecurityRiskScorer`, and `SecurityAuditGenerator`; coordinates exploit-pattern detection, severity-weighted risk scoring, and pre-promotion security audit report generation for the generated Android application (§70.1; §70.3; BS §58.2) | risk score and audit report records via `FindingDispositionStore` | §70.3 |
| `SecurityRiskScorer` | module | `nirman-android` | Severity-weighted aggregation of `AppSecurityScanner` findings into a structured `SecurityRiskScore` bound to the artifact revision; read-only projection — `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion (§70.1; §70.3) | `SecurityRiskScore` record in `FindingDispositionStore` | §70.3 |
| `SecurityAuditGenerator` | module | `nirman-android` | Composes `FindingDispositionStore` records, `SecurityRiskScore`, SBOM completeness, and `ArtifactProvenance` identity into a security audit report artifact record attached before promotion; read-only projection — promotion authority remains with `ProvenanceRecorder` (§70.1; §70.3) | security audit report artifact record | §70.3 |
| `FindingDispositionStore` | module | `nirman-android` | Records security and dependency findings as blocking or accepted with reason; ensures findings are never silently dropped before artifact promotion (§70.1; BS §58.5) | security finding disposition records | §70.1 |
| `DependencyIntelligenceService` | service | `nirman-android` | Read-only coordination facade over `DependencyHealthService`, `DependencyResolver`, `SubstitutionDetector`, `SbomBuilder`, and `FindingDispositionStore`; routes typed dependency-intelligence queries from kernel workers and IPC command handlers to authoritative components without creating a second authority; `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion (§53.8.1; BS §58.3) | none — read-only; proposals routed through `MutationBroker` | §53.8.1 |
| `AndroidTestIntelligenceService` | service | `nirman-android` | Read-only aggregate query facade over test comprehension engines; routes on-demand test-to-code mapping, coverage gap analysis, untested branch detection, test intent extraction, assertion strength analysis, flakiness detection, fixture tracing, mock boundary validation, pyramid balance, and redundant test elimination without creating a second authority (BS §47.5) | none — read-only; proposals routed through `MutationBroker` | §53.5.1 |
| `TestToCodeMappingEngine` | module | `nirman-android` | Bi-directional symbol-to-test and test-to-symbol mapping across unit, instrumentation, and scenario tests (§53.5.2) | none — analytical queries | §53.5.2 |
| `CoverageGapLocator` | module | `nirman-android` | Prioritized coverage gap locator synthesizing AST source coverage, state-space transitions, and requirement gaps (§53.5.3) | none — analytical queries | §53.5.3 |
| `UntestedBranchDetector` | module | `nirman-android` | Correlates `AndroidDataFlowAnalyzer` CFG decision points with test execution traces to locate untested branches (§53.5.4) | none — analytical queries | §53.5.4 |
| `TestIntentExtractor` | module | `nirman-android` | Deterministic inbound test AST parser extracting semantic behavioral intents from test declarations and assertions (§53.5.5) | none — analytical queries | §53.5.5 |
| `AssertionStrengthAnalyzer` | module | `nirman-android` | Static analyzer evaluating assertion density, specificity, and vacuity across test ASTs (§53.5.6) | none — analytical queries | §53.5.6 |
| `FlakyTestSignatureDetector` | module | `nirman-android` | Static scanner detecting flakiness anti-patterns (sleeps, unseeded random, unconfined dispatchers) in test code (§53.5.7) | none — analytical queries | §53.5.7 |
| `FixtureDependencyTracer` | module | `nirman-android` | Maps tests to shared fixtures, seed data, and test assets, computing fixture change blast radius (§53.5.8) | none — analytical queries | §53.5.8 |
| `MockAndStubBoundaryAnalyzer` | module | `nirman-android` | Verifies in-project test double signatures, contract fidelity, and over-mocking anti-patterns (§53.5.9) | none — analytical queries | §53.5.9 |
| `TestPyramidBalanceAnalyzer` | module | `nirman-android` | Evaluates test tier cardinality and detects inverted test pyramid anti-patterns (§53.5.10) | none — analytical queries | §53.5.10 |
| `RedundantTestDetector` | module | `nirman-android` | Identifies duplicate and subsumed test cases via AST structural fingerprints and execution path overlap (§53.5.11) | none — analytical queries | §53.5.11 |
| `AndroidProductIntelligenceService` | service | `nirman-android` | Read-only aggregate query facade over product and requirement intelligence; routes companion requirement mining, pre-construction conflict detection, testability scoring, persona inference, offline domain knowledge lookup, and regulatory compliance analysis without creating a second authority (BS §42.1, BS §69.11) | none — read-only; proposals routed through `MutationBroker` | §73.15 |
| `ImplicitRequirementMiner` | module | `nirman-android` | Deterministically expands high-level user goals into mandatory companion requirements for auth, data lists, and transactions (§73.15.1; BS §42.1) | none — analytical queries | §73.15.1 |
| `RequirementConflictDetector` | module | `nirman-android` | Pre-construction semantic and architectural contradiction detector across proposed requirements (§73.15.2; BS §69.11) | none — analytical queries | §73.15.2 |
| `RequirementTestabilityScorer` | module | `nirman-android` | Statically evaluates observable post-conditions and testability of requirements on Android (§73.15.3; BS §69.11) | none — analytical queries | §73.15.3 |
| `PersonaInferenceEngine` | module | `nirman-android` | Infers target stakeholder personas, touch target ergonomics, and accessibility profiles (§73.15.4; BS §69.2) | none — analytical queries | §73.15.4 |
| `AndroidDomainKnowledgeCatalog` | module | `nirman-android` | Local offline catalog of idiomatic Android Room entity models and standard state-machine workflows (§73.15.5; BS §69.2) | none — analytical queries | §73.15.5 |
| `RegulatoryComplianceAnalyzer` | module | `nirman-android` | Audits declared permissions, target API levels, and data collection against Google Play policies and regional regulations (§73.15.6; BS §42.1) | none — analytical queries | §73.15.6 |
| `AndroidRepairIntelligenceService` | service | `nirman-android` | Read-only aggregate query facade over repair patterns, failure classifications, and recovery recommendations (§51.4; BS §42.4) | none — read-only; proposals routed through `MutationBroker` | §51.4 |
| `BuildReproducibilityChecker` | module | `nirman-artifacts` | Verifies multi-pass deterministic byte equality and reproducibility of APK outputs under `ArtifactAuthority` (§83.4; BS §42.1) | none — analytical verification | §83.4 |
| `RepairOscillationDetector` | module | `nirman-agents` | Detects cyclical patch regressions (A breaks B, B breaks A) across transaction checkpoints (§58.1.1; BS §42.4) | none — anomaly detection | §58.1.1 |
| `BlankScreenDetector` | module | `nirman-preview` | Frame luminescence, entropy, and semantics-tree inspection detecting blank or unpopulated screens (§73.5.2; BS §56.5) | none — preview validation | §73.5.2 |
| `DeadControlDetector` | module | `nirman-android` | Verifies that interactive UI elements trigger observable state transitions or feedback during exploration (§62.1.1; BS §56.3) | none — scenario validation | §62.1.1 |
| `AndroidGenerationIntelligenceService` | service | `nirman-android` | Read-only aggregate query facade over code generation patterns, placeholder detection, syntactic truncation validation, and anti-pattern checks (§47.5.3; BS §43.1) | none — read-only; proposals routed through `MutationBroker` | §47.5.3 |
| `PlaceholderResidueDetector` | module | `nirman-android` | Statically scans AST and XML resources for unexpanded placeholder markers (TODO, FIXME, Lorem ipsum) (§47.4; BS §43.1) | none — pre-commit verification | §47.4 |
| `TruncatedFileDetector` | module | `nirman-android` | Syntactic continuity verifier detecting premature EOF, unclosed delimiters, and cut-off completions (§47.4; BS §43.1) | none — pre-commit verification | §47.4 |
| `MockResidualDetector` | module | `nirman-android` | Production source set scanner preventing unauthorized mock doubles and fake data from leaking into release builds (§47.4; BS §43.1) | none — pre-commit verification | §47.4 |
| `StartupRegressionTracker` | module | `nirman-android` | Measures TTID and TTFD cold-start launch latency from Logcat and flags performance regressions (§62.1.2; BS §56.3) | none — performance tracking | §62.1.2 |
| `MemoryLeakDetector` | module | `nirman-android` | Evaluates heap growth and Activity retention across repeated lifecycle churn and navigation cycles (§62.1.3; BS §56.3) | none — leak detection | §62.1.3 |
| `TestDataLeakageDetector` | module | `nirman-android` | Verifies persistent storage isolation, ensuring synthetic seed data does not survive teardown (§62.5.1; BS §56.4) | none — isolation validation | §62.5.1 |
| `AndroidDataIntelligenceService` | service | `nirman-android` | Read-only aggregate query facade over Room schemas, migrations, query performance, and offline sync (§47.5.4; BS §43.1) | none — read-only; proposals routed through `MutationBroker` | §47.5.4 |
| `RoomSchemaMigrationAnalyzer` | module | `nirman-android` | Statically diffs Room schema JSONs, verifies migration paths, and checks for destructive table/column drops (§47.4; BS §43.1) | none — pre-commit verification | §47.4 |
| `QueryPerformanceAnalyzer` | module | `nirman-android` | Statically detects N+1 queries in Room DAOs, recommends indices, and verifies SQL parameter binding (§47.4; BS §43.1) | none — pre-commit verification | §47.4 |
| `OfflineSyncProtocolPlanner` | module | `nirman-android` | Validates offline-first sync architecture, reactive Flow repositories, and WorkManager Outbox patterns (§47.4; BS §43.1) | none — pre-commit verification | §47.4 |
| `AndroidIntegrationIntelligenceService` | service | `nirman-android` | Read-only aggregate query facade over API contracts, third-party integrations, and mobile auth (§74.7.1; BS §43.1) | none — read-only; proposals routed through `MutationBroker` | §74.7.1 |
| `ApiContractDriftDetector` | module | `nirman-android` | Statically compares client network interfaces and DTOs against OpenAPI specifications to detect schema drift (§74.7.2; BS §43.1) | none — pre-commit verification | §74.7.2 |
| `ThirdPartyIntegrationAnalyzer` | module | `nirman-android` | Validates third-party SDK wrappers, credential storage boundaries, circuit breakers, and webhook HMAC checks (§74.7.3; BS §43.1) | none — pre-commit verification | §74.7.3 |
| `AuthFlowSecurityHardener` | module | `nirman-android` | Audits mobile auth flows for OAuth 2.0 PKCE, token refresh mutexes, Keystore encryption, and route guards (§74.7.4; BS §43.1) | none — pre-commit verification | §74.7.4 |
| `AndroidDesignIntelligenceService` | service | `nirman-android` | Read-only aggregate query facade unifying design token compliance, visual QA, accessibility, string externalization, and dark pattern prevention (§73.16.1; BS §43.1) | none — read-only; proposals routed through `MutationBroker` | §73.16.1 |
| `VisualHierarchyAnalyzer` | module | `nirman-android` | Statically and dynamically evaluates contrast ratios, layout overflows, text truncations, and design token adherence (§73.16.2; BS §43.1) | none — pre-commit verification | §73.16.2 |
| `AndroidAccessibilityAuditor` | module | `nirman-android` | Audits native Android accessibility: 48dp touch targets, TalkBack contentDescription, focus order, and color-blind safety (§73.16.3; BS §43.1) | none — pre-commit verification | §73.16.3 |
| `StringExternalizationEngine` | module | `nirman-android` | Scans hardcoded string literals into strings.xml, enforces RTL mirroring, and validates plurals and locale formatting (§73.16.4; BS §43.1) | none — pre-commit verification | §73.16.4 |
| `DarkPatternDetector` | module | `nirman-android` | Statically scans UI compositions to detect pre-checked consent checkboxes, deceptive button contrast, and hidden cancellation flows (§73.16.5; BS §43.1) | none — pre-commit verification | §73.16.5 |
| `AndroidPrivacyIntelligenceService` | service | `nirman-android` | Read-only aggregate query facade unifying PII classification, personal data flow tracking, data minimization, privacy policy generation, and OSS notice composition (§70.7.1; BS §43.1) | none — read-only; proposals routed through `MutationBroker` | §70.7.1 |
| `PiiFieldClassifier` | module | `nirman-android` | Statically analyzes Room entity fields, Compose form inputs, and network DTOs to detect and classify PII fields (§70.7.2; BS §43.1) | none — analytical queries | §70.7.2 |
| `DataMinimizationChecker` | module | `nirman-android` | Audits detected PII and sensor access against the application's declared functional requirements (§70.7.3; BS §43.1) | none — analytical queries | §70.7.3 |
| `PrivacyPolicyGenerator` | module | `nirman-android` | Synthesizes a project-specific Privacy Policy document and Google Play Data Safety declaration draft (§70.7.4; BS §43.1) | none — analytical queries | §70.7.4 |
| `OpenSourceNoticeComposer` | module | `nirman-android` | Aggregates library licenses from `ResolvedDependency` and `SbomBuilder` metadata into `NOTICE.txt` and Compose viewer (§70.7.5; BS §43.1) | none — analytical queries | §70.7.5 |
| `AndroidAppObservabilityService` | service | `nirman-android` | Read-only aggregate query facade over structured logging, in-app crash reporting, metrics, tracing, diagnostics, and analytics schemas (§73.17.1; BS §43.1) | none — read-only; proposals routed through `MutationBroker` | §73.17.1 |
| `StructuredLoggingScaffolder` | module | `nirman-android` | Scaffolds structured Logcat/Timber wrappers, contextual tags, PII masking, and release R8 stripping rules (§73.17.2; BS §43.1) | none — analytical and scaffolding synthesis | §73.17.2 |
| `CrashReportingScaffolder` | module | `nirman-android` | Integrates client-side UncaughtExceptionHandler and local persistent crash caching in private app storage (§73.17.3; BS §43.1) | none — analytical and scaffolding synthesis | §73.17.3 |
| `AppMetricsScaffolder` | module | `nirman-android` | Instruments AndroidX Metrics, JankStats frame rendering listeners, and TTID/TTFD startup latency markers (§73.17.4; BS §43.1) | none — analytical and scaffolding synthesis | §73.17.4 |
| `TracingInstrumentationScaffolder` | module | `nirman-android` | Instruments AndroidX Tracing, Perfetto trace sections, and Compose recomposition tracking markers (§73.17.5; BS §43.1) | none — analytical and scaffolding synthesis | §73.17.5 |
| `InAppDiagnosticsScaffolder` | module | `nirman-android` | Scaffolds debug-variant Compose health dashboard and ZIP/Share Intent diagnostic report exporter (§73.17.6; BS §43.1) | none — analytical and scaffolding synthesis | §73.17.6 |
| `AnalyticsSchemaGenerator` | module | `nirman-android` | Synthesizes type-safe Kotlin sealed class analytics event hierarchies and abstract dispatcher interfaces (§73.17.7; BS §43.1) | none — analytical and scaffolding synthesis | §73.17.7 |
| `FeatureUsageTracker` | module | `nirman-android` | Scaffolds local feature adoption counters, first-use flags, and interaction frequency tracking via DataStore (§73.17.8; BS §43.1) | none — analytical and scaffolding synthesis | §73.17.8 |
| `AndroidPlatformTargetService` | service | `nirman-android` | Read-only aggregate query facade over permissions, Gradle config, shrinker rules, notification channels, deep links, and target SDK compliance (§73.18.1; BS §43.1) | none — read-only; proposals routed through `MutationBroker` | §73.18.1 |
| `ManifestPermissionDeriver` | module | `nirman-android` | Statically derives `<uses-permission>` tags and scaffolds modern ActivityResultContracts runtime permission flows (§73.18.2; BS §43.1) | none — analytical and scaffolding synthesis | §73.18.2 |
| `GradleConfigSynthesizer` | module | `nirman-android` | Scaffolds and reconciles Kotlin DSL build.gradle.kts, settings.gradle.kts, and libs.versions.toml version catalogs (§73.18.3; BS §43.1) | none — analytical and scaffolding synthesis | §73.18.3 |
| `ShrinkerRuleGenerator` | module | `nirman-android` | Synthesizes and validates ProGuard/R8 rules for kotlinx.serialization, Room entities, DAOs, and JNI preservation (§73.18.4; BS §43.1) | none — analytical and scaffolding synthesis | §73.18.4 |
| `NotificationChannelSetup` | module | `nirman-android` | Manages Android 8.0+ notification channel creation and Android 13+ POST_NOTIFICATIONS runtime permission flows (§73.18.5; BS §43.1) | none — analytical and scaffolding synthesis | §73.18.5 |
| `DeepLinkIntentFilterGenerator` | module | `nirman-android` | Synthesizes manifest `<intent-filter>` declarations and Navigation Compose 2.8+ type-safe deep link routes (§73.18.6; BS §43.1) | none — analytical and scaffolding synthesis | §73.18.6 |
| `TargetApiDeadlineTracker` | module | `nirman-android` | Validates targetSdk compliance against Google Play Store submission deadlines and flags impending deprecations (§73.18.7; BS §43.1) | none — analytical and compliance verification | §73.18.7 |
| `ApprovalGatePolicyEngine` | alias | — | alias of the three-outcome allow/ask/deny classifier inside `PolicyAuthority` (§16.2.1, §23.6, BS §23.7); the `RiskClassifier` sub-module (§44.3.1) feeds its risk-class input | — | §44.3.1 |
| `RiskClassifier` | module | `nirman-policy` | Deterministic risk-class classifier sub-module of `PolicyAuthority` assigning `ROUTINE`, `REVIEWABLE`, `PRIVILEGED`, or `HARD_GATED` to every incoming tool action before the allow/ask/deny decision; feeds the approval card risk explanation (§23.6) and BS §28.5 autonomy capability evidence | none — classification only; decision remains with `PolicyAuthority` | §44.3.1 |
| `DestructiveActionConfirmationFlow` | alias | — | alias of the `DENY` outcome path of `PolicyAuthority` (BS §23.7) that generates a `HARD_GATED` approval request for destructive operations; no separate state | — | §44.3.1 |
| `PauseResumeController` | alias | — | alias of the `PAUSE` / `RESUME` lifecycle transitions owned by `LifecycleAuthority` (`SessionReducer`, §45.1; BS §23.8, TA §5.1) | — | §44.3.1 |
| `ActionUndoCoordinator` | alias | — | alias of the file-tier and task-tier checkpoint restore path of `CheckpointManager` (§18, BS §27.6); the checkpoint system is the undo mechanism — no separate undo ledger exists | — | §44.3.1 |
| `MidRunEditCoordinator` | module | `nirman-control-plane` | Receives `workspace_file_saved` hook events while an autonomous task is active, validates against the active workspace lease, routes the mutation through `ConstructionTransactionManager`, and queues a dependency delta for the next `EVALUATE_PROGRESS` cycle (§44.3.1, BS §4.5) | none — routing only; writes committed by `ConstructionTransactionManager` | §44.3.1 |
| `ProgressNarrator` | module | `nirman-control-plane` | Read-only projection that subscribes to the `EventStore` event stream (§45.2) and converts typed task events into human-readable narrative progress strings surfaced via the `SupervisorConnection` (§57.3); holds no execution state and must not coerce event semantics | none — read-only projection | §44.3.2 |
| `DecisionExplanationGenerator` | alias | — | alias of the approval-card content producer inside `PolicyAuthority` that populates the required action, policy reason, risk explanation, and predicted side effect fields (§23.6, BS §4.6) | — | §44.3.2 |
| `DiffSummarizer` | module | `nirman-control-plane` | Converts a committed `ConstructionTransaction` record (§45.3) into a human-readable mutation summary (changed files, added/removed lines, affected symbols, motivating requirement); attached to the `ChangeReportRecord` obligation and projected to the UI | none — read-only transformation | §44.3.2 |
| `UncertaintyCommunicator` | module | `nirman-kernel` | Subscribes to `UncertaintyRegistry` and `ContradictionDetector` (§58.12) and surfaces unresolved uncertainty findings as structured decision nodes in the execution tree (§23.2); converts internal uncertainty entries to a user-facing `UncertaintyNotice` routed to `DecisionNodeManager`; does not resolve uncertainty | none — advisory surface only | §44.3.2 |
| `PlanPreviewRenderer` | alias | — | alias of the execution-plan projection derived from the `TaskGraph` (SCHEMAS §1.58) by `TaskGraphCompiler` and presented in the execution tree UI (§23.2, BS §4.3) | — | §44.3.2 |
| `CostTimeEstimatePresenter` | alias | — | alias of the cost-and-elapsed-time telemetry projection (TA §23.4, BS §23.8 activity panel) sourced from `ResourceGovernor` and `EventStore`; AI monetary spend is telemetry only and never an execution gate (ADR-218) | — | §44.3.2 |
| `LocalDecisionEngineProvisioner` | service | `nirman-control-plane` | Supervisor-owned provisioning service that installs, verifies, profiles, self-tests, and admits the bounded auxiliary decision engine (ADR-252); uses release-pinned manifests with SHA-256 verification and HTTPS acquisition | `local_decision_engine_profiles, local_decision_acceptance_profiles` | §49.5 |
| `LocalDecisionEngine` | service | `nirman-kernel` | Supervisor-local bounded inference service that accepts typed decision requests and returns `LocalDecisionProposal` records; has no filesystem, process, provider, emulator, policy, evidence, artifact, or completion authority; advisory output only | none — produces `LocalDecisionProposal` records consumed by decision/recovery/routing components | §58.17 |
| `LocalDecisionProposalValidator` | module | `nirman-kernel` | Validates typed decision results from `LocalDecisionEngine` against primitive domains and calibration state; rejects malformed or stale proposals before they reach decision/recovery/routing consumers | none — validation only; rejects invalid proposals | §58.17 |
| `BlockerReportGenerator` | module | `nirman-control-plane` | Monitors the task graph for `BLOCKED` / `USER_REQUIRED` requirement nodes (BS §27.10) and assembles a structured blocker report surfaced through the Action Center (BS §4.6) and `PARTIALLY_BLOCKED` path; does not set blocked decisions | none — read-only assembly | §44.3.2 |
| `QuestionBatchingEngine` | module | `nirman-kernel` | Implements the clarification gate of BS §69.11: groups, deduplicates, and prioritizes pending clarification questions before surfacing them as a single batched `ClarificationRequest`; routes answers back through `GoalInterpreter` as requirement-level updates | none — batching and routing only | §44.3.2 |
| `CompletionReportComposer` | module | `nirman-control-plane` | Assembles the final `TaskResult` (BS §23.9, §30, SCHEMAS §1.11) from `EvidenceAuthority` ledger, `EventStore` records, and `ArtifactAuthority` promotion records on completion decision; reads authoritative records only — does not produce evidence or decide completion | none — read-only composition | §44.3.2 |
| `OutputWalkthroughGenerator` | module | `nirman-control-plane` | Generates a human-readable post-task walkthrough artifact from `EpisodeRecord` (BS §28.3), evidence ledger, and committed event sequence after task completion or escalation; persisted as a documentation artifact in the task directory and linked from the completion report | none — read-only documentation artifact | §44.3.2 |
| `FeedbackIngestionPipeline` | service | `nirman-control-plane` | Supervisor-hosted coordinator that receives `FeedbackRecord` items from the UI, persists them to `feedback_records` through `EventStore` (§45.2), and routes each item to the appropriate feedback sub-component based on `FeedbackRecord.kind` (§44.3.3, SCHEMAS §2.127) | `feedback_records` (through `EventStore`) | §44.3.3 |
| `ChangeRequestParser` | module | `nirman-kernel` | Parses natural-language mid-session change requests (distinct from initial goals handled by `GoalInterpreter`) into structured `RequirementDelta` records (SCHEMAS §2.128) routed into the active `TaskGraph` through `LifecycleAuthority` (§44.3.3) | none — proposals only; graph updates committed by `LifecycleAuthority` | §44.3.3 |
| `ScreenshotAnnotationIngestor` | module | `nirman-control-plane` | Ingests annotated screenshots (user-drawn regions, labels, overlays) into `VisualSpecification` delta records; screenshot pixels stored in task directory, annotation metadata persisted through `EvidenceAuthority` (§44.3.3) | annotation metadata through `EvidenceAuthority` | §44.3.3 |
| `BugReportReproConverter` | module | `nirman-android` | Converts externally submitted bug descriptions (stack trace, Logcat excerpt, user symptom) into candidate `E2EScenario` reproduction specs (§62.1) suitable for the `Debugging Worker`; proposals only — `ScenarioSynthesizer` and `ToolBroker` govern admission and execution (§44.3.3) | none — proposals only | §44.3.3 |
| `RatingSignalCollector` | module | `nirman-control-plane` | Collects explicit user rating signals (thumbs-up/down, star, labelled satisfaction) and persists them as `FeedbackRecord.kind = RATING` items attached to `EpisodeRecord` entries; telemetry only — no task-state changes (§44.3.3, BS §28.3) | `feedback_records` entries through `FeedbackIngestionPipeline` | §44.3.3 |
| `ImplicitDissatisfactionDetector` | module | `nirman-control-plane` | Monitors `EventStore` for revert operations, repeated re-requests, consecutive checkpoint restores, and rapid edits to recently generated code, producing `FeedbackRecord.kind = IMPLICIT_DISSATISFACTION` items and advisory signals to `RecoveryAuthority` without altering task state (§44.3.3) | `feedback_records` entries through `FeedbackIngestionPipeline` | §44.3.3 |
| `FeedbackRequirementMapper` | module | `nirman-kernel` | Maps `FeedbackRecord` items to the most specific requirement nodes in the active `TaskGraph` using the requirement-node dependency graph and changed-file set; mapping is a proposal routed through `GoalInterpreter` (§44.3.3) | none — proposals only | §44.3.3 |
| `UserPreferenceLearner` | module | `nirman-context` | Extracts validated user preference facts from confirmed `EpisodeRecord` decisions and `FeedbackRecord.kind = CORRECTION` items; writes to `ProjectMemoryStore` through `MemoryWriter` (§59.1); never writes preferences inferred from model summaries alone — durable user action or explicit confirmation is required provenance (§44.3.3) | `MemoryRecord`s (through `MemoryWriter`) | §44.3.3 |
| `CodeStyleExtractor` | module | `nirman-android` | Analyzes committed source history and confirmed correction feedback to extract code style conventions; populates style-constraint fields in `SkillPackage` candidates (§19.1) that must pass full skill admission before becoming active constraints; never modifies existing admitted skills (§44.3.3) | none — proposals only; skill admission governed by §19.1 | §44.3.3 |
| `CorrectionMemoryStore` | module | `nirman-context` | Named partition of `ProjectMemoryStore` (§59.1) for user-confirmed correction facts; every entry must carry source `FeedbackRecord.feedbackId`, `EpisodeRecord.episodeId`, and the original agent output; `MemoryWriter` validation rules apply; model-generated statements without durable user confirmation are ineligible (§44.3.3) | `MemoryRecord`s scoped to correction partition (through `MemoryWriter`) | §44.3.3 |
| `FeedbackTriagePrioritizer` | module | `nirman-control-plane` | Prioritizes pending `FeedbackRecord` items from the `FeedbackIngestionPipeline` queue by feedback kind (CORRECTION > IMPLICIT_DISSATISFACTION > RATING), recency, affected-requirement criticality, and co-occurrence frequency; produces an ordered dispatch list — `TaskScheduler` and `PolicyAuthority` govern actual worker dispatch (§44.3.3) | none — ordering only | §44.3.3 |
| `ThreeWayAstMergeEngine` | module | `nirman-android` | Structural 3-way Tree-sitter AST merge reconciling base generated code, user manual edits, and agent synthesis (§47.4; BS §4.5) | none — proposal producer; commits via `MutationBroker` | §47.4 |
| `RegenerationSafeZoneMarker` | module | `nirman-android` | Statically parses and enforces protected code region boundaries (`// nirman:protected-start`) against mutation proposals (§47.4; BS §4.5, §43.2) | none — analytical safety check; evaluated by `MutationBroker` | §47.4 |
| `MicroLoopValidator` | module | `nirman-android` | Pre-transaction AST syntax validation and isolated incremental compilation check enforcing sub-5s feedback for Tier 1 micro-loop (§47.4; BS §52.3) | none — validation gate; evaluated before transaction staging | §47.4 |
| `AndroidThreatSketchSynthesizer` | module | `nirman-android` | Generates structured threat models, attack surfaces, and negative E2E validation scenarios for security-sensitive archetypes (§70.7.6; BS §58.2b) | none — analytical security model producer | §70.7.6 |
| `DependencyVulnerabilityAutomerger` | module | `nirman-android` | Bumps vulnerable patch dependencies in `libs.versions.toml` and orchestrates isolated smoke verification (§73.18.8; BS §58.3) | none — proposals committed via `MutationBroker` under `Security Worker` | §73.18.8 |

The §21 hierarchy resolves to these rows as follows: Lifecycle authority is `LifecycleAuthority`; Permission authority is `PolicyAuthority`; Sandbox authority is `PolicyAuthority` for the profiles of build spec §26.5, enforced by `ToolBroker`, `TerminalSupervisor`, and `WorkerRuntime` through restricted tokens and Job Objects; Storage authority is the SQLite execution ledger of §57.5, written only through `EventStore` and `ConstructionTransactionManager`; Evidence authority is `EvidenceAuthority`; Recovery authority is `RecoveryAuthority`; Promotion authority is `PreviewPromotionGate` for previews, `ArtifactAuthority` for artifacts, `CapabilityPromotionAuthority` for capability maturity, and `UpdateController` for self-update activation and rollback (§25.2). `Nirman.exe` hosts none of these rows; `NirmanWorker.exe` hosts only the `nirman-agents` rows; every other row runs inside `NirmanSupervisor.exe` (§3.5).

---

## 58. Agent Execution Kernel and Runtime Formalization

### 58.1 Module topology

The following modules make autonomous reasoning and execution explicit without creating a second runtime authority:

```text
GoalInterpreter
      ↓
TaskGraphCompiler
      ↓
AgentExecutionKernel
      ├── AgentLoopReducer
      ├── ProgressEvaluator
      ├── SkillRuntime
      ├── WorkerRuntime
      ├── SwarmPlanner
      ├── DelegationProtocol
      ├── KnowledgeLedger / TaskBlackboard
      ├── WorkspaceLeaseManager
      ├── ToolSessionRegistry
      ├── ToolCapabilityGraph
      ├── EnvironmentCapabilityPlanner
      ├── TargetPlatformResolver
      ├── PlatformCapabilityRegistry
      ├── ValidationPlanner
      ├── MutationRegressionAnalyzer
      ├── TrajectoryReplayEngine
      ├── SimulationExecutor
      ├── DeadlockDetector
      ├── WorkerAnomalyDetector
      ├── BackpressureController
      ├── CancellationPropagationManager
      ├── DecisionNodeManager
      ├── LocalDecisionEngine
      ├── LocalDecisionProposalValidator
      ├── UncertaintyRegistry
      ├── PlanCompiler / Replanner
      └── ExecutionHistoryManager
```

These modules produce proposals and state transitions, but LifecycleAuthority, PolicyAuthority, ToolBroker, ConstructionTransactionManager, EvidenceAuthority, and ArtifactAuthority remain the non-delegable authorities. The kernel runs in `NirmanSupervisor.exe`; the reasoning it drives runs in the worker's own `NirmanWorker.exe` process and reaches the kernel only as messages over the `WorkerConnection` (§3.5; §57.11).

- `WorkerAnomalyDetector` — The supervisor component that detects cognitive stalls, mutation thrashing, and schema deviation anomalies in active workers.

**Supervisor worker anomaly detection.** To protect workspace integrity and prevent compute thrashing from failing worker processes, `WorkerAnomalyDetector` monitors message streams across `WorkerConnection` during active leases:
1. *Cognitive stall anomaly (`COGNITIVE_STALL_ANOMALY`):* A worker sends heartbeats on Control Channel `0x00`, but emits zero proposals, observation requests, or evidence queries on Data Channel `0x01` for $> 180$ seconds while marked in an active reasoning state.
2. *Mutation thrash anomaly (`MUTATION_THRASH_ANOMALY`):* A worker emits $> 3$ consecutive AST mutation proposals modifying the identical AST node without acquiring new discriminating evidence or altering its error signature.
3. *Schema deviation anomaly (`SCHEMA_DEVIATION_ANOMALY`):* A worker emits $> 2$ consecutive malformed payloads failing `WorkerConnection` schema validation or containing unparseable JSON/bincode structures.
Upon detecting any of these three conditions, `WorkerAnomalyDetector` flags the attempt, revokes the worker's AppContainer lease via `WorkspaceLeaseManager`, records an `ANOMALY_REVOCATION` event in the execution ledger, and signals `RecoveryAuthority` to quarantine the attempt and dispatch a replacement worker under an escalated reasoning profile.

### 58.1.1 RepairOscillationDetector

`RepairOscillationDetector` operates within `WorkerAnomalyDetector` to prevent cyclic repair oscillation ("fix A breaks test B, fix B breaks test A"):
1. *Cycle detection across checkpoints:* Tracks the multi-transaction history of modified symbol anchors and failing test assertions. If a patch targeting failure $F_1$ causes test $T_2$ to fail, and the subsequent patch targeting failure $F_2$ restores the error condition of $T_1$, an `OSCILLATION_DETECTED` anomaly is raised immediately.
2. *Oscillation mitigation:* Halts further micro-patching on the oscillating symbols, revokes the active lease, and forces `RecoveryAuthority` to bypass Level 1/2 micro-repairs and escalate directly to Level 4 (checkpoint rollback and alternative architectural design) on the canonical recovery ladder (§28).
3. *Telemetry recording:* Persists the oscillating symbol set and mutually conflicting test IDs in `ProjectMemoryEntry` (SCHEMAS §2.101) to prevent repeating the cyclic repair in future sessions.

- `WorkerFailoverReconstitutionProtocol` — The deterministic protocol executed by the supervisor when recovering from a worker crash, anomaly eviction, or preemption event.

**Worker failover reconstitution protocol.** When a worker process crashes, times out, is evicted by `WorkerAnomalyDetector`, or is preempted by `SupervisorPreemptionProtocol`, the supervisor executes this deterministic failover sequence:
1. *Isolation & quarantine:* Revokes the prior lease in `WorkspaceLeaseManager`, terminates any lingering process handles via `TerminateJobObject`, and isolates any uncommitted filesystem mutations into a quarantined patch branch (`quarantine/attempt-<id>`).
2. *Clean context seeding:* Generates a fresh `ContextPackage` seeded per §27.4:
   - Restores workspace files to the last validated `Checkpoint`;
   - Injects the original user intent, task specification, and active interface agreements;
   - Records the prior failure fingerprint and quarantined patch signatures as active negative constraints to prevent repeat failures;
   - Attaches the current `AndroidSymbolGraph` and fresh evidence ledger watermark.
3. *AppContainer allocation:* `WorkerRuntime` allocates a fresh `NirmanWorker.exe` AppContainer process under a newly generated `leaseId` with an incremented `attemptId`, a new unique container SID, and a dedicated named-pipe `WorkerConnection`.
4. *Direct resumption:* The replacement worker initiates its lifecycle directly at the `PLAN` phase of the reasoning cycle, bypassing already completed and verified exploratory stages.
5. *Causal audit trail:* Emits a `WORKER_FAILOVER_COMMITTED` event to the SQLite execution ledger linking `priorWorkerId` $\to$ `evictionReason` $\to$ `replacementWorkerId`, ensuring transparent lineage without losing task context.

### 58.2 AgentExecutionKernel contract

> **Schema projection:** `AgentExecutionKernel` is defined in `nirman-schemas.md` §2.41. Owner: TA §58.2.

The kernel loop is:

```text
OBSERVE
  ↓
UNDERSTAND
  ↓
PLAN
  ↓
SELECT_ACTION
  ↓
AUTHORIZE
  ↓
EXECUTE
  ↓
OBSERVE_RESULT
  ↓
UPDATE_STATE
  ↓
EVALUATE_PROGRESS
  ├── CONTINUE
  ├── RECOVER
  ├── DELEGATE
  ├── REPLAN
  └── COMPLETE
```

`SELECT_ACTION` applies the frontier-first rule of build spec §52.3 (ADR-225): the kernel reads the `EvidenceFrontier` slice for the active requirement and the open hypotheses from `HypothesisManager` (§71.5); while an `UNRESOLVED`, `CONTRADICTED`, or `REQUIRED_VALIDATION` item or an untested discriminating test exists, only observation proposals are admissible, and a mutation proposal is answered `EVIDENCE_NOT_ACQUIRED` without reaching `PolicyAuthority`. Every admitted mutation `AgentProposal` carries `targetFrontierItemId` and `motivatingEvidenceId`; `ProgressEvaluator` treats a cycle that acquired evidence as progress even when no file changed, and a cycle that changed files against an unobserved frontier as none.

`AgentLoopReducer` is the kernel's deterministic step function: it folds a cycle outcome (§71.4) into the next proposed task-execution state and emits that proposal as a validated kernel event. It commits nothing. The only committer of a lifecycle transition is `LifecycleAuthority`, the `SessionReducer` of §45.1 (build spec §33.2; ADR-159), which accepts or rejects the kernel's proposal like any other event. A provider delta, partial stream, worker message, or UI action may request a transition but cannot apply one directly.

### 58.3 Durable schemas

> **Schema projection:** `AgentLoopRecord` is defined in `nirman-schemas.md` §2.42. Owner: TA §58.3.

> **Schema projection:** `AgentProposal` is defined in `nirman-schemas.md` §2.43. Owner: TA §58.3.

> **Schema projection:** `AgentProfile` is defined in `nirman-schemas.md` §2.44. Owner: TA §58.3.

A proposal is immutable after validation. Any change creates a new proposal revision linked to the evidence or contradiction that caused it.

### 58.4 SkillRuntime

`SkillRuntime` resolves skill discovery, compatibility, composition, input binding, context assembly, execution, tool mediation, output validation, and evidence capture. It verifies skill version, required ToolBroker version, Android profile, worker role, input/output schema, permissions, and resource requirements before execution. Input/output verification compares the invocation's declared names against the manifest and the body's `Emits` line, which the verifier pins identical; conformance to the `## Output contract` bullets is a worker instruction-following obligation.

> **Schema projection:** `SkillExecutionRecord` is defined in `nirman-schemas.md` §2.45. Owner: TA §58.4.

A skill composition is a directed acyclic graph with bounded depth, explicit inputs/outputs, shared revision identity, and a single validation contract. A composed skill cannot grant another skill permissions.

### 58.5 SwarmPlanner and delegation

`SwarmPlanner` analyzes change surface, dependencies, symbols, requirements, risk, validation cost, capability graph, workspace capacity, emulator availability, provider concurrency, physical resource pressure, recent validated worker outcomes, task-graph join semantics, and recovery/validation reserve. It emits a `SwarmPlan` bound to graph/project/plan revisions. Before launch, `InterfaceAgreement` completeness is required and every assignment is admitted under the current lease/fencing epoch. Historical outcome feedback is advisory only.

> **Schema projection:** `SwarmPlan` is defined in `nirman-schemas.md` §2.114. Owner: TA §58.5.

`DelegationProtocol` supports:

```text
delegate(request)
spawn(worker_instance)
handoff(contract)
resume(scope)
cancel(scope)
replace(worker)
retry(strategy)
escalate(reason)
merge(results)
```

- `GoalHierarchyGenerator` — The deterministic planner service that derives four-tier goal trees from accepted requirements.
- `ProjectMilestonePlanner` — The runtime planning service that organizes generated Android project construction into ordered milestones.
- `TaskBatchingOptimizer` — The planning component that fuses co-located micro-mutations into atomic composite tasks.

**Goal hierarchy generation.** `GoalHierarchyGenerator` structures accepted requirements into a four-tier directed acyclic hierarchy:
$$\text{L0: RootUserGoal} \longrightarrow \text{L1: FeatureCapability} \longrightarrow \text{L2: AcceptanceCriteria} \longrightarrow \text{L3: TaskNode}$$
- *L0 RootUserGoal:* The overarching product intent synthesized from the user's instructions.
- *L1 FeatureCapability:* Discrete functional capabilities (e.g., Workout Logging, GPS Route Recording, Offline Sync, User Preferences).
- *L2 AcceptanceCriteria:* Specific observable behavioral conditions verifiable by test assertions.
- *L3 TaskNode:* Atomic code mutations, layout creations, or configuration patches executed by individual worker instances.
An L0 goal is marked satisfied if and only if all L1 capabilities are verified; an L1 capability is satisfied if and only if all child L2 criteria have passing evidence in the `EvidenceLedger`.

**Runtime project milestone planning.** Construction of a generated Android application is orchestrated by `ProjectMilestonePlanner` through a seven-phase sequence:
1. *Phase 1: Baseline Scaffold & Manifest:* Root build configurations, AGP toolchain lock, compileSdk/minSdk settings, and base `AndroidManifest.xml`.
2. *Phase 2: Local Persistence & Entities:* Room `@Database`, `@Entity` definitions, `@Dao` interfaces, and DataStore preferences.
3. *Phase 3: Core UI Screens & Design Tokens:* Material 3 themes, reusable Composable components, and screen scaffolds.
4. *Phase 4: Navigation Graph & State Management:* Jetpack Navigation Compose type-safe routes, ViewModels, and StateFlow streams.
5. *Phase 5: Background Work & Integrations:* AndroidX WorkManager workers, Retrofit network clients, and runtime permission flows.
6. *Phase 6: E2E Autonomous Scenario Verification:* Headless emulator exploration, UI interaction crawls, and fault injection tests.
7. *Phase 7: Release Packaging & Verification:* Signed APK / optional AAB generation, manifest merger checks, and installation validation.

**Task batching optimization.** To prevent process spawning overhead and lease contention from fine-grained mutation tasks, `SwarmPlanner` employs `TaskBatchingOptimizer`:
- *Co-location clustering:* Identifies micro-mutation tasks targeting the same file lease (e.g., adding several localized string resources in `res/values/strings.xml`, or adding adjacent UI vector assets in `res/drawable/`) or tightly-coupled files within the same Android package.
- *Composite task synthesis:* Fuses eligible micro-tasks into a single composite task node executed by one worker pass under a single `ConstructionTransaction`.
- *De-batching fallback:* If the composite mutation fails compilation or AST validation, `TaskBatchingOptimizer` automatically dissolves the batch back into granular individual tasks, isolating the failing edit for precise L1/L2 repair without cascading failures.

### 58.5.1 Swarm admission, joins, and outcome feedback

`SwarmAdmissionController` is a scheduler component, not an authority. It evaluates the existing physical `ResourceIntegrityAuthority`, task queue depth, parent-child concurrency, emulator slots, provider concurrency, and reserved recovery/validation capacity. It may queue, reduce concurrency, repartition, or serialize work.

Task graph fan-in uses `ALL`, `ANY`, or `QUORUM(n)` semantics. `OPTIONAL` work never blocks a dependent requirement unless the graph explicitly marks the dependency `HARD`. Failure propagation follows the node's declared `dependencyFailurePolicy`.

**Step priority ranking.** When parallel tasks or sub-agents compete for worker leases, emulator instances, or provider concurrency slots, `SwarmAdmissionController` schedules work strictly by priority tiers:
1. *P0 (Build and Compile Blockers):* Toolchain acquisition, Gradle wrapper alignment, missing core class definitions, or syntax errors preventing compilation.
2. *P1 (Schema and Data Contracts):* Room database entities, DAOs, repository interfaces, and shared data models required by downstream screens.
3. *P2 (Core UI and Navigation):* Primary screen composables, top-level navigation routes, and interactive input elements.
4. *P3 (Visual Polish and Transitions):* Edge-to-edge system bars, micro-animations, color styling, and non-blocking layout refinements.
5. *P4 (Telemetry and Documentation):* Non-functional logging, README updates, and internal code annotations.

`AgentQualityScorer`/historical outcome data may influence worker/profile selection only as advisory input. It cannot modify permissions, evidence requirements, or completion.

Every operation carries parent task, cancellation lineage, input references, expected outputs, required capabilities, profile, permissions, resource reservation, workspace lease, and validation requirements. Dynamic worker creation is bounded by policy and never changes the authority graph.

> **Schema projection:** `JoinBarrierState` is defined in `nirman-schemas.md` §2.121. Owner: TA §58.5.1.

Fan-in is durable: every join carries `JoinBarrierState` (expected, completed, and failed children; accepted results; quorum count; join revision; join state). A parent MAY wake only when its join contract becomes satisfiable (ADR-247); satisfaction is evaluated by the Supervisor control plane and recorded with the satisfying event, never inferred from transport traffic. When a join barrier reaches a terminal state (`SATISFIED`, `CANCELLED`, or `SUPERSEDED`), its outcome is immutable for that `joinRevision`. Child results arriving after barrier satisfaction are recorded as non-mutating completion records in `quarantined_messages` and never alter the satisfied barrier or re-wake the parent; child outputs produced after a join is `CANCELLED` remain quarantined under §58.12.1 and cannot promote without independent revalidation.

### 58.6 KnowledgeLedger and TaskBlackboard

`KnowledgeLedger` stores typed, scoped `KnowledgeArtifact` records. `TaskBlackboard` is a task-scoped projection containing the goal, requirements, architecture, decisions, constraints, assumptions, active workers, completed/blocked work, findings, conflicts, evidence, known failures, and next actions. A separate graph database is not implied. When typed relationships are required, the ledger may store:

> **Schema projection:** `KnowledgeRelation` is defined in `nirman-schemas.md` §2.46. Owner: TA §58.6.

`KnowledgeRelation` is a scoped projection edge and never grants authority. It must not allow identifiable project content to cross the memory boundary.

Workers may read, propose, attach evidence, request changes, and retrieve relevant entries. Only authoritative services may commit a decision, change the task graph, mark a requirement complete, change policy, or promote an artifact.

Shared-memory selection is explicit, never implicit. A `KnowledgeArtifact` records how it entered the ledger and how it leaves it. `promotionDecision` is `PROPOSED` while the artifact awaits an authoritative commit, `ADMITTED` once an authoritative service has committed it, `REJECTED` when authority declined it, and `SUPERSEDED` once a later artifact displaces it. `promotionRationale` states why that decision was taken, so a supersession is readable without replaying the ledger. `selectionRegime` names the mechanism by which the artifact competed for its place: `SOLE_SOURCE` when it is the only artifact for its scope, `COMPETITIVE_SELECTION` when a selection rule chose it among materially different candidates, `EVICTION` when a bounded store removed it to admit another, and `SUPERSESSION` when a newer artifact replaced it. When `supersededBy` names a successor, `promotionDecision` MUST be `SUPERSEDED` and the successor MUST NOT itself be superseded — a supersession chain is acyclic.

Selection pressure introduces two failure modes that no proposing worker may resolve: **drift**, where a retained artifact's supporting evidence weakens until the artifact no longer warrants retention, and **poisoning**, where a fabricated or stale artifact propagates because no independent authority ever re-evaluated it. `EvidenceAuthority` owns the defence against both. On any evidence invalidation, revocation, or expiry that reaches an artifact's `evidence_ids`, `EvidenceAuthority` MUST re-evaluate the artifact: it either re-admits the artifact on remaining valid evidence or marks it `SUPERSEDED` through `supersededBy`. Temporal decay of an artifact's evidence is evaluated on the same authority boundary. A worker may propose, attach evidence to, or request change of an artifact, but a worker MUST NOT promote, supersede, or evict one. Drift defence and poisoning defence are `EvidenceAuthority` responsibilities and introduce no new authority (§44.1, §57.12).

> **Schema projection:** `KnowledgeArtifact` is defined in `nirman-schemas.md` §2.47. Owner: TA §58.6.

### 58.7 WorkspaceLeaseManager and ToolSessionRegistry

`WorkspaceLeaseManager` gives every worktree or copy-on-write workspace one owner, one parent checkpoint, a renewable heartbeat, an expiration, cleanup rules, and recovery rules. Stale leases become recoverable resources only after process and revision checks.

`ToolSessionRegistry` represents terminals, ADB, emulators, debuggers, LSPs, preview processes, and other long-lived tools as reconnectable sessions:

> **Schema projection:** `ToolSession` is defined in `nirman-schemas.md` §2.48. Owner: TA §58.7.

A tool session may be reattached after worker replacement or UI restart, but reattachment does not expand its capability scope.

### 58.8 Tool Capability Graph and environment planning

`ToolCapabilityGraph` maps an outcome to capability requirements, skills, worker profiles, tools, and environment prerequisites. For example, Android BLE validation may require Android APIs, a compatible SDK, a native module, Bluetooth permissions, ADB, an emulator or selected Nirman-managed local Android emulator, and device-test capability.

`EnvironmentCapabilityPlanner` evaluates each prerequisite before expensive execution and classifies it as `AVAILABLE`, `REPAIRABLE`, `USER_REQUIRED`, or `UNAVAILABLE`. It records the toolchain lock, environment fingerprint, repair attempt, and evidence used for the classification.

Platform dimensions are explicit (build spec §79). The planner resolves host and target platforms through `TargetPlatformResolver`, consults the `PlatformCapabilityRegistry` matrix as a prior for preflight, and classifies cross-compilation capability and native target-runtime capability as separate prerequisites. It never derives native runtime capability from a successful build or cross-build: the cross-build admission decision point and the native-validation gate are the §84.3 decision points owned by the existing authorities, and a classification is never raised by model assertion.

- `ToolReliabilityScorer` — The scoring service that tracks empirical tool execution success and failure rates.

**Tool reliability scoring and fallback routing.** `ToolReliabilityScorer` computes an empirical reliability score $R(T) \in [0.0, 1.0]$ for each tool $T$ based on historical execution outcomes:
$$R(T) = \frac{\text{SuccessCount}(T)}{\text{TotalInvocations}(T) + 2}$$
When $R(T) < 0.60$, `ToolCapabilityGraph` marks tool $T$ as `DEGRADED` and automatically routes execution requests to registered fallback tools or alternative strategies. Tool execution cost awareness applies strictly to physical host resources (disk space, execution timeout, memory limits, emulator lock contention) under `ResourceIntegrityAuthority`; AI token and monetary costs have zero authority over tool execution or task continuation (ADR-218).

### 58.9 ValidationPlanner and mutation regression analysis

`ValidationPlanner` chooses validation from changed files, symbols, call graph, route graph, dependency graph, requirement traceability, project type, risk, previous failures, emulator profiles, and resource availability. `MutationRegressionAnalyzer` predicts affected behavior and expands validation when a change touches a manifest, permission, navigation route, data model, native module, build file, authentication boundary, or shared UI component.

> **Schema projection:** `ValidationPlan` is defined in `nirman-schemas.md` §2.49. Owner: TA §58.9.

### 58.10 TrajectoryReplayEngine and SimulationExecutor

`TrajectoryReplayEngine` replays recorded observations, structured proposals, tool calls, tool results, state transitions, evidence references, and next decisions against a new model, prompt, skill, schema, or runtime. Replay is read-only with respect to real projects and cannot send external side effects.

`SimulationExecutor` produces a dry-run plan with predicted workers, skills, files, commands, permissions, devices, tests, resources, and risks. It uses explicit statuses: `PREDICTED`, `SIMULATED`, `OBSERVED`, and `VERIFIED`. It must not mutate source files, execute commands, start an emulator, or claim that a predicted test passed.

### 58.11 Deadlock, backpressure, and cancellation

> **Schema projection:** `AwaitCondition` is defined in `nirman-schemas.md` §2.120. Owner: TA §58.11.

`DeadlockDetector` MUST detect dependency, worker-wait, reservation, approval, workspace, and ToolSession cycles. Reservation acquisition is atomic or globally ordered. A detected cycle produces `CoordinationStallRecord` or a deadlock finding and routes to reorder, replacement, serialization, lease recovery, cancellation, replanning, or recovery.

`BackpressureController` reserves and queues Gradle processes, emulator slots, Nirman-managed local Android emulators, GPU capacity, storage, and provider concurrency. It applies priority and fairness, exposes waiting reasons, and reduces parallelism before system pressure becomes failure.

`CancellationPropagationManager` propagates cancellation from goal to task graph, workers, skills, ToolSessions, child processes, PTY, emulator actions, and pending provider requests. Each node supports graceful cancellation, forced termination, cleanup, checkpoint preservation, and rollback semantics.
`CancellationPropagationManager` additionally quiesces child dispatch, prevents new messages from entering a cancelled descendant, releases reservations after cancellation reaches the descendant, preserves produced artifacts, and seals the cancelled attempt. Cancellation is cooperative first: `CancellationPropagationManager` dispatches a `CANCEL` message over the reserved control lane (§58.11.2). A cancellation unacknowledged within the worker stale threshold (60 seconds, build spec §26.3, §52.12) escalates to forced termination via `TerminateJobObject`, sealing the attempt and releasing all workspace leases and semantic reservations.

Independent worker or skill pause must preserve context references, leases, ToolSessions, checkpoints, and unresolved questions. Unrelated workers may continue.

### 58.11.1 Asynchronous waiting via `AwaitCondition`

> **Schema projection:** `AwaitCondition` is defined in `nirman-schemas.md` §2.120. Owner: TA §58.11.

No agent waits on an agent. Every cross-worker wait becomes a durable `AwaitCondition` with an explicit predicate, owner (the waiting worker), cancellation lineage, and wake condition (ADR-247): the producer emits its event or result and the Supervisor wakes the waiter only when the predicate becomes satisfiable. Synchronous call-and-block chains (A waits on B waits on C waits on A) cannot be expressed, which removes an entire class of orchestration deadlocks. `CANCELLED` and `SUPERSEDED` await conditions wake deterministically with their reason.

### 58.11.2 Reserved control lane

`CANCEL`, `FENCE`, `REPLACE`, `PLAN_SUPERSEDED`, `RECONCILE`, and `RECOVER` cross a reserved-capacity control lane (`nirman-schemas.md` §2.90 `reservedControlLane`), not merely a higher-priority queue (ADR-246). Bulk traffic can never occupy the lane's capacity, so control delivery is bounded even under payload saturation. The lane changes delivery guarantees only; it grants no authority. These six kinds are the published preemption subset of lane `0x00`; `HEARTBEAT` crosses the lane as well — it is part of `controlMessageKinds` (`nirman-schemas.md` §2.90) and of the connection-control subset (technical architecture §57.11) — but it is not part of this preemption subset.

- `SupervisorPreemptionProtocol` — The supervisor protocol that deterministically revokes worker leases, invalidates write capabilities, and preempts stalled or anomalous processes.

**Supervisor preemption protocol.** When the supervisor detects a premise invalidation (`PREMISE_MISMATCH`), an anomaly (`WorkerAnomalyDetector`), a hard safety boundary violation, or an explicit user cancellation, `SupervisorPreemptionProtocol` executes atomic preemption:
1. *Fenced control notice:* Emits a high-priority `PREEMPT` control notice across Reserved Control Lane `0x00` carrying the eviction reason, target lease ID, and cancellation watermark.
2. *Atomic capability severance:* `WorkspaceLeaseManager` immediately marks the active lease `PREEMPTED` in the SQLite ledger, and `ToolBroker` / `ConstructionTransactionManager` reject any in-flight mutation tokens from that worker as `LEASE_FENCED`.
3. *Process termination & containment:* Gives the worker a 500ms grace window to flush its in-memory telemetry, after which the supervisor terminates the worker's Job Object via `TerminateJobObject`. Uncommitted workspace edits are quarantined in an isolated recovery branch, preventing half-applied modifications from leaking into the primary workspace.
4. *Audit logging:* Writes an immutable `PREEMPTION_EVENT` to the execution ledger containing causal trigger details, invalidated token counts, and downstream recovery requirements.

### 58.11.3 Delivery recovery

On reconnect (`reconnectPolicy: RESUMABLE`) the Supervisor reconciles each connection from durable mailbox/order watermarks: `PENDING` redispatches; in-flight reconciles and redelivers once; `APPLIED` never reapplies; unknown states reconcile before any new dispatch on that stream. Duplicate deliveries are detected by `messageId`/`deduplicationKey` plus immutable payload fingerprint; a conflicting duplicate is rejected and quarantined (build spec §26.2).

### 58.12 DecisionNodeManager, uncertainty, and replanning

> **Schema projection:** `PremiseInvalidationRecord` is defined in `nirman-schemas.md` §2.123. Owner: TA §58.12.

`DecisionNodeManager` represents ambiguous architecture or recovery choices with a question, options, evidence, trade-offs, recommendation, impact, and resume conditions. A decision node is separate from a generic command approval and remains bound to a task and plan revision.

**Deterministic decision scoring equation.** When competing architectural strategies, recovery branches, or technology proposals are evaluated, `DecisionNodeManager` calculates a deterministic score for each alternative $A$:
$$\text{Score}(A) = w_e \cdot \text{EvidenceCoverage}(A) + w_r \cdot (1 - \text{RegressionRisk}(A)) - w_u \cdot \text{UncertaintyPenalty}(A) - w_c \cdot \text{ComplexityWeight}(A)$$
where all metrics are normalized to $[0.0, 1.0]$:
- $\text{EvidenceCoverage}(A)$: Proportion of requirements with verified pass evidence.
- $\text{RegressionRisk}(A)$: Graph density and historical failure rate of touched modules.
- $\text{UncertaintyPenalty}(A)$: Ratio of `UNKNOWN` or `ASSUMED` dependencies in the closure of $A$.
- $\text{ComplexityWeight}(A)$: Cyclomatic complexity and number of cross-module interface boundaries.
- Default normalized weights: $w_e = 0.35$, $w_r = 0.25$, $w_u = 0.25$, $w_c = 0.15$ (summing to 1.0).
Ties are broken deterministically by selecting the alternative with the smaller AST modification delta.

`UncertaintyRegistry` tracks `KNOWN`, `PROBABLE`, `ASSUMED`, `UNKNOWN`, `CONTRADICTED`, `VERIFIED`, and `BLOCKED` facts with source, confidence, evidence, expiry, scope, and next action. `ContradictionDetector` creates a controlled decision revision when requirements, assumptions, device constraints, toolchains, or architecture facts conflict.

`PlanCompiler` and `Replanner` compile a new plan when evidence invalidates the current one. Each revision records `planRevision`, `supersedesPlan`, reason, trigger evidence, affected nodes, and recovery/migration action.

When `Replanner` creates a new plan revision, it invokes `PlanAssignmentMigrator`. The migrator classifies every active assignment as RETAIN, REBASE, QUIESCE, CANCEL, or REPLACE. REBASE requires a new context-integrity check and interface-agreement check. REPLACE fences the old lease before launch. An old plan may not produce a consequential proposal after migration.

### 58.12.1 Premise invalidation, propagation, and quarantine

> **Schema projection:** `PremiseInvalidationRecord` is defined in `nirman-schemas.md` §2.123. Owner: TA §58.12.

A falsified premise must become durable authoritative state and propagate to every currently dependent active assignment; no worker may continue consequential work until that dependency is revalidated or its plan/work is reconciled (ADR-249). The pipeline is: evidence proposal → authoritative admission → invalidation record → dependency propagation → active-assignment marking → quarantine → revalidation OR replan/migrate. The proposing evidence may come from a worker or from an authoritative/kernel observation path (`proposedByWorkerId?`); admission and marking travel the existing kernel authority path above: an UncertaintyRegistry fact becomes `CONTRADICTED` or `BLOCKED`, dependent nodes are resolved from the task graph, and every affected active assignment carries `assignmentValidity: INVALIDATED | REVALIDATION_REQUIRED` and `invalidatedByRecordId` independently of lifecycle status and lease/fencing state.

Output quarantine is the default: evidence and artifacts produced under the falsified premise are quarantined (`quarantinedEvidenceIds`, `quarantinedArtifactIds`). Compatibility may bypass replan only when the existing dependency graph proves independence (§36.4); discard requires proof that the affected output cannot be safely revalidated.

Push + checkpoint/epoch reconciliation are mandatory dual paths. Push is notification; the durable record is authority. Notification reuses the reserved control lane `RECONCILE` message (§58.11.2) with payload discriminator `reconcileReason = PREMISE_INVALIDATION` and `invalidationId`; the record MUST commit before dispatch, and checkpoint reconciliation remains mandatory because push delivery can be missed. No new authority, component, or control kind is introduced.

- `PlanningDeadEndDetector` — The detection service that identifies cyclic failure states and exhausted branches in the task graph.

**Planning search-tree dead-end detection.** `PlanningDeadEndDetector` monitors task graph execution paths to prevent cyclic failure loops:
- *State cycle detection:* If an action sequence $A_1 \to A_2 \to \dots \to A_k$ reproduces an identical failure signature, error diagnostic, or workspace Merkle digest as an earlier failed state, the sequence is classified as a `STATE_CYCLE`.
- *Branch exhaustion:* If all permissible tool actions or mutation operators for a task node produce either a `STATE_CYCLE` or a rejected hypothesis, the branch is marked `DEAD_END`.
- *Recovery action:* On dead-end classification, `PlanningDeadEndDetector` immediately halts the active branch, quarantines its intermediate outputs, and invokes `Replanner` to backtrack to the last stable checkpoint and prune that branch from future exploration.

### 58.13 Coordination progress

> **Schema projection:** `CoordinationStallRecord` is defined in `nirman-schemas.md` §2.118. Owner: TA §58.13.


`CoordinationProgressMonitor` records `CoordinationStallRecord`. A swarm is making coordination progress only when at least one authoritative frontier item is reduced, a dependency is resolved, validated evidence is added, a project/plan revision advances, or an integration checkpoint is accepted. Heartbeats and message traffic alone are insufficient.

When the configured coordination window has elapsed without qualifying progress, the monitor routes through existing RecoveryAuthority. It MUST NOT terminate a healthy goal merely because elapsed time passed.

`CoordinationProgressMonitor` also detects livelocks, not only stalls (ADR-247): the same coordination signature — graph state, plan revision, frontier, evidence watermark, failure fingerprint, and strategy — repeated `repeatThreshold` times is a coordination cycle (`detectionKind: LIVELOCK`). A coordination cycle routes through the recovery ladder REPLAN → REPARTITION → SERIALIZE → REPLACE → BACKTRACK → ESCALATE; workers are never permitted to loop by resending near-identical coordination messages.

### 58.14 ExecutionHistoryManager

> **Schema projection:** `ExecutionEpoch` is defined in `nirman-schemas.md` §2.119. Owner: TA §58.14.


`ExecutionHistoryManager` separates active state from retained history using semantic indexing inside each tier rather than relying on unstructured text summaries:

| Tier | Semantic Indexing and Content | Access |
|---|---|---|
| Hot | Current WorkingState, active edit set, active graph frontier, current evidence frontier, active failures, current plan | Kernel context |
| Warm | Recent causal chains, recent observations, recent successful/rejected strategies, checkpoints, preview/test results | Task request |
| Cold | Historical causal graph, superseded plans, older failures, architectural decisions, screenshots | Indexed retrieval or replay |
| Archived | Replayable raw history, full traces, old artifacts, crash dumps, retired sessions | Explicit audit restore |

Compaction must preserve semantic summaries, evidence links, revision identity, artifact provenance, and replay references. Model-generated summary text is never the canonical memory; memory records require validated provenance from the execution ledger. Never make a summary the sole surviving representation of authoritative state. Garbage collection cannot delete active checkpoint parents, mandatory completion evidence, unresolved failure evidence, or artifact provenance. Tiering controls retrieval priority and representation, not authority. Archived data remains authoritative evidence when explicitly restored.

Execution history is divided into explicit `ExecutionEpoch`s. Epoch rollover is a semantic continuation boundary, not a new task. A new epoch is created only after the prior epoch's continuation snapshot, event watermark, pending messages, unresolved effects, active leases, and required evidence references are durably sealed. Replay of the new epoch must not re-execute an effect already completed in the predecessor epoch.

### 58.15 Causal Execution Memory

The runtime models autonomous problem solving as a durable causal sequence across thousands of actions. Every meaningful action is structured as:

```text
Observation
 → Interpretation
 → Hypothesis
 → Decision
 → Action
 → Result
 → Evidence
 → Consequence
```

Deliberation checkpoints, rejected strategies, alternative hypotheses, and proven AST repair patches from `EpisodicRepairPatternCatalog` (§47.4) are integrated directly into the `MemoryStore` and `ContextOrchestrator` rather than operating as an isolated parallel subsystem. Each causal node records its input observation, explanatory hypothesis, policy decision, executed action, observable result, resulting evidence item, and downstream project consequences.

### 58.16 Runtime invariants

1. Only `LifecycleAuthority` (the `SessionReducer`, §45.1) commits lifecycle state; `AgentLoopReducer` proposes.
2. Only the ToolBroker executes tools.
3. Only PolicyAuthority grants capabilities.
4. Only ConstructionTransactionManager mutates the project.
5. Only EvidenceAuthority confirms completion.
6. Only ArtifactAuthority promotes APK output.
7. Replay and simulation are side-effect free.
8. Dynamic workers and skills cannot expand permissions.
9. A stale lease cannot write to a workspace.
10. Cancellation reaches every descendant execution node.
11. A predicted result cannot be represented as observed evidence.
12. History compaction cannot remove required proof.
13. No agent waits on an agent; every cross-worker wait is a durable `AwaitCondition` (§58.11.1).
14. No message is authoritative by itself; only Supervisor-committed state transitions are.
15. No acknowledgement means "applied" unless the authoritative state transition is durable (§57.11.2).
16. No worker continues execution from a superseded epoch or revision.
17. No repeated coordination state may loop forever (§58.13 livelock rule).
18. Local auxiliary inference is advisory and never authoritative.
19. Failure or absence of the optional local auxiliary engine MUST NOT invalidate the core runtime contract.

### 58.17 LocalDecisionEngine and LocalDecisionProposal lifecycle

`LocalDecisionEngine` is a supervisor-local bounded inference service. It accepts typed decision requests and returns `LocalDecisionProposal` records.

It has no direct workspace access, no direct filesystem authority, no provider credential access, no network authority, no emulator authority, no policy authority, and no promotion authority.

> **Schema projection:** `LocalDecisionEngineProfile` is defined in `nirman-schemas.md` §2.129. Owner: TA §49.5.
>
> **Schema projection:** `LocalDecisionProposal` is defined in `nirman-schemas.md` §1.79. Owner: BS §66.10.1.
>
> **Schema projection:** `LocalDecisionAcceptanceProfile` is defined in `nirman-schemas.md` §1.80. Owner: BS §66.10.1.

The local execution path is:

```text
Decision request
→ profile admission
→ health/resource admission
→ input/context fingerprint
→ local inference
→ typed result validation
→ acceptance-profile evaluation
→ LocalDecisionProposal
→ consumer-declared fallback or deterministic consumer
```

The proposal is valid only while its profile identity, model revision, input revision, context package identity, state hash, purpose, calibration state, decision-acceptance profile identity, and freshness (`expiresAt`) remain valid.

A proposal is rejected or invalidated when:
- the decision-acceptance profile identity or criterion-set revision changes;
- the proposal's admission or health preconditions are no longer satisfied;
- the applicable acceptance predicate changes;
- the profile is no longer `ACTIVE` when the consumer requires active local admission;
- calibration requirements of the acceptance profile are no longer satisfied;
- the model revision changes;
- the local profile is disabled or quarantined;
- the underlying project revision changes;
- the context package identity changes;
- the decision purpose is not permitted by the profile;
- the result is malformed or outside its primitive domain;
- calibration is invalid;
- required evidence is no longer fresh; or
- the current time is at or after `expiresAt`.

`ACCEPTED_AS_INPUT` is permitted only when:
1. `admissionState = ACTIVE`;
2. `healthState = READY`;
3. the profile/model/runtime identity matches the current admitted identity;
4. the proposal passes primitive-domain validation;
5. required calibration conditions pass;
6. the applicable `LocalDecisionAcceptanceProfile` has `status = FROZEN`, its exact `profileId` matches `decisionAcceptanceProfileId`, and its target model/profile/runtime identities match the currently admitted identity;
7. `acceptanceOutcome = ACCEPTED`;
8. revision, context, state, and freshness checks all pass.

`EXPERIMENTAL` and `QUARANTINED` profiles MUST NOT produce `ACCEPTED_AS_INPUT`.

Local-engine failure classes are deterministic: `ENGINE_UNAVAILABLE`, `PROFILE_NOT_ADMITTED`, `RESOURCE_NOT_ADMITTED`, `MODEL_INTEGRITY_FAILURE`, `MODEL_RUNTIME_FAILURE`, `INVALID_RESULT`, and `PROPOSAL_STALE`.

A local-engine failure MUST NOT itself become a task failure when the engine is optional. The consumer MUST use its declared fallback path.

The engine may be loaded once and reused for multiple calls inside its admitted lifecycle. Shutdown, supervisor restart, profile invalidation, or resource pressure may unload it. No task correctness property may depend on process residency.

No local-engine output can bypass PolicyAuthority, ToolBroker, MutationBroker, ConstructionTransaction, evidence validation, promotion, or completion authorities.

## 59. Memory/Context Runtime

**ContractId:** `CONTRACT.RUNTIME.MEMORY, CONTRACT.RUNTIME.CONTEXT`  
**Authoritative build-spec section:** §38 / §53  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §53, which is canonical for the context-assembly, sufficiency-gate, context-integrity, attention-reliability, and re-grounding contract; this section defines only the implementing components and algorithms. Retrieval modes follow build spec §19 and §31; memory scopes, retention, and deletion follow the authority build spec §53 names (§38). This section adds the assembly, orchestration, and re-grounding components.

### 59.1 Components

The primary context architecture is coordinated by the `ContextOrchestrator` and its specialized subcomponents:

| Component | Responsibility |
|---|---|
| MemoryWriter | Writes classified memory records from validated events only |
| MemoryStore | Persists records with scope, provenance, and retention |
| ConstraintRegistry | Sole canonical authority and SQLite-backed registry for admitted `ConstructionRequirement` records, active constraints, and locked decisions; assigns and validates canonical requirement identity, preserves source/derivation/supersession lineage, and projects applicable requirement IDs into each `AndroidConstructionContract` revision |
| ContextOrchestrator | Sole primary context engine; coordinates WorkingSet planning, multi-modal retrieval, capacity planning against provider context capacity, and integrity verification; ensures skeleton-vs-interpreted-summary consistency through its `HierarchicalSynthesizer` subcomponent |
| WorkingSetPlanner | Partitions context into required, active, supporting, historical, and excluded sets |
| ContextFidelityManager | Enforces context fidelity levels across exact, structural, semantic, summary, and historical tiers |
| ExactRetriever | Resolves pinned symbols, target files, and explicitly referenced paths at EXACT fidelity; the first retriever of the §59.6 sequence |
| SemanticRetriever | Traverses the hierarchical Repository Semantic Graph over bidirectional dependency edges |
| TemporalRetriever | Retrieves recent causal action sequences and events from the Warm history tier |
| MemoryRetriever | Queries structured memory records, locked decisions, and failure fingerprints |
| EvidenceRetriever | Queries the active EvidenceFrontier to prioritize unvalidated or contradicted claims |
| DependencyExpander | Computes graph neighborhoods and affected compilation units from the ImpactGraph |
| ContextCapacityPlanner | Fits the selected context representation to the provider's actual context capacity and tracks remaining admissible context capacity per request, without imposing a Nirman usage budget; bounds the DENSE block by the measured reliable recall span and records `attendabilityMap` |
| AttentionProfiler | Runs the recall probe fixtures on the structural cadence of build spec §53.11 (provider profile save, checkpoint creation, phase boundaries, every compaction, any `PREMISE_MISMATCH`); writes the per-model `AttentionReliabilityProfile` as evidence, never from declaration or self-report |
| PlacementPlanner | Computes `placementPlan` per BS §53.11: cache-stable prefix, SPARSE breadth block, DENSE precision block, state digest, instruction; positions the cache breakpoint before the DENSE block |
| RecallProbeService | Embeds deterministic recall probes with runtime-held expected answers, verifies responses by exact match, and emits probe evidence on structural events only |
| ResourceIntegrityAuthority | Implements BS §72 (listed as `ResourceGovernor` in the §57.2 process topology and §51.3; same service): evaluates host, process, workspace, emulator, storage, concurrency, and liveness pressure and admits work against physical capacity; holds no AI-usage cap |
| CacheManager | Manages prefix-cache checkpoints, structured KV caches, and cache hit optimization |
| CompactionPlanner | Executes non-destructive semantic compaction of historical context |
| RetrievalCompletenessChecker | Executes pre-model COVERAGE_CHECK verifying dependency, interface, and evidence completeness |
| ContextIntegrityVerifier | Validates revision bindings (context, goal, project, plan, evidence) per build spec §53.4 and §53.7 as an authoritative hard gate |
| HierarchicalSynthesizer | Subcomponent of ContextOrchestrator; derives the deterministic structural skeleton from RepositorySemanticGraph regions and maintains provenance-bearing interpreted summaries per skeleton level; owns no authority, holds no independent state, and mutates no source |

> **Schema projection:** `ConstructionRequirement` is defined in `nirman-schemas.md` §1.81. Owner: BS §42.1.

> **Schema projection:** `LockedDecision` is defined in `nirman-schemas.md` §1.82. Owner: BS §42.1.

`ConstraintRegistry` persists canonical `ConstructionRequirement` records in `constraint_records` and canonical `LockedDecision` records in `decision_records`. `MemoryStore` may index a `MemoryRecord.class = DECISION` for retrieval, but that record is a derived semantic-memory projection and has no admission, locking, revision, supersession, or invalidation authority.

`ConstraintRegistry` persists canonical `ConstructionRequirement` records in `constraint_records`. Proposals from conversation interpretation, product intelligence, feedback, or models are not canonical until admitted through the deterministic requirement path. Each accepted change increments `requirementRevision` without changing `requirementId`; applicability is bound through `applicableContractRevisions`, while a semantic replacement receives a new ID linked by supersession. The canonical `requirementId` is reused by `ConversationRequirementIndex.canonicalRequirementId`, `AndroidConstructionContract.requirementIds`, `RequirementToImplementationGraph`, applicable scenarios, coverage, proof, change intelligence, clarification dependencies, and requirement deltas; those consumers do not mint competing requirement identities. `originKind`, feature/message/evidence provenance, and derivation lineage are independent of `state` and supersession.

The architecture retains three dedicated implementation collaborators:
- `ContextAssembler`: internal assembly operation invoked by `ContextOrchestrator` to serialize the final `ContextPackage` payload in the placement layout of BS §53.11.
- `RegroundingService`: invoked by `ContextOrchestrator` at checkpoints, on contradiction detection, or upon context integrity invalidation.
- `RedactionFilter`: mandatory finalization stage executed before model gateway dispatch to strip secrets and credentials.

### 59.2 Repository Semantic Graph

> **Schema projection:** `DeviceMatrixRiskProfile` is defined in `nirman-schemas.md` §2.108. Owner: TA §59.2.

The workspace maintains a typed, queryable `RepositorySemanticGraph` updated incrementally on every workspace mutation. It structures code into a strict physical-to-semantic containment hierarchy:

```text
Repository
 → Module
   → File
     → Symbol
       → Region
         → Exact source
```

Nodes and bidirectional edges are defined as:

```text
RepositorySemanticGraph
Nodes:
- file
- symbol
- type
- method
- interface
- module
- dependency
- test
- resource
- route
- schema
- configuration
- generated_artifact

Bidirectional Edges:
- calls / called_by
- implements / implemented_by
- references / referenced_by
- tests / tested_by
- configures / configured_by
- generates / generated_by
- depends_on / depended_on_by
- changed_by / changes
- validated_by / validates
```

The `SemanticRetriever` and `DependencyExpander` traverse this graph in both directions to compute complete dependency neighborhoods without lexical search blindspots:

```text
Task
 → target symbols
 → incoming dependencies
 → outgoing dependencies
 → affected tests
 → interfaces
 → configuration
 → runtime/evidence dependencies
```

### 59.3 Context Fidelity

Context items are ingested into the `ContextPackage` with an explicit fidelity level mapped in `fidelityMap`:

| Fidelity Level | Representation | Operational Scope |
|---|---|---|
| `EXACT` | Verbatim source text, byte-for-byte fidelity | Active mutation targets, active interface definitions, and edited regions |
| `STRUCTURAL` | Complete symbol signatures, type declarations, method headers | Direct dependency neighborhood and imported/implemented symbols |
| `SEMANTIC` | Condensed schema contracts, route tables, and behavioral invariants | Related distant modules and cross-cutting dependencies |
| `SUMMARY` | High-level architectural, module, or package summaries | Distant project components and non-target packages |
| `HISTORICAL` | Structured causal lineage, decision rationales, failure fingerprints | Prior session events, completed plan steps, and superseded checkpoints |

Normative fidelity rules:
1. **Edited regions**: MUST be provided at `EXACT` fidelity; summaries cannot replace exact lines for code mutation.
2. **Active interfaces**: Types and interfaces directly invoked by edited code MUST be provided at `EXACT` fidelity.
3. **Direct dependency neighborhood**: Direct callers, callees, and imports MUST be provided at `STRUCTURAL` fidelity minimum.
4. **Related distant code**: Transitive dependencies and distant consumers are provided at `SEMANTIC` fidelity.
5. **Historical material**: Historical transactions and old deliberation are provided at `SUMMARY` or `HISTORICAL` fidelity.

Hard fidelity invariants:
- A context item may be transformed to lower fidelity only when its current task role permits that transformation.
- `EXACT → STRUCTURAL` is permitted only when line-level semantics are not required.
- `EXACT → SUMMARY` is prohibited for active mutation targets, active interfaces, required acceptance evidence, and unresolved failure locations.

### 59.4 Context Confidence

`ContextOrchestrator` evaluates context sufficiency across seven dimensions before model invocation:
- `coverage`: proportion of target symbols and files included in the working set.
- `freshness`: proportion of context items verified against the latest `projectRevision` and `evidenceRevision`.
- `fidelity`: adherence to mandatory fidelity rules (e.g. 100% of mutation targets at `EXACT`).
- `dependencyCompleteness`: completeness of the direct bidirectional dependency neighborhood.
- `evidenceCompleteness`: proportion of claims on the `EvidenceFrontier` with valid observations.
- `uncertainty`: absence of unclassified or conflicting assumptions in `UncertaintyRegistry`.
- `attentionReliability`: every `requiredItems` entry and every mutation-target `EXACT` item is `EXPECTED_RELIABLE` in the `attendabilityMap`, or is covered by a passing recall probe in the same package (BS §53.11).

The aggregate evaluation determines task eligibility:
- `HIGH`: Context is fully sufficient; eligible for immediate model invocation and autonomous mutation.
- `MEDIUM`: Context coverage is partial, or coverage is sufficient but attendability is not; model invocation prohibited until `SemanticRetriever` expands retrieval or `PlacementPlanner` re-projects the affected items, the step is narrowed to the reliable recall span, or a provider model whose profile satisfies the step is selected.
- `LOW`: Context is stale, contradictory, or severely incomplete; model invocation prohibited; invokes `RegroundingService`.

### 59.5 Memory record schema

> **Schema projection:** `MemoryRecord` is defined in `nirman-schemas.md` §2.50. Owner: TA §59.5.

`sourceEventIds` must be non-empty: MemoryWriter rejects a record with no source event, structurally enforcing the memory-write sourcing rule owned by build spec §53.2.

### 59.6 ContextOrchestrator algorithm and recovery

The orchestrator executes the following deterministic sequence:
1. **Integrity Preflight**: `ContextIntegrityVerifier` verifies that `contextRevision`, `goalRevision`, `projectRevision`, `planRevision`, and `evidenceRevision` match current authoritative ledger state, per build spec §53.4.
2. **Constraint & Decision Reservation**: Load active constraints and locked decisions from `ConstraintRegistry`; eviction rights are owned by build spec §53.5.
3. **Working-Set Planning**: `WorkingSetPlanner` queries the `EvidenceFrontier` to identify unvalidated/contradicted claims, sets semantic and temporal anchors, and identifies the active working set.
4. **Multi-Modal Retrieval**:
   - `ExactRetriever`: Pinned symbols and target files.
   - `SemanticRetriever`: Bidirectional traversal over `RepositorySemanticGraph`.
   - `TemporalRetriever`: Recent causal execution chains from Warm memory.
   - `MemoryRetriever`: Failure fingerprints and historical invariants.
5. **Fidelity Mapping**: `ContextFidelityManager` assigns fidelity levels (`EXACT`, `STRUCTURAL`, `SEMANTIC`, `SUMMARY`, `HISTORICAL`) ensuring edited regions and interfaces remain `EXACT`.
5b. **Placement & Attendability Mapping**: `PlacementPlanner` assigns every item to a block of the BS §53.11 layout, computes `placementPlan`, and marks each item `EXPECTED_RELIABLE`, `EXPECTED_DEGRADED`, or `UNKNOWN` in `attendabilityMap` from the provider model's `AttentionReliabilityProfile`; `RecallProbeService` embeds probes when a structural event schedules them.
6. **Sufficiency & Completeness Gate**:
   ```text
   CONTEXT_ASSEMBLE → COVERAGE_CHECK → INTEGRITY_CHECK → MODEL
   ```
   `RetrievalCompletenessChecker` evaluates context confidence. If confidence is `MEDIUM` or `LOW`, the invocation prohibition and remediation owned by build spec §53.4 apply.
7. **Context Fusion**:
   Combine:
   - exact source
   - structural graph context
   - semantic context
   - temporal context
   - causal memory
   - evidence frontier
   - active decisions and constraints
8. **Capacity Adaptation**: `ContextCapacityPlanner` first fits the DENSE block inside the measured reliable recall span by re-projecting required items and, when necessary, narrowing the step so fewer mutation targets are active at once; only then, if the selected representation exceeds the provider's actual context capacity, it progressively transforms non-required items:
   ```text
   EXACT → STRUCTURAL → SEMANTIC → SUMMARY
   ```
   only for items whose fidelity rules permit transformation. Eviction, fidelity, and `omittedForCapacity` recording follow build spec §53.3 and §53.5.
9. **Privacy Filtering**: `RedactionFilter` removes secrets, credentials, and private content.
10. **Payload Assembly & Ledger Emission**: `ContextAssembler` serializes the manifest defined in BS §53.3 and emits the cryptographically hashed package to the event ledger.

Recovery behavior:
When context integrity fails (`STALE_CONTEXT`, `CONTRADICTED_FACT`, `REVISION_MISMATCH`), the orchestrator aborts model dispatch, generates an integrity diagnostic, and triggers `RegroundingService` to re-synchronize working state from the durable ledger before re-attempting context assembly. When attendability fails (`RECALL_PROBE_FAILED`, `PREMISE_MISMATCH`), the orchestrator records the failure in the `AttentionReliabilityProfile` and applies the response order and authority boundary owned by build spec §53.4 and §53.11 (re-project, narrow step, select provider model by reliability, re-ground or escalate; never a pass count, never a pause of valid work).

### 59.7 Hybrid Cognitive Context

Nirman uses two complementary context paths:

DENSE PATH:
- exact edited regions
- active interfaces
- locked decisions
- active constraints
- current diagnostics
- current evidence
- active task state

SPARSE PATH:
- repository semantic graph
- dependency neighborhoods
- historical execution
- causal memory
- distant consumers
- prior failures
- architectural relationships

`ContextOrchestrator` fuses both paths into one revision-bound `ContextPackage`.

The dense path provides precision. The sparse path provides breadth. Neither path is authoritative independently; authoritative state remains in the durable project, execution, memory, and evidence stores.

The two paths are also physical regions of the transmitted request (BS §53.11): the DENSE path occupies the block adjacent to the instruction and is bounded by the provider model's measured reliable literal-recall span, where full attention is most dependable; the SPARSE path occupies the breadth block, where gist recall from compressed attention state suffices. The boundary between them is measured per model by `AttentionProfiler`, not assumed.

### 59.8 Cache Architecture

`CacheManager` optimizes prefix caching and structured KV reuse across provider requests. Cache is strictly an optimization, never memory authority:
- Cache hit ≠ observation
- Cache hit ≠ evidence
- Cache hit ≠ authoritative state
- Cache invalidation ≠ task failure

If a cache is invalid, cold, or unavailable, Nirman deterministically reconstructs the context from durable state and continues without degradation.

CacheManager implements the cache-breakpoint and prefix/DENSE authority rules owned by build spec §53.11: the breakpoint precedes the DENSE block, a cache hit never moves DENSE content into the prefix, and the DENSE copy is authoritative for the request.

### 59.9 Re-grounding trigger conditions

RegroundingService must run at checkpoint creation, before plan recompilation, after a runtime directive is accepted, after user-edit reconciliation, on resume from pause or restart, after a candidate branch selection, after every compaction, and after a failed recall probe or a `PREMISE_MISMATCH` that the re-project and narrow steps did not resolve.

### 59.10 Persistence and isolation

Memory records are stored in the SQLite execution ledger keyed by project. Cross-project reads are prevented at query level by mandatory project scoping. Runtime-improvement records are stored in a separate table with no path, identifier, or content columns.

### 59.11 Architecture tests

Assembly is correct only when a locked decision remains present in every subsequent ContextPackage until superseded; when a memory write with no source event is rejected; when a project-scoped query cannot return another project's records; when an invalidated or stale ContextPackage is rejected before action authorization; when a historical ContextPackage is reproducible from the ledger; when a constraint placed at the head of a 90 percent-filled window on a provider model with a large declared capacity but a smaller measured reliable span is either recalled by probe or the package is re-projected before mutation; when a proposal whose anchor hashes or premises disagree with the originating package is rejected as `PREMISE_MISMATCH` before any transaction opens and the provider model's profile records the failure as `LEARNED`; when every compaction is followed by a passing re-projection probe; and when provider model selection under a lighter profile changes on measured reliability, not on declared capacity.

### 59.12 Recall probes, placement bounds, and attention learning

`RecallProbeService` implements the probe classes of BS §53.11: constraint restatement by identifier and wording, anchored-symbol and signature echo, anchor-hash and line-anchor echo, multi-needle recall, post-compaction re-projection of the constraint ledger, and tool-result recall. Expected answers are generated and held by the runtime and verified by exact match; a probe result is an evidence record and is never inferred from the model's own description of its memory.

Probe cadence is bound to structural events only: provider profile save (the §24.3 connection test runs the §24.8 fixtures 13 and 14), checkpoint creation, phase boundaries, every compaction, and any `PREMISE_MISMATCH`. No component schedules a probe on a token, request, or pass count, and no probe result pauses, throttles, degrades, or fails valid work; probe results change placement, step size, and provider or model selection only.

`AttentionProfiler` aggregates probe results into `positionalRecall` cells keyed by fill bucket and position bucket, derives `reliableLiteralSpanTokens` as the largest fill bucket whose literal pass rate meets the configured threshold (BS §80.3), and sets `confidence` from sample counts. Every `PREMISE_MISMATCH` returned by the mutation broker (BS §43.2) updates the cell corresponding to the mismatched item's recorded position with `source: LEARNED`, so the profile improves without additional model calls. An `UNPROFILED` model receives the unprofiled DENSE block bound of BS §80.3 and consequential steps against it carry an in-package probe that must pass.

Each plan step carries `requiredReliability`, derived from the size of its DENSE block and whether it is consequential. `DeliberationModelRouter` (§72) and `ResourceGovernor` (§51.3) may select a lighter provider model only when its `AttentionReliabilityProfile` satisfies the step's `requiredReliability`; this is a capability match, like tool-calling or vision support, not a budget, and it operates inside the unchanged permission ceiling.

### 59.13 Hierarchical project synthesis

Implements build spec §53, which remains canonical for the context contract, over the §59.2 `RepositorySemanticGraph`. Owned by `ContextOrchestrator` through its `HierarchicalSynthesizer` subcomponent (§59.1). This section creates no authority and mutates no source.

S1 Deterministic skeleton. The runtime MUST derive a deterministic structural skeleton by projecting graph regions onto the §59.2 containment hierarchy (Repository → Module → File → Symbol → Region → Exact source) with levels L0 system (Repository), L1 subsystem (Module), L2 unit (File), L3 detail (Symbol/Region). Skeleton derivation MUST use graph nodes and edges only; no model output participates in skeleton construction.

S2 Interpreted summaries. Each skeleton level MUST carry interpreted summaries expressed as typed claims bearing provenance: graph node ids, projectRevision, producer worker id, validation status, and supersedes link. Summaries MUST be referenced from the `ContextPackage` manifest defined in BS §53.3. A summary becomes a memory FACT only through a validated event per BS §53.2; a model statement is never a memory write.

S3 Refresh and invalidation. On every workspace mutation the runtime MUST invalidate synthesis entries for affected regions on the same trigger as the §59.2 incremental graph update, and MUST rebuild skeleton entries before they are served again. Interpreted summaries bound to a superseded projectRevision MUST NOT be served; the orchestrator MUST serve skeleton-only content or re-derive. Revision mismatch on the synthesis artifact MUST raise STALE_CONTEXT through the §59.6 recovery path (`RegroundingService`). User-originated edits MUST invalidate affected entries per the §59.9 re-grounding triggers.

S4 Exact-source authority for mutation. Mutation-affecting context MUST resolve to EXACT-fidelity source through `ExactRetriever`; required EXACT items MUST remain EXACT, and EXACT items MUST NOT be replaced by summaries when required for mutation or line-level reasoning (BS §53.3). Every synthesis entry MUST carry its level and fidelity label. A summary MUST NOT be cited as the basis for a mutation; a plan citing a summary in place of exact source MUST be rejected by `RetrievalCompletenessChecker`.

S5 Precedence. On conflict between a deterministic skeleton fact and an interpreted claim, the skeleton fact MUST prevail. `ContextOrchestrator`, through its `HierarchicalSynthesizer` subcomponent, compares each interpreted claim against the corresponding deterministic skeleton fact before that summary is admitted to the synthesis artifact or cache or served in a `ContextPackage`. A mismatching interpreted claim is not admitted or served as authoritative synthesis content. The conflict MUST be recorded as `CONTRADICTED_FACT` and the §59.6 recovery path aborts model dispatch, generates the integrity diagnostic, and invokes `RegroundingService`. An interpreted claim MUST NOT override a deterministic structural fact.

S6 Compatibility. Synthesis output MUST enter `WorkingSetPlanner` partitions as anchored supporting content. `ContextCapacityPlanner` MUST account synthesis bytes against provider capacity like any other context. Synthesis staleness MUST feed the §59.9 re-grounding triggers. The §59.6 ten-step sequence is unchanged.

S7 Bounded cost. Skeleton derivation MUST be incremental per region and deterministic. Interpreted summaries MUST be generated lazily per level on demand and cached with revision binding. All synthesis model calls MUST be governed by `ReasoningEffortSelector` effort grants and `ResourceIntegrityAuthority` budgets; synthesis MUST NOT create a separate budget class.

S8 Rebuildability. The synthesis artifact MUST be rebuildable from the graph plus projectRevision. Any persisted synthesis form MUST be revision-bound and MUST carry the S4 labels, so that no cache is ever mistaken for source.

## 60. Peer Coordination and Semantic Reservations

**ContractId:** `CONTRACT.RUNTIME.RESERVATION`  
**Authoritative build-spec section:** §54  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §54, which is canonical for reservation conflict rules, shared-surface single-writer semantics, stale-contract invalidation, and commit-barrier duties; this section defines only the implementing components. Extends §8 (Workspace Isolation and Reconciliation) and §46 (Lease and Capability Runtime), which remain the authority on workspace leases. This section adds the semantic layer above file ownership.

### 60.1 Components

| Component | Responsibility |
|---|---|
| ReservationRegistry | Grants, renews, revokes, and queries semantic reservations |
| SurfaceIndex | Maps symbols, routes, schema tables, resources, and permissions to files |
| ConflictDetector | Evaluates requested reservations against held reservations |
| StaleContractInvalidator | Invalidates read_stable holders when a surface changes |
| CommitBarrier | Serializes proposal merges and revalidates freshness |
| SharedSurfaceApplier | Applies `SharedSurfaceChangeRequest`s to the single-writer Android surfaces semantically and in dependency order on behalf of the reconciliation worker (build spec §54.2; ADR-225) |

### 60.2 Reservation state machine

```text
requested -> granted -> renewed* -> released
requested -> denied
granted   -> expired      (lease not renewed)
granted   -> revoked      (authority decision)
granted   -> invalidated  (surface changed under read_stable)
```

Transition authority follows build spec §54.4 — only the deterministic runtime performs state transitions; a worker may request, renew, and release, but never grant. The machine below is the implementation projection.

### 60.3 Conflict matrix

| Held \ Requested | read_stable | modify | delete | create |
|---|---|---|---|---|
| read_stable | allow | deny | deny | n/a |
| modify | deny | deny | deny | n/a |
| delete | deny | deny | deny | n/a |
| create | n/a | n/a | n/a | deny |

A denial returns the holding worker and task so the requester can request a handoff rather than retry blindly. The matrix is ConflictDetector's implementation projection of the conflict rule owned by build spec §54.2; cases build spec §54.2 does not specify (`create`, `read_stable`/`read_stable`) are implementation detail with no contract standing.

### 60.4 Invalidation propagation

When a mutation commits on a surface, StaleContractInvalidator implements the invalidation duties of build spec §54.3: it must find every `read_stable` reservation on that surface, mark each holder's dependent work `unvalidated`, clear affected validation evidence, and notify the holder's task. Work marked unvalidated cannot reach CommitBarrier until revalidated.

### 60.5 CommitBarrier checks

At the barrier, in order: verify all reservations held by the proposal are still `granted`; verify no dependent surface changed after the proposal's validation timestamp; verify validation evidence postdates the last relevant surface change (build spec §54.5); then apply the mutation transactionally through the reducer of §45. Any failed check rejects the proposal with a typed reason.

### 60.6 Architecture tests

Coordination is correct only when two workers requesting `modify` on one symbol produce one grant and one typed denial; when a symbol rename invalidates a dependent worker's `read_stable` work; and when a proposal validated before a dependent change is rejected at the barrier rather than merged.

## 61. User/Edit Reconciliation Coordinator

**ContractId:** `CONTRACT.RUNTIME.RECONCILIATION`  
**Authoritative build-spec section:** §55  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §55, which is canonical for origin classification, reconciliation behavior, and prohibited behaviors; this section defines only the detecting and reconciling components. No existing section covers concurrent human editing; this is a new runtime component that consumes the reservation layer of §60 and the mutation records of §45.

### 61.1 Components

| Component | Responsibility |
|---|---|
| ProjectWatcher | Observes filesystem changes in the project tree |
| OriginClassifier | Determines whether a change is RUNTIME, USER, EXTERNAL, or GENERATED |
| MutationLedgerIndex | Provides expected content fingerprints for runtime-authored writes |
| ReconciliationCoordinator | Pauses affected mutation and drives re-derivation |
| BaselineUpdater | Adopts user content as the new baseline |

### 61.2 Origin classification algorithm

OriginClassifier implements the build spec §55.2 origin table. For each observed change the classifier computes the file fingerprint and compares it to the fingerprint recorded by the last runtime mutation for that path. A match classifies RUNTIME. A mismatch on a path under an active runtime reservation classifies USER or EXTERNAL. Paths matching generated-output patterns and build directories classify GENERATED and are excluded from reconciliation and from context assembly.

Origin classification follows build spec §55.2: mutation records and file fingerprints, never timestamps alone (build steps and editors rewrite timestamps).

### 61.3 Reconciliation sequence

```text
observe change
  -> classify origin
  -> if USER or EXTERNAL on reserved surface:
       pause mutation on that surface
       invalidate validation evidence for the surface
       re-read file and update SurfaceIndex
       re-derive plan validity
       if contradicts a locked decision -> emit DecisionNode
       else -> BaselineUpdater adopts content, resume
```

### 61.4 Prohibited operations

The components enforce the prohibitions owned by build spec §55.4: BaselineUpdater never writes the runtime's prior version over user content, the evidence store does not accept validation for a surface whose fingerprint changed after the validation ran, and the completion authority of §23 rejects a completion claim citing pre-edit evidence.

### 61.5 Attribution in evidence

Every mutation record carries an `origin` field. Final reports implement the attribution requirement of build spec §55.5, rendering user-originated changes distinctly from runtime-originated changes so the user is never told the runtime produced their own edit.

### 61.6 Architecture tests

Reconciliation is correct only when a user edit during an active run survives to the final artifact; when validation predating the edit is discarded; when a user edit contradicting a locked decision produces a decision node; and when generated build output never triggers reconciliation.

## 62. Stateful E2E Engine

**ContractId:** `CONTRACT.RUNTIME.E2E`  
**Authoritative build-spec section:** §56  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §56, which is canonical for the required scenario classes, determinism, seed provenance, evidence requirements, and testing-strength semantics; this section defines its implementation. Extends §35 (Complete Android Capability Fixture Contract) and §50 (Preview Coordinator and Android Runtime Validation), which remain the authority on emulator sessions and fixtures.

### 62.1 Components

> **Schema projection:** `ScenarioValidationMatrix` is defined in `nirman-schemas.md` §2.103.1. Owner: TA §62.1.

| Component | Responsibility |
|---|---|
| ScenarioRegistry | Stores scenario definitions and requirement links |
| ScenarioCompiler | Lowers a scenario to `AndroidDeviceAdapter` operation sequences; ADB and instrumentation are adapter-internal execution, never emitted steps (§62.10) |
| SeedDataProvisioner | Establishes preconditions through the app's own data layer |
| ScenarioExecutor | Runs steps against an emulator session and records results |
| StateProbe | Verifies persisted state after process death or restart |
| ScenarioEvidenceWriter | Writes step results, screenshots, and Logcat windows |
| ScreenGraphExplorer | Explores the installed application from the launch activity into a `ScreenGraph` (ADR-225) |
| ScenarioSynthesizer | Derives `E2EScenario` steps for acceptance criteria and the nine scenario classes of build spec §56.3 from the `ScreenGraph` |
| StateSpaceCoverageEvaluator | Evaluates state-transition coverage across the five testing-strength dimensions of build spec §56.7 |
| MetamorphicVerifier | Executes the invariant/metamorphic checks required by build spec §56.7 |
| FaultInjectionCoordinator | Orchestrates fault-injection scenarios (permission denial, process death, configuration change, network loss, UI/runtime faults, persistence faults) |
| DeterminismClassifier | Classifies scenario runs as DETERMINISTIC, FLAKY, or NONDETERMINISTIC based on repeated execution results |
| DifferentialRegressionEvaluator | Reruns old passing scenarios after repair and detects regression patterns |
| NegativeProofEvaluator | Enforces the negative-proof dimension of build spec §56.7 before completion |
| StartupRegressionTracker | Measures cold start launch latency (TTID/TTFD) and detects performance regressions (§62.1.2) |
| MemoryLeakDetector | Evaluates heap growth and Activity retention across lifecycle churn and navigation (§62.1.3) |
| TestDataLeakageDetector | Verifies persistent storage isolation, ensuring synthetic seed data does not survive teardown (§62.5.1) |

> **Schema projection:** `ScreenGraph` is defined in `nirman-schemas.md` §2.92. Owner: TA §62.1.

> **Schema projection:** `StateSpaceCoverageModel` is defined in `nirman-schemas.md` §2.103. Owner: TA §62.1.

All nine components (StateSpaceCoverageEvaluator, MetamorphicVerifier, FaultInjectionCoordinator, DeterminismClassifier, DifferentialRegressionEvaluator, NegativeProofEvaluator, StartupRegressionTracker, MemoryLeakDetector, TestDataLeakageDetector) are subordinate components of CONTRACT.RUNTIME.E2E. They create no authority and no completion decision.

Scenario execution pipeline:
GoldenSnapshot
→ deterministic install/seed
→ baseline scenario execution
→ state/transition coverage evaluation
→ applicable fault injection
→ recovery observation
→ same failing scenario replay
→ differential regression replay
→ invariant/metamorphic verification
→ negative-proof evaluation
→ EvidenceLedger

`ScreenGraphExplorer` runs before scenario synthesis on a `GoldenSnapshot`-restored device: it performs a bounded breadth-first exploration from the launch activity, taking each actionable element of the current `ScreenModel` once, deduplicating screens by `screenFingerprint`, recording every transition as an edge with its observed result, and stopping at `maxDepth`, `maxActionsPerScreen`, or an exhausted frontier. Exploration is observation, not validation: a crash or ANR met during exploration enters the failure-fingerprint path of §51.1, and an `EXTERNAL_INTENT` edge is recorded and not followed. `ScenarioSynthesizer` implements the derivation duty of build spec §56.2 — one `E2EScenario` per acceptance criterion and per required class of §56.3 — by mapping each to a path in the graph and emitting an `E2EScenario` whose `steps` name `ScreenModel` element identities and whose `assertions` name observable postconditions; `coveredRequirementIds` and `uncoveredRequirementIds` are written to the graph, and an uncovered requirement is reported to the planner as a `REPLAN` input rather than silently dropped. Synthesized scenarios pass through `ScenarioRegistry` and the determinism rule of §62.4 exactly like authored ones.

### 62.1.1 DeadControlDetector

`DeadControlDetector` executes within `ScreenGraphExplorer` during automated application exploration to detect unresponsive, inert interactive controls:
1. *Interactive element enumeration:* Traverses the active `ScreenModel` and Compose semantics node tree to enumerate all controls with click, swipe, or input actions (e.g. `Button`, `IconButton`, `Clickable`, `FloatingActionButton`, `Switch`, `Tab`).
2. *State and feedback delta probe:* Injects synthetic interactions via `ScenarioExecutor` and probes for observable post-conditions: navigation transition, ViewModel state mutation, Room database write, snackbar/dialog presentation, or network dispatch.
3. *Defect classification:* Flags any interactive element whose stimulus produces zero observable state change or user feedback across consecutive frames as a `DEAD_CONTROL` defect, preventing hollow UI implementations from satisfying completion evidence.

### 62.1.2 StartupRegressionTracker

`StartupRegressionTracker` measures cold-start launch latency and tracks startup performance regressions across autonomous build cycles:
1. *Launch milestone harvesting:* Ingests Android activity manager Logcat records (`Displayed` / `Fully drawn`) to measure Time to Initial Display (TTID) and Time to Full Display (TTFD) during `Cold start` scenario execution (BS §56.3).
2. *Historical baseline comparison:* Evaluates observed startup metrics against the project profile's historical baseline stored in the execution ledger.
3. *Regression gating:* Flags any regression exceeding the detector threshold (25% default, or the profile's latency ceiling) as a `STARTUP_LATENCY_REGRESSION` finding; the finding enters the evidence chain and is weighed per build spec §56.7 — it is not an independent completion gate.

### 62.1.3 MemoryLeakDetector

`MemoryLeakDetector` performs runtime heap allocation analysis and Activity lifecycle leak verification:
1. *Lifecycle churn orchestration:* Drives repeated configuration changes (portrait/landscape rotation), process backgrounding/foregrounding, and deep navigation traversal via `FaultInjectionCoordinator`.
2. *Heap allocation inspection:* Triggers deterministic garbage collection via ADB and parses `dumpsys meminfo` heap distributions to inspect native and Dalvik heap growth across cycles.
3. *Retained instance detection:* Identifies retained destroyed Activity instances, View hierarchies captured in static references, or unbonded coroutine scopes, emitting `MEMORY_LEAK_DETECTED` failure evidence, which enters the evidence chain and is weighed under the negative-proof dimension of build spec §56.7.

### 62.1.4 ComposeIdlingBarrier

`ComposeIdlingBarrier` coordinates deterministic runtime synchronization between Nirman's scenario execution engine and the Android Jetpack Compose runtime:
1. *Compose idling synchronization:* Bridges `AndroidDeviceAdapter` with the on-device test orchestrator via Compose `IdlingResource` and `TestMonotonicFrameClock`, guaranteeing that synthetic gestures and screenshot captures occur only when recompositions, animations, and snapshot state propagation have completely quiesced.
2. *Asynchronous coroutine quiescence:* Coordinates with Kotlin Coroutine dispatchers under test (`StandardTestDispatcher`), asserting that pending background jobs bound to the active screen lifecycle have finished processing prior to node semantics tree extraction.
3. *Flake-free UI observation:* Eliminates race conditions where screenshots or accessibility node hierarchies are captured mid-frame, providing deterministic ground-truth visual and semantic evidence to `ScreenGraphExplorer` and `PerceptualHashComparator`.

### 62.2 ScreenGraph Analysis Service

> **Schema projection:** `ScreenGraphAnalysisRecord` is defined in `nirman-schemas.md` §2.99. Owner: TA §62.2.

The runtime provides a deterministic service that computes reachability and analysis from the ScreenGraph.

**Responsibilities:**

- Compute reachability matrix from ScreenGraph (identifies unreachable screens/states)
- Identify dead ends (screens with no actionable elements except back/exit)
- Suggest targeted test cases for critical unexplored edges
- Compute coverage percentage against AndroidConstructionContract requirements
- Flag unreachable states as defects (not just untested)

**Schema:** `ScreenGraphAnalysisRecord` (SCHEMAS §2.99)

**Contract:** `CONTRACT.RUNTIME.E2E`

**Precedence:** Runtime intelligence, not authority - findings guide testing but cannot auto-complete requirements or auto-block tasks.

> **Schema projection:** `AndroidSemanticState` is defined in `nirman-schemas.md` §2.102. Owner: TA §62.2.

> **Schema projection:** `ScenarioStep` is defined in `nirman-schemas.md` §2.51. Owner: TA §62.2.

### 62.3 Step and assertion schema

> **Schema projection:** `ScenarioStep` is defined in `nirman-schemas.md` §2.51. Owner: TA §62.2.

The system-event set realizes the required scenario classes of build spec §56.3 — the states single-screen validation misses — through process death, configuration change, permission grant and deny, network loss, and app backgrounding. Each is backed by a dedicated `AndroidDeviceAdapter` operation (§73.12): `forceStop` for process death, `setOrientation` for configuration change, the permission path of the hygiene policy for grant and deny, `setNetworkState` for network loss, and `sendToBackground` for backgrounding; `wait_for` steps resolve through `waitFor`, never through a fixed sleep (§62.4). Alarm fire and notification delivery join the system-event set: `advanceClock` fires due alarms deterministically from the seeded basis, `collectNotifications` observes posted notifications, and `wait_for` accepts `notification present`; `StateProbe` executes `probe_state` steps against persisted state and posted notifications.

### 62.4 Determinism enforcement

> **Schema projection:** `RequirementToImplementationGraph` is defined in `nirman-schemas.md` §2.104. Owner: TA §62.4.

Determinism marking and the exclusion of non-deterministic scenarios from completion evidence are governed by build spec §56.2; `ScenarioExecutor` enforces them through explicit `wait_for` conditions and never fixed sleeps as synchronization. State changes produced by a seeded clock advance, including clock-rendered text, are expected transitions when the same seed and advance reproduce the same post-state — an implementation projection of the seed-clock basis of build spec §56.4 — and MUST NOT be classified FLAKY.

### 62.5 Seed provenance

`SeedDataProvisioner` implements the seed-provenance requirement of build spec §56.4, recording how each precondition was established; seeded state is labeled in evidence so it cannot be mistaken for behavior the application produced.

### 62.5.1 TestDataLeakageDetector

`TestDataLeakageDetector` executes after scenario completion and teardown to verify state isolation:
1. *Persistent storage inspection:* Inspects the target application's private filesystem directory (`/data/data/<package>/`) on the emulator after test completion, scanning SQLite/Room database tables, SharedPreferences XML files, and DataStore protobuf files.
2. *Seed marker reconciliation:* Cross-references stored data records with `SeedDataProvisioner` seed identities and temporary test fixtures.
3. *Leakage prevention:* Rejects any scenario run where synthetic seed data, mock user tokens, or test fixtures survive teardown into persistent storage with `TEST_DATA_LEAKAGE_DETECTED`, ensuring production state remains clean.

### 62.6 Persistence

Scenario definitions, runs, step results, and evidence references are stored in the execution ledger and linked to requirement identifiers, enabling the traceability chain of build spec §66.3. `ScenarioEvidenceWriter` performs these writes, binding time-bearing evidence to the run's clock basis.

### 62.7 Causal Surface Identification

The runtime provides a deterministic service that traces requirements to implementation symbols and dependencies.

**Responsibilities:**

- Build RequirementToImplementationGraph on task initialization (requirement → behavior → UI transition → symbols → dependencies → scenario → evidence)
- On failure, identify the smallest causal surface (component, symbol, dependency) responsible
- Replace broad "fix the crash" with precise "repair the lifecycle dependency"

**Schema:** `RequirementToImplementationGraph` (§nirman-schemas.md §2.104)

**Contract:** `CONTRACT.RUNTIME.E2E`

**Precedence:** Runtime intelligence, not authority - causal surfaces guide repair but cannot auto-complete requirements.

### 62.8 Architecture Fitness Evaluation

> **Schema projection:** `ArchitectureFitnessReport` is defined in `nirman-schemas.md` §2.106. Owner: TA §62.8.

The runtime provides a deterministic service that evaluates technology plan quality after generation.

**Responsibilities:**

- Post-build evaluation: excessive JS/native crossings, lifecycle hazards, unnecessary complexity, testability
- Propose technology-plan revision when the architecture itself causes recurring failures
- Provide feedback loop to requirement synthesis

**Schema:** `ArchitectureFitnessReport` (§nirman-schemas.md §2.106)

**Contract:** `CONTRACT.RUNTIME.AGENT_BUILDABILITY`

**Precedence:** Runtime intelligence, not authority - findings guide planning but cannot auto-change technology plans.

### 62.9 Architecture tests

The engine is correct only when a data-persistence scenario detects an app that loses data on process death; when a flaky scenario is quarantined rather than reported as passing; when every requirement's scenario link resolves in the ledger; and when a scheduled-behavior scenario proves its clock basis with an observed delivery.

### 62.10 Adapter-side resolution

Implements the adapter-binding rule of build spec §56.8 (CLAUSE.PREVIEW_SYNC.ADAPTER_BOUND): test execution routes through `AndroidDeviceAdapter`. The technology adapter resolves the binding but MUST NOT execute the test. The build spec §56.7 in-process bindings (Espresso, Compose UI Test) execute through `runInstrumentation`: the adapter runs the resolved binding on-device and returns typed per-test results as `Observation`s, so assertion evidence for Views and Compose compositions enters the chain without an out-of-band runner. ADB is adapter-internal transport; no component outside the adapter emits ADB steps.

## 63. Regression Localization Service

**ContractId:** `CONTRACT.RUNTIME.LOCALIZATION`  
**Authoritative build-spec section:** §62  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §62, which is canonical for localization order, the repair constraint, the reproduce-first gate, and failure-signature learning; this section defines its implementation. Extends §30 (Self-Improvement Manager) failure analysis and §58 mutation/regression intelligence, which remain the authority on predicting affected validation.

### 63.1 Components

| Component | Responsibility |
|---|---|
| RegressionDetector | Identifies assertions or scenarios that passed before and fail now |
| CandidateChangeCollector | Collects mutation records between last-pass and current revision |
| ImpactGraphLocalizer | Finds mutations reaching the failing surface |
| SignatureMatcher | Matches the symptom against stored failure signatures |
| CheckpointBisector | Narrows the causing revision using existing checkpoints |
| CauseRecorder | Records the identified cause, confidence, and repair link |

### 63.2 Localization pipeline

```text
detect regression
  -> collect candidate mutations (last_pass_revision .. failing_revision)
  -> ImpactGraphLocalizer: filter to mutations reaching failing surface
       single candidate -> cause identified (high confidence)
  -> SignatureMatcher: match symptom to known signature
       match -> cause class identified (medium confidence)
  -> CheckpointBisector: binary search over existing checkpoints
       -> cause identified (high confidence)
  -> otherwise: record unlocalized_regression and escalate
```

Bisection consumes existing checkpoints per build spec §62.3 — reuse when checkpoints are available rather than rebuilding — through the two-tier checkpoint architecture of §18; full rebuild bisection is prohibitively expensive for Android projects.

`FailureContextPackage` is the bounded product of the localization pipeline: the Diagnostic Worker's root-cause hand-off containing the relevant error evidence, changed-file scope, environment identity, prior strategies, checkpoint, validation results, privacy classification, and next-action constraints.

### 63.3 Repair scoping

Repair scoping follows build spec §62.4: the identified cause surface is the permitted repair scope, and the mutation broker rejects a repair mutation outside that scope unless the planner records an explicit widening reason. `CauseRecorder` attaches the failure fingerprint shared with §51.1, so an identified cause enters `RepairPattern` lookup before model reasoning (ADR-225). The reproduce-first gate of build spec §62.4 governs repair-transaction admission; `ReproScenario` execution uses the §62.1 engine. An unlocalized regression is handled exactly as build spec §62.4 provides: recorded and escalated to the planner rather than rewriting unrelated code, which is what destroys validated work.

### 63.4 Failure signature schema

> **Schema projection:** `FailureSignature` is defined in `nirman-schemas.md` §2.52. Owner: TA §63.4.

Failure signatures implement build spec §62.5 — each links symptom, cause class, and successful repair — and are written as FAILURE memory records under the memory-record rules of build spec §53.2; records are project-scoped unless anonymized for runtime-improvement memory.

### 63.5 Architecture tests

Localization is correct only when an injected single-line regression is attributed to its mutation; when a repair mutation outside the cause scope is rejected; when bisection consumes existing checkpoints without full rebuilds; and when an unlocalized regression escalates rather than triggering rewrite.

## 64. Verification Orchestrator

**ContractId:** `CONTRACT.RUNTIME.VERIFICATION`  
**Authoritative build-spec section:** §57  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §57, which is canonical for in-loop verification, assertion ordering, non-vacuity, and the verification method matrix; this section defines its sequencing implementation. Extends §53 (Integrated Workflow and Quality Services) and §58 ValidationPlanner, which remain the authority on selecting which validation to run. This section adds in-loop verification sequencing.

### 64.1 Components

| Component | Responsibility |
|---|---|
| DiagnosticRunner | Runs compiler diagnostics and lint on affected surfaces |
| IncrementalCompiler | Compiles at module granularity after each mutation |
| AssertionAuthor | Records assertions before implementation for behavioral requirements |
| MutationProber | Injects faults to test whether assertions can fail |
| PropertyProber | Exercises input domains for counterexamples |
| VerificationLedger | Records every verification run, method, and outcome |

### 64.2 In-loop gate sequence

```text
structured mutation applied
  -> premise check: StructuredPatch anchors and premises match the originating ContextPackage
       PREMISE_MISMATCH -> reject before any transaction; profile learns; re-project or narrow
  -> DiagnosticRunner on affected surface
       new diagnostic -> repair or revert (mutation does not advance)
  -> IncrementalCompiler on affected module
       failure -> repair or revert
  -> affected assertions executed
       failure -> RegressionLocalizationService (§63)
  -> mutation marked verified, dependent work unblocked
```

As the implementation projection of the build spec §57.1 in-loop rule, a mutation that has not passed this sequence is `unverified` and cannot be cited as evidence, cannot pass the CommitBarrier of §60, and cannot be included in a promoted artifact. Unit assertions execute on the host through `AndroidBuildAdapter` under the locked toolchain; scenario assertions execute through `ScenarioExecutor` (§62.1); every execution records a `VerificationRun`.

### 64.3 Assertion ordering enforcement

For a requirement with observable behavior, `AssertionAuthor` persists the assertion with `authoredAtRevision` preceding the implementation revision, implementing the test-before-code rule of build spec §57.3 (the assertion must fail before implementation and pass after; assertions authored after a passing implementation are marked `post_hoc` with the evidence weight that rule assigns).

### 64.4 Vacuity check

`MutationProber` implements the non-vacuity requirement of build spec §57.5 for critical logic: it injects at least one fault into the implementation, confirms the assertion set fails, and records a passing set as `vacuous`, rejected as evidence.

### 64.5 Verification record schema

> **Schema projection:** `VerificationRun` is defined in `nirman-schemas.md` §2.53. Owner: TA §64.5.

The record carries the assertion's `authoredAtRevision` and `assertionTiming` (`pre` or `post_hoc`), so the completion authority can weight timing without re-deriving it.

### 64.6 Architecture tests

Orchestration is correct only when a mutation introducing a compile error cannot advance; when an assertion authored after implementation is flagged `post_hoc`; when a vacuous assertion set for a critical requirement is rejected; when a property counterexample blocks the probed mutation; and when every promoted artifact contains only verified mutations.

A repair is not verified by the newly passing assertion alone; repair verification follows build spec §56.7 (same-scenario re-execution from the deterministic starting state) and §57.6 (differential regression, stale or foreign evidence rejected). `VerificationLedger` records the comparison — repaired revision, failing revision, last-known-good revision, rerun original failing scenario, rerun affected previously passing scenarios — and rejects completion on any regression or invalid evidence dependency.

### 64.7 PropertyProber

`PropertyProber` extracts the input domain from the recorded assertion, generates inputs within the declared bound under the recorded seed, and executes each through the §64.2 unit or scenario path. A counterexample is recorded with its failing input and seed as a `VerificationRun` with method `property_probe` (SCHEMAS §2.53), and the probed mutation does not advance until the property holds or the requirement is re-scoped. Generation is deterministic in the seed: the same seed and bound always yield the same input sequence.

## 65. Android Emulator Scenario Coordinator

**ContractId:** `CONTRACT.RUNTIME.DEVICE_MATRIX`  
**Authoritative build-spec section:** §59  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §59, which is canonical for matrix declaration, the primary-profile rule, divergence semantics, and the capability-status mapping; this section defines its implementation. Extends §49 (Android Toolchain Authority and Environment) and §50 (Preview Coordinator), which remain the authority on device health and session lifecycle.

### 65.1 Components

| Component | Responsibility |
|---|---|
| DeviceMatrixResolver | Resolves the declared emulator-profile matrix against available Nirman-managed emulator sessions |
| DevicePool | Allocates and recycles Nirman-managed local Android emulator sessions |
| ScenarioDistributor | Assigns scenarios to emulator profiles and orders execution |
| DivergenceAnalyzer | Compares per-emulator-profile outcomes for the same scenario |
| CoverageReporter | Reports per-emulator-profile scenario coverage and declared gaps |

### 65.2 Resolution and admission

Implementing build spec §59.2, `DeviceMatrixResolver` classifies each declared entry as `available`, `unavailable`, or `user_required` before execution begins, using the toolchain authority of §49 (`user_required` covers firmware settings, licensed images, and consents; `unavailable` covers what the host cannot provide). The run proceeds only when the primary profile is available, and unavailable secondary entries are declared coverage gaps, never passes — both rules owned by build spec §59.2. Execution order is primary first, then declared-matrix order; the same matrix and scenario set always yield the same order.

### 65.3 Pool constraints

DevicePool must respect the resource reservations of the backpressure controller, since concurrent emulator boots are among the most expensive host operations. Boot cost estimates come from the ResourceProfiler of §69. The pool must serialize boots when host capacity cannot sustain parallel emulators.

### 65.4 Divergence record

> **Schema projection:** `ScenarioDivergence` is defined in `nirman-schemas.md` §2.54. Owner: TA §65.4.

Divergence classification follows build spec §59.4: a cross-profile divergence is a defect, not emulator noise, recorded with both profiles before repair. Default classification is `defect`; `environment_limitation` is an implementation-level annotation requiring cited evidence that the failure originates in the device or vendor rather than the application, and it does not change the defect outcome build spec §59.4 assigns. The record stays open until the scenario agrees on every profile: repair re-runs the full matrix, not only the failing profile.

### 65.5 Capability status mapping

`CoverageReporter` applies the capability-status mapping of build spec §59.5 using the build spec §5.6 vocabulary (a capability verified only on the primary profile is `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS`, not `SUPPORTED`). The per-outcome mapping — all matrix devices passed yields `SUPPORTED`; primary passed with declared gaps yields `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS`; primary passed and a secondary failed yields `DEGRADED` with the divergence cited; primary unavailable yields `USER_REQUIRED` — is the implementation projection of that mapping. The per-scenario-per-profile report rows are the coverage record; partial-coverage reporting follows build spec §59.3.

### 65.6 Architecture tests

Coordination is correct only when a missing secondary device produces a declared gap in the report; when a scenario passing on one API level and failing on another is recorded as a divergence defect; when emulator boots serialize under constrained host capacity; and when a primary-only pass reports `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS` rather than `SUPPORTED`.

## 66. Runtime Directive Service

**ContractId:** `CONTRACT.RUNTIME.DIRECTIVE`  
**Authoritative build-spec section:** §61  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §61, which is the sole authority on directive precedence, work preservation, and acceptance criteria; this section defines only the components, queueing, and the DirectiveEffect projection. Extends §7 (Scheduler and Background Execution) and §16 (Goal Mode and Long-Horizon Execution), which remain the authority on task lifecycle and cancellation semantics.

### 66.1 Components

| Component | Responsibility |
|---|---|
| DirectiveIntake | Validates and admits directives from the user or policy |
| DirectiveValidator | Rejects directives requesting prohibited behavior |
| DirectiveQueue | Holds admitted directives until the next decision point |
| ConstraintRegistrar | Registers accepted directives as active constraints |
| PlanReconciler | Determines which plan steps and evidence remain valid |

### 66.2 Application at decision boundaries

Application timing — decision-point only, never inside a mutation, tool call, or transaction — is owned by build spec §61.3; the kernel implements it: DirectiveQueue is drained at each decision point, directives are applied in issue order, and the applied set is recorded in the event ledger before selecting the next action.

### 66.3 Validation rules

The rejection grounds — policy gates, permission ceilings, evidence requirements, safety boundaries — are owned by build spec §61.4; DirectiveValidator enforces that list (additionally rejecting a directive that approves its own decision node or marks a requirement complete, as application-level projections of those grounds), records each rejection with the reason, surfaces it to the user, and never partially applies a rejected directive.

### 66.4 Plan reconciliation outcomes

> **Schema projection:** `DirectiveEffect` is defined in `nirman-schemas.md` §2.55. Owner: TA §66.4.

The outcome classes — remains valid, unvalidated, abandoned — are owned by build spec §61.5; PlanReconciler classifies every in-flight step into them, and work whose assumptions changed is recorded `invalidated` (the §61.5 unvalidated class) and requires revalidation before promotion.

### 66.5 Interaction with re-grounding

Re-grounding after application is a build spec §61.3 obligation (per §53.8); this section wires RegroundingService (§59) into it so the new constraint appears in every subsequent ContextPackage. A directive registered but absent from the next context package fails the build spec §61.6 acceptance clause.

### 66.6 Architecture tests

Architecture tests mirror the acceptance criteria of build spec §61.6 and add one implementation check: the applied DirectiveEffect record accounts for every in-flight step.

## 67. Agent Runtime Debugger

**ContractId:** `CONTRACT.RUNTIME.DEBUGGER`  
**Authoritative build-spec section:** §63  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §63, which is canonical for inspectable state, the debugger operations, the read-only default, and acceptance criteria; this section defines only the components, the RuntimeSnapshot projection, and the ledger read paths. Extends §17 (Lifecycle Hook Dispatcher) and §55 (Private Reasoning and Visible ReasoningStream Architecture), which remain the authority on the privacy boundary.

### 67.1 Components

| Component | Responsibility |
|---|---|
| StateSnapshotter | Captures inspectable runtime state at a point in time |
| DecisionBoundaryController | Pauses and resumes at kernel decision points |
| SurfaceTracer | Reconstructs all mutations and validations for one surface |
| DecisionTracer | Reconstructs why a step was selected, with cited evidence |
| EvidenceGapQuery | Lists requirements lacking required evidence kinds |

### 67.2 Snapshot schema

> **Schema projection:** `RuntimeSnapshot` is defined in `nirman-schemas.md` §2.56. Owner: TA §67.2.

As the implementation projection of the build spec §63.2/§63.3 boundary, the snapshot carries the context package manifest (never assembled prompt text) and tool inputs and outputs (never model reasoning tokens).

### 67.3 Privacy enforcement

The privacy boundary of build spec §63.3 is enforced structurally: the debugger's only read paths are the event ledger and the reasoning stream's structured events, so private reasoning tokens (never persisted per §55) are unreachable by construction.

### 67.4 Read-only guarantee

The read-only default of build spec §63.5 holds here: outside pause and resume, every debugger operation is a ledger query, and the debugger holds no mutation broker handle, no file-write permission, and no authority over authority decisions, evidence, or completion state.

### 67.5 Reconstruction from ledger

Because the runtime is event-sourced through the reducer of §45, SurfaceTracer and DecisionTracer operate on completed sessions as well as live ones. Any historical run remains fully inspectable without special instrumentation at the time it ran.

### 67.6 Architecture tests

Architecture tests mirror the acceptance criteria of build spec §63.6 and add one implementation check: a completed session is fully inspectable from the ledger alone.

## 68. External Trigger Gateway

**ContractId:** `CONTRACT.RUNTIME.TRIGGER`  
**Authoritative build-spec section:** §60  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §60, which is canonical for authority constraints, default posture, auditability, and acceptance criteria; this section defines only the gateway implementation. Extends §7 (Scheduler and Background Execution), which remains the authority on time-based initiation. This section adds externally originated admission.

### 68.1 Components

| Component | Responsibility |
|---|---|
| TriggerRegistry | Stores trigger definitions, scopes, and ceilings |
| TriggerAuthenticator | Verifies the originating credential or signature |
| AdmissionController | Decides whether a requested goal is within scope and ceiling |
| RateLimiter | Enforces per-trigger firing limits |
| TriggerAuditLog | Records every firing and its admission decision |

### 68.2 Admission pipeline

```text
trigger fires
  -> TriggerAuthenticator verifies credential
       fail -> reject, audit, stop
  -> RateLimiter check
       exceeded -> reject, audit, stop
  -> AdmissionController: goal kind in allowedGoalKinds?
       no -> reject, audit, stop
  -> AdmissionController: requested permissions <= permissionCeiling?
       no -> reject, audit, stop
  -> requiresApproval? -> emit DecisionNode, await user
  -> create task with permissions capped at ceiling
```

The cap formula — task ceiling = min(trigger ceiling, project policy ceiling) — implements the never-widen rule owned by build spec §60.3.

### 68.3 Default-disabled network surface

Default-disabled posture and explicit recorded enablement are owned by build spec §60.4 for external network-originated triggers; this section implements them for the network-originated source (`external_webhook`) and additionally guarantees no listening network surface exists while no webhook trigger is enabled.

### 68.4 Audit record schema

> **Schema projection:** `TriggerFiring` is defined in `nirman-schemas.md` §2.57. Owner: TA §68.4.

### 68.5 Isolation from authority

The authority constraints — no permission grants, no ceiling raises, no decision-node approvals, no policy-gate bypasses, no artifact promotions — are owned by build spec §60.3; the gateway implements them by being able only to create tasks, with all other operations remaining with the deterministic authorities of §23 and §27.

### 68.6 Architecture tests

Architecture tests mirror the acceptance criteria of build spec §60.6 and add two implementation checks: the rejection reason is typed, and an admitted task's ceiling equals min(trigger ceiling, policy ceiling).

## 69. Resource Profiler

**ContractId:** `CONTRACT.RUNTIME.PROFILING`  
**Authoritative build-spec section:** §64  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §64, which is the sole authority on profile records, honest estimation, degradation detection, and acceptance criteria; this section defines only instrumentation, the ResourceExecutionProfile projection, and the capacityVerdict gate. Extends §3 (Process Model) and the resource governance of §51, which remain the authority on process supervision and reservation enforcement.

### 69.1 Components

| Component | Responsibility |
|---|---|
| OperationTimer | Measures duration, peak memory, CPU, and disk delta per operation |
| ProfileStore | Persists profiles keyed by operation class, project, and host |
| ExecutionProfileEstimator | Derives the plan's `ResourceExecutionProfile` from stored profiles |
| CapacityChecker | Compares estimates against available host capacity |
| DegradationDetector | Flags operations drifting from their profile |

### 69.2 Measurement boundaries

Measurement must wrap the supervised process, not the model's description of it. The measured operation classes are those of build spec §64.1 (this implementation additionally instruments packaging steps and static-analysis passes); each is timed by the supervisor and written to ProfileStore keyed by host and project fingerprint per build spec §64.2.

### 69.3 Estimation contract

> **Schema projection:** `ResourceExecutionProfile` is defined in `nirman-schemas.md` §2.58. Owner: TA §69.3.

The profile's field set (with confidence and sample counts), physical-demand-only scope, and forbidden fields are owned by build spec §64.3; the unprofiled-below-minimum-samples honesty invariant is owned by build spec §64.4. This subsection defines the `ResourceExecutionProfile` projection that carries them — CPU, memory, disk, emulator slots, concurrency, build pressure, observed duration, confidence, and sample counts — feeding `ResourceIntegrityAuthority` (§59, BS §72); AI usage is telemetry and never an input to admission (BS §72).

### 69.4 Planning integration

The over-capacity obligations — reduce scope, re-sequence work, or surface the constraint, and never begin work predicted to exhaust the host — are owned by build spec §64.3; this section implements them via the `capacityVerdict` gate. `exceeds_declared_time_bound` is the implementation projection of the user-declared time bound of BS §64.3: it is returned only when the user has declared an explicit time bound for the goal and the observed duration of the same operations predicts it cannot be met; it is surfaced as a decision node and never terminates, degrades, or blocks a goal that has no declared bound (§7.2, ADR-218).

### 69.5 Degradation signals

The degradation-detection obligation — sustained drift from the profile raises a host or project health signal — is owned by build spec §64.5; this section implements it by comparing recent samples to the stored p90 and feeding the recovery ladder of §28, since such drift commonly indicates disk pressure, a corrupted cache, or a degraded emulator rather than an application defect.

### 69.6 Architecture tests

Architecture tests mirror the acceptance criteria of build spec §64.6 and add one implementation check: injected disk pressure surfaces as a host-health signal, not an application defect.

### 69.7 Performance measurement ownership

Performance measurements MUST use the following canonical ownership:

| Measurement family | Canonical record | Authority or consumer |
|---|---|---|
| CPU, memory, disk, process, emulator, workspace-I/O, concurrency, network, and liveness pressure | `ResourceIntegrityRecord` | `ResourceIntegrityAuthority` |
| Operation duration distributions, memory peaks, CPU peaks, disk deltas, and failure rates | `ResourceProfile` | Scheduler, validation planner, and recovery ordering |
| Parent, child, shared, estimated, and unavailable attribution | `UsageRecord` | Resource attribution and telemetry |
| Frame capture, transport, render, age, drops, freezes, blank surface, and presentation health | `FrameQualityObservation` | `RenderPipelineWatchdog` and preview diagnostics |
| Kernel transition liveness and evidence movement | `LoopHeartbeat` | `SupervisorLifecycle` and recovery detection |
| Durable event, reducer, replay, and transaction timing | Event metadata or an explicitly registered extension | Diagnostics only unless an existing authority adopts it |

A measurement record MUST identify its source operation, task or session, project revision where applicable, environment identity where applicable, policy or configuration version, capture time, and evidence or telemetry classification.

A performance measurement MUST NOT become a second source of truth for lifecycle, policy, permission, evidence, preview, artifact, signing, or completion state. The records named in the table above are already registered or owned elsewhere in the corpus; this subsection assigns ownership of measurements and registers no new schema, record, or authority.

## 70. Supply-Chain and Artifact Provenance Runtime

**ContractId:** `CONTRACT.RUNTIME.SUPPLY_CHAIN`  
**Authoritative build-spec section:** §58  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §58. Extends §39 (Sandbox and Process Separation) and §54 (Native Isolation and External Side-Effect Boundaries), which remain the authority on host isolation. This section verifies the produced application and its dependencies.

### 70.1 Components

| Component | Responsibility |
|---|---|
| DependencyResolver | Resolves declared dependencies to exact versions with integrity hashes |
| SubstitutionDetector | Flags names resembling known packages |
| AppSecurityScanner | Scans generated application code and manifest for insecure patterns and exploit patterns |
| SbomBuilder | Assembles the bill of materials for a produced artifact |
| ProvenanceRecorder | Binds artifact checksum to revision, toolchain, and SBOM |
| FindingDispositionStore | Records each finding as blocking or accepted with reason |
| SecurityRiskScorer | Severity-weighted aggregation of `AppSecurityScanner` findings into a structured `SecurityRiskScore` bound to the artifact revision; never grants authority or promotes artifacts |
| SecurityAuditGenerator | Composes all `FindingDispositionStore` records, `SecurityRiskScore`, SBOM completeness, and provenance identity into a security audit report artifact per promoted artifact; read-only projection — `ProvenanceRecorder` remains the promotion gate |

### 70.2 Dependency verification

> **Schema projection:** `ResolvedDependency` is defined in `nirman-schemas.md` §2.59. Owner: TA §70.2.

Dependency blocking follows build spec §58.3: a verdict other than `verified` blocks the build. A `hash_mismatch` against a previously recorded hash is treated as a supply-chain event, not a transient failure, and must be surfaced rather than auto-retried.

### 70.3 Application security checks

`AppSecurityScanner` must run before packaging and must check the categories enumerated in build spec §58.2, operating on the generated sources and merged manifest rather than on model claims about them. Each finding records the file, location, category, and severity. In addition to the §58.2 enumerated categories, `AppSecurityScanner` applies exploit-pattern matching against a deterministic catalog of known Android exploit patterns (intent-redirection chains, fragment injection, unsafe broadcast receivers, exported provider access without read/write permission guards, and Parcel deserialization gadgets), producing a typed `ExploitPatternFinding` per match.

After `AppSecurityScanner` completes, `SecurityRiskScorer` aggregates findings per build spec §58.2 — by severity, with the category breakdown as an implementation elaboration — into a structured `SecurityRiskScore` (critical count, high count, medium count, low count, overall risk level, and blocking status) bound to the artifact revision. `SecurityRiskScore` is a read-only projection; `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

`SecurityAuditGenerator` then composes `FindingDispositionStore` records, the `SecurityRiskScore`, SBOM completeness, and `ArtifactProvenance` identity into a security audit report artifact that is attached to the artifact record before promotion. The security audit report is a read-only projection artifact; promotion authority remains with `ProvenanceRecorder`. Together, `AppSecurityScanner`, `SecurityRiskScorer`, and `SecurityAuditGenerator` constitute the `AndroidSecurityIntelligenceService` referenced by BS §58.2.

### 70.4 SBOM and provenance schema

> **Schema projection:** `ArtifactProvenance` is defined in `nirman-schemas.md` §2.60. Owner: TA §70.4.

`ProvenanceRecorder` enforces the promotion bar of build spec §58.4 and the disposition-completeness rule of §58.5, refusing to mark an artifact promotable when the SBOM is incomplete or any finding lacks a disposition.

### 70.5 Disposition discipline

`FindingDispositionStore` structurally enforces the disposition discipline of build spec §58.5: every finding terminates in `blocking` or `accepted_with_reason`, the store rejects a disposition with an empty reason — preventing silent suppression — and the final report renders all findings and dispositions.

### 70.6 Architecture tests

The runtime is correct only when a hardcoded secret blocks packaging; when an unpinned or hash-mismatched dependency blocks the build; when a name resembling a known package is flagged; when an artifact with an incomplete SBOM is not promotable; and when a finding cannot be dispositioned without a reason.

### 70.7 AndroidPrivacyIntelligenceService

**Role:** aggregate query facade and static privacy and compliance validation — read-only services; no authority, no AI-usage budget.

#### 70.7.1 AndroidPrivacyIntelligenceService

`AndroidPrivacyIntelligenceService` is the supervisor-owned, read-only aggregate query facade unifying Personal Identifiable Information (PII) classification, personal data flow tracking, data minimization checking, privacy policy generation, and open-source license notice composition across the generated Android application. It exposes a typed query surface to compliance workers (`Security Worker`, `Documentation Worker`, `Release Worker`) and registered IPC command handlers.

`AndroidPrivacyIntelligenceService` coordinates four deterministic analytical and composition components:
1. *PII field classification:* Invokes `PiiFieldClassifier` (§70.7.2) to detect and tag sensitive personal data fields across Room database entities, Jetpack Compose form inputs, and network data transfer objects.
2. *Data minimization and over-collection auditing:* Invokes `DataMinimizationChecker` (§70.7.3) to correlate detected PII and sensor access against the application's declared functional requirements, flagging unnecessary or excessive data collection.
3. *Privacy policy and data safety synthesis:* Invokes `PrivacyPolicyGenerator` (§70.7.4) to generate project-specific Privacy Policy documentation and Google Play Data Safety declaration drafts based on concrete codebase evidence.
4. *Open-source notice composition:* Invokes `OpenSourceNoticeComposer` (§70.7.5) to assemble third-party library license notices from verified SBOM metadata into an in-app notice file and display surface.

`AndroidPrivacyIntelligenceService` creates no second authority. It does not directly mutate project source or bypass policy; all privacy and documentation proposals route through `MutationBroker` (BS §43.2) and commit via `ConstructionTransaction` (BS §42.2). `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

#### 70.7.2 PiiFieldClassifier

`PiiFieldClassifier` is the static analysis module that inspects Android data models to locate and categorize personal identifiable information:
1. *AST field and type scanning:* Analyzes Kotlin data classes, Room `@Entity` fields, and serialization DTOs (`@Serializable`, `@JsonClass`) using semantic pattern matching and attribute dictionaries to classify fields into standard privacy categories (e.g. `NAME`, `EMAIL`, `PHONE_NUMBER`, `PHYSICAL_ADDRESS`, `GEOLOCATION`, `DEVICE_IDENTIFIER`, `BIOMETRIC_DATA`, `FINANCIAL_DATA`).
2. *Form input and sensor tagging:* Statically scans Jetpack Compose `TextField` inputs, keyboard type configurations (`KeyboardType.Email`, `KeyboardType.Phone`), and sensor API call-sites (Location services, Camera, Microphone) to tag originating PII collection points.
3. *Data-flow taint binding:* Correlates classified PII sources with `AndroidDataFlowAnalyzer` (TA §47.4) taint graphs to identify persistence sinks (Room database tables, DataStore preferences) and network egress points (Retrofit/Ktor endpoints).

#### 70.7.3 DataMinimizationChecker

`DataMinimizationChecker` audits personal data collection against declared functional requirements to enforce GDPR, CCPA, and Google Play data minimization principles:
1. *Functional necessity correlation:* Cross-references each classified PII source and runtime permission against the `GoalContract` and `ImplicitRequirementMiner` specifications to verify that every collected personal data element is functionally necessary for declared app capabilities.
2. *Excessive collection detection:* Flags instances where high-precision or sensitive data is requested when low-precision alternatives suffice (e.g. requesting `ACCESS_FINE_LOCATION` when coarse location or zip code satisfies the feature).
3. *Retention and auto-purge verification:* Verifies that local caches, temporary logs, and session tokens containing PII declare deterministic expiration or scheduled WorkManager pruning routines rather than persisting indefinitely.

#### 70.7.4 PrivacyPolicyGenerator

`PrivacyPolicyGenerator` generates legally grounded, evidence-backed privacy documentation for the Android project:
1. *Evidence-based privacy policy synthesis:* Generates a comprehensive `PRIVACY_POLICY.md` based on actual AST evidence (classified PII fields, persistent data stores, third-party SDKs, and declared permissions) rather than generic boilerplates.
2. *Google Play Data Safety mapping:* Produces structured responses for the Google Play Console Data Safety questionnaire, detailing which data types are collected or shared, whether data is encrypted in transit, and whether users can request data deletion.
3. *Erasure and opt-out flow guidance:* Verifies the presence of user data deletion flows ("Delete Account" / "Clear Data") and analytics opt-out toggles required for Google Play and GDPR compliance.

#### 70.7.5 OpenSourceNoticeComposer

`OpenSourceNoticeComposer` manages third-party software license compliance for the produced Android application:
1. *License aggregation from SBOM:* Consumes `ResolvedDependency` and `SbomBuilder` metadata (§70.1, §70.4) to extract canonical package names, versions, license identifiers (SPDX), and license text for all compiled dependencies.
2. *In-app notice asset generation:* Generates the required open-source notice text file (`res/raw/third_party_licenses.txt` or `assets/NOTICE.txt`) and synthesizes a compliant Jetpack Compose license display screen or dialog.
3. *Incompatible license verification:* In conjunction with `SbomBuilder` (§70.1) and `ProvenanceRecorder` (§70.1), verifies that no viral or restricted copyleft licenses (e.g. GPL-3.0) infect proprietary client artifacts, ensuring safe commercial and release distribution.

#### 70.7.6 AndroidThreatSketchSynthesizer

`AndroidThreatSketchSynthesizer` synthesizes pre-generation threat models and negative verification scenarios for security-sensitive Android applications:
1. *Attack surface enumeration:* Analyzes the `AndroidConstructionContract` to identify exposed attack surfaces: exported Android components, custom intent filters, deep links, cleartext network endpoints, and shared preferences.
2. *Trust boundary mapping:* Formulates structured `AndroidThreatSketch` models delineating boundaries between untrusted external inputs (network payloads, intent extras) and sensitive local storage (Android Keystore, Room databases).
3. *Negative scenario derivation:* Directly derives executable negative test scenarios (`NegativeE2EScenario`) passed to `AndroidScenarioDriver` to verify that forged intents, malformed deep links, and SQL injection payloads are rejected before packaging promotion.

## 71. Agent Reasoning Runtime and Capability Layer

**ContractId:** `CONTRACT.RUNTIME.REASONING`  
**Authoritative build-spec section:** BS §66  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §66, which is canonical for the reasoning contract, except the fine-grained cycle state machine of §71.4, which is canonical over cycle transitions (ADR-230; build spec §52.2 is the coarse durable projection and §66.3 the contract-level cycle, whose single `OBSERVE` name projects onto the canonical machine's `OBSERVE` and `OBSERVE_RESULT`). Extends §58 (Agent Execution Kernel and Runtime Formalization) and §21 (Authority Hierarchy and Recovery Invariants), which remain the authority on the execution loop and on who decides. This section adds the reasoning components that drive the existing loop. It introduces no second loop and no second authority.

### 71.1 Position in the runtime

```text
Provider / model
      |
      v
PrivateReasoningRuntime        (transient, never persisted)
      |
      v  ReasoningArtifact
AgentReasoningEngine
      |
      v  proposed action
AgentExecutionKernel (§58)
      |
      v  authorization request
Policy and lifecycle authorities (§21, §27)
      |
      v  granted invocation
CapabilityLayer -> skill | tool | worker | swarm | session
      |
      v  results
EvidenceAuthority evidence ledger (§23.3) -> ReflectionEngine -> next cycle
```

The reasoning engine sits above the kernel and below nothing. It cannot reach the capability layer except through the kernel, and the kernel cannot execute except through the authorities. The arrow from `AgentReasoningEngine` to `AgentExecutionKernel` is also the process boundary of §3.5: `PrivateReasoningRuntime`, `AgentReasoningEngine`, and the deliberation runtime of §72 run in the worker's `NirmanWorker.exe`; the kernel and everything beneath it run in `NirmanSupervisor.exe`, and the provider's response reaches `PrivateReasoningRuntime` only through the supervisor's `ModelGateway` and the `WorkerConnection`.

### 71.2 Components

| Component | Responsibility |
|---|---|
| PrivateReasoningRuntime | Hosts transient model reasoning; retains nothing verbatim |
| AgentReasoningEngine | Drives the cycle state machine and emits ReasoningArtifacts |
| HypothesisManager | Owns hypothesis lifecycle, discriminating tests, and rejection records |
| StrategySelector | Compares alternatives and records the cited selection basis |
| ReflectionEngine | Produces ReflectionRecords from expected-versus-observed comparison |
| CapabilityRegistry | Answers runtime capability discovery queries |
| CapabilityBroker | Converts a selected strategy into an authorized invocation |
| DelegationManager | Issues DelegationGrants and enforces the two ceiling invariants |
| SwarmGraphManager | Applies agent-proposed revisions to the live execution graph |
| ExecutionModeSelector | Proposes a mode and records the policy constraints applied |

### 71.3 ReasoningArtifact schema

> **Schema projection:** `ReasoningArtifact` is defined in `nirman-schemas.md` §1.27. Owner: BS §66.2.

The store structurally enforces the `selectionBasis` admissibility rule of build spec §66.2 — the basis must cite evidence, constraints, or prior failure signatures, never bare reasoning prose, so a basis that cites nothing is inadmissible even when non-empty — rejecting an inadmissible artifact and so preventing unjustified strategy selection. No field of this record holds model reasoning text; `selectedStrategy` and `expectedEffect` are declarative statements, not transcripts.

### 71.4 Cycle state machine

```text
OBSERVE -> UNDERSTAND -> HYPOTHESIZE -> STRATEGIZE -> SELECT -> AUTHORIZE
AUTHORIZE  granted -> EXECUTE -> OBSERVE_RESULT -> REFLECT -> UPDATE -> DECIDE
AUTHORIZE  denied  -> STRATEGIZE   (denial recorded as an active constraint)
DECIDE  continue -> OBSERVE
DECIDE  repair   -> HYPOTHESIZE
DECIDE  replan   -> UNDERSTAND
DECIDE  delegate -> DELEGATE -> OBSERVE
DECIDE  branch   -> SPECULATE (§88) -> OBSERVE
DECIDE  terminate -> COMPLETED | BLOCKED | WAITING | RECOVERED | SAFELY_FAILED | ESCALATED
```

Transitions are recorded as kernel events. The engine cannot enter `EXECUTE` from any state except a granted `AUTHORIZE`, which makes the authority path structural rather than procedural.

This section is the **canonical cycle state machine** and the single authority over cycle transitions (ADR-230). It is the *fine-grained* machine: its thirteen states and the edges above are the only legal cycle transitions, and an implementation MUST reject and record any transition not drawn here. The build spec §52.2 vocabulary is not a second machine. It is the **coarse durable-projection vocabulary** — the nine names under which cycle states are recorded in `LoopHeartbeat.stateEntered` and reported to the supervisor — and build spec §52.2 carries the total, surjective projection from these thirteen states onto those nine. Every fine state maps to exactly one coarse state and every coarse state has at least one fine preimage, so no cycle state is unrecordable and no recorded name is unreachable. The six terminal cycle outcomes above are neither fine nor coarse cycle states; they are the kernel cycle outcomes that build spec §26.14 already names and that the build spec §33.2 mapping table projects onto task and session states.

### 71.5 HypothesisManager

> **Schema projection:** `Hypothesis` is defined in `nirman-schemas.md` §1.29. Owner: BS §66.6.

`HypothesisManager` enforces the hypothesis-evidence rules of build spec §66.6: no `SUPPORTED` or `REJECTED` without an evidence reference, no retest of a `REJECTED` hypothesis against unchanged evidence, and untested discriminating tests exposed so the kernel prefers testing over untargeted repair. Rejected hypotheses are written as FAILURE memory records under the memory-record rules of build spec §53.2 and feed the failure signatures of §63.4.

`NegativePremiseStore` is the named partition within `ProjectMemoryStore` (§59.1) that indexes rejected hypotheses, failed AST patch fingerprints, and refuting evidence records. Before entering `STRATEGIZE` (§71.4) or authorizing a mutation proposal, `StrategySelector` queries `NegativePremiseStore` to prune candidate hypotheses that match known refuted premises, preventing repetitive regression cycles and redundant model deliberation (BS §52.3).

### 71.6 CapabilityRegistry and discovery

> **Schema projection:** `CapabilityDescriptor` is defined in `nirman-schemas.md` §2.61. Owner: TA §71.6.

Discovery is a query, not a grant, per build spec §66.7: `discoverCapabilities(objective, constraints, environment)` returns descriptors whose availability is computed from the toolchain authority of §49 and the environment planner, with permissions still evaluated at invocation. A newly registered skill or tool becomes discoverable without modifying the reasoning engine, which is what makes the runtime extensible rather than hardcoded.

### 71.7 Invocation and delegation persistence

> **Schema projection:** `CapabilityInvocation` is defined in `nirman-schemas.md` §1.30. Owner: BS §66.7.

> **Schema projection:** `DelegationGrant` is defined in `nirman-schemas.md` §1.31. Owner: BS §66.8.

Artifacts, reflections, hypotheses, invocations, and grants are stored in the SQLite execution ledger, keyed by task and project, and are therefore replayable by the trajectory engine of §58 and inspectable by the debugger of §67 without special instrumentation.

### 71.8 DelegationManager enforcement

- `WorkerCompatibilityValidator` — The delegation component that verifies model, context, and modal compatibility before a worker grant is issued.

Before issuing a grant the manager computes:

```text
child.capabilityCeiling     ⊆ parent.capabilityCeiling
child.resourceRequirements  ⊆ parent.admissibleResourceCapacity
child.depth                 = parent.depth + 1  ≤  maxDepth
child.workspaceScope        ⊆ parent.workspaceScope
```

Any violation denies the grant with a typed reason. `parent.admissibleResourceCapacity` is the parent's currently admissible physical capacity as evaluated by ResourceIntegrityAuthority (§59, BS §72) net of aggregate outstanding child resource reservations; the manager must recompute it at issue time rather than trusting a cached value, since sibling grants and host pressure change it. `executionTimeout` retains its build spec §66.8 semantics — a liveness bound for a hung child, not an AI-usage or goal-duration budget. Revoking a parent grant must cascade to every descendant, reusing the cancellation propagation of §58.

**Pre-dispatch worker compatibility validation.** Before `DelegationManager` issues a `DelegationGrant`, `WorkerCompatibilityValidator` executes a 4-dimensional compatibility audit:
1. *Context Capacity Match:* The task's assembled `ContextPackage` size must not exceed the candidate model profile's verified attendable context capacity ($C_{\text{task\_package}} \le C_{\text{model\_context}}$).
2. *Modal Capability Match:* If the task requires visual verification (e.g., `Visual QA Worker` analyzing emulator screenshots or icon assets), the candidate model profile must explicitly declare multimodal vision support.
3. *Structured Output Match:* If the task contract requires strict schema-validated proposals, the candidate model must support native or grammar-constrained structured output.
4. *Toolchain Profile & AppContainer Ceiling:* The worker's assigned AppContainer sandbox profile must encompass all files, paths, and `ToolBroker` capabilities required by the task.
If compatibility validation fails, `DelegationManager` does not dispatch an invalid worker; it escalates the model profile or routes to an alternate qualified worker role, recording `INCOMPATIBLE_WORKER_PROFILE`.

### 71.9 Failure modes and recovery

| Failure | Runtime behavior |
|---|---|
| Artifact with empty selectionBasis | Rejected at write; cycle returns to STRATEGIZE |
| Authority denies the proposed action | Denial recorded as constraint; STRATEGIZE re-entered |
| Hypothesis rejected with no evidence | Write rejected; hypothesis remains TESTED |
| All hypotheses rejected | Cycle terminates SAFELY_FAILED or ESCALATED |
| Delegation ceiling violation | Grant denied; parent continues without the child |
| Child's physical resource requirements can no longer be admitted | Child is queued, rescheduled, or checkpointed per BS §72; parent observes and replans |
| Swarm revision denied by policy | Graph unchanged; denial recorded |
| Mode selection exceeds policy | Mode downgraded to the highest permitted mode |
| Cycle repeats a strategy against unchanged evidence | RepeatedFailureDetector raises StrategyChangeRequired; checkpoint, then strategy change, evidence acquisition, delegation, or ESCALATED rather than silent continuation |

No failure mode above permits proceeding on an assumption. Each either records a constraint and retries within authority, or terminates in a declared state.

### 71.10 Architecture tests

The runtime is correct only when an artifact with an empty selection basis is rejected; when a denied invocation returns the cycle to strategy selection with the denial visible in the next artifact's constraints; when a rejected hypothesis is not retested against unchanged evidence; when a delegation request exceeding either ceiling is denied and recorded; when revoking a parent grant terminates every descendant; when a newly registered capability becomes discoverable without a code change to the reasoning engine; and when no persisted record in any of these tables contains verbatim model reasoning.


## 72. Deep Deliberation Runtime

**ContractId:** `CONTRACT.RUNTIME.DELIBERATION`  
**Authoritative build-spec section:** BS §68  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §68. Extends §71 (Agent Reasoning Runtime) and §58 (Agent Execution Kernel), which remain the authority on the reasoning cycle and the execution loop. This section adds the runtime that decides how much reasoning to perform. It introduces no third loop.

### 72.1 Position in the runtime

```text
AgentExecutionKernel (§58)
      |
      v
AgentReasoningEngine (§71)
      |
      v  at HYPOTHESIZE / STRATEGIZE
DeepDeliberationRuntime (§72)
      |
      +-- deliberate            (progress-governed passes)
      +-- acquire evidence      (read-only tool observation)
      +-- compete hypotheses    (discriminating tests)
      +-- critique strategy     (counterexample search)
      +-- escalate model        (same permission ceiling)
      +-- detect no progress    (change approach, not more reasoning)
      |
      v  sufficiency reached or termination recorded
ReasoningArtifact (§71.3)
      |
      v
Kernel AUTHORIZE -> CapabilityBroker
```

Deliberation returns control to the reasoning engine. It never reaches the capability layer directly and never bypasses the kernel's authorization step.

### 72.2 Components

| Component | Responsibility |
|---|---|
| DeliberationController | Drives passes and records the deliberation decision per §68.3 |
| DeliberationProgressEvaluator | Measures per-pass movement: evidence delta, uncertainty delta, hypotheses eliminated, strategy stability |
| DiminishingReturnDetector | Classifies NO_PROGRESS against the configured threshold and forces an approach change |
| RepeatedFailureDetector | Detects a strategy retried against unchanged evidence, uncertainty, and constraints and raises StrategyChangeRequired |
| EvidenceAcquisitionPlanner | Selects the cheapest decisive read-only observation; runs on every EvidenceAcquisitionTrigger |
| ReasoningEffortSelector | Selects the granted level from task requirements, uncertainty, risk, provider capability, policy, and available execution capacity |
| SufficiencyEvaluator | Evaluates the build spec §68.7 conjunction, not stated confidence |
| HypothesisEvaluator | Runs competition, ranks by decisiveness, records refutation |
| StrategyCritic | Adversarial critique and counterexample search; emits findings and evidence requests only |
| DeliberationModelRouter | Escalates model within an unchanged permission ceiling; a selected model must satisfy the step's `requiredReliability` (§59.12) |
| DeliberationContinuationManager | Persists session state across requests and compaction |
| DeliberationRecordStore | Persists records; rejects inadmissible ones |

There is no budget manager. No component owns an AI-usage ceiling, reserves or settles reasoning expenditure, or refuses a pass on a count; `reasoningUsage` and `resourceUsage` are written as telemetry after the fact.

### 72.3 DeliberationRecord and session schema

> **Schema projection:** `DeliberationRecord` is defined in `nirman-schemas.md` §1.33. Owner: BS §68.2.

> **Schema projection:** `DeliberationSession` is defined in `nirman-schemas.md` §2.62. Owner: TA §72.3.

DeliberationRecordStore must reject a record whose `passCount` exceeds one while `continuationReasons` has fewer entries than the additional passes, and must reject any record containing verbatim model reasoning in a text field. No field of either schema is a reasoning transcript.

`reasoningUsage.accountingStatus` distinguishes provider-`reported` usage, runtime-`estimated` usage, and `unavailable` usage. The store implements the accounting rules of build spec §68.2: it never fabricates provider-reported reasoning usage — when the provider does not expose reasoning-token accounting, the record states `estimated` or `unavailable`, and estimates remain telemetry that can never satisfy a sufficiency or certification requirement — and `reasoningUsage`, `resourceUsage`, `passCount`, and `toollessPassCount` are observational fields no component reads to authorize, deny, throttle, pause, or terminate a pass.

**Reasoning reproducibility contract.** To ensure deterministic replay, audit verification, and regression tracking across deliberation passes without capturing verbatim chain-of-thought, every pass recorded in `DeliberationRecord` points to its underlying `providerRequestRefs: requestId[]`. The runtime deterministically binds:
1. `requestHash`: The SHA-256 digest of the normalized prompt assembly, system instruction, and schema contract.
2. `contextIntegrityHash`: The cryptographically bound context package hash (BS §53.11).
3. `reasoningSeed`: The random seed supplied to the provider model (or `null` when provider-unsupported).
4. `modelProfileId` and sampling parameters (`temperature`, `topP`).
When historical replay or reproducibility testing is triggered via `TrajectoryReplayEngine` (§58.10), re-running the identical prompt digest and context hash under fixed seeds must reproduce the identical structured deliberation graph (hypotheses, strategy selection, and discriminating tests). Any structural divergence under deterministic sampling is flagged as `REASONING_DRIFT_DETECTED` and falls back to cached deliberation traces rather than silently branching into unverified states.

### 72.4 Pass loop

```text
enter deliberation (from HYPOTHESIZE or STRATEGIZE)
  -> ReasoningEffortSelector
       task requirements + uncertainty + risk + provider capability
       + policy + available execution capacity
       -> granted effort level, effortGrantId
  -> loop:
       ContextOrchestrator.assembleDeliberationContext()
         -> preserve objective, active hypotheses, rejected strategies,
            constraints, evidence, pending evidence-acquisition trigger;
            fit to provider context capacity (ContextCapacityPlanner)

       ModelGateway.request()
         -> normalized ReasoningSettings under effortGrantId
         -> provider/model

       ModelGateway response
         -> structured proposal / reasoning summary / read-only observation request

       evidence acquisition
         -> if pendingEvidenceAcquisitionTrigger or an observation was requested:
              EvidenceAcquisitionPlanner selects the cheapest decisive
              read-only observation; ToolBroker executes it
         -> a pass that acquired no new observation raises
              EvidenceAcquisitionTrigger for the next pass (recorded)

       DeliberationProgressEvaluator.measure()
         -> evidence delta
         -> uncertainty delta
         -> hypotheses eliminated
         -> strategy stability
         -> refutation attempted
       record reasoningUsage / resourceUsage (telemetry only)

       SufficiencyEvaluator.evaluate()
         sufficient
           -> checkpoint session; terminate SUFFICIENT

       DiminishingReturnDetector.classify() + RepeatedFailureDetector.classify()
         progress possible
           -> continuation decision with a recorded continuationReasons entry
           -> next pass
         diminishing returns | StrategyChangeRequired
           -> change strategy
           | GATHER_EVIDENCE
           | DELEGATE
           | BRANCH (speculation runtime, §88)
           | ESCALATE_MODEL
           | ESCALATE (human decision)
           | terminate NO_PROGRESS when none of those changes is available

  -> checkpoint session state
  -> emit DeliberationRecord
  -> return control to AgentReasoningEngine (kernel)
```

The loop has no path from a pass directly to execution. Sufficiency returns to the reasoning engine, which emits the ReasoningArtifact and submits it for authorization. The loop implements the no-usage-budget and evidence-acquisition-trigger rules of build spec §68.4 and §68.8: it has no usage-exhaustion path and no fixed pass ceiling, a pass is never refused because of tokens, requests, cost, reasoning tokens, pass count, or elapsed time, an observation-free pass is a signal to obtain evidence, never a termination condition, and anti-thrash protection comes from the build spec §68.13 detectors (DiminishingReturnDetector, RepeatedFailureDetector, StrategyChangeRequired), never a pass count. The termination outcomes are owned by build spec §68.14 (`SUFFICIENT`, `NO_PROGRESS`, `ESCALATED`, `ABANDONED`); this loop records `SUFFICIENT` and `NO_PROGRESS`, and escalation and abandonment follow the build spec §68.14 paths. Physical resource pressure is handled by ResourceIntegrityAuthority (BS §72) — a pass waits, is rescheduled, or is checkpointed — and never by the deliberation runtime terminating itself.

- `ExplorationStrategySelector` — The deliberation component that decides between exploiting known repair patterns and exploring speculative solutions.

**Exploration versus exploitation policy.** When deciding whether to refine an existing strategy or spawn a speculative candidate branch, `ExplorationStrategySelector` evaluates:
- *Exploit:* Selected when a matching repair pattern exists in `AndroidRepairRegistry` with verified historical success and prior attempts for that pattern on the current failure signature $< 2$.
- *Explore:* Selected when task uncertainty $> 0.40$, when consecutive exploitation attempts yield zero evidence progress, or when the problem involves novel dependency conflicts without registered repair recipes. Exploratory branches execute within isolated candidate sandboxes under BS §65 (`CONTRACT.RUNTIME.SPECULATION`).

### 72.5 ReasoningEffortSelector

The selector computes the granted level as the minimum of the requested level, the policy ceiling for the task's risk class, the level the currently available execution capacity (physical resource integrity, BS §72) admits, and the level the routed provider actually supports, raised to the task's minimum required effort from its requirements, uncertainty, and risk. The grant is issued under an `effortGrantId`; the grant, the requested level, and the binding constraint are recorded, so a downgrade is visible rather than silent. AI usage is not an input to the grant, per build spec §68.5: there is no remaining budget, and no grant is ever refused or lowered because of tokens, requests, cost, or elapsed time.

The selector must have no capability to raise a permission ceiling and no path to the policy engine's grant functions. Effort and permission are separate axes by construction.

Provider reasoning capability is an ordered capability, not a boolean.

The selector must resolve the requested runtime effort to a provider-supported effort level using the provider's declared ReasoningCapabilityProfile. If the provider cannot represent the requested level, the selector must record the requested level, the granted level, and the exact capability constraint.

A provider capability downgrade must never be represented as successful execution at the requested effort level.

If the task's minimum required effort exceeds the highest effort level supported by every approved provider/model, the runtime must terminate deliberation with a typed capability gap rather than silently executing at a lower level.

The runtime must distinguish:

- requested effort;
- granted runtime effort;
- provider-native effort;
- observed reasoning usage.

These values must remain separately auditable.

### 72.6 SufficiencyEvaluator

The evaluator implements the build spec §68.7 conjunction. It consults the required-evidence set for the change's risk class, the uncertainty threshold for that class, strategy stability across the last pass, the presence of a validation plan, and whether HypothesisEvaluator reports an untested discriminating test.

The evaluator implements the build spec §68.7 conjunction: a stated confidence value is an input to uncertainty only and can never satisfy it alone, and sufficiency is refused while any required-evidence element for the change's risk class is absent — for a change classified high-risk, architectural impact, dependency impact, affected-symbol analysis, regression plan, or validation plan.

### 72.7 HypothesisEvaluator and StrategyCritic

HypothesisEvaluator enumerates candidates, obtains a discriminating test per candidate from EvidenceAcquisitionPlanner, ranks by decisiveness divided by cost, executes the most decisive affordable test, and records refutation against the hypothesis records of §71.5. At DEEP and above it must report whether the last pass attempted refutation or only confirmation; a confirmation-only pass does not count as competition.

StrategyCritic runs before authorization at DEEP and above for the change classes enumerated in build spec §68.10. It holds no mutation broker handle, no evidence-approval capability, and no completion authority. Its output is a rejection finding or a list of evidence requests routed back through EvidenceAcquisitionPlanner.

**Pre-implementation counterfactual fault audit.** When executing adversarial critique for Android project strategies at `DEEP` and `EXHAUSTIVE` deliberation, `StrategyCritic` implements the counterfactual fault audit owned by build spec §68.10: the critique must explicitly evaluate its five operational hazards — schema regression, lifecycle mismatch, concurrency race hazards, offline-first divergence, and dependency breaking changes — realized for Android projects through five canonical operational checks (implementation projection):
1. *Process death & state recreation:* Evaluates whether in-memory states survive OS process termination when backgrounded, requiring explicit `SavedStateHandle` or `rememberSaveable` state hoisting.
2. *Runtime permission denial:* Evaluates whether revoking runtime permissions (e.g., `POST_NOTIFICATIONS`, `ACCESS_FINE_LOCATION`, `CAMERA`) crashes the app or triggers graceful unblocked fallback UI with rationale presentation.
3. *Network offline & degradation:* Evaluates behavior during immediate airplane mode or socket timeouts, ensuring data access routes through Room offline-first caching and StateFlow streams rather than unbuffered HTTP calls.
4. *Configuration changes & lifecycle tearing:* Evaluates screen orientation flips, split-screen toggling, and system dark/light theme switches, verifying that Coroutine scopes are bound to `viewModelScope` rather than ephemeral Activity contexts.
5. *API level divergence / minSdk hazards:* Evaluates framework API calls against the target project's `minSdk`, verifying that newer API usages are guarded by static `Build.VERSION.SDK_INT` checks.
If the candidate strategy fails any of these 5 counterfactual checks without defensive handling, `StrategyCritic` emits a `COUNTERFACTUAL_FAULT_HAZARD` rejection finding citing the specific fault vector, forcing the cycle back to strategy selection or generating targeted evidence requests.

> **Schema projection:** `TrajectoryAssessment` is defined in `nirman-schemas.md` §2.124. Owner: TA §72.7.

### 72.7.1 Trajectory reassessment

Local task correctness is not global trajectory correctness: a worker can be entirely correct about its assigned task while the swarm converges on an implementation that no longer satisfies the original intent (ADR-250). Trajectory reassessment evaluates the original intent and accepted requirements (`AgentTask.userRequest`, the normalized specification, and the Conversation aggregate's accepted requirements, build spec §82.1) against current trajectory evidence (frontier, evidence watermark, and committed change surfaces through the ImpactGraph/§36.4 dependency relation). Its verdict is `TRAJECTORY_ALIGNED` or `TRAJECTORY_DRIFTED`, its proposal `CONTINUE`, `REPLAN`, or `BRANCH_ALTERNATIVE`, and its evidence requests route back through `EvidenceAcquisitionPlanner`.

A trajectory assessment may recommend a change in course, but only existing authoritative planning/reconciliation machinery may enact that change. The evaluation is a logical supervisor activity, not a component: it cannot edit code, cannot invalidate evidence, cannot veto completion, cannot mint authority, and is not a second completion gate. It enters the §58.12/§58.12.1 reconciliation/replanning machinery and, where its proposal requires, the existing USER gates.

Triggers are execution-boundary predicates, never standalone elapsed time: `MEANINGFUL_GRAPH_PROGRESS`, `EPOCH_TRANSITION`, `STRATEGY_CHANGE`, or `ACCUMULATED_CONTRADICTION` (§58.13, §58.14, and §58.12 boundaries). Within one (graphRevision, planRevision, executionEpochId, triggerKind) boundary, an assessment is emitted at most once unless a later authoritative event creates a new trigger boundary; every trigger carries `triggerEventId` for deterministic causal provenance and replay/audit anchoring.

### 72.8 EvidenceAcquisitionPlanner

The planner selects observations that are read-only by construction: file and symbol reads, impact-graph queries, index lookups, log reads, and non-mutating diagnostics. A candidate observation that would mutate project state, install a dependency, or write to a device is not acquirable during deliberation and must be proposed as an ordinary action through §71.7 authorization instead.

Execution-resource estimates (`ResourceExecutionProfile`) come from the ResourceProfiler of §69, so the planner prefers a light decisive observation over a heavy one and reports `unprofiled` rather than guessing.

### 72.9 Persistence and continuation

Deliberation records and sessions are stored in the SQLite execution ledger keyed by task and project, and are therefore replayable by the trajectory engine of §58 and inspectable by the debugger of §67.

DeliberationContinuationManager checkpoints session state on every pass boundary. The context assembler of §59.6 must treat active hypotheses, rejected strategies, the effort grant, and any pending evidence-acquisition trigger as constraint-class content under §53.3, which makes them ineligible for eviction during compaction. A compaction that drops them is detectable by comparing session revision against the post-compaction context manifest, and is reported as a defect rather than tolerated.

### 72.10 Failure modes and recovery

| Failure | Runtime behavior |
|---|---|
| Agent requests an effort level above policy | Downgraded to highest permitted; grant reason recorded |
| Pass acquires no new observation | EvidenceAcquisitionTrigger raised; the next pass acquires evidence or changes approach, never reasons again over the same observation set |
| Strategy repeated against unchanged evidence | RepeatedFailureDetector raises StrategyChangeRequired; strategy change, evidence acquisition, delegation, branch, or escalation forced |
| Physical resource pressure during deliberation | ResourceIntegrityAuthority queues, reschedules, reduces concurrency, or checkpoints the pass per BS §72; deliberation resumes from the checkpoint; no deliberation outcome is produced by pressure |
| Diminishing returns detected | Approach change forced; a further plain pass is refused |
| All hypotheses refuted | Mark NO_PROGRESS for the current deliberation cycle; acquire new evidence, transform strategy, escalate, or branch |
| Critic finds a counterexample | Strategy rejected; return to STRATEGIZE with the finding as a constraint |
| Escalated model unavailable | Continue at the available model and record the capability gap |
| Compaction drops session state | Restore from the last pass checkpoint; report the compaction defect |
| Provider fails mid-session | Resume the deliberation from the last checkpoint. Revalidate the replacement provider's reasoning capability before issuing the next pass. Preserve the runtime effort requirement and effort grant; if the replacement provider cannot satisfy the required effort level, either route to another approved provider/model or terminate with a typed capability gap. A provider failover must never silently reduce required effort. |
| Record fails admissibility | Rejected at write; deliberation cannot report sufficiency |

No failure mode permits presenting an unvalidated leading strategy as sufficient.

### 72.11 Architecture tests

The runtime is correct only when an agent request for EXHAUSTIVE under a policy ceiling of EXTENDED is granted EXTENDED with the constraint recorded; when a deliberation that has consumed arbitrarily many tokens, requests, reasoning passes, and hours continues while progress remains possible and no usage-exhaustion outcome exists in the ledger; when an observation-free pass raises an evidence-acquisition trigger and the following pass acquires evidence or changes approach; when a strategy retried against unchanged evidence raises StrategyChangeRequired; when physical memory pressure injected mid-deliberation causes the pass to be checkpointed and resumed rather than terminated; when a high-risk change is refused sufficiency with a stated confidence of 0.95 and a missing regression plan; when a discriminating test refutes the leading hypothesis and the selected strategy changes as a result; when a counterexample finding returns the cycle to strategy selection without mutating the project; when an escalated model executes under the identical permission ceiling; when a forced context compaction preserves active hypotheses and rejected strategies and the session resumes without re-deriving them; when consecutive passes of flat uncertainty reaching the **configured** `diminishingReturnThreshold` produce NO_PROGRESS and an approach change rather than a further plain pass; when the ledger shows zero project mutation events between deliberation entry and the kernel `AUTHORIZE` grant; when an effort escalation carries a `grantDecisionReason` citing the observed condition that triggered it; and when no deliberation record in the ledger contains verbatim model reasoning.

Implements the configured-threshold rule of build spec §68.13: the threshold is configuration, not a runtime constant, and no component may hardcode a pass count for `NO_PROGRESS` — the classification is a function of the configured threshold, the measured per-pass movement, and consecutive-pass semantics. A test fixture supplies its own threshold value, and a runtime that behaves identically regardless of the configured value has not implemented the detector.

## 73. IntentSynthesisPromptContract and Truthful Preview Architecture

**Implements:** build spec §69 and `CONTRACT.RUNTIME.PROMPT_CONTRACT` (the build spec section is the authority)

### 73.1 Prompt contract boundary

Any coordinator, worker, skill, deliberation, or review prompt used for Android
construction MUST conform to the IntentSynthesisPromptContract. The concrete
prompt definitions remain owned by their respective prompt-class owners; this
section defines the common contract boundary and does not imply that prompt classes are fully specified here; build spec §80.8 explicitly records the worker, skill, deliberation, and review classes as not templated and owner-pending, and the coordinator class as derived through build spec §80.8.1.

The user's conversation message is not itself the provider/model prompt. The runtime normalizes it into requirements and constructs an internal model instruction from current state, context, evidence, policy constraints, and the role contract. No user-facing prompt-template entity is created. Internal model instructions are versioned and auditable by identity and hash; the user request is preserved as task provenance and is never the assembled provider instruction.

The prompt contract requires the model to extract product intent, screens, navigation, behavior, data, integrations, device capabilities, accessibility, branding, privacy, and release requirements; distinguish user facts from assumptions; propose an Android technology plan; identify uncertainty; propose a bounded next action; and name the evidence required to evaluate that action.

The prompt contract explicitly forbids a model from claiming that predicted, simulated, requested, or proposed work was executed; treating an internal bootstrap as a product template; selecting a non-Android generated target; authorizing tools or permissions; mutating files outside a transaction; or marking requirements, previews, tests, or artifacts complete.

Prompt output is accepted only as a schema-validated proposal. The execution path is:

```text
Prompt builder
    → provider/model
    → proposal parser and schema validator
    → policy and capability evaluation
    → ConstructionTransaction / ToolBroker
    → supervised observation
    → EvidenceAuthority
    → AgentLoopReducer (proposed transition)
    → LifecycleAuthority / SessionReducer (committed transition)
```

A prompt, model response, reasoning stream, or worker handoff cannot bypass this sequence.

### 73.2 No-template enforcement

The `AndroidTechnologyResolver` receives requirements and evidence, not a template identifier. The runtime rejects any proposal containing a user-facing template selection, a framework-selection requirement, an app-archetype dependency, or a non-Android target. Internal bootstraps are represented as implementation adapters with no user-visible catalog identity and no authority to constrain the contract.

The machine-checked project invariant remains:

```text
Project.targetPlatforms == ["android"]
Project.generatedOutputs ⊆ {APK, AAB, Android source project}
Project.deploymentArtifacts ⊆ {APK} ∪ {AAB when PackagingProfile explicitly requires AAB}
```

`generatedOutputs` includes source representation and internal build artifacts; it is not synonymous with deployment delivery. A ZIP, Git bundle, or Android source project remains user-owned source/workspace access and cannot satisfy an APK delivery requirement. The resolver may select Kotlin, Java, Compose, Views, NDK/CMake native modules, or a mixed native architecture only as an implementation consequence of the user’s intent, environment capabilities, and validation evidence (ADR-257).

**Closed-world Android Jetpack architectural decision matrix.** `AndroidTechnologyResolver` lowers functional requirement classifications to standard modern Android Jetpack components deterministically. The runtime rejects proposals attempting to introduce non-standard or deprecated architectures:

| Architectural Concern | Requirement Classification | Resolved Modern Android Technology | Rationale and Constraint |
|---|---|---|---|
| UI Presentation (Default) | Modern reactive UI, dynamic animations, modern design system | Jetpack Compose (Material 3, `androidx.compose.material3`) | Standard default for all new Android UI. Eliminates XML view boilerplate and lifecycle binding bugs. |
| UI Presentation (Specialized) | Custom legacy SDKs (embedded maps, custom surface rendering, legacy widgets) | Jetpack Compose with view interop or Android XML Views + ViewBinding | Selected ONLY when required library lacks native Compose bindings. Never use `findViewById`. |
| Local Relational Storage | Structured records, multi-entity relationships, relational queries, migrations | Room Database (`androidx.room:room-runtime` + KSP) | SQLite ORM with compile-time SQL verification, Kotlin Coroutines Flow observable queries, automated migrations. |
| Local Key-Value / State | User preferences, app configuration, simple auth tokens, toggles | Jetpack DataStore Preferences (`androidx.datastore:datastore-preferences`) | Asynchronous, transactional replacement for SharedPreferences; prevents main-thread disk I/O freezes. |
| Local Typed Object Storage | Complex non-relational serialized state, configuration schemas | Jetpack DataStore Proto with `kotlinx.serialization` | Type-safe structured storage without SQLite overhead. |
| Asynchrony and State | Reactive state propagation, UI state modeling, stream processing | Kotlin Coroutines + `StateFlow` / `SharedFlow` | Standard async model. Tied to `lifecycleScope` and `repeatOnLifecycle`. Strict prohibition of `RxJava` unless pre-existing. |
| Deferred / Periodic Background | Database sync, asset prefetching, scheduled maintenance, periodic telemetry | AndroidX WorkManager (`androidx.work:work-runtime-ktx`) | Guaranteed execution surviving process death and device reboots; respects battery/network constraints. |
| Continuous Active Background | Audio playback, ongoing turn-by-turn navigation, active workout recording | Foreground Service with `ServiceCompat.startForeground` | Mandatory user-visible persistent notification; explicit `android:foregroundServiceType` attribute (Android 14+ mandate). |
| Exact Time Alarms | Clock alarm, calendar reminder, medication alert at exact clock time | Android alarm scheduler with exact alarms (`setExactAndAllowWhileIdle`) | Requires handling `SCHEDULE_EXACT_ALARM` permission and system power-saver doze-mode resilience. |
| Networking and HTTP | REST APIs, JSON data fetching, multipart upload | Retrofit 2 + OkHttp 4 + `kotlinx.serialization` | Type-safe HTTP client with connection pooling, coroutine support, and compile-time serialization. |
| Screen Navigation | Multi-screen flow, deep links, argument passing | Jetpack Navigation Compose (2.8+) with Type-Safe Routes | Kotlin `@Serializable` objects/classes for route parameters; replaces string-based route parsing. |
| Dependency Injection | Multi-component dependency management | Single-module: Constructor injection via standard factory; Multi-module: Hilt (`com.google.dagger:hilt-android`) | Deterministic dependency graphs with compile-time validation via KSP. |

**Prohibited legacy Android anti-patterns.** `MutationBroker` and the code intelligence analyzer validate all proposed mutations against the closed-world anti-pattern table. Any match is rejected at pre-commit:

| Prohibited Pattern / API | Violation Category | Approved Replacement | Enforcement Rule |
|---|---|---|---|
| `android.os.AsyncTask` | Deprecated / Memory Leak | Kotlin Coroutines (`viewModelScope.launch`, `withContext(Dispatchers.IO)`) | AST query flags import or inheritance; rejected as `DEPRECATED_ASYNC_API`. |
| Raw `java.lang.Thread` / `android.os.Handler` for background work | Unbounded Concurrency / Leak Hazard | Kotlin Coroutines structured concurrency | AST query flags `Thread { ... }.start()` or `Handler.postDelayed`; rejected as `UNSTRUCTURED_CONCURRENCY`. |
| Unbonded background `Service` without persistent notification | Background Execution Limit (API 26+) | `WorkManager` (deferred) or `ForegroundService` (immediate user-facing) | Flagged in `AndroidManifest.xml` if `<service>` declared without foregroundType or WorkManager wrapper. |
| Direct `android.database.sqlite.SQLiteOpenHelper` | Unverified SQL / Leak Hazard | Room Database (`@Database`, `@Entity`, `@Dao`) | Raw SQLite queries without compile-time verification rejected when Room capability is declared. |
| `android.app.ProgressDialog` | Deprecated / Obstructive UI | Material 3 `CircularProgressIndicator` or `LinearProgressIndicator` in Compose | Rejected as `DEPRECATED_UI_DIALOG`. |
| Calling `findViewById(R.id...)` | Null-Safety Hazard / Deprecated | ViewBinding (`binding.viewId`) or Jetpack Compose declarative state | AST flags `findViewById`; rejected as `UNSAFE_VIEW_LOOKUP`. |
| Main-thread disk / network I/O (`NetworkOnMainThreadException`) | UI Thread Starvation / ANR | Coroutine with `withContext(Dispatchers.IO)` | IO method calls inside main dispatcher scope flagged as `MAIN_THREAD_IO_HAZARD`. |
| Missing `android:exported` on components with `<intent-filter>` | Manifest Security Failure (API 31+) | Explicit `android:exported="true"` or `android:exported="false"` on every Activity, Service, and Receiver | Manifest merger preflight fails with `MANIFEST_MISSING_EXPORTED_ATTRIBUTE`. |

### 73.3 PreviewCoordinator and revision identity

`PreviewCoordinator` is the sole service allowed to create, reload, install, promote, invalidate, or roll back a live Android preview. It consumes a `PreviewRequest` only after the source transaction has committed a project revision or a declared preview-only diagnostic operation has been authorized.

> **Schema projection:** `PreviewRequest` is defined in `nirman-schemas.md` §2.63. Owner: TA §73.3.

The resulting `PreviewRevision` is immutable and is the build spec §69.4 record, field for field (the build spec is the canonical owner):

> **Schema projection:** `PreviewRevision` is defined in `nirman-schemas.md` §1.35. Owner: BS §69.4.

### 73.4 Preview state machine

```text
NOT_REQUESTED
    ↓
REQUEST_AUTHORIZED
    ↓
BUILDING
    ↓
BUILD_OBSERVED
    ↓
INSTALLING
    ↓
INSTALL_OBSERVED
    ↓
LAUNCHING
    ↓
RUNNING_OBSERVED
    ↓
INTERACTION_OBSERVED
    ↓
VALIDATING
    ├── PROMOTED_CURRENT
    ├── FAILED_CANDIDATE
    ├── STALE
    ├── INVALIDATED
    └── RECOVERING
```

`RUNNING_OBSERVED` requires a supervised process or device observation associated with the declared project revision. `PROMOTED_CURRENT` requires the canonical `PreviewPromotionGate` defined in §73.5.1 to pass. A model claim or a successful build alone cannot produce either state.

The stages of this machine project onto the durable preview authority status (`previewAuthorityState`, `nirman-schemas.md` §1.35; build spec §69.4) as follows:

| Pipeline stage (technical architecture §73.4) | `previewAuthorityState` |
|---|---|
| `NOT_REQUESTED` | — (no preview request exists; no authority state) |
| `REQUEST_AUTHORIZED` | `REQUESTED` |
| `BUILDING`, `BUILD_OBSERVED` | `BUILDING` |
| `INSTALLING`, `INSTALL_OBSERVED` | `INSTALLING` |
| `LAUNCHING` | `LAUNCHING` |
| `RUNNING_OBSERVED`, `INTERACTION_OBSERVED`, `VALIDATING` | `OBSERVING` |
| `PROMOTED_CURRENT` | `CONNECTED` |
| `FAILED_CANDIDATE` | `BLOCKED` |
| `STALE` | `STALE` |
| `INVALIDATED` | `INVALIDATED` |
| `RECOVERING` | `LOST` |

### 73.5 Truth labels and evidence classes

All preview, execution, and validation projections carry one of `PREDICTED`, `SIMULATED`, `REQUESTED`, `OBSERVED`, `VERIFIED`, `STALE`, or `INVALIDATED`. The UI may show predicted or simulated information as a forecast, but it must label it clearly and must never render it as a running application or passed validation.

Evidence is classified separately:

| Evidence class | Produced by | Completion use |
|---|---|---|
| `PLAN_EVIDENCE` | Contract/planning services | Explains intended work; cannot prove execution |
| `PROCESS_EVIDENCE` | Process supervisor | Proves command/process observation |
| `DEVICE_EVIDENCE` | Emulator manager | Proves install, launch, interaction, or emulator state |
| `VISUAL_EVIDENCE` | Screenshot and comparison service | Proves a declared visual check |
| `TEST_EVIDENCE` | Test runner and oracle | Proves declared assertions |
| `ARTIFACT_EVIDENCE` | APK inspector | Proves artifact presence, hash, and contents |
| `PROMOTION_EVIDENCE` | EvidenceAuthority | Proves all required gates passed |

### 73.5.1 Canonical `PreviewPromotionGate`

All preview promotion decisions must evaluate one canonical gate. Individual workers, the UI, model output, and presentation reducers may report evidence, but none may promote a candidate independently.

A candidate `PreviewRevision` may become `OBSERVED` only when the exact candidate source revision, generated asset and branding fingerprint, selected toolchain lock, checkpoint, artifact fingerprint, device or emulator identity, and active emulator session are recorded, and the artifact has been installed and launched with supervised observation. Required interaction, screenshot, accessibility, visual, Logcat, crash, and runtime evidence must be current for the declared Android profile.

A candidate may become `VERIFIED` and replace the active last-known-good preview only when `PreviewPromotionGate` confirms all required evidence for the profile: source and asset identity match the checkpoint; the build and artifact hash are valid; installation and launch succeeded on the identified emulator session; required synthetic interactions and declared tests passed; required visual/accessibility and diagnostic checks passed; no invalidation, stale identity, crash, or policy condition is present; and the evidence set is durably recorded by the EvidenceAuthority. Missing, stale, mismatched, simulated, or model-authored evidence fails the gate.

The gate must return a typed result such as `PASS`, `MISSING_EVIDENCE`, `STALE_IDENTITY`, `FAILED_VALIDATION`, `POLICY_BLOCKED`, or `ENVIRONMENT_UNAVAILABLE`. A failed or incomplete candidate remains `FAILED_CANDIDATE`, `RECOVERING`, `STALE`, or `INVALIDATED`; it cannot replace last-known-good. The gate is the sole normative promotion predicate and must be used by the control plane, artifact authority, preview reducer, and release completion checks.

### 73.5.2 BlankScreenDetector

`BlankScreenDetector` validates rendered frame content and UI semantics before any candidate preview is admitted to `PreviewPromotionGate`:
1. *Frame luminescence and entropy analysis:* Evaluates captured emulator frames from `RenderTransport` for visual pathologies: solid white screens, solid black canvases, uniform background fill, or zero-entropy frames indicative of an unrendered Activity or stuck splash screen.
2. *Semantics hierarchy inspection:* Cross-references frame pixels with the active Compose semantics node tree. A frame is rejected as `BLANK_SCREEN` if the visual viewport contains zero text nodes, vector icons, or clickable interactive targets.
3. *Promotion failure:* Emits a `BLANK_SCREEN_OBSERVED` failure diagnostic, blocking candidate promotion to `OBSERVED` or `VERIFIED` and triggering the Android runtime sub-ladder (BS §42.4) to reload or relaunch the application.

### 73.6 Stepwise preview projection

The UI projection groups real events into understandable stages without fabricating execution:

```text
INTENT_ACCEPTED
  → CONTRACT_VALIDATED
  → PLAN_RECORDED
  → CHECKPOINT_CREATED
  → SOURCE_REVISION_COMMITTED
  → BUILD_OBSERVED
  → INSTALL_OBSERVED
  → LAUNCH_OBSERVED
  → INTERACTION_OBSERVED
  → VALIDATION_OBSERVED
  → PREVIEW_PROMOTED
```

A stage is marked complete only when its declared evidence exists and is current. While work is pending, the projection uses `PLANNED`, `QUEUED`, `RUNNING`, `WAITING`, `RECOVERING`, `FAILED`, or `BLOCKED`; none of these statuses is converted into `VERIFIED` by the presentation layer.

### 73.7 Last-known-good protection

Before a candidate preview is installed or promoted, the coordinator stores the active last-known-good `PreviewRevision`, checkpoint, artifact fingerprint, emulator identity, and evidence set. A candidate failure cannot overwrite or delete this record. Repair and rollback invalidate candidate evidence by reason and preserve the known-good evidence.

When the active project revision changes, the coordinator calculates compatibility. If source, asset, toolchain, device, contract, or artifact identity no longer matches, the previous preview becomes `STALE` rather than silently representing the new source. The preview panel must show both the stale/failed candidate and the available last-known-good revision until a new candidate is observed and promoted.

### 73.8 UI projection and reconnect

The preview panel is a read model of durable control-plane events. It never infers execution from model text, terminal color, file timestamps, or a heartbeat alone. It subscribes by project and task, records the last acknowledged event sequence, and reconstructs the same preview projection after reconnect, UI restart, sleep/resume, or supervisor restart. The implementation is governed by build spec §71 and technical architecture §75; `PreviewSyncEvent` is normalized before `PreviewProjectionReducer` applies it.

If the event stream is unavailable, the panel shows the last durable state with a stale-stream indicator. It does not advance the preview, progress stage, or evidence status locally. A reconnect replays missing events and recomputes the projection through the same reducer.

### 73.9 Architecture tests

The preview architecture must pass tests proving that:

1. A predicted or simulated preview cannot become current.
2. A successful build without launch observation cannot become `RUNNING_OBSERVED`.
3. A stale revision cannot satisfy a current completion gate.
4. A failed candidate preserves the last-known-good preview.
5. A UI disconnect does not stop execution or change preview truth.
6. Duplicate and out-of-order events reconstruct one deterministic projection.
7. Rollback invalidates affected evidence and restores the correct preview identity.
8. A template-selection proposal and a non-Android target proposal are rejected before mutation.
9. The final APK evidence refers to the same source, asset, and preview revisions.
10. The panel never labels a model statement as process, device, test, or artifact evidence.

### 73.10 Android technology adapter contract

The §73.2 `AndroidTechnologyResolver` selects Kotlin, Java, Compose, Views, NDK/CMake native modules, or a mixed native architecture only as an implementation consequence of the user's intent, environment capabilities, and validation evidence (ADR-257). Every resulting `AndroidTechnologyPlan` MUST resolve to exactly one registered `AndroidTechnologyAdapter` implementation.

`AndroidTechnologyAdapter` is a strategy/composition resolver and does not execute concrete preview, build, artifact, device, runtime, observation, validation, or failure-classification operations. `AndroidBuildAdapter` and `AndroidDeviceAdapter` are the sole concrete execution surfaces. Each concrete preview operation has exactly one execution authority: `AndroidBuildAdapter` for build and artifact operations, or `AndroidDeviceAdapter` for device and runtime operations. The technology adapter resolves those authorities but never executes their concrete operations. The technology adapter is not a second execution surface and is not a second authority.

> **Schema projection:** `AndroidTechnologyAdapter` is defined in `nirman-schemas.md` §2.64. Owner: TA §73.10.

> **Schema projection:** `AndroidTechnologyAdapterResolution` is defined in `nirman-schemas.md` §2.65. Owner: TA §73.10.

The adapter MUST expose only the following operations. None of these operations perform concrete build, install, launch, observation, screenshot, UI hierarchy, Logcat, or validation work. Concrete operations are dispatched exclusively through the resolved `AndroidBuildAdapter` or `AndroidDeviceAdapter` returned by the resolution operations.

```text
AndroidTechnologyAdapter operations
- validatePlan()             -> AndroidTechnologyAdapterResolution
- initializeProject()        -> AndroidTechnologyAdapterResolution
- planBuild()                -> AndroidTechnologyAdapterResolution
- classifyFailure()          -> AndroidTechnologyAdapterResolution
- resolveBuildAdapter()      -> AndroidBuildAdapter identity (deterministic,
                                derived from the locked AndroidTechnologyPlan,
                                AndroidToolchainLock, and AndroidDeviceCapabilities;
                                selection is auditable and recorded in the
                                PreviewRequest decision trace)
- resolveDeviceAdapter()     -> AndroidDeviceAdapter identity (deterministic,
                                derived from the locked AndroidTechnologyPlan,
                                AndroidDeviceCapabilities, and active device
                                session; selection is auditable and recorded in
                                the PreviewRequest decision trace)
```

`resolveBuildAdapter()` and `resolveDeviceAdapter()` MUST resolve from the locked `AndroidTechnologyPlan`, `AndroidToolchainLock`, and `AndroidDeviceCapabilities` state, not from mutable runtime state. The selected identities are returned as registered adapter identities; selection itself remains deterministic and auditable. A change to the locked plan, toolchain, or device capabilities invalidates prior resolution results; `PreviewCoordinator` MUST re-resolve before dispatching any concrete operation. Resolution results do not constitute a second mutable authority; the registered `AndroidBuildAdapter` and `AndroidDeviceAdapter` returned by resolution remain the sole execution authorities for their respective operations.

The internal implementation families registered at M108 are execution strategies, not user-facing framework choices. The §73.2 no-template rule remains binding; the resolver never surfaces these family names to the user as a framework picker. Generated applications are native Android only (ADR-257).

```text
NativeAndroidAdapter (internal implementation family)
- composition: Kotlin or Java, Views or Compose, Gradle
- adapterId prefix: nirman.adapter.native
- operations: validatePlan, initializeProject, planBuild, classifyFailure,
  resolveBuildAdapter (Gradle native), resolveDeviceAdapter
  (AndroidDeviceAdapter for Nirman-managed local emulator session)

MixedAndroidAdapter (internal implementation family)
- composition: mixed Kotlin and Java, native modules using NDK or CMake
  when selected, Android platform APIs and services
- adapterId prefix: nirman.adapter.mixed
- operations: validatePlan, initializeProject, planBuild, classifyFailure,
  resolveBuildAdapter (composed Gradle native plus NDK or CMake),
  resolveDeviceAdapter (AndroidDeviceAdapter for the Nirman-managed
  local Android emulator)
```

The `AndroidTechnologyAdapter` registry is part of the toolchain lock surface. A revision, toolchain update, environment fingerprint change, or compatibility-rule change invalidates dependent resolution results and completion claims; the adapter registry entry, `adapterVersion`, and `technologyPlanHash` together identify a reproducible selection context. The `PreviewSyncEvent` payload defined in build spec §71.1 carries `adapterId`, `adapterVersion`, `technologyPlanHash`, and the resolved `buildAdapterIdentity` and `deviceAdapterIdentity` as required event fields when the event is emitted by an adapter-mediated operation; the §71 `PreviewSyncEvidenceRecord` carries the same fields per observation. This extends §71.1; it does not redefine the `PreviewSyncEvent` schema.

### 73.11 Deterministic preview-mode resolver

The preview mode is selected by a deterministic resolver over a recorded input set. The resolver is a pure function of the recorded input and the canonical rule table; it is not a model decision.

> **Schema projection:** `PreviewModeResolverInput` is defined in `nirman-schemas.md` §2.66. Owner: TA §73.11.

> **Schema projection:** `PreviewModeResolverOutput` is defined in `nirman-schemas.md` §2.67. Owner: TA §73.11.

The mode values `COMPOSE_RELOAD`, `INCREMENTAL_APK_INSTALL`, `FULL_APK_REINSTALL`, `HEADLESS_SMOKE`, `DIAGNOSTIC_SOURCE_ONLY`, `USER_REQUIRED`, and `BLOCKED` are the `PreviewRevision.previewMode` enumeration declared on the field in §73.3 and build spec §69.4. `CONSERVATIVE_FULL_REINSTALL` is part of that enumeration as a refinement of `FULL_APK_REINSTALL`: it is a full reinstall selected specifically because the impact information was insufficient to prove a faster safe path, not because a faster safe path was proven unsafe. Its presence makes the resolver's "unknown" outcome distinguishable from a "known unsafe" outcome and is recorded as part of the `PreviewRequest` decision trace.

Canonical predicates (typed, evidence-bound, not free-form):

```text
sameNativeIdentity:
  derived from the canonical native identity fingerprint, which combines:
  - applicationId (from AndroidManifest)
  - ABI list
  - signing identity (signing config fingerprint)
  - native dependency or module identity (CMake or native-module manifest hash)
  - relevant manifest and resource identity (manifest fingerprint, resource hash)
  - technology-plan and native build identity (AndroidTechnologyPlanHash plus
    AndroidBuildObservation.artifactFingerprints)
  Two native identity evaluations are sameNativeIdentity when their fingerprints
  are equal under the canonical comparison defined by the Android toolchain lock
  authority. Any mismatch in the components above yields sameNativeIdentity = false.
```

Canonical rule table (applied in order; first match wins):

```text
1. Build unavailable for the recorded toolchain lock, source revision,
   or environment fingerprint
   -> DIAGNOSTIC_SOURCE_ONLY
2. No usable device or runtime session for the declared profile
   -> HEADLESS_SMOKE or USER_REQUIRED (USER_REQUIRED when no
     replacement device can satisfy the profile)
3. Known unsafe or incompatible state detected: applicationId, ABI,
   signing identity, or runtime identity changed, or a recorded
   compatibility rule yields INCOMPATIBLE
   -> FULL_APK_REINSTALL
4. Native code, resource, manifest, dependency, or native-module change
   detected in impactGraphRevision
   -> INCREMENTAL_APK_INSTALL or FULL_APK_REINSTALL (FULL_APK_REINSTALL
     when the change touches applicationId, signing, ABI split, or
     native-module ABI; CONSERVATIVE_FULL_REINSTALL when rule 3 does
     not apply but the impact graph cannot localize the change to a
     reload-safe surface)
5. Compose-only compatible change detected, sameNativeIdentity holds,
   and a compatible runtime session is available
   -> COMPOSE_RELOAD
6a. Known unsafe state detected (signing mismatch, ABI mismatch,
    manifest version conflict, runtime fault unacknowledged, or
    compatibility-rule denial) but a clean rebuild is permitted
    -> FULL_APK_REINSTALL
6b. Insufficient impact information to prove a reload-safe surface;
    no rule above fired
    -> CONSERVATIVE_FULL_REINSTALL
7. Recognized incompatibility with no permitted rebuild path
    -> BLOCKED
```

The "unknown" outcome and the "known unsafe" outcome are explicitly distinct: rule 6a is recorded with reason `KNOWN_UNSAFE`; rule 6b is recorded with reason `INSUFFICIENT_IMPACT_INFORMATION`. The resolver MUST distinguish them in the `decisionReason` field so that the `PreviewRequest` decision trace and downstream repair logic do not conflate them.

A resolver output is recorded as part of the `PreviewRequest` decision trace. The mode returned is one of the `PreviewRevision.previewMode` values enumerated on the field in §73.3 and build spec §69.4; introducing new mode identifiers requires a versioned contract update through ADR-195. The resolver MUST NOT mutate authoritative state; it returns a decision, and `PreviewCoordinator` owns the resulting lifecycle transition.

### 73.12 Android device adapter contract

The device layer used by `PreviewCoordinator` for install, launch, interaction, screenshot, UI hierarchy, Logcat, crash, and permission observation is bound to a canonical `AndroidDeviceAdapter` interface. Every Nirman-managed local Android emulator implementation MUST satisfy this interface; the interface is an execution contract, not an authority. Clock, notification, and instrumentation execution are adapter operations like any other (SCHEMAS §2.68): virtual time advances only through `advanceClock` from a seeded basis, and no host wall-clock read is device evidence.

> **Schema projection:** `AndroidDeviceAdapter` is defined in `nirman-schemas.md` §2.68. Owner: TA §73.12.

System surfaces are the adapter's job, not a worker's and not a human's (ADR-225). When a captured `ScreenModel` (§74.2) has a `windowKind` other than `APP`, the adapter applies the matching `SystemDialogRule` of the session's `DeviceHygienePolicy` before the interaction result is returned: a runtime-permission dialog is answered as the running scenario's permission path declares (grant or deny — build spec §56.3 requires both), an application-crash or ANR dialog is captured as crash evidence through `collectCrash()` and dismissed, a keyguard or setup-wizard surface is a hygiene failure that invalidates the golden snapshot, and an external-intent chooser is cancelled and recorded as an `EXTERNAL_INTENT` edge. Every handled dialog is an observation in the evidence chain with its rule identity; a dialog no rule matches is a `failureClassification`, never a `USER_REQUIRED` decision, because the emulator holds no state a human must supply. The adapter classifies a `SYSTEM_DIALOG` surface to its `dialogKind` from the `ScreenModel` package, activity, and element signatures before rule matching. Rule selection is fixed: `RUNTIME_PERMISSION` answers per scenario (`ANSWER_PER_SCENARIO`); `APP_CRASH` and `ANR` capture and dismiss (`CAPTURE_AND_DISMISS`); `SYSTEM_UPDATE` dismisses (`DISMISS`) — system updates are never validation on a pinned image; `EXTERNAL_INTENT_CHOOSER` cancels (`CANCEL`). Keyguard and setup-wizard surfaces are the hygiene-failure outcome outside the four handlings: either keyguard signal — `windowKind` `KEYGUARD` or `dialogKind` `KEYGUARD` — invalidates the golden snapshot, and the setup wizard arrives as `SYSTEM_DIALOG` with `dialogKind` `SETUP_WIZARD` and has no dedicated `windowKind` by design. A `LAUNCHER` surface mid-scenario is never legitimate: the application left the foreground, so the adapter relaunches it under rung (c) of the build spec §28.2 sub-ladder — entered as the launch family, the attempt accounted to `RecoveryAuthority` — recording the deviation as a `LAUNCHER_EXIT` edge and the relaunch transition as `NAVIGATED`. `INPUT_METHOD` is expected-transient, not a handled surface: the keyboard is part of the application interaction, opens on editable focus, and closes on back-press, and its subtree is included in the `screenFingerprint` deterministically; an IME open with no editable focused that survives a back-press is a `failureClassification`. An `APP` surface whose `packageId` is not the scenario's under-test package is recorded as an `EXTERNAL_INTENT` edge and never driven; the adapter backs out or relaunches. Any other unmatched surface is a `failureClassification` like an unmatched dialog. Notification shade, recents, and status-bar surfaces are out of scope: no interaction input reaches them, delivery is proven through `collectNotifications`, and no build spec §56.3 class requires them.

Every operation returns a typed observation that carries `adapterId`, `adapterVersion`, `deviceId`, `deviceSessionId`, `runtimeSessionId`, `environmentFingerprint`, `applicationStateFingerprint`, `evidenceReferences`, `failureClassification`, and `invalidationDependencies`. Operations do not write `PreviewProjection`, evidence identity, artifact promotion, or completion state; those remain with the existing specialized authorities. A revision, toolchain update, environment fingerprint change, emulator identity change, or capability revocation invalidates dependent observations and completion claims unless the dependency graph proves independence.

### 73.13 Android build adapter contract

Build execution is bound to a canonical `AndroidBuildAdapter` interface. The interface is an execution contract, not an authority; it does not authorize builds, and it does not promote artifacts.

> **Schema projection:** `AndroidBuildAdapter` is defined in `nirman-schemas.md` §2.69. Owner: TA §73.13.

> **Schema projection:** `AndroidBuildObservation` is defined in `nirman-schemas.md` §2.70. Owner: TA §73.13.

The same interface MUST cover: Gradle native Kotlin/Java; Android Views; Jetpack Compose; mixed Kotlin/Java and Views/Compose; NDK or CMake native modules; Android Gradle plugins and platform services (ADR-257). `AndroidBuildAdapter` is invoked by `PreviewCoordinator` through the `AndroidTechnologyAdapter` selected for the `AndroidTechnologyPlan`; it does not create a separate build authority, and it does not bypass `ToolchainAuthority` or `ArtifactAuthority`. A revision, toolchain update, environment fingerprint change, or adapter version change invalidates dependent observations and completion claims.

### 73.14 Rendering principle and UI pipeline

Nirman does not render Android frameworks itself. The desktop preview panel is a projection of the real Android runtime produced by the concrete execution authorities resolved through the registered technology adapter; it is never a second renderer.

```text
Rendering authority
Android source
  → selected AndroidTechnologyAdapter (per AndroidTechnologyPlan)
  → resolved AndroidBuildAdapter (via resolveBuildAdapter)
  → resolved AndroidDeviceAdapter (via resolveDeviceAdapter)
  → native build or runtime toolchain (AndroidBuildAdapter)
  → APK or runtime process (AndroidDeviceAdapter)
  → Nirman-managed local Android emulator (AndroidDeviceAdapter)
  → AndroidBuildObservation and device observation
  → PreviewSyncEvent (carries adapterId, adapterVersion,
    technologyPlanHash, buildAdapterIdentity, deviceAdapterIdentity)
  → PreviewProjectionReducer
  → desktop preview projection
```

The legal pipeline from the UI to a preview operation is:

```text
UI
  → typed Preview command (per CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE)
  → PreviewCoordinator
  → AndroidTechnologyAdapter.validatePlan | planBuild | classifyFailure
  → AndroidTechnologyAdapter.resolveBuildAdapter
  → AndroidTechnologyAdapter.resolveDeviceAdapter
  → AndroidBuildAdapter (concrete build and artifact operations)
  → AndroidDeviceAdapter (concrete install, launch, observation,
    screenshot, UI hierarchy, Logcat, validation, failure-classification)
  → AndroidBuildObservation and AndroidDeviceAdapterObservation
  → PreviewSyncEvent
  → PreviewProjectionReducer
  → PreviewPanel
```

The technology adapter resolves the execution authorities; it does not execute their concrete operations itself. Concrete build, install, launch, observation, screenshot, UI hierarchy, Logcat, validation, and failure-classification operations have exactly one execution surface each: `AndroidBuildAdapter` for build and artifact operations, `AndroidDeviceAdapter` for device and runtime operations.

The following paths are forbidden and MUST be rejected by the typed command registry and the contract-graph verifier. The registry rejects them structurally: no `commandKind` in build spec §76.1 is registered in an `adb.`, `gradle.`, `metro.`, `expo.`, or `emulator.` namespace, and the `preview.*` command kinds dispatch only to `PreviewCoordinator`. The verifier rejects them as a build spec §67.11 semantic-documentation defect whenever a §76.1 registry row appears in one of those namespaces or the §73.10 `AndroidTechnologyAdapter operations` block lists anything other than its six resolution operations:

```text
UI → ADB
UI → Gradle
UI → Metro or Expo
UI → emulator
UI → AndroidTechnologyAdapter.executeBuild | install | launch | reload |
    observeRuntime | captureScreenshot | captureUiHierarchy |
    collectLogcat
```

The §73.8 rule that the preview panel is a read model of durable control-plane events is preserved; the technology adapter and the build and device adapters do not change the panel authority, they only supply observations through the existing `PreviewSyncEvent` and `PreviewSyncEvidenceRecord` flow.

### Cross-service intelligence output contract

All Android intelligence services and analytical modules are **typed read-only producers**, not authorities. An intelligence result MUST be bound to the applicable `projectId`, `projectRevision`, `planRevision` where relevant, `contextIntegrityHash`, input/source fingerprint, execution/observation identity, and evidence references. Each result MUST declare one semantic class: `OBSERVATION`, `ADVISORY_FINDING`, `VALIDATION_INPUT`, or `GATE_INPUT`. `GATE_INPUT` means an existing canonical authority may consume the result; it does not grant decision rights to the producer. Results become stale when their bound source revision, execution epoch, or relevant evidence watermark changes. Existing finding, observation, validation, or evidence schemas MUST be reused or extended; intelligence components MUST NOT invent an unbound parallel authority or completion record. Mutations remain exclusively under `MutationBroker` → `ConstructionTransactionManager`.

### 73.15 AndroidProductIntelligenceService

`AndroidProductIntelligenceService` is the supervisor-owned, read-only aggregate query facade that unifies requirement elicitation, specification formalization, spec-to-build traceability, and offline Android domain knowledge. It exposes a typed query interface to the kernel agent (`Requirements Planner`, `Primary Orchestrator`) and registered desktop IPC command handlers.

`AndroidProductIntelligenceService` creates no second authority. It does not directly mutate the `AndroidConstructionContract` or project source code; all mutation proposals route through `MutationBroker` (BS §43.2) and commit via `ConstructionTransaction` (BS §42.2). It does not maintain or expose a user-facing template catalog (`CLAUSE.PROMPT_CONTRACT.NO_TEMPLATE_CATALOG`). It has no external cloud scraper or competitor crawler dependencies and consumes zero AI token/duration budgets (ADR-218, BS §72).

#### 73.15.1 ImplicitRequirementMiner

`ImplicitRequirementMiner` deterministically expands high-level user requests and explicit feature declarations into mandatory companion requirements:
1. *Authentication companion expansion:* Expands login requirements to include password reset, secure session logout, token expiry handling, and account deletion pathways (Google Play account deletion mandate).
2. *Data collection and listing expansion:* Expands persistent entity lists to require empty-state UI layouts, swipe-to-refresh (`PullToRefreshBox`), loading shimmer/skeleton states, item deletion confirmations, and error-state fallbacks.
3. *Transactional interaction expansion:* Expands checkout, booking, or financial operations to include transaction confirmation modals, receipt display, offline idempotency guards, and network timeout recovery.
4. *Contract injection:* Proposes `ConstructionRequirement` candidates with `originKind = INFERRED`, explicit `sourceFeatureIds`, source message/evidence provenance, and `parentRequirementIds`/`derivationKinds` for inferred-companion, split, or merge derivation. `ImplicitRequirementMiner` cannot write `ConstraintRegistry` or `AndroidConstructionContract` directly: only admitted canonical requirement IDs are projected into the matching contract revision.

#### 73.15.2 RequirementConflictDetector

`RequirementConflictDetector` performs pre-construction semantic and architectural contradiction detection across proposed requirements before any transaction opens:
1. *Architectural contradictions:* Detects conflicting architectural choices (e.g. offline-only requirement paired with real-time cloud collaboration; conflicting database persistence choices; conflicting state management paradigms).
2. *Navigation and UI contradictions:* Identifies incompatible screen navigation models (e.g. bottom navigation bar combined with fullscreen modal-only workflows; circular back-stack navigation dependencies).
3. *Permission and capability contradictions:* Flags conflicting Android capabilities (e.g. background location tracking declared without background service entitlement; exact alarm usage without `SCHEDULE_EXACT_ALARM` justification).
4. *Conflict resolution guidance:* Emits structured contradiction diagnostics directly to `Requirements Planner`, triggering targeted clarification under BS §69.11 before code generation begins.

#### 73.15.3 RequirementTestabilityScorer

`RequirementTestabilityScorer` statically evaluates whether a synthesized requirement has deterministically observable post-conditions on Android:
1. *Observability scoring:* Computes a testability score based on whether requirement acceptance criteria bind to observable UI elements (Compose semantics nodes), inspectable persistent storage (Room entity queries), interceptable local network doubles, or mockable system hardware sensors.
2. *Ambiguity and unobservable assertion detection:* Flags vague requirements lacking concrete post-conditions (e.g. "app should feel snappy", "clean interface") and requires reformulation into verifiable metrics (e.g. frame render duration, text contrast ratio).
3. *Pre-commit verification:* Complements `PlanCompletenessValidator` (BS §69.11) by ensuring only testable requirements advance to `AndroidConstructionContract` commitment.

#### 73.15.4 PersonaInferenceEngine

`PersonaInferenceEngine` infers target stakeholder personas and ergonomic profiles from user intent and product descriptions:
1. *Ergonomic profile derivation:* Determines primary usage contexts (e.g. on-the-go one-handed usage for transit apps requiring bottom-anchored controls and minimum 48dp touch targets; seated dual-hand usage for productivity apps).
2. *Accessibility and visual profile:* Infers required contrast baselines, Dynamic Type font scaling ranges, and screen reader TalkBack content description priorities based on the intended audience (e.g. high-contrast, large-type defaults for senior/medical apps).
3. *UX complexity calibration:* Recommends appropriate progressive disclosure levels (streamlined wizard flows for consumer apps vs dense data dashboards for technical utilities) to guide UI component composition without templates.

#### 73.15.5 AndroidDomainKnowledgeCatalog

`AndroidDomainKnowledgeCatalog` provides a local, offline repository of idiomatic Android architecture patterns, entity models, and state-machine workflows:
1. *Domain entity models:* Maintains standard relational Room entity schemas and relationships for common application domains (E-Commerce: Cart, Product, Order; Fitness: Workout, Exercise, Set; Productivity: Task, Project, Tag) to assist `Requirements Planner` in generating normalized schemas without user-facing templates.
2. *Domain state-machine workflows:* Enforces standard Android lifecycle state transitions (e.g. Onboarding to Authentication to Dashboard; Cart to Checkout to PaymentConfirmation; Playback to Pause to Buffering) for comprehensive `ScenarioSynthesizer` (§62.1) coverage.
3. *Zero-template compliance:* Operates strictly as internal reference heuristics for requirement and schema formulation; never exposes a template picker or pre-built skeleton app to the user (`CLAUSE.PROMPT_CONTRACT.NO_TEMPLATE_CATALOG`).

#### 73.15.6 RegulatoryComplianceAnalyzer

`RegulatoryComplianceAnalyzer` evaluates declared permissions, target API levels, and data collection models against Google Play policies and regional privacy regulations:
1. *Google Play policy verification:* Audits requirements against Google Play policies (Families Policy requirements for children's apps, Prominent Disclosure mandates for background location and health data, Account Deletion URL/in-app requirements).
2. *Data Safety Section declarations:* Generates accurate Data Safety declarations (data collected, shared, encrypted in transit, ephemeral vs persistent) based on the project's declared entities and network endpoints.
3. *Regional compliance heuristics:* Identifies regulatory constraints (COPPA, GDPR, CCPA/CPRA, India DPDP Act) requiring in-app consent dialogs, privacy policy links, or local data encryption before artifact release.

### 73.16 AndroidDesignIntelligenceService

**Role:** aggregate query facade and static UI/UX and accessibility validation — read-only services; no authority, no AI-usage budget.

#### 73.16.1 AndroidDesignIntelligenceService

`AndroidDesignIntelligenceService` is the supervisor-owned, read-only aggregate query facade unifying design token compliance, visual hierarchy and layout QA, mobile accessibility auditing, string externalization, and dark pattern prevention across the generated Android application. It exposes a typed query surface to UI and visual QA workers (`UI Worker`, `Visual QA Worker`, `Content Worker`) and registered IPC command handlers.

`AndroidDesignIntelligenceService` coordinates four deterministic analytical components:
1. *Visual QA and hierarchy analysis:* Invokes `VisualHierarchyAnalyzer` (§73.16.2) to evaluate color contrast ratios, layout overflows, density bucket coverage, and design token adherence on rendered frames and Compose trees.
2. *Android accessibility auditing:* Invokes `AndroidAccessibilityAuditor` (§73.16.3) to verify minimum 48dp touch targets, TalkBack `contentDescription` semantics, focus order, and color-blind palette safety, while enforcing native Compose semantics over invalid web ARIA attributes.
3. *Localization and string externalization:* Invokes `StringExternalizationEngine` (§73.16.4) to scan for hardcoded string literals, manage `strings.xml` and `<plurals>`, verify RTL mirroring (`start`/`end`), and check localized date/number formatters.
4. *Dark pattern detection:* Invokes `DarkPatternDetector` (§73.16.5) to detect manipulative UX patterns, pre-selected consent boxes, and disguised cancellation actions.

`AndroidDesignIntelligenceService` creates no second authority. It does not directly mutate project source or bypass policy; all UI/UX proposals route through `MutationBroker` (BS §43.2) and commit via `ConstructionTransaction` (BS §42.2). `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

#### 73.16.2 VisualHierarchyAnalyzer

`VisualHierarchyAnalyzer` provides static and dynamic visual QA verification across Android layouts and rendered screens:
1. *Contrast ratio verification:* Statically evaluates text and icon foreground colors against backgrounds in light and dark color schemes, enforcing WCAG 2.1 AA contrast floors (4.5:1 for normal text, 3:1 for large text and interactive icons).
2. *Layout overflow and clipping detection:* Traverses Compose layout constraints and rendered viewport bounds to flag element clipping, unconstrained text overflows, and unintentional text truncations.
3. *Design token and density parity:* Verifies that UI composables utilize declared design tokens (`MaterialTheme.colorScheme`, `spacing`, `typography`) rather than hardcoded hex values or raw dimensions, and checks that image assets supply required density buckets (`mdpi` through `xxxhdpi`) or vector drawables.

#### 73.16.3 AndroidAccessibilityAuditor

`AndroidAccessibilityAuditor` audits mobile accessibility compliance against Android guidelines:
1. *Touch target size verification:* Enforces the Android minimum interactive touch target dimension of 48dp $\times$ 48dp across all clickable controls (`Button`, `IconButton`, `FloatingActionButton`, clickable composables).
2. *Screen reader semantics and TalkBack support:* Verifies that all non-text actionable elements declare meaningful `contentDescription` attributes via Jetpack Compose `Modifier.semantics`, rejecting non-applicable web ARIA attributes in favor of native Android AccessibilityNodeInfo properties.
3. *Focus navigation and color independence:* Checks hardware keyboard and D-pad focus traversal order (`Modifier.focusProperties`), and validates that UI state (such as error, warning, or selected) is never conveyed by color alone, maintaining visual accessibility under simulated color-vision deficiencies.

#### 73.16.4 StringExternalizationEngine

`StringExternalizationEngine` verifies and automates Android localization readiness:
1. *Hardcoded string literal detection:* Statically scans Kotlin source files and Compose composables for hardcoded UI text literals, proposing extraction into `res/values/strings.xml`.
2. *RTL layout and mirroring validator:* Verifies bidirectional layout compatibility (`android:supportsRtl="true"`), ensuring padding, margin, and alignment specifications use directional `start`/`end` properties rather than hardcoded `left`/`right`.
3. *Pluralization and locale formatting:* Audits quantity-dependent text to ensure usage of Android `<plurals>` resources rather than string concatenation, and verifies date, time, number, and currency formatting via localized Android APIs (`DateTimeFormatter`, `NumberFormat.getCurrencyInstance(locale)`).

#### 73.16.5 DarkPatternDetector

`DarkPatternDetector` statically scans UI compositions and transaction flows to prevent deceptive UX designs:
1. *Consent and opt-in neutrality:* Detects pre-checked opt-in checkboxes for marketing or tracking consent, verifying default-neutral user choice.
2. *Deceptive choice hierarchy:* Identifies unequal visual hierarchies that disguise decline, cancel, or opt-out actions through degraded contrast, obscured positioning, or tiny font sizes.
3. *Subscription and cancellation transparency:* Verifies that account deletion and subscription management pathways provide direct, unhindered navigation without hidden cancellation loops.

### 73.17 AndroidAppObservabilityService

**Role:** aggregate query facade and generated app observability scaffolding — read-only services; no authority, no AI-usage budget.

#### 73.17.1 AndroidAppObservabilityService

`AndroidAppObservabilityService` is the supervisor-owned, read-only aggregate query facade coordinating observability, diagnostic instrumentation, and telemetry scaffolding across the generated Android application. It exposes a typed query surface to engineering and QA workers (`Android Platform Worker`, `UI Worker`, `Test and QA Worker`) and registered IPC command handlers.

`AndroidAppObservabilityService` coordinates seven deterministic analytical and scaffolding components:
1. *Structured logging scaffolding:* Invokes `StructuredLoggingScaffolder` (§73.17.2) to scaffold structured Logcat wrappers or Timber integration, tag conventions, and R8/ProGuard release stripping rules.
2. *In-app crash reporting integration:* Invokes `CrashReportingScaffolder` (§73.17.3) to scaffold client-side `Thread.UncaughtExceptionHandler`, persistent crash log caching in private application storage (`filesDir/crash_reports/`), and crash dump formatting.
3. *Metrics and performance instrumentation:* Invokes `AppMetricsScaffolder` (§73.17.4) to instrument AndroidX Metrics (`androidx.metrics:metrics-performance` / JankStats) for frame rendering performance, jank tracking, and startup latency (TTID/TTFD) markers.
4. *Tracing instrumentation:* Invokes `TracingInstrumentationScaffolder` (§73.17.5) to scaffold AndroidX Tracing (`androidx.tracing:tracing-ktx`), Perfetto trace sections (`trace("section") { ... }`), and Compose recomposition markers.
5. *In-app diagnostics screen and exporter:* Invokes `InAppDiagnosticsScaffolder` (§73.17.6) to generate an in-app debug Jetpack Compose health dashboard (under debug build variants) inspecting Room database state, cache size, network latency, and WorkManager queue status, along with a ZIP/Share Intent diagnostic exporter.
6. *Analytics event schema generation:* Invokes `AnalyticsSchemaGenerator` (§73.17.7) to synthesize type-safe Kotlin sealed class event hierarchies and abstract dispatcher interfaces.
7. *Feature usage tracking:* Invokes `FeatureUsageTracker` (§73.17.8) to scaffold local feature adoption counters and user interaction frequency tracking via Jetpack DataStore.

*Prohibited cloud patterns and Android adaptation:*
Cloud server alerting rules (Prometheus alertmanager, PagerDuty) and cloud error budgets (server uptime SLOs) are strictly prohibited under Nirman's binding product invariants (BS §1.5, ADR-180, ADR-218, ADR-220: client-only Android target, no cloud/server output, no AI token/monetary/reasoning/error budgets as execution controls). In client Android applications, alert rules are adapted strictly to local in-app notification threshold checks (e.g. low storage warning, battery conservation mode), and reliability targets are enforced as crash-free session rate objectives evaluated deterministically by `AndroidQualityGate` (TA §53.4) without halting valid builds through budget eviction.

*Generated-app observability boundaries:*
1. *Local debug and validation instrumentation:* Local debug logging, Logcat output, startup latency markers, and Nirman-managed validation instrumentation may be generated as needed for development and emulator verification.
2. *Production telemetry and analytics requirements:* Production crash reporting, analytics, feature adoption tracking, or remote telemetry require explicit user intent in the product specification, a declared destination backend, explicit end-user consent mechanisms, clear retention policies, and data minimization.
3. *Nirman ingestion prohibition:* Nirman itself NEVER ingests published-app telemetry, production user analytics, or post-release crash reports.
4. *Release artifact hygiene:* Validation-only instrumentation and debug diagnostics (including `InAppDiagnosticsScaffolder` dashboards and non-production loggers) MUST NOT silently enter release artifacts. ProGuard/R8 rules and build variant source isolation strip debug instrumentation from release APK or optional AAB packages, verified by a dedicated release stripping fixture in milestone M117.

`AndroidAppObservabilityService` creates no second authority. It does not directly mutate project source or bypass policy; all observability scaffolding proposals route through `MutationBroker` (BS §43.2) and commit via `ConstructionTransaction` (BS §42.2). `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

#### 73.17.2 StructuredLoggingScaffolder

`StructuredLoggingScaffolder` scaffolds structured, performant logging infrastructure for the Android application:
1. *Logging abstraction:* Scaffolds a lightweight logging facade (such as Timber or a custom structured `AppLogger`) enforcing standard log levels (`VERBOSE`, `DEBUG`, `INFO`, `WARN`, `ERROR`), contextual tags, and structured JSON-compatible event payloads.
2. *PII masking:* Integrates with `PiiFieldClassifier` (§70.7.2) to ensure user credentials, tokens, and personal identifiable information are automatically masked or redacted before reaching Logcat streams.
3. *Release build stripping:* Synthesizes ProGuard/R8 rules (`-assumenosideeffects class android.util.Log { ... }`) to strip verbose and debug log invocations from release APK binaries, protecting intellectual property and eliminating unnecessary runtime overhead.

#### 73.17.3 CrashReportingScaffolder

`CrashReportingScaffolder` integrates client-side uncaught exception handling and local crash persistence:
1. *Uncaught exception hook:* Scaffolds a default `Thread.UncaughtExceptionHandler` registered during `Application.onCreate` to intercept fatal uncaught exceptions before process termination.
2. *Local crash persistence:* Writes normalized crash records (stack trace, active thread name, device model, Android OS version, app version code, available memory, and timestamp) to private internal storage (`filesDir/crash_reports/`).
3. *Crash dispatch adapter:* Provides an abstract crash reporter interface allowing optional forwarding to external crash logging SDKs (such as Firebase Crashlytics or Sentry) when declared in product requirements, or presenting a graceful crash dialogue on next application launch.

#### 73.17.4 AppMetricsScaffolder

`AppMetricsScaffolder` instruments application runtime performance and UI responsiveness:
1. *Frame rendering and jank tracking:* Integrates AndroidX Metrics (`androidx.metrics:metrics-performance`) and `JankStats` listeners on the main Activity window to observe frame render durations and track jank states across Compose screens.
2. *Startup latency instrumentation:* Scaffolds `reportFullyDrawn()` invocations and Activity launch timing markers to measure Time-To-Initial-Display (TTID) and Time-To-Full-Display (TTFD).
3. *Network latency metrics:* Injects OkHttp `EventListener` metrics collectors to record DNS resolution times, TLS handshake durations, and round-trip request latency.

#### 73.17.5 TracingInstrumentationScaffolder

`TracingInstrumentationScaffolder` instruments system tracing across critical application journeys:
1. *AndroidX Tracing integration:* Adds `androidx.tracing:tracing-ktx` dependencies and instruments critical paths (database queries, network deserialization, heavy image decoding) using inline `trace("section_name") { ... }` blocks.
2. *Perfetto and Systrace compatibility:* Emits trace tags compatible with Android Studio Profiler, Perfetto, and Systrace for deep visual timeline inspection.
3. *Compose recomposition tracking:* Configures Compose compiler trace flags and debug recomposition markers on critical UI screens to facilitate performance auditing during emulator runs.

#### 73.17.6 InAppDiagnosticsScaffolder

`InAppDiagnosticsScaffolder` scaffolds an in-app debug diagnostics dashboard and export utility:
1. *Debug diagnostics Compose screen:* Generates a debug-only Jetpack Compose screen (restricted to `debug` build variants via source set isolation) displaying live device hardware parameters, Room database table row counts and schema versions, active DataStore preference keys, network cache metrics, and scheduled WorkManager job states.
2. *Diagnostic archive exporter:* Synthesizes a one-tap export routine that bundles recent Logcat excerpts, local database schema dumps, device metadata, and memory status into a compressed ZIP file or dispatches it via an Android `Intent.ACTION_SEND` share sheet for rapid issue triage.
3. *Storage inspection:* Provides utilities to inspect internal private storage consumption, cache directories, and database file sizes, alerting developers when disk allocations exceed normal thresholds.

#### 73.17.7 AnalyticsSchemaGenerator

`AnalyticsSchemaGenerator` synthesizes strongly-typed analytics event models for the Android project:
1. *Type-safe event taxonomy:* Generates Kotlin sealed class or sealed interface hierarchies representing all tracked domain events (e.g. `ScreenViewEvent`, `ButtonClickedEvent`, `TransactionCompletedEvent`, `FeatureUsedEvent`) with strictly typed parameter schemas.
2. *Analytics dispatcher contract:* Scaffolds an `AnalyticsDispatcher` interface with standard event logging methods (`logEvent(event: AnalyticsEvent)`), decoupling business logic from third-party vendor analytics SDKs.
3. *Validation and payload sanitization:* Validates event names and parameter keys against platform length and formatting constraints (e.g. Firebase Analytics 40-character limits and alphanumeric character rules) and checks against `PiiFieldClassifier` to prevent PII leakage into analytics streams.

#### 73.17.8 FeatureUsageTracker

`FeatureUsageTracker` scaffolds local feature adoption and usage telemetry:
1. *First-use detection:* Implements Jetpack DataStore preference tracking to record feature discovery timestamps and first-use flags, supporting progressive onboarding tooltips and feature discovery flows.
2. *Interaction frequency counters:* Maintains local counters tracking feature utilization frequency, enabling adaptive UI prioritization and local recency/frequency caching.
3. *Funnel milestone tracking:* Tracks step completions across multi-stage user journeys (e.g. onboarding, checkout, registration) to detect abandonment points without requiring cloud analytics infrastructure.

### 73.18 AndroidPlatformTargetService

**Role:** aggregate query facade and Android platform target compliance — read-only services; no authority, no AI-usage budget.

#### 73.18.1 AndroidPlatformTargetService

`AndroidPlatformTargetService` is the supervisor-owned, read-only aggregate query facade that coordinates Android OS platform-specific generation, Gradle configuration, shrinker rules, system integrations, and device fragmentation compliance. It exposes a typed query surface to platform engineering workers (`Android Platform Worker`, `Architecture Worker`, `Release Worker`) and registered IPC command handlers.

`AndroidPlatformTargetService` coordinates seven deterministic analytical and synthesis components:
1. *Manifest permission derivation:* Invokes `ManifestPermissionDeriver` (§73.18.2) to statically derive required `<uses-permission>` tags and runtime permission requests based on framework API usage.
2. *Gradle configuration synthesis:* Invokes `GradleConfigSynthesizer` (§73.18.3) to scaffold and maintain multi-module `build.gradle.kts`, `settings.gradle.kts`, and `libs.versions.toml` version catalogs.
3. *Shrinker rule generation:* Invokes `ShrinkerRuleGenerator` (§73.18.4) to synthesize and validate ProGuard/R8 consumer rules for reflection, serialization, Room entities, and JNI entry points.
4. *Notification channel setup:* Invokes `NotificationChannelSetup` (§73.18.5) to scaffold Android 8.0+ (API 26+) notification channel structures and Android 13+ (API 33+) `POST_NOTIFICATIONS` runtime permission flows.
5. *Deep link and intent filter generation:* Invokes `DeepLinkIntentFilterGenerator` (§73.18.6) to generate `<intent-filter>` declarations in `AndroidManifest.xml` and Jetpack Compose Navigation 2.8+ type-safe deep links.
6. *Target API deadline tracking:* Invokes `TargetApiDeadlineTracker` (§73.18.7) to verify that `targetSdk` satisfies current Google Play Store submission mandates and warn on approaching deprecation deadlines.
7. *Dependency vulnerability automerging:* Invokes `DependencyVulnerabilityAutomerger` (§73.18.8) to bump vulnerable patch dependencies in `libs.versions.toml` and orchestrate isolated smoke verification.

*Integration with established platform components:*
`AndroidPlatformTargetService` directly integrates with and builds upon Nirman's established canonical platform analyzers: API level compliance and desugaring are enforced via `AndroidApiLevelValidator` (§47.4); Jetpack lifecycle flows are verified by `AndroidDataFlowAnalyzer` (§47.4) and the closed-world decision matrix (§73.2); background work execution is governed by `OfflineSyncProtocolPlanner` (§47.4) and WorkManager policies (§73.2); and multi-device matrix coverage is validated by `AndroidEmulatorScenarioCoordinator` (§65) against `DeviceMatrixEntry` profiles (BS §59.2).

`AndroidPlatformTargetService` creates no second authority. It does not directly mutate project source or bypass policy; all platform target proposals route through `MutationBroker` (BS §43.2) and commit via `ConstructionTransaction` (BS §42.2). `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

#### 73.18.2 ManifestPermissionDeriver

`ManifestPermissionDeriver` statically maps Android framework API usage to manifest and runtime permissions:
1. *AST framework API scanning:* Walks the Tree-sitter AST of source files to identify calls to protected Android framework APIs (e.g. location services, camera, Bluetooth, telephony, biometrics).
2. *Manifest permission mapping:* Queries the `AndroidSymbolGraph` `REQUIRES_PERMISSION` edges (§47.3) to derive the exact `<uses-permission>` tags required in `AndroidManifest.xml`, distinguishing normal permissions from dangerous (runtime) permissions and special permissions (`SCHEDULE_EXACT_ALARM`, `MANAGE_EXTERNAL_STORAGE`).
3. *Runtime permission flow scaffolding:* Scaffolds modern AndroidX `rememberLauncherForActivityResult` with `ActivityResultContracts.RequestPermission()` or `RequestMultiplePermissions()`, ensuring mandatory rationale dialogs and permission-denied fallbacks are generated for all dangerous permissions.

#### 73.18.3 GradleConfigSynthesizer

`GradleConfigSynthesizer` coordinates the synthesis and reconciliation of modern Android Gradle build configurations:
1. *Version catalog management:* Generates and maintains `gradle/libs.versions.toml`, managing `[versions]`, `[libraries]`, and `[plugins]` blocks in accordance with single-writer reconciliation rules (BS §35).
2. *Kotlin DSL convention plugins:* Synthesizes type-safe `build.gradle.kts` files using Gradle Kotlin DSL, applying standard Android Gradle Plugin (AGP) convention plugins (`com.android.application`, `com.android.library`, `org.jetbrains.kotlin.plugin.compose`, `org.jetbrains.kotlin.plugin.serialization`).
3. *Build variant and flavor configuration:* Configures `debug` and `release` build types, application ID suffixes, signing configurations, and optimization flags (`isMinifyEnabled`, `isShrinkResources`).

#### 73.18.4 ShrinkerRuleGenerator

`ShrinkerRuleGenerator` synthesizes and validates ProGuard/R8 consumer shrinking and obfuscation rules:
1. *Serialization rule synthesis:* Generates `-keepclassmembers` and serialization rules for `kotlinx.serialization` `@Serializable` classes, Moshi, or Gson models, preventing runtime `ClassNotFoundException` and JSON field deserialization failures.
2. *Room entity and DAO keep rules:* Synthesizes keep rules for Room database entities, DAOs, and generated type adapters to preserve SQLite table mappings and reflection-based database initializers.
3. *JNI and reflection preservation:* Scans for `@Keep` annotations, JNI `native` method declarations, and dynamic reflection calls, emitting targeted keep rules to prevent symbol stripping during R8 release builds.

#### 73.18.5 NotificationChannelSetup

`NotificationChannelSetup` manages Android notification channel creation and permission compliance:
1. *Channel and group synthesis:* Generates boilerplate for notification manager channel and channel group creation on Android 8.0+ (API 26+), defining channel IDs, human-readable names, descriptions, importance levels (`IMPORTANCE_HIGH`, `IMPORTANCE_DEFAULT`), vibration patterns, and sound attributes.
2. *POST_NOTIFICATIONS runtime handling:* Scaffolds Android 13+ (API 33+) `android.permission.POST_NOTIFICATIONS` runtime permission checks before notifications are posted, ensuring seamless compatibility across Android versions.
3. *Channel intent routing:* Provides helper extensions for directing users directly to app notification settings (`Settings.ACTION_CHANNEL_NOTIFICATION_SETTINGS`) when notification permissions or specific channels are disabled.

#### 73.18.6 DeepLinkIntentFilterGenerator

`DeepLinkIntentFilterGenerator` coordinates Android deep linking across manifests and navigation graphs:
1. *Manifest intent-filter synthesis:* Emits `<intent-filter android:autoVerify="true">` blocks in `AndroidManifest.xml` with `<action android:name="android.intent.action.VIEW" />`, `<category android:name="android.intent.category.DEFAULT" />`, `<category android:name="android.intent.category.BROWSABLE" />`, and targeted `<data>` URI schemes, hosts, and path prefixes.
2. *Navigation Compose deep link binding:* Generates Jetpack Compose `navDeepLink` definitions matching type-safe route serializable classes, ensuring external URIs map directly into the app's navigation graph with typed argument extraction.
3. *App Links asset verification:* Synthesizes the required `assetlinks.json` Digital Asset Links file template for Android App Links domain verification.

#### 73.18.7 TargetApiDeadlineTracker

`TargetApiDeadlineTracker` enforces Google Play target SDK compliance and deadline tracking:
1. *Google Play targetSdk verification:* Validates the project's `targetSdk` against current Google Play Store submission requirements (e.g. requiring target SDK 34 or 35 for new apps and updates).
2. *Annual deadline tracking:* Cross-references the active calendar date against Google Play's annual August 31st target SDK deadlines, flagging warning diagnostics when the project target SDK is nearing obsolescence.
3. *Migration guidance:* Identifies framework behavioral changes introduced in newer target SDK levels (e.g. Android 14 foreground service types, Android 15 edge-to-edge enforcement) to guide autonomous upgrade planning.

#### 73.18.8 DependencyVulnerabilityAutomerger

`DependencyVulnerabilityAutomerger` coordinates autonomous security upgrades for third-party Gradle dependencies:
1. *Vulnerability assessment:* Cross-references the resolved dependency tree against security advisories, flagging dependencies with known CVEs.
2. *Non-breaking patch resolution:* Queries Maven Central and Google Maven to derive the minimal non-breaking patch version update within the declared major/minor compatibility window.
3. *Automated staging and verification:* Updates version declarations in `gradle/libs.versions.toml`, stages an isolated `ConstructionTransaction`, executes project test suites and headless emulator smoke tests, and promotes the update upon passing verification evidence.

## 74. Integration Boundary Implementation Contract

**Implements:** build spec §70 and `CONTRACT.RUNTIME.INTEGRATION_BOUNDARY`
**Canonical schema owner:** `CanonicalSchemaRegistry` in §36.1
**Implementation owner:** the Rust control plane and supervised boundary services

The runtime implements the common boundary envelope as a correlation projection. It does not replace the authoritative specialized contracts. `WorkflowCoordinator` creates or updates the boundary reference, the relevant deterministic authority admits the operation, and the specialized service owns its state transition.

The persisted envelope is the build spec §70 `IntegrationBoundaryContract`, field for field (the build spec is the canonical owner); `BoundaryOperationProjection` below is the runtime state projection keyed by its `operationRef`:

> **Schema projection:** `IntegrationBoundaryContract` is defined in `nirman-schemas.md` §1.36. Owner: BS §70.

`BoundaryOperationProjection` is not a second lifecycle authority:

> **Schema projection:** `BoundaryOperationProjection` is defined in `nirman-schemas.md` §2.71. Owner: TA §74.

The projection is valid only when `specializedStateRef` resolves to the state machine owned by the applicable service. Lease loss fences the operation by revoking capabilities and rejecting new writes. A timeout or cancellation produces a durable lifecycle event. A retry after an unknown device or external outcome requires the relevant transaction reconciliation, idempotency read-back, or compensation evidence before a new effect is authorized. A stale source revision, contract version, adapter version, toolchain, emulator state, application state, environment state, artifact, credential, or policy invalidates dependent observations and downstream effects.

### 74.1 Android service integration

> **Schema projection:** `AndroidServiceIntegration` is defined in `nirman-schemas.md` §2.72. Owner: TA §74.1.

> **Schema projection:** `ContractDouble` is defined in `nirman-schemas.md` §2.95. Owner: TA §74.1.

> **Schema projection:** `ContractDoubleScenario` is defined in `nirman-schemas.md` §2.132. Owner: TA §74.1.

`ContractDouble` is owned and run by the supervisor inside `nirman-android`, bound to a loopback address only, and torn down with the session (ADR-225; build spec §76.5). Workers never open it and never reach it — the worker network rule of §3.5 is unchanged — and the emulator reaches it only through the host alias. Its request, response, and error bodies are generated from the integration's schema references and its fixtures from the integration's declared scenarios; a response the schema cannot produce is a double failure, not application evidence. Every exchange is recorded, and the evidence it yields carries `DOUBLE_BACKED` so that `EvidenceAuthority` never counts it toward `FUNCTIONAL` for the real service.

For each declared external integration, the supervisor deterministically selects fault and edge scenarios based on: (1) authentication type, (2) write versus read operation, (3) retry safety and idempotency support, (4) pagination, (5) file transfer, (6) streaming or chunked transfer, (7) offline queueing requirements, (8) sensitive-data classification, and (9) declared acceptance criteria. The selection record establishes why each fault mode from `faultMode` (`SUCCESS`, `VALIDATION_ERROR`, `AUTHENTICATION_FAILURE`, `AUTHORIZATION_FAILURE`, `RATE_LIMIT`, `RETRY_AFTER`, `TIMEOUT`, `DISCONNECT_BEFORE_HEADERS`, `DISCONNECT_DURING_BODY`, `MALFORMED_RESPONSE`, `PARTIAL_RESPONSE`, `DUPLICATE_RESPONSE`, `OUT_OF_ORDER_RESPONSE`, `UNKNOWN_SUBMISSION_OUTCOME`, `SCHEMA_EVOLUTION`) is applicable or excluded. Double-backed execution produces evidence labeled `DOUBLE_BACKED` with an immutable binding to `coveredRequirementIds`. `EvidenceAuthority` never counts `DOUBLE_BACKED` evidence toward promoting an external service to `FUNCTIONAL`, ensuring mock simulation cannot masquerade as real-service verification (ADR-258).

An Android service integration is a supporting dependency of the generated Android application. It does not create a second generated target. Its functional state is promoted only from the declared integration scenario and evidence, not from local compilation, application launch, or endpoint reachability alone. `requiredOperationality` is the build spec §5.7.5 minimum: the integration satisfies its boundary only when the current `IntegrationOperationality.aggregateState` for `integrationId` meets or exceeds `requiredOperationality` in the build spec §5.7.5 order (`CONFIGURED` < `REACHABLE` < `FUNCTIONAL`; `DEGRADED`, `USER_REQUIRED`, `UNAVAILABLE`, `BLOCKED`, and `UNKNOWN` never satisfy a `requiredOperationality` of `CONFIGURED` or above), and the comparison is made by `PolicyAuthority` from the recorded operationality evidence, never from the model's report.

Credential handling for an AndroidServiceIntegration is supervisor-owned.
The supervisor resolves credentialReference through the configured secure
credential provider only when the declared integration operation requires it.
Workers and generated-project processes MUST NOT receive Nirman credential
store access. Missing, invalid, or expired credentials are classified through
IntegrationOperationality and surfaced as USER_REQUIRED, never inferred away
by the model.

The runtime MUST distinguish safe project configuration from secret material:
non-secret identifiers and endpoint configuration may belong to the generated
project when required by its contract; secret values remain referenced through
the secure credential boundary. Credential acquisition, replacement,
validation, and invalidation MUST be attributable to the integrationId,
operation, project revision, and evidence chain.

### 74.2 UI hierarchy observation

> **Schema projection:** `UiHierarchyObservation` is defined in `nirman-schemas.md` §2.73. Owner: TA §74.2.

UI-hierarchy evidence may support accessibility, navigation, state, and visual checks. It cannot replace supervised Nirman-managed local Android emulator execution and cannot satisfy validation while requested, predicted, simulated, stale, or invalidated.

> **Schema projection:** `ScreenModel` is defined in `nirman-schemas.md` §2.91. Owner: TA §74.2.
> **Schema projection:** `VisualObservation` is defined in `nirman-schemas.md` §2.97. Owner: TA §74.2.

`VisualObservation` is the visual supplement to the primary `ScreenModel` perception of §74.2 and carries contract `CONTRACT.RUNTIME.E2E`.

`ScreenModel` is the text-native perception channel of the autonomous loop (ADR-225). `AndroidDeviceAdapter.captureUiHierarchy` produces the raw hierarchy; the device layer normalizes it into a `ScreenModel` whose elements carry identity, text, bounds, and actionability, and whose `screenFingerprint` is stable across captures of the same screen state. Workers and `ScenarioSynthesizer` act on the `ScreenModel`, never on pixels: a tap targets an element identity, an assertion names an element property, and a screen is recognized by its fingerprint. Screenshots remain evidence for humans and for visual criteria; a vision model (`visionModelId`) is optional and its absence marks visual criteria `NOT_OBSERVED` — it never blocks functional completion and never substitutes for a `ScreenModel`. A `ScreenModel` whose `windowKind` is not `APP` is a system surface handled by the device adapter (§73.12), not by a worker — except `INPUT_METHOD`, which is part of the application interaction (§73.12).

### 74.3 Signing and export verification

> **Schema projection:** `ExportVerificationRecord` is defined in `nirman-schemas.md` §2.74. Owner: TA §74.3.

> **Schema projection:** `AndroidArtifactInspectionRecord` is defined in `nirman-schemas.md` §2.133. Owner: TA §74.3.

`ExportVerificationRecord` is the canonical implementation record for both source access and deployment delivery. For local deployment it is materialized as an `APKExportRecord` view with `artifactKind: APK`, `deploymentDelivery: REQUIRED_APK`, `destinationKind: LOCAL_WINDOWS_FILESYSTEM`, the verified signing identity, validation decision, promotion decision, source and destination identities, source and destination hashes, byte count, and post-copy evidence. A declared AAB uses the same record only when the immutable `PackagingProfile` is `APK_AND_AAB`; it never makes AAB mandatory. Source, ZIP, and Git exports use `SOURCE_ACCESS_ONLY` and cannot satisfy artifact delivery or completion.

Before any APK or AAB is promoted to preview release or exported, `AndroidArtifactInspector` executes content inspection to emit an authoritative `AndroidArtifactInspectionRecord` (ADR-259). The inspection performs structural verification of the package: (1) compares `packageName`, `versionCode`, `versionName`, `minSdk`, and `targetSdk` against the authoritative `AndroidConstructionContract`; (2) validates the cryptographic signature identity (`signingIdentityRef`) and signing certificate digest; (3) computes and binds `manifestDigest`, `dexDigests`, and `resourceTableDigest`; (4) inventories native libraries (`nativeLibraryInventory`) and ABIs (`abiInventory`) to verify x64/ARM target consistency; (5) inventories declared permissions (`permissionInventory`) and exported components (`exportedComponentInventory`) against security policy; (6) compares packaged assets against `BrandManifest` and asset manifests (`assetManifestComparison`); (7) scans for embedded plain-text API keys or secrets (`embeddedSecretFindings`); (8) verifies debuggable flag (`debuggable: false` for release artifacts), backup policy, and cleartext traffic policy; (9) flags any unexpected archive entries (`unexpectedEntries`); and (10) binds the Software Bill of Materials (`sbomRef`). The resulting record is committed to `EvidenceAuthority` and evaluated by `PreviewPromotionGate` and `ArtifactAuthority`. The inspector produces evidence only; it holds no independent promotion authority.

Local export is complete only when the authorized destination exists, its byte count and content hash match the source artifact, the path is within approved export scope, and post-copy verification evidence is durable. Export does not by itself prove signing, preview currency, integration functionality, documentation certification, or user-goal completion.

### 74.4 Deployment export admission and reconciliation
The artifact export handler accepts a deployment request only after resolving the declared `PackagingProfile`, artifact kind, signing identity binding, validation decision, promotion decision, and destination policy. The only deployment destination is the approved local Windows filesystem. It creates one durable `ExportVerificationRecord` before copying, transitions through copy and post-copy verification states, and records source/destination identity and hash equality. A failed, interrupted, or unknown copy remains durable and is reconciled before retry. The handler may separately serve source/workspace, ZIP, or Git access, but that branch is marked `SOURCE_ACCESS_ONLY` and cannot emit deployment evidence or advance completion.

### 74.5 Documentation certification report

> **Schema projection:** `DocumentationCertificationReport` is defined in `nirman-schemas.md` §2.75. Owner: TA §74.5.

`result` carries the verifier's terminal status verbatim (build spec §67.11): `FAIL` when `defectCount` is non-zero; `DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS` when `defectCount` is zero and `checksUnevaluated` is non-zero, with every skipped subject listed in `unevaluatedSubjects`; `DOCUMENTATION_CERTIFIED` only when both are zero. The report has no `PASSED` value, so a report can never be read as complete certification while a check went unevaluated.

The report certifies documentation identity, registry resolution, graph structure, and declared semantic documentation rules only. It never certifies runtime source, Windows isolation, provider behavior, Android execution, preview truth, recovery, signing, or APK validity.

### 74.6 Orchestration wiring implementation

IntegrationBoundaryContract defines the static boundary; OrchestrationWiringMatrix records one runtime traversal of that boundary. The supervisor validates every boundary before execution:

```text
boundaryId
  → IntegrationBoundaryContract (lookup)
  → operation/schema/protocol/adapter compatibility check
  → authority/policy check
  → execution
  → observation/event
  → matrix completion (durable ledger write)
```

The matrix is durable in the supervisor ledger and rebuilt/reconciled after restart. Critical subgraphs:

**Worker/context path:**
```text
WorkerConnection
  → ContextOrchestrator
  → ContextPackage
  → ModelGateway
  → ProviderAdapter
  → normalized response
  → WorkerConnection
  → AgentReasoningEngine
```

**Deliberation closure:**

The four static traversals in build spec §84.1.1 are one mediated subgraph,
not direct worker, tool, or execution shortcuts. `DeepDeliberationRuntime`
requests context only as `WorkerConnection.MODEL_CALL`; the supervisor's
`ContextOrchestrator` applies its integrity gate, assembles the context, and
returns only normalized `MODEL_EVENT`/context-manifest material. A read-only
diagnostic selected by `EvidenceAcquisitionPlanner` crosses the admitted
`ToolBroker` operation, becomes an `EvidenceRecord` through `EvidenceAuthority`,
and returns to the worker only through re-grounded `CYCLE_INPUT`. A completed
deliberation crosses `WorkerConnection` as `REASONING_ARTIFACT` and/or
`DELIBERATION_RECORD`, reaches `AgentExecutionKernel`, and can reach
`CapabilityBroker`, `DelegationManager`, or `WorkerRuntime` only after the
existing kernel and `PolicyAuthority` path admits it.

Each physical message or operation above has its own runtime
`IntegrationBoundaryContract`/`OrchestrationWiringMatrix` traversal; the
logical labels do not collapse those traversals into a new contract. The
worker's cancellation, fencing, restart, and replacement semantics remain
those of `WorkerConnection`, durable coordination, the kernel, and
`RecoveryAuthority`. Deliberation remains at the existing `HYPOTHESIZE` and
`STRATEGIZE` cycle states of §71.4; it does not add a lifecycle state.

**Preview interaction path:**
```text
Preview UI input
  → SupervisorConnection
  → PreviewCoordinator
  → AndroidDeviceAdapter
  → emulator
  → resulting stamped frame
  → RenderTransport
  → FrameNotice
  → PreviewHost

A displayed frame is valid only when its frame identity, project revision, artifact identity, device state, application state, and causal input identity all resolve to the currently promoted PreviewRevision.
```

**Construction/evidence path:**
```text
ConstructionTransaction
  → Checkpoint
  → Validation
  → EvidenceDependency validation
  → EvidenceAuthority
  → Promotion
```

Required critical orchestration subgraphs additionally include:
- plan emission → interface completeness → worker admission
- worker message persistence → dispatch → ACK/recovery
- plan revision → assignment migration → stale proposal rejection
- reservation acquisition → wait-for graph → deadlock recovery
- coordination progress → stall record → recovery
- execution epoch seal → roll-forward → replay
- provider outage → circuit → stream/retry reconciliation

### 74.7 Android API, Integration, and Identity Intelligence Services

**Role:** aggregate query facade and static contract validation — read-only services; no authority, no AI-usage budget.

#### 74.7.1 AndroidIntegrationIntelligenceService

`AndroidIntegrationIntelligenceService` is the supervisor-owned, read-only aggregate query facade that unifies API contract integrity, third-party integration analysis, webhook signature verification, and mobile authentication security. It exposes a typed query surface to integration workers (`Backend & Service Engineering Worker`, `Integration Double Worker`, `Android Data and Integration Worker`) and registered IPC command handlers.

`AndroidIntegrationIntelligenceService` coordinates four deterministic analytical components:
1. *Integration double management:* Interacts with supervisor-managed `ContractDouble` (§74.1) fixtures to verify that mocked endpoints strictly adhere to declared request and response schemas.
2. *API contract drift detection:* Invokes `ApiContractDriftDetector` (§74.7.2) to detect breaking schema changes, type mismatches, and route omissions between Android client network layers and API specifications.
3. *Third-party vendor integration verification:* Invokes `ThirdPartyIntegrationAnalyzer` (§74.7.3) to validate SDK wrapper boundaries, circuit breakers, exponential backoff, and webhook HMAC signature handling.
4. *Authentication and identity hardening:* Invokes `AuthFlowSecurityHardener` (§74.7.4) to audit OAuth 2.0 PKCE implementations, token refresh mutexes, biometric re-auth, Keystore encryption, and navigation route guards.

`AndroidIntegrationIntelligenceService` creates no second authority. It does not directly mutate project source or bypass policy; all integration proposals route through `MutationBroker` (BS §43.2) and commit via `ConstructionTransaction` (BS §42.2). `ProvenanceRecorder` remains the provenance gate inside `ArtifactAuthority`'s promotion.

#### 74.7.2 ApiContractDriftDetector

`ApiContractDriftDetector` provides static contract verification across client and server boundaries:
1. *Client-to-spec parity analysis:* Statically parses OpenAPI specifications and correlates them against Android client Retrofit/Ktor interfaces, serialization DTOs, and route parameters. Any missing field, type mismatch, or endpoint mismatch is flagged as `API_CONTRACT_DRIFT`.
2. *Breaking change detection:* Compares successive API specifications to identify non-additive mutations, such as removed endpoints, renamed properties, altered data types, or newly mandatory query/body parameters, emitting `BREAKING_API_CHANGE` findings.
3. *Error and pagination standard verification:* Asserts that remote endpoints conform to standardized error models (such as RFC 7807 Problem Details) and consistent cursor or limit-offset pagination structures.

#### 74.7.3 ThirdPartyIntegrationAnalyzer

`ThirdPartyIntegrationAnalyzer` verifies external vendor SDK integration and communication resilience:
1. *Credential isolation verification:* Audits source files and Gradle build scripts to ensure third-party API keys and merchant secrets are not committed as plaintext literals, verifying that credentials resolve through Gradle `local.properties`, system environment variables, or Android Keystore (`TAINT_SENSITIVE_LEAK`).
2. *Fault tolerance and circuit breaking:* Statically verifies the presence of client-side circuit breakers and exponential backoff retry policies with jitter in OkHttp interceptors and WorkManager sync workers, preventing cascading thread exhaustion during third-party outages.
3. *Webhook signature verification:* Analyzes webhook receivers to guarantee incoming webhook payloads enforce cryptographic HMAC SHA-256 signature verification headers before processing.

#### 74.7.4 AuthFlowSecurityHardener

`AuthFlowSecurityHardener` audits authentication, authorization, and session security across the Android application:
1. *OAuth 2.0 PKCE compliance:* Verifies that mobile OAuth authorization flows implement Proof Key for Code Exchange (PKCE) with cryptographically random code verifiers and SHA-256 code challenges, rejecting insecure client-secret embedding.
2. *Thread-safe token refresh:* Analyzes OkHttp `Authenticator` and Ktor auth plugins to ensure JWT token refresh routines synchronize concurrent requests using mutex locks, preventing race conditions and duplicate refresh token exchanges.
3. *Keystore-backed storage and route guarding:* Verifies that auth tokens are stored in `EncryptedSharedPreferences` backed by the Android KeyStore, checks that biometric authentication utilizes AndroidX `BiometricPrompt`, and asserts that Jetpack Compose navigation graphs define explicit route guards redirecting unauthenticated sessions to login.

### 74.6.1 Local fast-decision traversal

| Traversal | Producer | Consumer | Schema | Authority | Persistence | Failure / Recovery | Invalidation | Test |
|---|---|---|---|---|---|---|---|---|
| `LDE-INPUT` | DecisionNodeManager / recovery classifier / router | LocalDecisionEngine | LocalDecisionEngineProfile + LocalDecisionAcceptanceProfile + typed decision request | Supervisor runtime admission | local_decision_engine_profiles + local_decision_acceptance_profiles | profile/resource/runtime/acceptance-profile failure → consumer-declared fallback | profile/model/runtime-adapter/acceptance-profile identity or input-state change | TEST-LDE-001 |
| `LDE-OUTPUT` | LocalDecisionEngine | DecisionNodeManager / recovery classifier / router | LocalDecisionProposal + LocalDecisionAcceptanceProfile | Deterministic consumer authority | local_decision_proposals + local_decision_acceptance_profiles | invalid/stale/below-acceptance/quarantined/mismatched-profile proposal → reject/fallback | model/profile/runtime-adapter/acceptance-profile revision, context hash, state hash, input revision, `expiresAt` | TEST-LDE-001 |

The consumer-declared fallback is part of the owning decision contract. A consumer with no explicit fallback declaration is an integration defect and MUST NOT invent fallback behavior at implementation time.

The local fast-decision traversal is an optional internal branch. It does not create a provider request edge and does not replace the mandatory provider-backed reasoning traversal of BS §84.1.

## 75. Preview Synchronization Implementation Contract

**Implements:** build spec §71 and `CONTRACT.RUNTIME.PREVIEW_SYNC`
**Canonical schema owner:** `CanonicalSchemaRegistry` in §36.1
**Implementation owner:** `WorkflowCoordinator`, `PreviewCoordinator`, the durable event store, and the UI projection runtime

### 75.1 Event and reducer implementation

The architecture implements the exact `PreviewSyncEvent`, `PreviewProjection`, `PreviewProjectionReducer`, and `PreviewSyncEvidenceRecord` schemas defined by build spec §71.1. The event store assigns the durable per-project/task sequence. `WorkflowCoordinator` normalizes intent, agent, worker, build, device, evidence, recovery, and promotion outcomes into events. Every non-root event carries causal parentage, runtime-session identity where applicable, and an authority class. `PreviewCoordinator` is the only service that can emit an accepted promotion event. The UI consumes snapshots and events but never writes projection state.

`PreviewProjectionReducer` is a pure deterministic reducer over a snapshot and an ordered event range. It must be replayable without side effects, must record the reducer version and projection revision, and must produce the same state for the same snapshot and event range. The reducer delegates specialized decisions to the existing lifecycle, evidence, device, artifact, recovery, and promotion authorities; it does not grant permissions or approve evidence. Frame transport and presentation measurements are diagnostic observations and never preview truth; the canonical statement of that boundary is build spec §71.1 (`Preview performance is not preview truth`), and transport defaults versus diagnostic thresholds versus fixture thresholds versus product guarantees are distinguished by build spec §71.0.1.

### 75.2 Event ownership table

| Event family | Canonical producer | Required prerequisite | Authority class | Reducer update |
|---|---|---|---|---|
| Intent and contract | intent/contract services | accepted user request and schema validation | DECLARATIVE / PLANNED | intent and contract stage |
| Plan and checkpoint | planner and transaction authority | authorized plan and durable checkpoint | PLANNED | plan/checkpoint refs |
| Source and build | commit barrier and process supervisor | source revision and operation capability | EXECUTION_OBSERVED | source/build/artifact fields |
| Install and runtime | emulator manager and supervised process | emulator transaction and observed result | RUNTIME_OBSERVED | install/launch/runtime fields |
| Observation and validation | observation services and independent validators | matching preview identity | EVIDENCE_BACKED / VALIDATED | evidence and validation refs |
| Recovery and invalidation | RecoveryAuthority and evidence authority | typed failure or invalidation | EXECUTION_OBSERVED | recovery/stale/invalidated fields |
| Promotion | `PreviewCoordinator` through `PreviewPromotionGate` | complete current evidence bundle | CERTIFIED | active preview reference |
| Stream control | event store and authenticated supervisor connection | sequence/replay protocol | EXECUTION_OBSERVED | cursor and stream status |

### 75.3 Reducer consistency and stream recovery

The event store and reducer enforce these rules:

1. Events are applied by durable sequence, not arrival time.
2. A repeated event ID with the same payload hash is idempotent.
3. A repeated event ID with a different payload is quarantined as a protocol violation.
4. A sequence gap blocks advancement and requests replay.
5. An older event can be retained as historical evidence only when its identity matches the candidate it describes; it cannot overwrite current projection fields.
6. A revision, checkpoint, artifact, device, application, environment, branch, contract, or policy mismatch marks the event stale or invalidated.
7. Stream loss freezes preview advancement and displays the last durable projection with a stale-stream indicator.
8. Reconnect validates snapshot cursor and projection revision, replays the missing range, and returns to connected state only after continuity is proven.
9. Reducer replay is side-effect free and deterministic.
10. Promotion and completion consume the reducer’s current projection only after the canonical evidence and promotion authorities pass.
11. For a compatible identity, current supervised runtime/device observation reconciles contradictory persisted runtime state; for an incompatible identity, the projection is marked stale or invalidated rather than merged.
12. Events after cancellation, rollback, promotion, or worker fencing are historical or quarantined unless a new authorized lineage admits them.

### 75.4 Runtime certification evidence and tests

`PreviewSyncEvidenceRecord` is persisted with the event sequence range, reducer version, projection revision, preview revision, source revision, checkpoint, branch identity, artifact fingerprint, emulator identity, runtime-session identity, state fingerprints, event IDs, observation references, evidence references, validation references, invalidated evidence, recovery events, promotion record, certification decision, and completion decision. Runtime certification must execute the complete chat instruction → agent proposal → authorized mutation → source revision → build → APK → install → emulator runtime → observation → validation → promotion → event replay → panel projection path.

The test family must inject duplicate and conflicting events, out-of-order events, sequence gaps, stale candidate results, late device observations, UI disconnect, supervisor restart, event replay, failed candidate recovery, and a successful last-known-good promotion. The expected panel state must be identical after live application and replay, and no predicted, requested, simulated, stale, invalidated, or model-authored record may appear as current verified preview evidence.

## 76. Autonomous Continuation and Specialist Gate Contract

This section implements the event-driven continuation requirements in build spec §27.11. It does not create a second scheduler, worker authority, validation authority, or recovery state machine. The existing runtime tick, lifecycle-hook, trigger, task-graph, `RecoveryAuthority`, `DependencyHealthService`, `ConstructionTransaction`, `PreviewPromotionGate`, and evidence authorities remain canonical.

### 76.1 Trigger-to-action continuation

The runtime tick consumes durable events and schedules the next authorized action without requiring another chat message. A saved workspace revision schedules affected formatting, lint, typecheck, and focused tests when the project policy enables them. A completed build schedules artifact inspection, affected tests, regression checks, and runtime prerequisites. A captured failure schedules diagnostic classification and repair context creation. A dependency change schedules compatibility, vulnerability, license, provenance, size, and duplicate-class checks before commit or build continuation. A local preview promotion or artifact export request schedules health checks, artifact inspection, required validation, signing/certificate checks, and post-copy verification.

Each continuation carries the current task, goal, project revision, checkpoint, branch or candidate, worker run, operation capability, correlation and causation identifiers, policy decision, attempt history, and evidence references. The scheduler must not issue a blind retry: the next action must identify new evidence, a materially different strategy, a changed worker/model profile, a restored checkpoint, or a changed environment condition.

### 76.2 Diagnostic feedback loop

`RuntimeTraceAnalyzer` captures structured process output, stack-trace references, Logcat, ANRs, native crash reports, install failures, permission denials, activity/service lifecycle events, and test-runner diagnostics. It produces a stable failure fingerprint and a bounded `FailureContextPackage` containing the relevant error evidence, changed-file scope, environment identity, prior strategies, checkpoint, validation results, privacy classification, and next-action constraints. The package is sent to the next authorized diagnostic or coding worker; raw private reasoning is never required or persisted.

The loop is:

```text
source or runtime event
  → automatic affected checks
  → failure observation
  → trace capture and failure fingerprint
  → FailureContextPackage
  → authorized diagnosis or repair
  → checkpointed patch
  → build and runtime validation
  → evidence update or materially different recovery strategy
```

The recovery-attempt policy (`recoveryAttemptPolicy`) is policy-configurable and bounded: it caps materially different recovery attempts per failure fingerprint. It is an anti-thrashing and liveness constraint rather than an AI token, request, monetary, reasoning, or duration budget (BS §72). Repeating the same command, patch, prompt, or provider route does not count as a new attempt. When the policy's bound is reached or safe strategies are exhausted, the runtime changes strategy, backtracks, delegates, escalates, degrades, or reports a
truthful blocker. Exhaustion of materially equivalent attempts MUST trigger strategy transformation, delegation, backtracking, branching, or escalation. Exhaustion MUST NOT itself terminate the goal.

AndroidWorkflowCoordinator MUST route every Android construction and runtime failure through RecoveryAuthority and AgentExecutionKernel. The Android loop MUST NOT implement an independent retry or termination policy.

### 76.3 Specialist worker responsibilities

Specialist gates are responsibilities assigned to the canonical worker roles of §6.5 (build spec §23.4); they are not additional roles and do not become additional authorities. Each gate names the §6.5 role that carries it, so the specialist fixtures of development plan M110 dispatch through the same `WorkerRegistry` taxonomy as every other task.

| Specialist gate | Canonical worker role (§6.5) | Required responsibility | Blocking evidence or gate |
|---|---|---|---|
| Orchestration | Primary Orchestrator | Maintain one shared goal, acceptance contract, task graph, dependency order, and handoff record | Authorized plan and task-graph transition |
| Security scanning | Security Worker | Detect secrets, unsafe configuration, dependency vulnerabilities, license violations, provenance gaps, and client-bundle exposure | Security and dependency evidence before commit or artifact promotion |
| Schema/type consistency | Reconciliation Worker | Compare schemas, types, UI/control-plane messages, Android service contracts, and persisted records for drift | Schema compatibility and contract-parity result |
| Diff-aware patching | Debugging Worker (repairs) or the owning implementation worker (UI Worker, Android Data and Integration Worker) | Apply scoped patches against the current revision, preserve unrelated user edits, and emit a reviewable diff | Workspace revision, reservation, and reconciliation checks |
| Diagnostics | Diagnostic Worker (as the child of the failing worker) or Debugging Worker | Classify failures, correlate stack traces and runtime observations, and produce `FailureContextPackage` | Failure fingerprint and evidence references |
| Validation | Test and QA Worker, with Emulator Driver Worker for scenario execution and Visual QA Worker for visual/accessibility checks | Run focused and regression checks, Android build/emulator validation, and visual/accessibility checks | Independent validation and current evidence |
| Memory/index update | Documentation Worker | Update the project index, settled decisions, conventions, failure patterns, sanitized episode summaries, verify KDoc/code consistency via `DocCodeMismatchDetector`, and synthesize export `README.md` via `ProjectReadmeSynthesizer` | Privacy classification and memory-write policy |
| Release preparation | Release Worker | Prepare artifact, signing, certificate, promotion, and local export records without bypassing authorities | `PreviewPromotionGate`, signing authority, and export verification |
| Adversarial critique | Critic Worker | Search for the counterexample that would make the selected strategy or completion claim wrong; request the discriminating evidence | A critique finding blocks authorization until answered with evidence (build spec §68.10) |
| Integration doubles | Integration Double Worker | Author and conform `ContractDouble` fixtures for every declared integration without a reachable backend | Double conformance evidence; `IntegrationState` never passes `SPECIFIED` on its account |

The orchestrator reconciles specialist handoffs against one shared contract and the current project revision. A worker report cannot mark a task complete, promote a preview, approve a dependency, or authorize an external effect. A specialist may recommend a result only through its typed operation and evidence contract.

### 76.4 Acceptance requirements

The implementation must prove that file-save continuation, build-completion continuation, failure-to-diagnostics feedback, dependency scanning, local promotion/export health checks, rollback, specialist handoffs, and sanitized memory updates are durable and replayable. It must also prove that a disconnected UI does not stop authorized background work, a failed health check preserves last-known-good state, a stale worker cannot apply a patch, a security or dependency gate can block a commit, and a model statement cannot substitute for evidence.

Nirman remains a Windows-first local host for Android generation. The isolation boundary is the approved Windows workspace and supervised process environment; no container, virtual machine, WSL, or generic web/cloud deployment runtime is implied by this contract.

## 77. Runtime Resource Integrity Implementation Contract

**Implements:** build spec §72 and `CONTRACT.RUNTIME.RESOURCE_INTEGRITY`

### 77.1 Canonical schema

`ResourceIntegrityRecord` (BS §72) is persisted with the task and operation ledger. `ResourceIntegrityAuthority` (§59) evaluates `resourceRequirements` against currently admissible physical capacity before admission and records `observedPressure`, `pressureResponse`, and `livenessState` while work runs. It receives process telemetry, host memory and disk signals, emulator slot state, workspace I/O and concurrency counters, provider context capacity, and liveness probes through typed records. Provider `UsageRecord`s (tokens, requests, estimated cost, reasoning usage) are linked through `usageTelemetryRefs` and are observational: the authority never reads them to admit, deny, throttle, degrade, pause, or terminate work, and no record field carries an AI-usage ceiling, reservation, remaining budget, or exhaustion outcome.

### 77.2 Lifecycle and authority

The lifecycle is `DECLARED → ADMITTED → RUNNING → COMPLETED`, with `QUEUED`, `CONCURRENCY_REDUCED`, `RESCHEDULED`, `CHECKPOINTED`, `SERIALIZED`, `RECLAIMING`, `RECOVERING`, and `BLOCKED_NO_SAFE_PATH` side states. Under physical pressure the authority applies queueing, concurrency reduction, scheduling, checkpointing, work serialization, cache and resource reclamation, and recovery in that order before a blocking outcome, and blocks only when no safe path remains. It may queue, reschedule, reduce concurrency, checkpoint, or contain a hung operation, but cannot grant an operation capability, promote evidence, widen permission, or mark work complete; it cannot override safety, privacy, signing, evidence, or completion authority. A liveness timeout is scoped to one hung operation and MUST NOT terminate a healthy goal for elapsed time. Tasks are never terminated, degraded, paused, or blocked because of token consumption, provider request count, monetary expenditure, reasoning usage, or elapsed autonomous-goal duration.

### 77.3 Failure and recovery

Memory exhaustion, disk exhaustion, process-count limits, emulator slot contention, workspace I/O saturation, hung operations, and telemetry loss produce durable diagnostics. Recovery may queue, reschedule, reduce concurrency, checkpoint, serialize, reclaim rebuildable caches, restart a contained process, or resume from the last checkpoint when capacity returns; it must never retry an unknown external charge blindly, and a `BLOCKED_NO_SAFE_PATH` outcome preserves the last checkpoint and event log and is never reported as completion. Unknown or unreported provider usage is recorded as `unavailable` telemetry and does not change execution.

## 78. Agent Trust Boundary Implementation Contract

**Implements:** build spec §73 and `CONTRACT.RUNTIME.AGENT_TRUST`

### 78.1 Canonical schema

`AgentTrustAssessment` is produced before a skill, MCP-compatible tool, plugin, or instruction-bearing package is admitted. Scanners run in a restricted local process and record content hashes, provenance, requested capabilities, static findings, behavioral findings, destinations, and the policy decision.

### 78.2 Lifecycle and authority

The lifecycle is `DISCOVERED → HASHED → SCANNED → POLICY_REVIEW → ADMITTED | QUARANTINED | REVOKED | EXPIRED`. Trust assessment is necessary but not sufficient for execution; capability, permission, credential, workspace, and external-effect authorities remain in force.

### 78.3 Failure and recovery

Hash drift, revoked content, scanner failure, malformed manifests, hidden instructions, or undeclared access requests cause quarantine or re-assessment. A quarantined package cannot execute through a cached admission record.

## 79. Context and Cache Governance Implementation Contract

**Implements:** build spec §74 and `CONTRACT.RUNTIME.CONTEXT_GOVERNANCE`

### 79.1 Canonical schema

`ContextCachePolicy` is resolved for each provider request and context package. `ContextGovernance` records selected content, protected content, compaction trigger, cache key inputs, invalidation causes, redactions, telemetry disclosures, resulting context lineage, the `placementPlan` of the transmitted package, and the post-compaction recall probe result.

### 79.2 Lifecycle and authority

The lifecycle is `DECLARED → SELECTED → COMPACTED_OR_FULL → CACHED_OR_UNCACHED → TRANSMITTED → INVALIDATED`. Context governance cannot delete mandatory constraints, change user intent, or convert summarized content into a fresh observation. Compaction output never carries active constraints, locked decisions, acceptance criteria, or revision identity; `CompactionPlanner` re-projects them from `ConstraintRegistry` and the ledger into the DENSE block after every compaction, and `RecallProbeService` verifies the re-projection before the next consequential step.

### 79.3 Failure and recovery

Context overflow, failed compaction, cache mismatch, cache corruption, privacy-policy change, or provider continuation loss causes context rebuild or safe reduction. The runtime must preserve required constraints and evidence references while recording what was excluded or summarized. A failed post-compaction recall probe is a context-quality failure, not a task failure: the runtime re-projects, narrows, or selects a provider model by measured reliability and records the outcome in the `AttentionReliabilityProfile`.

## 80. Android Runtime Integrity Implementation Contract

**Implements:** build spec §75 and `CONTRACT.RUNTIME.ANDROID_INTEGRITY`

### 80.1 Canonical schema

`AndroidRuntimeIntegrityObservation` is emitted by supervised device and runtime collectors. It binds each signal to project revision, artifact, package, device, runtime session, source, applicability, timestamp, and evidence.

### 80.2 Lifecycle and authority

The lifecycle is `REQUESTED → COLLECTING → OBSERVED → VALIDATED | NOT_APPLICABLE | UNAVAILABLE | USER_REQUIRED | INVALIDATED`. Runtime collectors observe; `ValidationAuthority` interprets the signal against the declared acceptance policy. No single signal can replace required build, install, UI, behavior, or evidence checks.

### 80.3 Failure and recovery

ANR, emulator session loss, unavailable Play Integrity, battery or Doze uncertainty, permission denial, stale runtime sessions, and collector errors produce typed evidence gaps. Recovery may restart collection, reconnect the device, change the declared profile, or report an honest coverage limitation; it cannot convert absence into a pass.

## 81. Frontend–Control-Plane Protocol Implementation Contract

**Implements:** build spec §76 and `CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE`

### 81.1 Canonical protocol schemas

The implementation owns the `UICommandRegistry`, `UICommandEnvelope`, `ProjectionSnapshot`, `UIResponseEnvelope`, `UIErrorEnvelope`, and `EventSubscription` schemas defined by build specification §76. Rust validates schema version, authenticated installation identity, user scope, project scope, command capability, expected projection revision, idempotency key, causation, and sensitive-field policy before invoking a use case.

The authoritative `ProjectionSnapshot` includes typed references for `taskProjection`, `workerProjection`, `previewProjection`, `artifactProjection`, `evidenceProjection`, `deliveryProjection`, and `backgroundContinuityProjection`. The continuity projection carries `BackgroundContinuityRecord`, its `stateVersion`, aggregate state, all continuity dimensions, authority decision reference, and last-known-good reference. The delivery projection carries `ExportVerificationRecord`, its export state, delivery kind, destination kind, artifact fingerprint, and post-copy verification reference. These are read-only projection fields; the frontend cannot synthesize or mutate them.

### 81.2 Command-to-domain wiring

```text
WinUI 3 ViewModel / presentation controller
  → typed IPC client
  → UICommandEnvelope
  → SupervisorConnection
  → command registry and schema validator
  → application use case
  → deterministic authority checks
  → repository and owned SQLite transaction
  → event store
  → projection projector
  → UIResponseEnvelope + ProjectionSnapshot + durable event stream
```

The command handler owns the transaction and maps persistence results to domain results. It does not return raw database rows. The projection projector maps durable domain state to a stable read model. A rejected command creates no domain mutation; an accepted command returns a durable command result or a typed failure. A duplicate idempotency key returns the stored prior result when the request fingerprint matches and returns `CONFLICT` when it does not.

### 81.3 Transport, replay, and failure behavior

The local IPC transport uses the `SupervisorConnection` handshake and the `EventSubscription` lifecycle. Snapshot-plus-event replay is cursor-atomic. The server applies per-connection backpressure, bounds batches, and records acknowledgements. A slow or disconnected UI does not stop eligible autonomous work; it only stops presentation updates until replay succeeds. A stale command is rejected with the current projection reference. A timeout or cancellation is durable and cannot be converted into success by a late response.

`UIErrorEnvelope` is safe for presentation and references protected diagnostics. Error categories map to recovery actions without giving the frontend recovery authority. Transport failures, schema incompatibility, supervisor restart, event retention gaps, and authentication expiry each have distinct recovery behavior and evidence.

### 81.4 Frontend state layers

WinUI 3 state is divided into `AuthoritativeProjectionState`, `OptimisticInputState`, `PendingCommandState`, `RejectedCommandState`, and `ConnectionState`. Only the first is derived from supervisor snapshots and durable events; optimistic input cannot update task, worker, preview, artifact, evidence, policy, signing, or completion truth. Reconnect discards stale derived state and rebuilds from the accepted snapshot cursor.

### 81.5 Generated Android service adapter

The generated Android project uses its own typed API client and `AndroidServiceIntegration` adapter. The adapter normalizes authentication, token refresh, timeout, retry, offline, idempotency, response, and application-error behavior for the generated app. It does not call Nirman IPC or write the Nirman ledger. Functional Android-service validation produces application evidence linked to the generated project revision and integration identity.

### 81.6 Technical acceptance tests

The implementation is accepted only when an executable fixture covers each initial command kind, authorization and scope denial, schema mismatch, duplicate and conflicting idempotency, stale projection, typed error mapping, cancellation, timeout, replay after reconnect, snapshot cutover, slow-client backpressure, supervisor restart, SQLite transaction rollback, and generated Android service error handling.

## 82. Background Continuity Implementation Contract
**Implements:** build spec §77 and `CONTRACT.RUNTIME.BACKGROUND_CONTINUITY`
**Canonical schema owner:** `CanonicalSchemaRegistry` in §36.1
**Implementation owner:** the existing supervisor/process-supervision authority, `WorkspaceLeaseManager`, the existing checkpoint store, `RecoveryAuthority`, the existing device-session/device-operation manager, the existing integration/provider operationality manager, and the durable event store

### 82.1 Canonical state and transition implementation
The implementation persists `BackgroundContinuityRecord` with a monotonic `stateVersion` and independently persists `ContinuityDimensions`. A transition is admitted only from the current version, under the current supervisor instance, lease, fencing token, project branch, and applicable host, device, and provider session identities. Each accepted transition emits a durable event containing causation, authority decision, checkpoint or reconciliation reference, recovery action, and evidence status. The aggregate state is recomputed from all dimensions by the precedence rule in build spec §77; it is not assigned by whichever event arrived last. The frontend receives this record only through the typed authoritative projection.

### 82.2 Recovery and reconciliation behavior
UI disconnect is presentation-only. Supervisor restart and host restart reload the last checkpoint, fence abandoned leases, reconcile descendants and unknown outcomes, and resume only eligible operations. Sleep, hibernation, and shutdown use the same recovery path after host and toolchain revalidation. Device loss invalidates device-bound evidence and preview state while retaining the project checkpoint. Provider or network outage records provider operationality and applies declared retry/backoff/degradation rules. No recovery path may fabricate an observation, validation pass, artifact, or completion result.

### 82.3 Projection, crosswalk, and runtime acceptance
The projection maps continuity dimensions and the derived aggregate to truthful UI labels and preserves last-known-good preview and evidence references. `IntegrationOperationality.UNAVAILABLE` or `DEGRADED` maps to `providerAvailabilityState=UNAVAILABLE` or `DEGRADED`; an unavailable or reattaching emulator session maps to `deviceAvailabilityState=UNAVAILABLE` or `REATTACHING`; and device-bound preview/evidence is invalidated through the existing evidence dependency graph. These mappings do not rewrite the source operationality or runtime-integrity records. Stale events are rejected by state version, session identity, branch identity, and fencing token. Acceptance fixtures must cover UI closure/reconnect, supervisor restart, reboot, sleep/hibernate, shutdown, device reattachment, provider/network outage, unknown-outcome reconciliation, stale-event rejection, and safe failure. Documentation certification proves only that these contracts and fixture declarations exist; runtime certification must execute the fixtures.

## 83. APK Export Provenance Implementation Contract
**Implements:** build spec §78 and `CONTRACT.RUNTIME.APK_EXPORT`
**Canonical schema owner:** `ExportVerificationRecord` in §74.3
**Implementation owner:** `ArtifactAuthority`, the existing signing-identity policy authority, `EvidenceAuthority`/`ValidationAuthority`, `PreviewPromotionGate` for preview promotion, the existing external-effect transaction/reconciliation authority, and the local Windows filesystem adapter. The labels `SigningAuthority`, `PromotionAuthority`, and `ExternalEffectCoordinator` are implementation aliases only and are not additional authorities.

### 83.1 Deployment admission and profile binding
The export handler resolves `packagingProfileId`, artifact kind, source revision, checkpoint, signing identity binding, validation decision, promotion decision, and destination policy before copying. It accepts only a verified declared APK for required local delivery or a declared AAB when the profile explicitly requests `APK_AND_AAB`. The deployment destination is `LOCAL_WINDOWS_FILESYSTEM`; any external deployment destination is rejected. Source, ZIP, and Git export is handled as `SOURCE_ACCESS_ONLY` and is never a deployment artifact; when source access is generated, `ProjectReadmeSynthesizer` (§76.3) synthesizes a verified `README.md` at the project root from the `AndroidToolchainLock` and completed `CapabilityRegistry` records.

### 83.2 Durable copy operation
The handler creates one `ExportVerificationRecord` before copying and records source identity, destination identity, source hash, destination hash, byte count, copy lifecycle, request fingerprint, idempotency key, and post-copy check. A copy that may have partially completed follows `UNKNOWN → RECONCILING`; destination inspection and source/destination identity and hash comparison must resolve it to `VERIFIED`, `FAILED`, or `BLOCKED` before retry. A hash or identity mismatch blocks completion and preserves the last-known-good artifact evidence. `reconciliationReference`, `failureEvidenceId`, and the corresponding external-effect or filesystem-inspection evidence are mandatory for the `UNKNOWN` and `RECONCILING` path.

The `artifact.export` response is `UIResponseEnvelope` carrying `ArtifactExportResponsePayload`, which embeds the complete `ExportVerificationRecord` (every §74.3 field, no projection subset) so the command boundary cannot expose less than the durable record (development plan M117, ADR-203). The request side is `ArtifactExportCommandPayload` with the policy-mandatory request fields; neither payload introduces a second export record type.

### 83.3 Runtime acceptance
Acceptance fixtures prove required APK delivery, optional declared AAB behavior, rejection of undeclared artifact kinds and external deployment destinations, source/destination hash equality, destination identity, interrupted-copy reconciliation, signing/validation/promotion linkage, and refusal to treat source access as deployment completion. Documentation certification proves contract presence only; runtime certification must execute the fixtures.

### 83.4 BuildReproducibilityChecker

`BuildReproducibilityChecker` executes under `ArtifactAuthority` (§83) to verify deterministic build outputs:
1. *Multi-pass clean build verification:* Executes two independent clean builds of the Android target from identical source revisions and toolchain locks in isolated build directories.
2. *Byte-for-byte and archive equality:* Compares APK and optional AAB zip central directory records, manifest timestamps, and uncompressed DEX/resource checksums, verifying that generated artifacts are byte-identical or signature-equivalent.
3. *Entropy and non-determinism detection:* Identifies non-deterministic build inputs (unpinned dependency dynamic versions, nondeterministic file iteration in packaging tasks, unstripped build machine absolute paths) and flags them as `REPRODUCIBILITY_DEFECT` findings before release promotion.

## 84. Platform Capability and Cross-Compilation Implementation Contract
**Implements:** build spec §79 and `CONTRACT.RUNTIME.PLATFORM_CAPABILITY`
**Canonical schema owner:** `CanonicalSchemaRegistry` in §36.1 (new entries below)
**Implementation owner:** `EnvironmentCapabilityPlanner` (classification), `ToolBroker`/`PolicyAuthority` (command admission), `EvidenceAuthority` (evidence binding and invalidation), and the completion evaluator (gate closure), with `WorkspaceLeaseManager` and `ToolSessionRegistry` as the lease and session substrate. This contract creates no new authority. The names `CrossCompilationAuthority` and `NativeRuntimeValidationAuthority` are fixed as decision points, not authorities: `CrossCompilationAuthority` is the cross-build admission decision point inside `ToolBroker`/`PolicyAuthority` fed by the `EnvironmentCapabilityPlanner` classification, and `NativeRuntimeValidationAuthority` is the native-runtime validation gate inside `EvidenceAuthority` and the completion evaluator.

### 84.1 Schemas

`EnvironmentCapabilityRecord` (registry: §36.1): `environment_id`, `host_platform`, `host_architecture`, `target_platform`, `target_architecture`, `shell`, `compiler`, `linker`, `sdk`, `runtime`, `build_tools`, `installer_tools`, `native_dependencies`, `tool_versions`, `environment_fingerprint`, `capability_results`, `repair_attempts`, `required_user_actions`, `runtime_validation_available`, `cross_compilation_available`, `evidence_ids`, `recorded_at`, `supersedes`. Host and target are explicit fields; nothing downstream may re-infer them.

`PlatformCapabilityEntry` (registry: §36.1): `capability_id`, `host_platform`, `expected_result: available | environment_dependent | unavailable_by_platform`, `required_toolchain`, `evidence_requirements`, `matrix_version`. The matrix is a prior for preflight; the observed record wins.

| capability_id | host_platform | expected_result | required_toolchain | evidence_requirements | matrix_version |
|---|---|---|---|---|---|
| `job_object_containment` | windows | environment_dependent | windows_sdk | windows_host_fingerprint, process_launch_observation_with_executable_path, job_object_assignment_before_resume, tree_termination_observation, orphaned_descendant_reconciliation | 1 |
| `path_length` | windows | environment_dependent | windows_sdk | windows_host_fingerprint, effective_max_path_length, long_path_policy_status | 1 |
| `security_software_interference` | windows | environment_dependent | windows_sdk | windows_host_fingerprint, real_time_scanning_detected, exclusion_status | 1 |
| `hypervisor_availability` | windows | environment_dependent | windows_sdk | windows_host_fingerprint, firmware_virtualization_enabled, hypervisor_platform_present, conflicting_consumers | 1 |
| `HOST_TOOL_OBSERVATION` | windows | available | none (host shell) | windows_host_fingerprint, tool_version_probe_output_per_required_tool | 1 |
| `ENVIRONMENT_REPAIR` | windows | environment_dependent | package_manager_or_sdk_manager | windows_host_fingerprint, repair_admission_decision, pre_and_post_repair_tool_version_probe | 1 |
| `WINDOWS_HOST_TOOLCHAIN` | windows | environment_dependent | dotnet_sdk, windows_app_sdk, msbuild, rust_toolchain_x64 | windows_host_fingerprint, toolchain_version_probe, target_build_observation | 1 |
| `WINDOWS_NATIVE_EXECUTION` | windows | environment_dependent | windows_sdk, validation_environment_lease | windows_host_fingerprint, validation_environment_lease_id, process_launch_observation_with_executable_path | 1 |
| `ANDROID_BUILD_TOOLCHAIN` | windows | environment_dependent | jdk, gradle, android_sdk, platform_tools | windows_host_fingerprint, android_toolchain_manifest_lock, gradle_build_observation | 1 |
| `ANDROID_EMULATOR_EXECUTION` | windows | environment_dependent | android_emulator, hypervisor_acceleration | windows_host_fingerprint, hypervisor_availability_result, emulator_boot_observation_with_session_id | 1 |
| `DESIGN_IMPORT` | windows | environment_dependent | figma_access_token_or_local_design_file | windows_host_fingerprint, figma_connectivity_or_file_presence, design_token_extraction_observation, compose_translation_fidelity_observation | 1 |
| `ANDROID_SOURCE_ENGINEERING` | windows | environment_dependent | jdk, gradle, android_sdk | windows_host_fingerprint, android_toolchain_manifest_lock, source_code_generation_observation | 1 |
| `ANDROID_BUILD` | windows | environment_dependent | jdk, gradle, android_sdk, platform_tools | windows_host_fingerprint, gradle_build_observation, apk_production_observation | 1 |
| `ANDROID_INSTALL_LAUNCH` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, apk_install_observation, app_launch_observation | 1 |
| `ANDROID_UI_OBSERVATION` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, ui_hierarchy_capture_observation | 1 |
| `ANDROID_INTERACTION_EXECUTION` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, input_injection_observation | 1 |
| `ANDROID_LOGCAT_DIAGNOSTICS` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, logcat_capture_observation | 1 |
| `ANDROID_VISUAL_VALIDATION` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, screenshot_capture_observation, design_comparison_observation | 1 |
| `ANDROID_ACCESSIBILITY_VALIDATION` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, accessibility_scan_observation | 1 |
| `ANDROID_PERFORMANCE_VALIDATION` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, startup_timing_observation, memory_profile_observation, frame_timing_observation | 1 |
| `ANDROID_BACKGROUND_EXECUTION` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, workmanager_execution_observation | 1 |
| `ANDROID_NATIVE_DEVICE_CAPABILITIES` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, camera_observation, ble_observation, nfc_observation, location_observation | 1 |
| `ANDROID_NETWORK_INTEGRATION` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, api_integration_observation | 1 |
| `ANDROID_AUTHENTICATION` | windows | environment_dependent | android_emulator | windows_host_fingerprint, emulator_boot_observation, auth_flow_observation | 1 |
| `ANDROID_PACKAGING` | windows | environment_dependent | jdk, gradle, android_sdk, platform_tools | windows_host_fingerprint, bundle_production_observation, asset_pack_observation | 1 |
| `ANDROID_ARTIFACT_INSPECTION` | windows | environment_dependent | jdk, gradle, android_sdk, platform_tools | windows_host_fingerprint, apk_inspection_observation, aab_inspection_observation | 1 |
| `ANDROID_SIGNING_INSPECTION` | windows | environment_dependent | jdk, gradle, android_sdk, platform_tools | windows_host_fingerprint, signing_config_observation, certificate_fingerprint_observation | 1 |
| `ANDROID_RELEASE_VALIDATION` | windows | environment_dependent | jdk, gradle, android_sdk, platform_tools | windows_host_fingerprint, lint_observation, quality_gate_observation, performance_gate_observation | 1 |

The twenty-four upper-case rows are the closed skill capability-id vocabulary of build spec §79.7: they are the only ids a `SkillPackage.requiredCapabilities` may name, and each is classified per environment by `EnvironmentCapabilityPlanner` from the listed evidence, never asserted by a skill or a model. `HOST_TOOL_OBSERVATION` is the one capability whose expected result is `available`, because the preflight skill that produces every other classification must not be gated by a classification it has not yet produced.

Job Object containment is a Windows target-runtime facility already required by BS §79.3. Per CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, target_runtime_validation is USER_REQUIRED absent a Windows observation.

Path length capability is a Windows target-runtime facility already required by BS §79.3. Per CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, target_runtime_validation is USER_REQUIRED absent a Windows observation.

Security-software interference is a Windows target-runtime facility already required by BS §79.3. Per CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, target_runtime_validation is USER_REQUIRED absent a Windows observation.

Hypervisor availability is a Windows target-runtime facility already required by BS §79.3. Per CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, target_runtime_validation is USER_REQUIRED absent a Windows observation.

`ValidationEnvironment` (registry: §36.1): `environment_id`, `platform`, `architecture`, `toolchain`, `runtime`, `available_tools`, `available_devices`, `isolation_profile`, `network_policy`, `fingerprint`, `health`, `lease_id`, `reserved_by_task`, `acquired_at`, `released_at`.

`BuildGateRecord` (registry: §36.1): `gate_id`, `stage: compile | target_build | bundle | artifact_inspection | install | launch | runtime_validation | platform_specific_validation | recovery_validation | certification`, `platform`, `environment_id`, `revision`, `command_or_operation_ref`, `evidence_ids`, `result: VERIFIED | UNVERIFIED | UNAVAILABLE | USER_REQUIRED | FAILED`, `recorded_at`.

> **Schema projection:** `BuildGateRecord` is defined in `nirman-schemas.md` §2.112. Owner: TA §84.1.

`WorkerContract` extension (canonical owner: the `WorkerContract` entry in §36.1): adds `required_host_platforms`, `required_target_platforms`, `required_architectures`, `required_capabilities`, `required_skills`, `required_toolchain`, `required_validation_environment`, `cross_compilation_allowed`, `native_execution_required`, `evidence_requirements`. The scheduler, not the worker, refuses a worker whose fields are not satisfied by the current `EnvironmentCapabilityRecord`.

### 84.2 Persistence and invalidation

Records persist in the SQLite execution ledger through the storage authority with the standard atomic-write, migration, backup, and rollback rules (§36.1). `EnvironmentCapabilityRecord` and `BuildGateRecord` are revision- and fingerprint-bound: any change to source revision, toolchain identity, environment fingerprint, target platform, isolation profile, or policy version invalidates dependent `BuildGateRecord` results and any `ValidationResult`, `CertificationDecision`, or completion claim that consumed them, through the existing evidence dependency graph (TA §23, BS §5.7.4). `PlatformCapabilityEntry` rows are versioned; a matrix version change re-opens preflight classification without invalidating observed records.

### 84.3 Resolution and gates

`TargetPlatformResolver` (module: §58.1) resolves the declared target for a task from the task contract and the immutable packaging/profile declarations, records host and target in the `EnvironmentCapabilityRecord`, and rejects a task whose target is not a declared target of the product scope (Android for generated applications; the Windows desktop host for Nirman itself).

`PlatformCapabilityRegistry` (module: §58.1) serves the BS §79.3 matrix to the planner and reports `environment_dependent` cells for preflight classification.

The cross-build admission decision point (`CrossCompilationAuthority`) evaluates, before a target-build command executes: the command's declared operation (`TARGET_BUILD` versus `RUNTIME_VALIDATION`), the observed toolchain, and the classification of the required capabilities. A `TARGET_BUILD` may be admitted on a proven toolchain. A command or claim that implies runtime validation is not admitted on a non-matching host; it is re-routed to the BS §79.11 blocked state.

The native-runtime validation gate (`NativeRuntimeValidationAuthority`) admits a validation task only when a matching `ValidationEnvironment` lease exists, and closes only when the declared `evidence_requirements` are satisfied by bound observations. It reports `AVAILABLE`, `REPAIRABLE`, `USER_REQUIRED`, or `UNAVAILABLE` to the task graph and never reports a simulated pass.

Android toolchain preflight (TA §49) continues to own Android build and device capability. This contract governs the host/target dimensions §49 does not, and the two record sets cross-reference through `environment_id`.

### 84.4 Failure and recovery

Loss of a `ValidationEnvironment` mid-task invalidates its in-flight validation evidence, fences the lease, and moves the node to the BS §79.11 blocked state with the resume condition "matching validation environment available." Toolchain or fingerprint drift detected by preflight invalidates dependent records and re-opens the affected gates without touching unaffected host-platform evidence. Repairs run only through the normal policy/transaction path; a failed repair records a `repair_attempt` and the classification remains truthful.

### 84.5 Runtime acceptance

`TEST-PLAT-001` (evidence `EV-PLAT-001`) implements the BS §79.13 fixtures A–D and MUST additionally prove: the planner emits the extended traceability chain with the environment-requirement and capability-resolution edges populated; the target-mismatch guard rejects a runtime-validation claim made without a leased matching validation environment before execution (the host is always Windows; the fixture varies the validation environment, never the host operating system); worker scheduling honors the `WorkerContract` platform fields; lease loss fences in-flight validation; and a matrix version change re-runs preflight without invalidating unrelated observed records. Documentation certification proves only that these contracts and fixture declarations exist; runtime certification must execute the fixtures.

---

## 85. Content Intelligence Implementation Contract

**Implements:** build spec §81 and `CONTRACT.RUNTIME.CONTENT_INTELLIGENCE`

### 85.1 Schemas

Canonical schemas: `Content`, `ContentRevision`, `ContentRevisionDraft`, `ContentMutation`, `ContentValidationResult`, `ContentPropagationPlan`, `TerminologyProfile`, `ContentEvidence`, `ContentDependency`.

> **Schema projection:** `Content` is defined in `nirman-schemas.md` §1.66. Owner: BS §81.1.

`Content` is the persisted logical content resource; `currentRevisionId` names the admitted `ContentRevision` that is current for the project revision.

> **Schema projection:** `ContentRevision` is defined in `nirman-schemas.md` §1.67. Owner: BS §81.1.

> **Schema projection:** `ContentDependency` is defined in `nirman-schemas.md` §1.68. Owner: BS §81.1.

`ContentMutation` is the proposal object a `ContentWorker` submits; it becomes a `ContentRevision` only when `ContentTransactionCoordinator` admits it through `ContentValidator` and `ContentAuthority` inside the parent `ConstructionTransaction`.

> **Schema projection:** `ContentRevisionDraft` is defined in `nirman-schemas.md` §2.76. Owner: TA §85.1.

> **Schema projection:** `ContentMutation` is defined in `nirman-schemas.md` §2.77. Owner: TA §85.1.

> **Schema projection:** `ContentValidationResult` is defined in `nirman-schemas.md` §2.78. Owner: TA §85.1.

> **Schema projection:** `ContentPropagationPlan` is defined in `nirman-schemas.md` §2.79. Owner: TA §85.1.

> **Schema projection:** `TerminologyProfile` is defined in `nirman-schemas.md` §2.80. Owner: TA §85.1.

> **Schema projection:** `ContentEvidence` is defined in `nirman-schemas.md` §2.81. Owner: TA §85.1.

`ContentMutation.proposedContentRevision` is a `ContentRevisionDraft` — proposed values only, with no `contentRevisionId`, `transactionId`, `validationStatus`, `approvalState`, or `sourceEvidenceIds`; a mutation carrying an already-admitted `ContentRevision` or any of those authoritative fields is rejected, so a worker cannot smuggle authoritative state into a proposal. `ContentAuthority` mints the `ContentRevision` from the draft at admission. `ContentMutation.baseProjectRevision` must equal the current project revision at admission or the mutation is rejected as stale (M120 fixture I). `ContentValidationResult.checks` covers the validation areas of BS §81.2. `ContentPropagationPlan` is the materialized result of the `ImpactGraph` traversal of §85.4 and BS §81.3. `ContentEvidence` is an evidence view owned by `EvidenceAuthority`: `freshness` and `invalidationState` are derived from the `ImpactGraph`, never asserted by a content worker.

### 85.2 Runtime authorities and roles

The Content Intelligence runtime comprises deterministic authorities, coordinators, and proposal workers:
- `ContentAuthority`: authoritative owner of content state, revision identity, content admission, and lifecycle transitions. ContentAuthority owns admission and lifecycle, not storage.
- `ContentWorker`: proposal only; reads requirements and context, generates `ContentMutation` proposals, cannot directly write canonical state, and cannot mark content complete.
- `ContentTransactionCoordinator`: coordinates content mutations and applies them through the existing `ConstructionTransaction` path.
- `ContentStore`: canonical persistence implementation for Content records; `ContentAuthority` remains authoritative for state admission and lifecycle; durable SQLite persistence; atomically stores content revisions with respect to parent construction transactions, surviving restart and compaction.
- `ContentValidator`: validation only; verifies terminology consistency, locale completeness, accessibility suitability, placeholder preservation, and interpolation correctness.

No Content Worker (§6.5; ADR-227) may directly mark content complete. `ContentWorker` is its runtime component and proposes mutations; `ContentTransactionCoordinator` validates and admits the mutation through `ContentValidator` and `ContentAuthority`; the transaction commits; and `EvidenceAuthority` generates authoritative evidence.

### 85.3 Persistence and retention

`ContentStore` persists content revisions durably in SQLite. Content records survive UI restart, supervisor restart, and context compaction. Content is retained for the lifetime of the project revision it belongs to, and is garbage-collected only when the referenced project revision is garbage-collected.

`ContentStore` MUST participate in the existing checkpoint/recovery protocol. Content mutations MUST be atomic with respect to the parent `ConstructionTransaction`.

### 85.4 Failure, dependency graph, and recovery

Content Worker failure rolls back the partial `ConstructionTransaction` and records a `failureEvidenceId`. Content recovery MUST NOT produce partial or inconsistent content state. Content recovery MUST NOT bypass validation.

Content dependencies are modeled as typed edges into the project `ImpactGraph`:
```text
ContentRevision
      ↓
ContentDependency*
      ↓
ImpactGraph
      ↓
affected UI / locale / accessibility / preview / tests / evidence
```

When any upstream dependency in the `ImpactGraph` mutates (requirements, brand assets, accessibility semantics, navigation, API terminology, feature flags, permissions, legal rules, locale catalogs), the `ImpactGraph` propagates invalidation to all dependent `ContentRevision`s, compiled resources, preview surfaces, and dependent evidence artifacts. Dependent evidence is invalidated by `EvidenceAuthority`, and the completion evaluator forbids completion until all traversed dependents pass revalidation.

### 85.5 Boundary with Android locale resources and regression localization

Content Intelligence owns content authoring, terminology, tone, brand voice, UX copy, accessibility copy, content consistency, and orchestration of translated content across `supportedLocales`. Android locale-resource mechanics (resource qualifiers, string-resource compilation, runtime locale fallback) belong to the Android code-intelligence layer of §47 and the build pipeline; `ContentRevision`s are written into those resources through the mutation broker and never bypass them.

`CONTRACT.RUNTIME.LOCALIZATION` is the regression-localization service of §63 (build spec §62) and carries no internationalization semantics. Content Intelligence meets it in exactly one place: a content regression detected by `ContentValidationResult` is localized to its causing `ContentMutation` by the §63 pipeline (impact graph, then signature match, then bisection) so that repair is cause-scoped (`CLAUSE.LOCALIZE.CAUSE_SCOPE`).

### 85.6 Propagation

Content impact analysis MUST identify affected UI surfaces, locales, accessibility labels, resources, preview surfaces, tests, and artifacts.

### 85.7 Acceptance

Fixture `TEST-CONTENT-001` proves content creation, propagation, terminology consistency, localization, accessibility, rollback, and evidence invalidation.

---

## 86. Conversation Context Implementation Contract

**Implements:** build spec §82 and `CONTRACT.RUNTIME.CONVERSATION_CONTEXT`

### 86.1 Schemas

Canonical schemas: `Conversation`, `ConversationMessage`, `ConversationAttachment`, `ConversationRequirement`, `ConversationDecision`, `ConversationSuggestion`, `ConversationTaskLink`, `ConversationRequirementIndex`, `ConversationDecisionIndex`, `ConversationRebaseRecord`.

> **Schema projection:** `Conversation` is defined in `nirman-schemas.md` §1.69. Owner: BS §82.

> **Schema projection:** `ConversationMessage` is defined in `nirman-schemas.md` §2.82. Owner: TA §86.1.

> **Schema projection:** `ConversationRequirement` is defined in `nirman-schemas.md` §1.70. Owner: BS §82.

> **Schema projection:** `ConversationDecision` is defined in `nirman-schemas.md` §1.71. Owner: BS §82.

> **Schema projection:** `ConversationSuggestion` is defined in `nirman-schemas.md` §1.72. Owner: BS §82.

> **Schema projection:** `ConversationAttachment` is defined in `nirman-schemas.md` §1.73. Owner: BS §82.

> **Schema projection:** `ConversationTaskLink` is defined in `nirman-schemas.md` §2.83. Owner: TA §86.1.

> **Schema projection:** `ConversationRequirementIndex` is defined in `nirman-schemas.md` §2.84. Owner: TA §86.1.

> **Schema projection:** `ConversationDecisionIndex` is defined in `nirman-schemas.md` §2.85. Owner: TA §86.1.

> **Schema projection:** `ConversationRebaseRecord` is defined in `nirman-schemas.md` §2.86. Owner: TA §86.1.

`ConversationDecision` is a proposal envelope owned by Conversation, not a canonical locked decision. `ConversationStore` may move it from `PROPOSED` to `SUBMITTED`; only a `ConstraintRegistry` admission result may set it to `ADMITTED` and populate `canonicalDecisionId`. Rejected or withdrawn proposals create no canonical identity, and only the canonical `LockedDecision.state` and supersession links determine whether a settled decision is current.

`ConversationRequirement` is a proposal envelope owned by Conversation, not a canonical construction requirement. `ConversationStore` may move it from `PROPOSED` to `SUBMITTED`; only a `ConstraintRegistry` admission result may set it to `ADMITTED` and populate `canonicalRequirementId`. A rejected or withdrawn proposal never enters `AndroidConstructionContract.requirementIds`. After admission, all settled-requirement reads resolve through `ConstructionRequirement`; the proposal remains only as source provenance.

`ConversationMessage.contentReference` points at the durable message body; `sourceEventId` binds the message to the control-plane event that produced it. `ConversationTaskLink` is the durable form of `taskLineage`. `ConversationRequirementIndex` and `ConversationDecisionIndex` reference canonical `ConstraintRegistry`/`MemoryStore` records by `canonicalRequirementId`/`canonicalDecisionId` and carry no second copy of their content. `sourceEvidenceIds` reference `EvidenceRecord` identifiers owned by `EvidenceAuthority`. A `ConversationRebaseRecord` is written for every `RECONCILE/REBASE` and every `USER_REQUIRED` outcome of §86.5, so a rebase is auditable rather than silent.

### 86.2 Persistence and resolver

`ConversationStore` persists the aggregate durably in SQLite. `ConversationContinuationResolver` reconstructs continuation state from durable records.

The resolver MUST consume project revision, goal, requirements, decisions, suggestion outcomes, attachments, task lineage, and valid evidence.

`ConversationStore` MUST participate in the existing checkpoint/recovery protocol. Conversation records survive UI restart, supervisor restart, and context compaction.

A Continue resolution that changes `conversationRevision`, project binding (`expectedProjectRevision`, `projectRevisionId`), rebase state (`ConversationRebaseRecord`), or task lineage (`ConversationTaskLink`) MUST commit those related records atomically in one SQLite transaction. A partially committed Continue resolution is invalid and MUST be recovered or rolled back before execution resumes: on restart, `ConversationContinuationResolver` verifies that the latest `conversationRevision`, its rebase record, and its task links agree, and rolls back to the last coherent revision when they do not.

Attachment lifecycle and security: `ConversationAttachment` lifecycle transitions from `ACTIVE` to `DELETED` (`ACTIVE → DELETED`). Attachments enforce `contentHash`, `mimeType`, `sizeBytes`, `storageOwner`, `privacyClassification`, `deletionStatus`, `projectIsolation`, `providerTransmissionPolicy`, and `revisionBinding`. Attachments are strictly isolated per project. Provider transmission MUST delegate to existing `ContextGovernance` (§79.1) and `ProviderContextEnvelope.transmissionDecision` (§38; build spec §5.7.8), ADR-199; Conversation must not create a second policy authority. Private, high-risk, or oversized attachments are sanitized, capped, or redacted by `ContextGovernance` before model context inclusion.

### 86.3 Failure and recovery

ConversationStore failure triggers reconciliation. Conversation continuation MUST reject stale or contradictory state and trigger reconciliation. Attachment recovery MUST preserve identity and provenance. Compaction failure MUST surface a `USER_REQUIRED` node rather than silently dropping requirements or decisions.

### 86.4 Boundary with MEMORY + CONTEXT + BACKGROUND_CONTINUITY

Conversation does NOT create a second memory, task, or project authority. Conversation owns conversational lineage only (messages, attachments, decisions, suggestions, active goal, task lineage).

`CONTRACT.RUNTIME.MEMORY` remains authoritative for retained semantic memory records. `CONTRACT.RUNTIME.CONTEXT` remains authoritative for reconstruction policy and context capacity governance. `CONTRACT.RUNTIME.BACKGROUND_CONTINUITY` remains authoritative for background execution and interruption/resume state. Task and project state remain authoritative for execution state.

Storage authority separation:
- `Conversation`: durable conversation lineage and conversation-owned records (messages, attachments, suggestions, revision bindings).
- `MemoryStore` (§59.1) for semantic memory, `ContextOrchestrator` (§59.1) for assembled context, and `ConstraintRegistry` (§59.1) for settled requirements and locked decisions: the canonical semantic, context, and requirement/decision authorities. No `ContextStore`, `RequirementStore`, or `DecisionStore` component exists.
Conversation references and indexes canonical requirements and decisions via typed lineage indices (`ConversationRequirementIndex`, `ConversationDecisionIndex`); `ConversationStore` reads from `MemoryStore`, `ConstraintRegistry`, and the `ContextOrchestrator` output and does not duplicate their canonical data or override their decisions.

### 86.5 Concurrency and revision consistency state machine

Conversation consistency is governed by the triple revision invariant:

```text
ConversationRevision
        ↕
ProjectRevision
        ↕
TaskRevision
```

When `Continue` is invoked, the `ConversationContinuationResolver` (the `ConversationResolver` of build spec §82.1; one component) evaluates `Conversation.expectedProjectRevision` against `Project.currentRevision`:
`Project.currentRevision` in this state machine is exactly the ADR-242 derived projection (§45.3):
the `projectRevisionAfter` of the latest committed ConstructionTransaction in authoritative commit-event
sequence order (TA §45.2) for the project.
1. `MATCH` (`Conversation.expectedProjectRevision == Project.currentRevision`):
   State transitions to `CONTINUE`. Next task graph is synthesized from current conversation state.
2. `MISMATCH` (`Conversation.expectedProjectRevision != Project.currentRevision`):
   State transitions to `RECONCILE / REBASE`. The resolver inspects intervening `ConstructionTransaction`s and `ChangeImpactReport`s. If non-conflicting (orthogonal worker patches, independent asset build, background validation), the resolver rebases `expectedProjectRevision` to `Project.currentRevision`, records `ConversationRebaseRecord`, increments `ConversationRevision`, and transitions to `CONTINUE`.
3. `UNRESOLVABLE`:
   If intervening mutations conflict with conversation requirements or modify user-locked decisions, state transitions to `USER_REQUIRED`. Autonomous execution halts, exposing a structured diff of the revision discrepancy to the user. Continuing work without explicit user resolution or silently resurrecting stale intent is strictly forbidden.

The Continue state machine above remains authoritative for the Conversation ↔ Project consistency check. `TaskRevision` (ADR-248) is an additional task-lineage consistency check performed for affected task execution and reconstruction: a task-revision mismatch enters reconciliation/replan for that task; it is not itself a `USER_REQUIRED` verdict.

### 86.6 Integration with Background Continuity

Conversation continuation integrates directly with `CONTRACT.RUNTIME.BACKGROUND_CONTINUITY` (§82, ADR-202):

```text
Conversation continuation
+
Background continuity
=
resume semantics
```

When the WinUI presentation client reconnects or the host wakes from suspension, `BackgroundContinuity` restores process supervision and watchdog health, while `ConversationContinuationResolver` resolves conversational intent against the current project revision. The two combine to determine whether background work continues seamlessly (`MATCH` or non-conflicting `MISMATCH`) or halts safely for user guidance (`UNRESOLVABLE`).

`Continue` MUST resolve a durable conversation before task creation. It MUST reject stale or contradictory state and trigger reconciliation when required. The user-facing entry is the registered `conversation.continue` command kind (build spec §76.1) dispatched through the §81.2 command-to-domain wiring; background resumption reaches the same `ConversationContinuationResolver` use case without a UI command, and both write the same `ConversationRebaseRecord` and projection.

### 86.7 Acceptance

`TEST-CONV-001` proves UI restart, supervisor restart, compaction, accepted/rejected suggestions, attachment continuity, goal continuity, revision continuity, and task lineage.

---

## 87. Change Intelligence Implementation Contract

**Implements:** build spec §83 and `CONTRACT.RUNTIME.CHANGE_INTELLIGENCE`

### 87.1 Schema and atomic reporting unit

The canonical atomic reporting unit is:

```text
MutationReportUnit = committed ConstructionTransaction
```

Explicit lifecycle invariant:
```text
Exactly one ChangeReportRecord exists per committed ConstructionTransaction.
Its status may transition:
INCOMPLETE → COMPLETE
INCOMPLETE → UNRESOLVED

A COMPLETE ChangeImpactReport is immutable.
```

Status transitions and rejected modifications:
Permitted transitions are strictly `INCOMPLETE → COMPLETE` (upon successful projection or recovery) and `INCOMPLETE → UNRESOLVED` (when authoritative inputs cannot be reconciled). Any attempt to transition `COMPLETE → INCOMPLETE` or mutate a complete report (`COMPLETE → modified`) MUST be rejected.

Exactly one `ChangeReportRecord` is produced per committed `ConstructionTransaction`. Individual file writes, scratch edits, intermediate worker patches, and rollbacks within a transaction do NOT produce isolated partial reports. Aborted or rolled-back transactions record recovery/failure evidence under `ConstructionTransaction` and do not produce completed change impact reports.

Canonical schemas: `ChangeReportRecord`, `ChangeImpactReport`.

> **Schema projection:** `ChangeReportRecord` is defined in `nirman-schemas.md` §1.74. Owner: BS §83.1.

`projectRevisionAfter` is the authoritative committed project revision represented by this `ChangeReportRecord`; it is taken from the committed `ConstructionTransaction` and must equal the nested report's `projectRevisionAfter`. `status` is the authoritative lifecycle state of the reporting unit.

> **Schema projection:** `ChangeImpactReport` is defined in `nirman-schemas.md` §1.75. Owner: BS §83.1.

Field provenance:
- `reportId`: assigned by ChangeIntelligenceProjector
- `transactionId`: from ConstructionTransaction (authoritative)
- `projectionStatus`: COMPLETE; immutable and informational — `ChangeReportRecord.status` is the authoritative lifecycle state
- `projectRevisionBefore` / `projectRevisionAfter`: from ConstructionTransaction (authoritative)
- `requirementIds`: from the requirement/goal/directive that caused the mutation (authoritative)
- `changed`: from ConstructionTransaction mutation record (authoritative)
- `causeType` / `causeId`: the single authoritative causal source — requirement, goal, directive, repair cause, or approved action — from the ConstructionTransaction's recorded cause (authoritative)
- `why`: projection rendered from `causeType`/`causeId`; carries no independent content
- `files`: from ConstructionTransaction changed paths (authoritative)
- `runtimeEffects`: from impact analysis against affected surface graph (authoritative)
- `testsAffected` / `testsRun`: from ValidationResult (authoritative)
- `previewAffected`: from PreviewRevision currentness check (authoritative)
- `evidenceInvalidated` / `evidenceRetained`: from EvidenceAuthority (authoritative)
- `verified`: from authoritative verification results only (authoritative)
- `unresolvedIssues`: from validation/preview/evidence reconciliation
- `recoveryActions`: from RecoveryAuthority (authoritative)
- `recommendedNextStep`: advisory projection from impact + validation + recovery analysis
- `recommendationSource`: the analysis component that generated the recommendation
- `recommendationBasis`: the evidence basis for the recommendation
- `requiredAuthority`: the authority that must approve/act on the recommendation
- `generatedAt`: timestamp from projector
- `projectionVersion`: projection schema version from projector

### 87.2 Generation and deterministic precedence

`ChangeIntelligenceProjector` derives the report from `ConstructionTransaction`, impact analysis, validation results, `PreviewRevision`, and `EvidenceAuthority` state.

Deterministic precedence when sources disagree:
1. ConstructionTransaction (authoritative for mutation identity, files, revision)
2. ImpactAnalysis (authoritative for affected surface graph)
3. ValidationResult (authoritative for verification/test results)
4. PreviewRevision (authoritative for preview impact/currentness)
5. EvidenceAuthority (authoritative for evidence validity/invalidation)
6. RecoveryAuthority (authoritative for recovery actions)

### 87.3 Invalidation

The projector MUST expose source, asset, toolchain, preview, test, integration, and evidence invalidation resulting from the mutation.

### 87.4 Read-only projector boundary

`ChangeIntelligenceProjector` is a read-only projection component. It MUST NOT mutate project state, grant permissions, approve evidence, or mark completion. `RecoveryAuthority` is the authoritative source of recovery actions; the projector cannot manufacture them. No field in `ChangeImpactReport` may be independently invented by the projector. `recommendedNextStep` is strictly advisory.

### 87.5 Persistence and retention

A `ChangeReportRecord` and its associated `ChangeImpactReport` (when `status == COMPLETE`) are persisted durably by `ChangeIntelligenceStore` in SQLite by `recordId` + `transactionId` + project revision, survive restart, and are revision-addressable.

`ChangeIntelligenceStore` persists report records in the durable SQLite task/project ledger, keyed by `recordId` and `transactionId`; `transactionId` is unique, because exactly one `ChangeReportRecord` exists per committed `ConstructionTransaction` (§87.1, CLAUSE.CHANGE.EXACTLY_ONE_REPORT). Regeneration never creates a second record: it replaces the record's `report` with a new `ChangeImpactReport` carrying a higher `projectionVersion` for the same `transactionId` and source revision, and the superseded report is retained as an artifact-store copy referenced from the record's history. Reports survive UI and supervisor restart and remain addressable through the project revision history.

The `ChangeReportRecord` obligation is coupled to the durable commit through the existing transaction/outbox mechanism: the `ConstructionTransactionManager` writes the initial `ChangeReportRecord` obligation (`status: INCOMPLETE`, `report: null`) in the same SQLite transaction that commits the parent `ConstructionTransaction`, so a committed transaction and its initial record obligation become durable atomically. `transactionId` is unique in `ChangeIntelligenceStore`; a second record for the same `transactionId` is rejected at write.

### 87.6 Failure and reconstruction semantics

Projector failure MUST NOT fail or roll back the committed parent `ConstructionTransaction`.

When report projection encounters an error, missing input, or timeout:
```text
Mutation committed (ConstructionTransaction committed)
        ↓
Change report generation failed
        ↓
ChangeReportRecord created with status: INCOMPLETE (report: null)
        ↓
Recovery job reconstructs report
        ↓
ChangeReportRecord updated with status: COMPLETE (report: ChangeImpactReport)
```

1. The parent `ConstructionTransaction` remains durably committed in SQLite.
2. A `ChangeReportRecord` is written to `ChangeIntelligenceStore` with `status: INCOMPLETE`, `report: null`, and failure diagnostics.
3. `RecoveryAuthority` schedules an asynchronous `ChangeIntelligenceRecoveryJob` to reconstruct the complete `ChangeImpactReport` from durable transaction, impact analysis, preview, and validation records. The `ChangeIntelligenceRecoveryJob` is a `RecoveryAuthority`-owned reconstruction job — a durable, idempotent unit of recovery work keyed by `transactionId`, not a component, authority, or service with decision rights of its own. It commits only through `RecoveryAuthority` (§21 "Recovery authority").
4. The projector MUST NOT fabricate missing values. Missing transaction state, inconsistent revision identity, incomplete impact data, unavailable validation results, preview identity mismatch, or evidence state disagreement produces a typed incomplete report. If authoritative state cannot be reconciled, `ChangeReportRecord.status` is set to `UNRESOLVED` and cannot support completion.
5. Crash between parent commit and projection: because the initial record obligation commits atomically with the parent (§87.5), a crash at any later point leaves an `INCOMPLETE` record, never a missing one. After restart, `RecoveryAuthority` MUST additionally scan for any committed `ConstructionTransaction` lacking a `ChangeReportRecord` and create exactly one `INCOMPLETE` record for it idempotently (keyed by `transactionId`), then schedule reconstruction as in step 3. Duplicate records for the same transaction are forbidden; the scan is safe to repeat.

### 87.7 Presentation contract

The authoritative `ChangeImpactReport` projects into the WinUI 3 presentation client across Calm, Inspect, and Developer modes (§55, build spec §45.1):

```text
Change summary
Why
Files
Runtime impact
Tests
Preview
Evidence invalidated
Verified
Next action
```

The presentation client displays these structured dimensions with clickable file diffs, preview surface status, and evidence links. The client MUST NOT infer mutation facts from model prose.

### 87.8 Acceptance

`TEST-CHANGE-001` proves complete reports, revision binding, actual file lists, runtime impact, affected tests, preview impact, evidence invalidation, verification, and recommended next action.


## 88. Speculative Candidate Branching Runtime

**ContractId:** `CONTRACT.RUNTIME.SPECULATION`  
**Authoritative build-spec section:** §65  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §65. Extends §8 (Workspace Isolation and Reconciliation), §45 (Reducer, Event Store, and Transaction Manager), §58.7 (WorkspaceLeaseManager), §64 (Verification Orchestrator), and §59 (Memory/Context Runtime), which remain the authority on workspace isolation, event commitment, leases, verification, and memory writes. This section adds the runtime that produces competing candidates safely and selects one by evidence. It is entered only from the kernel transition `DECIDE branch -> SPECULATE` of §71.4 or the deliberation outcome `BRANCH` of §72.4; no other component may open a candidate branch.

### 88.1 Components

| Component | Responsibility |
|---|---|
| SpeculationAdmissionGate | Evaluates the §65.3 admission conditions: a declared uncertainty (`UncertaintyRegistry`, §58.12), admissible physical capacity for every additional candidate (`ResourceIntegrityAuthority`, §77, using `ResourceExecutionProfile` estimates from §69), and a shared objective validation metric; denies with a typed reason otherwise, and the caller executes a single approach |
| CandidateWorkspaceProvisioner | Creates one worker worktree per candidate (§8.1) from the same `parentRevision`, each under its own `WorkspaceLeaseManager` lease (§58.7); candidates never share a working tree and never write to the main workspace |
| CandidateRunner | Executes each candidate's approach inside its own workspace under the ordinary kernel authority path (§71.4); a candidate has no capability its parent task lacks |
| CandidateValidator | Runs the identical `validationPlan` against every candidate through the Verification Orchestrator (§64) and records one `VerificationRun` set per candidate |
| CandidateSelector | Applies the §65.4 rules: compares `comparableMetrics` produced by identical validation plans, marks exactly one `selectedAsWinner`, and on a tie or universal failure records the outcome and escalates through `DecisionNodeManager` (§58.12) instead of choosing |
| CandidateDiscarder | Removes losing candidate worktrees from the deliverable path and writes their failure signatures as `FAILURE` memory records (§59.5, §63.4); losing code never reaches an integration workspace |

### 88.2 Candidate branch schema

The runtime persists the build spec §65.2 `CandidateBranch` record without addition or omission. Every field is written by a deterministic component, never by model output:

> **Schema projection:** `CandidateBranch` is defined in `nirman-schemas.md` §1.26. Owner: BS §65.2.

`branchId` is issued by the reducer (§45.1) when the branch is opened; `parentRevision` is the committed revision the branch was created from; `isolatedWorkspace` is the absolute worktree path from the provisioner; `resourceRequirements` is the physical demand the admission gate reserved; `validationPlan` is the plan identity shared by every sibling; `comparableMetrics` is written only by `CandidateValidator`; `selectedAsWinner` is written only by `CandidateSelector`.

### 88.3 Speculation sequence

```text
1. Kernel or deliberation emits branch intent with the declared uncertainty and candidate approaches
2. SpeculationAdmissionGate admits or denies; denial -> single approach, reason recorded
3. CandidateWorkspaceProvisioner opens N worktrees from parentRevision, one lease each
4. CandidateRunner executes each approach under kernel authority (AUTHORIZE before EXECUTE)
5. CandidateValidator runs the identical validationPlan in every candidate workspace
6. CandidateSelector compares comparableMetrics
     one best-validated candidate -> selectedAsWinner = true; outcome validated
     tie or all failed            -> no winner; escalate via DecisionNodeManager
7. Winner is reconciled into the integration workspace through the §8.3 algorithm and committed by ConstructionTransactionManager (§45.3)
8. CandidateDiscarder abandons losers: worktree removed, failure signature retained, evidence retained
```

Steps 1–8 are kernel events in the event store (§45.2), so replay reconstructs which candidate produced every artifact.

### 88.4 Persistence

`CandidateBranch` records, per-candidate `VerificationRun` sets, and the selection decision are persisted in the session event store and evidence ledger (§23.3) keyed by `branchId` and `parentRevision`. Losing candidates' evidence and failure signatures are retained per §59.5; their worktrees are removed under the lease cleanup rules of §58.7. Retention follows the project's evidence retention policy; a candidate record is never deleted while any evidence cites its `branchId`.

### 88.5 Failure and recovery

| Failure | Behavior |
|---|---|
| Candidate workspace creation fails | Branch is `abandoned` before execution; remaining candidates continue only if the admission conditions still hold, otherwise the run falls back to a single approach |
| Candidate lease expires or process dies | The candidate is `failed`; its worktree is recovered under §58.7 rules and never merged |
| Validation cannot run identically for every candidate | Selection is refused; outcome recorded and escalated (§65.4) |
| Runtime restart mid-speculation | Replay restores every `CandidateBranch` from events; candidates in `pending` resume or are marked `failed` from lease state; no candidate is re-created against a different `parentRevision` |
| Winner reconciliation conflicts with the main workspace | Handled by the §8.3 reconciliation algorithm; the winner is not committed until the commit barrier (§45.4) passes |

`CandidateDiscarder` and every commit and completion gate enforce `CLAUSE.SPECULATE.DISCARD_HYGIENE` (build spec §65.5).

### 88.6 Architecture tests

`TEST-SPEC-001` proves, against an Android fixture with two comparable approaches: parallel candidates leave the primary workspace untouched; the winner is selected only from identical validation evidence; a tie and a universal failure both escalate instead of selecting; a losing candidate's code is absent from the promoted artifact while its failure signature is present in memory; a restart during speculation replays every candidate to a consistent state; and an admission denial results in exactly one executed approach. Its evidence artifact is `EV-SPEC-001`, the milestone-level (M92) constituent of the capability evidence `EV-VER-001` that `CAP.ANDROID.QUALITY_GATE` resolves through build spec §67.15; `EV-VER-001` is not complete for that capability while `EV-SPEC-001` is missing.

---

## References

[1]: https://learn.microsoft.com/en-us/windows/apps/winui/ "WinUI 3 Documentation"
[2]: https://sqlite.org/docs.html "SQLite Documentation"
[3]: https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects "Windows Job Objects"
[4]: https://git-scm.com/docs/git-worktree "Git Worktree Documentation"
[5]: https://developers.openai.com/api/reference/overview "OpenAI API Reference Overview"
[6]: https://platform.openai.com/docs/api-reference/responses/create "Responses API Create Reference"
[7]: https://platform.openai.com/docs/api-reference/chat/create "Chat Completions Create Reference"
