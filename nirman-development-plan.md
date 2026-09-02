# Nirman Engineering Development Plan

## Ordered Build Plan for the Desktop Application

**Document status:** Living implementation roadmap — contract-gated milestones
**Application:** Nirman  
**Primary engineering rule:** Build the local control plane and recovery model before attempting broad autonomous capabilities.

**Canonical ownership:** The Build Spec owns product contracts, invariants, and capability/contract registries. The Technical Architecture owns implementation schemas, protocols, and module boundaries. The Development Plan owns sequencing, milestones, fixtures, and exit gates. The Decision Log owns accepted decisions, rationale, and supersession. The README is explanatory only. AGENTS defines agent operating constraints only. The verifier certifies documentation and semantic checks only; it is never a runtime authority.

---

## 1. Development Strategy

Nirman should be built in vertical slices. Every milestone must produce a usable and testable part of the application instead of completing isolated infrastructure with no end-to-end workflow.

The first usable slice should allow a user to open Nirman, configure an AI provider, create an Android project, ask for a small change, review a plan, execute policy-allowed edits or approve a hard-gated action, run an emulator or device preview, execute validation, and undo the task. The next slices should make that flow resilient to long-running tasks, worker failures, application closure, parallel work, Android packaging, and device testing.

The team should keep the master specification stable as the product contract, update the technical architecture when implementation decisions change, and record significant trade-offs in the decision log.

---

## 2. Delivery Milestones

| Milestone | Focus | Main output |
|---|---|---|
| M0 | Repository and engineering foundation | Source repository, conventions, local certification, fixture projects |
| M1 | Desktop shell and local workspace | Windows application shell and project manager |
| M2 | Control plane and persistent state | Background task daemon, SQLite state, event stream |
| M3 | Provider and model runtime | Provider profiles, keychain, streaming, usage telemetry |
| M4 | Dynamic Android synthesis and local runtime | Instruction/screenshot analysis, framework resolver, emulator/device preview, process manager, diagnostics |
| M5 | Single-worker agent loop | Plan, inspect, edit, test, repair, checkpoint, undo |
| M6 | Permissions and sandbox profiles | Policy engine, approvals, restricted execution |
| M7 | Background execution and recovery | Resume after UI close or restart, notifications, adaptive guardrails |
| M8 | Multi-worker coordination | Canonical workers, contracts, event bus, isolated worktrees, reconciliation |
| M9 | Android device and visual testing | Emulator/device profiles, screenshots, Logcat, phone/tablet checks |
| M10 | Android packaging | APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` build, artifact validation, signing boundaries |
| M11 | Android capability registry and representative profile coverage | Internal profile identity, AI-selected technology compositions, toolchain/device matrix, and representative fixture evidence |
| M12 | Advanced extensibility | Skills, external tools, hooks, model routing, scheduled tasks |
| M13 | Goal Mode and non-blocking background work | Durable goals, resumable tasks, background control |
| M14 | Lifecycle hooks | Deterministic pre/post action hooks and policy interception |
| M15 | Scheduled automations | Persistent local schedules, fairness, safe recurring tasks |
| M16 | Granular checkpoints and backtracking | File/task checkpoints, retention, restore, strategy changes |
| M17 | Context scaling and external tools | Retrieval/large-context modes and mediated adapters |
| M18 | Durable task graph and execution tree | Nested live progress, worker nodes, evidence links |
| M19 | Evidence-backed status and telemetry | Event ledger, heartbeats, resource and validation telemetry |
| M20 | Autonomous validation coordinator | Dependency-aware checks, affected tests, regression sharding |
| M21 | Policy-boundary approvals and termination | Unattended profile, truthful termination, hard safety boundaries |
| M22 | Provider-neutral AI settings and model gateway | Chat, response-item, message protocols, tools, streaming |
| M23 | Controlled self-development loop | Candidate build, health checks, promotion, rollback |
| M24 | Adaptive long-horizon provider execution | Context compaction, routing, continuation, provider recovery |
| M25 | Runtime supervisor and durable execution loop | Runtime ticks, leases, restart recovery, continuity |
| M26 | Graduated recovery ladder | Failure fingerprints, strategy changes, backtracking |
| M27 | Self-observation and episode evaluation | Quality metrics, fixtures, trajectory replay |
| M28 | Self-improvement proposal manager | Improvement hypotheses, scoped candidates, test plans |
| M29 | Candidate canary, promotion, and rollback | Baselines, canaries, post-promotion monitoring |
| M30 | Canonical documentation and worker registry | Renumbered sections, one role taxonomy, roadmap crosswalk |
| M31 | Unattended / Full Autonomy profile | Routine in-workspace actions allowed; deployment and signing gated |
| M32 | Persistent terminal subsystem | PTYs, interactive prompts, shell profiles, multi-terminal logs |
| M33 | Skills registry and invocation contract | Skill schema, scanning, permissions, versioning, rollback |
| M34 | Windows lifecycle and multi-project resilience | Reboot autostart, sleep/resume, notification fallback, fair scheduling |
| M35 | Long-horizon scale and unified execution surface | Map sharding, checkpoint retention, affected tests, side-by-side preview |
| M36 | Runtime authority and autonomous recovery invariants | Deterministic authorities, model non-authority, safe recovery, evidence gates |
| M37 | Android-only target contract | Android profiles, emulator/device validation, APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` artifacts, and Android-only project resolution |
| M38 | Certified Android profile coverage and production acceptance | Certified profile matrix, mixed architectures, Android capability classes, end-to-end APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` validation, and evidence reports |

---

## 3. M0: Repository and Engineering Foundation

### Objectives

Create the source repository and define the code-quality baseline before implementing agent behavior. The repository should include separate packages for the desktop UI, control plane, agent runtime, policy engine, provider runtime, project runtime, and tests.

### Work items

| Work item | Acceptance condition |
|---|---|
| Repository layout | Modules follow the architecture boundaries |
| TypeScript and Rust conventions | Formatting, linting, and type checks run through the local certification command |
| Configuration model | Development, test, and production settings are separate |
| Logging standard | Structured logs include task, worker, project, and correlation IDs |
| Test fixtures | At least three representative Android projects exist |
| Security baseline | Secret files are excluded from logs and test fixtures |
| Local certification pipeline | Documentation, foundation, Rust, frontend, fixture, and static checks run through the local certification command |

### Exit gate

A clean checkout can install dependencies, run local static checks, execute unit tests, validate Android fixtures, and start the development shell without manually editing configuration files. GitHub, GitHub Actions, or any hosted CI service is not required for this gate.

---

## 4. M1: Desktop Shell and Local Workspace

### Objectives

Build the visible Windows application shell and the project-management experience. The shell should open local folders, display the workspace layout, and communicate with a placeholder control plane.

### Work items

1. Implement the Tauri shell and React interface.
2. Add welcome, create-project, open-project, and recent-project screens.
3. Add the main workspace layout with chat, file tree, editor, preview, tasks, and logs regions.
4. Add project metadata storage without secrets.
5. Add application-level error handling and restart messaging.
6. Add keyboard navigation and accessible status indicators.

### Exit gate

A user can create a workspace, reopen it, view project metadata, close the application, and return to the same project without data loss.

---

## 5. M2: Control Plane and Persistent State

### Objectives

Implement the local control plane before building the autonomous agent. The control plane must own tasks, workers, events, approvals, checkpoints, and recovery.

### Work items

| Component | Required behavior |
|---|---|
| IPC API | Authenticated local communication between UI and daemon |
| SQLite store | Versioned schema with migrations and transactions |
| Event bus | Durable events with task sequence numbers and replay |
| Task scheduler | Idempotent state transitions and resource reservations |
| Process registry | Track process trees, ports, output, and ownership |
| Recovery scanner | Rehydrate interrupted tasks after restart |
| Notification adapter | Surface approvals and failures while minimized |

### Exit gate

A test task can be created, persisted, streamed to the UI, paused, resumed, cancelled, and recovered after stopping and restarting the control plane.

---

## 6. M3: Provider Runtime Foundation

### Objectives

Implement the minimum provider-runtime foundation required to unblock Android construction. M22 remains the full provider-neutral gateway and capability-certification milestone; M3 must not duplicate that later machinery.

### Work items

1. Define `ProviderProfile` with provider identity, base URL, protocol, model ID, and capability declarations.
2. Store only a secure credential reference; raw API keys must not enter project files, logs, or durable task summaries.
3. Implement an authenticated provider request with a request ID and correlation to the control-plane operation.
4. Normalize successful responses into one local response shape.
5. Implement deterministic timeout and cancellation handling for the request.
6. Classify basic authentication, transport, timeout, cancellation, and invalid-response failures.
7. Persist a durable usage record containing request identity, provider/model identity, timing, and provider-reported or explicitly estimated usage.

M3 does not implement the full reasoning, capability-routing, continuation, multi-provider fallback, or long-horizon machinery owned by M22 and later milestones.

### Exit gate

A configured `ProviderProfile` can execute an authenticated request and produce a normalized response or typed basic failure, with timeout/cancellation behavior, request identity, durable usage, and secret-redaction evidence. This foundation does not claim the complete provider-neutral gateway or autonomous Android completion.

---

## 7. M4: Dynamic Android Project Synthesis and Local Runtime

### Objectives

Create the Android construction foundation needed to turn a validated goal and visual references into an Android-only project workspace, selected technology plan, detected toolchain requirements, environment/preflight record, and durable checkpoint. Later milestones own full build, device, preview, artifact, and runtime certification.

### Work items

1. Define and validate an `AndroidConstructionContract` from the user goal, visual references, assets, device requirements, and technology plan.
2. Enforce `targetPlatforms == ["android"]` at project construction.
3. Implement the first Android technology-plan resolver without exposing a fixed template or framework picker as the primary creation path.
4. Implement environment diagnostics for Java, Gradle, Android SDK, platform-tools, emulator/device tooling, package managers, and required project dependencies.
5. Create the Android project workspace, record its source identity, and persist a preflight record describing available, repairable, user-required, and unavailable prerequisites.
6. Create a durable checkpoint before subsequent mutation or runtime work.

### Exit gate

A valid `AndroidConstructionContract` produces an Android project workspace with `targetPlatforms == ["android"]`, a selected `AndroidTechnologyPlan`, detected toolchain requirements, an environment/preflight record, and a durable checkpoint. The project may be built or previewed where the declared environment supports it. M4 does not certify autonomous completion or artifact delivery.

---

## 8. M5: Single-Worker Autonomous Development Loop

### Objectives

Implement the first complete agent loop with one worker. Do not add parallel workers until this path is reliable.

### Agent stages

```text
Inspect → Plan → Checkpoint → Mutate → Build → Install/Launch
→ Observe → Validate → Repair → Checkpoint
```

### Work items

1. Implement authorized tools for inspect, search, read, write, patch, command, build, install/launch, observe, validate, diff, checkpoint, repair, and rollback.
2. Add plan and Android acceptance-contract generation from the selected `AndroidTechnologyPlan`.
3. Add file-change grouping, source-revision tracking, and diff display.
4. Route every tool call through the M115 command envelope, control plane, authority, transaction, and durable event path; React must not invoke Android tooling directly.
5. Add an automatic checkpoint before mutation and a rollback/undo reference after a failed repair or validation.
6. Add Gradle/build execution, installation or launch, runtime observation, and diagnostic capture.
7. Add failure classification, injected-failure fixtures, and focused repair prompts.
8. Add bounded retry limits and escalation when the worker is stuck or the environment is unavailable.
9. Add a final structured task result and evidence summary linked to the source revision and checkpoint.

### Exit gate

A single worker completes the first real Android fixture through `Inspect → Plan → Checkpoint → Mutate → Build → Install/Launch → Observe → Validate → Repair → Checkpoint`. Required proof includes a real Android source revision, real Gradle/build result, real runtime or device observation where the declared environment supports it, real diagnostics, durable evidence records, repair after an injected failure, and a rollback/undo path. This gate does not by itself claim `RUNTIME_CERTIFIED` autonomous Android end-to-end completion; that claim belongs to M80 and later applicable runtime gates.

---

## 9. M6: Permissions and Sandbox Profiles

### Objectives

Make execution safe before enabling autonomous background work.

### Work items

1. Implement allow, ask, and deny policy outcomes.
2. Add path rules, command patterns, external-directory rules, network categories, and worker-specific policies.
3. Add protected-file defaults for environment secrets, keychains, personal directories, and credentials.
4. Add process-tree cancellation and resource quotas.
5. Implement the restricted Windows process profile.
6. Add native Windows restricted-process, ACL, Job Object, resource-quota, toolchain-isolation, and disposable-emulator-snapshot boundaries.
7. Add dependency and artifact safety checks.
8. Add repeated-action and doom-loop detection.
9. **Open contract-gap work item — ArtifactExport/PreviewStart portable policy metadata:** ArtifactExport currently exposes only source revision and destination path at the command boundary, while the durable implementation does not yet expose the canonical export-verification record containing packaging profile, verified artifact identity, source/destination identities, request fingerprint, and `UNKNOWN`/`RECONCILING` copy state. PreviewStart currently exposes task, project revision, checkpoint, source fingerprint, device identity, and changed paths, but not an authoritative workspace root or build identity. Do not apply broad or guessed M6 authorization to either schema. Close this gap only by wiring the canonical export record and preview event identity into the policy boundary; rules requiring a real device session or raw signing material remain deferred to their owning milestones. This is an open contract/integration dependency and is not an M6 completion claim.
10. **Partial closure of §9 work item 9 — command-payload field extensions:** the ArtifactExport command payload now exposes the six fields the M6 policy needs to be authoritative on the command boundary (`packaging_profile_id`, `artifact_kind`, `request_fingerprint`, `idempotency_key`, `deployment_delivery`, `destination_kind`); the PreviewStart command payload now exposes `workspace_root` and `build_identity` as optional fields. The canonical `ExportVerificationRecord` still has 22 other fields (artifact/destination/source file identity, hashes, byte count, lifecycle state, post-copy check, policy decision, signing/validation/promotion binding, reconciliation/failure evidence) that remain exposed only via the durable observation, not the command payload. A new verifier check (`command_payload_field_coverage`) was added to enforce command-payload field coverage against the canonical records so this drift class cannot silently reappear.

### Exit gate

A restricted worker cannot read protected files, write outside its workspace, execute denied commands, exceed its quota without a durable event, or bypass an explicit deny rule through autonomous mode.

---

## 10. M7: Background Execution and Recovery

### Objectives

Allow tasks to continue when the UI is minimized or closed and recover safely after control-plane or operating-system restart.

### Work items

1. Run the control plane as a user-scoped background process when enabled.
2. Persist task, worker, event, approval, checkpoint, and recovery records.
3. Add heartbeats and stale-worker detection.
4. Add pause, resume, cancel, retry-from-checkpoint, and fork behavior.
5. Add operating-system notifications for approval and failure events.
6. Add adaptive telemetry and guardrails for time, turns, tokens, cost, disk, and processes; do not impose a fixed completion deadline unless the user explicitly configures a hard safety cap.
7. Add a startup recovery scan and repairable-interruption state.
8. Add optional local interval and schedule support for safe tasks.

### Exit gate

A task can run while the application is minimized, can request approval, can be approved after the UI returns, can survive application restart, and can resume from a verified checkpoint after a simulated process failure.

---

## 11. M8: Multi-Worker Coordination

### Objectives

Add specialized workers and isolated parallel execution only after the single-worker loop and background runtime are reliable.

### Work items

1. Implement worker roles and task contracts.
2. Implement durable worker messages and acknowledgements.
3. Implement a shared task ledger with atomic task claims.
4. Add dependency-aware scheduling.
5. Add isolated Git worktrees or copy-on-write workspace fallback.
6. Add worker heartbeats, crash recovery, and per-worker budgets.
7. Implement review, test, debug, and reconciliation worker chains.
8. Implement changed-file and changed-symbol conflict detection.
9. Add transactional integration checkpoints.

### Exit gate

Three independent workers can work on isolated tasks, return structured handoffs, and integrate without changing the main workspace until reconciliation and validation succeed. A forced conflict must be detected and presented rather than silently overwritten.

---

## 12. M9: Android Device Testing and Visual QA

### Objectives

Add visual and Android emulator/device verification without exposing personal credentials or unapproved device state.

### Work items

1. Launch a disposable Android emulator snapshot or connect an explicitly selected physical device.
2. Add screen and flow navigation, synthetic form interaction, Logcat capture, and screenshot capture.
3. Add named Android device profiles, orientation profiles, and custom device testing.
4. Add visual baseline storage and comparison metadata.
5. Add accessibility and responsive-layout checks.
6. Add screenshot references to worker handoffs and final task results.

### Exit gate

A device worker can test an Android fixture on selected phone and tablet profiles using synthetic data and return reproducible screenshots, Logcat diagnostics, permission results, and crash traces without reading personal device data.

---

## 13. M10: Android Packaging

### Objectives

Package supported Android projects as installable APK artifacts; produce an AAB only when the active PackagingProfile requires `APK_AND_AAB`, and provide reliable local build outputs.

### Work items

1. Add Android application metadata, icons, package identifiers, and build profiles.
2. Add local debug and release build profiles and artifact directories.
3. Add APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` generation and installation workflows.
4. Add build logs, checksums, and artifact scanning.
5. Add release review and explicit publish/signing approval.
6. Add emulator and device installation validation.

### Exit gate

A supported Android project can be built into an installable APK artifact, or an AAB artifact only when the active PackagingProfile requires `APK_AND_AAB`; the artifact path and checksum are recorded, secrets are scanned, and the user can install or locate the result without a hosted build service.

---

## 14. M11: Android Capability Registry and Representative Profile Coverage

### Objectives

Implement the internal Android capability registry and profile identity used for AI-driven selection and composition. Record technology composition, toolchain locks, device matrices, fixtures, known exclusions, and evidence status for representative Java, Kotlin, Android Views, Jetpack Compose, Expo/React Native, custom native-module, device-API, and mixed-architecture profiles. This milestone establishes profile-level support evidence; it does not claim universal production coverage.

### Work items

1. Add the Android technology capability registry and project-plan schema.
2. Add Java, Android SDK, emulator, device, and package-manager diagnostics.
3. Add device-manager abstraction and connection state.
4. Add Android logs, install, reload, and build status.
5. Add APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` build profiles where the local environment supports them.
6. Add signing configuration with secrets stored outside project source.

### Exit gate

A supported mobile fixture can be generated, launched on one emulator or device, tested with synthetic data, and built into a local artifact with clear environment diagnostics.

---

## 15. M12: Advanced Extensibility

### Objectives

Add reusable skills, hooks, external tools, model routing, scheduled local tasks, and deeper project memory.

### Work items

1. Implement project-local and user-level skill packages.
2. Add skill discovery, compatibility metadata, and permission control.
3. Add pre-action and post-action hooks.
4. Add external-tool adapters with isolated permissions.
5. Add model fallback and task-to-model routing.
6. Add bounded long-term project memory.
7. Add scheduled safe tasks and notification policies.
8. Add advanced native project profiles.

### Exit gate

A project can install a skill, use it only when relevant, enforce its permissions, record its actions, and remove it without modifying the core runtime.

---

## 16. Test Strategy

### 16.1 Unit tests

Unit tests should cover state transitions, policy evaluation, path validation, command classification, message validation, provider normalization, resource accounting, repository-map ranking, and checksum generation.

### 16.2 Integration tests

Integration tests should run the control plane, database, scheduler, worker mock, event bus, and process manager together. They should simulate approval, timeout, cancellation, restart, duplicate events, stale workers, and recovery.

### 16.3 Fixture-task evaluations

The team should maintain fixture projects that represent Android dashboards, authenticated utilities, offline-first apps, forms, API integrations, notification flows, device-permission workflows, and intentionally broken Android projects. Every release should run a fixed set of prompts and score changed-file scope, successful Android build, emulator/device behavior, test status, visual behavior, recovery, and safety policy compliance.

### 16.4 Security tests

Security tests should verify that protected files cannot enter model context, denied commands cannot run, external directories require approval, personal credentials and unapproved device data are never used, untrusted packages are restricted, and secrets do not appear in logs or artifacts.

### 16.5 Recovery tests

Recovery tests should forcibly close the UI, terminate the control plane, kill a worker, fill the disk quota, cross an adaptive time or usage threshold, interrupt a database transaction, create a merge conflict, and disconnect the provider. Crossing an ordinary threshold must verify that the task adapts or continues rather than ending automatically. Every scenario should end in a clear resumable, escalated, or safely rolled-back state.

### 16.6 Platform capability and cross-build tests

Platform capability tests should run on at least two host platforms with at least one non-target host→target pair (for example, Linux host → Windows target). They must prove the four-state invariant (host ≠ target ≠ validation ≠ certification), that a successful cross-build records `ARTIFACT_BUILD = VERIFIED` with the runtime state `UNVERIFIED`, that a model or worker completion claim without matching-platform observation is durably rejected, that a revision, toolchain, or fingerprint change invalidates prior target-platform evidence, and that an absent validation environment produces a durable `USER_REQUIRED`/`UNAVAILABLE` node with the continue/cannot-continue lists while independent work proceeds. The fixture family is `TEST-PLAT-001` with evidence `EV-PLAT-001` (M118; build spec §79.13).

---

## 17. Release Gates

A milestone can be released only when its functional acceptance criteria, security checks, recovery tests, documentation, and migration behavior pass. “The agent usually works” is not a release gate.

Each release should publish a short engineering report containing completed milestones, known limitations, fixture-task results, failed tests, changes to permissions, schema migration notes, and environment compatibility.

---

## 18. Recommended Team Sequence

The recommended sequence is to build the control plane and state model first, then the provider runtime, then dynamic Android project synthesis and one reliable end-to-end agent loop. Security and recovery must be implemented before background autonomy. Multi-worker coordination must be implemented only after single-worker recovery is dependable.

This sequence reduces the risk of building a visually impressive chat interface around an unreliable execution engine.

---

## 19. Immediate Next Tasks

1. Create the source repository using the module boundaries in the technical architecture.
2. Define the SQLite schema and migration strategy.
3. Define the IPC API and event envelope.
4. Implement the control-plane health endpoint and task lifecycle.
5. Build the desktop shell around a mock task stream.
6. Implement provider profiles and secure credential references.
7. Add the first screenshot-and-instruction-driven Android fixture generation task.
8. Implement a single safe file-edit task end to end.
9. Add checkpoint and undo behavior.
10. Add process quotas and restricted execution before enabling autonomous mode.

---

## 20. Extension Milestones from the Advanced Autonomy Requirements

### M13: Goal Mode and non-blocking background work

Implement durable goal contracts, completion-condition evaluation, resource budgets, stop conditions, progress tracking, reconnectable task streams, background UI behavior, and operating-system notifications. The task must continue without stealing user focus and must survive a controlled UI restart.

**Exit gate:** A user can define a goal once, continue working elsewhere, close and reopen the desktop interface, and inspect objective completion results rather than relying on a final model message.

### M14: Lifecycle hooks

Implement the named hook-event table defined in the master specification. Add blocking and non-blocking hook types, timeouts, deduplication, failure policies, policy enforcement, and audit records.

**Exit gate:** A pre-tool security hook can block an unsafe action, a post-tool hook can update the project index, and a worker-failure hook can start a recovery action without bypassing permissions.

### M15: Scheduled automations

Implement local recurring task definitions with interval, calendar, project-change, failed-validation, and manual triggers. Add schedule persistence, duplicate-run prevention, budgets, inherited permissions, pause/disable controls, run history, and notifications.

**Exit gate:** A safe local test or documentation task can run on a schedule, recover correctly after a control-plane restart, and never publish or use personal credentials without per-run approval.

### M16: Granular checkpoints and backtracking

Implement file-level checkpoints alongside task-level checkpoints. Add last-known-good restoration, strategy history, failure fingerprinting, materially different recovery plans, and preview invalidation after rollback.

**Exit gate:** A repeated failing implementation is restored to a known-good state, retried using a different worker or approach, and reported with a complete strategy history.

### M17: Context scaling and external-tool compatibility

Implement retrieval-based and large-context modes, context-package reports, secret filtering, token-budget fallback, external-tool capability discovery, scoped connections, health checks, and policy mediation.

**Exit gate:** A small-context provider uses repository retrieval, a large-context provider can use a filtered near-full repository, and an external tool cannot bypass Nirman path, network, or approval rules.

## 21. Revised Fixture and Recovery Evaluation Matrix

| Evaluation | Required behavior |
|---|---|
| Goal completion | Task ends only when objective conditions pass or a defined stop condition is reached |
| UI disconnect | Background task continues and event sequence replays after reconnect |
| Hook enforcement | Blocking safety hook prevents the action and records the reason |
| Scheduled run | A recurring task runs once per trigger and survives daemon restart |
| File checkpoint | One file restores without changing unrelated files |
| Backtracking | Failed strategy returns to a known-good state before trying a different strategy |
| Context scaling | Mode selection is visible and falls back safely when budget is insufficient |
| External tool | Tool is scoped, audited, and cannot bypass the policy engine |
| Subagent isolation | Parallel workers cannot mutate the main workspace before reconciliation |

## 22. Execution-Surface Milestones

### M18: Durable task graph and nested execution tree

Add a persisted task graph that represents the goal, extracted requirements, phases, dependencies, worker handoffs, commands, previews, tests, builds, approvals, checkpoints, recovery attempts, and final evidence. Build an expandable execution tree in the task view with node states, timestamps, owners, workspaces, heartbeats, warnings, and evidence links.

**Exit gate:** A task can be inspected as a nested tree while running, after completion, and after a control-plane restart. Child events replay in order and no completed node lacks evidence.

### M19: Evidence-backed status and telemetry

Add an evidence ledger for command results, test reports, build artifacts, screenshots, device results, security scans, review findings, approvals, and environment diagnostics. Add runtime telemetry for elapsed time, turns, provider requests, token/resource usage, active workers, last checkpoint, current blocker, and next action.

**Exit gate:** A model summary alone cannot mark a task or phase complete. The final result links each completion claim to captured evidence and exposes the task’s resource and recovery history.

### M20: Autonomous validation coordinator

Implement the default validation loop: emulator/device preview or launch, focused checks, Android build or package, security/dependency/reliability checks, device/accessibility/visual QA, failure classification, repair or backtracking, regression validation, and completion evaluation. Project profiles may mark stages as required, optional, or unavailable.

**Exit gate:** A required but unavailable validation stage blocks completion, while optional stages are clearly labeled as skipped or unavailable. A regression after repair triggers backtracking or escalation.

### M21: Policy-boundary approvals and termination coordinator

Refine approvals so routine reversible actions in an approved workspace do not interrupt the user, while protected-file access, risky dependencies, external services, credentials, destructive actions, publishing, and signing create precise approval requests. Implement terminal classifications for completed, completed with warnings, blocked, escalated, cancelled, and failed.

**Exit gate:** Safe work runs without approval spam, privileged work pauses at the exact boundary, ordinary usage thresholds trigger adaptation rather than termination, and tasks stop only for a defined completion, decision, explicit hard safety or policy limit, cancellation, environment failure, or unrecoverable error.

## 23. Execution-Surface Evaluation Matrix

| Evaluation | Required result |
|---|---|
| Task launcher | Chat starts a durable background task without owning its execution loop |
| Plan visibility | User can see phases, dependencies, progress, checkpoints, and completion state |
| Nested activity | Commands, tests, builds, worker handoffs, approvals, and repairs appear as expandable child nodes |
| Evidence status | Completed claims link to execution or review evidence |
| Worker observability | Active action, heartbeat, elapsed time, workspace, and resource usage are visible |
| Validation loop | Required preview, tests, build, security, reliability, and visual/device checks run or block completion |
| Policy boundaries | Routine actions are not approval-blocked; privileged actions create precise approval requests |
| Reconnection | UI close or disconnect does not lose task state or event history |
| Termination | Task stops only at a defined completion, decision, limit, cancellation, environment failure, or unrecoverable failure |
| Final result | Changed files, checkpoints, evidence, tests, warnings, blockers, usage, and completion classification are available |

## 24. Provider Runtime and Self-Development Milestones

### M22: Provider-neutral AI settings and model gateway

Implement provider profiles with custom base URLs, API-key references, model IDs, protocol selection, capability probes, optional vision/embedding models, privacy policies, network policies, health status, and normalized reasoning capability profiles.

The ModelGateway must normalize Chat Completions, Responses-style, message-oriented, and compatible local-provider requests. It must support structured output, multimodal input, tool calls, streaming, cancellation, usage accounting, request IDs, context-capacity detection, reasoning-effort configuration, provider-native reasoning capability detection, reasoning-token accounting, and deterministic mapping between Nirman's reasoning levels and provider-specific parameters.

Provider capability detection must distinguish native reasoning support, supported effort levels, maximum reasoning-token capacity when known, reasoning-usage reporting, and continuation support.

**Exit gate:** The user can configure a provider manually, test the selected model, detect text/vision/tool/structured-output/streaming/cancellation/context/reasoning capabilities, verify the supported reasoning-effort levels, run a multi-turn request, execute a tool call, stream or emulate events, cancel a request, and inspect normalized usage and request IDs without exposing the key.

### M23: Controlled self-development loop

Implement the stable launcher/controller, isolated self-development worktree, source checkpoint, self-development contract, candidate build, temporary profile, health checks, smoke task, task replay, compatibility checks, atomic promotion, and automatic rollback. The current running application must remain unchanged until the candidate passes the required validation policy.

**Exit gate:** Nirman can modify its own source in isolation, build a candidate, launch it separately, run static/unit/integration/provider/sandbox/recovery/smoke checks, promote it through the controller, and roll back after an injected startup, migration, IPC, or health-check failure.

### M24: Adaptive long-horizon provider execution

Implement continuation across provider request boundaries without a default time or token completion lock. Add context compaction, retrieval fallback, model routing, concurrency reduction, provider retry classification, context-overflow recovery, reasoning-effort routing, reasoning-budget reservation and settlement, provider-native reasoning normalization, provider capability gaps, and task-state persistence.

Native provider reasoning and runtime deliberation must remain separate resources. The runtime must be able to combine higher provider-native reasoning effort with multiple bounded deliberation passes while preserving the total deliberation budget, evidence requirements, and authority boundaries.

User-configured hard caps remain available but are opt-in.

**Exit gate:** A long-running fixture task can continue through multiple provider requests and context compactions, adapt its model or worker strategy, recover from a transient provider failure, and complete without being stopped by an ordinary usage threshold.

## 25. Provider and Self-Development Evaluation Matrix

| Evaluation | Required result |
|---|---|
| Custom provider | Base URL, key reference, protocol, and model ID can be configured manually |
| Capability detection | Text, vision, tools, structured output, streaming, cancellation, and context behavior are tested or explicitly overridden |
| Protocol normalization | Chat, response-item, and message-oriented requests reach the same internal gateway |
| Tool continuity | Tool-call IDs and tool results remain correctly associated across turns |
| Streaming | Partial events are durable and reconnectable; non-streaming providers still produce lifecycle events |
| Reasoning capability | Provider reports whether native reasoning is supported and which normalized effort levels it can satisfy |
| Reasoning normalization | NORMAL/EXTENDED/DEEP/EXHAUSTIVE requests map deterministically to provider-specific parameters |
| Reasoning accounting | Reported reasoning usage is distinguished from estimated or unavailable usage |
| Reasoning budget | Concurrent provider requests cannot consume the same remaining deliberation budget |
| Reasoning capability gap | A provider unable to satisfy the required minimum effort produces a typed capability gap or approved failover |
| Self-update isolation | Current installation is unchanged until candidate validation succeeds |
| Candidate health | Candidate launches in a temporary profile and passes IPC, database, provider, preview, and smoke checks |
| Migration safety | Failed migration leaves the previous version and recoverable database available |
| Rollback | Injected candidate failure atomically restores the previous version and task state |
| Long horizon | Ordinary token/time usage thresholds adapt execution rather than terminating the goal |

## 26. Complete Runtime and Self-Improvement Milestones

### M25: Runtime supervisor and durable execution loop

Implement the stable supervisor, control-plane ownership, idempotent runtime ticks, task-graph scheduling, worker leases, launch intents, heartbeats, reconnectable events, and restart recovery. The runtime must continue after each model response and provider request rather than treating a response as the end of the task.

**Exit gate:** A broad goal can run through multiple provider requests, worker handoffs, validation cycles, and application restarts while preserving the task graph, checkpoints, evidence, and next action.

### M26: Graduated recovery ladder

Implement transient retry, focused diagnostics, context/index refresh, strategy change, checkpoint backtracking, model or worker escalation, specialist delegation, isolated alternative solutions, and precise escalation. Add failure fingerprints, progress-quality measurement, and duplicate-strategy detection.

**Exit gate:** A fixture task with repeated compiler, runtime, environment, provider, and merge failures automatically changes strategy, preserves the last known-good state, and stops only when no safe recovery path remains.

### M27: Self-observation and episode evaluation

Implement episode records, validated task summaries, project-scoped memory, runtime quality metrics, fixture evaluation runs, trajectory replay, and regression comparison by runtime version, provider profile, model profile, project type, and worker role.

**Exit gate:** The system can explain why a task succeeded or failed, compare two runtime candidates on the same fixture suite, and identify whether the main weakness was requirements, context, planning, tool use, editing, environment, or validation.

### M28: Self-improvement proposal manager

Implement recurring-failure clustering, improvement hypotheses, proposal records, affected-component analysis, expected-metric definitions, safety impact, test plans, rollback plans, and scoped promotion policies. Restrict high-risk components such as the supervisor, sandbox, policy engine, credentials, updater, migrations, and evidence engine to the highest validation level.

**Exit gate:** Nirman can create a proposal from repeated validated failures, generate an isolated candidate, and show the evidence and expected improvement before changing runtime behavior.

### M29: Candidate canary, promotion, and rollback

Implement observe-only, candidate-only, canary, trusted auto-promotion, and manual-promotion modes. Run targeted tests, broad regression fixtures, provider tests, sandbox tests, migration tests, recovery tests, smoke tasks, and representative task replay before promotion. Monitor post-promotion quality and automatically roll back or disable a degraded scope.

**Exit gate:** An injected candidate failure, migration error, IPC failure, crash loop, regression, or safety degradation restores the previous version and preserves user projects and task state.

## 27. Complete Runtime and Self-Improvement Evaluation Matrix

| Evaluation | Required result |
|---|---|
| Runtime continuity | Provider responses are intermediate steps, not task termination |
| Supervisor recovery | UI, worker, or control-plane restart preserves task state |
| Lease correctness | A crashed worker cannot permanently claim a task |
| Strategy diversity | Repeated failures cause materially different recovery attempts |
| Progress quality | The runtime detects when requests are not producing verified progress |
| Episode analysis | Completed and failed tasks produce structured, privacy-filtered records |
| Candidate quality | Improvements are judged against fixed fixtures and baseline metrics |
| Scoped promotion | A candidate can be limited to a project, provider, worker role, or task class |
| Rollback | Post-promotion regressions automatically restore the known-good runtime |
| Memory safety | Long-term memory contains validated, privacy-filtered records only |

## 28. Core Autonomous Runtime Acceptance Criteria

The following acceptance criteria are mandatory for the core autonomous runtime. They should be evaluated independently and as part of full end-to-end fixture tasks.

| Capability | Acceptance criterion |
|---|---|
| **Specialized workers** | A representative task can assign architecture, implementation, debugging, testing, security, visual QA, performance, and release work to separate scoped workers, and each worker returns a durable handoff with evidence. |
| **Self-healing loop** | Injected compiler, runtime, test, environment, provider, and merge failures cause classification, a materially different strategy, checkpoint backtracking where needed, continued validation, and no repeated identical loop. |
| **Evidence-based completion** | A task cannot be marked complete from model text alone; completion links to passing tests, builds, screenshots, health checks, security results, device results, review findings, or validated artifacts. |
| **Adaptive resource management** | A long-running fixture task crosses ordinary time, token, or usage thresholds and continues by compacting context, changing models, reducing concurrency, retrying transient failures, or repairing the environment rather than stopping automatically. |
| **Self-development mode** | Nirman changes its own source only in an isolated worktree, builds and launches a candidate separately, runs health and smoke checks, promotes through the stable controller, and rolls back after an injected failure. |
| **Project memory** | A later task can recall a validated architecture decision, previous fix, failed strategy, convention, and user preference while excluding credentials and protected content. |
| **Environment repair** | A fixture with missing or incompatible SDKs, dependencies, ports, emulators, or toolchains is diagnosed and repaired or clearly escalated according to policy, with the repair recorded as evidence. |

A release cannot claim complete autonomous-runtime support until all seven criteria pass in isolated tests and in at least one combined end-to-end fixture task.

## 29. Audit Closure Milestones

### M30: Canonical documentation and worker registry

Renumber the advanced product-specification sections, remove duplicate roadmap references, create one crosswalk between roadmap phases and milestones, and make the canonical worker registry the only role taxonomy used by the product, architecture, tests, and decision records. The registry must include the Performance Worker and define every worker’s scope, tools, workspace, and mutation authority.

M30 MUST establish the canonical semantic identity graph before any further specification extension is accepted. Section numbers are addresses, not identities. Every ContractId, CapabilityId, ADR, milestone, invariant, test, evidence ID, worker role, and schema must have one canonical identity. Cross-document references MUST resolve through canonical identity, not positional text substitution.

**Exit gate:** All cross-document references resolve to one section or milestone, all worker names match exactly, the Performance Worker has a contract, a registry test rejects undefined or duplicate roles, and the canonical identity verifier (INVARIANT.DOCUMENTATION.CANONICAL_IDENTITY) passes with 0 defects. A release MUST fail if two objects claim one canonical identity, one reference resolves to the wrong semantic object, a reverse edge does not return to its source, or a reference resolves only because of a stale section number.

### M31: Unattended / Full Autonomy policy profile

Implement a named project-scoped profile for Goal Mode background tasks. It allows routine reversible actions inside the workspace, including dependency installation, local commits, formatting, testing, builds, preview restarts, and approved environment repair. It denies external-directory access, raw credentials, destructive commands, operating-system changes, remote pushes, publishing, signing, and unapproved sensitive-data transmission.

**Exit gate:** A background fixture task completes a dependency install, local commit, build, preview restart, and repair without approval pauses, while deployment, signing, credential access, destructive commands, and remote pushes remain hard-gated.

### M32: Persistent terminal subsystem

Implement per-worker PTY or equivalent terminal sessions with persistent working directory and environment state, explicit Windows shell profiles, controlled stdin, interactive-prompt detection, unattended prompt policy, long-running process registration, multi-terminal UI, rolling log storage, rotation, compression, and raw evidence retention.

**Exit gate:** An unattended fixture can activate an environment, install dependencies, start a dev server, respond to a declared safe prompt, detect an unsafe prompt, preserve the terminal session after UI disconnect, and reconnect with searchable logs.

### M33: Skills registry and invocation contract

Implement the SkillPackage schema, trigger and explicit invocation, worker compatibility, required tools, permission requests, input/output schemas, scanning, trust status, versioning, health checks, update, disable, and rollback. Loading a skill must never grant permissions automatically.

**Exit gate:** A safe skill can be discovered, scanned, invoked by a matching task, execute only declared tools through the policy engine, and roll back after a failed update. An unsafe or undeclared skill action must be rejected.

### M34: Windows lifecycle and multi-project resilience

Implement active-task login startup, boot/resume/suspend/hibernate event handling, execution power requests, process and emulator restoration, notification fallback, startup summaries, weighted fair-share scheduling, priority aging, and cross-project resource accounting.

**Exit gate:** After an injected reboot, suspend/resume, suppressed notification, and competing multi-project workload, eligible tasks resume from checkpoints, hard decisions remain visible, and no project starves another.

### M35: Long-horizon scale and unified execution surface

Implement incremental repository-map shards, dependency fingerprints, checkpoint retention and content-addressed compaction, Android-profile disk quotas, affected-test computation, cached results, regression sharding, architectural-drift checks, and the side-by-side Android preview plus execution-surface layout.

**Exit gate:** A large Android fixture updates only affected map shards, retains a valid restore path while pruning safe intermediates, runs affected tests before sharded regressions, detects architectural drift, and shows the correct emulator/device preview revision beside the live task tree.

## 30. Audit Closure Evaluation Matrix

| Audit area | Required evidence |
|---|---|
| Documentation consistency | Renumbered sections, one roadmap crosswalk, no duplicate role taxonomy |
| Unattended autonomy | Routine project-local actions complete without approval pauses |
| Terminal reliability | Persistent session, prompt handling, shell selection, multi-terminal logs |
| Background resilience | Reboot, sleep/resume, notification fallback, fair scheduling |
| Skills | Schema, scan, invocation, permissions, versioning, rollback |
| Long-horizon coding | Incremental map, affected tests, retention, drift checks |
| Swarm coordination | Decomposition heuristic, interface agreement, bounded nesting, reconciliation |
| Preview visibility | Preview revision and nested execution tree visible together |

### M36: Runtime authority and autonomous recovery invariants

Implement and test the rule that models propose work but deterministic lifecycle, permission, sandbox, storage, evidence, recovery, promotion, and termination authorities control execution. The runtime must recover, retry, checkpoint, repair, reconcile, degrade, or fail safely without trusting model claims or uncommitted model memory.

**Exit gate:** Fault-injection tests prove that model output cannot grant permissions, bypass sandbox rules, mark tasks complete without evidence, delete recovery state, promote an unvalidated candidate, disable mandatory hooks, or suppress a hard safety termination. Recovery tests prove that the last known-good state remains restorable across worker, process, provider, UI, database, and self-update failures.

### M37: Android-only target contract

Make Android project profiles, emulator/device validation, Logcat, Gradle, APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` artifacts, permissions, notifications, offline behavior, and device-specific acceptance tests the only generated-project requirements. Keep the desktop shell solely as the local development host.

**Exit gate:** A scope test accepts supported Android project requests, resolves the correct Android profile, launches emulator/device validation, produces Android artifact evidence, and confirms that every project-generation path resolves only to an Android profile.

### M38: Complete Android technology coverage

Implement the capability registry, technology planner, framework resolver, mixed-architecture project synthesis, native-module integration, device-capability resolution, and end-to-end validation for the full Android technology surface. The user must describe the application rather than select a framework or template.

**Exit gate:** A capability fixture suite covering JavaScript, Java, Kotlin, Views, Compose, native modules, background services, device APIs, offline behavior, notifications, media, location, sensors, and mixed projects can be generated from instructions and optional screenshots, built, installed or launched, tested, visually validated, repaired, and packaged as APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` artifacts.

## 31. Definition of Done for Nirman v1

Nirman v1 is complete when a Windows user can create or open a supported local project, configure an AI provider, ask for a feature, review and approve the plan, observe structured file changes, run a local preview, execute validation, inspect evidence, undo the task, and recover the task after a controlled application restart. The output must remain a normal user-owned project that can be opened and built outside Nirman.

The product is not considered autonomous-ready unless the runtime, rather than the model, remains the authority over lifecycle, permissions, sandboxing, storage, evidence, recovery, promotion, rollback, and termination.


## 32. End-to-End Autonomous Android Session

Implement one durable session that owns the complete path from a chat instruction and optional screenshots to a validated Android artifact. The session must persist the application contract, visual specification, technology plan, task graph, workers, terminals, sandbox, preview revision, checkpoints, validation, recovery, and artifact state independently of the chat interface.

### Acceptance criteria

1. One instruction plus optional screenshots creates one resumable autonomous Android session.
2. The session continues while the chat view is closed or the user opens another task.
3. The session selects Android technologies without requiring a framework or template choice.
4. The session creates an isolated project workspace and starts the required terminals and device runtime.
5. The session updates the Android preview after validated changes.
6. Every preview state is linked to a project revision and checkpoint.
7. The session records worker handoffs, commands, tests, screenshots, recovery attempts, and evidence.
8. The session produces an installable APK, or an AAB only when the active PackagingProfile requires `APK_AND_AAB`, when the project contract requires packaging.

## 33. Progress Ledger and Stall Recovery

Implement a progress ledger and stall detector that measure changed files, new evidence, preview movement, test transitions, worker handoffs, strategy changes, validated requirements, and artifact transitions.

### Acceptance criteria

1. Repeated commands, repeated patches, repeated failures, unchanged workspaces, missing evidence, stale emulators, and unresponsive processes are detected.
2. A detected stall causes a meaningful strategy change, context refresh, environment repair, delegation, checkpoint restore, technology change, or isolated alternative.
3. The same failed action is not repeated indefinitely.
4. Fault-injection tests prove that the runtime can recover from worker, process, provider, emulator, device, Gradle, and preview failures.
5. The task either continues toward completion or reports a precise evidence-backed blocker.

## 34. Live Preview and APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` Completion Gate

Make the Android emulator or connected device a first-class validation surface and require the preview revision to remain synchronized with the execution tree.

### Acceptance criteria

1. The preview displays the active device, project revision, checkpoint, installation state, reload state, Logcat, runtime errors, screenshots, and visual comparison results.
2. A broken candidate never replaces the last valid preview revision.
3. The final completion gate verifies build success, APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` existence, checksum, artifact scan, installation or launch, main-flow execution, visual validation, permissions, and fatal runtime errors.
4. The final report links the artifact to the source revision and evidence ledger.

## 35. Full Android Capability Fixture Coverage

Maintain generated-from-instruction fixtures for JavaScript-driven Android, Java, Kotlin, Android Views, Jetpack Compose, mixed architectures, custom native modules, background services, WorkManager, notifications, camera and media, location and sensors, Bluetooth and NFC, offline-first storage, API-heavy applications, authentication and permissions, tablet and multi-orientation layouts, device-integrated applications, and APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` delivery.

These fixtures are internal acceptance categories, not user-facing templates. The user must be able to describe the application without selecting a framework.

### Acceptance criteria

Every capability fixture can be generated from an instruction and optional screenshots, built, installed or launched, tested, visually validated, repaired, reconciled, and packaged as an installable APK, or an AAB only when the active PackagingProfile requires `APK_AND_AAB`, where applicable. The technology plan must explain the selected stack and the evidence ledger must prove the result.

## 36. No-Routine-Intervention Gate

The Unattended / Full Autonomy profile must allow routine project-local actions to continue without approval pauses while preserving deterministic authority boundaries.

### Acceptance criteria

Editing, dependency installation, terminal execution, emulator launch, testing, screenshots, repair, checkpoints, worker handoffs, reconciliation, and local artifact creation proceed automatically under the configured policy. Credentials, destructive operations, publishing, signing, protected paths, hard safety violations, and unrecoverable blockers remain gated or terminate safely.


## 37. Production Runtime Contracts and Lifecycle Authority

Implement versioned contracts for sessions, application requirements, visual specifications, technology plans, task graphs, workers, terminal sessions, preview revisions, evidence, recovery, artifacts, and provider profiles. Implement the authoritative session lifecycle and safe terminal states.

### Acceptance criteria

1. Every durable runtime object validates against a versioned schema.
2. State transitions are deterministic, persisted, replayable, and rejected when invalid.
3. Model output, worker messages, skills, hooks, and UI events cannot commit lifecycle transitions directly.
4. Corrupt state triggers migration, backup restoration, or safe execution disablement without destroying projects.

## 38. Renewable Leases and Operation Capabilities

Implement renewable session leases for long-running Android work and single-use operation capabilities for sensitive operations. Capabilities must bind session, worker, workspace, project revision, scope, action type, policy, and expiry.

### Acceptance criteria

Session leases renew only with valid heartbeats and progress or a classified external wait. Operation capabilities reject reuse, expiry, policy mismatch, scope mismatch, revision mismatch, and unauthorized action types. Models cannot mint, extend, or broaden capabilities.

## 39. Android Project Ingestion and Integrity

Implement Android-aware project discovery for Gradle files, manifests, resources, assets, localization, native modules, generated outputs, device configuration, secrets, keystores, local properties, and repository state. Add canonical paths, exclusions, project fingerprints, scope fingerprints, dependency graphs, and TOCTOU checks.

### Acceptance criteria

External changes between planning, editing, reconciliation, preview installation, packaging, and promotion are detected. Stale revisions are rejected and re-ingested. Secrets, signing material, generated outputs, and unrelated personal data are excluded from model context by default.

## 40. Provider Gateway and Controlled Tool Protocol

Implement a provider gateway for Chat Completions, Responses-style requests, messages, screenshots, structured outputs, typed tool calls, tool results, streaming task events, cancellation, usage, context limits, and normalized provider errors. Add role-based provider profiles for planning, coding, vision, debugging, testing, and review.

### Acceptance criteria

Every tool call is schema-validated, permission-checked, session-bound, worker-bound, sandbox-bound, privacy-classified, and recorded with evidence. Unknown tools, arguments, provider routes, and secret-access attempts are rejected. Provider failures are classified and routed through the recovery policy.

## 41. Sandbox and Process Separation

Implement separate authority and process domains for the desktop shell, control-plane supervisor, workers, Android build processes, emulator/device manager, preview application, provider transport, and credential service.

### Acceptance criteria

Generated code cannot access personal files, browser data, SSH keys, unrelated projects, signing keys, or arbitrary credentials. A worker cannot escape its workspace or broaden its network, process, device, or filesystem permissions. Fault injection proves policy enforcement at every boundary.

## 42. Evidence, Memory, Replay, and Task History

Implement separate event, evidence, memory, and replay stores. Add session/project/runtime memory boundaries, source and confidence metadata, retention and deletion, task reopening, validation reruns, strategy forks, provider comparisons, checkpoint restore, preview revision comparison, and artifact/evidence downloads.

### Acceptance criteria

A model claim cannot mark a requirement complete without evidence. Users can inspect, correct, export, delete, reopen, replay, fork, restore, and revalidate tasks without exposing credentials or unclassified private data to models.

## 43. Production Windows Host Reliability

Implement offline startup, atomic state writes, file locks, migrations, crash recovery, signed per-user installers, upgrade rollback, state preservation, virtualized large-project views, local editor assets, privacy-filtered logs, and memory-leak tests.

### Acceptance criteria

The host opens projects, settings, history, and checkpoints without a provider. Provider unavailability disables execution without crashing startup. Reboot, sleep/resume, upgrade failure, corrupted state, and process crash preserve recoverable task and project state.

## 44. Device Matrix and Productivity Surface

Implement device-matrix testing across phone, tablet, Android API levels, densities, orientations, and connected devices. Add one-click goal launch, technology rationale, changed-files timeline, build-health view, artifact center, recovery explanation, project-memory editor, privacy/network panel, and environment-repair center.

### Acceptance criteria

The user can start one Android goal, observe the live preview beside the execution tree, inspect technology selection and recovery, compare device results, restore a checkpoint, and download a validated APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` with evidence.

## 45. Integrated Production Readiness Gate

Nirman is not production-ready until a complete Android fixture passes the following path without routine approval pauses: one instruction plus screenshots → contract extraction → technology selection → environment preparation → synthesis → worker/tool execution → emulator preview → failure injection and recovery → device validation → APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` packaging → evidence report → task replay and checkpoint restore.


---

### Milestone refinement rule

M39–M117 refine and operationalize the earlier M0–M38 roadmap. They do not create a second execution order or supersede earlier milestone ownership. When a coarse milestone and refined milestone overlap, the refined milestone defines the implementation gate while the coarse milestone remains the product-level grouping.

# M39–M50: Integrated Android Construction Runtime Milestones

These milestones extend the existing Nirman roadmap with the accepted construction, orchestration, toolchain, code-intelligence, repair, preview, UX, and resource-governance requirements. They preserve Android-only generated output and the existing autonomous session contract.

## M39 — AndroidConstructionContract and schema authority

Implement the versioned AndroidConstructionContract, including intent, screenshots, features, UI, data, integrations, technology plan, Android requirements, device matrix, validation model, and artifact model. Add strict schema validation, migrations, source references for inferences, and explicit distinction between user facts and model proposals.

**Exit gate:** every new session produces a valid contract; malformed or unknown fields are rejected; all downstream workers consume the same contract; contract versions can be migrated and replayed.

## M40 — Pure session reducer and event replay

Implement the side-effect-free session reducer, append-only event store, monotonic event sequences, transition validation, and crash reconstruction. Add impossible-transition tests and replay tests.

**Exit gate:** forced supervisor termination followed by restart reconstructs the same session state from durable events and checkpoints.

## M41 — ConstructionTransaction and commit barrier

Implement pre-mutation checkpointing, project revisions, transaction workspaces, mutation budgets, conflict detection, serialized per-revision commits, semantic reconciliation, rollback, and evidence-linked promotion.

**Exit gate:** stale and conflicting worker proposals are rejected or reconciled without corrupting the project; failed transactions roll back atomically.

## M42 — Renewable leases and operation capabilities

Implement renewable SessionLease records, progress-aware heartbeat renewal, worker lease revocation, single-use operation capabilities, scope fingerprints, base-revision binding, and consumption before external side effects.

**Exit gate:** expired leases cannot mutate; capability reuse is rejected; provider/device/signing operations require valid scoped capabilities; restart resumes only from durable boundaries.

## M43 — AndroidToolchainManifest and clean-machine authority

Implement Android toolchain discovery, lock generation, version/hash/license validation, isolated environment construction, environment snapshots, and authorized toolchain repair for JDK, Gradle, AGP, Kotlin, SDK, build tools, platform tools, NDK, CMake, ADB, emulator, Node/package manager, and selected React Native/Expo tooling.

**Exit gate:** a clean-machine fixture builds using only the locked Android toolchain; host PATH and unrelated SDK installations cannot change the result.

## M44 — Provider bridge protocol and supervision

Implement protocol handshake, loopback authentication, provider/model capability validation, health states, request normalization for Chat Completions/Responses/message protocols, streaming events, cancellation, restart, offline mode, and privacy-filtered logging.

**Exit gate:** bridge crash, protocol mismatch, authentication failure, provider outage, malformed output, rate limit, and unsupported capability all produce deterministic recoverable states without session corruption.

## M45 — Multi-language AndroidCodeIntelligence

Implement language adapters and graph indexing for Kotlin, Java, XML, manifests, Gradle Kotlin DSL/Groovy, TypeScript/JavaScript, C/C++ native modules, JSON/YAML/TOML, SQL, and lockfiles. Add file/module/symbol/resource/permission/navigation/test/device impact graphs.

**Exit gate:** lightweight discovery upgrades to full semantic mode before mutation; affected files and tests are computed for representative Android projects across selected technology plans.

## M46 — Structured mutation broker

Implement parser-aware and schema-aware mutations, path and revision validation, file ownership, dependency policy, mutation budgets, whole-file fallback restrictions, formatting, syntax validation, and content-integrity checks.

**Exit gate:** direct model writes and unsafe blind replacements are rejected; valid structured changes commit with evidence; invalid syntax or out-of-scope paths never reach the project.

## M47 — AndroidRequirementManifest and repair registry

Implement Android capability/requirement inference, missing and over-permission detection, manifest/resource validation, and deterministic repair patterns for toolchain, dependency, source/build, runtime, visual, accessibility, emulator, ADB, APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB`, and signing failures.

**Exit gate:** representative failure fixtures classify correctly, select an allowed repair, respect retry budgets, restore checkpoints when required, and produce validation evidence.

## M48 — Preview fallback matrix and revision binding

Implement incremental emulator install, Compose reload, React Native/Expo fast refresh, full APK reinstall, physical-device preview, headless smoke tests, diagnostic preview, stale-preview detection, screenshot capture, interaction evidence, and Logcat binding.

**Exit gate:** stale previews cannot satisfy completion; every promoted PreviewRevision identifies source revision, artifact, device, API level, mode, and evidence.

## M49 — Decision trace, progressive disclosure, and resource governor

Implement concise decision traces without hidden chain-of-thought, Calm/Inspect/Developer UI modes, environment and health dashboards, adaptive CPU/RAM/disk/emulator/worker/provider/context governance, safe cache pruning, and non-bypassable evidence gates.

**Exit gate:** the user can understand progress, waiting, recovery, blocking, and evidence without reading raw logs; resource pressure changes scheduling but never weakens security or completion criteria.

## M50 — End-to-end production acceptance

Run a clean-machine Android fixture matrix covering native Kotlin/Compose, Java/Views, React Native/Expo, native modules, offline data, permissions, screenshots, API integrations, emulator/device validation, dependency repair, provider outage, bridge restart, worker crash, reboot, sleep, disk pressure, stale revision, conflict reconciliation, APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` packaging, signing, and artifact export.

**Exit gate:** a single instruction plus optional screenshots can produce a validated Android APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` with source revision, checksum, environment snapshot, preview evidence, validation evidence, and replayable session history without routine human intervention.

## Integrated acceptance matrix

| Capability | Required proof |
|---|---|
| Construction contract | Validated versioned contract consumed by all workers |
| Reducer/replay | Identical reconstructed state after restart |
| Transactionality | Atomic commit/rollback and stale revision rejection |
| Leases/capabilities | Expiry, scope, nonce, and revocation tests |
| Toolchain authority | Clean-machine locked-toolchain build |
| Provider supervision | Health, handshake, outage, restart, and privacy tests |
| Code intelligence | Multi-language indexing and affected-test computation |
| Mutation broker | Structured-only mutation and scope enforcement |
| Android requirements | Permission/manifest/resource drift detection |
| Repair registry | Classified fixtures with evidence-backed fixes |
| Preview | Revision-bound emulator/device evidence |
| Resource governance | Pressure tests without gate weakening |
| Artifact completion | APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` checksum, signing, environment, and validation proof |
# M51–M58: Integrated Workflow and Quality Intelligence Milestones

These milestones add the accepted README-derived capabilities to the existing Nirman roadmap. They do not change the Android-only generated target.

## M51 — IntegratedAndroidWorkflowCoordinator

Implement the canonical coordinator connecting prompt normalization, AndroidConstructionContract, preflight, AndroidTechnologyPlan, task graph, worker allocation, ConstructionTransaction, build, preview, testing, quality review, recovery, packaging, and evidence promotion.

**Exit gate:** one session can traverse every boundary with durable events, idempotent command handling, and restart recovery from the last validated boundary.

## M52 — Preflight risk and feasibility engine

Implement `PreflightService`, `PreflightReport`, and `RiskAndFeasibilityEngine` for provider, toolchain, workspace, dependency, device, permissions, signing, storage, and validation-capacity checks.

**Exit gate:** known blockers are identified before expensive generation; routine local repairs are dispatched through policy; unavailable credentials, devices, and prohibited actions become explicit states.

## M53 — Independent Android quality gate

Implement independent correctness, architecture, build, security, dependency, runtime, UI, accessibility, performance, test, and release reviews. Add blocking, warning, and informational finding classes.

**Exit gate:** quality workers can block artifact promotion with evidence-backed findings, and a quality score alone cannot mark a session complete.

## M54 — Failure-mode prevention catalogue

Implement `FailureModeRegistry` with triggers, prevention checks, classification, permitted scope, recovery strategies, retry budgets, stop conditions, and evidence requirements for Android toolchain, dependency, source, runtime, device, visual, accessibility, packaging, and signing failures.

**Exit gate:** representative fault fixtures classify consistently and select deterministic recovery or safe states.

## M55 — Acceptance-test traceability

Implement `TestTraceabilityService` mapping every mandatory contract requirement to acceptance criteria, tests, devices, results, evidence, and artifact revisions. Support honest skipped, blocked, flaky, and not-applicable states.

**Exit gate:** no mandatory requirement can be reported complete without an executable validation path or an explicit governed exception.

## M56 — Architecture and contract drift detection

Implement `ArchitectureDriftDetector` and `ContractDriftDetector` for missing features, unreachable screens, undocumented permissions, missing migrations, untested criteria, unauthorized dependencies, stale generated files, architecture violations, and revision mismatch.

**Exit gate:** drift cannot be silently hidden by editing the approved contract in place; changes require versioned reconciliation and revalidation.

## M57 — Project handbook, release intelligence, and runtime analysis

Implement generated project handbooks, release-intelligence reports, Logcat/ANR/native crash analysis, dependency health checks, vulnerability/license/provenance checks, and worker-quality metrics.

**Exit gate:** every managed project has a concise validated handbook; every promoted artifact has a release report; runtime findings and dependency changes are evidence-linked.

## M58 — Validated repair promotion and final integration

Implement independent-fixture validation for learned repair patterns, bounded alternative strategy branches, and end-to-end regression testing across native, Compose, Java/Views, React Native/Expo, native modules, offline data, permissions, emulators, physical devices, provider failures, toolchain failures, and artifact gates.

**Exit gate:** capability support is reported from passing fixtures and retained evidence, not module counts or unsupported percentages.

## Integrated acceptance matrix

| Capability | Required proof |
|---|---|
| Workflow coordinator | Idempotent end-to-end session with restart recovery |
| Preflight | Blocker/risk report before expensive work |
| Quality gate | Independent findings and promotion blocking |
| Failure modes | Deterministic classification and recovery fixtures |
| Test traceability | Requirement-to-test-to-evidence matrix |
| Drift detection | Contract and architecture mismatch detection |
| Runtime analysis | Crash/ANR/Logcat fingerprinting linked to repair |
| Dependency health | Compatibility, security, license, and lock checks |
| Handbook/release report | Revision-bound generated documentation |
| Worker metrics | Routing metrics without authority escalation |
| Repair promotion | Independent validation before trusted reuse |
| Scope integrity | Android-only generated-target audit |
# M59–M61: Reasoning Visibility and Streaming Milestones

## M59 — PrivateReasoningRuntime and StructuredReasoningSummarizer

Implement the internal reasoning boundary for planning, self-critique, hypothesis generation, alternative comparison, diagnosis, and strategy selection. Add schema-validated summaries containing objectives, constraints, alternatives, selected action, uncertainty, confidence, expected validation, and next step.

**Exit gate:** private reasoning is never exposed verbatim, persisted as a transcript, sent in worker handoffs, used as evidence, or treated as a runtime command.

## M60 — ReasoningStreamEvent, filtering, and authenticated delivery

Implement allowed event types, deterministic redaction, session/task/worker/revision binding, monotonic sequencing, durable persistence, authenticated local streaming, acknowledgements, reconnect, back-pressure, and UI filters.

**Exit gate:** understanding, constraints, plan, alternatives, decisions, actions, observations, recovery, evidence, waiting, next-step, and completion events stream in order; secrets, source content, personal data, hidden instructions, and private reasoning are withheld.

## M61 — Reasoning replay and presentation modes

Implement Calm, Inspect, and Developer presentations, event replay without side effects, summary requests, pause-auto-scroll behavior, event filtering, evidence links, stale-state indicators, and fault-injection tests for disconnects, duplicate events, gaps, provider cancellation, and summarization failure.

**Exit gate:** the user can reconnect after UI/control-plane restart and recover the filtered stream; presentation changes do not alter execution or policy behavior.

## Integrated acceptance matrix

| Capability | Required proof |
|---|---|
| Private reasoning boundary | No verbatim private reasoning in UI, storage, logs, handoffs, or exports |
| Structured summary | Valid schema with constraints, decision, uncertainty, and next step |
| Filtering | Deterministic redaction and withholding of unsafe summaries |
| Streaming | Authenticated, ordered, durable, reconnectable delivery |
| Runtime separation | Visible decisions cannot authorize tools or mutations |
| Replay | Side-effect-free reconstruction from filtered events |
| Status truthfulness | Working, waiting, recovering, blocked, stale, complete, and safely failed are distinct |
| Back-pressure | UI disconnect cannot stop autonomous execution |
# M62–M64: Brand and Asset Completion Milestones

## M62 — BrandManifest, AssetManifest, and BrandAssetWorker

Implement versioned BrandManifest and AssetManifest schemas, brand-intent extraction, screenshot references, asset provenance, content hashes, regeneration history, and the scoped BrandAssetWorker.

**Exit gate:** a user request for a logo, icon, splash screen, notification icon, illustration, or visual identity creates explicit asset requirements and a traceable asset plan.

## M63 — Android asset integration and validation

Implement adaptive launcher icon, legacy icon, monochrome icon where applicable, splash, notification, in-app, theme, density, format, dimension, contrast, accessibility, resource-reference, and manifest integration validation.

**Exit gate:** requested assets resolve in the Android project, pass validators, appear in the live preview, and stale or invalid asset revisions are rejected.

## M64 — ArtifactAssetInspector and final completion gate

Implement built APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` extraction, asset presence and reachability checks, content-hash comparison, placeholder detection, preview binding, fallback records, and branding-change invalidation.

**Exit gate:** an APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` cannot be promoted when requested branding assets are missing, stale, unintegrated, invalid, or placeholder-only. A complete artifact includes asset evidence, provenance, preview verification, and release-report references.

## Integrated acceptance matrix

| Capability | Required proof |
|---|---|
| Brand intent | Versioned BrandManifest from user request and screenshots |
| Asset planning | AssetManifest with explicit requested types and statuses |
| Generation | Provider or approved local/vector fallback with provenance |
| Integration | Correct Android resources, references, and manifest entries |
| Preview | Current AssetManifest displayed in PreviewRevision |
| Artifact inspection | APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` contains requested assets and matching hashes |
| Accessibility | Contrast, transparency, silhouette, and theme checks |
| Change handling | Affected assets regenerate and stale evidence is invalidated |
| Completion gate | Missing or placeholder-only requested branding blocks promotion |
# 5A. Locked Implementation Stages

The detailed milestones below are executed through four architectural stages. Nirman must not attempt every autonomous capability simultaneously.

## Stage 1 — Foundation

Build Tauri 2, React, TypeScript, Vite, Tailwind CSS, shadcn/ui, Rust/Tokio, SQLite, Git, native Windows process controls, the provider gateway, project management, CodeMirror 6, xterm.js, Android toolchain detection, basic Android build and preview, checkpoints, and undo.

**Stage 1 exit gate:** a user can open or create an Android workspace, configure a provider, chat, inspect files, edit a file, run a supervised terminal, build or preview an Android project, create a checkpoint, and undo a change.

## Stage 2 — Reliable single-worker autonomy

Implement the authoritative session state machine, ToolBroker, PolicyAuthority, operation capabilities, ConstructionTransactionManager, EvidenceAuthority, RecoveryAuthority, process supervision, and one reliable autonomous worker.

**Stage 2 exit gate:** a single worker can inspect, plan, mutate, build, preview, test, repair, checkpoint, roll back, and produce an evidence-backed Android artifact across injected provider, process, toolchain, emulator, and source failures.

## Stage 3 — Durable autonomy

Extract or launch `NirmanSupervisor.exe` as the durable runtime. Add worker leases, background execution, persistent ConPTY terminals, Windows login/reboot/sleep recovery, ResourceGovernor, Android device management, PreviewCoordinator, provider supervision, reconnectable event streaming, and UI projection recovery.

**Stage 3 exit gate:** the supervisor continues eligible work when the UI closes, reconnects after UI restart, recovers after Windows restart, preserves SQLite execution state, and resumes from a validated checkpoint without routine human intervention.

## Stage 4 — Swarm and self-development

Only after Stages 1–3 pass their acceptance gates, add multiple workers, Git worktrees, reconciliation, Goal Mode, schedules, self-observation, self-improvement, candidate promotion, rollback, and advanced long-horizon optimization.

**Stage 4 exit gate:** parallel proposals reconcile through serialized commit barriers, self-development candidates run separately from the stable runtime, and every promotion or rollback is evidence-backed.

## 5B. Stack-lock implementation tasks

| Task | Required result |
|---|---|
| Desktop shell | Tauri 2 Windows shell and bundler |
| UI | React/TypeScript/Vite with Tailwind and shadcn/ui |
| UI state | Presentation-only Zustand or equivalent projection store |
| Rust runtime | Tokio supervisor interfaces and typed commands/events |
| Database | SQLite migrations, execution-ledger schema, SQLx evaluation |
| Editor | CodeMirror 6 first-release integration |
| Terminal | xterm.js renderer with Rust ConPTY supervisor |
| Provider | ModelGateway, adapters, streaming, capability detection, tool normalization |
| Android | Toolchain manifest, JDK/Gradle/SDK/ADB/emulator health and build |
| Windows | Restricted tokens, Job Objects, ACLs, environment filtering, process supervision |
| Packaging | Nirman.exe and Windows installer path |

## 5C. Sequencing invariant

Swarm work and self-development cannot begin until the single-worker runtime passes restart, provider-failure, process-failure, emulator-failure, rollback, evidence, and APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` artifact tests. This sequencing rule is mandatory even when later milestones are already specified.
# M65–M80: Agent Execution Kernel and Long-Horizon Runtime Formalization

These milestones formalize the autonomous runtime without changing Nirman’s Android-only generated target. They must be implemented after the foundation and durable-supervisor stages, and their gates must be tested with Android fixture projects and injected failures.

| Milestone | Focus | Required result |
|---|---|---|
| M65 | AgentExecutionKernel and loop reducer | Observe, understand, plan, select, authorize, execute, observe result, update, evaluate, continue/recover/delegate/validate/complete |
| M66 | SkillRuntime and composition | Discover, select, bind, execute, validate, compose compatible skills, and record SkillExecutionRecord |
| M67 | Dynamic worker instances and AgentProfiles | Construct bounded workers from role, profile, skills, tools, workspace, permissions, resources, context, and recovery policy |
| M68 | DelegationProtocol and knowledge ledger | Typed delegate/spawn/handoff/resume/cancel/replace/retry/escalate/merge operations and scoped KnowledgeArtifacts |
| M69 | TaskBlackboard and WorkspaceLeaseManager | Controlled task blackboard, renewable workspace ownership, stale lease recovery, and no duplicate workspace writes |
| M70 | Stateful ToolSessions and capability graph | Reconnectable terminal, ADB, emulator, debugger, LSP, and preview sessions mapped to required capabilities |
| M71 | EnvironmentCapabilityPlanner | Classify prerequisites as AVAILABLE, REPAIRABLE, USER_REQUIRED, or UNAVAILABLE before expensive work |
| M72 | ValidationPlanner | Select focused or expanded Android validation from changed files, symbols, graph impact, risk, requirements, and devices |
| M73 | Mutation and regression intelligence | Predict affected behavior using call, route, dependency, traceability, and historical-failure relationships |
| M74 | TrajectoryReplayEngine | Replay decisions and tool results against new models, prompts, skills, schemas, and runtimes without side effects |
| M75 | SimulationExecutor | Provide clearly labeled dry-run predictions without mutating source, executing commands, or claiming observed evidence |
| M76 | Deadlock and backpressure controls | Detect dependency/resource/approval cycles and reserve scarce Gradle, emulator, device, GPU, storage, and provider capacity |
| M77 | Cancellation and independent pause/resume | Propagate cancellation through every descendant and preserve exact pause/resume state for workers and skills |
| M78 | Decision, uncertainty, contradiction, and replanning services | Add structured decision nodes, fact states, contradiction revisions, and evidence-triggered plan recompilation |
| M79 | ExecutionHistoryManager | Implement hot, warm, cold, and archived history with safe compaction and evidence-preserving garbage collection |
| M80 | End-to-end autonomous-runtime certification | First milestone permitted to claim `RUNTIME_CERTIFIED` autonomous Android end-to-end execution: prove one long-running Android goal through dynamic allocation, failure recovery, replanning, device validation, required APK packaging or explicitly declared AAB packaging, replay, and history compaction |

## M65–M80 acceptance gates

### Kernel and authority gate

A model cannot execute directly. Every proposal passes schema, revision, capability, policy, transaction, observation, evidence, and reducer checks. Invalid transitions are rejected and replayable.

### Skill and worker gate

A composed Android workflow can allocate compatible skills and dynamically configured worker instances without granting new permissions. Every skill and worker produces a typed handoff and evidence record.

### Resource and liveness gate

Injected task cycles, worker waits, resource starvation, approval waits, stale leases, provider delays, and emulator contention produce deadlock or backpressure findings rather than indefinite waiting or uncontrolled spawning.

### Recovery gate

Cancellation, pause, worker replacement, process failure, provider failure, emulator loss, and tool-session reconnect preserve checkpoints, leases, context references, and evidence. Resumption does not repeat completed side effects.

### Traceability gate

An evaluator can select any mandatory requirement and follow it through acceptance criterion, task node, worker contract, skill, code change, validation run, evidence, and APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` artifact.

### Replay and simulation gate

A recorded trajectory can be replayed against a changed model or runtime without touching the real project. A dry run clearly distinguishes predicted commands and tests from observed and verified results.

### Long-horizon history gate

A multi-hour Android task can compact active state, move old records to warm/cold/archive tiers, restore a historical trace, and retain all required completion evidence, checkpoint parents, and artifact provenance.

### M80 certification fixture

The certification fixture should include a user instruction and optional screenshots for an Android application with multiple screens, offline data, a device capability, branded assets, background work, and a release artifact. The fixture must inject a dependency failure, a provider interruption, a stale worker, an emulator interruption, a contradiction in requirements, and a validation failure. Nirman must recover, replan, validate, produce the APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB`, and retain an inspectable trajectory without routine human intervention.

# M81–M93: Long-Horizon Intelligence, Verification, and Documentation Certification

These milestones implement build spec §53–§67 and technical architecture §59–§71. They follow the AgentExecutionKernel milestones M65–M80 and must be tested against Android fixture projects with injected failures. No milestone here may begin before the single-worker and durable-supervisor gates of Stages 1–3 have passed.

| Milestone | Focus | Required result |
|---|---|---|
| M81 | Memory and Context Runtime | Classified memory records with mandatory source events, ConstraintRegistry, ContextAssembler with constraint-priority budgeting, RegroundingService at all six trigger points, project-scoped isolation |
| M82 | Peer Coordination and Semantic Reservations | ReservationRegistry with the full conflict matrix, SurfaceIndex, StaleContractInvalidator, CommitBarrier freshness checks |
| M83 | User/Edit Reconciliation | ProjectWatcher, fingerprint-based OriginClassifier, evidence invalidation on user edit, BaselineUpdater that never reverts user content |
| M84 | Stateful E2E Scenario Engine | ScenarioRegistry, SeedDataProvisioner with recorded provenance, all eight required scenario classes, determinism quarantine |
| M85 | Advanced Verification | In-loop diagnostics and incremental compilation gate, assertion-before-implementation ordering, MutationProber vacuity rejection |
| M86 | Regression Localization | Impact-graph localization, signature matching, checkpoint bisection, cause-scoped repair enforcement, escalation on unlocalized regression |
| M87 | Adversarial Security and Supply Chain | AppSecurityScanner, exact-version dependency resolution with integrity hashes, SubstitutionDetector, SBOM and provenance, disposition discipline |
| M88 | Multi-Device E2E | DeviceMatrixResolver, DevicePool under backpressure, ScenarioDistributor, DivergenceAnalyzer, capability-status mapping |
| M89 | Runtime Directives and Agent Debugger | DirectiveIntake with validation and decision-boundary application, PlanReconciler effect accounting, read-only RuntimeSnapshot, SurfaceTracer, DecisionTracer |
| M90 | Historical Resource Profiling | Supervisor-level measurement, project/host-keyed profiles, PlanCostEstimate with honest confidence, capacity gating, DegradationDetector |
| M91 | External Event Gateway | TriggerRegistry, authentication, AdmissionController with ceiling capping, default-disabled webhook surface, complete firing audit |
| M92 | Speculative Candidate Branching | Isolated candidate workspaces, admission conditions, evidence-only selection, escalation on tie, discard hygiene with retained signatures |
| M93 | Documentation Coverage Certification | Ledger-based invariant verification for all ten invariants and a complete twelve-edge traceability chain for every capability |

## M81–M93 acceptance gates

### Memory and context gate

A session is interrupted by a runtime restart and resumes without re-asking a settled question. A locked decision remains present in every subsequent context package until superseded. A memory write attempted without a source event is rejected. A project-scoped query cannot return another project's records. A historical context package is reproduced from the ledger.

### Coordination gate

Two workers requesting modification of the same symbol produce one grant and one typed denial. A symbol rename invalidates a dependent worker's read-stable work and marks it unvalidated. A proposal validated before a dependent surface change is rejected at the commit barrier rather than merged.

### Reconciliation gate

A user edit made during an active run survives into the final artifact, validation predating the edit is discarded, an edit contradicting a locked decision produces a decision node, and generated build output never triggers reconciliation.

### Verification gate

A mutation introducing a compile error cannot advance to dependent work. An assertion authored after a passing implementation is flagged post hoc. A vacuous assertion set for critical logic is rejected. A data-persistence scenario detects an app that loses data on process death. A flaky scenario is quarantined rather than reported as passing.

### Localization gate

An injected single-line regression is attributed to its causing mutation. A repair mutation outside the identified cause scope is rejected. Bisection reuses existing checkpoints without full rebuilds. An unlocalized regression escalates rather than triggering broad regeneration.

### Security and supply-chain gate

A fixture containing a hardcoded secret is blocked before packaging. An unpinned or hash-mismatched dependency blocks the build. A package name resembling a known package is flagged. An artifact with an incomplete SBOM is not promotable. A finding cannot be dispositioned without a recorded reason.

### Multi-device gate

A missing secondary device produces a declared coverage gap rather than an implicit pass. A scenario passing on one API level and failing on another is recorded as a divergence defect. Emulator boots serialize when host capacity cannot sustain parallel emulators.

### Directive and debugger gate

A directive issued mid-run changes subsequent behavior without a restart and appears as an active constraint in the next context package. A directive requesting a permission increase is rejected with a recorded reason. A live run pauses at a decision boundary rather than mid-mutation. A completed session is fully inspectable from the ledger, and no debugger operation mutates the project.

### Resource profiling gate

Repeated identical fixture runs converge to stable profiles. An over-capacity plan is reduced or surfaced before execution begins. An operation class below the minimum sample count is reported as unprofiled rather than estimated. Injected disk pressure raises a host-health signal rather than an application defect.

### Trigger gate

A disabled webhook trigger opens no listening network surface. An over-scoped trigger request is rejected with a typed reason and audited. An admitted task's permission ceiling equals the minimum of the trigger and policy ceilings.

### Speculation gate

Parallel candidates leave the primary workspace untouched. The winning candidate is selected by identical validation evidence. A tie or universal failure escalates instead of arbitrary selection. Discarded candidate code never appears in the promoted artifact while its failure signature is retained.

## Foundational milestone contract mapping

These earlier milestones are referenced by the twelve-edge table of build spec §67.15 and must carry the same identifiers.

### Coarse-to-refined milestone ownership map

To prevent duplicate implementation of the same capability across coarse (M0–M38) and refined (M39–M117) milestones, every canonical capability carries exactly one owning refined milestone. Coarse milestones establish scope and interfaces; refined milestones own the executable contract, test identity, and evidence identity. An agent must not open a second implementation path for a capability already owned by a refined milestone. The mapping is:

| Capability area | Coarse scope (M0–M38) | Refined owner (M39–M117) | Canonical contract | Test / Evidence |
|---|---|---|---|---|
| Toolchain / clean build | M4 local runtime, M5 build→install | M43 AndroidToolchainManifest (ADR-049 toolchain authority) | CONTRACT.RUNTIME.WORKSPACE, CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | TEST-TC-001 / EV-TC-001 |
| Provider gateway | M3 foundation, M22 settings | M22 ModelGateway (full), M44 bridge | CONTRACT.RUNTIME.AUTHORITY, CONTRACT.RUNTIME.SCOPE | TEST-PRV-001 / EV-PRV-001 |
| Preview | M4/M9/M20 runtime preview | M108 PreviewSync | CONTRACT.RUNTIME.PREVIEW_SYNC (ADR-195) | TEST-PSYNC-001 / EV-PSYNC-001 |
| Execution kernel | M39 construction runtime | M65 AgentExecutionKernel | CONTRACT.RUNTIME.AUTHORITY (ADR-066/071) | TEST-GEN-001 / EV-GEN-001 |
| Packaging | M10 packaging | M10 (mechanism), M11 (coverage), M117 (export delivery) | CONTRACT.RUNTIME.APK_EXPORT (ADR-203) | TEST-APK-001 / EV-APK-001 |
| Reservation/lease/capability | M39 session/lease | M39/M43 reservation+lease+capability ordering | CONTRACT.RUNTIME.WORKSPACE (ADR-068) | TEST-RES-001 / EV-RES-001 |
| Evidence graph | M38 evidence foundations | M93 twelve-edge coverage; M108/114 evidence linkage | CONTRACT.RUNTIME.EVIDENCE (ADR-071) | TEST-INV-001 / EV-INV-001 |
| Continuity | M116 background continuity | M116 (orthogonal to lifecycle, ADR-202) | CONTRACT.RUNTIME.BACKGROUND_CONTINUITY | TEST-BG-001 / EV-BG-001 |
| Frontend boundary | M115 protocol | M115 (ADR-201) split gates A–F | CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE | TEST-FCP-001 / EV-FCP-001 |

Refinement rule: when a coarse milestone and a refined milestone appear to overlap, the refined milestone's contract is authoritative. Coarse acceptance semantics are overridden by the refined milestone's exit gate. No capability may be implemented twice; the owning refined milestone is the single source of the executable contract.

| Milestone | Implements ContractId | Locking ADR | Test id | Evidence id |
|---|---|---|---|---|
| M11 | CONTRACT.RUNTIME.SCOPE | ADR-180 | TEST-GEN-001 | EV-GEN-001 |
| M65 | CONTRACT.RUNTIME.AUTHORITY, CONTRACT.RUNTIME.EVIDENCE | ADR-066, ADR-071 | TEST-GEN-001 | EV-GEN-001 |
| M66 | CONTRACT.RUNTIME.SKILL | ADR-154 | TEST-SKL-001 | EV-SKL-001 |
| M69 | CONTRACT.RUNTIME.WORKSPACE | ADR-068 | TEST-RES-001 | EV-RES-001 |

## M81–M96 contract mapping

Each milestone may implement one or more registered contracts, but each contract must have one canonical owning milestone. This mapping is the addressing source for the reverse traversal required by §67.9; shared implementation milestones must list every contract they own and its acceptance evidence.

| Milestone | Implements ContractId | Locking ADR | Test id | Evidence id | Verifies |
|---|---|---|---|---|---|
| M81 | CONTRACT.RUNTIME.MEMORY, CONTRACT.RUNTIME.CONTEXT | ADR-140, ADR-141, ADR-155 | TEST-MEM-001 | EV-MEM-001 | Memory and context gate |
| M82 | CONTRACT.RUNTIME.RESERVATION | ADR-142, ADR-143 | TEST-RES-001 | EV-RES-001 | Coordination gate |
| M83 | CONTRACT.RUNTIME.RECONCILIATION | ADR-144 | TEST-RCN-001 | EV-RCN-001 | Reconciliation gate |
| M84 | CONTRACT.RUNTIME.E2E | ADR-146 | TEST-E2E-001 | EV-E2E-001 | Verification gate |
| M85 | CONTRACT.RUNTIME.VERIFICATION | ADR-148 | TEST-VER-001 | EV-VER-001 | Verification gate |
| M86 | CONTRACT.RUNTIME.LOCALIZATION | ADR-147 | TEST-LOC-001 | EV-LOC-001 | Localization gate |
| M87 | CONTRACT.RUNTIME.SUPPLY_CHAIN | ADR-149 | TEST-SEC-001 | EV-SEC-001 | Security and supply-chain gate |
| M88 | CONTRACT.RUNTIME.DEVICE_MATRIX | ADR-150 | TEST-DEV-001 | EV-DEV-001 | Multi-device gate |
| M89 | CONTRACT.RUNTIME.DIRECTIVE, CONTRACT.RUNTIME.DEBUGGER | ADR-145, ADR-152 | TEST-DIR-001 | EV-DIR-001 | Directive and debugger gate |
| M90 | CONTRACT.RUNTIME.PROFILING | ADR-153 | TEST-DIR-001 | EV-DIR-001 | Resource profiling gate |
| M91 | CONTRACT.RUNTIME.TRIGGER | ADR-151 | TEST-TRG-001 | EV-TRG-001 | Trigger gate |
| M92 | CONTRACT.RUNTIME.SPECULATION | ADR-156 | TEST-VER-001 | EV-VER-001 | Speculation gate |
| M93 | CONTRACT.RUNTIME.INVARIANTS | ADR-157 | TEST-INV-001 | EV-INV-001 | Documentation certification fixture |
| M94 | CONTRACT.RUNTIME.REASONING | ADR-167, ADR-168, ADR-169, ADR-170, ADR-171 | TEST-RSN-001 | EV-RSN-001 | Reasoning and delegation gate |
| M95 | CONTRACT.RUNTIME.DELIBERATION | ADR-172, ADR-173, ADR-174, ADR-175, ADR-176, ADR-177, ADR-178, ADR-179, ADR-184 | TEST-DEL-001 | EV-DEL-001 | Deep deliberation and provider-reasoning gate |
| M96 | CONTRACT.RUNTIME.PROMPT_CONTRACT, CONTRACT.RUNTIME.SCOPE | ADR-181, ADR-180 | TEST-GEN-001 | EV-GEN-001 | Intent synthesis and no-template enforcement gate |
| M107 | CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | ADR-194 | TEST-IB-001 | EV-IB-001 | Boundary schema, lifecycle, evidence, and reconciliation gate |
| M108 | CONTRACT.RUNTIME.PREVIEW_SYNC | ADR-195 | TEST-PSYNC-001 | EV-PSYNC-001 | Preview synchronization protocol and first Android vertical slice |
| M111 | CONTRACT.RUNTIME.COST_GOVERNANCE | ADR-197 | TEST-COST-001 | EV-COST-001 | Cost reservation, settlement, exhaustion, and degradation gate |
| M112 | CONTRACT.RUNTIME.AGENT_TRUST | ADR-198 | TEST-TRUST-001 | EV-TRUST-001 | Agent-layer trust scanning and revocation gate |
| M113 | CONTRACT.RUNTIME.CONTEXT_GOVERNANCE | ADR-199 | TEST-CONTEXT-001 | EV-CONTEXT-001 | Context compaction, cache, and telemetry governance gate |
| M114 | CONTRACT.RUNTIME.ANDROID_INTEGRITY | ADR-200 | TEST-INTEGRITY-001 | EV-INTEGRITY-001 | Android runtime integrity and honest coverage gate |
| M115 | CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE | ADR-201 | TEST-FCP-001 | EV-FCP-001 | Frontend–control-plane protocol and generated service adapter gate |
| M116 | CONTRACT.RUNTIME.BACKGROUND_CONTINUITY | ADR-202 | TEST-BG-001 | EV-BG-001 | Background continuity state machine, interruption recovery, fencing, reconciliation, and truthful projection gate |
| M117 | CONTRACT.RUNTIME.APK_EXPORT | ADR-203 | TEST-APK-001 | EV-APK-001 | Local APK export provenance, packaging-profile admission, hash equality, and post-copy verification gate |
| M118 | CONTRACT.RUNTIME.PLATFORM_CAPABILITY | ADR-206 | TEST-PLAT-001 | EV-PLAT-001 | Platform capability truth, cross-build admission, and native-validation gate |

M93 must additionally run the contract-graph verifier of build spec §67.11 across all eleven §67.11 contract-graph checks in both traversal directions, plus the verifier's document-structure check. It must fail on any duplicate authority, unregistered contract, undeclared extension, authority cycle, clause contradiction, unversioned override, dangling reference, forward break, reverse break, orphan contract, canonical-identity violation, or structure violation.

### M93 Contract-Graph Certification Regression Gate

M93 establishes `DOCUMENTATION_CERTIFIED` status only. It runs the contract-graph verifier of build spec §67.11 across the declared contract-graph checks and document-structure checks, and verifies that every capability in the §5.6 coverage matrix resolves to its required traceability chain. It does not establish runtime certification, Android build/device certification, preview certification, APK certification, or product completion. Runtime certification remains executable and milestone-specific.

Any missing edge, duplicate authority, unregistered contract, undeclared extension, authority cycle, clause contradiction, unversioned override, dangling reference, forward break, reverse break, orphan contract, canonical-identity violation, or structure violation is a documentation defect that must be recorded and resolved. No capability may be reported as supported while its required documentation chain is incomplete, and no runtime or product claim may be promoted from this gate alone.

# M94: Agent Reasoning Runtime and Bounded Delegation

Implements build spec §66 and technical architecture §71. This milestone follows M81–M93 and must not begin before the AgentExecutionKernel milestones M65–M80 and the certification milestone M93 have passed their gates. It adds the reasoning cycle that drives the existing kernel loop; it does not introduce a second execution loop.

| Milestone | Implements ContractId | Locking ADR | Test id | Evidence id | Verifies |
|---|---|---|---|---|---|
| M94 | CONTRACT.RUNTIME.REASONING | ADR-167, ADR-168, ADR-169, ADR-170, ADR-171 | TEST-RSN-001 | EV-RSN-001 | Reasoning and delegation gate |

**Required results:** AgentReasoningEngine driving the cycle state machine; ReasoningArtifact persistence with mandatory cited selectionBasis; HypothesisManager with the full CREATED/TESTED/SUPPORTED/REJECTED/SUPERSEDED lifecycle and evidence-bound rejection; ReflectionEngine producing expected-versus-observed records; CapabilityRegistry with runtime discovery; CapabilityBroker routing every invocation through the policy authorities; DelegationManager enforcing both ceiling invariants with cascading revocation; SwarmGraphManager applying agent-proposed graph revisions through the standard authority path; ExecutionModeSelector proposing modes within policy.

### Reasoning and delegation gate

A goal produces a recorded ReasoningArtifact with a cited selectionBasis before any mutation occurs. An artifact submitted with an empty selectionBasis is rejected at write and the cycle returns to strategy selection. No persisted record in any reasoning table contains verbatim model reasoning. Every executed action produces a ReflectionRecord classifying the outcome as SUCCESS, PARTIAL, FAILURE, or UNKNOWN with evidence references. A hypothesis rejected with refuting evidence is retained and is not retested against unchanged evidence. An untargeted repair is not attempted while an untested discriminating test remains available. A capability invocation denied by policy returns the cycle to strategy selection with the denial present as an active constraint in the next artifact. A delegation request whose child capability ceiling exceeds its parent's, or whose resource budget exceeds the parent's remaining budget after outstanding sibling grants, is denied with a typed reason. Revoking a parent grant terminates every descendant. A newly registered capability becomes discoverable without a code change to the reasoning engine. A mode request exceeding policy is downgraded to the highest permitted mode and recorded. Every cycle terminates in exactly one of COMPLETED, BLOCKED, WAITING, RECOVERED, SAFELY_FAILED, or ESCALATED, and SAFELY_FAILED is never reported as completion.

# M95: Deep Deliberation Runtime

Implements build spec §68 and technical architecture §72. Prerequisite: M94 must pass its reasoning and delegation gate. This milestone adds the deliberation runtime that decides how much reasoning to perform inside the existing cycle; it introduces no third execution loop.

**Required results:** DeliberationController driving bounded passes; DeliberationBudgetManager enforcing every ceiling including maxToollessPasses; ReasoningEffortSelector granting the minimum of request, policy ceiling, fundable level, and provider capability with the binding constraint recorded; SufficiencyEvaluator implementing the §68.7 conjunction rather than reading stated confidence; HypothesisEvaluator competing candidates by decisiveness over cost and reporting refutation-versus-confirmation; CounterexampleEngine emitting findings and evidence requests with no mutation capability; EvidenceAcquisitionPlanner restricted to non-mutating observations and costed from the resource profiler; DeliberationModelRouter escalating under an unchanged permission ceiling; DeliberationContinuationManager checkpointing session state at every pass boundary; DeliberationProgressEvaluator and DiminishingReturnDetector forcing an approach change on NO_PROGRESS; DeliberationRecordStore rejecting inadmissible records.

### Deep deliberation gate

An agent request for EXHAUSTIVE under a policy ceiling of EXTENDED is granted EXTENDED with the binding constraint recorded, and never self-granted. A deliberation exceeding its pass budget terminates BUDGET_EXHAUSTED and the cycle does not execute the leading strategy. Consecutive observation-free passes are refused at the maxToollessPasses bound until evidence is acquired. A change classified high-risk is refused sufficiency at a stated confidence of 0.95 while its regression plan is missing. A discriminating test refutes the leading hypothesis and the selected strategy changes as a result. A counterexample finding returns the cycle to strategy selection with no project mutation. An escalated model executes under the identical permission ceiling. A forced context compaction preserves active hypotheses and rejected strategies, and the session resumes without re-deriving them. Deliberation reaching the fixture's configured `diminishingReturnThreshold` across consecutive passes of flat uncertainty produces NO_PROGRESS and an approach change rather than a further plain pass. No deliberation record contains verbatim model reasoning.

**Causal escalation.** An escalation event is not sufficient. The gate requires that the recorded condition is the causal trigger for the escalation, evidenced as an ordered chain in the event ledger:

```text
condition observed          (uncertainty above threshold | competing hypotheses
                             unresolved | high-risk classification)
      -> ReasoningEffortSelector decision citing that condition
      -> requested level
      -> granted level with binding constraint
      -> additional deliberation performed at the granted level
      -> outcome changed relative to the pre-escalation strategy
```

A run whose effort level differs before and after without that chain fails the gate. Specifically, an escalation whose `grantDecisionReason` cites no observed condition, or which is requested at fixture start rather than in response to a condition, does not satisfy causal escalation even though `levelBefore != levelAfter`.

**Causal strategy revision.** A strategy revision must be justified by a change in evidence or constraints. A revision from strategy A to strategy B against an unchanged evidence set, unchanged uncertainty, and unchanged constraint set is manufactured activity and fails the gate. Each recorded `rejectedStrategies` entry must cite the refuting evidence reference or the constraint that invalidated it.

**No mutation during deliberation.** Certification must inspect the event ledger and assert that no project mutation event occurs between deliberation entry and the kernel `AUTHORIZE` grant. The permitted ordering is:

```text
DELIBERATION_PASS*  (observations only, zero mutation events)
      -> ReasoningArtifact emitted
      -> AUTHORIZE granted
      -> mutation events permitted
```

Any mutation event carrying a deliberation pass as its originating context is a shadow execution path and fails the gate regardless of the run's outcome. This assertion covers the whole deliberation phase, not only the adversarial critic.

### M95 fault-injection fixtures

The gate above states required behavior. These seven fixtures inject the specific
fault each rule exists to prevent, so the rule is proven rather than asserted.
Each runs against a real Android fixture project with a configured
`DeliberationBudget`, and each must produce the stated observable outcome.

| Fixture | Injected condition | Required observable outcome |
|---|---|---|
| FIX-DEL-01 no-evidence loop | A question the model cannot resolve from the current observation set | Passes proceed until `maxToollessPasses`, then the runtime refuses a further plain pass and either acquires evidence or terminates; it never loops indefinitely |
| FIX-DEL-02 budget exhaustion | A budget too small to reach sufficiency | Terminates `BUDGET_EXHAUSTED`; the leading strategy is not executed; the cycle yields `WAITING`, `SAFELY_FAILED`, or `ESCALATED` |
| FIX-DEL-03 forced compaction | Context compaction triggered mid-deliberation with several hypotheses rejected | Session resumes with active hypotheses and rejected strategies intact; no rejected hypothesis is re-derived or retested against unchanged evidence |
| FIX-DEL-04 provider failover | The provider fails between passes of one deliberation | The session resumes from the last deliberation checkpoint with the same remaining runtime budget and required effort level. The replacement provider's reasoning capability is revalidated before continuation. If it supports the required level, continuation occurs at that level; otherwise the runtime selects another approved provider/model or terminates with a typed capability gap. Continuation state is never reset silently |
| FIX-DEL-05 native reasoning normalization | Provider exposes native reasoning with a provider-specific effort parameter | NORMAL/EXTENDED/DEEP/EXHAUSTIVE runtime requests are translated into the provider's declared parameter space; the normalized request and granted capability are recorded; no provider-specific setting bypasses the runtime budget |
| FIX-DEL-06 reasoning usage accounting | Provider reports reasoning usage for one pass | Reported reasoning usage is recorded and settled against the reserved budget; the ledger distinguishes reported usage from runtime wall-clock and model-request counts |
| FIX-DEL-07 reasoning capability gap | Provider does not support the requested minimum reasoning effort | The runtime records the capability gap and either selects an approved compatible provider/model or terminates safely; it never claims the requested effort was performed |

Each fixture must also assert the two invariants that hold across all of them:
the ledger contains zero project mutation events before the `AUTHORIZE` grant,
and no persisted deliberation record contains verbatim model reasoning.

A fixture that passes because the runtime never entered deliberation does not
count. Each must show a recorded `DeliberationRecord` with `passCount` greater
than one before its outcome is evaluated.

### M95 certification fixture

Certification requires a deliberately difficult Android fixture exercising the full loop end to end: the initial strategy fails; the agent enumerates multiple competing hypotheses; it acquires discriminating evidence; at least one hypothesis is refuted and recorded with its refuting evidence; additional deliberation budget is consumed with a stated reason per pass; reasoning effort escalates on a recorded condition; an alternative strategy is selected on evidence rather than preference; implementation proceeds through the ordinary authority path; validation discovers a second issue; deliberation resumes with prior rejections intact; the cause is localized and repaired within its cause scope; stateful end-to-end scenarios pass on the primary device; and the final report proves completion with evidence of an applicable kind for every requirement.

Passing this fixture requires the deliberation runtime to demonstrably change the outcome. Because the milestone exists to demonstrate the complete mechanism, all three of the following are mandatory and each must be causally connected to the subsequent outcome:

| Required demonstration | Causal requirement |
|---|---|
| Evidence-backed hypothesis refutation | A discriminating test result refutes a named hypothesis, and the refutation changes which strategy is selected |
| Causal effort escalation | An observed condition triggers the escalation per the causal-escalation chain, and the additional deliberation at the granted level changes the outcome |
| Evidence-backed strategy revision | A change in evidence or constraints causes the revision, cited on the rejected strategy |

A run reaching completion while missing any one of the three does not certify this milestone. Nor does a run exhibiting all three as uncaused events: an escalation without a citing condition, a refutation without a discriminating test result, or a revision against an unchanged evidence and constraint set each fail independently of the run's final outcome.

This is the anti-vacuity rule of §57.5 applied to the deliberation capability itself. An assertion set that passes against a runtime which never actually deliberated is vacuous evidence, exactly as an assertion set that passes against a deliberately broken implementation is vacuous evidence.


## M96 — IntentSynthesisPromptContract and no-template enforcement

Implement the shared prompt-builder contract for coordinator, worker, skill, review, and deliberation prompts. Prompts must extract Android product intent, distinguish facts from assumptions, propose an Android technology plan without a framework or template choice, and produce schema-validated proposals rather than executable commands. Add negative fixtures for template-selection requests, app-archetype assumptions, non-Android target proposals, and model claims that predicted work was executed.

**Exit gate:** prompt fixtures reject user-facing template selection, reject non-Android generated targets, preserve user intent and uncertainty, and route every accepted proposal through schema validation, policy, ToolBroker, transaction, observation, and evidence authorities.

## M97 — Revision-bound PreviewCoordinator

Implement `PreviewCoordinator`, `PreviewRequest`, and immutable `PreviewRevision` identity binding project revision, checkpoint, source fingerprint, contract version, technology-plan version, asset manifest, build variant, artifact, device, execution truth, runtime state, validation state, and evidence IDs. The coordinator is the only service allowed to create, reload, install, promote, invalidate, or roll back a live Android preview.

**Exit gate:** a preview cannot be created as current from model text, a prediction, a simulation, or a build result alone; every current preview has observed build, install, launch, device, and revision evidence.

## M98 — Truthful stepwise preview projection

Implement the side-by-side Android preview and execution/evidence surface with `PREDICTED`, `SIMULATED`, `REQUESTED`, `OBSERVED`, `VERIFIED`, `STALE`, and `INVALIDATED` truth labels. Add the preview state machine, last-known-good protection, stale-candidate display, evidence drawer, revision comparison, reconnectable event projection, and no-fabrication presentation tests.

**Exit gate:** a failed candidate cannot replace the last-known-good preview; UI disconnect and reconnect reconstruct the same projection; predicted or simulated stages are never displayed as running, passed, or verified.

## M99 — End-to-end synthesis and preview certification

Run a fixture that starts from one Android product concept and optional screenshots, selects the implementation autonomously, constructs code and branding assets, updates the emulator/device preview through real revisions, injects build, install, runtime, and stale-revision failures, recovers from a checkpoint, and produces an APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` whose source, assets, preview, tests, and evidence identities match.

**Exit gate:** the complete path passes without a user-facing template or framework picker, with no fake execution status, and with a revision-bound evidence report proving the promoted APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB`.

### M96–M99 acceptance matrix

| Capability | Required proof |
|---|---|
| No-template synthesis | Negative prompt fixtures reject template, archetype, and non-Android proposals |
| Intent contract | User intent, screenshots, assets, constraints, and uncertainty are persisted in a versioned contract |
| Prompt authority boundary | Model output becomes a proposal and cannot authorize tools, mutations, or completion |
| Preview identity | Every current PreviewRevision has project revision, checkpoint, source, asset, device, and evidence identity |
| Truth labels | Predicted, simulated, requested, observed, verified, stale, and invalidated states remain distinct |
| Last-known-good | Failed candidates preserve the previous valid preview and evidence |
| Reconnect | UI restart, supervisor restart, and event replay reconstruct the same preview projection |
| Final artifact | APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` source, asset, preview, validation, checksum, and release evidence refer to the same revision |


## M100 — Canonical state, artifact, and completion semantics

M100–M105 are contract-hardening prerequisites for any milestone that claims runtime verification, preview promotion, integration completion, packaging, or certified capability coverage. They may be implemented as a preparatory track before the corresponding vertical-slice milestone, even though their acceptance summary is listed after M99. No later milestone may promote a capability using an invariant that has not passed its applicable M100–M105 gate.

Implement separate fields and reducers for product lifecycle, assurance, capability maturity, integration operationality, signing, artifact, preview, and delivery state. Define the v1 artifact policy as an installable APK required for local completion, with AAB available only as an explicitly declared optional artifact profile. Add `PackagingProfile`, `ArtifactSet`, `SigningState`, `ReproducibilityLevel`, `DeliveryState`, and `CompletionDecision` schemas.

**Exit gate:** illegal combinations such as completed-without-evidence, verified-from-simulation, current-preview-with-stale-identity, supported-without-profile evidence, delivered-without-checksum, functional-without-integration evidence, and inspected-with-unknown-signing state are rejected by deterministic tests.

## M101 — Evidence dependencies and operational integrations

Implement the `Observation → EvidenceArtifact → ValidationResult → CertificationDecision → CompletionDecision` dependency chain with explicit freshness, supersession, and cascading invalidation. Add operationality states for required APIs and external services, including configured, reachable, functional, degraded, user-required, unavailable, blocked, and unknown. Build and launch evidence must not satisfy a required integration by themselves.

**Exit gate:** changing a source revision, asset manifest, toolchain lock, device session, artifact, validation policy, dependency snapshot, or required integration invalidates all dependent completion claims unless the dependency graph proves independence. A required integration without functional evidence remains non-complete.

## M102 — External-effect and resource-accounting reconciliation

Implement idempotency keys, request fingerprints, target identity, unknown-outcome reconciliation, read-back, compensation state, and local transaction linkage for remote or externally visible effects. Add parent, child, shared, measured, estimated, and unavailable resource attribution to the execution ledger.

**Exit gate:** a lost response after a possibly delivered side effect cannot cause an unsafe duplicate request; parent and child usage remains attributable; estimates are never presented as observed consumption or completion evidence.

## M103 — Profile maturity, trust, signing, and reproducibility certification

Extend each Android capability profile with stable profile identity, technology composition, toolchain lock, environment identity, repository-trust requirement, device matrix, fixture IDs, known exclusions, required evidence, signing policy, reproducibility level, last validated revision, and evidence report. Add repository trust classification and explicit signing lifecycle states.

**Exit gate:** a capability status cannot become `SUPPORTED` or `CERTIFIED` without matching fixture and evidence records; release reports distinguish rebuildable, reproducible, bitwise reproducible, not reproducible, and unknown outcomes; debug signing cannot be represented as release signing.

## M104 — Hidden-human-dependency and runtime-proof fixtures

Add adversarial fixtures for interactive terminal prompts, provider login, expired credentials, device unlock, emulator dialogs, package-manager confirmation, signing selection, missing environment variables, GUI-only installers, external-service approval, and suppressed notifications. Add separate runtime-certification jobs for schema compilation, reducer transitions, transactions, leases, Windows process and IPC behavior, provider fixtures, Android builds, emulator/device execution, preview truth, APK inspection, failure injection, restart recovery, and self-development rollback.

**Exit gate:** an unattended task either completes through an explicitly authorized automatic path, creates a durable `USER_REQUIRED` decision, or reaches a truthful blocked state. It must never remain silently running. Documentation graph certification is reported separately from runtime and artifact certification.

## M105 — Schema parity and cross-document conformance

Choose one canonical owner for every machine-readable schema and make every implementation, architecture, roadmap, and decision reference explicit. Add parity checks for field identity, enum values, lifecycle semantics, migration version, authority owner, and evidence dependencies. Extend the acceptance matrix with independent mutations of source, assets, toolchain, artifact, device session, contract version, and evidence freshness.

**Exit gate:** a schema mutation, state-enum mutation, artifact-policy mutation, or missing dependency relation fails certification rather than being hidden by duplicate explanatory prose.

### M100–M105 acceptance matrix

| Capability | Required proof |
|---|---|
| State separation | Lifecycle, assurance, capability, integration, signing, artifact, preview, and delivery fields are independent and reducer-tested |
| Artifact policy | APK is the required local artifact; AAB is only produced when an explicit optional profile requests it |
| Evidence dependency | Dependent evidence and completion claims invalidate after identity or policy changes |
| Integration operationality | Required API/service behavior reaches its declared minimum operational state through supervised evidence |
| External-effect safety | Unknown remote outcomes reconcile through idempotency or read-back before retry |
| Resource attribution | Parent, child, shared, estimated, and unavailable usage are represented without double counting |
| Capability maturity | `SUPPORTED` and `CERTIFIED` require profile, fixture, and current evidence identities |
| Hidden-human dependency | Prompts and manual gates become explicit decisions or safe automatic actions, never silent waits |
| Runtime certification | Runtime, Windows host, Android, recovery, security, preview, and artifact tests are separate from documentation certification |
| Schema parity | Canonical schema fields, states, migrations, and authorities remain aligned across implementation documents |
| Cross-entity preview | Source, assets, toolchain, artifact, device session, contract, and evidence identities all satisfy the current predicate |

## Implementation-status boundary

Milestones M100–M105 define implementation work, not completed capability. Until their exit gates pass on executable fixtures, the relevant status remains `PLANNED` or `SPECIFIED`. A documentation certification pass cannot promote a runtime capability, preview, or APK artifact to `VERIFIED` or `CERTIFIED`.

## M106 — Documentation-verifier conformance

Extend the documentation verifier’s test suite beyond mutation detection. Add positive and negative fixtures for false positives, valid optional extensions, duplicate references, malformed tables, Unicode and Markdown formatting variation, registry ordering changes, contract identifiers in prose and comments, fenced code blocks, duplicate rows, ambiguous headings, and repeated explanatory text. Keep the existing mutation battery non-vacuous and preserve the boundary that this verifier certifies documentation structure only, never runtime behavior.

**Exit gate:** the conformance suite passes on valid documents, detects each declared malformed or ambiguous fixture, reports the expected defect class, and remains deterministic across repeated runs. Runtime certification remains a separate implementation test family.

## M107 — Integration boundary contract and wiring conformance

M107 implements build spec §70 and technical architecture §74. It follows the contract-hardening prerequisites M100–M106. It must not introduce a second lifecycle, transaction, evidence, preview, provider, skill, artifact, signing, or completion authority.

Implement the versioned `IntegrationBoundaryContract` reference envelope and `BoundaryOperationProjection`. Add schema parity and compatibility records for payloads, responses, protocols, adapters/bridges, authorities, specialized state references, transaction domains, permissions, credentials, timeouts, cancellation, retries, observations, evidence, validation, downstream effects, and invalidation. Complete UI command/projection correlation, Android service-integration records, provider/context binding, UI-hierarchy observations, skill/external-tool lifecycle vocabulary, signing and certificate inspection, post-copy artifact export verification, and documentation certification reporting.

The fixture matrix must cover UI reconnect and stale-command rejection; provider and context correlation; skill-to-capability-to-tool mediation; worker lease-loss fencing; patch/revision freshness; Android service functional evidence and independent operationality dimensions; emulator/device installation and UI-hierarchy evidence; signing certificate inspection; source/destination export hash equality; unknown external-effect reconciliation; timeout and cancellation propagation; adapter/protocol incompatibility; invalidation of downstream evidence; and separation of documentation certification from runtime certification.

**Exit gate:** every applicable boundary-crossing operation resolves one registered `IntegrationBoundaryContract`, all universal-chain references are resolvable, specialized authorities remain singular, unknown outcomes cannot be retried unsafely, stale identities cannot produce current effects, and all M107 fixtures produce durable evidence. A documentation verifier pass alone cannot promote runtime capability or artifact status.

## M108 — Preview synchronization protocol and first Android vertical slice

M108 implements build spec §71 and technical architecture §75. It must follow the chat, control-plane, worker, workspace, build, device, evidence, and preview authorities already defined by the earlier milestones. It does not permit an agent, worker, UI, or model to mutate preview state directly.

Implement `PreviewSyncEvent`, `PreviewProjection`, `PreviewProjectionReducer`, and `PreviewSyncEvidenceRecord` with canonical schema registry entries, version compatibility, durable event sequences, idempotent replay, projection revisions, preview identity checks, causal lineage, authority classes, and evidence lineage. Record acceptance using `TEST-PSYNC-001` and `EV-PSYNC-001`. Connect the user chat request to intent acceptance, contract validation, agent authorization, source revision, checkpoint, Android build, APK artifact, emulator/device installation, launch, interaction, observation, validation, promotion, and panel projection.

### M108 work items

| Work item | Implements | Acceptance condition |
|---|---|---|
| Technology Adapter Runtime | TA §73.10; `CLAUSE.PREVIEW_SYNC.ADAPTER_BOUND` | Three internal adapter families (`NativeAndroidAdapter`, `JavaScriptAndroidAdapter`, `MixedAndroidAdapter`) registered as strategy and composition adapters; only `validatePlan`, `initializeProject`, `planBuild`, `classifyFailure`, `resolveBuildAdapter`, `resolveDeviceAdapter` exposed; no concrete execution operation on the technology adapter; `resolveBuildAdapter` and `resolveDeviceAdapter` are deterministic over the locked `AndroidTechnologyPlan`, `AndroidToolchainLock`, and `AndroidDeviceCapabilities`; emitted `PreviewSyncEvent` and `PreviewSyncEvidenceRecord` carry `adapterId`, `adapterVersion`, `technologyPlanHash`, and the resolved `buildAdapterIdentity` or `deviceAdapterIdentity` |
| Deterministic Preview Mode Resolver | TA §73.11; `CLAUSE.PREVIEW_SYNC.MODE_RESOLVER` | Pure-function resolver over `PreviewModeResolverInput` with the canonical rule table returning `PreviewModeResolverOutput`; mode values are limited to the §73.3 enumeration; resolver never mutates state; resolver output recorded as part of the `PreviewRequest` decision trace; no model, worker, UI, or prompt selects the preview mode directly |
| Android Device Adapter | TA §73.12 | `AndroidDeviceAdapter` interface satisfied by both emulator and physical-device implementations; every operation returns a typed observation carrying `adapterId`, `adapterVersion`, `deviceId`, `deviceSessionId`, `runtimeSessionId`, `environmentFingerprint`, `applicationStateFingerprint`, `evidenceReferences`, `failureClassification`, `invalidationDependencies`; operations do not write `PreviewProjection`, evidence identity, artifact promotion, or completion state |
| Android Build Adapter | TA §73.13 | `AndroidBuildAdapter` interface covering Gradle native, Gradle plus Metro or Expo, React Native, NDK or CMake, and mixed native plus JavaScript; returns `AndroidBuildObservation`; does not create a second build authority; does not bypass `ToolchainAuthority` or `ArtifactAuthority` |
| Preview Panel Pipeline | TA §73.14 | The legal UI→`PreviewCoordinator`→`AndroidTechnologyAdapter`→`AndroidBuildAdapter`/`AndroidDeviceAdapter`→observation→`PreviewSyncEvent`→`PreviewProjectionReducer`→`PreviewPanel` path is the only legal pipeline; `UI → ADB`, `UI → Gradle`, `UI → Metro or Expo`, `UI → emulator` are rejected by the typed command registry and by the contract-graph verifier |

### M108 acceptance chain

For every certified Android profile, the durable execution path MUST resolve:

```text
AndroidTechnologyPlan
  → AndroidTechnologyAdapter (adapterId, adapterVersion, technologyPlanHash)
  → AndroidTechnologyAdapter.validatePlan | planBuild | classifyFailure
  → AndroidTechnologyAdapter.resolveBuildAdapter (deterministic, auditable)
  → AndroidTechnologyAdapter.resolveDeviceAdapter (deterministic, auditable)
  → AndroidToolchainLock
  → AndroidBuildAdapter → AndroidBuildObservation
  → Artifact identity
  → AndroidDeviceAdapter → install and launch observation
  → RuntimeObservation
  → PreviewSyncEvent (carries adapterId, adapterVersion,
    technologyPlanHash, buildAdapterIdentity, deviceAdapterIdentity)
  → PreviewProjection
  → Evidence (PreviewSyncEvidenceRecord carries the same identities)
  → PreviewPromotionGate
```

The technology adapter resolves the execution authorities; it does not execute their concrete operations itself. Concrete build, install, launch, observation, screenshot, UI hierarchy, Logcat, validation, and failure-classification operations have exactly one execution surface each.

### M108 parameterized fixture matrix

The `TEST-PSYNC-001` / `EV-PSYNC-001` acceptance harness MUST run the same event-store, reducer, projection, evidence, promotion, and replay tests over each row of the matrix. Each row is a profile instance of `AndroidCapabilityProfile` (with the new `adapterId`, `adapterVersion`, `technologyPlanHash`, `buildStrategyId`, `previewStrategyId`, `runtimeStrategyId`, `validationStrategyId` fields populated) and exercises one legal preview-mode branch from the §73.11 rule table.

| Fixture | Required path |
|---|---|
| Kotlin + Views | `NativeAndroidAdapter`; `INCREMENTAL_APK_INSTALL` or `FULL_APK_REINSTALL` |
| Kotlin + Compose | `NativeAndroidAdapter`; `COMPOSE_RELOAD` for Compose-only changes with `sameNativeIdentity`, otherwise `INCREMENTAL_APK_INSTALL` or `FULL_APK_REINSTALL` |
| Java + Views | `NativeAndroidAdapter`; `INCREMENTAL_APK_INSTALL` or `FULL_APK_REINSTALL` |
| React Native | `JavaScriptAndroidAdapter`; `RN_EXPO_FAST_REFRESH` for JavaScript or TypeScript-only changes with `sameNativeIdentity` and `healthyMetroExpoRuntime`, otherwise `INCREMENTAL_APK_INSTALL` or `FULL_APK_REINSTALL` |
| Expo | `JavaScriptAndroidAdapter`; `RN_EXPO_FAST_REFRESH` for JavaScript or TypeScript-only changes with `sameNativeIdentity` and `healthyMetroExpoRuntime`, otherwise `INCREMENTAL_APK_INSTALL` or `FULL_APK_REINSTALL` |
| Native module | `MixedAndroidAdapter`; `FULL_APK_REINSTALL` when ABI changes, otherwise `INCREMENTAL_APK_INSTALL` |
| NDK or CMake | `MixedAndroidAdapter` (composed Gradle plus NDK or CMake); `FULL_APK_REINSTALL` when ABI changes, otherwise `INCREMENTAL_APK_INSTALL` |
| Device API | `AndroidDeviceAdapter`; `INCREMENTAL_APK_INSTALL` or `FULL_APK_REINSTALL` with device-permission observation recorded |
| Native + JavaScript | `MixedAndroidAdapter`; `RN_EXPO_FAST_REFRESH` for JavaScript or TypeScript-only changes with `sameNativeIdentity` and `healthyMetroExpoRuntime`, otherwise `INCREMENTAL_APK_INSTALL` or `FULL_APK_REINSTALL` |
| Insufficient impact information (any row) | `CONSERVATIVE_FULL_REINSTALL` selected by §73.11 rule 7b with `decisionReason = INSUFFICIENT_IMPACT_INFORMATION` |
| Known unsafe-to-fast-refresh (any row) | `FULL_APK_REINSTALL` selected by §73.11 rule 7a with `decisionReason = KNOWN_UNSAFE_TO_FAST_REFRESH` |

The matrix is one parameterized test harness. M108 MUST NOT spawn nine separate test systems; the existing M108 fixture runner, the verifier, and the conformance battery must be extended to walk the matrix with `profileId`-keyed inputs.

### M108 documentation-versus-runtime status

Documentation and contracts: the contract and parameterized fixture specification cover nine Android profiles plus the §73.11 fallback branches. This is the strongest defensible claim from the documentation layer.

Parameterized coverage: nine technology profiles specified as `AndroidCapabilityProfile` instances and reachable through `resolveBuildAdapter` / `resolveDeviceAdapter` against the canonical rule table.

Runtime certification: not claimed by this milestone. Runtime certification of the nine profiles requires `TEST-PSYNC-001` fixture executions against matching environment fingerprints, toolchain locks, device sessions, and source revisions per ADR-195, and is tracked separately. The current device-preview behavior depends on an actually attached matching device and the runtime adapter implementations; neither is asserted by this documentation milestone.

**Exit gate:** one real Android fixture completes the full path from chat intent to durable task/goal, requirements and acceptance criteria, agent plan, authorized worker execution, source revision, build, APK, emulator/device runtime, observed evidence, validated promotion, durable synchronization event sequence, and reconstructed preview panel projection. The fixture must prove that a model statement, successful build, or worker progress message cannot make the panel show a current running preview, and that every displayed claim retains causal provenance. The contract-graph verifier §67.11 reports zero defects; `CLAUSE.PREVIEW_SYNC.ADAPTER_BOUND` and `CLAUSE.PREVIEW_SYNC.MODE_RESOLVER` are reported SEALED in §67.12. Each row of the M108 parameterized fixture matrix is parameterized into `TEST-PSYNC-001`; runtime execution of each row is tracked separately and is not asserted by this milestone.

## M109 — Preview projection resilience and runtime-certification evidence

M109 implements the resilience and certification portion of build spec §71.4–§71.5 and technical architecture §75.3–§75.4. Add UI and supervisor fixtures for duplicate events, conflicting duplicate payloads, out-of-order delivery, missing sequence ranges, stale revisions, late device observations, stream loss, UI reconnect, supervisor restart, failed candidates, last-known-good preservation, rollback, recovery, and deterministic replay.

Persist `PreviewSyncEvidenceRecord` for each displayed completed stage and verify event range, reducer version, projection revision, preview revision, branch/candidate identity, device identity, runtime session, artifact fingerprint, state fingerprints, observation references, evidence references, validation references, invalidated evidence, recovery events, and promotion or completion decisions. Documentation certification remains separate from runtime certification. Include fixtures for worker stalls, watchdog fencing, replacement-worker resume, emulator/device restart and reconnect, cancellation, rollback, late events, and wrong-revision or wrong-device evidence.

**Exit gate:** live application and replay produce identical projections; disconnected UI cannot advance truth; stale or late events cannot overwrite current state; failed candidates cannot replace last-known-good; current runtime observations reconcile compatible persisted state; incompatible observations become stale or invalidated; and the complete fixture evidence proves chat-to-preview synchronization rather than only documentation presence.

## M110 — Event-driven autonomous continuation and specialist gates

M110 implements the continuation requirements in build spec §27.11 and technical architecture §76 by composing the existing trigger, runtime-tick, task-graph, verification, recovery, dependency-health, workspace, memory, and promotion contracts. It does not create a new authority or permit generic deployment behavior.

Implement durable continuation triggers for saved workspace revisions, completed builds, observed failures, dependency changes, local preview promotion, declared APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` export, and stream reconnect. Each trigger must schedule the next authorized action with the current revision, checkpoint, worker run, operation capability, correlation and causation identifiers, attempt history, and evidence references. Failure handling must capture diagnostics and stack-trace references, create a stable failure fingerprint and `FailureContextPackage`, and provide that context to the next authorized diagnostic or coding worker.

Add specialist-worker fixtures for security scanning, schema/type consistency, diff-aware patching, diagnostics, validation, memory/index updates, orchestration, and release preparation. Security or dependency findings must block the affected commit or promotion until the applicable authority resolves them. A failed health or promotion check must preserve last-known-good state. A repeated identical retry must not count as a new strategy.

**Exit gate:** file-save, build-completion, failure, dependency, promotion/export, and reconnect events continue the task without another chat click; every continuation is durable and replayable; specialist handoffs reconcile against one shared contract; real failure context reaches repair; failed gates preserve last-known-good; and no model, worker, or UI message can substitute for security, validation, runtime, signing, or export evidence.

## M111 — Cost governance and adaptive resource control

Implement reservation, settlement, reconciliation, cost caps, token and request budgets, provider-reported or estimated usage, resource telemetry, explicit exhaustion outcomes, and cost-based degradation.

**Exit gate:** an executable fixture proves that usage is recorded, caps are enforced, unknown usage is reconciled, context or concurrency can be reduced safely, and budget exhaustion never becomes false completion or silent permission expansion.

## M112 — Agent-layer trust boundary and extension security

Implement pre-admission scanning for skills, MCP-compatible tools, plugins, workflow packages, and instruction-bearing content. Cover provenance, hashes, version drift, prompt injection, hidden capability escalation, secret access, undeclared destinations, revocation, quarantine, and policy admission.

**Exit gate:** clean, malicious, malformed, drifted, revoked, and undeclared-access fixtures produce the correct admission or quarantine decision with durable evidence, and no untrusted instruction can alter authority, policy, completion, or target scope.

## M113 — Context compaction and cache governance

Implement `ContextCachePolicy`, protected-context classes, compaction triggers, cache key compatibility, invalidation, privacy exclusion, telemetry disclosure, and causal lineage preservation across provider requests.

**Exit gate:** fixtures prove that compaction preserves constraints, cache reuse requires compatible identity, invalidation follows source or policy changes, cache hits are visible, and context overflow does not cause secret leakage or evidence loss.

## M114 — Android runtime integrity and honest coverage

Implement independent collection and validation for applicable Play Integrity, ANRs, startup/crash behavior, battery-sensitive behavior, Doze/background restrictions, permissions, device availability, and runtime-session identity.

**Exit gate:** applicable, unavailable, unsupported, stale, and failed signal fixtures are distinguished; no missing signal becomes a pass; and the Android completion report shows the exact coverage and evidence for each declared integrity requirement.

## M115 — Frontend–control-plane protocol and generated service adapter

Implement the authenticated command registry, typed response and error envelopes, command-to-use-case-to-authority-to-transaction mappings, projection snapshots, subscription replay, snapshot cutover, backpressure, stale-command handling, and generated Android service adapter.

**Exit gate:** executable fixtures prove every initial command kind, scope and authorization rejection, idempotency, stale revision behavior, typed error mapping, cancellation, timeout, reconnect, event-gap recovery, supervisor restart, SQLite rollback, optimistic-state separation, and generated Android service error normalization.

## M116 — Background continuity and interruption recovery
Implement orthogonal UI, host, device, provider, lease, and reconciliation dimensions plus the deterministic aggregate precedence defined by the continuity contract. Wire continuity transitions through the authoritative projection and event replay path, while preserving the existing product lifecycle and completion authorities. Cover UI closure, UI reconnect, supervisor restart, host reboot, sleep/hibernate, shutdown, device loss, provider/network outage, checkpoint resume, lease fencing, unknown-outcome reconciliation, and safe failure.
**Exit gate:** executable fixtures prove that eligible work continues without an open UI, concurrent interruption dimensions do not overwrite one another, interrupted work resumes only from durable state, stale sessions cannot advance current state, unknown outcomes reconcile before retry, unavailable device/provider conditions map to operationality/session evidence, continuity appears in `ProjectionSnapshot`, and last-known-good evidence is preserved.

## M117 — Local APK export provenance and delivery admission
Implement profile-bound local deployment export using `ExportVerificationRecord` with the `APKExportRecord` view, including artifact identity, packaging profile, source revision, checkpoint, source/destination file identities, request fingerprint, idempotency key, signing binding, validation and promotion decisions, reconciliation reference, failure evidence, destination identity, source/destination hashes, byte count, and copy state. Wire export state into the authoritative delivery projection. Preserve separate source/workspace, ZIP, and Git access as `SOURCE_ACCESS_ONLY`.
**Exit gate:** executable fixtures prove required APK delivery, optional declared AAB behavior, rejection of undeclared artifact kinds or external deployment destinations, `UNKNOWN → RECONCILING` copy recovery, source/destination hash equality, idempotent retry protection, signing/validation/promotion linkage, delivery projection visibility, and refusal to treat source access as deployment completion.

### M117 command-boundary closure (resolves open contract-gap work item from M6 §9)
The M6 partial closure exposed six command-payload fields but left the remaining 22 `ExportVerificationRecord` fields reachable only through durable observation, not the command boundary (dev-plan M6 §9/§10). M117 must close this: the `ArtifactExport` command response envelope MUST surface the canonical `ExportVerificationRecord` in full at the command boundary (artifact/destination/source file identity, hashes, byte count, lifecycle state, post-copy verification, policy decision, signing/validation/promotion binding, reconciliation/failure evidence) — not merely the six request-side payload fields. This is required by ADR-203 (provenance-complete export). The `command_payload_field_coverage` verifier check added in M6 must be extended to assert response-side record coverage so the command boundary cannot drift from the durable record. No second export record may be introduced; the command envelope references the single canonical `ExportVerificationRecord` owned by the `CanonicalSchemaRegistry`.

## M118 — Platform Capability System, Platform Build Skills, and Cross-Build Adversarial Fixtures

M118 implements build spec §79 and technical architecture §84 and locks ADR-206. It extends the existing `EnvironmentCapabilityPlanner` (M71), `ToolCapabilityGraph` (M70), `WorkspaceLeaseManager`, `ToolSessionRegistry`, and evidence dependency machinery. It must not create a second lifecycle, policy, evidence, preview, or completion authority, and it must not add a container, VM, WSL, or simulated substitute for the declared target platform.

| Work item | Acceptance condition |
|---|---|
| Environment capability records | `EnvironmentCapabilityRecord`, `PlatformCapabilityEntry`, `ValidationEnvironment`, and `BuildGateRecord` have `CanonicalSchemaRegistry` entries, version compatibility, and ledger persistence bound to revision and environment fingerprint |
| Target platform resolution | `TargetPlatformResolver` and `PlatformCapabilityRegistry` resolve the declared target before planning, record host and target as explicit fields, and reject undeclared targets |
| Cross-build admission gate | the `CrossCompilationAuthority` decision point admits `TARGET_BUILD` only on a proven toolchain and refuses runtime-validation claims from a non-matching host before execution |
| Native validation gate | the `NativeRuntimeValidationAuthority` gate requires a reserved `ValidationEnvironment` lease and bound evidence; absence reports `USER_REQUIRED`/`UNAVAILABLE`, never a simulated pass |
| Worker contract platform fields | the scheduler honors `requiredHostPlatforms`, `requiredTargetPlatforms`, `requiredArchitectures`, `requiredCapabilities`, `requiredValidationEnvironment`, `crossCompilationAllowed`, `nativeExecutionRequired`, and `evidenceRequirements` |
| Platform skills | `environment-preflight`, `environment-repair`, `windows-desktop-build`, `windows-runtime-validation`, `cross-platform-build-diagnostics`, and `android-toolchain` skill packages with declared capabilities, trigger conditions, permission-neutrality, and fixture sets |
| Hallucination-prevention fixtures | `TEST-PLAT-001` implements build spec §79.13 fixtures A–D plus target-mismatch, scheduling, lease-loss, and matrix-version fixtures; evidence recorded as `EV-PLAT-001` |
| Verifier conformance | contract-graph verifier checks: the six sealed platform capability clauses of BS §67.12, the ExtensionDeclarations of BS §37 and §52, the twelve-edge row for `CONTRACT.RUNTIME.PLATFORM_CAPABILITY`, and cross-document identity of ADR-206, M118, TA §84, `TEST-PLAT-001`, and `EV-PLAT-001` |

**Exit gate:** on a non-Windows host with the Windows toolchain present, a "build and validate" task produces a verified Windows artifact with `WINDOWS_RUNTIME = UNVERIFIED` and a durable `USER_REQUIRED`/`UNAVAILABLE` validation node carrying the continue/cannot-continue lists; a model or worker completion claim without target observation is durably rejected with the missing evidence cited; a revision or fingerprint change invalidates prior target evidence and re-closes the certification gate; the four-state invariant holds in the ledger for every executed task; independent work continued during the wait; and no fixture passes by simulation. Documentation graph certification is reported separately from runtime certification.

### M118 acceptance matrix

| Capability | Required proof |
|---|---|
| Four-state invariant | host, target, validation, and certification states are distinct ledger fields and are never collapsed into one build or completion result |
| Cross-build honesty | `ARTIFACT_BUILD = VERIFIED` and `WINDOWS_RUNTIME = UNVERIFIED` coexist in the same task record; aggregate status is at most `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS` |
| Deterministic classification | capability state changes only through observed preflight, an authorized repair, or an explicit user action — never model assertion |
| Evidence binding | target evidence validates only against the matching environment fingerprint, target platform, and source revision |
| Work splitting | independent host-platform work continues while the validation node waits; the wait is durable, cited, and resumable |
|| No substitute target | no container, VM, WSL, or simulated environment produces native-validation evidence |

## M119 — Platform Skill Registry Persistence and Fail-Closed Selection

M119 extends the existing `CONTRACT.RUNTIME.SKILL` (ADR-154, BS §23, TA §19.1) with durable platform skill package persistence and fail-closed selection against the `EnvironmentCapabilityRecord`. It extends the `EnvironmentCapabilityPlanner` (M118) and `DurableControlPlane` (M2) so that skill invocation records are revision- and fingerprint-bound, and that capability-bearing skills are denied admission when their required capabilities are not `AVAILABLE` or `REPAIRABLE` in the current environment record. It must not create a new authority; selection flows through the existing `ToolBroker`/`PolicyAuthority` admission path and the `EvidenceAuthority` binding and invalidation path (TA §84.3–§84.4).

|| Work item | Acceptance condition |
|---|---|---|
| Skill package persistence | `SkillPackage` has a `CanonicalSchemaRegistry` entry (TA §19.1) with version compatibility; packages persist through the M2 SQLite ledger as `SkillInvocationRecord` and `SkillAdmission` records |
| Fail-closed selection | `select_required_skills` resolves required skill ids against the registry and the `EnvironmentCapabilityRecord`; an admitted capability-bearing skill requires a matching `PlatformCapabilityState::Available` or `Repairable` record; absence reports `Blocked` or `NotFound`, never inferred success |
| Capability-bearing admission | a skill whose `required_capabilities` intersect a non-`Available`/`Repairable` capability in the environment record is blocked fail-closed before any tool call or instruction load |
| Trust and scan gating | unscanned packages (`ScanStatus::Pending`/`Scanning`) are `NotInvocable`; revoked packages (`TrustStatus::Revoked`) are `NotInvocable`; built-in packages are the M118 v1 set |
| Durable invocation records | `SkillInvocationRecord` persists invocation id, skill version, session, permissions granted, tool calls with policy outcomes, start/complete timestamps, and terminal status; restart reloads records from the ledger |
| Idempotency and versioning | re-saving the same package upserts; distinct versions coexist; `installedAt` and `lastUsedAt` are recorded |
| Evidence binding | `SkillInvocationRecord` and `BuildGateRecord` share a `environmentId` and `sourceRevision`; a fingerprint or revision change invalidates dependent invocation evidence through the existing evidence dependency graph (TA §23, BS §5.7.4) |
| Verifier conformance | contract-graph verifier checks: the twelve-edge row for `CONTRACT.RUNTIME.SKILL` and `CAP.ANDROID.SKILL_WORKFLOW` identity `M66`; the six platform capability clauses of BS §67.12 remain sealed; the `SkillPackage` schema fields appear in the registry entry |

**Exit gate:** on a non-Windows host, the M118 v1 built-in skill set loads exactly 6 packages; `environment-preflight` is admitted (no capabilities required); `windows-runtime-validation` is blocked fail-closed against the `WINDOWS_NATIVE_EXECUTION` = `UNAVAILABLE` capability result; a revoked or unscanned package is `NotInvocable`; invocation records survive supervisor restart with version pinning; and a changed environment fingerprint invalidates prior invocation evidence. Documentation graph certification is reported separately from runtime certification.

