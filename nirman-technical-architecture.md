# Nirman Technical Architecture

## Implementation Blueprint for the Windows-First Desktop Application

**Document status:** Living implementation specification — accepted architecture
**Application:** Nirman  
**Scope:** Local-first autonomous application development with configurable cloud AI providers  
**Relationship to master specification:** This architecture document explains how to implement the behavior defined by the master product specification. It contains architecture and interfaces, not production source code.

**Canonical ownership:** The Build Spec owns product contracts, invariants, and capability/contract registries. The Technical Architecture owns implementation schemas, protocols, and module boundaries. The Development Plan owns sequencing, milestones, fixtures, and exit gates. The Decision Log owns accepted decisions, rationale, and supersession. The README is explanatory only. AGENTS defines agent operating constraints only. The verifier certifies documentation and semantic checks only; it is never a runtime authority.

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
│ Workspaces   │ │ Policies   │ │ Builds │ │ Cloud/Local AI  │
│ Worktrees    │ │ Sandboxes  │ │ Preview│ │ Models          │
└──────────────┘ └────────────┘ └────────┘ └─────────────────┘
```

The control plane should communicate with the user interface through a local authenticated IPC channel. The production transport is named pipes. A loopback HTTP or WebSocket API may be used internally for development and debugging, but it must require a per-installation secret or operating-system authenticated channel and must not be used for the production SupervisorConnection. The interface must not be able to impersonate another project or bypass task policies by modifying client-side state.

---

## 3. Process Model

Nirman is one user-facing Windows application implemented by two cooperating processes. This is an implementation boundary, not a product boundary.

```
                    ONE NIRMAN PRODUCT
                           │
              ┌────────────┴────────────┐
              │                         │
        Nirman.exe              NirmanSupervisor.exe
        visible UI                headless runtime
              │                         │
              └──── authenticated IPC ──┘
```

`Nirman.exe` is the visible client. `NirmanSupervisor.exe` is the durable local runtime authority.

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

Every worker runs as a child process or isolated runtime task with a declared role, model profile, workspace, permissions, limits, and task contract. A worker must not decide its own isolation profile or expand its own permissions.

A worker may use the provider router to call a model and the tool gateway to request filesystem, process, preview, browser, or external-tool actions. It cannot invoke the operating system directly outside those gateways.

### 3.4 Runtime processes

Development servers, test runners, package managers, emulators, browsers, and build commands are runtime processes. The process manager tracks each process tree and associates it with a task, worker, project, workspace, and resource profile.

The process manager must support cancellation of the whole process tree, not only the parent process. It must capture stdout and stderr separately, enforce output limits, and preserve the final diagnostic output when a process is terminated.

Job handles MUST be created with handle inheritance DISABLED. If a child inherits the handle, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE does not reap the tree when the parent exits, because an open handle keeps the job alive. Termination MUST NOT rely on parent-child process-tree walking alone. A grandchild assigned to its own nested job, or reparented after its parent exits, is missed. Assignment to the supervisor job at spawn is the only durable containment. Every spawned build, emulator, device, and package-manager process MUST be assigned to the job BEFORE it is resumed. The gradlew.bat → java.exe and Metro/Expo → node.exe shapes are the cases that leak. A leaked Gradle daemon holds file locks and corrupts the next run; supervisor restart MUST reconcile orphaned descendants from the ledger before starting new work.

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
Apply automatically under unattended policy
  ↓
Expose the selected strategy or escalate at a hard gate
```

Under the `Unattended / Full Autonomy` profile, the runtime must select and apply an eligible deterministic recovery strategy using failure classification, checkpoint validity, retry budgets, risk policy, and current evidence. The UI exposes the selected strategy but is not required for routine recovery. A user decision is required only when policy returns `USER_REQUIRED`, `BLOCKED`, or `ESCALATED`, or when a declared hard safety, credential, signing, destructive, or emulator gate is reached. A task should never resume from an unverified partial filesystem state. It should either continue from a validated checkpoint or create a recovery branch containing the partial state for inspection.

---

## 5. Task and Worker State Machines

### 5.1 Task state machine

```text
QUEUED → PLANNING → READY → RUNNING → VALIDATING → COMPLETED
                    │          │          │
                    │          │          ├── WAITING_APPROVAL
                    │          │          ├── WAITING_RESOURCE
                    │          │          ├── RECOVERING
                    │          │          └── CANCEL_REQUESTED
                    │          │
                    │          └── FAILED_RETRYABLE → RECOVERING
                    │
                    └── ESCALATED
```

Every transition should include a reason, actor, timestamp, task revision, and event ID. The transition function must reject invalid transitions, such as moving a cancelled task directly to completed without a new retry decision.

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

---

## 6. Inter-Worker Coordination Protocol

### 6.1 Task contracts

The orchestrator should assign each worker a task contract containing:

```text
TaskContract
- contractId
- parentTaskId
- workerRole
- objective
- acceptanceCriteria
- allowedPaths
- forbiddenPaths
- allowedTools
- deniedTools
- modelProfile
- resourceBudget
- inputReferences
- dependencyContracts
- expectedOutputSchema
- deadline
```

The worker must return a structured result matching the expected output schema. Free-form commentary may be included, but the orchestrator must not depend on parsing it to determine success.

### 6.2 Message envelope

```text
WorkerMessage
- messageId
- taskId
- contractId
- senderId
- recipientId or topic
- type
- correlationId
- sequence
- payload
- evidenceReferences
- acknowledgementRequired
- createdAt
- expiresAt
```

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
| UI Worker | Frontend screens, components, styling, interactions | Assigned workspace |
| Android Data and Integration Worker | Android data layer, persistence schemas, service integrations, business logic | Assigned workspace |
| Test and QA Worker | Tests, fixtures, regression checks | Test paths and approved commands |
| Debugging Worker | Failure diagnosis and repairs | Assigned repair paths |
| Security Worker | Security, permissions, secrets, dependencies | Read-only by default |
| Visual QA Worker | Browser/device visual and accessibility checks | Read-only |
| Performance Worker | Profiling, resource use, bottleneck and regression analysis | Read-only |
| Documentation Worker | Documentation, decisions, release notes | Documentation paths |
| Release Worker | Builds, packaging, checksums, release reports | Build and artifact paths |
| Reconciliation Worker | Conflict analysis and integration validation | No direct mutation until integration |

The orchestrator should select swarm size from task complexity, dependency coupling, changed-file boundaries, target platforms, interface agreements, expected validation cost, and available resources. It should prefer one worker for tightly coupled work, parallel read-only workers for exploration and review, and isolated write-capable workers only when file and interface boundaries are explicit.

For coupled work, the orchestrator must create an interface agreement before parallel implementation. The agreement may contain API shapes, shared types, route contracts, database schemas, event formats, design tokens, or artifact contracts. Workers validate against it before reconciliation.

Worker nesting is limited to two levels by default: the Primary Orchestrator may delegate to workers, and a worker may request one narrowly scoped diagnostic child. A child cannot create further workers, change the parent contract, expand permissions, or integrate changes. All worker handoffs remain attached to the parent task graph.

## 7. Scheduler and Background Execution

### 7.1 Scheduler responsibilities

The scheduler selects runnable tasks, reserves resources, launches workers, manages dependencies, handles approvals, records heartbeats, detects stale processes, and decides whether to retry or escalate failures.

A scheduler tick should be deterministic and idempotent. Running the same scheduling cycle twice must not launch duplicate workers for the same contract.

### 7.2 Resource-aware scheduling

The scheduler should calculate available CPU, memory, disk, provider concurrency, and workspace capacity before launching a worker. It should reduce concurrency under resource pressure and preserve resources for validation and recovery.

Across multiple projects, the scheduler must use fair-share scheduling. Each active project receives a minimum service opportunity, while explicit project priority, task urgency, validation deadlines, and resource eligibility influence the next choice. A single project or swarm must not starve other active projects. The scheduler should use weighted round-robin with aging so waiting tasks gradually gain priority and a task that repeatedly yields resources is not permanently penalized.

Initial defaults should be configurable and conservative:

| Resource | Default |
|---|---:|
| Write-capable workers per task | 3 |
| Read-only workers per task | 5 |
| Global active workers | 8 or available-resource policy |
| Worker heartbeat | 10 seconds |
| Worker stale threshold | 60 seconds |
| Default task time policy | No fixed completion lock; adaptive monitoring and optional hard safety cap |
| Default task disk quota | Android-profile-based; emulator, device, build, cache, and checkpoint storage are computed together |
| Default repair strategy changes | 3 |

### 7.3 Background approval notifications

An approval request should be durable. If the application is minimized, the control plane may issue a Windows notification. If the application is closed, the request should appear when Nirman reopens.

Approval requests must expire. The user can approve once, approve a matching rule for the session, deny once, deny the worker, or pause the entire task. The approval record must include the exact command, path, worker, workspace, policy, and task state.

### 7.4 Scheduled tasks

Scheduled tasks should be implemented only after reliable background execution exists. A schedule record should contain a local cron-like expression or interval, project ID, task prompt, allowed mode, maximum budget, notification policy, and whether approval is required.

Scheduled tasks should never automatically publish, push, spend money, or use personal credentials. They may run local checks, update documentation, refresh dependencies in a restricted workspace, or generate reports according to user policy.

### 7.5 Reboot, sleep, and notification resilience

The stable supervisor should register a per-user startup entry for projects with active unattended tasks. It must detect boot, login, suspend, resume, hibernate, and shutdown transitions and write those transitions to the task event ledger.

During active Goal Mode work, the runtime should request an operating-system execution power policy where supported so the machine does not enter sleep while a build, test, emulator, or provider operation is active. The user must see this setting and may disable it. If sleep or hibernation still occurs, the supervisor must mark active processes stale, revalidate provider requests, restart eligible local processes, restore ports and emulator state where possible, and resume from the last validated checkpoint.

Approval and warning events must have multiple delivery paths: in-app queue, tray badge, operating-system notification, task history, and startup summary after reboot. If notifications are suppressed, the task must not remain invisibly parked; the control plane should record the pending decision and show it on the next connection. Unattended profiles should avoid routine approval states by policy, while genuine hard-gated decisions remain visible and durable.

---

## 8. Workspace Isolation and Reconciliation

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

Nirman should implement at least four profiles:

| Profile | Characteristics |
|---|---|
| Trusted local | Fast, user process, workspace and command policies |
| Restricted process | Restricted token, process-tree control, workspace paths, environment filtering |
| High-risk restricted process | Strongest native boundary for untrusted repositories and risky dependencies |
| Disposable/Isolated | Temporary, fully isolated environment for untrusted code execution; destroyed after use |

The interface should explain when a requested operation requires a stronger profile. A worker must not be able to switch itself to a weaker profile.

### 9.2 Windows process controls

The Windows runtime should use process-tree management and resource accounting through Windows Job Objects where available. It should use restricted process tokens, controlled environment variables, explicit working directories, and deny-by-default access to protected paths.

The sandbox abstraction must not rely on a single Windows API. It should expose capabilities such as filesystem isolation, network restriction, process limits, memory limits, CPU limits, and disposable cleanup, then report which capabilities are active.

### 9.3 Network policy

Network access should be categorized as provider traffic, package-manager traffic, Android runtime traffic, Nirman-managed local Android emulator traffic, or external-tool traffic. Each category should have an independent policy.

The default autonomous build profile should allow provider requests and approved Android dependency sources only. Emulator/device runtime traffic and Android project network access should be explicitly visible. External network access should be disabled in high-risk review profiles.

### 9.4 Dependency safety

Before executing an unfamiliar dependency or install script, Nirman should record its source, version, lockfile change, requested scripts, and scan status. Unverified packages should be restricted to a disposable or explicitly approved environment.

---

## 10. Preview and Device Architecture

### 10.1 Android development preview manager

The preview manager starts the Android development server or native build process, assigns or discovers required ports, tracks the process tree, checks Nirman-managed local Android emulator readiness, installs or reloads the application, captures Logcat and runtime errors, and exposes the current emulator state to the desktop interface.

A preview instance must be associated with a project revision and checkpoint. If the revision changes, the preview reports whether it hot-reloaded or restarted. If the project is rolled back, the preview must be restarted or marked stale.

### 10.2 Android device-profile testing

A preview test can define multiple Android emulator profiles:

```text
AndroidDeviceProfile
- name
- platformVersion
- apiLevel
- architecture
- width
- height
- density
- orientation
- locale
- permissions
- networkProfile
```

The device worker should install the build, launch activities, execute synthetic interactions, capture screenshots, record Logcat and crash output, verify permissions and orientation, and return a structured visual report.

The emulator validation subsystem MUST expose an authoritative `InteractionExecutor`.

```text
InteractionExecutor
- interactionId
- scenarioId
- deviceId
- artifactFingerprint
- applicationStateFingerprintBefore
- action
- interactionMethod
- targetIdentity
- inputDataClass
- observedResult
- applicationStateFingerprintAfter
- screenshotEvidenceId
- uiHierarchyEvidenceId
- logEvidenceId
- createdAt
```

The InteractionExecutor operates only against the running generated Android application through an admitted Android device adapter. It MUST NOT satisfy an interaction requirement by modifying source, invoking application internals outside the declared test interface, or asserting an expected state without observing it.

Every interaction produces an observed result that enters the normal Evidence → ValidationResult → CompletionDecision chain.

### 10.3 Android device manager

The Android device manager should provide a normalized interface for Nirman-managed Android emulators:

```text
Device
- id
- name
- kind
- platformVersion
- architecture
- connectionState
- availableStorage
- hotReloadState
- logStream
- installState
```

The first release may support one active device, but the interface should not assume that limitation. Device logs, installation results, reload failures, and build artifacts should be attached to the task record.

### 10.4 Screenshot and visual-specification pipeline

The input manager should accept screenshots, image sets, annotated references, and optional user assets as first-class project inputs. It should create a durable `VisualReference` record:

```text
VisualReference
- referenceId
- taskId
- sourcePath
- imageHash
- deviceHypothesis
- screenStateHypothesis
- extractedLayout
- extractedTypography
- extractedColorTokens
- extractedComponents
- interactionClues
- uncertaintyNotes
- privacyStatus
- createdAt
```

The visual worker converts references into an editable visual specification rather than directly copying pixels. The specification records screens, navigation states, layout regions, component roles, spacing, typography, colors, assets, interactions, responsive behavior across Android emulator profiles, and unresolved uncertainties. The implementation worker uses that specification to synthesize Android code, while the validation worker compares Nirman-managed local Android emulator screenshots against the reference and reports visual differences with evidence.

Screenshots sent to a cloud model must pass the project privacy policy. The system must redact or warn about sensitive text and identify the provider receiving the image. A visual reference is never treated as executable instruction; it is input data interpreted through the task contract.

A `VisualReference` establishes visual intent only. It never establishes behavior.

A reference image MAY establish: layout structure, visual hierarchy, component identity for standard components, color relationships, approximate spacing rhythm, and typographic scale.

A reference image MUST NOT be treated as establishing:

| Not establishable from an image | Must instead come from |
|---|---|
| Dynamic behavior on interaction | The instruction or a clarifying question |
| Input validation rules | The instruction or a clarifying question |
| Responsive behavior across sizes and orientations | The device profile and technology plan |
| Animation, transition, or gesture | The instruction; otherwise §5.4's ceiling applies |
| Semantic purpose of a non-standard component | A clarifying question |

The `interactionClues` field records hypotheses only. A hypothesis in that field MUST NOT be promoted to a requirement without confirmation from the instruction or an answered clarifying question. Every unpromoted hypothesis belongs in `uncertaintyNotes` and in the intent model's unresolved ambiguities.

Spacing, typography, and color derived from an image are estimates, not measurements. They MUST be recorded as assumptions with their derivation noted, and MUST yield to any explicitly stated value.

Visual comparison of a Nirman-managed local Android emulator screenshot against a reference is a visual observation only. It contributes evidence about appearance; it never establishes that behavior, state, or navigation is correct. Behavioral proof comes from the stateful scenarios of BS §56 (CLAUSE.EVIDENCE.CLAIM_SEPARATION applies unchanged).

### 10.5 Dynamic Android project synthesis

The project synthesizer builds a project graph from the goal contract, visual specification, existing files, assets, device requirements, integrations, and validation plan. It selects or composes the required Android technologies and creates the project structure, screens, navigation, state, data layer, permissions, services, tests, build configuration, and artifact profile.

The technology resolver must treat all Android implementation styles as available capabilities. It may select Java, Kotlin, Android Views, Jetpack Compose, Expo/React Native, custom native modules, Gradle plugins, background services, device APIs, or a mixed architecture. Its decision must be based on the requested behavior, screenshot evidence, performance needs, device APIs, offline requirements, build constraints, dependency compatibility, and validation evidence—not on a fixed user-facing template list.

```text
AndroidTechnologyPlan
- planId
- taskId
- requestedCapabilities
- visualRequirements
- selectedLanguages
- selectedUIFrameworks
- selectedRuntimeLayers
- selectedNativeModules
- selectedBuildPlugins
- selectedDeviceAPIs
- selectedLibraries
- compatibilityConstraints
- rejectedAlternatives
- requiredToolchains
- validationPlan
- confidence
- revision
```

Internal bootstraps may provide known-good build foundations, but they are implementation details rather than product limitations. The resolver must be able to create different project shapes, combine technologies, replace an incompatible layer, and add native modules when validation proves that the current architecture cannot satisfy the goal. The user may inspect the technology plan, but should not be required to choose the stack before describing the desired application.

Project synthesis must be incremental. It should first create a buildable Android shell, then implement the visual and behavioral contract, then integrate data and device capabilities, and finally harden the project through tests, visual comparison, packaging, signing-boundary checks, and recovery. Each synthesis stage produces a checkpoint and evidence.

### 10.6 Android device and host isolation

Android validation MUST use disposable Nirman-managed local Android emulator snapshots only. Physical Android hardware is outside product scope and MUST NOT participate in preview, validation, recovery, completion, or delivery. It must not reuse personal credentials, host-side secrets, or unapproved emulator data. Test data should be synthetic by default. Emulator sessions, installed packages, permissions, logs, screenshots, and cleanup state must be attached to the task record.

### 10.7 Emulator frame transport

The Nirman-managed local Android emulator is the canonical PreviewRuntime for the primary development workflow. It MUST run headless on the Windows host and its rendering surface MUST be projected into the WinUI 3 PreviewHost. A detached emulator window is not a valid primary preview.

No physical Android hardware is supported, required, or accepted as an alternative runtime. The Nirman-managed local Android emulator is the sole canonical Android preview and validation runtime.

The transport is a named, versioned interface with a required baseline and a permitted upgrade:

- **Baseline** — the emulator's local gRPC control endpoint on loopback, using its screenshot-stream RPC. Low frame rate, minimal dependencies, sufficient for a truthful preview.
- **Permitted upgrade** — a WebRTC/video-stream path for higher frame rate and input forwarding, admitted through the SAME `PreviewPromotionGate`. Not a second authority.

Every delivered frame MUST bind `deviceId`, `PreviewRevision`, `artifactFingerprint`, and `deviceStateFingerprint`. An unbound frame MUST be labelled `STALE` and MUST NOT satisfy completion (CLAUSE.PREVIEW_SYNC.IDENTITY_MATCH, CLAUSE.PREVIEW_SYNC.EVIDENCE_BOUND). Frame capture is an `AndroidDeviceAdapter` operation carrying `adapterId`, `adapterVersion`, `technologyPlanHash`, and `deviceAdapterIdentity` (CLAUSE.PREVIEW_SYNC.ADAPTER_BOUND). Transport loss MUST invalidate the projection through the single canonical reducer (CLAUSE.PREVIEW_SYNC.SINGLE_REDUCER) and MUST NOT freeze the last frame while presenting it as live (CLAUSE.PREVIEW_SYNC.NO_LOCAL_ADVANCE). Physical devices use a separate documented capture path; label semantics are identical. Loopback only. The transport MUST NOT bind to an external interface.

### 10.8 Embedded PreviewHost

PreviewHost is the WinUI 3 presentation surface that displays the rendering projection of the Nirman-managed Android emulator.

PreviewHost does not execute Android commands and is not an authority.

Required path:

Android Emulator
→ RenderTransport
→ PreviewCoordinator
→ PreviewSyncEvent
→ PreviewProjectionReducer
→ PreviewHost
→ WinUI 3 Preview panel

Input follows the reverse controlled path:

WinUI PreviewHost
→ typed PreviewInteraction command
→ Supervisor
→ AndroidDeviceAdapter
→ emulator input channel

The PreviewHost MUST NOT invoke ADB, Gradle, emulator APIs, or application internals directly.

```text
PreviewSurface
- previewSurfaceId
- previewSurfaceSessionId
- projectId
- taskId
- previewRevisionId
- deviceSessionId
- runtimeSessionId
- renderTransportId
- renderTransportVersion
- inputChannelId
- viewportStateFingerprint
- status
- createdAt

PreviewInteraction
- interactionId
- previewSurfaceId
- deviceId
- runtimeSessionId
- action
- targetIdentity
- inputDataClass
- expectedObservation
- createdAt
```

These are execution and projection records, not authorities.

---

---

## 11. Toolchain and Environment Management

### 11.1 Version resolution

Each project should declare required tool versions or compatible ranges. The runtime should resolve those requirements through local version managers, portable installations, or configured executable paths, then isolate each project through environment filtering, cache separation, process scopes, and toolchain bindings.

A project environment record should include:

```text
EnvironmentRecord
- projectId
- operatingSystem
- executablePaths
- detectedVersions
- requestedVersions
- resolutionSource
- compatibilityStatus
- reproducibilityStatus
- maxPathLength
- longPathPolicyEnabled
- lastVerifiedAt
```

Two projects that require different Node.js, Java, Android SDK, Rust, or package-manager versions must be able to run without silently changing global state.

### 11.2 Environment diagnostics

Diagnostics should distinguish missing, incompatible, inaccessible, unverified, and healthy tools. A failed build must name the missing executable or incompatible version and explain the next action.

### 11.3 Android runtime abstraction

The runtime should expose Android-focused interfaces for process execution, filesystem policy, environment discovery, Java/Kotlin compilation, Gradle execution, JavaScript bundling when selected, native module builds, Nirman-managed local Android emulator management, Logcat, quotas, screenshots, signing-boundary checks, and APK artifacts. The Windows desktop host supplies the local process and sandbox implementation; the generated-project contract remains Android-specific and technology-neutral.

---

### 11.4 Persistent terminal session manager

The runtime should expose a `TerminalSession` abstraction instead of treating every command as a one-shot shell call.

```text
TerminalSession
- sessionId
- taskId
- workerId
- workspaceId
- shellProfile
- workingDirectory
- environmentFingerprint
- processGroupId
- ptySupported
- stdinPolicy
- outputLogPath
- rotationPolicy
- status
- createdAt
- lastActivityAt
```

A worker can reuse a terminal session for commands that depend on working directory, environment variables, virtual-environment activation, package-manager state, or a long-running development server. Session environment changes must be explicit and recorded rather than inferred from arbitrary shell output.

The terminal manager must detect interactive prompts through known prompt signatures, stdin readiness, process activity, and configurable prompt classifiers. In unattended mode, it should answer only declared safe prompts using a task policy; otherwise it should terminate safely, capture the prompt, and classify the task as requiring a decision. Dev servers and emulators must be registered as long-running processes rather than mistaken for hung commands.

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
│   └── reconciliation/
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
| Local IPC | Authenticated loopback WebSocket or named-pipe abstraction |
| Metadata storage | SQLite with migrations and WAL mode where appropriate |
| Task logs | Append-only files referenced from SQLite |
| Worktree management | Git worktrees with temporary copy fallback |
| Worker process | Child process with declared runtime profile |
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

```text
GoalContract
- goalId
- taskId
- statement
- completionConditions
- validationPlan
- scope
- autonomyPolicy
- resourceBudget
- stopConditions
- progressSummary
- lastEvaluatedAt
- status
```

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
    ├── Budget reached → pause and escalate
    ├── Safety stop → pause and request approval
    └── Repeated failure → backtrack or escalate
```

The goal evaluator must record each condition result and should not rely on a final model statement. A task may continue after a worker reports completion if objective validation is still incomplete.

### 16.2.1 Execution profiles and approval precedence

Nirman must define approval behavior through an explicit execution profile rather than through isolated UI wording. The profile is authoritative for routine approval behavior, while safety and authority gates remain mandatory in every profile.

| Profile | Routine policy-allowed actions | Hard-gated actions |
|---|---|---|
| `Interactive / Review` | May request or require approval according to the project policy and review settings. | Protected paths, credentials, signing, destructive actions, external-emulator access, publishing, and other declared hard gates. |
| `Unattended / Full Autonomy` | Automatically executes routine reversible actions inside the approved workspace, including local dependency installation, formatting, tests, builds, preview restarts, checkpoints, and authorized environment repair. | The same hard gates; it pauses or escalates instead of bypassing them. |

Routine approval prompts must not be required merely because the UI is disconnected or a task is running in the background. Every approval request is bound to the exact action fingerprint, policy, worker, workspace, and risk. User approval authorizes only the requested policy-bound action; it never promotes a preview or artifact without deterministic evidence.

### 16.2.2 Profile terminology namespaces

Nirman uses multiple profile concepts. Each has an explicit namespace, ID prefix, and canonical owner to prevent field collision:

| Concept | Namespace | ID prefix | Canonical owner | Purpose |
|---|---|---|---|---|
| Execution profile | `profile.execution` | `exec-profile` | PolicyAuthority | Approval behavior for routine actions |
| Autonomy profile | `profile.autonomy` | `autonomy-profile` | PolicyAuthority | Unattended vs interactive execution policy |
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

```text
Schedule
- scheduleId
- projectId
- goalDefinition
- triggerType
- triggerExpression
- enabled
- allowedMode
- approvalPolicy
- resourceBudget
- notificationPolicy
- lastRunId
- nextRunAt
- failureCount
```

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
| Context | `context_budget_reached` | Compact or switch retrieval strategy |
| Runtime | `process_failed` | Capture diagnostic and classify error |
| Configuration | `external_tool_connected` | Register tool capabilities and policies |

Blocking hooks must complete before the associated action proceeds. They require a timeout, retry policy, and failure behavior. Non-blocking hooks run asynchronously and cannot mutate the result of the completed action. Hooks must be idempotent or include a deduplication key.

## 18. Two-Tier Checkpoint and Backtracking Architecture

The checkpoint manager should maintain file-level snapshots and task-level revisions.

```text
FileCheckpoint
- checkpointId
- taskId
- filePaths
- contentHashes
- parentRevision
- createdAt

TaskCheckpoint
- checkpointId
- taskId
- projectRevision
- workerWorkspaces
- metadataSnapshot
- previewRevision
- validationSnapshot
- createdAt
```

Checkpoint storage must use a retention policy for long-running sessions. Every task retains the initial source checkpoint, the last known-good checkpoint, all checkpoints referenced by an active recovery strategy, and a configurable number of recent task checkpoints. Older intermediate checkpoints should be compacted into content-addressed snapshots or pruned only when no active branch, preview, recovery attempt, or evidence record references them. Before deletion, the system must verify that a full restore path remains available.

Android tasks should use profile-based quotas for JavaScript, native, emulator and combined build workflows. The quota manager must account for worktrees, dependency stores, Gradle caches, APK artifacts, emulator images, logs, screenshots, and checkpoints. It should prefer deduplicated content-addressed storage and cleanup of rebuildable caches before deleting checkpoints.

Backtracking should restore a known-good checkpoint before trying a materially different strategy. The recovery manager should keep a strategy history:

```text
RecoveryAttempt
- attemptId
- taskId
- failureFingerprint
- checkpointRestored
- strategyDescription
- workerRole
- modelProfile
- actionsTaken
- validationResult
- createdAt
```

The recovery planner must reject a new attempt when it is substantially identical to a previous failed attempt. It may change the context mode, worker role, model profile, implementation approach, or test strategy before resuming.

## 19. Context Scaling Architecture

The context engine should expose two provider-independent modes:

| Mode | Pipeline |
|---|---|
| Retrieval | Repository map → relevance ranking → selected files and symbols → task context |
| Large context | Secret filtering → generated-file filtering → repository packing → token-budget check → task context |

The provider capability registry should report context capacity, vision support, tool support, structured-output support, and streaming support. The context planner selects a mode based on the provider capability, project size, privacy policy, task type, and user preference.

The context package should record included paths, excluded paths, summaries, token estimates, redactions, and the reason for selecting the mode. If the large-context estimate exceeds the configured budget, the planner must fall back to retrieval mode rather than silently truncating critical files.

The repository map must scale incrementally. It should update changed files and affected dependency regions instead of rebuilding the entire map after every action. Large projects should use sharded indexes, symbol-level summaries, dependency fingerprints, cache invalidation, and background compaction. The map manager should expose freshness, shard size, rebuild progress, and stale-region warnings to the task runtime.

### 19.1 Skill Package Registry and Invocation

The skill registry should store:

```text
SkillPackage
- skillId
- name
- description
- version
- scope: built_in | user | project
- compatibleWorkerRoles
- triggerConditions
- requiredTools
- requiredCapabilities
- permissionRequests
- inputSchema
- outputSchema
- sourcePath
- scanStatus
- trustStatus
- enabled
- installedAt
- lastUsedAt
```

A skill is selected by the orchestrator from a task requirement, explicit user request, or matching trigger condition. Loading a skill adds instructions and schemas; it never grants permissions automatically. Skill tool calls still pass through the policy engine and are logged as ordinary tool calls.

User or shared skills must be scanned for prompt injection, unsafe commands, secret access, hidden network behavior, and dependency changes before activation. Updates must be versioned, health-checked, and reversible. Built-in runtime capabilities take precedence over skills when both provide the same function, while skills may add domain-specific workflow instructions around those capabilities.

Skills should be testable through fixture tasks and should declare the minimum tools, worker roles, and project profiles they require.

## 20. External Tool Protocol Adapter

Nirman’s internal Tool Gateway remains authoritative, but an adapter may expose or consume standardized external tool servers. The adapter should translate external tool calls into Nirman policy requests before execution.

```text
ExternalToolConnection
- connectionId
- projectScope
- serverIdentity
- declaredTools
- networkPolicy
- dataPolicy
- allowedWorkers
- approvalPolicy
- enabled
- lastHealthCheck
```

External tools should be capability-discovered, permission-scoped, health-checked, and auditable. A tool that requests local file access must still pass through the filesystem policy. A tool that causes an external side effect must create an approval request unless the project policy explicitly allows it.

## 21. Authority Hierarchy and Recovery Invariants

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
8. A large-context request falls back to retrieval when the token budget is insufficient.
9. An external tool cannot bypass path, network, or approval policies.
10. A skill cannot grant itself permissions or execute undeclared tools.
11. The repository map updates only affected shards after a small file change.
12. A stale repository-map region is detected and refreshed before an edit.
13. A skill with an unsafe instruction or undeclared network behavior is rejected before activation.
14. The canonical worker registry exposes the Performance Worker and every worker has a declared contract.
15. An interface agreement is required before parallel workers modify coupled frontend/backend contracts.

## 23. Execution Surface and Evidence Model

### 23.1 Durable task graph

The control plane should represent each autonomous task as a durable directed graph rather than a flat progress string. The graph contains a root goal, requirement nodes, implementation phases, worker tasks, dependency edges, validation nodes, checkpoints, approvals, recovery attempts, and final evidence.

```text
TaskGraph
- graphId
- taskId
- goalContractId
- nodes
- edges
- currentNodeIds
- completedNodeIds
- blockedNodeIds
- lastValidatedCheckpoint
- completionEvaluation
- updatedAt
```

A node may be `pending`, `ready`, `running`, `waiting_approval`, `waiting_resource`, `completed`, `failed`, `blocked`, `skipped`, or `cancelled`. A node can be marked `completed` only after its evidence requirements pass. Model summaries may explain a node, but they cannot complete it without an execution or review evidence record.

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

```text
EvidenceRecord
- evidenceId
- taskId
- nodeId
- type
- command or source
- inputsHash
- outputPath
- result
- exitCode
- artifactReferences
- capturedAt
- reproducibilityStatus
```

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

An approval request must include the exact action, worker, workspace, path or destination, policy rule, risk explanation, requested data, predicted side effect, and choices. Approval must be bound to the request fingerprint and must expire when the action, task state, or policy changes.

### 23.7 Termination coordinator

The termination coordinator evaluates whether a task may continue after every validation, recovery, budget, approval, and environment event. It must recognize these terminal classifications:

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

The official API reference distinguishes a response-oriented surface for direct model requests, tool use, multimodal inputs, and stateful interactions from a chat-completion surface based on conversation messages.[7] Nirman should support both without assuming that every configured endpoint supports the same capabilities.

### 24.2 Provider profile

The AI Settings page should store a provider profile with the following shape:

```text
ProviderProfile
- providerId
- displayName
- compatibilityMode: OPENAI_COMPATIBLE | ANTHROPIC_COMPATIBLE
- protocol: chat_completions | responses | messages | custom
- baseUrl
- apiKeySecretRef
- modelId
- visionModelId
- embeddingModelId
- rerankerModelId
- organizationIdOptional
- projectIdOptional
- customHeadersSecretRefs
- defaultParameters
- capabilityOverrides
- reasoningModelIdOptional
- reasoningCapabilityProfile
- defaultReasoningEffort
- maxReasoningTokensOptional
- privacyPolicy
- networkPolicy
- enabled
- lastConnectionTest
- lastHealthStatus
```

The API key and sensitive headers must be stored only through the operating-system keychain. The profile may display a masked key fingerprint and last validation time, but never the raw key.

The `reasoningCapabilityProfile` field holds the provider's discovered reasoning capability:

```text
ReasoningCapabilityProfile
- supportsNativeReasoning: true | false | unknown
- supportedEffortLevels: NORMAL | EXTENDED | DEEP | EXHAUSTIVE[]
- maxReasoningTokensOptional
- reasoningUsage: reported | estimated | unavailable
- effortParameterMapping: provider-specific normalized mapping
- supportsPerRequestEffortChange: true | false
- supportsContinuation: true | false | unknown
```

`effortParameterMapping` is configuration metadata, not authority: it records how normalized effort levels translate into provider-specific parameters, but it can never alter budgets, permission ceilings, or authority state.

### 24.3 AI Settings page behavior

The settings interface should allow the user to create, duplicate, test, disable, and delete provider profiles. It should support custom base URLs and model IDs. Save is disabled until a connection Test against the configured endpoint and model returns a successful validated response per ADR-208. Any edit to key, base URL, model ID, or compatibility mode invalidates a prior pass and re-disables Save.

The connection test should discover or validate the configured endpoint, verify authentication, test the selected model, detect available features, measure a basic response, and record the provider request ID. Model discovery through a models endpoint is optional; a user must be able to enter a model ID manually when discovery is unavailable.

The page should show capability badges for text, vision, file input, tool calls, structured output, streaming, cancellation, background requests, embeddings, reasoning, supported reasoning effort levels, reasoning usage reporting, and context capacity. A capability badge must be based on a successful probe or explicit user override, not a provider name alone.

Reasoning capability must be displayed separately from general text generation. A model that can generate text but does not expose or support provider-native reasoning must not be presented as supporting the configured deep-reasoning capability.

### 24.4 Canonical request and response model

The model gateway should convert all supported protocols into a canonical internal request:

```text
ModelRequest
- requestId
- taskId
- workerId
- conversationId
- modelId
- systemInstructions
- messagesOrInputItems
- tools
- responseSchemaOptional
- modalities
- contextReferences
- temperatureOptional
- reasoningSettings
- serviceTierOptional
- stream
- providerBackgroundOptional
- cancellationSignal
- privacyLabels
```

```text
ReasoningSettings
- effortLevel: NORMAL | EXTENDED | DEEP | EXHAUSTIVE
- maxReasoningTokensOptional
- maxReasoningTimeMsOptional
- providerNativeParameters
- deliberationId
- passNumber
- budgetReservationId
```

`providerNativeParameters` may contain provider-specific reasoning controls, but those values are generated by ModelGateway from the normalized runtime request. A model response or provider cannot modify the runtime's budget, permission ceiling, or authority state.

`budgetReservationId` must refer to a runtime-owned reservation covering the maximum reasoning expenditure permitted for the request.

The canonical response should be event-oriented:

```text
ModelEvent
- requestId
- sequence
- type: started | text_delta | reasoning_delta | tool_call_delta |
        tool_call_complete | usage | completed | failed | cancelled
- providerEventTypeOptional
- contentOptional
- toolCallOptional
- usageOptional
- finishReasonOptional
- requestIdFromProviderOptional
- errorOptional
- createdAt
```

The orchestrator consumes the canonical events and writes them to the task event ledger. A provider’s streaming format must never be rendered directly by the UI as the source of truth.

### 24.5 Tool-call normalization

Chat-completion tool calls, response-item function calls, and message-oriented tool calls should normalize to:

```text
ToolCallRequest
- callId
- toolName
- argumentsJson
- taskId
- workerId
- policyContext
- providerRequestId
```

Nirman validates arguments against the registered tool schema, sends the request through the policy engine, executes the tool only when allowed, and returns a normalized `ToolCallResult` with status, output reference, redactions, duration, and evidence ID. Tool results must be associated with the original call ID so the next model request can be serialized correctly for the selected protocol.

### 24.6 Streaming, cancellation, retry, and long-running requests

The provider runtime should support streaming when available and should emit partial content and tool-call deltas into the durable event stream. If a provider does not support streaming, the control plane should still emit request-started, request-completed, and usage events.

Cancellation should be cooperative first, then terminate the local request process if the provider client does not stop. A cancelled request must not be treated as a task failure unless the task cannot continue safely.

Provider retries should classify authentication errors, invalid requests, rate limits, transient network failures, provider overload, context overflow, unsupported capabilities, and content-policy responses separately. A retry must preserve the request correlation ID and must not duplicate a tool execution. When a request is too large, the context planner should compact or retrieve less context instead of silently dropping required instructions.

The provider may offer its own background-request mode, but Nirman’s local control plane remains the authoritative owner of task persistence and resumption. Provider-side background execution is an optimization, not a substitute for local recovery.

### 24.7 Context and token policy

Nirman should record token usage when the provider reports it, but token budget must not be a default end-to-end completion lock. The user may configure hard caps, but the normal policy is adaptive context compaction, retrieval, model routing, concurrency reduction, and continuation.

A provider profile may declare a context capacity or allow Nirman to learn it from probe results. If the selected request exceeds a provider’s actual hard context limit, the gateway must return a structured context-overflow result so the context planner can reduce or reassemble the request.

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

A provider adapter is not production-ready until it passes the fixtures relevant to its declared capabilities.

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

### 25.3 Self-development task contract

A self-development task should include:

```text
SelfDevContract
- taskId
- sourceRevision
- targetGoal
- allowedSourcePaths
- forbiddenPaths
- allowedTools
- testPlan
- buildProfiles
- promotionPolicy
- rollbackPolicy
- healthChecks
- compatibilityChecks
- releaseNotesRequired
```

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

The self-development worker may continue autonomously through implementation, tests, rebuilds, candidate launches, and repair cycles. It must stop for promotion only when the promotion policy requires approval or when a hard safety, compatibility, migration, or health condition fails.

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

When a lease expires, the supervisor should inspect process liveness, preserve the worker workspace, record an interruption, and choose among resume, requeue, recovery, or escalation. A worker must never keep a task permanently claimed after a crash.

## 28. Recovery Ladder and Problem-Solving Depth

### 28.1 Recovery levels

Nirman should use a graduated recovery ladder. It should not jump immediately to a new model or ask the user for help.

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

### 28.2 Failure fingerprints

The runtime should fingerprint failures using normalized command, exit code, error class, stack-trace structure, changed-file set, environment state, provider response class, and validation stage. Fingerprints should be stable enough to detect repeated failures but specific enough to distinguish a new cause.

The recovery manager should maintain a failure-pattern record containing the fingerprint, affected project area, attempted strategies, successful fixes, last known-good checkpoint, and confidence. This record can feed project memory and future self-improvement proposals after sensitive data is removed.

### 28.3 Progress quality

The runtime should measure whether an attempt made verified progress. Progress may include more passing tests, fewer runtime errors, a smaller conflict set, successful environment setup, a valid artifact, or a newly satisfied acceptance condition. A task that consumes requests without improving evidence should be routed into recovery rather than continuing the same loop.

## 29. Self-Observation and Self-Evaluation

### 29.1 Episode recording

Every completed, failed, cancelled, recovered, or escalated task should produce an `EpisodeRecord`:

```text
EpisodeRecord
- episodeId
- taskId
- projectFingerprint
- goalClass
- stackProfile
- providerProfile
- planRevision
- workerRoles
- actionsSummary
- checkpoints
- failures
- recoveryStrategies
- validationResults
- finalClassification
- resourceTelemetry
- userCorrections
- privacyClassification
- createdAt
```

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
| Human intervention rate | Number and category of decisions required per task |

These metrics are for diagnosis and improvement. They must not be used to conceal failed tasks or to optimize only for speed at the expense of correctness.

### 29.3 Evaluation runs

An evaluation run executes a fixed fixture suite against a specific runtime version, provider configuration, model profile, prompt policy, tool registry, and project environment. The run must record the exact inputs and produce comparable results.

The evaluation engine should include ordinary feature tasks, multi-file refactors, environment failures, provider failures, merge conflicts, visual regressions, database migrations, sandbox tests, long-running continuation, self-update failures, and recovery scenarios.

## 30. Self-Improvement Manager

### 30.1 Improvement sources

The self-improvement manager may create improvement proposals from recurring failure patterns, regression clusters, provider incompatibilities, task-intervention categories, benchmark results, user corrections, stale instructions, tool failures, and observed performance degradation.

It must not automatically convert a single unusual failure into a permanent rule. An improvement proposal should include evidence frequency, affected task classes, confidence, expected benefit, possible regressions, scope, and rollback plan.

### 30.2 Improvement proposal

```text
ImprovementProposal
- proposalId
- sourceEpisodes
- problemStatement
- hypothesis
- affectedComponents
- proposedChanges
- expectedMetrics
- safetyImpact
- testPlan
- rollbackPlan
- approvalPolicy
- status
```

Possible proposal targets include prompts, task decomposition rules, model routing, context retrieval, tool schemas, failure classifiers, UI instructions, validation rules, worker roles, skill packages, provider adapters, and runtime code.

Changes to the supervisor, policy engine, credential handling, sandbox, updater, database migration system, or evidence engine must receive the highest review level and may not be auto-promoted solely from a model-generated proposal.

### 30.3 Candidate generation loop

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

Nirman should maintain three memory scopes:

| Memory scope | Contents | Lifetime |
|---|---|---|
| Task memory | Current goal, plan, evidence, failures, checkpoints, and active assumptions | Until task retention expires |
| Project memory | Architecture decisions, conventions, fixes, routes, dependencies, and validated preferences | Project lifetime, user-deletable |
| Runtime improvement memory | Anonymized failure patterns, evaluation results, provider compatibility, and candidate outcomes | Runtime version lifetime, user-controlled |

Memory should be written from validated events and user-confirmed decisions, not from every model statement. The user must be able to inspect, correct, export, and delete memory. Secrets, raw credentials, protected files, and unclassified private content must be excluded.

## 32. Complete Runtime and Self-Improvement Failure Modes

The architecture must explicitly handle:

| Failure | Required response |
|---|---|
| Supervisor crash | Rehydrate task state, reconcile leases, and resume from the last validated checkpoint |
| Worker crash | Preserve workspace, record interruption, and requeue or recover |
| Provider outage | Retry with classification, use an approved fallback, or continue after service recovery |
| Context overflow | Compact, retrieve, or change provider; never silently omit critical requirements |
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

```text
AutonomousAndroidSession
- sessionId
- goal
- screenshotsAndAssets
- AndroidApplicationContract
- VisualSpecification
- AndroidTechnologyPlan
- taskGraph
- workerRegistry
- terminalSessions
- sandboxProfile
- activeRevision
- previewState
- checkpoints
- validationState
- recoveryState
- artifactState
- completionState
```

### 34.1 Input-fusion pipeline

The input manager combines the user instruction, screenshots, supplied assets, existing project files, device requirements, integrations, and delivery requirements into an application contract, editable visual specification, and technology plan. Screenshots are interpreted as visual evidence and never as executable permission. The technology resolver selects or composes the required Android implementation without requiring a user-facing framework or template choice.

### 34.2 Preview revision bridge

The preview manager and execution manager share a `projectRevisionId`, `activeBranchId`, `checkpointId`, and promotion lineage. Every emulator or emulator state records the revision, emulator identity, installation state, reload state, Logcat stream, runtime errors, screenshot, visual comparison result, and responsible task node. Preview currency additionally requires:

```text
DeviceStateFingerprint
- deviceIdentity
- apiLevel
- locale
- orientation
- permissionsSnapshot
- systemSettingsSnapshot
- networkMode
- installedPackageState

ApplicationStateFingerprint
- packageName
- processState
- databaseSnapshot
- preferencesSnapshot
- appPermissions
- accountSessionState

EnvironmentStateFingerprint
- toolchainLock
- environmentIdentity
- dependencySnapshot
- providerProfile
- validationPolicyVersion
```

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

The stall detector identifies repeated commands, repeated patches, repeated failure fingerprints, repeated strategy fingerprints, unchanged workspaces, absent evidence, unresponsive processes, stale emulators, and heartbeats without useful progress. A detected stall causes a controlled strategy transition: refresh context, repair the environment, change technology, delegate diagnosis, restore a checkpoint, reduce scope to a safe subtask, or construct an isolated alternative. The scheduler must reject identical retries that do not provide a new strategy fingerprint, new causal fingerprint, or positive progress delta.

### 34.4 Swarm handoff and reconciliation contract

Parallel workers receive explicit contracts, isolated workspaces, allowed tools, expected outputs, and validation rules. Each handoff must include changed files, assumptions, dependencies, tests, evidence, unresolved issues, and recommended next actions. The reconciliation worker integrates only validated outputs, resolves conflicts, runs integrated Android checks, updates the preview revision, and commits the next checkpoint.

### 34.5 Autonomous validation and artifact gate

For applicable Android delivery, the validation coordinator must prove build success, APK existence, checksum, artifact scan, installation or launch, main-flow execution, visual comparison, permission behavior, and absence of unresolved fatal runtime errors. The artifact is complete only when it is linked to the project revision and evidence ledger.

Routine project-local actions are allowed under the project’s Unattended / Full Autonomy policy. The runtime may edit, install dependencies, run terminals, launch devices, build, test, capture screenshots, repair, checkpoint, delegate, reconcile, and create local artifacts without repeated approval. Protected credentials, destructive actions, publishing, signing policy, protected paths, hard safety violations, and unrecoverable blockers remain deterministic authority boundaries.

## 35. Complete Android Capability Fixture Contract

The test harness must include generated-from-instruction fixtures for JavaScript-driven Android, Java, Kotlin, Android Views, Jetpack Compose, mixed architectures, custom native modules, background services, WorkManager, notifications, camera and media, location and sensors, Bluetooth and NFC, offline-first storage, API-heavy applications, authentication and permissions, tablet and multi-orientation layouts, device-integrated applications, and APK delivery. These fixtures validate AI technology selection and composition; they are not user-facing templates.

## 36. Production Runtime Contract Architecture

The production runtime is divided into deterministic authorities and model-driven proposal services. The model gateway proposes plans, edits, tool calls, recovery strategies, and improvement proposals. The supervisor, lifecycle authority, permission authority, sandbox authority, storage authority, evidence authority, recovery authority, promotion authority, and termination authority decide what can execute and what counts as complete.

### 36.1 Canonical runtime contracts

The following contracts are versioned and validated at the control-plane boundary:

```text
CanonicalSchemaRegistry
AutonomousAndroidSession
AndroidApplicationContract
VisualSpecification
AndroidTechnologyPlan
CapabilityProfile
TaskGraph
WorkerContract
TerminalSession
PreviewRevision
EvidenceRecord
EvidenceDependency
ValidationResult
CertificationDecision
CompletionDecision
RecoveryRecord
ArtifactRecord
ArtifactSet
IntegrationOperationality
ExternalEffectRecord
IntegrationBoundaryContract
UsageRecord
ProviderProfile
FrontendControlPlaneContract
UICommandRegistry
UICommandEnvelope
ProjectionSnapshot
UIResponseEnvelope
UIErrorEnvelope
EventSubscription
CostGovernanceRecord
AgentTrustAssessment
ContextCachePolicy
AndroidRuntimeIntegrityObservation
ContinuityDimensions
BackgroundContinuityRecord
APKExportRecord
EnvironmentCapabilityRecord
PlatformCapabilityEntry
ValidationEnvironment
BuildGateRecord
```

Each contract has a schema version, owner, lifecycle status, project scope, source revision, created timestamp, updated timestamp, and audit references where applicable. Persistent records use atomic writes, file locking, migration backups, and rollback.

`CanonicalSchemaRegistry` is the sole machine-readable ownership index for these contracts. Each entry records `schemaId`, `canonicalOwner`, `version`, fields, enum values, invariants, migration policy, authority, persistence location, and acceptance-fixture IDs. Repeated schema descriptions in other documents are explanatory or implementation views and must identify the registry entry they implement; they cannot silently redefine fields or enum semantics.

Schema compatibility is explicit:

```text
ContractCompatibility
- fromVersion
- toVersion
- compatibleRead
- compatibleWrite
- migrationRequired
- evidenceInvalidationPolicy
- runtimeRestartRequired
- acceptanceFixtureIds
```

A self-development candidate or contract migration cannot be promoted until its read/write compatibility, migration, restart, replay, and rollback behavior pass the declared fixtures. `IntegrationBoundaryContract` is the common reference envelope for boundary-crossing operations; specialized contracts remain authoritative for payloads, state machines, authorities, transactions, evidence, preview, providers, skills, artifacts, signing, and completion. `IntegrationBoundaryContract` is the common reference envelope for boundary-crossing operations; specialized contracts remain authoritative for payloads, state machines, authorities, transactions, evidence, preview, providers, skills, artifacts, signing, and completion.

### 36.2 Lifecycle authority

The lifecycle authority implements the state machine:

```text
Created → Understanding → Planning → EnvironmentPreparing
  → ProjectSynthesizing → Implementing → Previewing
  → Testing → Recovering → Revalidating → Packaging → Completed
```

Terminal states are `BlockedByPolicy`, `BlockedByMissingInformation`, `ProviderUnavailable`, `EnvironmentUnrecoverable`, `Cancelled`, and `SafelyFailed`. State transitions are accepted only through deterministic transition guards. A model response, worker message, skill, hook, or frontend event can request a transition but cannot commit one.

### 36.3 Renewable session leases and operation capabilities

The session supervisor maintains a renewable lease containing session ID, supervisor generation, last heartbeat, progress sequence, project revision, sandbox profile, and authority policy. Lease renewal is permitted only when the task is making validated progress or is waiting on a classified external condition.

Sensitive operations use single-use operation capabilities with an action type, session ID, worker ID, workspace ID, project revision, scope fingerprint, permission policy, issued time, expiry, and consumption state. The capability manager consumes the capability before side effects and rejects reuse, revision mismatch, scope mismatch, policy mismatch, or expired capabilities. The model cannot create or broaden capabilities.

### 36.4 Cross-entity evidence and completion contracts

The following records are canonical implementation contracts, not additional authorities. They connect the existing lifecycle, evidence, preview, artifact, policy, toolchain, device, and integration services so that no subsystem can report a stronger state than its dependencies permit.

```text
EvidenceDependency
- dependencyId
- evidenceId
- dependencyType: source | asset | toolchain | device | artifact |
                    integration | policy | checkpoint | environment
- dependencyIdentity
- validFromEventId
- invalidatedByEventId
- invalidationReason

ArtifactSet
- artifactSetId
- requiredArtifacts: APK | APK_AND_AAB
- sourceRevision
- assetManifestVersion
- toolchainLockId
- environmentIdentityId
- validationPolicyVersion
- artifactRecordIds
- signingState
- reproducibilityLevel
- deliveryState

IntegrationOperationality
- integrationId
- required
- endpointIdentity
- credentialReference
- schemaVersion
- policyProfile
- state: NOT_REQUIRED | SPECIFIED | CONFIGURED | REACHABLE |
         FUNCTIONAL | DEGRADED | USER_REQUIRED | UNAVAILABLE |
         BLOCKED | UNKNOWN
- healthEvidenceId
- functionalEvidenceId
- invalidatedBy

ExternalEffectRecord
- effectId
- operationType
- targetIdentity
- requestFingerprint
- authorityGrantId
- idempotencyKey
- requestState: NOT_SENT | SENT | ACKNOWLEDGED | UNKNOWN | FAILED
- responseReference
- compensationPlan
- compensationState
- localTransactionId
- reconciliationState: KNOWN_SUCCESS | KNOWN_FAILURE | UNKNOWN | RECONCILING | RESOLVED

`ExternalEffectRecord.reconciliationState` generalizes the export-only `UNKNOWN → RECONCILING` pattern (ADR-203) to every external side effect. Every adapter that performs an external effect—ADB install, Nirman-managed local Android emulator launch, provider/model request, signing operation, filesystem copy, package installation, process creation, and remote API—MUST record an `ExternalEffectRecord` and implement reconciliation against the canonical `reconciliationState` lifecycle:
- `KNOWN_SUCCESS` / `KNOWN_FAILURE`: observed and verified terminal state.
- `UNKNOWN`: the effect was issued but its outcome could not be confirmed (timeout, partial response, device/provider drop, interrupted copy, process disappearance).
- `RECONCILING`: an `UNKNOWN` outcome is being resolved by destination/identity/hash inspection or provider/device status re-check; no retry of the effect is permitted until resolution.
- `RESOLVED`: reconciliation completed and the outcome was deterministically classified as success or failure (recorded in `responseReference` / compensation state).

An `UNKNOWN` outcome MUST NOT be retried, promoted, or reported as success until it transitions to `RESOLVED`. This applies uniformly; the export copy path (M117/ADR-203) is one instance, not a special case. The `CanonicalSchemaRegistry` owns this schema; adapter-local variants are explanatory only and must not redefine the enum.

UsageRecord
- usageId
- parentUsageId
- taskId
- workerId
- providerRequestId
- processGroupId
- resourceClass
- reservedAmount
- observedAmount
- attributionStatus: DIRECT | INHERITED | SHARED | ESTIMATED | UNAVAILABLE
- startEventId
- endEventId
```

`RepositoryTrust`, `EnvironmentIdentity`, `SigningState`, `ReproducibilityLevel`, `CapabilityMaturity`, `ProductLifecycleState`, `AssuranceState`, `IntegrationOperationality`, and `DeliveryState` are separate fields. They MUST NOT be collapsed into a single status or inferred from model output.

The canonical evidence chain is:

```text
Observation → EvidenceArtifact → ValidationResult → CertificationDecision → CompletionDecision
```

A source revision, asset manifest, toolchain lock, emulator session, dependency snapshot, validation policy, or required integration change invalidates dependent evidence and completion claims unless the dependency graph proves independence. `EvidenceAuthority`, `PreviewPromotionGate`, `ArtifactAuthority`, `AndroidQualityGate`, and the completion evaluator consume the same dependency relation.

The canonical preview-current predicate is:

```text
preview_is_current(P) =
    P.projectRevision == activeProjectRevision
AND P.sourceFingerprint == activeSourceFingerprint
AND P.assetManifestVersion == activeAssetManifestVersion
AND P.toolchainLock == activeToolchainLock
AND P.artifactFingerprint == installedArtifactFingerprint
AND P.deviceSession == activeDeviceSession
AND P.contractVersion == activeContractVersion
AND P.executionTruth in {OBSERVED, VERIFIED}
AND requiredEvidence(P) is current
AND no invalidation exists after P.observedAt
AND no policy or safety block is active
```

Only the preview coordinator may promote a candidate through this predicate. UI, workers, models, artifact inspection, and presentation reducers may report facts but cannot independently make a preview current.

Completion requires current mandatory evidence, selected profile maturity, required integration operationality, preview and artifact gates when declared, signing policy, reproducibility policy, and no unresolved blocking condition. `COMPLETED`, `VERIFIED`, `CURRENT`, `SUPPORTED`, `DELIVERED`, `FUNCTIONAL`, and `CERTIFIED` states are rejected when their required dependencies are missing, stale, invalidated, or model-authored.

### 36.5 Transaction domains and capability promotion

The runtime separates transaction domains because they have different rollback semantics:

```text
LocalTransaction
- stagedSourceRevision
- changedPaths
- checkpointId
- commitState: STAGED | VALIDATED | COMMITTED | ROLLED_BACK

DeviceTransaction
- deviceSessionId
- installedArtifactFingerprint
- appStateFingerprint
- observationState: REQUESTED | INSTALLED | LAUNCHED | OBSERVED | UNKNOWN
- cleanupPolicy

ExternalEffectTransaction
- externalEffectId
- idempotencyKey
- requestState
- reconciliationState
- compensationState
```

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

```text
SigningIdentityBinding
- artifactHash
- applicationId
- versionCode
- certificateFingerprint
- signingScheme
- keystoreIdentity
- buildVariant
- signingPolicyVersion
- inspectionEvidenceId
```

A release-signed claim is invalid without this binding and an observed signing-inspection result.

```text
SigningOperation
- operationId
- artifactId
- artifactHashBeforeSigning
- keystoreIdentityReference
- requestedCertificateFingerprint
- buildVariant
- signingScheme
- policyDecisionId
- operationState: REQUESTED | AUTHORIZED | IN_PROGRESS | OBSERVED |
                 FAILED | BLOCKED
- startedAt
- completedAt
- evidenceId

CertificateInspection
- inspectionId
- artifactId
- artifactHash
- applicationId
- versionCode
- observedCertificateFingerprint
- signingSchemesObserved
- expectedBindingRef
- result: PASSED | FAILED | UNKNOWN
- inspectedAt
- evidenceId
```

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

Provider-specific failures are classified into authentication, rate limiting, context overflow, unsupported capability, transport, timeout, cancellation, and provider-unavailable categories. Recovery policy chooses retry, fallback, context reduction, model change, or safe waiting according to the configured provider policy.

## 39. Sandbox and Process Separation

The host is divided into explicit process domains:

| Domain | Responsibility | Default access |
|---|---|---|
| Desktop shell | Chat, project navigation, preview framing, settings | IPC only; no direct project filesystem |
| Control-plane supervisor | Lifecycle, leases, task graph, permissions, persistence | Authority services and approved worker control |
| Worker process | Planning, coding, debugging, testing, visual QA | Isolated workspace and declared tools |
| Build process | Gradle, SDK, package manager, compiler | Project workspace, toolchain, declared network |
| Emulator/device manager | Device lifecycle, install, capture, Logcat | Emulator/device APIs only |
| Preview application | Runs generated Android app | Disposable app/device profile |
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

The Windows host must initialize without a provider, enter Offline Mode when network access is unavailable, preserve history and checkpoints, and resume eligible active sessions after restart or reboot. State writes use temp-file-plus-rename, file locks, versioned migrations, backups, and rollback. The installer and updater preserve user state and keep the previous version runnable if candidate startup, IPC, migration, or health checks fail.

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
| Lifecycle reducer | Durable state transitions and replay | Side effects |
| Policy authority | Permissions, sandbox profiles, resource budgets, network and device policy | AI strategy |
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
└── ProjectMemoryStore
        │
        ├── isolated worker processes
        ├── persistent PTY terminals
        ├── Android SDK/JDK/Gradle/ADB/emulator processes
        ├── provider bridge or direct provider adapters
        └── SQLite state and evidence database
```

No UI command may bypass the control plane to invoke a terminal, edit a file, launch an emulator, contact a provider, install a package, or promote an artifact.

---

## 45. Reducer, Event Store, and Transaction Manager

### 45.1 Session reducer

`SessionReducer` is a pure function over validated events. It receives the previous state and an event, validates the transition, and returns the next immutable state. Side effects are emitted as commands for supervised handlers.

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

Required event families include session lifecycle, task graph and worker, lease, transaction, terminal and process health, provider request, toolchain, preview and device, validation and evidence, recovery and checkpoint, artifact and signing, and decision trace events.

Replay reconstructs session state and can optionally re-run validation commands against a checkpoint without re-running model generation.

### 45.3 ConstructionTransactionManager

The manager creates a pre-mutation checkpoint, captures the project fingerprint and base revision, validates worker scope and operation capability, stages changes in a transaction workspace, runs syntax/graph/policy/mutation-budget checks, applies the candidate revision, re-indexes affected files, runs affected tests/build/preview checks, collects evidence, and commits or rolls back atomically.

Writes are serialized per project revision. Independent read-only analysis may proceed concurrently.

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

### 47.1 Project ingestion pipeline

```text
Discover workspace → normalize root and exclusions → classify Android files
→ compute hashes and project fingerprint → build lightweight index
→ resolve toolchain and identity → full semantic graph before mutation
```

The ingestion service understands Kotlin, Java, XML, manifests, Gradle files, JavaScript/TypeScript, native modules, resources, assets, SQL, JSON, YAML, TOML, lockfiles, signing configuration, emulator metadata, and test sources.

It excludes build output, caches, generated intermediates, credentials, keystores, and vendor code from model mutation unless a transaction explicitly authorizes a narrowly scoped operation.

### 47.2 Fingerprint model

The project fingerprint is a canonical hash over normalized relative paths, content hashes, selected metadata, technology-plan hash, toolchain-lock hash, and relevant external state. It is recomputed before every transaction commit, preview promotion, package operation, and self-development promotion.

TOCTOU protection rejects an operation when files changed outside the transaction, a worker’s base revision is stale, the selected toolchain changed, or the preview/device revision no longer corresponds to the candidate source.

### 47.3 Language adapter interface

```text
AndroidLanguageAdapter
- detect(path: str) -> LanguageDetectionResult
  - params: path: str (file path to detect language for)
  - returns: languageId, confidence, fileExtensions
  - errors: LanguageDetectionError
- parse(path: str, content: str) -> ParsedUnit
  - params: path: str, content: str (file content)
  - returns: languageId, ast, symbols, references, imports, metadata
  - errors: ParseError, UnsupportedLanguageError
- index_symbols(parsed_unit: ParsedUnit) -> SymbolIndex
  - params: parsed_unit: ParsedUnit
  - returns: symbols: list of SymbolEntry, references: list of ReferenceEntry
  - errors: IndexingError
- resolve_references(index: SymbolIndex) -> ResolvedIndex
  - params: index: SymbolIndex
  - returns: resolved: list of ResolvedReference, unresolved: list of UnresolvedReference
  - errors: ResolutionError
- calculate_affected_nodes(change: StructuredPatch) -> AffectedNodeSet
  - params: change: StructuredPatch
  - returns: affectedFiles: list, affectedSymbols: list, affectedModules: list
  - errors: ImpactAnalysisError
- validate_structured_patch(patch: StructuredPatch) -> PatchValidationResult
  - params: patch: StructuredPatch
  - returns: valid: bool, violations: list, affectedNodes: list
  - errors: PatchValidationError
- format_or_serialize(updated_unit: ParsedUnit) -> SerializedUnit
  - params: updated_unit: ParsedUnit
  - returns: content: str, format: str, encoding: str
  - errors: SerializationError
```

Adapters are selected by file type and technology plan. No single parser is mandatory for every Android project.

### 47.4 Impact analysis

The graph service calculates affected files, modules, resources, permissions, tests, emulator profiles, preview surfaces, and artifact outputs. The affected-test set is persisted with each transaction and evidence record, so long-horizon sessions can validate changed behavior without rebuilding unrelated areas unnecessarily.

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

The isolated environment controls JDK, Gradle, Android SDK, build tools, platform tools, NDK, CMake, ADB, emulator, Node/package manager when selected, Metro/Expo when selected, temporary directories, Gradle caches, package caches, and project-local configuration. Host PATH and unrelated user configuration are not trusted.

Hypervisor preflight MUST be a precondition of emulator readiness. The isolated environment MUST record firmware virtualization enabled, hypervisor platform present, and conflicting hypervisor consumers before emulator launch.

### 49.2 EnvironmentSnapshot

The environment snapshot includes toolchain lock hash, tool versions and hashes, selected emulator identity, API level and ABI, build variant, relevant environment variables, Gradle and package lock hashes, provider metadata without secrets, project fingerprint, and command policy. It is attached to build, recovery, preview, and artifact evidence.

### 49.3 Toolchain repair

Toolchain repair may install, hydrate, or repair components only through an approved operation capability. It records acquisition source, checksum, license metadata, before/after health, and rollback behavior. A repair that changes the lock requires a new checkpoint and technology-plan compatibility validation.

---

## 50. Preview Coordinator and Android Runtime Validation

`PreviewCoordinator` selects the least expensive valid preview mode for the current change and falls back when that mode cannot prove the requested behavior.

```text
Change classification → preview mode selection → build/install/reload
→ health and revision verification → screenshot/interaction/log evidence
→ PreviewRevision commit or stale/failure event
```

The coordinator supports incremental emulator install, Compose reload, React Native/Expo fast refresh, full APK reinstall, Nirman-managed local Android emulator execution, headless smoke tests, and diagnostic-only source preview. Diagnostic preview can support recovery but can never satisfy final completion.

A `PreviewRevision` includes source revision, artifact hash, device serial/profile, API level, build variant, technology-plan hash, preview mode, launch timestamp, health status, screenshot IDs, and Logcat evidence.

---

## 51. Repair Registry, Decision Trace, and Resource Governor

### 51.1 Repair registry

`AndroidRepairRegistry` maps structured failure fingerprints to repair strategies. Each pattern contains classifier, severity, likely cause, allowed scope, preconditions, operation type, retry budget, checkpoint rule, validation command, and evidence requirements.

Patterns cover JDK/Gradle/AGP/Kotlin/Compose compatibility, missing SDKs, Gradle/dependency conflicts, resource and manifest errors, DEX/R8 failures, NDK/native-module failures, Metro/Expo failures, emulator/ADB/install failures, runtime crashes, permission errors, visual/accessibility issues, and APK/signing failures.

A learned repair can be promoted into the trusted registry only after repeated successful validation across independent fixtures. Model suggestions remain untrusted until promoted by deterministic evidence.

### 51.2 DecisionTrace service

The service records concise decision summaries without hidden chain-of-thought. It stores inputs, constraints, candidate actions, selected action, policy checks, provider/model provenance, confidence, outcome, and evidence references. The UI can show why a technology, worker, repair, checkpoint, preview mode, or provider was selected.

### 51.3 ResourceGovernor

The governor monitors CPU, RAM, disk, checkpoint storage, emulator memory, Gradle memory, worker/provider concurrency, context size, log volume, build duration, and device slots. It can compact context, reduce concurrency, prune safe caches, stop redundant workers, select affected tests, defer nonessential checks, or use an approved lighter provider profile. It cannot weaken sandbox, permission, evidence, signing, or artifact gates.

---

## 52. Technical Acceptance Tests

The architecture is accepted only when killing the supervisor during a transaction leaves a recoverable event log and checkpoint; replaying events reconstructs the same authoritative session state; stale worker proposals are rejected without changing the project; changed files or toolchain locks invalidate pending transactions through TOCTOU checks; parallel workers can analyze and propose while conflicting writes are serialized; provider bridge restart and protocol mismatch do not corrupt the session; builds use the locked Android toolchain; preview promotion rejects stale source or artifact revisions; resource pressure changes scheduling without bypassing completion gates; and an APK is not promoted without revision, checksum, environment, validation, and signing evidence.
## 53. Integrated Workflow and Quality Services

### 53.1 WorkflowCoordinator

`WorkflowCoordinator` is the single control-plane service that connects the autonomous Android session contract to execution and completion. It owns no side-effect implementation itself; it emits typed commands to supervised services and consumes validated events.

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

```text
PreflightReport
├── report_id
├── session_id
├── technology_plan_hash
├── environment_snapshot_id
├── checks[]
│   ├── area
│   ├── status
│   ├── severity
│   ├── probability
│   ├── blocker
│   ├── evidence_ids
│   ├── mitigation
│   ├── fallback
│   └── autonomous_repair_allowed
└── overall_status
```

Routine environment repairs may be dispatched through authorized capabilities. The report must distinguish unavailable credentials, policy restrictions, required device absence, provider limitations, and repairable local deficiencies.

### 53.3 FailureModeRegistry

`FailureModeRegistry` stores preventive and reactive rules. A record contains failure fingerprint, detection source, classification, preconditions, prevention checks, permitted repair scope, strategy alternatives, retry budget, checkpoint rule, stop condition, and required evidence.

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

### 53.6 ArchitectureDriftDetector and ContractDriftDetector

The detectors compare the current project graph and build outputs with the approved contract and technology plan. They identify missing features, unreachable screens, undocumented permissions, data models without migrations, untested acceptance criteria, unauthorized dependencies, stale generated files, architecture-boundary violations, and preview/artifact revision mismatch.

A drift finding cannot be dismissed by editing the contract in place. Contract changes require a new version, rationale, reconciliation event, and revalidation of affected requirements.

### 53.7 RuntimeTraceAnalyzer

`RuntimeTraceAnalyzer` normalizes Logcat, stack traces, ANRs, native crash reports, install failures, permission denials, activity/service lifecycle events, and test-runner diagnostics. It produces stable failure fingerprints that feed `FailureModeRegistry`, `RecoveryAuthority`, and affected-test computation.

The analyzer must redact secrets, tokens, personal data, and full user content before persistence or provider submission.

### 53.8 DependencyHealthService

`DependencyHealthService` evaluates Gradle, Maven, npm/pnpm/yarn when selected, native module, and lockfile dependencies for version compatibility, transitive conflicts, known vulnerabilities, license policy, provenance, size impact, duplicate classes, and upgrade risk.

Dependency changes are proposed through ConstructionTransaction and require restore, build, relevant tests, security review, and rollback evidence before commit.

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

The architecture is complete when the coordinator can run preflight before expensive work, independent quality gates can block promotion, every mandatory requirement maps to executable tests, contract and architecture drift is detected, runtime traces feed repair classification, dependency health is checked before commit, handbook and release reports are generated from validated state, worker metrics are recorded, learned repairs require independent validation, and native isolation or remote side effects cannot weaken the core authority model.
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

```text
ReasoningStreamEvent
├── event_id
├── sequence
├── session_id
├── task_id
├── worker_id
├── trace_id
├── project_revision
├── event_type
├── status
├── title
├── summary
├── rationale_summary
├── uncertainty_summary
├── action_category
├── policy_reference_ids
├── evidence_ids
├── redaction_flags
├── created_at
└── supersedes_event_id?
```

Allowed event types are `UNDERSTANDING`, `CONSTRAINT`, `PLAN`, `ALTERNATIVE`, `DECISION`, `ACTION`, `OBSERVATION`, `RECOVERY`, `EVIDENCE`, `NEXT_STEP`, `WAITING`, and `COMPLETION`. Runtime events remain distinct from reasoning events. A reasoning event can explain a proposed action, but only a validated runtime event can authorize or prove that action.

### 55.3 Stream pipeline

```text
Provider response or worker result
        ↓
Schema validation
        ↓
Summarize into allowed event type
        ↓
Redact secrets, private data, source content, and hidden instructions
        ↓
Bind to session/task/worker/revision
        ↓
Append atomically with monotonic sequence
        ↓
Publish over authenticated local event channel
        ↓
Acknowledge and checkpoint delivery
```

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

### 56.1 BrandAssetWorker

`BrandAssetWorker` is the specialized worker responsible for turning user brand intent, screenshots, supplied assets, and the AndroidConstructionContract into validated Android visual assets. It may propose generated or vector assets, but the runtime validates every output before integration and promotion.

Responsibilities include brand-intent extraction, BrandManifest creation, asset planning, provider/image-generation requests, vector or deterministic local fallback, adaptive-icon preparation, splash integration, notification-icon preparation, density/format conversion, resource integration, content hashing, visual inspection, accessibility checks, and regeneration after a branding change.

The worker is scoped to the asset transaction and cannot modify unrelated source, change the technology plan, grant permissions, or mark the APK complete.

### 56.2 BrandManifest and AssetManifest schemas

```text
BrandManifest
├── manifest_id
├── version
├── app_identity
├── semantic_brand_description
├── source_prompt_hash
├── source_screenshot_ids
├── color_system
├── typography_intent
├── spacing_intent
├── theme_behavior
├── requested_asset_types
└── accessibility_expectations

AssetManifestEntry
├── asset_id
├── brand_manifest_version
├── asset_type
├── source_intent
├── source_screenshot_ids
├── output_path
├── format
├── dimensions
├── density_or_adaptive_variant
├── content_hash
├── provider_model_metadata
├── generation_status
├── integration_status
├── validation_status
└── regeneration_history
```

Schemas are versioned and strict. Each asset entry is linked to the source revision and ConstructionTransaction that generated or changed it.

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
2. BrandAssetWorker cannot modify unrelated source or bypass transaction scope.
3. Adaptive, legacy, monochrome, splash, notification, in-app, and theme assets are validated according to the target Android configuration.
4. Resource references and manifest entries resolve before build.
5. APK extraction confirms requested assets are actually packaged.
6. Stale AssetManifest versions cannot satisfy PreviewRevision or artifact gates.
7. Branding changes invalidate affected evidence and regenerate only impacted assets.
8. Provider failure and fallback behavior are explicit and replayable.
9. Placeholder-only output blocks completion when branded assets were requested.
## 57. Locked Implementation Stack and Supervisor Process Architecture

### 57.1 Implementation stack

Nirman v1 uses C#/.NET with WinUI 3 and Windows App SDK for the Windows desktop application. XAML is the presentation language and WinUI 3 Fluent Design is the initial design system. The presentation layer uses a presentation-only MVVM or equivalent state architecture.

Rust with Tokio owns the authoritative local runtime and control plane. SQLite is the execution ledger. SQLx is the preferred asynchronous access layer, with rusqlite permitted only when isolated safely from Tokio scheduling.

The Windows runtime uses native APIs including ConPTY, restricted process tokens, Windows Job Objects, ACL-scoped workspaces, environment filtering, process-tree supervision, and resource quotas.

The Android toolchain remains externally installed or managed by Nirman's toolchain authority.

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
├── LifecycleAuthority
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
└── SQLite execution ledger
```

The first implementation may host the Rust control-plane modules in-process with the WinUI 3 application to reduce initial process complexity. The production durable-autonomy architecture separates Nirman.exe from NirmanSupervisor.exe.

### 57.3 SupervisorConnection

```text
SupervisorConnection
├── connection_id
├── ui_instance_id
├── supervisor_instance_id
├── protocol_version
├── installation_identity
├── authenticated_user_scope
├── project_scope
├── last_event_sequence
├── heartbeat_state
├── supervisor_health
└── reconnect_policy
```

The connection performs a protocol/version handshake, authenticates the UI instance, validates project scope, subscribes to durable events after a supplied sequence, reports supervisor health, and handles reconnect after UI crash, UI restart, supervisor restart, Windows reboot, and sleep/resume. A UI connection cannot impersonate another project, publish forged events, or invoke a command outside its capability scope.

The canonical `UICommandEnvelope`, `ProjectionSnapshot`, `UIResponseEnvelope`, `UIErrorEnvelope`, and `EventSubscription` schemas, command registry, transaction ownership, and replay rules are defined by technical architecture §81. `SupervisorConnection` carries the authenticated transport and cursor required by that contract.

### 57.4 SupervisorLifecycle, singleton invariant, and recovery scan

One user + one installation → one authoritative supervisor instance. The supervisor is a per-user singleton.

Singleton enforcement:
- Mutex/lock ownership: the supervisor acquires a named Windows mutex on startup. A second instance detects the existing mutex, refuses to start, and exits.
- Stale supervisor detection: if the mutex exists but the owning process is dead, the new instance takes ownership after verifying no active leases or tasks are in flight.
- Split-brain prevention: only one supervisor may hold the installation lease and write to the SQLite ledger at a time. The lease is fenced by a monotonic token.
- Second-instance refusal: any additional supervisor process beyond the singleton must terminate immediately without acquiring leases or opening the ledger.
- Supervisor takeover after crash: on crash recovery, the new supervisor fences abandoned leases, reconciles unknown outcomes, and resumes only eligible operations.

`NirmanSupervisor.exe` starts at Windows user login when an eligible session or scheduled task exists, owns all long-running process trees, and records graceful or abnormal shutdown. On startup it validates SQLite integrity, migrations, leases, checkpoints, project fingerprints, process records, terminal sessions, preview revisions, and pending provider requests.

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

### 57.5 SQLite execution ledger

SQLite stores transactional execution metadata, not merely settings:

```text
projects, sessions, tasks, task_revisions, task_states,
workers, worker_contracts, worker_leases, handoffs,
events, event_sequences, approvals, policies,
checkpoints, recovery_records, provider_profiles,
provider_capabilities, terminal_sessions, process_records,
preview_revisions, device_profiles, validation_runs,
evidence_records, artifacts, toolchain_manifests,
project_locks, decision_records, reasoning_stream_events
```

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

Provider adapters normalize configured Chat Completions, Responses-style, message-oriented, local-compatible, vision, tool-call, structured-output, cancellation, streaming, capability, and retry behavior. Partial provider output never executes. Complete structured proposals still require scope, schema, policy, revision, capability, and transaction validation.

### 57.8.1 ProviderAdapter interface

```text
ProviderAdapter
- adapterId
- adapterVersion
- providerId
- compatibilityMode: OPENAI_COMPATIBLE | ANTHROPIC_COMPATIBLE
- protocol: chat_completions | responses | messages | custom
- supportedInputModalities: text | image | audio | tool_call | structured_output
- supportedOutputModalities: text | tool_call | structured_output | reasoning
- streamingSupported: bool
- capabilityProfile: ProviderCapabilityProfile

ProviderAdapter operations
- initialize(profile: ProviderProfile) -> AdapterInitializationResult
  - params: profile: ProviderProfile
  - returns: adapterId, adapterVersion, protocol, capabilities, healthCheck
  - errors: AdapterInitializationError, UnsupportedProtocolError
- healthCheck() -> AdapterHealthResult
  - params: none
  - returns: healthy: bool, latencyMs, modelReachable, capabilitiesValid, checkedAt
  - errors: HealthCheckError
- buildRequest(request: ModelRequest) -> ProviderRequest
  - params: request: ModelRequest (messages, tools, schema, context, cancellation)
  - returns: providerRequest: dict, requestHash, estimatedTokens
  - errors: RequestBuildError, UnsupportedFeatureError
- sendRequest(providerRequest: ProviderRequest) -> ProviderResponse
  - params: providerRequest: ProviderRequest
  - returns: rawResponse: dict, responseId, modelId, finishReason, usage, latencyMs
  - errors: ProviderRequestError, TimeoutError, RateLimitError, AuthenticationError
- normalizeResponse(rawResponse: dict) -> NormalizedResponse
  - params: rawResponse: dict (raw provider response envelope)
  - returns: textBlocks, imageBlocks, toolCalls, structuredOutput, usage, finishReason, reasoningMetadata
  - errors: NormalizationError, MalformedResponseError
- streamRequest(providerRequest: ProviderRequest) -> StreamEvent
  - params: providerRequest: ProviderRequest
  - returns: StreamEvent (token, tool_call_delta, structured_output_delta, done, error)
  - errors: StreamError, TimeoutError
- cancelRequest(requestId: str) -> CancelResult
  - params: requestId: str
  - returns: cancelled: bool, cancelTimestamp
  - errors: CancelError
- detectCapability(modelId: str) -> CapabilityDetectionResult
  - params: modelId: str
  - returns: modelId, capabilities: list, confidence, detectedAt
  - errors: CapabilityDetectionError
- validateResponse(rawResponse: dict) -> ResponseValidationResult
  - params: rawResponse: dict
  - returns: valid: bool, violations: list, toolCallIds: list, schemaCompliant: bool
  - errors: ResponseValidationError

### 57.9 Git and worktree subsystem

Git is a first-class subsystem for checkpoints, rollback, worker isolation, reconciliation, diffs, revision identity, recovery branches, and artifact provenance. Parallel workers use isolated worktrees or copy-on-write fallback. Reconciliation produces an integration revision only after conflict, dependency, requirement, and test-impact checks pass.

### 57.10 Architecture acceptance criteria

The architecture acceptance criteria are the conditions that must be met for the architecture to be considered correct. They are verified by runtime fixtures and evidence, not by documentation certification alone.

The architecture acceptance criteria are satisfied when the UI can restart while the supervisor continues a task; the supervisor can start after Windows reboot and recover eligible sessions; SQLite reconstructs the same state after event replay; ConPTY terminals survive reconnect; stale UI projections cannot mutate authority; provider proposals cannot bypass ToolBroker or PolicyAuthority; the native WinUI editor and terminal surfaces remain presentation components; Android toolchains are supervised locally; and the final APK remains bound to source revision, toolchain lock, preview, evidence, and artifact checksums.


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
      ├── BackpressureController
      ├── CancellationPropagationManager
      ├── DecisionNodeManager
      ├── UncertaintyRegistry
      ├── PlanCompiler / Replanner
      └── ExecutionHistoryManager
```

These modules produce proposals and state transitions, but LifecycleAuthority, PolicyAuthority, ToolBroker, ConstructionTransactionManager, EvidenceAuthority, and ArtifactAuthority remain the non-delegable authorities.

### 58.2 AgentExecutionKernel contract

```text
AgentExecutionKernel
- start(goal_id, session_id)
- observe(observation)
- propose(proposal)
- authorize(proposal)
- execute(authorized_action)
- observe_result(result)
- evaluate_progress()
- continue_or_recover()
- delegate(request)
- validate(plan)
- replan(trigger)
- complete(evidence_set)
```

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
  ├── VALIDATE
  ├── RECOVER
  ├── DELEGATE
  ├── REPLAN
  └── COMPLETE
```

Only `AgentLoopReducer` may commit a lifecycle transition. A provider delta, partial stream, worker message, or UI action may request a transition but cannot apply one directly.

### 58.3 Durable schemas

```text
AgentLoopRecord
- loop_id
- session_id
- task_id
- agent_instance_id
- state
- state_version
- goal_revision
- plan_revision
- project_revision
- last_observation_id
- last_proposal_id
- progress_status
- retry_strategy
- cancellation_scope
- created_at
- updated_at

AgentProposal
- proposal_id
- source_provider
- source_worker
- input_revision
- action_type
- action_arguments
- expected_observation
- required_capabilities
- risk_class
- schema_status
- policy_status
- transaction_status
- evidence_status

AgentProfile
- profile_id
- model_profile
- reasoning_mode
- context_strategy
- skill_ids
- tool_capabilities
- permission_profile
- autonomy_level
- generation_parameters
- max_children
- resource_policy
- recovery_policy
- validation_policy
- memory_policy
```

A proposal is immutable after validation. Any change creates a new proposal revision linked to the evidence or contradiction that caused it.

### 58.4 SkillRuntime

`SkillRuntime` resolves skill discovery, compatibility, composition, input binding, context assembly, execution, tool mediation, output validation, and evidence capture. It verifies skill version, required ToolBroker version, Android profile, worker role, input/output schema, permissions, and resource requirements before execution.

```text
SkillExecutionRecord
- execution_id
- skill_id
- skill_version
- task_id
- worker_id
- agent_instance_id
- input_hash
- context_references
- tools_used
- permissions_used
- files_changed
- evidence_ids
- duration_ms
- model_usage
- result_status
- rollback_reference
```

A skill composition is a directed acyclic graph with bounded depth, explicit inputs/outputs, shared revision identity, and a single validation contract. A composed skill cannot grant another skill permissions.

### 58.5 SwarmPlanner and delegation

`SwarmPlanner` analyzes change surface, dependencies, symbols, requirements, risk, validation cost, capability graph, workspace capacity, emulator availability, provider concurrency, and resource pressure. It emits a `SwarmPlan` containing parallel groups, serialized dependencies, worker profiles, interfaces, leases, capacity reservations, and integration checkpoints.

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

Every operation carries parent task, cancellation lineage, input references, expected outputs, required capabilities, profile, permissions, resource reservation, workspace lease, and validation requirements. Dynamic worker creation is bounded by policy and never changes the authority graph.

### 58.6 KnowledgeLedger and TaskBlackboard

`KnowledgeLedger` stores typed, scoped `KnowledgeArtifact` records. `TaskBlackboard` is a task-scoped projection containing the goal, requirements, architecture, decisions, constraints, assumptions, active workers, completed/blocked work, findings, conflicts, evidence, known failures, and next actions. A separate graph database is not implied. When typed relationships are required, the ledger may store:

```text
KnowledgeRelation
- relationId
- fromArtifactId
- toArtifactId
- relationType: derived_from | supports | contradicts | invalidates |
                supersedes | depends_on
- sourceEventId
- projectScope
- createdAt
- invalidatedAt
```

`KnowledgeRelation` is a scoped projection edge and never grants authority. It must not allow identifiable project content to cross the memory boundary.

Workers may read, propose, attach evidence, request changes, and retrieve relevant entries. Only authoritative services may commit a decision, change the task graph, mark a requirement complete, change policy, or promote an artifact.

```text
KnowledgeArtifact
- artifact_id
- kind: finding | decision | constraint | assumption | architecture_fact |
        failure_pattern | test_result | artifact | environment_fact
- source_worker
- source_task
- project_revision
- confidence
- evidence_ids
- valid_from
- valid_until
- scope
- supersedes
```

### 58.7 WorkspaceLeaseManager and ToolSessionRegistry

`WorkspaceLeaseManager` gives every worktree or copy-on-write workspace one owner, one parent checkpoint, a renewable heartbeat, an expiration, cleanup rules, and recovery rules. Stale leases become recoverable resources only after process and revision checks.

`ToolSessionRegistry` represents terminals, ADB, emulators, debuggers, LSPs, preview processes, and other long-lived tools as reconnectable sessions:

```text
ToolSession
- session_id
- tool_type
- owner_worker
- task_id
- project_id
- environment_fingerprint
- process_group
- state
- capability_scope
- input_policy
- output_reference
- heartbeat
- reconnect_policy
- cleanup_policy
- evidence_ids
```

A tool session may be reattached after worker replacement or UI restart, but reattachment does not expand its capability scope.

### 58.8 Tool Capability Graph and environment planning

`ToolCapabilityGraph` maps an outcome to capability requirements, skills, worker profiles, tools, and environment prerequisites. For example, Android BLE validation may require Android APIs, a compatible SDK, a native module, Bluetooth permissions, ADB, an emulator or selected Nirman-managed local Android emulator, and device-test capability.

`EnvironmentCapabilityPlanner` evaluates each prerequisite before expensive execution and classifies it as `AVAILABLE`, `REPAIRABLE`, `USER_REQUIRED`, or `UNAVAILABLE`. It records the toolchain lock, environment fingerprint, repair attempt, and evidence used for the classification.

Platform dimensions are explicit (build spec §79). The planner resolves host and target platforms through `TargetPlatformResolver`, consults the `PlatformCapabilityRegistry` matrix as a prior for preflight, and classifies cross-compilation capability and native target-runtime capability as separate prerequisites. It never derives native runtime capability from a successful build or cross-build: the cross-build admission decision point and the native-validation gate are the §84.3 decision points owned by the existing authorities, and a classification is never raised by model assertion.

### 58.9 ValidationPlanner and mutation regression analysis

`ValidationPlanner` chooses validation from changed files, symbols, call graph, route graph, dependency graph, requirement traceability, project type, risk, previous failures, emulator profiles, and resource availability. `MutationRegressionAnalyzer` predicts affected behavior and expands validation when a change touches a manifest, permission, navigation route, data model, native module, build file, authentication boundary, or shared UI component.

```text
ValidationPlan
- plan_id
- task_id
- project_revision
- affected_nodes
- required_checks
- focused_checks
- expanded_checks
- risk_score
- device_matrix
- resource_reservations
- stop_conditions
- evidence_requirements
```

### 58.10 TrajectoryReplayEngine and SimulationExecutor

`TrajectoryReplayEngine` replays recorded observations, structured proposals, tool calls, tool results, state transitions, evidence references, and next decisions against a new model, prompt, skill, schema, or runtime. Replay is read-only with respect to real projects and cannot send external side effects.

`SimulationExecutor` produces a dry-run plan with predicted workers, skills, files, commands, permissions, devices, tests, resources, and risks. It uses explicit statuses: `PREDICTED`, `SIMULATED`, `OBSERVED`, and `VERIFIED`. It must not mutate source files, execute commands, start an emulator, or claim that a predicted test passed.

### 58.11 Deadlock, backpressure, and cancellation

`DeadlockDetector` analyzes cycles across task dependencies, worker waits, resource reservations, approvals, workspace leases, and ToolSessions. A detected cycle produces a typed finding and may trigger reorder, replacement, lease recovery, cancellation, replanning, or a `DecisionNode`.

`BackpressureController` reserves and queues Gradle processes, emulator slots, Nirman-managed local Android emulators, GPU capacity, storage, and provider concurrency. It applies priority and fairness, exposes waiting reasons, and reduces parallelism before system pressure becomes failure.

`CancellationPropagationManager` propagates cancellation from goal to task graph, workers, skills, ToolSessions, child processes, PTY, emulator actions, and pending provider requests. Each node supports graceful cancellation, forced termination, cleanup, checkpoint preservation, and rollback semantics.

Independent worker or skill pause must preserve context references, leases, ToolSessions, checkpoints, and unresolved questions. Unrelated workers may continue.

### 58.12 DecisionNodeManager, uncertainty, and replanning

`DecisionNodeManager` represents ambiguous architecture or recovery choices with a question, options, evidence, trade-offs, recommendation, impact, and resume conditions. A decision node is separate from a generic command approval and remains bound to a task and plan revision.

`UncertaintyRegistry` tracks `KNOWN`, `PROBABLE`, `ASSUMED`, `UNKNOWN`, `CONTRADICTED`, `VERIFIED`, and `BLOCKED` facts with source, confidence, evidence, expiry, scope, and next action. `ContradictionDetector` creates a controlled decision revision when requirements, assumptions, device constraints, toolchains, or architecture facts conflict.

`PlanCompiler` and `Replanner` compile a new plan when evidence invalidates the current one. Each revision records `planRevision`, `supersedesPlan`, reason, trigger evidence, affected nodes, and recovery/migration action.

### 58.13 ExecutionHistoryManager

`ExecutionHistoryManager` separates active state from retained history:

| Tier | Content | Access |
|---|---|---|
| Hot | Current graph, active workers, current plan, latest evidence, blockers | Kernel context |
| Warm | Recent events, terminal summaries, checkpoints, preview/test results | Task request |
| Cold | Older events, handoffs, failures, superseded plans, screenshots | Indexed retrieval or replay |
| Archived | Full traces, old artifacts, crash dumps, retired sessions | Explicit audit restore |

Compaction must preserve semantic summaries, evidence links, revision identity, artifact provenance, and replay references. Garbage collection cannot delete active checkpoint parents, mandatory completion evidence, unresolved failure evidence, or artifact provenance.

### 58.14 Runtime invariants

1. Only the reducer commits lifecycle state.
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

## 59. Memory/Context Runtime

**ContractId:** `CONTRACT.RUNTIME.MEMORY, CONTRACT.RUNTIME.CONTEXT`  
**Authoritative build-spec section:** §38 / §53  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §53. Extends §19 (Context Scaling Architecture) and §31 (Runtime Memory and Learning Boundaries), which remain the authority on retrieval modes and memory scopes. This section adds the assembly and re-grounding components.

### 59.1 Components

| Component | Responsibility |
|---|---|
| MemoryWriter | Writes classified memory records from validated events only |
| MemoryStore | Persists records with scope, provenance, and retention |
| ConstraintRegistry | Holds active constraints and locked decisions for a session |
| ContextAssembler | Builds a ContextPackage for every model call |
| RegroundingService | Re-reads goal, constraints, decisions, and evidence at checkpoints |
| RedactionFilter | Removes secrets and unclassified private content before assembly |

### 59.2 Memory record schema

```text
MemoryRecord
- recordId
- projectId
- sessionId
- class: DECISION | CONSTRAINT | FACT | FAILURE | ARTIFACT
- statement
- sourceEventIds
- sourceRevision
- confidence
- scope: task | project | runtime_improvement
- retentionPolicy
- supersededBy
- createdAt
```

`sourceEventIds` must be non-empty. MemoryWriter must reject a record with no source event, which structurally prevents model claims from becoming memory.

### 59.3 ContextAssembler algorithm

The assembler must, in order: load active constraints and locked decisions from ConstraintRegistry; reserve their token cost first; select the retrieval or large-context mode per §19; rank candidate files by impact-graph relevance to the current surface; apply RedactionFilter; fill remaining budget; and emit the ContextPackage manifest defined in build spec §53.3 to the event ledger.

Constraint content is never evicted for budget. When file content must be reduced, the assembler records each omission in `omittedForBudget`.

### 59.4 Re-grounding trigger conditions

RegroundingService must run at checkpoint creation, before plan recompilation, after a runtime directive is accepted, after user-edit reconciliation, on resume from pause or restart, and after a candidate branch selection.

### 59.5 Persistence and isolation

Memory records are stored in the SQLite execution ledger keyed by project. Cross-project reads are prevented at query level by mandatory project scoping. Runtime-improvement records are stored in a separate table with no path, identifier, or content columns.

### 59.6 Architecture tests

Assembly is correct only when a locked decision remains present in every subsequent ContextPackage until superseded; when a memory write with no source event is rejected; when a project-scoped query cannot return another project's records; and when a historical ContextPackage is reproducible from the ledger.

## 60. Peer Coordination and Semantic Reservations

**ContractId:** `CONTRACT.RUNTIME.RESERVATION`  
**Authoritative build-spec section:** §54  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §54. Extends §8 (Workspace Isolation and Reconciliation) and §46 (Lease and Capability Runtime), which remain the authority on workspace leases. This section adds the semantic layer above file ownership.

### 60.1 Components

| Component | Responsibility |
|---|---|
| ReservationRegistry | Grants, renews, revokes, and queries semantic reservations |
| SurfaceIndex | Maps symbols, routes, schema tables, resources, and permissions to files |
| ConflictDetector | Evaluates requested reservations against held reservations |
| StaleContractInvalidator | Invalidates read_stable holders when a surface changes |
| CommitBarrier | Serializes proposal merges and revalidates freshness |

### 60.2 Reservation state machine

```text
requested -> granted -> renewed* -> released
requested -> denied
granted   -> expired      (lease not renewed)
granted   -> revoked      (authority decision)
granted   -> invalidated  (surface changed under read_stable)
```

Only the deterministic runtime performs state transitions. A worker may request, renew, and release, but never grant.

### 60.3 Conflict matrix

| Held \ Requested | read_stable | modify | delete | create |
|---|---|---|---|---|
| read_stable | allow | deny | deny | n/a |
| modify | deny | deny | deny | n/a |
| delete | deny | deny | deny | n/a |
| create | n/a | n/a | n/a | deny |

A denial returns the holding worker and task so the requester can request a handoff rather than retry blindly.

### 60.4 Invalidation propagation

When a mutation commits on a surface, StaleContractInvalidator must find every `read_stable` reservation on that surface, mark each holder's dependent work `unvalidated`, clear affected validation evidence, and notify the holder's task. Work marked unvalidated cannot reach CommitBarrier until revalidated.

### 60.5 CommitBarrier checks

At the barrier, in order: verify all reservations held by the proposal are still `granted`; verify no dependent surface changed after the proposal's validation timestamp; verify validation evidence exists for the changed surfaces; then apply the mutation transactionally through the reducer of §45. Any failed check rejects the proposal with a typed reason.

### 60.6 Architecture tests

Coordination is correct only when two workers requesting `modify` on one symbol produce one grant and one typed denial; when a symbol rename invalidates a dependent worker's `read_stable` work; and when a proposal validated before a dependent change is rejected at the barrier rather than merged.

## 61. User/Edit Reconciliation Coordinator

**ContractId:** `CONTRACT.RUNTIME.RECONCILIATION`  
**Authoritative build-spec section:** §55  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §55. No existing section covers concurrent human editing; this is a new runtime component that consumes the reservation layer of §60 and the mutation records of §45.

### 61.1 Components

| Component | Responsibility |
|---|---|
| ProjectWatcher | Observes filesystem changes in the project tree |
| OriginClassifier | Determines whether a change is RUNTIME, USER, EXTERNAL, or GENERATED |
| MutationLedgerIndex | Provides expected content fingerprints for runtime-authored writes |
| ReconciliationCoordinator | Pauses affected mutation and drives re-derivation |
| BaselineUpdater | Adopts user content as the new baseline |

### 61.2 Origin classification algorithm

For each observed change the classifier computes the file fingerprint and compares it to the fingerprint recorded by the last runtime mutation for that path. A match classifies RUNTIME. A mismatch on a path under an active runtime reservation classifies USER or EXTERNAL. Paths matching generated-output patterns and build directories classify GENERATED and are excluded from reconciliation and from context assembly.

Classification must never rely on modification time alone, because build steps and editors both rewrite timestamps.

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

BaselineUpdater must never write the runtime's prior version over user content. The evidence store must not accept validation for a surface whose fingerprint changed after the validation ran. The completion authority of §23 must reject a completion claim citing pre-edit evidence.

### 61.5 Attribution in evidence

Every mutation record carries an `origin` field. Final reports must render user-originated changes distinctly from runtime-originated changes so the user is never told the runtime produced their own edit.

### 61.6 Architecture tests

Reconciliation is correct only when a user edit during an active run survives to the final artifact; when validation predating the edit is discarded; when a user edit contradicting a locked decision produces a decision node; and when generated build output never triggers reconciliation.

## 62. Stateful E2E Engine

**ContractId:** `CONTRACT.RUNTIME.E2E`  
**Authoritative build-spec section:** §56  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §56. Extends §35 (Complete Android Capability Fixture Contract) and §50 (Preview Coordinator and Android Runtime Validation), which remain the authority on emulator sessions and fixtures.

### 62.1 Components

| Component | Responsibility |
|---|---|
| ScenarioRegistry | Stores scenario definitions and requirement links |
| ScenarioCompiler | Translates a scenario into instrumentation and ADB steps |
| SeedDataProvisioner | Establishes preconditions through the app's own data layer |
| ScenarioExecutor | Runs steps against a emulator session and records results |
| StateProbe | Verifies persisted state after process death or restart |
| ScenarioEvidenceWriter | Writes step results, screenshots, and Logcat windows |

### 62.2 Step and assertion schema

```text
ScenarioStep
- stepIndex
- kind: ui_action | system_event | wait_for | assert | probe_state
- target
- input
- timeoutMs
- expected
- result: passed | failed | skipped | error
- screenshotRef
- logcatRange
```

System events must include process death, configuration change, permission grant and deny, network loss, and app backgrounding, since these are the states single-screen validation misses.

### 62.3 Determinism enforcement

ScenarioExecutor must use explicit `wait_for` conditions and never fixed sleeps as synchronization. A scenario that passes and fails across repeated runs on the same revision and device must be marked `deterministic: false` and excluded from completion evidence until stabilized.

### 62.4 Seed provenance

SeedDataProvisioner records how each precondition was established. Seeded state is labeled in evidence so it cannot be mistaken for behavior the application produced, satisfying the honesty invariant of build spec §66.1.

### 62.5 Persistence

Scenario definitions, runs, step results, and evidence references are stored in the execution ledger and linked to requirement identifiers, enabling the traceability chain of build spec §66.3.

### 62.6 Architecture tests

The engine is correct only when a data-persistence scenario detects an app that loses data on process death; when a flaky scenario is quarantined rather than reported as passing; and when every requirement's scenario link resolves in the ledger.

### 62.7 Adapter-side resolution

Test execution MUST route through `AndroidDeviceAdapter` per CLAUSE.PREVIEW_SYNC.ADAPTER_BOUND. The technology adapter resolves the binding but MUST NOT execute the test.

## 63. Regression Localization Service

**ContractId:** `CONTRACT.RUNTIME.LOCALIZATION`  
**Authoritative build-spec section:** §62  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §62. Extends §30 (Self-Improvement Manager) failure analysis and §58 mutation/regression intelligence, which remain the authority on predicting affected validation.

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

Bisection must reuse checkpoints from the two-tier checkpoint architecture of §18 rather than rebuilding, because full rebuild bisection is prohibitively expensive for Android projects.

### 63.3 Repair scoping

The identified cause surface becomes the permitted repair scope. The mutation broker must reject a repair mutation outside that scope unless an authority records an explicit widening reason. This prevents broad regeneration from destroying validated work.

### 63.4 Failure signature schema

```text
FailureSignature
- signatureId
- symptomKind: compile | lint | assertion | scenario | runtime_crash | performance
- symptomFingerprint
- causeClass
- causeSurfaceKind
- successfulRepairKind
- occurrences
- lastSeenAt
```

Signatures are written as FAILURE memory records per §59.2 and are project-scoped unless anonymized for runtime-improvement memory.

### 63.5 Architecture tests

Localization is correct only when an injected single-line regression is attributed to its mutation; when a repair mutation outside the cause scope is rejected; when bisection consumes existing checkpoints without full rebuilds; and when an unlocalized regression escalates rather than triggering rewrite.

## 64. Verification Orchestrator

**ContractId:** `CONTRACT.RUNTIME.VERIFICATION`  
**Authoritative build-spec section:** §57  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §57. Extends §53 (Integrated Workflow and Quality Services) and §58 ValidationPlanner, which remain the authority on selecting which validation to run. This section adds in-loop verification sequencing.

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
  -> DiagnosticRunner on affected surface
       new diagnostic -> repair or revert (mutation does not advance)
  -> IncrementalCompiler on affected module
       failure -> repair or revert
  -> affected assertions executed
       failure -> RegressionLocalizationService (§63)
  -> mutation marked verified, dependent work unblocked
```

A mutation that has not passed this sequence is `unverified` and cannot be cited as evidence, cannot pass the CommitBarrier of §60, and cannot be included in a promoted artifact.

### 64.3 Assertion ordering enforcement

For a requirement with observable behavior, AssertionAuthor must persist the assertion with a `authoredAtRevision` preceding the implementation revision. VerificationLedger marks assertions authored after a passing implementation as `post_hoc`. The completion authority weights `post_hoc` assertions lower and must not accept them as sole evidence for a critical requirement.

### 64.4 Vacuity check

For requirements marked critical, MutationProber must inject at least one fault into the implementation and confirm the assertion set fails. An assertion set that passes against the injected fault is recorded as `vacuous` and rejected as evidence.

### 64.5 Verification record schema

```text
VerificationRun
- runId
- mutationId
- method: diagnostics | lint | incremental_compile | unit | scenario | screenshot | mutation_probe | property_probe | performance
- surfaces
- outcome: passed | failed | vacuous | skipped
- evidenceRefs
- durationMs
- ranAtRevision
```

### 64.6 Architecture tests

Orchestration is correct only when a mutation introducing a compile error cannot advance; when an assertion authored after implementation is flagged `post_hoc`; when a vacuous assertion set for a critical requirement is rejected; and when every promoted artifact contains only verified mutations.

## 65. Android Emulator Scenario Coordinator

**ContractId:** `CONTRACT.RUNTIME.DEVICE_MATRIX`  
**Authoritative build-spec section:** §59  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §59. Extends §49 (Android Toolchain Authority and Environment) and §50 (Preview Coordinator), which remain the authority on device health and session lifecycle.

### 65.1 Components

| Component | Responsibility |
|---|---|
| DeviceMatrixResolver | Resolves the declared matrix against actually available devices |
| DevicePool | Allocates and recycles Nirman-managed local Android emulator sessions |
| ScenarioDistributor | Assigns scenarios to devices and orders execution |
| DivergenceAnalyzer | Compares per-device outcomes for the same scenario |
| CoverageReporter | Reports per-device scenario coverage and declared gaps |

### 65.2 Resolution and admission

DeviceMatrixResolver must classify each declared entry as `available`, `unavailable`, or `user_required` before execution begins, using the toolchain authority of §49. The run proceeds only when the primary device is available. Unavailable secondary entries are recorded as declared coverage gaps, never as passes.

### 65.3 Pool constraints

DevicePool must respect the resource reservations of the backpressure controller, since concurrent emulator boots are among the most expensive host operations. Boot cost estimates come from the ResourceProfiler of §69. The pool must serialize boots when host capacity cannot sustain parallel emulators.

### 65.4 Divergence record

```text
ScenarioDivergence
- divergenceId
- scenarioId
- passingEmulatorProfiles
- failingEmulatorProfiles
- differingAttributes: apiLevel | density | formFactor | abi | vendor
- classification: defect | environment_limitation
- evidenceRefs
```

Default classification is `defect`. Classification as `environment_limitation` requires cited evidence that the failure originates in the device or vendor rather than the application.

### 65.5 Capability status mapping

CoverageReporter maps results to the build spec §5.6 vocabulary: all matrix devices passed yields `SUPPORTED`; primary passed with declared gaps yields `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS`; primary passed and a secondary failed yields `DEGRADED` with the divergence cited; primary unavailable yields `USER_REQUIRED`.

### 65.6 Architecture tests

Coordination is correct only when a missing secondary device produces a declared gap in the report; when a scenario passing on one API level and failing on another is recorded as a divergence defect; and when emulator boots serialize under constrained host capacity.

## 66. Runtime Directive Service

**ContractId:** `CONTRACT.RUNTIME.DIRECTIVE`  
**Authoritative build-spec section:** §61  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §61. Extends §7 (Scheduler and Background Execution) and §16 (Goal Mode and Long-Horizon Execution), which remain the authority on task lifecycle and cancellation semantics.

### 66.1 Components

| Component | Responsibility |
|---|---|
| DirectiveIntake | Validates and admits directives from the user or policy |
| DirectiveValidator | Rejects directives requesting prohibited behavior |
| DirectiveQueue | Holds admitted directives until the next decision point |
| ConstraintRegistrar | Registers accepted directives as active constraints |
| PlanReconciler | Determines which plan steps and evidence remain valid |

### 66.2 Application at decision boundaries

A directive is applied only at a kernel decision point, never inside a mutation, tool call, or transaction. The kernel drains DirectiveQueue at each decision point, applies directives in issue order, and records the applied set in the event ledger before selecting the next action.

### 66.3 Validation rules

DirectiveValidator must reject a directive that requests raising a permission ceiling, bypassing an evidence requirement, approving its own decision node, disabling a policy gate, or marking a requirement complete. Rejection is recorded with the reason and surfaced to the user; a rejected directive never partially applies.

### 66.4 Plan reconciliation outcomes

```text
DirectiveEffect
- directiveId
- appliedAtEventId
- planRevisionBefore
- planRevisionAfter
- stepsUnchanged
- stepsInvalidated
- stepsAbandoned
- evidenceInvalidated
- workPreserved
```

PlanReconciler must classify every in-flight step. Validated work not touched by the directive stays validated; work whose premise the directive removed becomes `abandoned`; work whose assumptions changed becomes `invalidated` and requires revalidation before promotion.

### 66.5 Interaction with re-grounding

After a directive is applied, RegroundingService of §59 must run so the new constraint appears in every subsequent ContextPackage. A directive that is registered but absent from the next context package is a defect.

### 66.6 Architecture tests

The service is correct only when a directive issued mid-run alters subsequent behavior without restart; when a directive requesting a permission increase is rejected with a recorded reason; when the DirectiveEffect record accounts for every in-flight step; and when the constraint appears in the next assembled context.

## 67. Agent Runtime Debugger

**ContractId:** `CONTRACT.RUNTIME.DEBUGGER`  
**Authoritative build-spec section:** §63  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §63. Extends §17 (Lifecycle Hook Dispatcher) and §55 (Private Reasoning and Visible ReasoningStream Architecture), which remain the authority on the privacy boundary.

### 67.1 Components

| Component | Responsibility |
|---|---|
| StateSnapshotter | Captures inspectable runtime state at a point in time |
| DecisionBoundaryController | Pauses and resumes at kernel decision points |
| SurfaceTracer | Reconstructs all mutations and validations for one surface |
| DecisionTracer | Reconstructs why a step was selected, with cited evidence |
| EvidenceGapQuery | Lists requirements lacking required evidence kinds |

### 67.2 Snapshot schema

```text
RuntimeSnapshot
- snapshotId
- capturedAtEventId
- kernelState
- activePlanRevision
- activeConstraints
- lockedDecisions
- contextPackageManifest
- pendingToolCalls
- completedToolCalls
- heldReservations
- heldLeases
- evidenceLedgerSlice
- recoveryLadderPosition
- resourceReservations
```

The snapshot contains the context package manifest, not the assembled prompt text, and contains tool inputs and outputs, not model reasoning tokens.

### 67.3 Privacy enforcement

The debugger reads from the event ledger and the reasoning stream's structured events only. It must have no access path to private reasoning tokens, which are never persisted per §55. This makes the privacy boundary structural rather than policy-based.

### 67.4 Read-only guarantee

All debugger operations except pause and resume are read-only queries against the ledger. The debugger has no mutation broker handle, no permission to write project files, and no authority to alter evidence or completion state.

### 67.5 Reconstruction from ledger

Because the runtime is event-sourced through the reducer of §45, SurfaceTracer and DecisionTracer operate on completed sessions as well as live ones. Any historical run remains fully inspectable without special instrumentation at the time it ran.

### 67.6 Architecture tests

The debugger is correct only when a live run pauses at the next decision point rather than mid-mutation; when a mutation traces to a cited requirement and decision; when a completed session is inspectable from the ledger alone; and when no debugger operation produces a project mutation or evidence change.

## 68. External Trigger Gateway

**ContractId:** `CONTRACT.RUNTIME.TRIGGER`  
**Authoritative build-spec section:** §60  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §60. Extends §7 (Scheduler and Background Execution), which remains the authority on time-based initiation. This section adds externally originated admission.

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

The created task's permission ceiling is the minimum of the trigger ceiling and the project policy ceiling. A trigger can never widen permissions.

### 68.3 Default-disabled network surface

Triggers with source `external_webhook` are disabled at registration and require an explicit user enablement recorded in the decision trace. The gateway must not open a listening network surface while no webhook trigger is enabled.

### 68.4 Audit record schema

```text
TriggerFiring
- firingId
- triggerId
- firedAt
- source
- authenticationResult
- requestedGoal
- admissionDecision: admitted | rejected
- rejectionReason
- createdTaskId
- effectivePermissionCeiling
```

### 68.5 Isolation from authority

The gateway may create tasks. It may not approve decision nodes, promote artifacts, grant tool permissions, or mark requirements complete. All such operations remain with the deterministic authorities of §23 and §27.

### 68.6 Architecture tests

The gateway is correct only when a disabled webhook trigger opens no network surface; when an over-scoped request is rejected with a typed reason and audited; when an admitted task's ceiling equals the minimum of trigger and policy ceilings; and when every firing has an audit record.

## 69. Resource Profiler

**ContractId:** `CONTRACT.RUNTIME.PROFILING`  
**Authoritative build-spec section:** §64  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §64. Extends §3 (Process Model) and the resource governance of §51, which remain the authority on process supervision and reservation enforcement.

### 69.1 Components

| Component | Responsibility |
|---|---|
| OperationTimer | Measures duration, peak memory, CPU, and disk delta per operation |
| ProfileStore | Persists profiles keyed by operation class, project, and host |
| CostEstimator | Estimates plan cost from stored profiles |
| CapacityChecker | Compares estimates against available host capacity |
| DegradationDetector | Flags operations drifting from their profile |

### 69.2 Measurement boundaries

Measurement must wrap the supervised process, not the model's description of it. Each Gradle invocation, emulator boot, instrumentation run, packaging step, static analysis pass, and provider call is timed by the supervisor and written to ProfileStore with the host and project fingerprint.

### 69.3 Estimation contract

```text
PlanCostEstimate
- planRevision
- perOperationEstimates
- totalEstimatedDurationMs
- totalEstimatedPeakMemory
- estimatedDiskRequired
- confidence: profiled | sparse | unprofiled
- sampleCounts
- capacityVerdict: fits | exceeds_time | exceeds_memory | exceeds_disk
```

An operation class with fewer than the configured minimum samples must report `unprofiled` and must not receive a fabricated numeric estimate, satisfying the honesty invariant of build spec §66.1.

### 69.4 Planning integration

When `capacityVerdict` is not `fits`, the kernel must reduce scope, reorder work to lower peak concurrency, or surface the constraint as a decision node before execution. Beginning work that the estimate predicts will exhaust the host is prohibited.

### 69.5 Degradation signals

DegradationDetector compares recent samples to the stored p90. Sustained regression raises a host-health signal consumed by the recovery ladder of §28, since such drift commonly indicates disk pressure, a corrupted Gradle cache, or a degraded emulator image rather than an application defect.

### 69.6 Architecture tests

Profiling is correct only when repeated identical fixture runs converge to stable profiles; when an over-capacity plan is reduced or surfaced before execution; when an unprofiled operation is reported as unprofiled; and when injected disk pressure raises a host-health signal rather than an application defect.

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
| AppSecurityScanner | Scans generated application code and manifest for insecure patterns |
| SbomBuilder | Assembles the bill of materials for a produced artifact |
| ProvenanceRecorder | Binds artifact checksum to revision, toolchain, and SBOM |
| FindingDispositionStore | Records each finding as blocking or accepted with reason |

### 70.2 Dependency verification

```text
ResolvedDependency
- coordinate
- resolvedVersion
- integrityHash
- resolutionSource
- previouslyRecordedHash
- verdict: verified | hash_mismatch | unresolvable | substitution_suspected
```

A verdict other than `verified` blocks the build. A `hash_mismatch` against a previously recorded hash is treated as a supply-chain event, not a transient failure, and must be surfaced rather than auto-retried.

### 70.3 Application security checks

AppSecurityScanner must run before packaging and must check the categories enumerated in build spec §58.2, operating on the generated sources and merged manifest rather than on model claims about them. Each finding records the file, location, category, and severity.

### 70.4 SBOM and provenance schema

```text
ArtifactProvenance
- artifactId
- artifactKind: apk | aab
- artifactChecksum
- sourceRevision
- toolchainVersions
- signingIdentityClass
- dependencies: ResolvedDependency[]
- securityFindings
- dispositions
- builtAt
- reproducibilityInputs
```

ProvenanceRecorder must refuse to mark an artifact promotable when the SBOM is incomplete or any finding lacks a disposition.

### 70.5 Disposition discipline

Every finding must terminate in `blocking` or `accepted_with_reason`. The store must reject a disposition with an empty reason, which structurally prevents silent suppression. The final report renders all findings and dispositions.

### 70.6 Architecture tests

The runtime is correct only when a hardcoded secret blocks packaging; when an unpinned or hash-mismatched dependency blocks the build; when a name resembling a known package is flagged; when an artifact with an incomplete SBOM is not promotable; and when a finding cannot be dispositioned without a reason.


## 71. Agent Reasoning Runtime and Capability Layer

**ContractId:** `CONTRACT.RUNTIME.REASONING`  
**Authoritative build-spec section:** BS §66  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §66. Extends §58 (Agent Execution Kernel and Runtime Formalization) and §21 (Authority Hierarchy and Recovery Invariants), which remain the authority on the execution loop and on who decides. This section adds the reasoning components that drive the existing loop. It introduces no second loop and no second authority.

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
EvidenceStore (§23.3) -> ReflectionEngine -> next cycle
```

The reasoning engine sits above the kernel and below nothing. It cannot reach the capability layer except through the kernel, and the kernel cannot execute except through the authorities.

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

```text
ReasoningArtifact
- artifactId: uuid
- cycleId: uuid
- taskId: uuid
- producedAtEventId: int
- objective: text
- assumptions: text[]
- activeConstraints: constraintId[]
- lockedDecisions: decisionId[]
- hypotheses: hypothesisId[]
- alternativesConsidered: { strategy, rejectionReason }[]
- selectedStrategy: text
- selectionBasis: { kind: evidence | constraint | failure_signature | policy, ref }[]
- confidence: float
- uncertainties: text[]
- expectedEffect: text
- nextAction: { capabilityId, arguments }
- requiredCapabilities: capabilityId[]
- delegationPlan: grantId[]
- validationPlan: { method, targetSurface }[]
```

The store must reject an artifact with an empty `selectionBasis`, which structurally prevents unjustified strategy selection. No field of this record holds model reasoning text; `selectedStrategy` and `expectedEffect` are declarative statements, not transcripts.

### 71.4 Cycle state machine

```text
OBSERVE -> UNDERSTAND -> HYPOTHESIZE -> STRATEGIZE -> SELECT -> AUTHORIZE
AUTHORIZE  granted -> EXECUTE -> OBSERVE_RESULT -> REFLECT -> UPDATE -> DECIDE
AUTHORIZE  denied  -> STRATEGIZE   (denial recorded as an active constraint)
DECIDE  continue -> OBSERVE
DECIDE  repair   -> HYPOTHESIZE
DECIDE  replan   -> UNDERSTAND
DECIDE  delegate -> DELEGATE -> OBSERVE
DECIDE  branch   -> SPECULATE (§65) -> OBSERVE
DECIDE  terminate -> COMPLETED | BLOCKED | WAITING | RECOVERED | SAFELY_FAILED | ESCALATED
```

Transitions are recorded as kernel events. The engine cannot enter `EXECUTE` from any state except a granted `AUTHORIZE`, which makes the authority path structural rather than procedural.

### 71.5 HypothesisManager

```text
Hypothesis
- hypothesisId
- taskId
- statement
- predictedObservation
- discriminatingTest: { method, targetSurface }
- state: CREATED | TESTED | SUPPORTED | REJECTED | SUPERSEDED
- supportingEvidenceRefs
- refutingEvidenceRefs
- supersededBy
- resultingRepairKind
- createdAtEventId
```

The manager must refuse to mark a hypothesis `SUPPORTED` or `REJECTED` without an evidence reference, must refuse to retest a `REJECTED` hypothesis against unchanged evidence, and must expose whether an untested discriminating test remains so the kernel can prefer testing over untargeted repair. Rejected hypotheses are written as FAILURE memory records per §59.2 and feed the failure signatures of §63.4.

### 71.6 CapabilityRegistry and discovery

```text
CapabilityDescriptor
- capabilityId
- kind: skill | tool | worker | swarm | session | analysis | packaging
- inputSchema
- outputSchema
- requiredPermissions
- requiredEnvironment
- resourceProfileRef
- validationContract
- evidenceKinds
- failureStrategy
- rollbackStrategy
- availability: available | environment_missing | user_required | unavailable
```

Discovery is a query, not a grant. `discoverCapabilities(objective, constraints, environment)` returns descriptors whose availability is computed from the toolchain authority of §49 and the environment planner, with permissions still evaluated at invocation. A newly registered skill or tool becomes discoverable without modifying the reasoning engine, which is what makes the runtime extensible rather than hardcoded.

### 71.7 Invocation and delegation persistence

```text
CapabilityInvocation
- invocationId
- cycleId
- capabilityId
- arguments
- requestedPermissions
- authorityDecision: granted | denied | requires_approval
- denialReason
- resourceReservationRef
- resultRef
- evidenceRefs
- startedAt
- endedAt

DelegationGrant
- grantId
- parentAgentId
- childAgentId
- depth
- maxDepth
- capabilityCeiling: capabilityId[]
- resourceBudget
- timeBudget
- workspaceScope
- terminationPolicy
- issuedAtEventId
- revokedAtEventId
```

Artifacts, reflections, hypotheses, invocations, and grants are stored in the SQLite execution ledger, keyed by task and project, and are therefore replayable by the trajectory engine of §58 and inspectable by the debugger of §67 without special instrumentation.

### 71.8 DelegationManager enforcement

Before issuing a grant the manager computes:

```text
child.capabilityCeiling ⊆ parent.capabilityCeiling
child.resourceBudget    ≤ parent.resourceBudget − Σ(outstanding child budgets)
child.depth             = parent.depth + 1  ≤  maxDepth
child.workspaceScope    ⊆ parent.workspaceScope
```

Any violation denies the grant with a typed reason. The manager must recompute the outstanding-budget sum at issue time rather than trusting a cached value, since sibling grants change it. Revoking a parent grant must cascade to every descendant, reusing the cancellation propagation of §58.

### 71.9 Failure modes and recovery

| Failure | Runtime behavior |
|---|---|
| Artifact with empty selectionBasis | Rejected at write; cycle returns to STRATEGIZE |
| Authority denies the proposed action | Denial recorded as constraint; STRATEGIZE re-entered |
| Hypothesis rejected with no evidence | Write rejected; hypothesis remains TESTED |
| All hypotheses rejected | Cycle terminates SAFELY_FAILED or ESCALATED |
| Delegation ceiling violation | Grant denied; parent continues without the child |
| Child exhausts its budget | Child terminates; parent observes and replans |
| Swarm revision denied by policy | Graph unchanged; denial recorded |
| Mode selection exceeds policy | Mode downgraded to the highest permitted mode |
| Cycle exceeds iteration bound | Checkpoint, then ESCALATED rather than silent continuation |

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
      +-- deliberate            (bounded passes)
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
| DeliberationBudgetManager | Owns budget accounting and refuses passes beyond ceiling |
| ReasoningEffortSelector | Converts an agent request plus policy into a granted level |
| SufficiencyEvaluator | Evaluates the §68.7 conjunction, not stated confidence |
| HypothesisEvaluator | Runs competition, ranks by decisiveness, records refutation |
| StrategyComparator | Compares candidates under an identical evaluation basis |
| CounterexampleEngine | Adversarial critique; emits findings and evidence requests only |
| EvidenceAcquisitionPlanner | Selects the cheapest decisive read-only observation |
| DeliberationModelRouter | Escalates model within an unchanged permission ceiling |
| DeliberationContinuationManager | Persists session state across requests and compaction |
| DeliberationProgressEvaluator | Measures per-pass movement |
| DiminishingReturnDetector | Classifies NO_PROGRESS and forces an approach change |
| DeliberationRecordStore | Persists records; rejects inadmissible ones |

### 72.3 DeliberationRecord and session schema

```text
DeliberationRecord
- deliberationId: uuid
- cycleId: uuid
- taskId: uuid
- effortLevelRequested: NORMAL | EXTENDED | DEEP | EXHAUSTIVE
- effortLevelGranted: NORMAL | EXTENDED | DEEP | EXHAUSTIVE
- grantDecisionReason: text
- escalationTriggerEventId: eventId | null
- objective: text
- question: text
- passCount: int
- toollessPassCount: int
- hypothesesConsidered: hypothesisId[]
- hypothesesRejected: hypothesisId[]
- evidenceAcquired: evidenceRef[]
- evidenceDeltaByPass: { pass, evidenceRefs }[]
- alternativesConsidered: { strategy, rejectionReason }[]
- selectedStrategy: text
- rejectedStrategies: { strategy, refutingEvidenceRef }[]
- strategyRevisionRefs: evidenceRef[]
- refutationAttemptedByPass: { pass, attempted: true | false }[]
- uncertaintyBefore: float
- uncertaintyAfter: float
- confidenceBefore: float
- confidenceAfter: float
- continuationReasons: { pass, reason }[]
- reasonForTermination: text
- modelProfilesUsed: profileId[]
- providerRequestRefs: requestId[]
- reasoningUsage: {
    reasoningTokensReported,
    reasoningTokensEstimated,
    accountingStatus
  }
- resourceUsage: {
    reasoningTimeMs,
    modelRequests,
    wallClockMs
  }
- outcome: SUFFICIENT | BUDGET_EXHAUSTED | NO_PROGRESS | ESCALATED | ABANDONED

DeliberationSession
- sessionId
- deliberationId
- revision
- activeHypotheses: hypothesisId[]
- rejectedStrategies: { strategy, refutingEvidenceRef }[]
- evidenceAcquired: evidenceRef[]
- effortLevelGranted
- remainingBudget: DeliberationBudget
- providerContinuationState
- lastCheckpointEventId
```

DeliberationRecordStore must reject a record whose `passCount` exceeds one while `continuationReasons` has fewer entries than the additional passes, and must reject any record containing verbatim model reasoning in a text field. No field of either schema is a reasoning transcript.

`reasoningUsage.accountingStatus` distinguishes provider-`reported` usage, runtime-`estimated` usage, and `unavailable` usage. The runtime never fabricates provider-reported reasoning usage: when the provider does not expose reasoning-token accounting, the record states `estimated` or `unavailable`, and estimates remain telemetry that can never satisfy a sufficiency or certification requirement.

### 72.4 Pass loop

```text
enter deliberation (from HYPOTHESIZE or STRATEGIZE)
  -> ReasoningEffortSelector
       request + policy + capacity + provider capability
       -> granted effort level
  -> loop:
       DeliberationBudgetManager.reservePass()
         reservation unavailable -> terminate BUDGET_EXHAUSTED

       ContextAssembler.assembleDeliberationContext()
         -> preserve objective, active hypotheses, rejected strategies,
            constraints, evidence, remaining budget

       ModelGateway.request()
         -> normalized ReasoningSettings
         -> provider/model

       ModelGateway response
         -> structured proposal / reasoning summary / read-only observation request

       ToolBroker
         -> read-only evidence acquisition only

       DeliberationProgressEvaluator.measure()
         -> evidence delta
         -> uncertainty delta
         -> hypotheses eliminated
         -> strategy stability
         -> refutation attempted

       BudgetManager.settlePass()

       DiminishingReturnDetector.classify()

         no_progress
           -> GATHER_EVIDENCE
           | ESCALATE_MODEL
           | BRANCH
           | DELEGATE
           | ESCALATE
           | terminate NO_PROGRESS

       if toollessPassCount >= maxToollessPasses
           -> EvidenceAcquisitionPlanner MUST run before another pass

       SufficiencyEvaluator.evaluate()

         sufficient
           -> terminate SUFFICIENT

         insufficient
           -> continuation decision with a recorded continuationReasons entry
           -> next pass if budget permits

  -> emit DeliberationRecord
  -> return control to AgentReasoningEngine
```

The loop has no path from a pass directly to execution. Sufficiency returns to the reasoning engine, which emits the ReasoningArtifact and submits it for authorization.

### 72.5 ReasoningEffortSelector

The selector computes the granted level as the minimum of the requested level, the policy ceiling for the task's risk class, the level the remaining budget can fund, and the level the routed provider actually supports. The grant, the requested level, and the binding constraint are recorded, so a downgrade is visible rather than silent.

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

The evaluator implements the §68.7 conjunction. It consults the required-evidence set for the change's risk class, the uncertainty threshold for that class, strategy stability across the last pass, the presence of a validation plan, and whether HypothesisEvaluator reports an untested discriminating test.

A stated confidence value is an input to uncertainty only and can never satisfy the conjunction alone. For a change classified high-risk the evaluator must refuse sufficiency while architectural impact, dependency impact, affected-symbol analysis, regression plan, or validation plan is absent.

### 72.7 HypothesisEvaluator and CounterexampleEngine

HypothesisEvaluator enumerates candidates, obtains a discriminating test per candidate from EvidenceAcquisitionPlanner, ranks by decisiveness divided by cost, executes the most decisive affordable test, and records refutation against the hypothesis records of §71.5. At DEEP and above it must report whether the last pass attempted refutation or only confirmation; a confirmation-only pass does not count as competition.

CounterexampleEngine runs before authorization at DEEP and above for the change classes enumerated in §68.10. It holds no mutation broker handle, no evidence-approval capability, and no completion authority. Its output is a rejection finding or a list of evidence requests routed back through EvidenceAcquisitionPlanner.

### 72.8 EvidenceAcquisitionPlanner

The planner selects observations that are read-only by construction: file and symbol reads, impact-graph queries, index lookups, log reads, and non-mutating diagnostics. A candidate observation that would mutate project state, install a dependency, or write to a device is not acquirable during deliberation and must be proposed as an ordinary action through §71.7 authorization instead.

Cost estimates come from the ResourceProfiler of §69, so the planner prefers a cheap decisive observation over an expensive one and reports `unprofiled` rather than guessing.

### 72.9 Persistence and continuation

Deliberation records and sessions are stored in the SQLite execution ledger keyed by task and project, and are therefore replayable by the trajectory engine of §58 and inspectable by the debugger of §67.

DeliberationContinuationManager checkpoints session state on every pass boundary. The context assembler of §59.3 must treat active hypotheses, rejected strategies, and remaining budget as constraint-class content under §53.3, which makes them ineligible for eviction during compaction. A compaction that drops them is detectable by comparing session revision against the post-compaction context manifest, and is reported as a defect rather than tolerated.

### 72.10 Failure modes and recovery

| Failure | Runtime behavior |
|---|---|
| Agent requests an effort level above policy | Downgraded to highest permitted; grant reason recorded |
| Budget exhausted before sufficiency | Terminate BUDGET_EXHAUSTED; cycle yields WAITING or ESCALATED |
| Observation-free pass bound reached | Further passes refused until evidence is acquired |
| Diminishing returns detected | Approach change forced; a further plain pass is refused |
| All hypotheses refuted | Terminate NO_PROGRESS; escalate or branch |
| Critic finds a counterexample | Strategy rejected; return to STRATEGIZE with the finding as a constraint |
| Escalated model unavailable | Continue at the available model and record the capability gap |
| Compaction drops session state | Restore from the last pass checkpoint; report the compaction defect |
| Provider fails mid-session | Resume the deliberation from the last checkpoint. Revalidate the replacement provider's reasoning capability before issuing the next pass. Preserve the runtime effort requirement and remaining budget; if the replacement provider cannot satisfy the required effort level, either route to another approved provider/model or terminate with a typed capability gap. A provider failover must never silently reduce required effort. |
| Record fails admissibility | Rejected at write; deliberation cannot report sufficiency |

No failure mode permits presenting an unvalidated leading strategy as sufficient.

### 72.11 Architecture tests

The runtime is correct only when an agent request for EXHAUSTIVE under a policy ceiling of EXTENDED is granted EXTENDED with the constraint recorded; when a deliberation exceeding its pass budget terminates BUDGET_EXHAUSTED and the cycle does not execute the leading strategy; when consecutive observation-free passes are refused at the bound until evidence is acquired; when a high-risk change is refused sufficiency with a stated confidence of 0.95 and a missing regression plan; when a discriminating test refutes the leading hypothesis and the selected strategy changes as a result; when a counterexample finding returns the cycle to strategy selection without mutating the project; when an escalated model executes under the identical permission ceiling; when a forced context compaction preserves active hypotheses and rejected strategies and the session resumes without re-deriving them; when consecutive passes of flat uncertainty reaching the **configured** `diminishingReturnThreshold` produce NO_PROGRESS and an approach change rather than a further plain pass; when the ledger shows zero project mutation events between deliberation entry and the kernel `AUTHORIZE` grant; when an effort escalation carries a `grantDecisionReason` citing the observed condition that triggered it; and when no deliberation record in the ledger contains verbatim model reasoning.

The threshold is configuration, not a runtime constant. No component may hardcode a pass count for `NO_PROGRESS`: the classification is a function of the configured threshold, the measured per-pass movement, and consecutive-pass semantics. A test fixture supplies its own threshold value, and a runtime that behaves identically regardless of the configured value has not implemented the detector.

## References

[1]: https://learn.microsoft.com/en-us/windows/apps/winui/ "WinUI 3 Documentation"

[2]: https://sqlite.org/docs.html "SQLite Documentation"

[3]: https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects "Windows Job Objects"

[4]: https://git-scm.com/docs/git-worktree "Git Worktree Documentation"

[5]: https://playwright.dev/docs/intro "Playwright Documentation"


[7]: https://developers.openai.com/api/reference/overview "OpenAI API Reference Overview"

[8]: https://platform.openai.com/docs/api-reference/responses/create "Responses API Create Reference"

[9]: https://platform.openai.com/docs/api-reference/chat/create "Chat Completions Create Reference"


---



## 73. IntentSynthesisPromptContract and Truthful Preview Architecture

### 73.1 Prompt contract boundary

All coordinator, worker, skill, deliberation, and review prompts that can influence Android construction must implement the `IntentSynthesisPromptContract`. The prompt builder supplies the current contract version, project revision, checkpoint, selected evidence, assigned scope, allowed capabilities, and unresolved questions. It must not inject a user-facing template or framework choice.

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
    → AgentLoopReducer
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

`generatedOutputs` includes source representation and internal build artifacts; it is not synonymous with deployment delivery. A ZIP, Git bundle, or Android source project remains user-owned source/workspace access and cannot satisfy an APK delivery requirement. The resolver may select Kotlin, Java, Compose, Views, React Native/Expo, native modules, or a mixed architecture only as an implementation consequence of the user’s intent, environment capabilities, and validation evidence.

### 73.3 PreviewCoordinator and revision identity

`PreviewCoordinator` is the sole service allowed to create, reload, install, promote, invalidate, or roll back a live Android preview. It consumes a `PreviewRequest` only after the source transaction has committed a project revision or a declared preview-only diagnostic operation has been authorized.

```text
PreviewRequest
- schemaVersion
- requestId
- projectId
- taskId
- projectRevisionId
- checkpointId
- sourceFingerprint
- contractVersion
- technologyPlanVersion
- assetManifestVersion
- buildVariant
- deviceId
- androidApiLevel
- requestedMode
- selectedLanguage
- selectedUiFramework
- changedPaths
- requiredEvidenceKinds
- policyDecisionId
- workspaceRoot
- buildIdentity
```

The resulting `PreviewRevision` is immutable and contains:

```text
PreviewRevision
- previewRevisionId
- projectRevisionId
- checkpointId
- sourceFingerprint
- artifactId
- artifactFingerprint
- deviceId
- androidApiLevel
- buildVariant
- previewMode
- executionTruth
- buildStatus
- installStatus
- launchStatus
- runtimeStatus
- validationStatus
- evidenceIds
- createdAt
- observedAt
- invalidatedAt
- invalidatedReason
```

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

### 73.5 Truth labels and evidence classes

All preview, execution, and validation projections carry one of `PREDICTED`, `SIMULATED`, `REQUESTED`, `OBSERVED`, `VERIFIED`, `STALE`, or `INVALIDATED`. The UI may show predicted or simulated information as a forecast, but it must label it clearly and must never render it as a running application or passed validation.

Evidence is classified separately:

| Evidence class | Produced by | Completion use |
|---|---|---|
| `PLAN_EVIDENCE` | Contract/planning services | Explains intended work; cannot prove execution |
| `PROCESS_EVIDENCE` | Process supervisor | Proves command/process observation |
| `DEVICE_EVIDENCE` | Emulator/device manager | Proves install, launch, interaction, or emulator state |
| `VISUAL_EVIDENCE` | Screenshot and comparison service | Proves a declared visual check |
| `TEST_EVIDENCE` | Test runner and oracle | Proves declared assertions |
| `ARTIFACT_EVIDENCE` | APK inspector | Proves artifact presence, hash, and contents |
| `PROMOTION_EVIDENCE` | EvidenceAuthority | Proves all required gates passed |

### 73.5.1 Canonical `PreviewPromotionGate`

All preview promotion decisions must evaluate one canonical gate. Individual workers, the UI, model output, and presentation reducers may report evidence, but none may promote a candidate independently.

A candidate `PreviewRevision` may become `OBSERVED` only when the exact candidate source revision, generated asset and branding fingerprint, selected toolchain lock, checkpoint, artifact fingerprint, device or emulator identity, and active emulator session are recorded, and the artifact has been installed and launched with supervised observation. Required interaction, screenshot, accessibility, visual, Logcat, crash, and runtime evidence must be current for the declared Android profile.

A candidate may become `VERIFIED` and replace the active last-known-good preview only when `PreviewPromotionGate` confirms all required evidence for the profile: source and asset identity match the checkpoint; the build and artifact hash are valid; installation and launch succeeded on the identified emulator session; required synthetic interactions and declared tests passed; required visual/accessibility and diagnostic checks passed; no invalidation, stale identity, crash, or policy condition is present; and the evidence set is durably recorded by the EvidenceAuthority. Missing, stale, mismatched, simulated, or model-authored evidence fails the gate.

The gate must return a typed result such as `PASS`, `MISSING_EVIDENCE`, `STALE_IDENTITY`, `FAILED_VALIDATION`, `POLICY_BLOCKED`, or `ENVIRONMENT_UNAVAILABLE`. A failed or incomplete candidate remains `FAILED_CANDIDATE`, `RECOVERING`, `STALE`, or `INVALIDATED`; it cannot replace last-known-good. The gate is the sole normative promotion predicate and must be used by the control plane, artifact authority, preview reducer, and release completion checks.

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

The §73.2 `AndroidTechnologyResolver` selects Kotlin, Java, Compose, Views, React Native/Expo, native modules, or a mixed architecture only as an implementation consequence of the user's intent, environment capabilities, and validation evidence. Every resulting `AndroidTechnologyPlan` MUST resolve to exactly one registered `AndroidTechnologyAdapter` implementation.

`AndroidTechnologyAdapter` is a strategy/composition resolver and does not execute concrete preview, build, artifact, device, runtime, observation, validation, or failure-classification operations. `AndroidBuildAdapter` and `AndroidDeviceAdapter` are the sole concrete execution surfaces. Each concrete preview operation has exactly one execution authority: `AndroidBuildAdapter` for build and artifact operations, or `AndroidDeviceAdapter` for device and runtime operations. The technology adapter resolves those authorities but never executes their concrete operations. The technology adapter is not a second execution surface and is not a second authority.

```text
AndroidTechnologyAdapter
- adapterId
- adapterVersion
- technologyIds
- supportedCompositions
- requiredToolchainCapabilities
- requiredDeviceCapabilities
- compatibilityRules

AndroidTechnologyAdapterResolution
- resolutionId
- adapterId
- adapterVersion
- technologyPlanHash
- toolchainLockId
- buildAdapterIdentity
- deviceAdapterIdentity
- compatibilityDecision: COMPATIBLE | COMPATIBLE_WITH_REPAIR | INCOMPATIBLE
- decisionReason
- resolvedAt
```

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

The three internal implementation families registered at M108 are execution strategies, not user-facing framework choices. The §73.2 no-template rule remains binding; the resolver never surfaces these family names to the user as a framework picker.

```text
NativeAndroidAdapter (internal implementation family)
- composition: Kotlin or Java, Views or Compose, Gradle
- adapterId prefix: nirman.adapter.native
- operations: validatePlan, initializeProject, planBuild, classifyFailure,
  resolveBuildAdapter (Gradle native), resolveDeviceAdapter
  (AndroidDeviceAdapter for Nirman-managed local emulator session)

JavaScriptAndroidAdapter (internal implementation family)
- composition: React Native or Expo, Metro or Expo runtime, native Gradle shell
- adapterId prefix: nirman.adapter.javascript
- operations: validatePlan, initializeProject, planBuild, classifyFailure,
  resolveBuildAdapter (Gradle plus Metro or Expo), resolveDeviceAdapter
  (AndroidDeviceAdapter for Nirman-managed local emulator session)

MixedAndroidAdapter (internal implementation family)
- composition: native plus JavaScript plus native modules, NDK or CMake
  when selected, device APIs
- adapterId prefix: nirman.adapter.mixed
- operations: validatePlan, initializeProject, planBuild, classifyFailure,
  resolveBuildAdapter (composed Gradle plus Metro or Expo plus NDK or
  CMake), resolveDeviceAdapter (AndroidDeviceAdapter for emulator or
  Nirman-managed local Android emulator)
```

The `AndroidTechnologyAdapter` registry is part of the toolchain lock surface. A revision, toolchain update, environment fingerprint change, or compatibility-rule change invalidates dependent resolution results and completion claims; the adapter registry entry, `adapterVersion`, and `technologyPlanHash` together identify a reproducible selection context. The `PreviewSyncEvent` payload defined in build spec §71.1 carries `adapterId`, `adapterVersion`, `technologyPlanHash`, and the resolved `buildAdapterIdentity` and `deviceAdapterIdentity` as required event fields when the event is emitted by an adapter-mediated operation; the §71 `PreviewSyncEvidenceRecord` carries the same fields per observation. This extends §71.1; it does not redefine the `PreviewSyncEvent` schema.

### 73.11 Deterministic preview-mode resolver

The preview mode is selected by a deterministic resolver over a recorded input set. The resolver is a pure function of the recorded input and the canonical rule table; it is not a model decision.

```text
PreviewModeResolverInput
- technologyPlanHash
- changedPaths
- impactGraphRevision
- sourceRevisionId
- buildIdentity
- artifactIdentity
- deviceSessionId
- runtimeSessionId
- environmentFingerprint
- toolchainLockId
- nativeIdentityFingerprint
- runtimeHealthObservationRef

PreviewModeResolverOutput
- previewMode: RN_EXPO_FAST_REFRESH | COMPOSE_RELOAD |
                INCREMENTAL_APK_INSTALL | FULL_APK_REINSTALL |
                CONSERVATIVE_FULL_REINSTALL |
                HEADLESS_SMOKE | DIAGNOSTIC_SOURCE_ONLY |
                USER_REQUIRED | BLOCKED
- decisionReason
- requiredOperations
- invalidationSet
```

The mode values `RN_EXPO_FAST_REFRESH`, `COMPOSE_RELOAD`, `INCREMENTAL_APK_INSTALL`, `FULL_APK_REINSTALL`, `HEADLESS_SMOKE`, `DIAGNOSTIC_SOURCE_ONLY`, `USER_REQUIRED`, and `BLOCKED` are the existing `PreviewRevision.previewMode` enumeration introduced in §73.3. `CONSERVATIVE_FULL_REINSTALL` is added to that enumeration as a refinement of `FULL_APK_REINSTALL`: it is a full reinstall selected specifically because the impact information was insufficient to prove a faster safe path, not because a faster safe path was proven unsafe. Its presence makes the resolver's "unknown" outcome distinguishable from a "known unsafe" outcome and is recorded as part of the `PreviewRequest` decision trace.

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

healthyMetroExpoRuntime:
  requires a current runtime or device observation tied to the same
  deviceSessionId and runtimeSessionId, with:
  - environmentFingerprint equal to the recorded toolchain environment
  - applicationStateFingerprint equal to the last accepted PreviewRevision
  - sourceFingerprint compatible with the recorded source revision
  - native identity fingerprint matching sameNativeIdentity
  - no recorded fault, crash, or transport-loss observation in the current
    runtime session
  An observation older than the current PreviewRevision freshness window or
  bound to a different session does not satisfy healthyMetroExpoRuntime.
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
6. JavaScript or TypeScript-only change with sameNativeIdentity and
   healthyMetroExpoRuntime
   -> RN_EXPO_FAST_REFRESH
7a. Known unsafe-to-fast-refresh state detected (signing mismatch,
    ABI mismatch, manifest version conflict, runtime fault unacknowledged,
    or compatibility-rule denial) but a clean rebuild is permitted
    -> FULL_APK_REINSTALL
7b. Insufficient impact information to prove a reload-safe or
    fast-refresh-safe surface; no rule above fired
    -> CONSERVATIVE_FULL_REINSTALL
8. Recognized incompatibility with no permitted rebuild path
    -> BLOCKED
```

The "unknown" outcome and the "known unsafe" outcome are explicitly distinct: rule 7a is recorded with reason `KNOWN_UNSAFE_TO_FAST_REFRESH`; rule 7b is recorded with reason `INSUFFICIENT_IMPACT_INFORMATION`. The resolver MUST distinguish them in the `decisionReason` field so that the `PreviewRequest` decision trace and downstream repair logic do not conflate them.

A resolver output is recorded as part of the `PreviewRequest` decision trace. The mode returned is one of the `PreviewRevision.previewMode` values enumerated in §73.3 (now including `CONSERVATIVE_FULL_REINSTALL`); introducing new mode identifiers requires a versioned contract update through ADR-195. The resolver MUST NOT mutate authoritative state; it returns a decision, and `PreviewCoordinator` owns the resulting lifecycle transition.

### 73.12 Android device adapter contract

The device layer used by `PreviewCoordinator` for install, launch, interaction, screenshot, UI hierarchy, Logcat, crash, and permission observation is bound to a canonical `AndroidDeviceAdapter` interface. Emulator and Nirman-managed local Android emulator implementations MUST satisfy this interface; the interface is an execution contract, not an authority.

```text
AndroidDeviceAdapter
- adapterId
- adapterVersion
- supportedAbiFamilies
- supportedAndroidApiLevels
- supportedDeviceKinds: [EMULATOR]

AndroidDeviceAdapter operations
- enumerate() -> DeviceEnumerationResult
  - params: none
  - returns: list of DeviceDescriptor (Nirman-managed local Android emulator)
  - errors: DeviceEnumerationError
- acquire(deviceDescriptor: DeviceDescriptor) -> DeviceAcquisitionResult
  - params: deviceDescriptor: DeviceDescriptor
  - returns: deviceSessionId, runtimeSessionId, environmentFingerprint
  - errors: DeviceAcquisitionError, DeviceUnavailableError
- prepare() -> DevicePreparationResult
  - params: none
  - returns: prepared: bool, deviceStateFingerprint, toolchainLockId
  - errors: DevicePreparationError
- boot() -> DeviceBootResult
  - params: none
  - returns: booted: bool, bootTimestamp, apiLevel, abiFamily
  - errors: DeviceBootError, BootTimeoutError
- waitReady(timeoutMs: int = 30000) -> DeviceReadyResult
  - params: timeoutMs: int (default 30000)
  - returns: ready: bool, readyTimestamp, healthObservation
  - errors: DeviceBootError, BootTimeoutError
- install(apkPath: str) -> DeviceInstallResult
  - params: apkPath: str (absolute path to APK)
  - returns: installed: bool, installTimestamp, packageId
  - errors: DeviceInstallError, InstallTimeoutError
- uninstall(packageId: str) -> DeviceUninstallResult
  - params: packageId: str
  - returns: uninstalled: bool, uninstallTimestamp
  - errors: DeviceUninstallError
- launch(packageId: str, activity: str) -> DeviceLaunchResult
  - params: packageId: str, activity: str
  - returns: launched: bool, launchTimestamp, processId
  - errors: DeviceLaunchError, LaunchTimeoutError
- forceStop(packageId: str) -> DeviceForceStopResult
  - params: packageId: str
  - returns: stopped: bool, stopTimestamp
  - errors: DeviceForceStopError
- reload() -> DeviceReloadResult
  - params: none
  - returns: reloaded: bool, reloadTimestamp
  - errors: DeviceReloadError
- interact(input: InteractionInput) -> DeviceInteractionResult
  - params: input: InteractionInput (tap, swipe, text, key)
  - returns: interactionId, result: bool, screenshotRef, uiHierarchyRef
  - errors: DeviceInteractionError, InteractionTimeoutError
- captureScreenshot() -> ScreenshotResult
  - params: none
  - returns: screenshotId, screenshotRef, capturedAt, deviceStateFingerprint
  - errors: ScreenshotCaptureError
- captureUiHierarchy() -> UiHierarchyResult
  - params: none
  - returns: uiHierarchyId, uiHierarchyRef, capturedAt
  - errors: UiHierarchyCaptureError
- collectLogcat(filter: str = "", since: str = "") -> LogcatResult
  - params: filter: str (default ""), since: str (default "")
  - returns: logcatId, logcatRef, lineCount, capturedAt
  - errors: LogcatCollectionError
- collectCrash() -> CrashResult
  - params: none
  - returns: crashId, crashRef, crashType, stackTrace, capturedAt
  - errors: CrashCollectionError
- collectPermissionState() -> PermissionStateResult
  - params: none
  - returns: permissionStateId, permissions: list, capturedAt
  - errors: PermissionStateCollectionError
- reset() -> DeviceResetResult
  - params: none
  - returns: reset: bool, resetTimestamp
  - errors: DeviceResetError
- snapshot() -> DeviceSnapshotResult
  - params: none
  - returns: snapshotId, snapshotRef, deviceStateFingerprint
  - errors: DeviceSnapshotError
- restore(snapshotId: str) -> DeviceRestoreResult
  - params: snapshotId: str
  - returns: restored: bool, restoreTimestamp
  - errors: DeviceRestoreError
- release() -> DeviceReleaseResult
  - params: none
  - returns: released: bool, releaseTimestamp
  - errors: DeviceReleaseError
```

Every operation returns a typed observation that carries `adapterId`, `adapterVersion`, `deviceId`, `deviceSessionId`, `runtimeSessionId`, `environmentFingerprint`, `applicationStateFingerprint`, `evidenceReferences`, `failureClassification`, and `invalidationDependencies`. Operations do not write `PreviewProjection`, evidence identity, artifact promotion, or completion state; those remain with the existing specialized authorities. A revision, toolchain update, environment fingerprint change, emulator identity change, or capability revocation invalidates dependent observations and completion claims unless the dependency graph proves independence.

### 73.13 Android build adapter contract

Build execution is bound to a canonical `AndroidBuildAdapter` interface. The interface is an execution contract, not an authority; it does not authorize builds, and it does not promote artifacts.

```text
AndroidBuildAdapter
- adapterId
- adapterVersion
- technologyPlanHash
- toolchainLockId
- buildVariant
- workingDirectory
- environmentFingerprint
- commandPlan
- artifactRules

AndroidBuildObservation
- buildId
- sourceRevisionId
- toolchainLockId
- adapterId
- adapterVersion
- environmentFingerprint
- exitCode
- artifactIds
- artifactFingerprints
- diagnostics
- logs
- reproducibilityStatus
- capturedAt

AndroidBuildAdapter operations
- build() -> AndroidBuildObservation
  - params: none (uses locked adapter state: technologyPlanHash, toolchainLockId, buildVariant)
  - returns: buildId, exitCode, artifactIds, artifactFingerprints, diagnostics, logs, reproducibilityStatus
  - errors: BuildError, ToolchainError, BuildTimeoutError
- inspectArtifact(artifactId: str) -> ArtifactInspectionResult
  - params: artifactId: str
  - returns: artifactId, fingerprint, sizeBytes, signingState, manifestSummary
  - errors: ArtifactInspectionError, ArtifactNotFoundError
- sign(packageId: str, signingConfig: SigningConfig) -> SigningResult
  - params: packageId: str, signingConfig: SigningConfig
  - returns: signingId, certificateFingerprint, signingScheme, artifactFingerprint
  - errors: SigningError, SigningPolicyViolationError
- export(artifactId: str, destination: ExportDestination) -> ExportResult
  - params: artifactId: str, destination: ExportDestination
  - returns: exportId, destinationPath, byteCount, contentHash, reconciliationReference
  - errors: ExportError, ExportTimeoutError, DestinationUnavailableError
```

The same interface MUST cover: Gradle native; Gradle plus Metro or Expo; React Native; NDK or CMake; and mixed native plus JavaScript. `AndroidBuildAdapter` is invoked by `PreviewCoordinator` through the `AndroidTechnologyAdapter` selected for the `AndroidTechnologyPlan`; it does not create a separate build authority, and it does not bypass `ToolchainAuthority` or `ArtifactAuthority`. A revision, toolchain update, environment fingerprint change, or adapter version change invalidates dependent observations and completion claims.

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

The following paths are forbidden and MUST be rejected by the typed command registry and the contract-graph verifier:

```text
UI → ADB
UI → Gradle
UI → Metro or Expo
UI → emulator
UI → AndroidTechnologyAdapter.executeBuild | install | launch | reload |
    observeRuntime | captureScreenshot | captureUiHierarchy |
    collectLogcat | runValidation
```

The §73.8 rule that the preview panel is a read model of durable control-plane events is preserved; the technology adapter and the build and device adapters do not change the panel authority, they only supply observations through the existing `PreviewSyncEvent` and `PreviewSyncEvidenceRecord` flow.

## 74. Integration Boundary Implementation Contract

**Implements:** build spec §70 and `CONTRACT.RUNTIME.INTEGRATION_BOUNDARY`
**Canonical schema owner:** `CanonicalSchemaRegistry` in §36.1
**Implementation owner:** the Rust control plane and supervised boundary services

The runtime implements the common boundary envelope as a correlation projection. It does not replace the authoritative specialized contracts. `WorkflowCoordinator` creates or updates the boundary reference, the relevant deterministic authority admits the operation, and the specialized service owns its state transition.

```text
IntegrationBoundaryRuntime
- boundaryId
- integrationBoundaryVersion
- sourceEntityRef
- destinationEntityRef
- payloadSchemaRef
- responseSchemaRef
- protocolVersion
- adapterOrBridgeRef
- authorityRefs
- operationRef
- specializedStateRef
- transactionRef
- correlationId
- causationId
- idempotencyKey
- compatibilityRef
- timeoutPolicyRef
- cancellationPolicyRef
- retryPolicyRef
- observationRefs
- evidenceRefs
- validationRef
- downstreamEffectRefs
- invalidationRefs
- failureRecoveryRef
- applicability
```

`BoundaryOperationProjection` is not a second lifecycle authority:

```text
BoundaryOperationProjection
- operationRef
- boundaryId
- state: PLANNED | AUTHORIZED | DISPATCHED | RUNNING | WAITING |
          OBSERVED | VALIDATED | APPLIED | RETRYABLE_FAILURE |
          CANCEL_REQUESTED | CANCELLED | BLOCKED | SAFELY_FAILED
- specializedStateRef
- timeoutPolicyRef
- cancellationPolicyRef
- retryAttempt
- idempotencyKey
- transactionRef
- observationRefs
- evidenceRefs
- validationRef
- downstreamEffectRefs
- invalidationRefs
```

The projection is valid only when `specializedStateRef` resolves to the state machine owned by the applicable service. Lease loss fences the operation by revoking capabilities and rejecting new writes. A timeout or cancellation produces a durable lifecycle event. A retry after an unknown device or external outcome requires the relevant transaction reconciliation, idempotency read-back, or compensation evidence before a new effect is authorized. A stale source revision, contract version, adapter version, toolchain, emulator state, application state, environment state, artifact, credential, or policy invalidates dependent observations and downstream effects.

### 74.1 Android service integration

```text
AndroidServiceIntegration
- integrationId
- appBoundaryRef
- endpointIdentity
- requestSchemaRef
- responseSchemaRef
- protocolVersion
- adapterRef
- authenticationProfileRef
- credentialReference
- datastoreOwner: local_android | external_service |
                  user_managed_supporting_service
- persistenceSchemaRef
- offlineAndCachePolicy
- idempotencyPolicy
- requiredOperationality
- functionalScenarioRefs
- acceptanceEvidenceRefs
- privacyAndNetworkPolicy
```

An Android service integration is a supporting dependency of the generated Android application. It does not create a second generated target. Its functional state is promoted only from the declared integration scenario and evidence, not from local compilation, application launch, or endpoint reachability alone.

### 74.2 UI hierarchy observation

```text
UiHierarchyObservation
- observationId
- taskId
- previewRevisionId
- deviceSessionId
- projectRevisionId
- applicationStateFingerprint
- hierarchyFormat
- hierarchyReference
- redactionPolicyId
- capturedAt
- truth: REQUESTED | OBSERVED | VERIFIED | STALE | INVALIDATED
- evidenceId
```

UI-hierarchy evidence may support accessibility, navigation, state, and visual checks. It cannot replace supervised Nirman-managed local Android emulator execution and cannot satisfy validation while requested, predicted, simulated, stale, or invalidated.

### 74.3 Signing and export verification

```text
ExportVerificationRecord
- exportId
- artifactId
- sourcePathReference
- destinationPathReference
- sourceArtifactHash
- destinationHash
- byteCount
- destinationFileIdentity
- exportOperationState: REQUESTED | COPYING | COPIED | UNKNOWN |
                        RECONCILING | VERIFIED | FAILED | BLOCKED
- postCopyCheck
- policyDecisionId
- packagingProfileId
- artifactKind: APK | AAB | SOURCE
- sourceRevision
- checkpointId
- sourceFileIdentity
- requestFingerprint
- idempotencyKey
- signingIdentityBindingId
- validationDecisionId
- promotionDecisionId
- reconciliationReference
- failureEvidenceId
- deploymentDelivery: REQUIRED_APK | DECLARED_AAB_OPTIONAL | SOURCE_ACCESS_ONLY
- destinationKind: LOCAL_WINDOWS_FILESYSTEM | USER_APPROVED_SOURCE_LOCATION
- evidenceId
- verifiedAt
```

`ExportVerificationRecord` is the canonical implementation record for both source access and deployment delivery. For local deployment it is materialized as an `APKExportRecord` view with `artifactKind: APK`, `deploymentDelivery: REQUIRED_APK`, `destinationKind: LOCAL_WINDOWS_FILESYSTEM`, the verified signing identity, validation decision, promotion decision, source and destination identities, source and destination hashes, byte count, and post-copy evidence. A declared AAB uses the same record only when the immutable `PackagingProfile` is `APK_AND_AAB`; it never makes AAB mandatory. Source, ZIP, and Git exports use `SOURCE_ACCESS_ONLY` and cannot satisfy artifact delivery or completion.

Local export is complete only when the authorized destination exists, its byte count and content hash match the source artifact, the path is within approved export scope, and post-copy verification evidence is durable. Export does not by itself prove signing, preview currency, integration functionality, documentation certification, or user-goal completion.

### 74.4 Deployment export admission and reconciliation
The artifact export handler accepts a deployment request only after resolving the declared `PackagingProfile`, artifact kind, signing identity binding, validation decision, promotion decision, and destination policy. The only deployment destination is the approved local Windows filesystem. It creates one durable `ExportVerificationRecord` before copying, transitions through copy and post-copy verification states, and records source/destination identity and hash equality. A failed, interrupted, or unknown copy remains durable and is reconciled before retry. The handler may separately serve source/workspace, ZIP, or Git access, but that branch is marked `SOURCE_ACCESS_ONLY` and cannot emit deployment evidence or advance completion.

### 74.5 Documentation certification report

```text
DocumentationCertificationReport
- reportId
- documentSnapshotHash
- verifierVersion
- registryVersion
- checksExecuted
- graphClassesChecked
- semanticRulesChecked
- defectCount
- defects
- result: PASSED | FAILED
- evidenceId
- generatedAt
```

The report certifies documentation identity, registry resolution, graph structure, and declared semantic documentation rules only. It never certifies runtime source, Windows isolation, provider behavior, Android execution, preview truth, recovery, signing, or APK validity.

## 75. Preview Synchronization Implementation Contract

**Implements:** build spec §71 and `CONTRACT.RUNTIME.PREVIEW_SYNC`
**Canonical schema owner:** `CanonicalSchemaRegistry` in §36.1
**Implementation owner:** `WorkflowCoordinator`, `PreviewCoordinator`, the durable event store, and the UI projection runtime

### 75.1 Event and reducer implementation

The architecture implements the exact `PreviewSyncEvent`, `PreviewProjection`, `PreviewProjectionReducer`, and `PreviewSyncEvidenceRecord` schemas defined by build spec §71.1. The event store assigns the durable per-project/task sequence. `WorkflowCoordinator` normalizes intent, agent, worker, build, device, evidence, recovery, and promotion outcomes into events. Every non-root event carries causal parentage, runtime-session identity where applicable, and an authority class. `PreviewCoordinator` is the only service that can emit an accepted promotion event. The UI consumes snapshots and events but never writes projection state.

`PreviewProjectionReducer` is a pure deterministic reducer over a snapshot and an ordered event range. It must be replayable without side effects, must record the reducer version and projection revision, and must produce the same state for the same snapshot and event range. The reducer delegates specialized decisions to the existing lifecycle, evidence, device, artifact, recovery, and promotion authorities; it does not grant permissions or approve evidence.

### 75.2 Event ownership table

| Event family | Canonical producer | Required prerequisite | Authority class | Reducer update |
|---|---|---|---|---|
| Intent and contract | intent/contract services | accepted user request and schema validation | DECLARATIVE / PLANNED | intent and contract stage |
| Plan and checkpoint | planner and transaction authority | authorized plan and durable checkpoint | PLANNED | plan/checkpoint refs |
| Source and build | commit barrier and process supervisor | source revision and operation capability | EXECUTION_OBSERVED | source/build/artifact fields |
| Install and runtime | device manager and supervised process | device transaction and observed result | RUNTIME_OBSERVED | install/launch/runtime fields |
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

`PreviewSyncEvidenceRecord` is persisted with the event sequence range, reducer version, projection revision, preview revision, source revision, checkpoint, branch identity, artifact fingerprint, emulator identity, runtime-session identity, state fingerprints, event IDs, observation references, evidence references, validation references, invalidated evidence, recovery events, promotion record, certification decision, and completion decision. Runtime certification must execute the complete chat instruction → agent proposal → authorized mutation → source revision → build → APK → install → device runtime → observation → validation → promotion → event replay → panel projection path.

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

A retry budget is policy-configurable and bounded. Repeating the same command, patch, prompt, or provider route does not count as a new attempt. When safe strategies are exhausted, the runtime backtracks, degrades, pauses for a required decision, or reports a truthful blocker.

### 76.3 Specialist worker responsibilities

Specialist workers are independent roles selected by the orchestrator; they do not become additional authorities.

| Specialist role | Required responsibility | Blocking evidence or gate |
|---|---|---|
| Orchestrator | Maintain one shared goal, acceptance contract, task graph, dependency order, and handoff record | Authorized plan and task-graph transition |
| Security worker | Detect secrets, unsafe configuration, dependency vulnerabilities, license violations, provenance gaps, and client-bundle exposure | Security and dependency evidence before commit or artifact promotion |
| Consistency worker | Compare schemas, types, UI/control-plane messages, Android service contracts, and persisted records for drift | Schema compatibility and contract-parity result |
| Diff-aware patch worker | Apply scoped patches against the current revision, preserve unrelated user edits, and emit a reviewable diff | Workspace revision, reservation, and reconciliation checks |
| Diagnostics worker | Classify failures, correlate stack traces and runtime observations, and produce `FailureContextPackage` | Failure fingerprint and evidence references |
| Validation worker | Run focused and regression checks, Android build/emulator validation, and visual/accessibility checks | Independent validation and current evidence |
| Memory/index worker | Update the project index, settled decisions, conventions, failure patterns, and sanitized episode summaries | Privacy classification and memory-write policy |
| Release worker | Prepare artifact, signing, certificate, promotion, and local export records without bypassing authorities | `PreviewPromotionGate`, signing authority, and export verification |

The orchestrator reconciles specialist handoffs against one shared contract and the current project revision. A worker report cannot mark a task complete, promote a preview, approve a dependency, or authorize an external effect. A specialist may recommend a result only through its typed operation and evidence contract.

### 76.4 Acceptance requirements

The implementation must prove that file-save continuation, build-completion continuation, failure-to-diagnostics feedback, dependency scanning, local promotion/export health checks, rollback, specialist handoffs, and sanitized memory updates are durable and replayable. It must also prove that a disconnected UI does not stop authorized background work, a failed health check preserves last-known-good state, a stale worker cannot apply a patch, a security or dependency gate can block a commit, and a model statement cannot substitute for evidence.

Nirman remains a Windows-first local host for Android generation. The isolation boundary is the approved Windows workspace and supervised process environment; no container, virtual machine, WSL, or generic web/cloud deployment runtime is implied by this contract.

## 77. Cost Governance Implementation Contract

### 77.1 Canonical schema

`CostGovernanceRecord` is persisted with the task and operation ledger. `CostAuthority` evaluates reservations before admission and settlements after completion. It receives provider usage, token estimates, process telemetry, emulator cost estimates, and configured caps through typed records.

### 77.2 Lifecycle and authority

The lifecycle is `UNSET → DECLARED → RESERVED → RUNNING → SETTLED`, with `RECONCILIATION_REQUIRED`, `DEGRADED`, `PAUSED_FOR_APPROVAL`, and `SAFE_FAILED` side states. Cost authority may deny, downgrade, pause, or request approval, but cannot grant an operation capability or promote evidence.

### 77.3 Failure and recovery

Unknown provider usage, missing settlement, telemetry loss, cap exhaustion, and disagreement between estimated and reported usage produce durable diagnostics. Recovery may reduce context, concurrency, or model profile, or pause for policy; it must never retry an unknown external charge blindly.

## 78. Agent Trust Boundary Implementation Contract

### 78.1 Canonical schema

`AgentTrustAssessment` is produced before a skill, MCP-compatible tool, plugin, or instruction-bearing package is admitted. Scanners run in a restricted local process and record content hashes, provenance, requested capabilities, static findings, behavioral findings, destinations, and the policy decision.

### 78.2 Lifecycle and authority

The lifecycle is `DISCOVERED → HASHED → SCANNED → POLICY_REVIEW → ADMITTED | QUARANTINED | REVOKED | EXPIRED`. Trust assessment is necessary but not sufficient for execution; capability, permission, credential, workspace, and external-effect authorities remain in force.

### 78.3 Failure and recovery

Hash drift, revoked content, scanner failure, malformed manifests, hidden instructions, or undeclared access requests cause quarantine or re-assessment. A quarantined package cannot execute through a cached admission record.

## 79. Context and Cache Governance Implementation Contract

### 79.1 Canonical schema

`ContextCachePolicy` is resolved for each provider request and context package. `ContextGovernance` records selected content, protected content, compaction trigger, cache key inputs, invalidation causes, redactions, telemetry disclosures, and resulting context lineage.

### 79.2 Lifecycle and authority

The lifecycle is `DECLARED → SELECTED → COMPACTED_OR_FULL → CACHED_OR_UNCACHED → TRANSMITTED → INVALIDATED`. Context governance cannot delete mandatory constraints, change user intent, or convert summarized content into a fresh observation.

### 79.3 Failure and recovery

Context overflow, failed compaction, cache mismatch, cache corruption, privacy-policy change, or provider continuation loss causes context rebuild or safe reduction. The runtime must preserve required constraints and evidence references while recording what was excluded or summarized.

## 80. Android Runtime Integrity Implementation Contract

### 80.1 Canonical schema

`AndroidRuntimeIntegrityObservation` is emitted by supervised device and runtime collectors. It binds each signal to project revision, artifact, package, device, runtime session, source, applicability, timestamp, and evidence.

### 80.2 Lifecycle and authority

The lifecycle is `REQUESTED → COLLECTING → OBSERVED → VALIDATED | NOT_APPLICABLE | UNAVAILABLE | USER_REQUIRED | INVALIDATED`. Runtime collectors observe; `ValidationAuthority` interprets the signal against the declared acceptance policy. No single signal can replace required build, install, UI, behavior, or evidence checks.

### 80.3 Failure and recovery

ANR, emulator session loss, unavailable Play Integrity, battery or Doze uncertainty, permission denial, stale runtime sessions, and collector errors produce typed evidence gaps. Recovery may restart collection, reconnect the device, change the declared profile, or report an honest coverage limitation; it cannot convert absence into a pass.

## 81. Frontend–Control-Plane Protocol Implementation Contract

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
The export handler resolves `packagingProfileId`, artifact kind, source revision, checkpoint, signing identity binding, validation decision, promotion decision, and destination policy before copying. It accepts only a verified declared APK for required local delivery or a declared AAB when the profile explicitly requests `APK_AND_AAB`. The deployment destination is `LOCAL_WINDOWS_FILESYSTEM`; any external deployment destination is rejected. Source, ZIP, and Git export is handled as `SOURCE_ACCESS_ONLY` and is never a deployment artifact.

### 83.2 Durable copy operation
The handler creates one `ExportVerificationRecord` before copying and records source identity, destination identity, source hash, destination hash, byte count, copy lifecycle, request fingerprint, idempotency key, and post-copy check. A copy that may have partially completed follows `UNKNOWN → RECONCILING`; destination inspection and source/destination identity and hash comparison must resolve it to `VERIFIED`, `FAILED`, or `BLOCKED` before retry. A hash or identity mismatch blocks completion and preserves the last-known-good artifact evidence. `reconciliationReference`, `failureEvidenceId`, and the corresponding external-effect or filesystem-inspection evidence are mandatory for the `UNKNOWN` and `RECONCILING` path.

### 83.3 Runtime acceptance
Acceptance fixtures prove required APK delivery, optional declared AAB behavior, rejection of undeclared artifact kinds and external deployment destinations, source/destination hash equality, destination identity, interrupted-copy reconciliation, signing/validation/promotion linkage, and refusal to treat source access as deployment completion. Documentation certification proves contract presence only; runtime certification must execute the fixtures.

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

Job Object containment is a Windows target-runtime facility already required by BS §79.3. Per CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, target_runtime_validation is USER_REQUIRED absent a Windows observation.

Path length capability is a Windows target-runtime facility already required by BS §79.3. Per CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, target_runtime_validation is USER_REQUIRED absent a Windows observation.

Security-software interference is a Windows target-runtime facility already required by BS §79.3. Per CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, target_runtime_validation is USER_REQUIRED absent a Windows observation.

Hypervisor availability is a Windows target-runtime facility already required by BS §79.3. Per CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, target_runtime_validation is USER_REQUIRED absent a Windows observation.

`ValidationEnvironment` (registry: §36.1): `environment_id`, `platform`, `architecture`, `toolchain`, `runtime`, `available_tools`, `available_devices`, `isolation_profile`, `network_policy`, `fingerprint`, `health`, `lease_id`, `reserved_by_task`, `acquired_at`, `released_at`.

`BuildGateRecord` (registry: §36.1): `gate_id`, `stage: compile | target_build | bundle | artifact_inspection | install | launch | runtime_validation | platform_specific_validation | recovery_validation | certification`, `platform`, `environment_id`, `revision`, `command_or_operation_ref`, `evidence_ids`, `result: VERIFIED | UNVERIFIED | UNAVAILABLE | USER_REQUIRED | FAILED`, `recorded_at`.

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

`TEST-PLAT-001` (evidence `EV-PLAT-001`) implements the BS §79.13 fixtures A–D and MUST additionally prove: the planner emits the extended traceability chain with the environment-requirement and capability-resolution edges populated; the target-mismatch guard rejects a runtime-validation claim from a non-matching host before execution; worker scheduling honors the `WorkerContract` platform fields; lease loss fences in-flight validation; and a matrix version change re-runs preflight without invalidating unrelated observed records. Documentation certification proves only that these contracts and fixture declarations exist; runtime certification must execute the fixtures.
