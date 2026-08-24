# Nirman Technical Architecture

## Implementation Blueprint for the Windows-First Desktop Application

**Document status:** Initial engineering architecture  
**Application:** Nirman  
**Scope:** Local-first autonomous application development with configurable cloud or local AI providers  
**Relationship to master specification:** This document explains how to implement the behavior defined in `nirman-build-spec.md`. It contains architecture and interfaces, not production source code.

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

The control plane should communicate with the user interface through a local authenticated IPC channel. A loopback HTTP or WebSocket API may be used internally, but it must require a per-installation secret or operating-system authenticated channel. The interface must not be able to impersonate another project or bypass task policies by modifying client-side state.

---

## 3. Process Model

### 3.1 Desktop user interface

The desktop interface should be built with Tauri and React/TypeScript. It displays state and sends user commands, but it should not directly execute arbitrary shell commands or mutate project files. All filesystem, process, provider, and build operations go through the control plane.

### 3.2 Control-plane process

The control plane is a user-scoped background process. It owns the task scheduler, state database, event bus, approval manager, worker registry, policy engine, and runtime manager. It should expose a stable local API to the desktop interface.

The control plane should start on user login whenever an active Goal Mode task exists, unless the user explicitly opts out for that project. A lightweight per-user startup entry should launch the stable supervisor/control-plane process without running a system service by default. If no task is active, the user may configure whether the control plane starts at login. After reboot, the supervisor must scan durable task state, reconcile process leases, and resume eligible tasks automatically without requiring the desktop UI to be opened.

### 3.3 Worker processes

Every worker runs as a child process or isolated runtime task with a declared role, model profile, workspace, permissions, limits, and task contract. A worker must not decide its own isolation profile or expand its own permissions.

A worker may use the provider router to call a model and the tool gateway to request filesystem, process, preview, browser, or external-tool actions. It cannot invoke the operating system directly outside those gateways.

### 3.4 Runtime processes

Development servers, test runners, package managers, emulators, browsers, and build commands are runtime processes. The process manager tracks each process tree and associates it with a task, worker, project, workspace, and resource profile.

The process manager must support cancellation of the whole process tree, not only the parent process. It must capture stdout and stderr separately, enforce output limits, and preserve the final diagnostic output when a process is terminated.

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
Offer resume, rollback, retry, or discard
```

A task should never resume from an unverified partial filesystem state. It should either continue from a validated checkpoint or create a recovery branch containing the partial state for inspection.

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
| Backend Worker | APIs, schemas, integrations, business logic | Assigned workspace |
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

The interface should explain when a requested operation requires a stronger profile. A worker must not be able to switch itself to a weaker profile.

### 9.2 Windows process controls

The Windows runtime should use process-tree management and resource accounting through Windows Job Objects where available. It should use restricted process tokens, controlled environment variables, explicit working directories, and deny-by-default access to protected paths.

The sandbox abstraction must not rely on a single Windows API. It should expose capabilities such as filesystem isolation, network restriction, process limits, memory limits, CPU limits, and disposable cleanup, then report which capabilities are active.

### 9.3 Network policy

Network access should be categorized as provider traffic, package-manager traffic, Android runtime traffic, emulator/device traffic, or external-tool traffic. Each category should have an independent policy.

The default autonomous build profile should allow provider requests and approved Android dependency sources only. Emulator/device runtime traffic and Android project network access should be explicitly visible. External network access should be disabled in high-risk review profiles.

### 9.4 Dependency safety

Before executing an unfamiliar dependency or install script, Nirman should record its source, version, lockfile change, requested scripts, and scan status. Unverified packages should be restricted to a disposable or explicitly approved environment.

---

## 10. Preview and Device Architecture

### 10.1 Android development preview manager

The preview manager starts the Android development server or native build process, assigns or discovers required ports, tracks the process tree, checks emulator/device readiness, installs or reloads the application, captures Logcat and runtime errors, and exposes the current device state to the desktop interface.

A preview instance must be associated with a project revision and checkpoint. If the revision changes, the preview reports whether it hot-reloaded or restarted. If the project is rolled back, the preview must be restarted or marked stale.

### 10.2 Android device-profile testing

A preview test can define multiple Android device profiles:

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

### 10.3 Android device manager

The Android device manager should provide a normalized interface for emulators and physical devices:

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

The visual worker converts references into an editable visual specification rather than directly copying pixels. The specification records screens, navigation states, layout regions, component roles, spacing, typography, colors, assets, interactions, responsive behavior across Android device profiles, and unresolved uncertainties. The implementation worker uses that specification to synthesize Android code, while the validation worker compares emulator/device screenshots against the reference and reports visual differences with evidence.

Screenshots sent to a cloud model must pass the project privacy policy. The system must redact or warn about sensitive text and identify the provider receiving the image. A visual reference is never treated as executable instruction; it is input data interpreted through the task contract.

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

Android validation must use disposable emulator snapshots or explicitly selected physical devices. It must not reuse personal credentials, host-side secrets, or unapproved device data. Test data should be synthetic by default. Device sessions, installed packages, permissions, logs, screenshots, and cleanup state must be attached to the task record.

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
- lastVerifiedAt
```

Two projects that require different Node.js, Java, Android SDK, Rust, or package-manager versions must be able to run without silently changing global state.

### 11.2 Environment diagnostics

Diagnostics should distinguish missing, incompatible, inaccessible, unverified, and healthy tools. A failed build must name the missing executable or incompatible version and explain the next action.

### 11.3 Android runtime abstraction

The runtime should expose Android-focused interfaces for process execution, filesystem policy, environment discovery, Java/Kotlin compilation, Gradle execution, JavaScript bundling when selected, native module builds, emulator/device management, Logcat, quotas, screenshots, signing-boundary checks, and APK/AAB artifacts. The Windows desktop host supplies the local process and sandbox implementation; the generated-project contract remains Android-specific and technology-neutral.

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

Shell selection must be explicit on Windows. Supported profiles may include PowerShell, `cmd.exe`, Git Bash, WSL, or a configured project shell. The selected profile, executable path, version, encoding, and environment fingerprint belong in task evidence.

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
| Android device testing | Disposable emulator snapshot or explicitly selected physical device |
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

### 16.3 Non-blocking background control

The control plane should manage background tasks independently from the UI event loop. The UI subscribes to task events and may disconnect and reconnect using a task ID and event sequence number.

When the user opens another project, the current task remains owned by the control plane. The scheduler must enforce per-project and global resource limits and must prevent background processes from taking keyboard or mouse focus.

### 16.4 Scheduling subsystem

A schedule record should contain:

```text
Schedule
- scheduleId
- projectId
- goalTemplate
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

The scheduler should calculate the next run transactionally, create a new task from the goal template, and prevent duplicate runs after a control-plane restart. A scheduled task must inherit the project’s permission policy and may not upgrade its own autonomy.

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

Android tasks should use profile-based quotas for JavaScript, native, emulator, physical-device, and combined build workflows. The quota manager must account for worktrees, dependency stores, Gradle caches, APK/AAB artifacts, emulator images, logs, screenshots, and checkpoints. It should prefer deduplicated content-addressed storage and cleanup of rebuildable caches before deleting checkpoints.

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
- privacyPolicy
- networkPolicy
- enabled
- lastConnectionTest
- lastHealthStatus
```

The API key and sensitive headers must be stored only through the operating-system keychain. The profile may display a masked key fingerprint and last validation time, but never the raw key.

### 24.3 AI Settings page behavior

The settings interface should allow the user to create, duplicate, test, disable, and delete provider profiles. It should support custom base URLs and model IDs, because local runtimes and compatible services may expose different model catalogs.

The connection test should discover or validate the configured endpoint, verify authentication, test the selected model, detect available features, measure a basic response, and record the provider request ID. Model discovery through a models endpoint is optional; a user must be able to enter a model ID manually when discovery is unavailable.

The page should show capability badges for text, vision, file input, tool calls, structured output, streaming, cancellation, background requests, embeddings, and context capacity. A capability badge must be based on a successful probe or explicit user override, not a provider name alone.

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
- reasoningSettingsOptional
- serviceTierOptional
- stream
- providerBackgroundOptional
- cancellationSignal
- privacyLabels
```

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

Nirman should use a stable launcher/controller process and a replaceable application process:

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

The runtime must model the user’s one-shot Android request as an `AutonomousAndroidSession`. The session owns the complete lifecycle from chat and screenshots to project synthesis, live preview, recovery, validation, and APK/AAB delivery. The session continues independently of the chat renderer and is resumable after UI closure, process restart, or host suspend/resume where the operating system permits it.

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

The preview manager and execution manager share a `projectRevisionId` and `checkpointId`. Every emulator or device state records the revision, device identity, installation state, reload state, Logcat stream, runtime errors, screenshot, visual comparison result, and responsible task node. If a candidate revision fails, the preview manager retains the last valid revision and marks the candidate as failed instead of presenting it as current.

### 34.3 Progress ledger and stall detector

The runtime maintains a progress ledger containing changed files, new evidence, preview movement, test transitions, worker handoffs, strategy changes, validated requirements, and artifact transitions. The stall detector identifies repeated commands, repeated patches, repeated failure fingerprints, unchanged workspaces, absent evidence, unresponsive processes, stale emulators, and heartbeats without useful progress.

A detected stall causes a controlled strategy transition: refresh context, repair the environment, change technology, delegate diagnosis, restore a checkpoint, reduce scope to a safe subtask, or construct an isolated alternative. The scheduler must reject identical retries that do not provide a new strategy or new evidence.

### 34.4 Swarm handoff and reconciliation contract

Parallel workers receive explicit contracts, isolated workspaces, allowed tools, expected outputs, and validation rules. Each handoff must include changed files, assumptions, dependencies, tests, evidence, unresolved issues, and recommended next actions. The reconciliation worker integrates only validated outputs, resolves conflicts, runs integrated Android checks, updates the preview revision, and commits the next checkpoint.

### 34.5 Autonomous validation and artifact gate

For applicable Android delivery, the validation coordinator must prove build success, APK/AAB existence, checksum, artifact scan, installation or launch, main-flow execution, visual comparison, permission behavior, and absence of unresolved fatal runtime errors. The artifact is complete only when it is linked to the project revision and evidence ledger.

Routine project-local actions are allowed under the project’s Unattended / Full Autonomy policy. The runtime may edit, install dependencies, run terminals, launch devices, build, test, capture screenshots, repair, checkpoint, delegate, reconcile, and create local artifacts without repeated approval. Protected credentials, destructive actions, publishing, signing policy, protected paths, hard safety violations, and unrecoverable blockers remain deterministic authority boundaries.

## 35. Complete Android Capability Fixture Contract

The test harness must include generated-from-instruction fixtures for JavaScript-driven Android, Java, Kotlin, Android Views, Jetpack Compose, mixed architectures, custom native modules, background services, WorkManager, notifications, camera and media, location and sensors, Bluetooth and NFC, offline-first storage, API-heavy applications, authentication and permissions, tablet and multi-orientation layouts, device-integrated applications, and APK/AAB delivery. These fixtures validate AI technology selection and composition; they are not user-facing templates.

## 36. Production Runtime Contract Architecture

The production runtime is divided into deterministic authorities and model-driven proposal services. The model gateway proposes plans, edits, tool calls, recovery strategies, and improvement proposals. The supervisor, lifecycle authority, permission authority, sandbox authority, storage authority, evidence authority, recovery authority, promotion authority, and termination authority decide what can execute and what counts as complete.

### 36.1 Canonical runtime contracts

The following contracts are versioned and validated at the control-plane boundary:

```text
AutonomousAndroidSession
AndroidApplicationContract
VisualSpecification
AndroidTechnologyPlan
TaskGraph
WorkerContract
TerminalSession
PreviewRevision
EvidenceRecord
RecoveryRecord
ArtifactRecord
ProviderProfile
```

Each contract has a schema version, owner, lifecycle status, project scope, source revision, created timestamp, updated timestamp, and audit references where applicable. Persistent records use atomic writes, file locking, migration backups, and rollback.

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

Generated code and project processes cannot read personal browser data, SSH keys, unrelated directories, signing keys, or arbitrary credentials. Sandbox profiles are selected by the policy authority and cannot be relaxed by model output.

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
| Preview coordinator | Revision-bound emulator/device deployment and preview fallback | Promoting stale preview state |
| Artifact authority | APK/AAB packaging, checksums, signing workflow, promotion | Modifying source without a transaction |

The invariant is:

> **The model proposes. Deterministic runtime authorities decide, execute, validate, recover, and promote.**

### 44.2 Runtime module graph

```text
Tauri React/TypeScript UI
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

Examples include dependency installation, device access, external network requests, signing, keystore use, writing outside generated source scope, and self-development promotion.

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
├── detect(path)
├── parse(path, content)
├── index_symbols(parsed_unit)
├── resolve_references(index)
├── calculate_affected_nodes(change)
├── validate_structured_patch(patch)
└── format_or_serialize(updated_unit)
```

Adapters are selected by file type and technology plan. No single parser is mandatory for every Android project.

### 47.4 Impact analysis

The graph service calculates affected files, modules, resources, permissions, tests, device profiles, preview surfaces, and artifact outputs. The affected-test set is persisted with each transaction and evidence record, so long-horizon sessions can validate changed behavior without rebuilding unrelated areas unnecessarily.

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

The gateway normalizes Chat Completions, Responses-style, and message-oriented providers into one internal representation containing text blocks, image blocks, tool calls, tool results, structured output, usage, finish reason, and retryability.

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

`ToolchainAuthority` resolves the technology plan to a verified `AndroidToolchainLock`. It checks versions, file hashes, licenses, paths, compatibility constraints, and required environment variables before any build or preview command.

The isolated environment controls JDK, Gradle, Android SDK, build tools, platform tools, NDK, CMake, ADB, emulator, Node/package manager when selected, Metro/Expo when selected, temporary directories, Gradle caches, package caches, and project-local configuration. Host PATH and unrelated user configuration are not trusted.

### 49.2 EnvironmentSnapshot

The environment snapshot includes toolchain lock hash, tool versions and hashes, selected device identity, API level and ABI, build variant, relevant environment variables, Gradle and package lock hashes, provider metadata without secrets, project fingerprint, and command policy. It is attached to build, recovery, preview, and artifact evidence.

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

The coordinator supports incremental emulator install, Compose reload, React Native/Expo fast refresh, full APK reinstall, physical device execution, headless smoke tests, and diagnostic-only source preview. Diagnostic preview can support recovery but can never satisfy final completion.

A `PreviewRevision` includes source revision, artifact hash, device serial/profile, API level, build variant, technology-plan hash, preview mode, launch timestamp, health status, screenshot IDs, and Logcat evidence.

---

## 51. Repair Registry, Decision Trace, and Resource Governor

### 51.1 Repair registry

`AndroidRepairRegistry` maps structured failure fingerprints to repair strategies. Each pattern contains classifier, severity, likely cause, allowed scope, preconditions, operation type, retry budget, checkpoint rule, validation command, and evidence requirements.

Patterns cover JDK/Gradle/AGP/Kotlin/Compose compatibility, missing SDKs, Gradle/dependency conflicts, resource and manifest errors, DEX/R8 failures, NDK/native-module failures, Metro/Expo failures, emulator/ADB/install failures, runtime crashes, permission errors, visual/accessibility issues, and APK/AAB/signing failures.

A learned repair can be promoted into the trusted registry only after repeated successful validation across independent fixtures. Model suggestions remain untrusted until promoted by deterministic evidence.

### 51.2 DecisionTrace service

The service records concise decision summaries without hidden chain-of-thought. It stores inputs, constraints, candidate actions, selected action, policy checks, provider/model provenance, confidence, outcome, and evidence references. The UI can show why a technology, worker, repair, checkpoint, preview mode, or provider was selected.

### 51.3 ResourceGovernor

The governor monitors CPU, RAM, disk, checkpoint storage, emulator memory, Gradle memory, worker/provider concurrency, context size, log volume, build duration, and device slots. It can compact context, reduce concurrency, prune safe caches, stop redundant workers, select affected tests, defer nonessential checks, or use an approved lighter provider profile. It cannot weaken sandbox, permission, evidence, signing, or artifact gates.

---

## 52. Technical Acceptance Tests

The architecture is accepted only when killing the supervisor during a transaction leaves a recoverable event log and checkpoint; replaying events reconstructs the same authoritative session state; stale worker proposals are rejected without changing the project; changed files or toolchain locks invalidate pending transactions through TOCTOU checks; parallel workers can analyze and propose while conflicting writes are serialized; provider bridge restart and protocol mismatch do not corrupt the session; builds use the locked Android toolchain; preview promotion rejects stale source or artifact revisions; resource pressure changes scheduling without bypassing completion gates; and an APK/AAB is not promoted without revision, checksum, environment, validation, and signing evidence.
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

Nirman uses native Windows isolation as its required execution model: restricted tokens, Windows Job Objects, ACL-scoped workspaces, environment filtering, process-tree supervision, resource quotas, toolchain isolation, and disposable Android emulator snapshots. This model is self-contained and must preserve Android emulator, GPU, and physical-device workflows.

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

The control plane exposes an authenticated local event stream over the existing Tauri event/IPC boundary or an authenticated loopback WebSocket. Every subscription is bound to the current installation, user session, project, and requested task scope.

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

The worker is scoped to the asset transaction and cannot modify unrelated source, change the technology plan, grant permissions, or mark the APK/AAB complete.

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
| Build packaging | Asset is present in the built APK/AAB and reachable at runtime |
| Revision | Workspace, preview, and artifact all reference the same AssetManifest version |

Validation results are evidence records and are linked to the source revision, PreviewRevision, and artifact hash.

### 56.5 Asset transaction and impact analysis

Brand changes use the normal ConstructionTransactionManager. The transaction captures the previous BrandManifest version, affected assets, resource files, manifest references, impacted screens, preview surfaces, and artifact outputs. It regenerates only the affected assets where the impact graph proves independence, invalidates stale asset evidence, refreshes the preview, and reruns the asset gate.

### 56.6 ArtifactAssetInspector

`ArtifactAssetInspector` runs after APK/AAB creation and before artifact promotion. It extracts and verifies launcher resources, adaptive and monochrome icon layers where required, splash resources, notification assets, in-app assets, theme resources, and font/illustration references. It compares extracted content hashes with AssetManifest entries and rejects an artifact with missing, stale, wrong-path, or placeholder-only requested assets.

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
5. APK/AAB extraction confirms requested assets are actually packaged.
6. Stale AssetManifest versions cannot satisfy PreviewRevision or artifact gates.
7. Branding changes invalidate affected evidence and regenerate only impacted assets.
8. Provider failure and fallback behavior are explicit and replayable.
9. Placeholder-only output blocks completion when branded assets were requested.
## 57. Locked Implementation Stack and Supervisor Process Architecture

### 57.1 Implementation stack

Nirman v1 uses Tauri 2 with React/TypeScript/Vite on the presentation side and Rust/Tokio for the authoritative local runtime. Tailwind CSS and shadcn/ui provide the initial design system. CodeMirror 6 is the initial editor and xterm.js is the terminal renderer. SQLite is the execution ledger; SQLx is the preferred asynchronous access layer, with rusqlite evaluated only if synchronous operations are isolated from Tokio scheduling.

The Windows runtime uses native APIs: ConPTY for terminals, restricted process tokens, Windows Job Objects, ACL-scoped workspaces, environment filtering, process-tree supervision, and resource quotas. Android tooling remains externally installed or managed by the toolchain authority and includes JDK, Gradle, AGP, SDK, ADB, emulator, NDK/CMake when required, and Node/Metro/Expo only when the technology plan selects them.

### 57.2 Process topology

```text
Nirman.exe
└── Tauri 2 + React/TypeScript/Vite
    ├── Chat and project navigation
    ├── CodeMirror editor
    ├── xterm.js terminal views
    ├── Android preview presentation
    ├── task graph and reasoning stream
    └── typed authenticated IPC/events
              │
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

The first implementation may host these supervisor modules in the Tauri Rust backend, but all interfaces must be designed so they can move into `NirmanSupervisor.exe` without changing the UI contract.

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

### 57.4 SupervisorLifecycle and recovery scan

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

Large logs, screenshots, diffs, patches, crash dumps, build output, and APK/AAB files remain in the filesystem artifact store with content hashes, revision references, and retention metadata. All durable records use migrations, atomic writes, schema versions, and integrity checks.

### 57.6 UIProjectionState

The React application maintains only presentation state: selected project, open tabs, expanded task nodes, filters, scroll position, optimistic form values, and the last acknowledged event sequence. It receives authoritative task, worker, preview, reasoning, evidence, and health state from the supervisor.

On reconnect, the UI discards stale projections and rebuilds them from the supervisor snapshot plus durable events. No client-side state can mark a task complete, authorize a command, promote an artifact, or change a policy.

### 57.7 Terminal architecture

```text
React xterm.js
      ↓ typed Tauri event/command
Supervisor TerminalSupervisor
      ↓
Windows ConPTY
      ↓
PowerShell / cmd.exe / Git Bash / approved shell
```

Rust owns working directory, environment snapshot, shell profile, process group, input policy, output limits, searchable rolling logs, cancellation, tree termination, heartbeat, and recovery. xterm.js renders output and sends user input through policy-checked commands; it never owns the process.

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

### 57.9 Git and worktree subsystem

Git is a first-class subsystem for checkpoints, rollback, worker isolation, reconciliation, diffs, revision identity, recovery branches, and artifact provenance. Parallel workers use isolated worktrees or copy-on-write fallback. Reconciliation produces an integration revision only after conflict, dependency, requirement, and test-impact checks pass.

### 57.10 Technical acceptance tests

The architecture passes when the UI can restart while the supervisor continues a task; the supervisor can start after Windows reboot and recover eligible sessions; SQLite reconstructs the same state after event replay; ConPTY terminals survive reconnect; stale UI projections cannot mutate authority; provider proposals cannot bypass ToolBroker or PolicyAuthority; CodeMirror and xterm.js remain presentation components; Android toolchains are supervised locally; and the final APK/AAB remains bound to source revision, toolchain lock, preview, evidence, and artifact checksums.


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

`SwarmPlanner` analyzes change surface, dependencies, symbols, requirements, risk, validation cost, capability graph, workspace capacity, device availability, provider concurrency, and resource pressure. It emits a `SwarmPlan` containing parallel groups, serialized dependencies, worker profiles, interfaces, leases, capacity reservations, and integration checkpoints.

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

`KnowledgeLedger` stores typed, scoped `KnowledgeArtifact` records. `TaskBlackboard` is a task-scoped projection containing the goal, requirements, architecture, decisions, constraints, assumptions, active workers, completed/blocked work, findings, conflicts, evidence, known failures, and next actions.

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

`ToolCapabilityGraph` maps an outcome to capability requirements, skills, worker profiles, tools, and environment prerequisites. For example, Android BLE validation may require Android APIs, a compatible SDK, a native module, Bluetooth permissions, ADB, an emulator or selected physical device, and device-test capability.

`EnvironmentCapabilityPlanner` evaluates each prerequisite before expensive execution and classifies it as `AVAILABLE`, `REPAIRABLE`, `USER_REQUIRED`, or `UNAVAILABLE`. It records the toolchain lock, environment fingerprint, repair attempt, and evidence used for the classification.

### 58.9 ValidationPlanner and mutation regression analysis

`ValidationPlanner` chooses validation from changed files, symbols, call graph, route graph, dependency graph, requirement traceability, project type, risk, previous failures, device profiles, and resource availability. `MutationRegressionAnalyzer` predicts affected behavior and expands validation when a change touches a manifest, permission, navigation route, data model, native module, build file, authentication boundary, or shared UI component.

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

`BackpressureController` reserves and queues Gradle processes, emulator slots, physical devices, GPU capacity, storage, and provider concurrency. It applies priority and fairness, exposes waiting reasons, and reduces parallelism before system pressure becomes failure.

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
6. Only ArtifactAuthority promotes APK/AAB output.
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

Implements build spec §56. Extends §35 (Complete Android Capability Fixture Contract) and §50 (Preview Coordinator and Android Runtime Validation), which remain the authority on device sessions and fixtures.

### 62.1 Components

| Component | Responsibility |
|---|---|
| ScenarioRegistry | Stores scenario definitions and requirement links |
| ScenarioCompiler | Translates a scenario into instrumentation and ADB steps |
| SeedDataProvisioner | Establishes preconditions through the app's own data layer |
| ScenarioExecutor | Runs steps against a device session and records results |
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

## 65. Multi-Device Scenario Coordinator

**ContractId:** `CONTRACT.RUNTIME.DEVICE_MATRIX`  
**Authoritative build-spec section:** §59  
**Role:** implementation of the named contract; adds no normative clause to it.

Implements build spec §59. Extends §49 (Android Toolchain Authority and Environment) and §50 (Preview Coordinator), which remain the authority on device health and session lifecycle.

### 65.1 Components

| Component | Responsibility |
|---|---|
| DeviceMatrixResolver | Resolves the declared matrix against actually available devices |
| DevicePool | Allocates and recycles emulator and physical device sessions |
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
- passingDevices
- failingDevices
- differingAttributes: apiLevel | density | formFactor | abi | vendor
- classification: defect | environment_limitation
- evidenceRefs
```

Default classification is `defect`. Classification as `environment_limitation` requires cited evidence that the failure originates in the device or vendor rather than the application.

### 65.5 Capability status mapping

CoverageReporter maps results to the build spec §5.5 vocabulary: all matrix devices passed yields `SUPPORTED`; primary passed with declared gaps yields `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS`; primary passed and a secondary failed yields `DEGRADED` with the divergence cited; primary unavailable yields `USER_REQUIRED`.

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
- objective: text
- question: text
- passCount: int
- toollessPassCount: int
- hypothesesConsidered: hypothesisId[]
- hypothesesRejected: hypothesisId[]
- evidenceAcquired: evidenceRef[]
- alternativesConsidered: { strategy, rejectionReason }[]
- selectedStrategy: text
- rejectedStrategies: { strategy, refutingEvidenceRef }[]
- uncertaintyBefore: float
- uncertaintyAfter: float
- confidenceBefore: float
- confidenceAfter: float
- continuationReasons: { pass, reason }[]
- reasonForTermination: text
- modelProfilesUsed: profileId[]
- providerRequestRefs: requestId[]
- resourceUsage: { reasoningTokens, modelRequests, wallClockMs }
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

### 72.4 Pass loop

```text
enter deliberation (from HYPOTHESIZE or STRATEGIZE)
  -> ReasoningEffortSelector: request + policy + capacity -> granted level
  -> loop:
       DeliberationBudgetManager.checkPass()
         exhausted -> terminate BUDGET_EXHAUSTED
       execute pass at granted level
       DeliberationProgressEvaluator.measure()
       DiminishingReturnDetector.classify()
         no_progress -> force GATHER_EVIDENCE | escalate model | BRANCH
                        | DELEGATE | ESCALATE ; never another plain pass
       if toollessPassCount == maxToollessPasses
           -> EvidenceAcquisitionPlanner must run before any further pass
       SufficiencyEvaluator.evaluate()
         sufficient -> terminate SUFFICIENT
  -> emit DeliberationRecord
  -> return control to AgentReasoningEngine
```

The loop has no path from a pass directly to execution. Sufficiency returns to the reasoning engine, which emits the ReasoningArtifact and submits it for authorization.

### 72.5 ReasoningEffortSelector

The selector computes the granted level as the minimum of the requested level, the policy ceiling for the task's risk class, the level the remaining budget can fund, and the level the routed provider actually supports. The grant, the requested level, and the binding constraint are recorded, so a downgrade is visible rather than silent.

The selector must have no capability to raise a permission ceiling and no path to the policy engine's grant functions. Effort and permission are separate axes by construction.

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
| Provider fails mid-session | Resume the session on failover; provider continuation state reissued |
| Record fails admissibility | Rejected at write; deliberation cannot report sufficiency |

No failure mode permits presenting an unvalidated leading strategy as sufficient.

### 72.11 Architecture tests

The runtime is correct only when an agent request for EXHAUSTIVE under a policy ceiling of EXTENDED is granted EXTENDED with the constraint recorded; when a deliberation exceeding its pass budget terminates BUDGET_EXHAUSTED and the cycle does not execute the leading strategy; when consecutive observation-free passes are refused at the bound until evidence is acquired; when a high-risk change is refused sufficiency with a stated confidence of 0.95 and a missing regression plan; when a discriminating test refutes the leading hypothesis and the selected strategy changes as a result; when a counterexample finding returns the cycle to strategy selection without mutating the project; when an escalated model executes under the identical permission ceiling; when a forced context compaction preserves active hypotheses and rejected strategies and the session resumes without re-deriving them; when consecutive passes of flat uncertainty reaching the **configured** `diminishingReturnThreshold` produce NO_PROGRESS and an approach change rather than a further plain pass; when the ledger shows zero project mutation events between deliberation entry and the kernel `AUTHORIZE` grant; when an effort escalation carries a `grantDecisionReason` citing the observed condition that triggered it; and when no deliberation record in the ledger contains verbatim model reasoning.

The threshold is configuration, not a runtime constant. No component may hardcode a pass count for `NO_PROGRESS`: the classification is a function of the configured threshold, the measured per-pass movement, and consecutive-pass semantics. A test fixture supplies its own threshold value, and a runtime that behaves identically regardless of the configured value has not implemented the detector.

## References

[1]: https://tauri.app/ "Tauri Documentation"

[2]: https://sqlite.org/docs.html "SQLite Documentation"

[3]: https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects "Windows Job Objects"

[4]: https://git-scm.com/docs/git-worktree "Git Worktree Documentation"

[5]: https://playwright.dev/docs/intro "Playwright Documentation"


[7]: https://developers.openai.com/api/reference/overview "OpenAI API Reference Overview"

[8]: https://platform.openai.com/docs/api-reference/responses/create "Responses API Create Reference"

[9]: https://platform.openai.com/docs/api-reference/chat/create "Chat Completions Create Reference"


---

