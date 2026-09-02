# Nirman Architecture Decision Log

## Purpose

This document records significant product and engineering decisions for Nirman. It prevents important choices from disappearing into chat history and makes future changes deliberate. A decision may be revised when new evidence appears, but the reason for revision must be recorded.

**Canonical ownership:** The Build Spec owns product contracts, invariants, and capability/contract registries. The Technical Architecture owns implementation schemas, protocols, and module boundaries. The Development Plan owns sequencing, milestones, fixtures, and exit gates. The Decision Log owns accepted decisions, rationale, and supersession. The README is explanatory only. AGENTS defines agent operating constraints only. The verifier certifies documentation and semantic checks only; it is never a runtime authority.

## Decision Status Values

| Status | Meaning |
|---|---|
| Proposed | Under discussion and not yet implemented |
| Accepted | Approved direction for implementation |
| Deferred | Intentionally postponed until a later milestone |
| Superseded | Replaced by a newer decision |
| Rejected | Considered and not selected |

---

## ADR-001: Nirman is a desktop application, not a hosted development platform

**Status:** Accepted  
**Decision:** Nirman will be a Windows-first desktop application that manages local projects and local execution. It may call cloud AI providers when configured, but it will not require hosted code execution for the core workflow.

**Reasoning:** Local execution gives users ownership of source code, previews, builds, and credentials. It also aligns with the product’s core distinction: the application helps build applications on the user’s own computer.

**Trade-off:** Users must install and maintain local development tools. Nirman must therefore provide strong diagnostics, version management, and recovery behavior.

---

## ADR-002: Separate the desktop UI from the local control plane

**Status:** Accepted  
**Decision:** A background control-plane process will own task execution, worker processes, events, approvals, persistence, and recovery. The desktop UI will be a client of that control plane.

**Reasoning:** A UI process can close, crash, or be restarted. Autonomous tasks need a durable owner that can continue or recover independently.

**Trade-off:** The application has more process and IPC complexity than a single-process desktop tool. That complexity is necessary for reliable background execution.

---

## ADR-002A: One user-facing Nirman application with two internal Windows processes

**Status:** Accepted

**Decision:** Nirman is one user-facing Windows desktop product delivered as one installation. Its production implementation consists of two cooperating processes:

| Process | User-facing role | Lifecycle |
|---|---|---|
| `Nirman.exe` | Visible desktop application: chat, projects, editor, preview, tasks, settings, evidence, notifications | Started and managed as the user-facing application |
| `NirmanSupervisor.exe` | Headless autonomous runtime/control plane | Started, monitored, restarted, and stopped automatically; never independently operated by the user |

`NirmanSupervisor.exe` is an implementation/runtime component, not a second user-facing application. It must not expose a normal application window, require separate configuration, create an independent taskbar workflow, or require the user to launch it manually.

The Nirman installer must install and version both executables as one product installation. Starting Nirman must ensure that the compatible supervisor is running. Closing or minimizing `Nirman.exe` must not terminate an eligible autonomous task owned by the supervisor.

The supervisor may continue background execution while the Nirman window is minimized or closed, subject to task policy, resource limits, Windows lifecycle state, and explicit stop conditions. When the user returns, Nirman reconnects to the existing supervisor session and replays durable state and events.

User mental model: one Nirman application, not two applications.

**Reasoning:** Separating presentation from autonomous execution prevents UI crashes, restarts, and closure from destroying long-running work while keeping the product experience equivalent to a single desktop application.

**Trade-off:** The implementation has process/IPC complexity, but that complexity remains invisible to ordinary users. This becomes the canonical terminology all other documents reference.

---

## ADR-003: Use a local transactional database for task state

**Status:** Accepted  
**Decision:** Nirman will use SQLite as the authoritative local transactional store for tasks, workers, events, approvals, checkpoints, policies, and recovery records. Large logs and artifacts will remain in files referenced from the database. A storage substitution requires a new accepted decision, schema-parity evidence, migration evidence, and replay/recovery certification; it is not an implicit equivalent.

**Reasoning:** The application needs durable state, migrations, atomic claims, event sequence numbers, and restart recovery without requiring a cloud database.

**Trade-off:** Database corruption and migration handling become product responsibilities. The runtime must provide integrity checks and safe backups.

---

## ADR-004: Use structured events instead of UI-parsed text

**Status:** Accepted  
**Decision:** The control plane will emit typed, durable events. The UI will render those events instead of parsing model messages or terminal text to infer task state.

**Reasoning:** Typed events allow reliable reconnection, replay, automation, analytics, debugging, and consistent status rendering.

**Trade-off:** Every important operation needs an event schema and versioning policy.

---

## ADR-005: Use allow, ask, and deny permissions

**Status:** Accepted  
**Decision:** Every tool and sensitive operation will resolve to one of three outcomes: allow, ask, or deny. Rules may be scoped by command, path, worker role, task, network category, skill, or external tool.

**Reasoning:** A single global autonomy switch is too coarse. Users need safe automation for routine work while retaining control over credentials, external directories, destructive commands, publishing, and signing.

**Trade-off:** The policy engine and approval UX require careful design. Ambiguous policy behavior is more dangerous than requiring additional approvals.

---

## ADR-006: Isolate write-capable workers

**Status:** Accepted  
**Decision:** Every write-capable worker will use a dedicated Git worktree, copy-on-write workspace, or disposable isolated workspace. Parallel workers must not write to the same mutable directory.

**Reasoning:** Isolation prevents accidental cross-worker corruption and makes reconciliation and rollback possible.

**Trade-off:** Worktrees consume disk space and make dependency installation more expensive. The scheduler must manage cleanup and shared caches carefully.

---

## ADR-007: Reconcile before integrating parallel changes

**Status:** Accepted  
**Decision:** Worker changes must pass through a reconciliation stage before reaching the main workspace. Non-overlapping changes may be applied automatically, while overlapping changes require conflict analysis, validation, and possibly user approval.

**Reasoning:** Blindly copying worker files can silently overwrite behavior. A separate integration workspace provides a safe place to run checks.

**Trade-off:** Parallel development becomes slower when conflicts occur, but correctness and recoverability are more important than maximum concurrency.

---

## ADR-008: Start with one reliable worker before adding swarms

**Status:** Accepted  
**Decision:** The first autonomous release will prioritize one worker that can inspect, plan, edit, preview, test, repair, checkpoint, and summarize. Specialized workers and parallel swarms will be added only after this loop is reliable.

**Reasoning:** Multi-agent coordination multiplies failure modes. Without a reliable single-worker state model, more workers create more activity but not more dependable progress.

**Trade-off:** Advanced demonstrations are delayed, but the core product has a stronger foundation.

---

## ADR-009: Use specialized workers with least privilege

**Status:** Accepted  
**Decision:** Nirman will define the canonical role-based workers recorded in ADR-049: Primary Orchestrator, Repository Scout, Requirements Planner, Architecture Worker, UI Worker, Android Data and Integration Worker, Test and QA Worker, Debugging Worker, Security Worker, Visual QA Worker, Performance Worker, Documentation Worker, Release Worker, and Reconciliation Worker. Each worker receives only the tools, paths, model profile, workspace, and resource policy required for its role.

**Reasoning:** Specialization reduces context pollution, improves task focus, allows cheaper models for narrow tasks, and reduces the blast radius of mistakes.

**Trade-off:** The system needs worker contracts, handoffs, role configuration, and orchestration logic.

---

## ADR-010: Persist durable execution plans

**Status:** Accepted  
**Decision:** Long-running tasks will have a durable execution plan containing objectives, acceptance criteria, dependencies, completed steps, blocked steps, assumptions, and next actions.

**Reasoning:** Long tasks cannot depend on a transient context window. A plan also gives the user a way to inspect and correct the agent’s direction. The task should continue end to end without an arbitrary time-based lock.

**Trade-off:** Plans can become stale. Nirman must update them after important changes and show when the current plan differs from the user’s latest request.

---

## ADR-011: Treat context as an indexed retrieval problem

**Status:** Accepted  
**Decision:** Nirman will maintain a repository map containing files, symbols, dependencies, routes, scripts, and recent changes. The agent will retrieve relevant context within a visible token budget instead of sending the entire project on every turn.

**Reasoning:** Large projects cannot be handled reliably through repeated full-project prompts. Structured retrieval reduces cost and improves relevance.

**Trade-off:** The index can be incomplete or stale. The runtime must refresh it after edits and allow workers to request full files when necessary.

---

## ADR-012: Make long-running tasks resumable and end-to-end

**Status:** Accepted  
**Decision:** Nirman will support extended background tasks with checkpoints, adaptive resource management, pause, resume, retry, backtracking, and recovery. It will continue end to end by default rather than stopping at an arbitrary duration. It may stop only when a genuine safety, policy, environment, cancellation, or unrecoverable-failure condition is reached.

**Reasoning:** “Do not stop until complete” is the intended product behavior. Unlimited repetition of the same failed action can waste resources, repeat unsafe actions, or hide a missing requirement, so the correct behavior is persistent effort with adaptive resource management, changing strategies, backtracking, and clear escalation only when a genuine stop condition is reached.

**Trade-off:** Some tasks will still stop before completion when a genuine safety, policy, environment, cancellation, or unrecoverable-failure condition is reached. Users may configure explicit hard time or usage caps when they want them, but those caps are not the default.

---

## ADR-013: Use failure fingerprinting and strategy changes

**Status:** Accepted  
**Decision:** The runtime will fingerprint compiler errors, test failures, commands, patches, and validation results. Repeated failures will trigger context cleanup, diagnostic-worker delegation, model escalation, or user escalation rather than identical retries.

**Reasoning:** Repeating the same action is not problem solving. Recovery must change the strategy or gather new evidence.

**Trade-off:** Failure classification adds implementation work and may occasionally escalate too early. The system should make the decision visible and allow a deliberate retry.

---

## ADR-014: Make sandbox strength explicit

**Status:** Accepted  
**Decision:** Nirman will expose native Windows execution profiles ranging from trusted local execution to restricted process and disposable emulator-snapshot environments. Isolation is enforced with restricted tokens, Windows Job Objects, ACL-scoped workspaces, environment filtering, process-tree supervision, and resource quotas.

**Reasoning:** A path policy alone cannot contain every process or dependency. Different tasks have different trust levels and resource needs.

**Trade-off:** Stronger native isolation can reduce performance and increase setup complexity. Nirman will tune restricted-process boundaries, workspace ACLs, toolchain isolation, and emulator snapshots without adding another sandbox layer.

---

## ADR-015: Use a dedicated browser profile for automated testing

**Status:** Accepted  
**Decision:** Browser testing is an optional, external, auxiliary capability and never a validation path for the generated Android application. Android emulator and physical-device execution remain the only core validation surface. When enabled, browser testing will use a Nirman-managed disposable profile with no personal cookies, passwords, extensions, or downloads.

**Reasoning:** Automated browser work must not accidentally access or modify the user’s personal browser session.

**Trade-off:** Users must configure test authentication separately if an application requires it. Real-account testing is deferred to explicit user-controlled workflows.

---

## ADR-016: Tie preview state to project revisions

**Status:** Accepted  
**Decision:** Every preview process will be associated with a project revision and checkpoint. Rollback will restart or invalidate a preview when its revision is no longer current.

**Reasoning:** A preview that displays an old or partially restored project creates false confidence.

**Trade-off:** Some rollbacks require a full restart instead of instant hot reload. Correctness is more important than preserving preview continuity.

---

## ADR-017: Support project-specific toolchain versions

**Status:** Accepted  
**Decision:** Projects will declare compatible tool versions, and Nirman will resolve them through local version managers, portable installations, or configured paths, with per-project environment filtering, cache separation, process scopes, and toolchain bindings.

**Reasoning:** A single global Node.js, Java, Android SDK, Rust, or package-manager version cannot support every project reliably.

**Trade-off:** Environment resolution becomes a major subsystem. The MVP should begin with diagnostics and explicit configured paths, then add managed versions.

---

## ADR-018: Keep credentials outside project source

**Status:** Accepted  
**Decision:** API keys and signing secrets will be stored through the operating-system keychain or secure secret provider. Project files, logs, prompts, and exported source must contain references or placeholders, not raw secrets.

**Reasoning:** Generated code and Git repositories are not safe secret stores.

**Trade-off:** The product must explain credential scope and may require users to configure secrets again on another machine.

---

## ADR-019: Use provider-neutral interfaces

**Status:** Accepted  
**Decision:** Nirman will implement a normalized provider interface for text, tools, structured output, vision, streaming, cancellation, capability detection, fallback, and usage telemetry.

**Reasoning:** Users may configure different cloud providers, and task roles may require different model capabilities. ADR-207 restricts Nirman to cloud AI providers only.

**Trade-off:** Provider-specific features cannot always be represented perfectly. Unsupported capabilities must be reported explicitly.

---

## ADR-020: Defer publishing and release signing behind approval

**Status:** Accepted  
**Decision:** Local build and export may be automated under policy, but publishing, pushing, release signing, uploading, or submission to external services always requires explicit approval.

**Reasoning:** These actions create external side effects and may have financial, legal, or reputational consequences.

**Trade-off:** Fully unattended release pipelines are deferred until a separate enterprise-grade policy and audit model exists.

---

## ADR-021: Let the configured AI select the complete Android implementation

**Status:** Accepted  
**Decision:** The product core builds Android applications end to end from user instructions, screenshots, assets, existing project files, device requirements, and integrations. The user does not select a framework or template. The technology resolver may choose and combine Java, Kotlin, Android Views, Jetpack Compose, Expo/React Native, custom native modules, Gradle plugins, background services, device APIs, and mixed architectures according to the requirements and validation evidence. The Windows desktop application is the development host, not a generated target.

**Reasoning:** Android applications vary widely in UI technology, device integration, performance, background behavior, packaging, and native dependencies. The configured AI must select the implementation rather than forcing the user to understand the technology stack in advance.

**Trade-off:** The resolver, project synthesizer, environment manager, and validation system must support a much broader Android capability surface. This increases engineering complexity but is required for the product’s end-to-end promise.

---

## ADR-022: Deferred decisions

The following decisions remain intentionally open:

| Topic | Current position | Decision trigger |
|---|---|---|
| Native Windows isolation | Required foundation | Restricted tokens, ACLs, Job Objects, process supervision, resource quotas, and emulator snapshots |
| High-risk restricted-process profile | Future hardening profile | Untrusted dependency and malware workflow requirements |
| Cloud worker execution | Not required for core app | Demand for remote execution without violating local-first principles |
| Multi-device Android preview | Future capability | Stable single-device workflow and resource telemetry |
| Automatic commits | Optional and policy-controlled | Reliable checkpoint and review behavior |
| External tool protocol | MCP-compatible adapter or equivalent | First design, issue, documentation, and test integrations |
| Long-term memory | Bounded project memory | Evaluation of retrieval quality and privacy controls |
| Scheduled automation | Safe local tasks only | Background daemon and approval notifications are stable |

---

## ADR-023: Add Goal Mode as a first-class execution contract

**Status:** Accepted  
**Decision:** Nirman will support a Goal Mode with a durable completion condition, validation plan, resource budget, autonomy policy, progress state, and explicit stop conditions.

**Reasoning:** Long-horizon tasks should be evaluated against objective completion conditions rather than ending because a model response ended. A goal contract also allows recovery after restarts and worker handoffs.

**Trade-off:** The validation engine must become more capable, and users must define conditions clearly enough to evaluate.

---

## ADR-024: Background work must be non-blocking

**Status:** Accepted  
**Decision:** Background tasks will run under the control plane without blocking the UI or stealing user focus. The UI will reconnect to durable task streams and provide notifications for completion, failure, and approval.

**Reasoning:** Users should be able to continue working while a long task runs. A background task should behave as an independent local job, not as a modal chat request.

**Trade-off:** The product needs task navigation, notifications, resource sharing, and reconnectable event streams.

---

## ADR-025: Define lifecycle hooks as named product contracts

**Status:** Accepted  
**Decision:** Nirman will expose named session, task, agent-loop, permission, worker, workspace, context, runtime, and configuration hook events. Hooks may be blocking or non-blocking and must be policy-controlled.

**Reasoning:** A named event contract enables security checks, index updates, notifications, recovery, extensions, and integrations without coupling them to model text.

**Trade-off:** Hook ordering, idempotency, timeout, and failure behavior must be documented and tested.

---

## ADR-026: Scheduled automations are a first-class capability

**Status:** Accepted  
**Decision:** Nirman will support recurring local automations with persisted schedules, inherited policies, budgets, duplicate-run prevention, run history, and notifications.

**Reasoning:** Safe recurring tests, documentation refreshes, dependency checks, and reports should not require a new chat request every time.

**Trade-off:** Scheduling increases the importance of approval expiry, resource limits, and safe handling after the application or computer restarts.

---

## ADR-027: Use two checkpoint tiers

**Status:** Accepted  
**Decision:** Nirman will maintain file-level checkpoints for granular undo and task-level checkpoints for complete project recovery.

**Reasoning:** A single task-level checkpoint is too coarse for small edits, while file-level history alone cannot safely restore a multi-worker project state.

**Trade-off:** The checkpoint manager must track file hashes, project revisions, preview revisions, worker workspaces, and validation snapshots.

---

## ADR-028: Recovery must backtrack, not repeat

**Status:** Accepted  
**Decision:** When a strategy fails repeatedly or structurally, Nirman will restore the last known-good state and try a materially different approach, worker role, context mode, model profile, or validation strategy.

**Reasoning:** Repeating a failed command or patch consumes resources without increasing the probability of success. Backtracking creates a clean state for a new strategy.

**Trade-off:** Backtracking may discard partial work. The discarded state must remain available in an isolated recovery branch or checkpoint for inspection.

---

## ADR-029: Support dual context-scaling modes

**Status:** Accepted  
**Decision:** Nirman will support retrieval-index context for constrained providers and filtered near-full-repository context for providers with sufficient capacity.

**Reasoning:** Provider context capacity varies substantially, and repository-scale work sometimes benefits from broad context while other tasks require precise retrieval.

**Trade-off:** The context engine must estimate budgets, filter secrets and generated files, report included and excluded content, and safely fall back when a large-context request is too large.

---

## ADR-030: Add optional standardized external-tool compatibility

**Status:** Accepted  
**Decision:** Nirman will support an optional standardized external-tool extension layer. The internal Tool Gateway and policy engine remain authoritative for every external action.

**Reasoning:** External systems such as issue trackers, design tools, documentation sources, databases, and browser services should be connectable without hard-coding every integration into the core application.

**Trade-off:** External tools increase the attack surface and privacy complexity. Every connection needs capability discovery, scope, health checks, audit records, and approval policies.

---

## ADR-031: Chat launches tasks but does not own execution

**Status:** Accepted  
**Decision:** The chat interface starts a durable task, while the control plane owns execution, persistence, progress, recovery, and reconnection.

**Reasoning:** Users must be able to leave a task running, continue other work, and return without losing the task state.

**Trade-off:** The UI needs task navigation, event replay, notifications, and a visible background-task model.

---

## ADR-032: Use a durable task graph and expandable execution tree

**Status:** Accepted  
**Decision:** Every autonomous task will have a durable graph of goals, requirements, phases, dependencies, workers, commands, validation, approvals, checkpoints, recovery, and evidence. The UI will present that graph as an expandable execution tree.

**Reasoning:** A generic spinner hides what the agent is actually doing. A nested execution tree makes long-running work inspectable and debuggable.

**Trade-off:** The graph and event model require versioned schemas and careful handling of large tasks.

---

## ADR-033: Completion status must be evidence-backed

**Status:** Accepted  
**Decision:** A task or phase may be marked complete only when its declared evidence requirements pass. Model-generated summaries explain results but cannot serve as proof by themselves.

**Reasoning:** Autonomous systems need objective completion signals such as test results, builds, screenshots, security scans, artifacts, and approvals.

**Trade-off:** Some goals are difficult to verify automatically. Those goals must remain unverified, blocked, or require explicit user acceptance rather than being silently treated as complete.

---

## ADR-034: Use a standard autonomous validation pipeline

**Status:** Accepted  
**Decision:** For Android-target profiles, Nirman will validate through Android emulator or selected physical-device launch, focused Android checks, build or package validation, security/dependency/reliability checks, Android device/accessibility/visual QA, repair or backtracking, regression checks, and final goal evaluation. Browser validation is optional external auxiliary tooling for a declared non-Android surface and is never required or authoritative for generated Android application completion.

**Reasoning:** Compilation alone does not prove that an application works or that existing behavior was preserved. Android runtime behavior requires supervised emulator/device evidence, while auxiliary browser evidence must not be mistaken for Android execution evidence.

**Trade-off:** Validation increases runtime and resource consumption. Project profiles must distinguish required, optional, and unavailable checks.

---

## ADR-035: Request approvals at policy boundaries

**Status:** Accepted  
**Decision:** Routine reversible actions inside an approved workspace should not trigger repeated approval prompts. Protected-file access, risky dependencies, external services, credentials, destructive operations, publishing, signing, and scope expansion require a precise approval request.

**Reasoning:** Asking for every small step destroys autonomy, while unrestricted privileged actions are unsafe.

**Trade-off:** The policy engine must classify actions accurately and bind approvals to exact request fingerprints.

---

## ADR-036: Termination is explicit, bounded, and truthful

**Status:** Accepted  
**Decision:** A task continues end to end until its goal is complete. It terminates only when a required decision is reached, an explicit hard safety or policy limit is hit, a dangerous or unresponsive process must be stopped, the user cancels, the environment or provider is unavailable, or no safe recovery path remains. Ordinary time, token, cost, and usage thresholds are adaptive guardrails, not automatic completion locks.

**Reasoning:** Extended activity does not prove guaranteed completion, but a fixed duration lock would prevent legitimate end-to-end work. Nirman should persist through long tasks, adapt resource use, change strategies, and continue until completion or a genuine stop condition.

**Trade-off:** Some tasks will still stop and require user input when a genuine decision, safety boundary, unavailable capability, or unrecoverable failure is reached. The final result must explain the exact termination classification and remaining conditions. Users may configure hard time or usage caps when they explicitly want them, but those caps are not the default.

---

## ADR-037: Use a provider-neutral Model Gateway

**Status:** Accepted  
**Decision:** Nirman will normalize chat-completion, response-item, message-oriented, and custom provider protocols behind one internal Model Gateway. The gateway will preserve provider-specific raw data while exposing common events for text, tool calls, structured output, vision, streaming, cancellation, usage, request IDs, and errors.

**Reasoning:** Users need to configure custom base URLs and model IDs, including compatible cloud services and local runtimes, without changing the agent orchestrator for each provider.

**Trade-off:** Some providers expose capabilities that cannot be mapped perfectly. The settings page must show detected capabilities and unsupported features explicitly.

---

## ADR-038: Make AI settings profile-based and capability-tested

**Status:** Accepted  
**Decision:** The AI Settings page will store multiple provider profiles with protocol, base URL, secure key reference, model IDs, optional specialist models, default parameters, privacy policy, network policy, and capability status. A connection test must validate the selected endpoint and model instead of trusting a provider label.

**Reasoning:** Model providers differ in protocol features, context capacity, tool calling, structured output, vision, streaming, and cancellation behavior.

**Trade-off:** Capability probes add setup requests, but they prevent runtime failures caused by unsupported assumptions.

---

## ADR-039: Use a stable controller for self-development updates

**Status:** Accepted  
**Decision:** Nirman self-development will use a stable launcher/controller that starts, health-checks, promotes, and rolls back replaceable application versions. The running application will never overwrite its own loaded binaries directly.

**Reasoning:** A self-modifying application needs an independent recovery authority. A stable controller can restore the previous version when a candidate fails to start, migrate, connect to IPC, or pass health checks.

**Trade-off:** The product must maintain two version layers and an atomic active-version pointer.

---

## ADR-040: Self-development requires candidate validation before promotion

**Status:** Accepted  
**Decision:** Self-development changes must occur in an isolated worktree and pass static analysis, tests, provider fixtures, sandbox tests, recovery tests, candidate launch, health checks, smoke tasks, task replay, and compatibility checks before promotion.

**Reasoning:** Compilation is not sufficient evidence that an autonomous change is safe. The self-development loop can affect the control plane, provider runtime, database, permissions, and recovery behavior.

**Trade-off:** Self-development takes longer, but it protects the running application and preserves rollback.

---

## ADR-041: Token availability is not a default completion constraint

**Status:** Accepted  
**Decision:** Nirman will continue long-running goals across provider requests and context compactions without a default token or time completion lock. Usage is recorded for telemetry and provider health. The user may configure explicit hard caps, but ordinary thresholds trigger adaptation rather than automatic termination.

**Reasoning:** The user’s configured provider may have ample or unlimited allowance. The product should not prematurely stop an end-to-end task because of a generic internal budget.

**Trade-off:** Long tasks still require provider availability, context management, process protection, and honest stop conditions. Provider-imposed limits remain outside Nirman’s control.

---

## ADR-042: Make the control plane the autonomous runtime authority

**Status:** Accepted  
**Decision:** The control plane and stable supervisor own task graphs, worker leases, runtime ticks, policies, provider requests, tools, checkpoints, validation, recovery, evidence, memory, artifacts, and self-improvement. The desktop UI and model responses are clients or inputs, not authoritative state.

**Reasoning:** Complete autonomy requires continuity across provider requests, worker handoffs, UI restarts, process failures, and validation cycles.

**Trade-off:** The control plane becomes the most important subsystem and requires strong persistence, event ordering, migration, and recovery testing.

---

## ADR-043: Use a graduated recovery ladder

**Status:** Accepted  
**Decision:** Nirman will recover from failures in levels: transient retry, focused diagnostics, context/environment refresh, strategy change, checkpoint backtracking, model or worker escalation, specialist delegation, isolated alternative solution, decision request, and safe escalation.

**Reasoning:** A robust agent should continue autonomously when a safe new strategy exists, while avoiding repeated identical attempts.

**Trade-off:** Recovery takes more time and produces more state, but it materially improves long-horizon reliability.

---

## ADR-044: Measure verified progress, not activity

**Status:** Accepted  
**Decision:** Runtime quality will be measured using evidence-backed progress such as satisfied acceptance conditions, passing tests, reduced errors, valid artifacts, successful environment repair, and stable previews rather than request count or elapsed activity.

**Reasoning:** A task can consume many model requests without improving the project. The runtime must detect unproductive loops and change strategy.

**Trade-off:** Progress evaluation requires structured evidence and project-specific validation signals.

---

## ADR-045: Store episode records for self-evaluation

**Status:** Accepted  
**Decision:** Every task outcome will produce a privacy-filtered episode record containing plan, actions, workers, failures, recoveries, validation, resource telemetry, user corrections, and final classification.

**Reasoning:** Nirman cannot improve reliably without understanding which part of the development loop failed.

**Trade-off:** Episode storage requires retention, redaction, user inspection, and deletion controls.

---

## ADR-046: Self-improvement requires scoped proposals and measurable candidates

**Status:** Accepted  
**Decision:** Recurring validated failures may produce scoped improvement proposals with evidence, hypothesis, affected components, expected metrics, safety impact, test plan, and rollback plan. A proposal must become a candidate before changing runtime behavior.

**Reasoning:** A single failure or model suggestion is insufficient evidence for a permanent rule or runtime change.

**Trade-off:** Self-improvement becomes slower than unrestricted self-editing, but it remains measurable and reversible.

---

## ADR-047: Use canary promotion and automatic rollback

**Status:** Accepted  
**Decision:** Self-improvement candidates will pass targeted tests, broad regression fixtures, provider and sandbox tests, recovery tests, smoke tasks, and representative task replay before canary or promotion. Post-promotion degradation triggers scoped disablement or rollback.

**Reasoning:** Runtime improvements can create regressions in unrelated task classes. Canary and rollback protect active users and projects.

**Trade-off:** The system must maintain baselines, candidate versions, quality metrics, and a stable recovery controller.

---

## ADR-048: Add an explicit Unattended / Full Autonomy profile

**Status:** Accepted  
**Decision:** Goal Mode background tasks use a named project-scoped profile that allows routine reversible actions inside the workspace, including dependency installation, local commits, builds, preview restarts, and approved environment repair. External-directory access, raw credentials, destructive commands, operating-system changes, remote pushes, publishing, signing, and unapproved sensitive-data transmission remain denied or hard-gated.

**Reasoning:** Asking for routine project-local actions defeats unattended execution, while allowing privileged or irreversible actions would weaken safety.

**Trade-off:** The user must configure project privacy and network policy once, and the profile must remain visible and auditable.

---

## ADR-049: Maintain one canonical worker registry

**Status:** Accepted  
**Decision:** All documents and runtime components use one worker taxonomy: Primary Orchestrator, Repository Scout, Requirements Planner, Architecture Worker, UI Worker, Android Data and Integration Worker, Test and QA Worker, Debugging Worker, Security Worker, Visual QA Worker, Performance Worker, Documentation Worker, Release Worker, and Reconciliation Worker.

**Reasoning:** Multiple unaligned role lists create undefined workers, inconsistent permissions, and impossible registry tests. The data-layer role is named "Android Data and Integration Worker" so it cannot be mistaken for a separate server-side generator. The role builds the generated Android application's data layer, persistence, and outbound integrations; it never produces a server-side deployable.

**Trade-off:** Existing role labels must be migrated to the canonical names, but the registry becomes implementable and auditable. Any legacy data-layer label is superseded by "Android Data and Integration Worker" and must not appear in any document or runtime component.

This decision supersedes every earlier worker-role taxonomy. Legacy role names are historical only and MUST NOT appear in active product specifications, architecture, milestones, tests, runtime registries, or agent instructions.

---

## ADR-050: Use bounded worker nesting and interface agreements

**Status:** Accepted  
**Decision:** The orchestrator chooses swarm size from complexity, dependencies, file boundaries, target platforms, interface agreements, validation needs, and resources. Coupled frontend/backend work requires a shared interface agreement before parallel implementation. Worker nesting is limited to two levels by default.

**Reasoning:** Post-hoc reconciliation alone is insufficient for interdependent work, and unrestricted nesting makes ownership and recovery ambiguous.

**Trade-off:** Some tasks will use fewer workers or serialize work, but coordination quality improves.

---

## ADR-051: Make terminals persistent and Windows-shell explicit

**Status:** Accepted  
**Decision:** Workers use persistent terminal sessions with working directory, environment fingerprint, shell profile, process group, PTY/stdin policy, and rotating logs. Windows shell selection is explicit and recorded. Interactive prompts are classified and handled through the unattended prompt policy.

**Reasoning:** One-shot commands and silent interactive prompts can stall long-running work.

**Trade-off:** The terminal subsystem is more complex, but it supports real development workflows and recoverable unattended execution.

---

## ADR-052: Treat skills as scanned, versioned, permission-neutral packages

**Status:** Accepted  
**Decision:** Skills use a structured package schema with triggers, compatible workers, required tools, input/output schemas, permission requests, scan status, trust status, version, and rollback. Loading a skill never grants permissions; every tool call passes through the policy engine.

**Reasoning:** Instruction packages can introduce unsafe commands, prompt injection, or hidden network behavior if they are not treated like executable extensions.

**Trade-off:** Skill installation requires scanning and lifecycle management, but shared and user-created skills become safer and reproducible.

---

## ADR-053: Resume eligible tasks after reboot and suspend/resume

**Status:** Accepted  
**Decision:** Active unattended tasks register a per-user startup entry, resume after reboot, observe suspend/resume and hibernate transitions, request execution power protection where supported, and restore eligible processes, ports, emulators, and checkpoints. Pending approvals also appear in startup summaries and in-app queues when OS notifications are suppressed.

**Reasoning:** Durable state alone does not provide unattended execution if the control plane does not restart or sleep leaves processes stale.

**Trade-off:** The application needs Windows lifecycle integration and must visibly disclose power-management behavior.

---

## ADR-054: Use fair-share scheduling across projects

**Status:** Accepted  
**Decision:** The global worker pool uses weighted round-robin with priority aging, project minimum service opportunities, task urgency, validation deadlines, and resource eligibility.

**Reasoning:** Per-task concurrency limits do not prevent one project from starving another.

**Trade-off:** A high-priority project may not receive every available worker, but multi-project unattended use remains predictable.

---

## ADR-055: Scale long-horizon state incrementally

**Status:** Accepted  
**Decision:** Repository maps use shards and dependency fingerprints; validation computes affected tests and uses cached results plus regression sharding; checkpoints use retention, content-addressed compaction, and reference-aware pruning; Android tasks use profile-based quotas for JavaScript, native, emulator, physical-device, and combined build workflows; validation includes architectural-drift checks.

**Reasoning:** Large Android projects with JavaScript, native, emulator, device, and build artifacts can make full repository rebuilds, full-suite validation, and unbounded checkpoint storage impractical.

**Trade-off:** The runtime needs dependency graphs, cache invalidation, retention metadata, and periodic full verification.

---

## ADR-056: Present preview and execution evidence together

**Status:** Accepted  
**Decision:** The default workspace shows the running application preview beside a resizable execution surface containing the task graph, worker steps, terminal streams, checkpoints, approvals, validation evidence, and next action. Both surfaces share a project revision identifier.

**Reasoning:** A preview alone does not show what the agent is doing, while a task tree disconnected from the running result makes visual verification harder.

**Trade-off:** The UI requires a coordinated two-pane state model, but users can directly relate behavior to the work that produced it.

---

## ADR-057: Runtime authorities, not models, control execution

**Status:** Accepted  
**Decision:** The target is autonomous system recovery, not model authority. Models may propose plans, edits, tool calls, recovery strategies, and self-improvements, but deterministic lifecycle, permission, sandbox, storage, evidence, recovery, promotion, rollback, and termination authorities control what can execute and what counts as complete.

**Required recovery behavior:** For recoverable failures, the runtime retries, refreshes state, repairs the environment, changes strategy, restores a checkpoint, delegates diagnosis, degrades optional capabilities, or records a safe terminal failure. It must preserve the last known-good state and must not depend on uncommitted model memory.

**Non-delegable controls:** A model, worker, skill, hook, external tool, or UI event cannot grant permissions, bypass isolation, delete recovery state, mark a task complete without evidence, promote an unvalidated candidate, disable mandatory controls, or suppress a hard safety termination.

**Reasoning:** Unbounded model authority would make the system less recoverable and less trustworthy. Autonomous recovery provides the desired hands-off behavior while deterministic authorities preserve safety and correctness.

**Trade-off:** The runtime must implement more control-plane logic and fault-injection tests, but autonomous behavior becomes observable, reversible, and enforceable.

---

## ADR-058: Make Android the sole generated application target

**Status:** Accepted  
**Decision:** The product core generates every supported category of Android application end to end. The user supplies the goal, behavior, visual references, assets, and device requirements; the configured AI selects or composes the required Android technologies, creates the project, runs it, validates it, repairs it, and packages it as an installable APK, or an AAB only when the active PackagingProfile requires `APK_AND_AAB`. No framework or template choice is required from the user. The Windows desktop application is the development host and is not a generated target.

**Scope boundary:** Every project-generation request, visual input, preview, validation flow, toolchain, artifact, and autonomous workflow resolves to a dynamically synthesized Android project. Internal bootstraps are allowed as implementation details, but they are not user-facing product limitations.

**Reasoning:** Focusing on one target lets the runtime deeply support Android screens, navigation, permissions, offline behavior, notifications, emulator/device testing, Logcat, Gradle, signing boundaries, and device-specific validation rather than spreading reliability across unrelated platforms.

**Trade-off:** The application deliberately concentrates on Android development quality, emulator/device validation, native build tooling, artifact generation, and autonomous runtime depth instead of spreading implementation effort across unrelated target profiles.

---

## Decision Review Rules

Every major change to the master specification, technical architecture, security model, or execution permissions should add or update a decision record. Rejected alternatives should remain documented when they explain an important trade-off.

A decision should be reviewed when a milestone exposes a failed assumption, a security test fails, a new operating-system constraint appears, or the product scope changes materially.


---

## ADR-059: One instruction creates one autonomous Android session

**Status:** Accepted  
**Decision:** A user instruction plus optional screenshots, assets, existing project files, device requirements, and integrations creates one durable `AutonomousAndroidSession`. The session owns the application contract, visual specification, technology plan, task graph, workers, terminals, sandbox, preview, checkpoints, validation, recovery, artifacts, and completion state independently of the chat interface.

**Reasoning:** A chat response is too temporary to own long-horizon Android development. A durable session lets the system continue, recover, and resume without routine human intervention.

**Trade-off:** The control plane must persist more state and expose a reconnectable execution surface, but the user can start work once and return to a real result.

---

## ADR-060: Preview revision is authoritative for live Android state

**Status:** Accepted  
**Decision:** The emulator or connected-device preview and the execution tree share a `projectRevisionId` and `checkpointId`. A failed candidate cannot replace the last valid preview revision. Screenshots, Logcat, installation state, visual comparisons, and runtime errors belong to the revision that produced them.

**Reasoning:** A live preview is useful only when the user can trust which source state is running and can distinguish a broken candidate from the last known-good app.

**Trade-off:** Preview orchestration requires revision-aware install, reload, capture, and rollback behavior.

---

## ADR-061: Detect stalls through a progress ledger

**Status:** Accepted  
**Decision:** The runtime records progress through changed files, new evidence, preview movement, test transitions, worker handoffs, strategy changes, validated requirements, and artifact transitions. Repeated actions without meaningful progress trigger strategy change, context refresh, delegation, environment repair, checkpoint restore, technology change, or an isolated alternative.

**Reasoning:** Long-running autonomy fails when the system remains active but does not improve the project. Heartbeats and token usage alone are not evidence of progress.

**Trade-off:** The runtime needs progress fingerprints and strategy comparison, but it avoids silent infinite repair loops.

---

## ADR-062: Reconcile swarm outputs before preview or release

**Status:** Accepted  
**Decision:** Parallel workers use isolated workspaces and structured handoffs. A reconciliation worker integrates only validated outputs, resolves conflicts, runs integrated Android checks, updates the preview revision, and creates the next checkpoint.

**Reasoning:** Parallel work is valuable only when the integrated project remains coherent and testable.

**Trade-off:** Reconciliation adds an explicit integration stage, but prevents workers from corrupting the main project or producing contradictory Android implementations.

---

## ADR-063: Full Android capability coverage is an internal acceptance contract

**Status:** Accepted  
**Decision:** The system must validate AI-selected generation across JavaScript-driven Android, Java, Kotlin, Android Views, Jetpack Compose, mixed architectures, custom native modules, background services, WorkManager, notifications, camera and media, location and sensors, Bluetooth and NFC, offline-first storage, API-heavy applications, authentication and permissions, tablet and multi-orientation layouts, device-integrated applications, and APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` delivery. These are internal fixture categories, not user-facing templates.

**Reasoning:** The user should describe the application while the configured AI selects and composes the implementation technology.

**Trade-off:** Capability fixtures and toolchain coverage are more complex, but framework knowledge is not pushed onto the user.

---

## ADR-064: APK delivery is a completion gate; AAB is profile-declared

**Status:** Accepted  
**Decision:** Where Android packaging is required, the task cannot be complete without build success, artifact existence, checksum, artifact scan, installation or launch evidence, main-flow results, visual validation, permission behavior, and no unresolved fatal runtime errors. The artifact must link to the source revision and evidence ledger.

**Reasoning:** Source generation or compilation alone does not prove that an Android application works on a device.

**Trade-off:** Completion takes longer and requires emulator/device validation, but results are materially more trustworthy.


---

## ADR-065: Use canonical versioned runtime contracts

**Status:** Accepted  
**Decision:** Define versioned contracts for `AutonomousAndroidSession`, `AndroidApplicationContract`, `VisualSpecification`, `AndroidTechnologyPlan`, `TaskGraph`, `WorkerContract`, `TerminalSession`, `PreviewRevision`, `EvidenceRecord`, `RecoveryRecord`, `ArtifactRecord`, and `ProviderProfile`.

**Reasoning:** Autonomous execution becomes unreliable when the UI, model, workers, and persistence layer invent incompatible state shapes.

**Trade-off:** Contract design and migrations add implementation work, but all runtime state becomes inspectable, testable, and replayable.

---

## ADR-066: Use deterministic lifecycle authority

**Locks:** `CONTRACT.RUNTIME.AUTHORITY`

**Status:** Accepted  
**Decision:** The lifecycle authority owns session transitions from creation through planning, synthesis, implementation, preview, validation, recovery, packaging, completion, and safe terminal states. Models and workers may propose transitions but cannot commit them.

**Reasoning:** A model must never become the authority for what state the system is in or whether a task is complete.

**Trade-off:** The control plane becomes more complex, but recovery and fault injection become enforceable.

---

## ADR-067: Use renewable leases plus scoped operation capabilities

**Status:** Accepted  
**Decision:** Long-running Android sessions use renewable leases with heartbeat and progress checks. Sensitive actions use single-use capabilities bound to session, worker, workspace, revision, scope, policy, action type, and expiry.

**Reasoning:** A fixed short execution token is unsuitable for long-horizon work, while unlimited model authority is unsafe. Separating session continuity from sensitive-operation authorization provides both persistence and control.

**Trade-off:** The runtime must manage lease renewal and capability consumption, but long tasks can continue safely.

---

## ADR-068: Use Android-aware project ingestion and revision integrity

**Locks:** `CONTRACT.RUNTIME.WORKSPACE`

**Status:** Accepted  
**Decision:** Project ingestion understands Android and Gradle structures, resources, manifests, native modules, devices, generated outputs, secrets, signing material, and repository state. Reconciliation, preview installation, packaging, and promotion require current project and scope fingerprints.

**Reasoning:** Android projects contain generated files, device configuration, credentials, and build outputs that generic file discovery cannot safely treat as ordinary source.

**Trade-off:** Ingestion and fingerprinting are more expensive, but stale or external changes cannot be silently overwritten.

---

## ADR-069: Use a normalized multimodal provider gateway with a deterministic tool broker

**Status:** Accepted  
**Decision:** Normalize configured Chat Completions, Responses-style requests, screenshots, structured outputs, typed tool calls, tool results, streaming task events, cancellation, usage, and provider failures. Providers never receive direct filesystem, process, emulator, or credential access; tool calls pass through the deterministic broker.

**Reasoning:** Users need broad provider configuration and multimodal coding, but provider APIs and model outputs must not control local execution directly.

**Trade-off:** The gateway and broker require adapters and capability detection, but provider portability and security improve.

---

## ADR-070: Separate sandbox and process domains

**Status:** Accepted  
**Decision:** Keep the desktop shell, supervisor, workers, build processes, emulator/device manager, preview application, provider transport, and credential service in distinct permission domains.

**Reasoning:** Generated Android code and build tools must not inherit the user’s personal host privileges.

**Trade-off:** Process orchestration is more involved, but host, project, device, network, and credential boundaries remain enforceable.

---

## ADR-071: Separate model claims, runtime events, and evidence

**Locks:** `CONTRACT.RUNTIME.EVIDENCE`

**Status:** Accepted  
**Decision:** A model claim never completes a requirement. Completion requires evidence records produced by deterministic validation services and linked to a project revision, checkpoint, and artifact where applicable.

**Reasoning:** Natural-language claims are not proof of builds, device behavior, visual fidelity, security, or packaging.

**Trade-off:** More evidence storage and validation work is required, but final results are trustworthy and auditable.

---

## ADR-072: Use privacy-scoped memory and replayable task history

**Status:** Accepted  
**Decision:** Maintain separate session, project, runtime-improvement, credential, event, evidence, and replay boundaries. Memory entries carry source, confidence, scope, revision, retention, and deletion metadata. Credentials and signing keys are excluded.

**Reasoning:** Long-horizon autonomy needs memory and replay, but unrestricted memory would create privacy and security risks.

**Trade-off:** Memory governance requires metadata and user controls, but users can correct, delete, fork, and replay work safely.

---

## ADR-073: Make Windows host reliability part of product quality

**Status:** Accepted  
**Decision:** Require offline startup, atomic persistence, migrations, crash recovery, signed per-user installers, upgrade rollback, state preservation, virtualized large-project views, local editor assets, privacy-filtered logs, and memory-leak tests.

**Reasoning:** A local autonomous runtime is only useful if the host survives provider outages, restarts, upgrades, large projects, and partial failures.

**Trade-off:** Installer and lifecycle engineering becomes a first-class workstream rather than a final packaging task.


---

## ADR-158: Canonical AndroidConstructionContract

**Locks:** `CONTRACT.RUNTIME.AUTHORITY`

**Status:** Accepted

**Decision:** Nirman will create one versioned AndroidConstructionContract for every autonomous session. It is the canonical handoff between user intent, screenshots, requirements, technology selection, workers, preview, validation, and APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` production.

**Rationale:** A single validated contract prevents workers from inventing inconsistent requirements, allows replay and migration, separates user facts from model inferences, and provides a stable target for evidence-backed completion.

**Consequences:** The contract schema must be versioned, migrated, validated, and stored with the session. Technology selection remains autonomous and is recorded rather than exposed as a user-facing framework choice.

## ADR-159: Pure reducer as lifecycle authority

**Locks:** `CONTRACT.RUNTIME.AUTHORITY`

**Status:** Accepted

**Decision:** Durable Nirman session state is reconstructed by a pure reducer over validated runtime events. Side effects are executed by supervised command handlers.

**Rationale:** Pure state transitions enable deterministic replay, crash recovery, impossible-transition detection, and testable lifecycle behavior.

**Consequences:** UI code and model workers cannot mutate lifecycle state directly. Every state change requires a validated event and an authoritative reducer transition.

## ADR-160: ConstructionTransaction as the atomic autonomous unit

**Locks:** `CONTRACT.RUNTIME.AUTHORITY`

**Status:** Accepted

**Decision:** Mutations, dependency changes, toolchain repairs, preview promotion, signing, and artifact promotion use ConstructionTransaction with a checkpoint, base revision, policy decision, validation evidence, and commit/rollback result.

**Rationale:** Model output is only a proposal. Transactionality prevents partial changes, stale writes, and evidence-free promotion.

**Consequences:** The transaction manager and evidence authority are mandatory runtime components. Every committed revision can be traced to its transaction.

## ADR-161: Parallel proposals with serialized commit barriers

**Locks:** `CONTRACT.RUNTIME.WORKSPACE`

**Status:** Accepted

**Decision:** Read-only analysis, planning, indexing, diagnosis, visual QA, performance analysis, and independent tests may run in parallel. Conflicting writes, reconciliation, preview promotion, signing, and artifact promotion are serialized per project revision.

**Rationale:** This preserves swarm productivity without allowing nondeterministic concurrent mutation.

**Consequences:** Workers declare base revisions, touched paths, semantic symbols, dependencies, and expected outputs. Reconciliation is required for overlap or stale proposals.

## ADR-162: Renewable session leases plus single-use operation capabilities

**Locks:** `CONTRACT.RUNTIME.WORKSPACE`

**Status:** Accepted

**Decision:** Long-running sessions use renewable progress-aware leases. Sensitive operations use single-use capabilities bound to session, worker, operation, scope fingerprint, base revision, and policy context.

**Rationale:** A fixed short token cannot safely represent a long Android build, while unlimited authority is unsafe. The two-level model supports autonomy with bounded authority.

**Consequences:** Expired leases revoke workers and block new work. Capabilities are consumed before external side effects and are never persisted in plaintext.

## ADR-163: Android toolchain manifest and project lock

**Locks:** `CONTRACT.RUNTIME.SUPPLY_CHAIN`

**Status:** Accepted

**Decision:** JDK, Gradle, AGP, Kotlin, Compose, Android SDK, build tools, platform tools, NDK, CMake, ADB, emulator, and selected JavaScript/native tooling are resolved through an authoritative manifest and project lock.

**Rationale:** Host-installed tools and configuration create nondeterministic builds and difficult recovery.

**Consequences:** Tool versions, hashes, paths, licenses, compatibility, and environment variables must be validated before build or preview. Authorized repair may update the lock only at a checkpoint boundary.

## ADR-164: Language-neutral AndroidCodeIntelligence

**Locks:** `CONTRACT.RUNTIME.LOCALIZATION`

**Status:** Accepted

**Decision:** Nirman will use language adapters for Kotlin, Java, XML, manifests, Gradle, TypeScript/JavaScript, C/C++ native modules, configuration formats, SQL, and lockfiles.

**Rationale:** Android projects span multiple languages and technology plans; a single Windows-specific parser cannot be the universal architecture.

**Consequences:** Full semantic analysis is required before high-impact mutation. The graph tracks symbols, resources, permissions, navigation, dependencies, tests, devices, and affected artifacts.

## ADR-165: Structured mutation broker with validated whole-file fallback

**Locks:** `CONTRACT.RUNTIME.VERIFICATION`

**Status:** Accepted

**Decision:** Models never write directly to project files. Parser-aware or schema-aware mutations are preferred. Whole-file generation is allowed only in an isolated transaction followed by syntax, graph, build, test, and integrity validation.

**Rationale:** This balances mutation safety with the heterogeneous file formats required by all Android technology choices.

**Consequences:** Blind replacements and out-of-scope writes are rejected. The broker owns path, revision, file ownership, mutation budget, dependency, and evidence checks.

## ADR-166: Authenticated supervised provider bridge

**Locks:** `CONTRACT.RUNTIME.AUTHORITY`

**Status:** Accepted

**Decision:** The provider bridge is loopback-only, session-authenticated, protocol-versioned, capability-checked, health-supervised, and restartable. It may be implemented inside the Rust backend or as a separately supervised local process.

**Rationale:** The reference local-service pattern is useful, but adding an unnecessary runtime increases packaging and failure surface.

**Consequences:** Provider requests are normalized across supported protocols, logged without secrets, and bound to sessions, workers, privacy classifications, and tool policies.

## ADR-074: Android requirement and permission authority

**Status:** Accepted

**Decision:** Nirman will maintain an AndroidRequirementManifest that infers and validates SDK, ABI, manifest, permissions, services, resources, accessibility, localization, background behavior, and release requirements.

**Rationale:** Android capability failures and over-permissioning must be detected deterministically rather than left to model claims or late runtime crashes.

**Consequences:** Each requirement carries source, confidence, affected files, validation rule, status, and evidence. Missing or excessive permissions block promotion until repaired or explicitly governed.

## ADR-075: Android repair-pattern registry

**Status:** Accepted

**Decision:** Nirman will maintain a deterministic AndroidRepairRegistry for environment, dependency, source/build, runtime, visual, accessibility, emulator, ADB, packaging, and signing failures.

**Rationale:** Known failure fingerprints should be repaired consistently before expensive open-ended model reasoning is attempted.

**Consequences:** Patterns specify scope, preconditions, retry budget, checkpoint policy, validation, and evidence. Learned repairs require repeated independent validation before trust promotion.

## ADR-076: Revision-bound preview fallback hierarchy

**Status:** Accepted

**Decision:** PreviewCoordinator selects incremental emulator install, Compose reload, React Native/Expo refresh, APK reinstall, physical device execution, headless smoke test, or diagnostic preview according to the change and selected technology.

**Rationale:** No single preview mechanism works for every Android technology or change type.

**Consequences:** Every PreviewRevision is bound to source revision, artifact, device, API level, build variant, technology plan, and evidence. Stale preview cannot satisfy completion.

## ADR-077: Decision trace without hidden chain-of-thought

**Status:** Accepted

**Decision:** Nirman records concise decision summaries containing inputs, constraints, alternatives, selected action, policy checks, provider/model provenance, confidence, outcome, and evidence links. Hidden chain-of-thought is not stored or exposed.

**Rationale:** Users and developers need to understand important autonomous decisions without creating a sensitive reasoning transcript.

**Consequences:** The UI can explain technology selection, worker routing, repairs, provider changes, checkpoint restores, and preview choices using auditable summaries.

## ADR-078: Adaptive ResourceGovernor cannot weaken safety

**Status:** Accepted

**Decision:** ResourceGovernor may compact context, reduce concurrency, prune safe caches, stop redundant workers, select affected tests, defer nonessential checks, or choose an approved lighter provider. It may never bypass sandboxing, permission checks, evidence, signing, or artifact gates.

**Rationale:** Long-horizon autonomy requires adaptation, but resource pressure must not turn into unsafe execution or false completion.

**Consequences:** CPU, memory, disk, emulator, Gradle, provider, context, log, duration, and device budgets are monitored and recorded in environment evidence.

## ADR-079: Android data-layer resolution instead of fixed ORM

**Status:** Accepted

**Decision:** The AI and runtime resolve the data layer from requirements using Room/SQLite, direct SQLite, DataStore, encrypted storage, a justified alternative, network cache/synchronization, or a composed strategy.

**Rationale:** Android applications have different storage, offline, encryption, migration, and synchronization needs. A fixed ORM would contradict automatic technology selection.

**Consequences:** The selected strategy is recorded in AndroidConstructionContract and AndroidTechnologyPlan, includes migrations and corruption recovery, and cannot change without plan reconciliation.

## ADR-080: Honest safe terminal states

**Status:** Accepted

**Decision:** Nirman prioritizes autonomous recovery and renewable waiting, but supports explicit ProviderUnavailable, BlockedByPolicy, EnvironmentUnrecoverable, Cancelled, and SafelyFailed states.

**Rationale:** Endless mutation loops and hidden blockage are less safe than truthful recovery boundaries.

**Consequences:** Every safe terminal state includes last checkpoint, failure classification, attempted strategies, evidence, recommended resume/fork action, and a replayable history.

## ADR-081: Android-only generated target remains invariant

**Status:** Accepted

**Decision:** Sync-AI-derived Windows frameworks, Windows packaging, web generation, fixed framework keyword resolution, Roslyn/XAML requirements, and EF Core requirements are not added to Nirman’s generated-target contract.

**Rationale:** Nirman is permanently an Android-only autonomous application builder. Windows is the desktop host, not a generated application target.

**Consequences:** All requirements, toolchain, preview, repair, artifact, UX, and acceptance logic must resolve to Android projects and APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` artifacts.
## ADR-082: Integrated Android workflow coordinator

**Status:** Accepted

**Decision:** Nirman will use one durable `IntegratedAndroidWorkflowCoordinator` to connect prompt normalization, contract creation, preflight, technology selection, worker scheduling, transactions, build, preview, testing, quality review, recovery, packaging, and evidence promotion.

**Rationale:** A real coordinator provides a testable lifecycle boundary instead of relying on a large prompt or loosely connected modules.

**Consequences:** Every phase emits durable events and must be idempotent across restart and command replay.

## ADR-083: Preflight risk and feasibility gate

**Status:** Accepted

**Decision:** Every substantial session runs a deterministic preflight before expensive generation. It evaluates provider, toolchain, workspace, dependencies, devices, permissions, signing, storage, and validation capacity.

**Rationale:** Early blocker detection reduces wasted work and makes autonomous recovery more effective.

**Consequences:** Repairable issues may be handled automatically under policy; credentials, policy blocks, and unavailable required devices remain explicit states.

## ADR-084: Independent Android quality gate

**Status:** Accepted

**Decision:** Artifact promotion requires an independent `AndroidQualityGate` covering contract, architecture, build, security, dependencies, runtime, UI, accessibility, performance, tests, and release integrity.

**Rationale:** The worker that writes code must not be the sole authority that judges it.

**Consequences:** Findings are blocking, warning, or informational. A score or model assertion cannot replace evidence.

## ADR-085: Proactive failure-mode catalogue

**Status:** Accepted

**Decision:** Nirman will maintain a `FailureModeRegistry` with preventive checks and recovery strategies for Android toolchain, dependency, source, runtime, device, visual, accessibility, packaging, and signing failures.

**Rationale:** Known failure classes should be detected and repaired consistently before open-ended diagnosis.

**Consequences:** Every pattern has scope, retry policy, checkpoint behavior, stop condition, and evidence requirements. New patterns need independent validation before trust.

## ADR-086: Acceptance-test traceability

**Status:** Accepted

**Decision:** Every mandatory Android requirement must map to an acceptance criterion, executable test, selected device/profile, result, evidence, and artifact revision.

**Rationale:** Test quantity is less valuable than traceable proof of required behavior.

**Consequences:** Skipped, blocked, flaky, and not-applicable tests are represented honestly. Missing mandatory validation blocks completion.

## ADR-087: Architecture and contract drift detection

**Status:** Accepted

**Decision:** Nirman will compare each major project revision with the approved AndroidConstructionContract and AndroidTechnologyPlan.

**Rationale:** Autonomous systems can gradually lose alignment with the original requirements or architecture.

**Consequences:** Drift cannot be hidden by changing the contract in place. Contract changes require versioning, rationale, reconciliation, and revalidation.

## ADR-088: Runtime trace and dependency health intelligence

**Status:** Accepted

**Decision:** Nirman will provide structured runtime trace analysis and dependency health analysis for Logcat, ANRs, native crashes, install failures, permission failures, dependency conflicts, vulnerabilities, licenses, provenance, size impact, and lockfile drift.

**Rationale:** These are recurring sources of Android build and runtime failure and should feed deterministic repair and quality decisions.

**Consequences:** Trace data is redacted before persistence or provider submission. Dependency changes require transaction, restore, build, tests, security checks, and rollback evidence.

## ADR-089: Project handbook and release-intelligence report

**Status:** Accepted

**Decision:** Every managed project receives a concise validated handbook, and every promoted APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` receives a revision-bound release-intelligence report.

**Rationale:** Users need an understandable project record and trustworthy artifact metadata after autonomous construction.

**Consequences:** Documentation is generated from validated state and cannot claim support beyond retained test and evidence results.

## ADR-090: Metrics are evidence for routing, not authority

**Status:** Accepted

**Decision:** Worker, strategy, repair, and validation metrics may improve routing and resource allocation but cannot grant permissions or mark completion.

**Rationale:** Historical success is useful for optimization but is not a safety authority.

**Consequences:** Metrics include success, regression, rollback, time-to-evidence, handoff completeness, and false-positive rates. Security and artifact gates remain deterministic.

## ADR-091: Bounded structured reasoning

**Status:** Accepted

**Decision:** Prompt normalization, self-critique, risk prediction, logical consistency, alternative comparison, reflection, and strategy evaluation produce concise structured records. Hidden chain-of-thought is not stored or displayed.

**Rationale:** Nirman needs explainability without retaining sensitive internal reasoning transcripts.

**Consequences:** A decision record contains inputs, constraints, alternatives, selected action, policy checks, provenance, confidence, outcome, and evidence IDs.

## ADR-092: Native Windows isolation as the complete sandbox foundation

**Status:** Accepted

**Decision:** Nirman relies exclusively on native Windows isolation as its required execution model, using restricted tokens, Windows Job Objects, ACL-scoped workspaces, environment filtering, process-tree supervision, resource quotas, toolchain isolation, and disposable Android emulator snapshots. No additional isolation runtime is required.

**Rationale:** Nirman needs reliable Windows host integration with Android emulators, GPU acceleration, physical devices, and local toolchains without introducing an additional isolation-runtime dependency.

**Consequences:** Workspace, process, toolchain, policy, and emulator-snapshot authorities are the complete isolation foundation. External sandbox setup, image management, virtual networking, volume management, and related maintenance are outside Nirman.

## ADR-093: No unsupported capability-count claims

**Status:** Accepted

**Decision:** Nirman will not use module counts, mechanism counts, or implementation percentages as proof of support. Capability status is derived from executable fixtures, health checks, and retained evidence.

**Rationale:** Quantitative claims without functional proof mislead users and obscure real reliability.

**Consequences:** Documentation and UI must distinguish planned, implemented, validated, degraded, and unavailable capabilities.

## ADR-094: Android-only scope remains unchanged

**Status:** Accepted

**Decision:** Web generation, Windows application generation, PWA output, universal web-wrapper generation, Electron as the Nirman shell, and desktop application output are not added. Android remains the sole generated target; C#/.NET + WinUI 3 remains the desktop host architecture.

**Rationale:** The README’s implementation stack is not compatible with Nirman’s product boundary.

**Consequences:** All new workflow, quality, risk, intelligence, preview, toolchain, and artifact services must resolve to Android projects and APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` deliverables.
## ADR-095: Private internal reasoning with visible structured summaries

**Status:** Accepted

**Decision:** Nirman may use private model reasoning internally for planning, hypothesis generation, self-critique, diagnosis, alternative comparison, and strategy selection. It will not expose or persist verbatim hidden chain-of-thought. Instead, it will produce filtered, concise, structured reasoning summaries for the user.

**Rationale:** Internal reasoning can improve complex autonomous work, while verbatim reasoning transcripts can expose sensitive content, hidden instructions, private data, or unreliable intermediate thoughts. Users still need visibility during long-running sessions.

**Consequences:** A summarizer and filter become mandatory boundaries between model output and the UI, event store, worker handoffs, exports, and evidence system.

## ADR-096: ReasoningStreamEvent is separate from runtime authority

**Status:** Accepted

**Decision:** Nirman will stream `ReasoningStreamEvent` records for understanding, constraints, plans, alternatives, decisions, actions, observations, recovery, evidence, waiting, next steps, and completion. A reasoning event can explain a proposed action but cannot authorize a tool, mutation, permission, or artifact promotion.

**Rationale:** Users need real-time transparency, but explanatory text must not be confused with an executable command or proof of success.

**Consequences:** Every visible decision is paired with separate policy, execution, validation, and evidence events where applicable.

## ADR-097: Deterministic redaction before display and persistence

**Status:** Accepted

**Decision:** `ReasoningStreamFilter` will redact or withhold API keys, tokens, private keys, passwords, personal data, sensitive project content, complete source files, raw provider messages, hidden instructions, and sensitive paths before streaming, persistence, handoff, or export.

**Rationale:** The reasoning stream must remain useful without becoming a leakage channel.

**Consequences:** Unsafe summaries are replaced with safe generic status events. Redaction metadata is recorded without retaining the withheld content.

## ADR-098: Durable authenticated stream with replay

**Status:** Accepted

**Decision:** Reasoning events are persisted with monotonic per-session sequences and delivered through the authenticated local event channel. Clients reconnect from the last acknowledged sequence. Replay is side-effect free.

**Rationale:** A long autonomous session must remain understandable after minimization, disconnection, sleep, reboot, provider restart, or control-plane restart.

**Consequences:** Back-pressure cannot stop autonomous execution. Duplicate and out-of-order events are detected and corrected through sequence replay.

## ADR-099: Progressive reasoning presentation

**Status:** Accepted

**Decision:** Nirman will provide Calm, Inspect, and Developer presentation levels. All levels use the same filtered event stream, but differ in detail and navigation.

**Rationale:** Beginners need a simple progress view, while advanced users need decision, operation, evidence, and replay detail.

**Consequences:** Changing presentation affects only visibility, not execution, permissions, model routing, or policy outcomes.

## ADR-100: Honest streamed status semantics

**Status:** Accepted

**Decision:** Streamed status must distinguish working, waiting, recovering, blocked, stale, complete, and safely failed. Repeated events or an active spinner do not count as progress.

**Rationale:** Visible streaming must improve trust rather than disguise stalls or policy blocks.

**Consequences:** The stall detector and lifecycle reducer remain authoritative for progress. The stream reports their state and cannot manufacture progress.

## ADR-101: Provider delta normalization without raw forwarding

**Status:** Accepted

**Decision:** Provider-native streaming deltas are normalized by ModelGateway. The UI receives only approved structured reasoning, progress, tool-status, observation, and evidence events. Partial provider output cannot trigger a mutation or tool operation.

**Rationale:** Different providers expose different streaming formats and may emit incomplete or unsafe output.

**Consequences:** Complete structured responses must pass schema, policy, scope, and transaction validation before execution.
## ADR-102: Branding and visual assets are first-class Android requirements

**Status:** Accepted

**Decision:** When the user requests a logo, icon, splash screen, notification icon, illustration, branded color system, or visual identity, Nirman treats those assets as mandatory product requirements and includes them in the completion contract.

**Rationale:** A generated app is incomplete when its requested product identity is missing, generic, stale, or unintegrated even if the source code builds successfully.

**Consequences:** Asset planning, generation, integration, preview verification, artifact inspection, and evidence are required parts of the autonomous Android workflow.

## ADR-103: Dedicated BrandAssetWorker and versioned manifests

**Status:** Accepted

**Decision:** Nirman will use a scoped `BrandAssetWorker` and versioned `BrandManifest` and `AssetManifest` records for branding and visual assets.

**Rationale:** Asset work needs explicit ownership, provenance, regeneration history, impact analysis, and validation rather than being an optional side effect of a UI worker.

**Consequences:** The worker cannot modify unrelated source, change runtime authority, grant permissions, or mark completion. Each asset is linked to a source revision and construction transaction.

## ADR-104: Asset completion requires project, preview, and artifact proof

**Status:** Accepted

**Decision:** Asset completion requires validation in the Android project, current live preview, and built APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB`. Workspace presence alone is insufficient.

**Rationale:** An asset can exist on disk but be referenced incorrectly, omitted from packaging, stale in preview, or invalid at runtime.

**Consequences:** `AssetValidator`, `PreviewCoordinator`, and `ArtifactAssetInspector` are required. Missing, stale, invalid, unintegrated, or placeholder-only requested assets block final promotion.

## ADR-105: Provenance and reproducibility for generated assets

**Status:** Accepted

**Decision:** Nirman records asset intent, screenshot references, provider/model metadata, prompt hashes, optional seed, output content hash, visual validation, and regeneration history. A seed is not treated as proof of identical AI output.

**Rationale:** Asset provenance supports debugging, privacy review, regeneration, comparison, and trustworthy release reports.

**Consequences:** Raw prompts, sensitive user data, API keys, and private provider content remain filtered from ordinary logs and visible reasoning.

## ADR-106: Asset fallback must be explicit

**Status:** Accepted

**Decision:** Provider failure may use an approved alternate profile, cached content-addressed output, or local/vector fallback, but the system must record whether the fallback satisfies the user’s request.

**Rationale:** Temporary placeholders are useful for recovery but must not silently pass a branded release requirement.

**Consequences:** Placeholder-only output blocks completion when branding was requested. The user sees the fallback status through the structured reasoning stream.

## ADR-107: Branding changes are revisioned and impact-scoped

**Status:** Accepted

**Decision:** A branding change creates a new BrandManifest revision, regenerates affected assets, updates Android resources, refreshes preview, invalidates stale evidence, and reruns the asset gate.

**Rationale:** Branding changes should be fast and should not unnecessarily regenerate unrelated application logic or assets.

**Consequences:** Asset impact analysis and revision binding are required for preview and artifact promotion.
## ADR-108: Lock C#/.NET + WinUI 3 for the Windows application

**Status:** Accepted

**Decision:** Nirman uses C#/.NET with WinUI 3 and Windows App SDK for its Windows desktop application. XAML is the presentation language and WinUI 3 Fluent Design is the UI system. The desktop UI is presentation-only and communicates with the Rust/Tokio control plane through the authenticated SupervisorConnection protocol over named pipes.

**Rationale:** Nirman is a Windows-first native desktop application. WinUI 3 provides the native Windows application surface while Rust/Tokio remains responsible for deterministic autonomous execution, process supervision, policy, persistence, recovery, Android tooling, and evidence.

**Consequences:** Tauri, Electron, React, TypeScript, Vite, Tailwind, shadcn/ui, and WebView-based desktop-shell architecture are not part of Nirman’s implementation stack. They may exist only as dependencies of unrelated development tooling and must not become Nirman's host UI architecture.

## ADR-109: Rust and Tokio own the authoritative control plane

**Status:** Accepted

**Decision:** Rust with Tokio owns the authoritative control plane: lifecycle, scheduling, workers, leases, filesystem and process authority, policy, terminals, provider credentials, recovery, resource governance, Android execution, evidence, and artifact promotion. C#/.NET + WinUI 3 is the presentation/client layer.

**Rationale:** These responsibilities must survive UI failure and require deterministic concurrency, Windows APIs, process control, and secure local authority. WinUI 3 must remain a client and never become a second runtime authority.

**Consequences:** WinUI state is presentation-only. The C# UI and model cannot bypass Rust runtime authorities.

## ADR-110: SQLite is the execution ledger

**Status:** Accepted

**Decision:** SQLite is mandatory for durable execution state, including projects, sessions, tasks, workers, leases, events, approvals, checkpoints, recovery, providers, terminals, previews, devices, validation, evidence, artifacts, toolchains, decisions, and reasoning events. SQLx is preferred initially; rusqlite remains an evaluated alternative when isolated safely.

**Rationale:** Nirman needs transactional state, migrations, event sequences, crash recovery, and replay without a cloud database.

**Consequences:** Large logs, screenshots, diffs, patches, crash dumps, build output, and APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` files remain in the filesystem artifact store with content-hash references.

## ADR-111: Separate Nirman.exe from NirmanSupervisor.exe for durable autonomy

**Status:** Accepted

**Decision:** The first vertical slice may host the Rust control-plane modules in-process with the WinUI 3 application to reduce initial process complexity. The production durable-autonomy architecture separates the reconnectable `Nirman.exe` UI from `NirmanSupervisor.exe`, which owns long-running execution and recovery.

**Rationale:** Autonomous work must continue when the UI closes, crashes, or reconnects after Windows restart or sleep/resume.

**Consequences:** The supervisor requires authenticated IPC, protocol handshake, health monitoring, installation/update behavior, login startup, and SQLite recovery scanning.

## ADR-112: Native WinUI editor surface is the first editor

**Status:** Accepted

**Decision:** Nirman uses a native WinUI editor surface for the first editor implementation. AvalonEdit or an equivalent native editor surface may be evaluated later.

**Rationale:** The primary product is autonomous construction, preview, validation, and recovery. A native WinUI editor surface is sufficient for the first editor surface with lower integration overhead and no WebView2 dependency.

**Consequences:** Editor state is presentation-only; semantic intelligence remains in Rust and language-specific analyzers.

## ADR-113: Native WinUI terminal surface; Rust owns ConPTY

**Status:** Accepted

**Decision:** A native WinUI terminal surface is the terminal renderer. Rust `TerminalSupervisor` owns Windows ConPTY, shell profiles, process groups, input policy, output capture, cancellation, quotas, and recovery.

**Rationale:** A native Windows renderer must not own process authority or unrestricted shell access. The native WinUI terminal surface provides deterministic rendering with no WebView2 dependency.

**Consequences:** Terminal UI reconnects to durable terminal sessions and cannot forge command results or bypass policy.

## ADR-114: Nirman orchestrates externally managed Android toolchains

**Status:** Accepted

**Decision:** Nirman resolves, validates, isolates, and supervises JDK, Gradle, AGP, Kotlin, Java, Android SDK, ADB, emulator, NDK/CMake, and selected Node/Metro/Expo tooling. It does not replace these ecosystems.

**Rationale:** Android build and device tooling must remain compatible with the Android ecosystem while Nirman supplies orchestration, evidence, and recovery.

**Consequences:** Toolchain manifests, locks, health checks, environment snapshots, and authorized repair are required.

## ADR-115: Four-stage implementation order is mandatory

**Status:** Accepted

**Decision:** Nirman implementation proceeds through Foundation, Reliable Single-Worker Autonomy, Durable Autonomy, then Swarm/Self-Development. Swarm and self-development are blocked until the earlier stage gates pass.

**Rationale:** The broad roadmap cannot be implemented safely or validated as one simultaneous effort.

**Consequences:** The development plan must track stage gates separately from feature milestones.

## ADR-116: The UI is a reconnectable projection

**Status:** Accepted

**Decision:** The C#/.NET WinUI 3 client maintains only presentation state. Nirman.exe rebuilds its projection from SupervisorConnection snapshots and durable events after reconnect.

**Rationale:** UI-owned execution state is lost during crashes, restarts, and long-running background work.

**Consequences:** Client state cannot mark completion, authorize operations, alter policies, or promote artifacts.

## ADR-117: WinUI 3 communicates with Rust through SupervisorConnection

**Status:** Accepted

**Decision:** The WinUI 3 client communicates with the Rust control plane through the typed authenticated SupervisorConnection protocol. The first implementation may use in-process interop where the supervisor boundary is not yet extracted; the production architecture uses named-pipe IPC with NirmanSupervisor.exe.

**Rationale:** This preserves one authoritative runtime while allowing the desktop UI to evolve independently.

**Consequences:** No Tauri IPC, WebView IPC, or Node control-plane server is part of Nirman's architecture.
## ADR-118: Make AgentExecutionKernel a first-class runtime subsystem

**Status:** Accepted

**Decision:** Nirman will expose an AgentExecutionKernel between goal/task compilation and worker, skill, and tool execution.

**Rationale:** Planning, execution, observation, recovery, delegation, and validation must form one durable runtime loop rather than remain scattered across worker prompts.

**Consequences:** The kernel produces proposals and transitions, while policy, transaction, evidence, lifecycle, and artifact authorities remain non-delegable.

## ADR-119: Separate agent-loop state from worker-process lifecycle state

**Status:** Accepted

**Decision:** Nirman will maintain a reasoning/execution loop state machine separately from process lifecycle states. The loop includes observe, understand, plan, select, authorize, execute, observe-result, update, evaluate, continue, validate, recover, delegate, replan, and complete.

**Rationale:** A worker process may be alive while its reasoning loop is blocked, validating, recovering, or waiting for a decision.

**Consequences:** Both state machines require durable sequence numbers, impossible-transition checks, and replayable events.

## ADR-120: Use SkillRuntime for compatibility, composition, execution, and evidence

**Status:** Accepted

**Decision:** Skills will execute through SkillRuntime, which verifies discovery, trust, compatibility, inputs, context, tools, permissions, outputs, and evidence. Compatible Android skills may compose into a bounded directed acyclic workflow.

**Rationale:** A skill registry alone does not define safe execution or provenance.

**Consequences:** Loading or composing a skill never grants a permission. Every invocation creates SkillExecutionRecord.

## ADR-121: Use SwarmPlanner to decide parallelism

**Status:** Accepted

**Decision:** SwarmPlanner will analyze dependency, change-surface, validation, capability, workspace, device, provider, and resource constraints before selecting parallel work.

**Rationale:** Correct integration and evidence matter more than maximizing worker count.

**Consequences:** Some complex goals remain serialized when parallelism would increase conflict or validation risk.

## ADR-122: Represent each worker as a runtime-configured instance

**Status:** Accepted

**Decision:** Each worker instance is constructed from a role, AgentProfile, task contract, skills, tools, permissions, resources, context, workspace lease, parent, and recovery policy.

**Rationale:** Responsibility and operating behavior are separate concerns.

**Consequences:** Worker creation is bounded and cannot expand authority or scope.

## ADR-123: Formalize typed delegation and replacement operations

**Status:** Accepted

**Decision:** Nirman will formalize delegate, spawn, handoff, resume, cancel, replace, retry, escalate, and merge operations with typed inputs, outputs, lineage, and validation requirements.

**Rationale:** Long-running autonomy needs explicit worker replacement and recovery semantics.

**Consequences:** Unstructured worker-to-worker instructions cannot change the task graph or authority policy.

## ADR-124: Share typed knowledge through a controlled ledger and blackboard

**Status:** Accepted

**Decision:** Workers exchange scoped KnowledgeArtifacts through KnowledgeLedger and TaskBlackboard. Only authoritative services may commit decisions, mutate the graph, mark requirements complete, change policy, or promote artifacts.

**Rationale:** Shared mutable memory causes stale assumptions, context pollution, and conflicting writes.

**Consequences:** Every artifact has source, revision, confidence, scope, validity, and evidence.

## ADR-125: Use renewable WorkspaceLease records

**Status:** Accepted

**Decision:** Every isolated worktree or copy-on-write workspace requires a renewable lease with owner, parent checkpoint, heartbeat, expiration, cleanup, recovery, and stale-owner handling.

**Rationale:** Long-running swarms need protection against orphan workspaces, duplicate ownership, zombie builds, and stale writes.

**Consequences:** A stale lease cannot write until recovery verifies process and revision state.

## ADR-126: Model long-lived tools as ToolSessions

**Status:** Accepted

**Decision:** Terminals, ADB, emulators, debuggers, LSPs, and preview processes will be represented as reconnectable ToolSessions with ownership, scope, environment fingerprint, process group, heartbeat, input policy, and cleanup.

**Rationale:** Stateful tools outlive individual worker messages and sometimes the UI connection.

**Consequences:** Reconnect preserves scope; it never grants additional capabilities.

## ADR-127: Plan through a Tool Capability Graph and Environment Capability Planner

**Status:** Accepted

**Decision:** Nirman will map goals to required capabilities, skills, workers, tools, and environment prerequisites, then classify prerequisites as AVAILABLE, REPAIRABLE, USER_REQUIRED, or UNAVAILABLE before expensive work.

**Rationale:** Early capability planning prevents late discovery of impossible Android validation paths.

**Consequences:** Physical-device access, signing credentials, privileged permissions, and unavailable hardware may remain user-required.

## ADR-128: Make ValidationPlanner and mutation/regression analysis authoritative for test selection

**Status:** Accepted

**Decision:** ValidationPlanner and MutationRegressionAnalyzer will select focused or expanded Android checks using files, symbols, call/route/dependency graphs, requirements, risk, prior failures, project type, and device profiles.

**Rationale:** Running the same fixed test set after every change is inefficient and can miss affected behavior.

**Consequences:** A high-risk manifest, permission, data, navigation, native-module, or build change expands validation automatically.

## ADR-129: Use side-effect-free trajectory replay

**Status:** Accepted

**Decision:** TrajectoryReplayEngine will replay recorded observations, proposals, tool calls, results, state changes, and evidence references against new models, prompts, skills, schemas, or runtimes without touching real projects or sending external side effects.

**Rationale:** Replay is required for model, skill, prompt, runtime, and self-improvement regression testing.

**Consequences:** Replay results are clearly separate from production execution evidence.

## ADR-130: Provide Simulation/Dry-Run Mode

**Status:** Accepted

**Decision:** SimulationExecutor will predict workers, skills, files, commands, permissions, devices, tests, resources, and risks without mutation or execution.

**Rationale:** Users and engineers need to inspect a proposed plan and test runtime behavior safely.

**Consequences:** Predicted, simulated, observed, and verified statuses must never be conflated.

## ADR-131: Detect deadlocks and apply agent-level backpressure

**Status:** Accepted

**Decision:** DeadlockDetector will analyze task, worker, resource, approval, lease, and ToolSession cycles. BackpressureController will reserve scarce Android and provider resources and expose queue, priority, fairness, and waiting state.

**Rationale:** Autonomous swarms can stall without repeating the same failed action.

**Consequences:** The scheduler may reduce concurrency or reorder work rather than launch additional workers.

## ADR-132: Propagate cancellation through the complete execution tree

**Status:** Accepted

**Decision:** Cancellation will propagate from goal to tasks, workers, skills, ToolSessions, child processes, PTY, emulator actions, and pending provider requests with graceful, forced, cleanup, checkpoint, and rollback semantics.

**Rationale:** Partial cancellation leaves resource leaks, stale leases, and misleading task state.

**Consequences:** Every descendant must acknowledge cancellation or be forcibly terminated under policy.

## ADR-133: Support independent worker and skill pause/resume

**Status:** Accepted

**Decision:** Nirman will pause and resume individual workers and skills while preserving context, leases, ToolSessions, checkpoints, and unresolved questions.

**Rationale:** Long-running goals may contain independent work that should continue while one branch is paused.

**Consequences:** Paused branches remain visible and cannot silently expire without recovery handling.

## ADR-134: Represent ambiguity as structured Human Decision Nodes

**Status:** Accepted

**Decision:** Multiple valid Android architectures or recovery paths will be represented as DecisionNodes containing question, options, evidence, trade-offs, recommendation, impact, and resume conditions.

**Rationale:** A decision node is richer and more durable than an unstructured approval request.

**Consequences:** The task can resume from the selected option without reconstructing context from chat history.

## ADR-135: Track uncertainty and contradiction as evidence-bound state

**Status:** Accepted

**Decision:** Nirman will track KNOWN, PROBABLE, ASSUMED, UNKNOWN, CONTRADICTED, VERIFIED, and BLOCKED facts with source, confidence, scope, expiry, evidence, and next action.

**Rationale:** Long tasks accumulate stale assumptions and conflicting requirements.

**Consequences:** Contradictions create controlled decision revisions instead of silent last-write-wins behavior.

## ADR-136: Recompile plans when evidence invalidates them

**Status:** Accepted

**Decision:** PlanCompiler and Replanner will create revisions recording planRevision, supersedesPlan, reason, trigger evidence, affected nodes, and recovery or migration action.

**Rationale:** A long-horizon plan must adapt to environment, requirements, toolchain, worker, and validation changes.

**Consequences:** Completed side effects remain immutable and the new plan starts from verified state.

## ADR-137: Tier execution history

**Status:** Accepted

**Decision:** ExecutionHistoryManager will maintain hot, warm, cold, and archived history tiers with evidence-preserving compaction and retrieval.

**Rationale:** Multi-hour Android tasks can produce more events and artifacts than active memory can retain.

**Consequences:** Garbage collection cannot delete required evidence, active checkpoint parents, unresolved failure evidence, or artifact provenance.

## ADR-138: Score workers using validated outcomes

**Status:** Accepted

**Decision:** AgentQualityScorer will evaluate correctness, evidence quality, regression rate, repair success, unnecessary actions, tool/context efficiency, error rate, handoff quality, and recovery quality.

**Rationale:** Model and worker routing should learn from validated results rather than configuration alone.

**Consequences:** Scores are advisory routing signals and cannot override policy or evidence authorities.

## ADR-139: Require end-to-end autonomous-runtime certification

**Status:** Accepted

**Decision:** Expanded swarm and self-development runtime capabilities may be promoted only after a long-running Android fixture passes dynamic allocation, skill composition, tool sessions, failure recovery, replanning, device validation, APK delivery; AAB only when the active PackagingProfile requires `APK_AND_AAB` packaging, traceability, replay, dry-run, cancellation, deadlock, and history-compaction tests.

**Rationale:** Capability claims require executable evidence, not module counts or architectural intent.

**Consequences:** The single-worker and durable-supervisor gates remain mandatory prerequisites for expanded autonomy.

## ADR-140: Classify every memory write and require source evidence

**Locks:** `CONTRACT.RUNTIME.MEMORY`

**Status:** Accepted

**Decision:** Memory records will be typed as DECISION, CONSTRAINT, FACT, FAILURE, or ARTIFACT, and MemoryWriter will reject any record whose `sourceEventIds` is empty.

**Rationale:** Untyped memory accumulates model speculation as if it were established fact. Requiring a source event makes it structurally impossible for a model claim to become memory.

**Consequences:** Memory volume is lower and provenance is queryable. Every memory-producing path must first emit a validated event.

## ADR-141: Never evict constraints or locked decisions for token budget

**Locks:** `CONTRACT.RUNTIME.CONTEXT`

**Status:** Accepted

**Decision:** ContextAssembler will reserve token budget for active constraints and locked decisions before selecting file content, and will reduce file content rather than constraint content when the budget is exceeded.

**Rationale:** Long-horizon failures are dominated by the agent forgetting a rule it was given earlier. File content is recoverable by re-reading; a forgotten constraint produces silent contract violation.

**Consequences:** Very large constraint sets reduce available file context, which forces retrieval mode earlier and makes constraint growth visible as a planning cost.

## ADR-142: Workers coordinate but hold no authority over each other

**Locks:** `CONTRACT.RUNTIME.RESERVATION`

**Status:** Accepted

**Decision:** Workers may publish reservations, exchange knowledge artifacts, and request handoffs, but may never grant permissions, approve evidence, mark work complete, or override an authority decision.

**Rationale:** Peer-granted authority allows a single incorrect worker to launder its own claims through another worker and defeat the evidence model.

**Consequences:** All arbitration is centralized in the deterministic runtime, which becomes a throughput bottleneck by design.

## ADR-143: Reserve semantic surfaces and invalidate stale contracts

**Locks:** `CONTRACT.RUNTIME.RESERVATION`

**Status:** Accepted

**Decision:** Workers will reserve symbols, routes, schema tables, resources, permissions, and build configuration before mutating them, and any change to a surface will invalidate every `read_stable` reservation on it, marking dependent work unvalidated.

**Rationale:** File-level leases permit semantically incompatible parallel changes — one worker renaming a field while another writes code against the old name. Both workers succeed locally and the merged build fails.

**Consequences:** Parallel work is more constrained and some proposals are rejected at the commit barrier and must be revalidated, which is preferred over merging a broken result.

## ADR-144: Treat user edits as authoritative and never overwrite them

**Locks:** `CONTRACT.RUNTIME.RECONCILIATION`

**Status:** Accepted

**Decision:** File changes will be classified by origin as RUNTIME, USER, EXTERNAL, or GENERATED using mutation fingerprints rather than timestamps. USER and EXTERNAL changes to reserved surfaces pause mutation, invalidate affected evidence, and adopt the user content as the new baseline.

**Rationale:** Silently reverting a user's edit is the most destructive failure an autonomous editor can commit, and validation predating a user edit is not evidence about the current code.

**Consequences:** Concurrent editing costs revalidation cycles, and the runtime must maintain content fingerprints for every mutation it performs.

## ADR-145: Apply runtime directives at decision boundaries with bounded authority

**Locks:** `CONTRACT.RUNTIME.DIRECTIVE`

**Status:** Accepted

**Decision:** User directives will be admitted mid-run, queued, and applied only at kernel decision points. A directive may constrain, reprioritize, forbid, require, refocus, or halt a surface, but may never raise a permission ceiling, bypass an evidence requirement, disable a policy gate, or mark a requirement complete.

**Rationale:** Restarting a multi-hour Android build to correct one instruction is unacceptable, but allowing arbitrary mid-run instructions to alter authority would make the evidence model advisory.

**Consequences:** Directives take effect with bounded latency rather than immediately, and the runtime must classify every in-flight step as unchanged, invalidated, or abandoned.

## ADR-146: Require deterministic stateful scenarios with declared seed provenance

**Locks:** `CONTRACT.RUNTIME.E2E`

**Status:** Accepted

**Decision:** Functional requirements will be verified by deterministic end-to-end scenarios covering cold start, authenticated flow, data persistence across process death, navigation depth, configuration change, permission grant and deny, offline behavior, and system-initiated death. Seed data will be established through the application's own data layer with recorded provenance, and non-deterministic scenarios will be excluded from completion evidence.

**Rationale:** Launching an app and screenshotting the first screen proves almost nothing about a stateful Android application. Flaky scenarios treated as passing are worse than absent scenarios because they manufacture false confidence.

**Consequences:** Verification is substantially more expensive, and stabilizing flaky scenarios becomes prerequisite work rather than optional cleanup.

## ADR-147: Localize regressions before repairing and confine repair to the cause

**Locks:** `CONTRACT.RUNTIME.LOCALIZATION`

**Status:** Accepted

**Decision:** When previously passing validation fails, the runtime will localize the cause using the impact graph, then failure-signature correlation, then checkpoint bisection, and will confine repair mutations to the identified cause surface. An unlocalized regression escalates instead of triggering broad regeneration.

**Rationale:** Broad regeneration in response to a regression destroys validated work, hides the actual defect, and can convert one failure into many.

**Consequences:** Localization consumes time before repair begins, and checkpoint retention becomes a functional requirement rather than a convenience.

## ADR-148: Verify inside the loop and reject vacuous assertions

**Locks:** `CONTRACT.RUNTIME.VERIFICATION`

**Status:** Accepted

**Decision:** Compiler diagnostics, lint, and incremental compilation will run after each structured mutation before dependent work proceeds. Behavioral requirements will have assertions authored before implementation, with later-authored assertions marked `post_hoc`. Critical-logic assertion sets must be proven to fail against an injected fault or be rejected as vacuous.

**Rationale:** Terminal-only validation lets errors accumulate until the cause is unrecoverable, and assertions written after a passing implementation tend to encode whatever the implementation already does.

**Consequences:** Per-mutation cost rises and generation is slower, in exchange for defects surfacing at the mutation that caused them.

## ADR-149: Verify the generated application and its supply chain, not only the host

**Locks:** `CONTRACT.RUNTIME.SUPPLY_CHAIN`

**Status:** Accepted

**Decision:** Before packaging, the runtime will scan generated application code and the merged manifest for insecure patterns, resolve every dependency to an exact version with an integrity hash, flag names resembling known packages, and produce a complete SBOM binding artifact checksum to revision and toolchain. Findings must be blocking or accepted with a recorded reason.

**Rationale:** Host sandboxing protects the developer machine but says nothing about whether the produced Android application is secure or whether its dependencies are trustworthy.

**Consequences:** Packaging gains a blocking gate, and artifacts without complete provenance cannot be promoted.

## ADR-150: Report multi-device coverage explicitly and treat divergence as a defect

**Locks:** `CONTRACT.RUNTIME.DEVICE_MATRIX`

**Status:** Accepted

**Decision:** Scenarios will be distributed across a declared device matrix. A run requires the primary device; unavailable secondary devices produce declared coverage gaps rather than implicit passes. A scenario that passes on one device and fails on another is classified as a defect unless cited evidence shows the failure originates in the device or vendor.

**Rationale:** Verification on a single emulator overstates confidence, and attributing device-specific failures to "device noise" is the standard way real Android defects are dismissed.

**Consequences:** Coverage reporting becomes more complex and often reports partial coverage, which is the honest result. Emulator capacity becomes a planning constraint.

## ADR-151: Disable external network triggers by default and cap their authority

**Locks:** `CONTRACT.RUNTIME.TRIGGER`

**Status:** Accepted

**Decision:** Externally originated work requests will pass through an authenticated gateway with a per-trigger permission ceiling, rate limit, and audit record. Webhook-sourced triggers are disabled at registration and open no listening network surface until explicitly enabled. A trigger may create tasks but never grant permissions, approve decisions, or promote artifacts.

**Rationale:** An unauthenticated path that starts autonomous code generation is a remote execution surface. Default-off with an explicit enablement decision is the only defensible posture.

**Consequences:** Automated integration requires deliberate configuration, and every firing carries audit overhead.

## ADR-152: Provide operator-grade runtime inspection without exposing private reasoning

**Locks:** `CONTRACT.RUNTIME.DEBUGGER`

**Status:** Accepted

**Decision:** A read-only debugger will expose kernel state, active plan, constraints, context package manifests, tool calls and results, held reservations and leases, evidence slices, recovery position, and resource reservations, for both live and completed sessions. It will have no access path to private model reasoning tokens and no mutation capability except pause and resume at decision boundaries.

**Rationale:** An autonomous runtime that cannot be inspected cannot be trusted or debugged, but exposing raw reasoning would violate the established privacy boundary and encourage users to treat speculation as fact.

**Consequences:** The event ledger must be complete enough to reconstruct runtime state, which raises persistence requirements.

## ADR-153: Estimate from measured history and label unprofiled operations honestly

**Locks:** `CONTRACT.RUNTIME.PROFILING`

**Status:** Accepted

**Decision:** The supervisor will measure duration, peak memory, CPU, and disk delta for each Gradle build, emulator boot, instrumentation run, packaging step, analysis pass, and provider call, keyed by project and host fingerprint. Plan cost will be estimated from these profiles, and operation classes below a minimum sample count will report `unprofiled` rather than a fabricated estimate.

**Rationale:** Capacity decisions made from guesses cause thrashing on constrained hosts, and a fabricated estimate is worse than an admitted absence of data.

**Consequences:** Early runs on a new project or host operate with sparse data and must declare lower confidence.

## ADR-154: Pin skill versions for the duration of an active session

**Locks:** `CONTRACT.RUNTIME.SKILL`

**Status:** Accepted

**Decision:** When a session binds a skill, that skill's version is pinned for the session's duration. A skill update installed mid-session applies only to sessions started afterward, and a pinned version remains resolvable until every session holding it completes.

**Rationale:** Changing a skill's instructions underneath a running long-horizon task makes the run irreproducible and can invalidate earlier work in ways the replay engine cannot explain.

**Consequences:** Skill versions must be retained while referenced, and urgent skill fixes do not reach in-flight sessions.

## ADR-155: Isolate project memory and anonymize cross-project learning

**Locks:** `CONTRACT.RUNTIME.MEMORY`

**Status:** Accepted

**Decision:** Project memory will be queryable only within its own project scope, enforced by mandatory project scoping at query level. Runtime-improvement memory may span projects only in anonymized form stored in a separate table with no path, identifier, or source-content columns.

**Rationale:** Leaking one project's architecture, conventions, or code into another is both a correctness failure and a confidentiality failure, and cross-project learning is only safe when it cannot carry identifiable content.

**Consequences:** Useful concrete patterns cannot transfer between projects, and learning transfer is limited to abstract failure and compatibility signals.

## ADR-156: Permit speculative candidate branches only under declared conditions

**Locks:** `CONTRACT.RUNTIME.SPECULATION`

**Status:** Accepted

**Decision:** The runtime may pursue multiple candidate approaches only when the task has a declared uncertainty, host capacity permits the additional cost, and candidates are comparable under an identical validation plan. Each candidate runs in an isolated workspace with its own revision lineage. Selection is decided by validation evidence; ties or universal failure escalate rather than selecting arbitrarily. Losing candidates contribute failure signatures but never code.

**Rationale:** Trying several approaches raises quality on genuinely uncertain work, but unconditional speculation multiplies cost and creates ambiguous evidence about which candidate a result came from.

**Consequences:** Speculation is rare and explicitly justified, and workspace isolation infrastructure is required before it can be enabled.

## ADR-157: Verify runtime invariants from the event ledger as a release gate

**Locks:** `CONTRACT.RUNTIME.INVARIANTS`

**Status:** Accepted

**Decision:** The ten runtime invariants — authority, evidence, provenance, reservation, freshness, constraint, isolation, honesty, recoverability, and ceiling — will be verified by replaying a completed session's event ledger and reporting each violation with its violating event. A release whose certification fixture produces any invariant violation must not be promoted.

**Rationale:** Invariants asserted only in documentation are unenforced. Verifying them mechanically from the ledger converts architectural intent into a testable release gate and prevents capability claims from outrunning behavior.

**Consequences:** The event ledger must record enough detail to prove each invariant, and certification requires a long-running Android fixture rather than unit tests alone.

## ADR-167: Drive execution from a recorded reasoning cycle with cited selection basis

**Locks:** `CONTRACT.RUNTIME.REASONING`

**Status:** Accepted

**Decision:** Autonomous work will be driven by an explicit reasoning cycle — observe, understand, hypothesize, strategize, select, authorize, execute, observe, reflect, update, decide — and each selection will emit a structured ReasoningArtifact whose selectionBasis cites evidence, constraints, failure signatures, or policy. An artifact with an empty selectionBasis is rejected at write.

**Rationale:** Without a recorded basis, strategy selection is unauditable and the runtime cannot distinguish a decision grounded in evidence from a plausible-sounding guess. Requiring a citation makes ungrounded selection structurally impossible rather than merely discouraged.

**Consequences:** Every cycle carries a persistence cost, and reasoning that cannot cite anything cannot proceed — which is the intended constraint, not a limitation.

## ADR-168: Persist structured reasoning artifacts and never verbatim private reasoning

**Locks:** `CONTRACT.RUNTIME.REASONING`

**Status:** Accepted

**Decision:** Private model reasoning will remain transient and will never be persisted, exposed, replayed, or cited as evidence or authority. The runtime will instead retain structured artifacts: objectives, assumptions, alternatives considered, selected strategy, cited basis, confidence, uncertainties, expected effect, hypotheses, and reflections.

**Rationale:** The engineering value of reasoning memory is knowing why a strategy was chosen and what was ruled out. That value is captured by structured records. Retaining raw hidden reasoning would make speculation durable, invite users to read it as fact, and violate the established §49 boundary.

**Consequences:** Reasoning is auditable without a reasoning transcript existing anywhere in the system. Debugging relies on structured records rather than reading the model's stream of thought.

## ADR-169: Make every autonomous capability agent-invocable and discoverable at runtime

**Locks:** `CONTRACT.RUNTIME.REASONING`

**Status:** Accepted

**Decision:** Every autonomous capability — skills, tools, workers, swarms, sessions, analysis, packaging — will be invocable by the agent through a capability layer with runtime discovery. The user interface may request goals, issue directives, and observe, but will not own or trigger capabilities. Discovery returns descriptors and grants nothing; permissions are evaluated at invocation.

**Rationale:** If the interface owns capability triggering, the agent is a text generator wired to buttons and every new capability requires agent changes. Runtime discovery makes the system extensible: a newly registered skill becomes usable without modifying the reasoning engine.

**Consequences:** The capability registry becomes a required runtime component, and every capability must declare schemas, permissions, validation, evidence kinds, and rollback behavior to be discoverable.

## ADR-170: Bound recursive delegation by capability and resource ceilings

**Locks:** `CONTRACT.RUNTIME.REASONING`

**Status:** Accepted

**Decision:** An agent may instantiate child agents, subject to two invariants enforced at grant time: a child capability ceiling is a subset of its parent's ceiling, and a child resource budget never exceeds the parent's remaining budget after outstanding sibling grants. Depth, fan-out, time budget, and workspace scope are also bounded, and revoking a parent grant cascades to every descendant.

**Rationale:** Recursive delegation is how a swarm scales, and it is also how authority leaks and how a host is exhausted. Expressing the bounds as set containment and arithmetic inequality makes them mechanically checkable rather than matters of judgment.

**Consequences:** Delegation requests are denied rather than degraded when a ceiling is exceeded, and the outstanding-budget sum must be recomputed at each issue because sibling grants change it.

## ADR-171: Let the agent select execution mode within policy bounds

**Locks:** `CONTRACT.RUNTIME.REASONING`

**Status:** Accepted

**Decision:** The agent will select the execution mode for a goal from INTERACTIVE, BACKGROUND, LONG_HORIZON, DEEP_EXECUTION, SWARM, UNATTENDED, RECOVERY, or VERIFICATION. Mode selection is a proposal that never raises a permission ceiling, never suppresses an evidence requirement, and never converts a decision node into an assumption. In UNATTENDED mode a required decision yields WAITING or ESCALATED rather than a guess.

**Rationale:** The runtime knows better than a user toggle whether a goal needs many validated iterations or a single interactive change. But a mode that could widen authority would turn a convenience into a privilege-escalation path.

**Consequences:** A mode request exceeding policy is downgraded to the highest permitted mode and recorded, so the user can see that the runtime wanted more latitude than policy allowed.

## ADR-180: Enforce the Android-only generated target as a machine-checked invariant

**Locks:** `CONTRACT.RUNTIME.SCOPE`

**Status:** Accepted

**Decision:** `Project.targetPlatforms` must equal exactly `["android"]` at every revision, enforced at project construction rather than stated as intent. The runtime rejects a project whose target list is empty, contains any other value, or pairs `android` with a second platform. Framework choices that run on Android — Kotlin, Java, Jetpack Compose, Android Views, React Native, Expo, native modules — are implementation styles selected by the technology resolver, not additional targets. No resolver path, worker role, or capability may produce a non-Android deployable.

**Rationale:** A generic platform field with a documented intention drifts. Nirman's scope boundary is its most load-bearing product decision, and a configuration value able to widen it silently would let a web or server target enter through the data model without any decision being recorded.

**Consequences:** The data model keeps generic field shapes for stability while the invariant constrains their values. A future multi-target product would require a new versioned scope contract and a superseding ADR, not a configuration change.

## ADR-172: Treat deliberation computation as a first-class runtime resource

**Locks:** `CONTRACT.RUNTIME.DELIBERATION`

**Status:** Accepted

**Decision:** Reasoning effort will be budgeted separately from host and provider resources through a DeliberationBudget carrying ceilings on reasoning time, passes, model requests, reasoning tokens, observation-free passes, evidence-acquisition passes, hypotheses, strategy candidates, specialist consultations, and candidate branches. The agent may request an effort level; only the deterministic runtime grants one, and a request above policy or capacity is downgraded to the highest permitted level and recorded.

**Rationale:** A task can have CPU, provider, and wall-clock capacity available and still need to stop deliberating, and can have tight capacity and still need to deliberate further on a high-risk change. Conflating the two makes the decision unexpressible. Letting the model grant its own effort would make unbounded thinking self-authorising.

**Consequences:** Every deliberation carries budget accounting, and a downgrade is visible to the user rather than silent — which is how they learn the runtime wanted more latitude than policy allowed.

## ADR-173: Escalate reasoning effort through declared levels on recorded conditions

**Locks:** `CONTRACT.RUNTIME.DELIBERATION`

**Status:** Accepted

**Decision:** Effort will move through NORMAL, EXTENDED, DEEP, and EXHAUSTIVE. Escalation requires a recorded condition — persistent uncertainty, competing hypotheses, high risk, or an unresolved architectural or destructive change — and de-escalation is permitted when uncertainty resolves. DEEP and above require hypothesis competition and adversarial critique. Effort level never alters permissions, evidence requirements, or authority.

**Rationale:** Fixed effort is wrong in both directions: it wastes computation on routine changes and under-thinks the changes that damage a project. Requiring a recorded condition prevents escalation by preference.

**Consequences:** The runtime must classify task risk before selecting effort, and EXHAUSTIVE work must terminate in branching or escalation rather than an unbounded search.

## ADR-174: Require deliberation passes to produce evidence, not only reasoning

**Locks:** `CONTRACT.RUNTIME.DELIBERATION`

**Status:** Accepted

**Decision:** Deliberation will interleave reasoning with read-only observation — code and symbol reads, impact-graph queries, log reads, non-mutating diagnostics. Consecutive passes acquiring no new observation count against a maxToollessPasses ceiling, and reaching it forces evidence acquisition or termination. Deliberation must not mutate project source; an observation that would mutate requires the ordinary authorization path.

**Rationale:** Repeated reasoning over an unchanged observation set is the dominant failure mode of extended thinking: it produces increasingly confident conclusions from the same evidence. Forcing observation is what converts thinking time into information.

**Consequences:** Deliberation consumes tool and I/O capacity, not just model capacity, and the planner must estimate observation cost before choosing.

## ADR-175: Compete hypotheses and critique strategies adversarially at DEEP effort

**Locks:** `CONTRACT.RUNTIME.DELIBERATION`

**Status:** Accepted

**Decision:** At DEEP and above the runtime will enumerate candidate hypotheses, define a discriminating test per candidate, rank by decisiveness over cost, execute the most decisive affordable test, and attempt refutation rather than confirmation. A selected strategy must then pass an adversarial critique that searches for a counterexample and emits either a rejection finding or evidence requests. The critic has no mutation capability, no evidence-approval capability, and no completion authority.

**Rationale:** A defect with four plausible causes is not solved by acting on the most available one. Refutation is what distinguishes diagnosis from guessing, and a critique that cannot reject anything is decoration.

**Consequences:** Hard problems take longer before the first mutation and are far more likely to be repaired at the cause, aligning with the cause-scoped repair rule of the localization contract.

## ADR-176: Preserve deliberation state across provider requests and context compaction

**Locks:** `CONTRACT.RUNTIME.DELIBERATION`

**Status:** Accepted

**Decision:** A DeliberationSession spans multiple model requests, tool observations, and context reconstructions, and will survive compaction, provider failover, and runtime restart. Continuation state — deliberation revision, objective, active hypotheses, evidence acquired, rejected strategies with reasons, granted effort level, remaining budget, provider continuation state — is checkpointed at every pass boundary and treated as constraint-class content ineligible for eviction during compaction.

**Rationale:** A provider request is not the unit of intelligence. If compaction discards active hypotheses or rejected strategies, the agent re-derives conclusions it already refuted and can loop indefinitely on a solved question.

**Consequences:** Compaction has less room for file content, and a compaction that drops session state is detectable by revision comparison and reported as a defect rather than tolerated.

## ADR-177: Terminate deliberation on diminishing returns rather than reasoning further

**Locks:** `CONTRACT.RUNTIME.DELIBERATION`

**Status:** Accepted

**Decision:** Each pass will record uncertainty change, evidence added, hypotheses eliminated, and strategy stability. When movement falls below a diminishing-return threshold across consecutive passes the deliberation is classified NO_PROGRESS, and the runtime must acquire evidence, escalate the model, branch candidates, delegate, or escalate to a human decision. A further plain reasoning pass is refused.

**Rationale:** Without a stall detector, "think longer" degenerates into thinking without converging. Flat uncertainty across passes means the current approach has extracted what it can, and the correct response is a different approach rather than more of the same.

**Consequences:** Every pass must be measurable, and BUDGET_EXHAUSTED and NO_PROGRESS must never be reported as sufficiency or permit the leading strategy to execute as though validated.

## ADR-178: Escalate the model without escalating authority

**Locks:** `CONTRACT.RUNTIME.DELIBERATION`

**Status:** Accepted

**Decision:** Deliberation may route to a stronger or specialist model based on problem complexity, required effort, context capacity, tool and vision capability, coding capability, historical failure rate for the surface, provider health, latency, cost, and privacy policy. The escalated model inherits the identical permission ceiling, evidence requirements, and authority path as the model it replaces.

**Rationale:** Harder problems warrant more capable models, but a routing decision that also widened permissions would turn model selection into a privilege-escalation path.

**Consequences:** Routing considers more inputs than task type, and an unavailable escalation target is recorded as a capability gap rather than silently substituted.

## ADR-179: Require skills to declare their reasoning and evidence requirements

**Locks:** `CONTRACT.RUNTIME.DELIBERATION`

**Status:** Accepted

**Decision:** A skill will declare a deliberation profile: minimum effort level, required evidence kinds, whether critique is mandatory, preferred model capabilities, maximum deliberation cost, allowed delegation, and failure strategies. The runtime honours the declared minimum and refuses to execute a skill whose required evidence kinds are unavailable in the current environment rather than proceeding with less.

**Rationale:** Effort should be a property of the work rather than a guess made per invocation. A data-layer migration inherently needs schema analysis, compatibility analysis, data-loss analysis, a rollback plan, and a test strategy; encoding that in the skill makes it unforgettable.

**Consequences:** Skill authors must characterise their reasoning needs, and a skill can be blocked by environment limitations before it produces a partial migration.


## ADR-181: Enforce intent-driven Android synthesis without user-facing templates

**Locks:** `CONTRACT.RUNTIME.SCOPE`

**Status:** Accepted

**Decision:** Every new Android application session begins from user intent, product concept, optional screenshots, supplied assets, device requirements, privacy constraints, and requested integrations. Nirman must not expose a template catalog, ask the user to choose an app archetype, require a framework selection, or treat an internal bootstrap as the product design. The technology resolver selects and composes Android implementation styles from evidence.

**Rationale:** Users describe the product they want, not the framework they happen to know. A template or framework picker would make the product dependent on predefined shapes and could cause the implementation to follow a starter structure rather than the actual requirements.

**Consequences:** Internal bootstraps, component libraries, and build profiles may improve reliability but have no user-facing identity or authority. Prompt, worker, skill, and deliberation contracts must reject template-selection requirements, archetype assumptions, and non-Android target proposals.

## ADR-182: Make the live preview a revision- and checkpoint-bound evidence projection

**Locks:** `CONTRACT.RUNTIME.E2E`, `CONTRACT.RUNTIME.VERIFICATION`

**Status:** Accepted

**Decision:** `PreviewCoordinator` is the sole service allowed to create, reload, install, promote, invalidate, or roll back a live Android preview. Every `PreviewRevision` binds project revision, checkpoint, source fingerprint, contract version, technology-plan version, asset manifest, build variant, artifact identity, device identity, execution truth, runtime state, validation state, and evidence IDs. Every promotion decision must pass the canonical `PreviewPromotionGate` defined in technical architecture §73.5.1; no worker, model, UI projection, successful build, or isolated evidence item may promote a candidate independently.

**Rationale:** A preview is trustworthy only when the user can identify exactly which source and checkpoint produced it and which observations prove that it is running. A build result or model statement alone cannot prove device behavior.

**Consequences:** A candidate preview cannot replace the last-known-good revision until the declared build, install, launch, interaction, and validation observations pass. Stale, predicted, simulated, requested, and invalidated states remain visible but cannot satisfy completion.

## ADR-183: Keep prompt and presentation layers subordinate to execution evidence

**Locks:** `CONTRACT.RUNTIME.REASONING`, `CONTRACT.RUNTIME.VERIFICATION`

**Status:** Accepted

**Decision:** Prompt builders and UI projections may explain intent, plans, predicted stages, observed actions, recovery, and evidence, but they cannot authorize tools, mutate source, promote previews, or mark tests and artifacts complete. Preview and execution screens must distinguish `PREDICTED`, `SIMULATED`, `REQUESTED`, `OBSERVED`, `VERIFIED`, `STALE`, and `INVALIDATED` states. Only supervised observations and independent validators can produce completion evidence.

**Rationale:** A streamed model response, a progress spinner, a file timestamp, or a successful compilation can be mistaken for actual execution. Separating presentation from authority prevents false progress and false completion.

**Consequences:** UI reconnect and replay reconstruct the durable projection from the event ledger. A disconnected or stale stream cannot advance status locally. Prompt or model claims are retained only as proposals or explanations and never as proof.

## ADR-184: Normalize provider-native reasoning without exposing or delegating runtime authority

**Locks:** `CONTRACT.RUNTIME.DELIBERATION`

**Status:** Accepted

**Decision:** Nirman will treat provider-native reasoning effort and runtime deliberation as separate but composable resources. The ModelGateway will normalize provider-specific reasoning controls into the runtime's NORMAL, EXTENDED, DEEP, and EXHAUSTIVE effort levels, while the deterministic runtime remains responsible for effort grants, reasoning budgets, pass limits, evidence requirements, and authority. Provider-native reasoning tokens or equivalent usage will be recorded as reported, estimated, or unavailable and will never be fabricated.

A provider that cannot satisfy the minimum reasoning capability required by the current deliberation must not silently downgrade the task. The runtime must select an approved compatible provider/model, explicitly downgrade when policy permits, or terminate with a typed capability gap.

No provider-native reasoning stream containing private model reasoning may be persisted or exposed verbatim. Only approved structured summaries and runtime events may enter the visible reasoning stream.

**Rationale:** A hard-problem-solving runtime needs to use models that support deeper inference when available, but provider-specific reasoning controls cannot become a second authority system. Separating native reasoning from runtime deliberation also prevents the system from treating one expensive model request as equivalent to evidence-producing iterative problem solving.

**Consequences:** Provider adapters must expose normalized reasoning capability metadata, the ModelGateway must translate effort levels deterministically, deliberation budgets must reserve and settle reasoning expenditure transactionally, and provider failover must revalidate reasoning capability before continuation.


---

## ADR-185: Make APK mandatory and AAB explicitly optional

**Locks:** `CONTRACT.RUNTIME.SUPPLY_CHAIN`, `CONTRACT.RUNTIME.VERIFICATION`

**Status:** Accepted

**Decision:** The minimum local Android deliverable is an installable APK. AAB generation is an optional separately declared packaging profile and is never implied by APK completion. Every packaging request records required artifacts, build variant, signing policy, reproducibility policy, installability checks, and evidence requirements before packaging begins.

**Reasoning:** APK is the local install-and-preview artifact. AAB has different release, signing, and distribution semantics and must not remain an undefined alternative in completion language.

**Trade-off:** Release workflows need an additional artifact profile, but completion and evidence semantics remain unambiguous.

---

## ADR-186: Separate lifecycle, assurance, capability, and delivery state

**Locks:** `CONTRACT.RUNTIME.AUTHORITY`, `CONTRACT.RUNTIME.EVIDENCE`

**Status:** Accepted

**Decision:** Product lifecycle, assurance, capability maturity, integration operationality, signing, artifact, preview, and delivery state are independent fields. `RUNNING`, `OBSERVED`, `VERIFIED`, `CERTIFIED`, `COMPLETED`, and `DELIVERED` are never treated as synonyms. The reducer rejects illegal combinations and the UI renders each dimension separately.

**Reasoning:** A task can be running without verified progress, an artifact can be exported while an integration remains user-required, and a preview can be observed without satisfying all verification gates.

**Trade-off:** State models become more explicit and require more UI and migration work, but false completion becomes mechanically detectable.

---

## ADR-187: Make evidence dependencies and external operationality canonical

**Locks:** `CONTRACT.RUNTIME.EVIDENCE`, `CONTRACT.RUNTIME.VERIFICATION`, `CONTRACT.RUNTIME.SUPPLY_CHAIN`

**Status:** Accepted

**Decision:** Evidence is represented as a dependency graph linking observations, evidence artifacts, validation results, certification decisions, and completion decisions. Source, asset, toolchain, device, artifact, policy, dependency, environment, and required-integration changes cascade invalidation to dependent evidence. Required external integrations use explicit operationality states and cannot be considered functional from local build or launch evidence alone.

**Reasoning:** A flat evidence list cannot reliably reveal stale dependencies, and an installable APK does not prove that a required service or API behaves correctly.

**Trade-off:** Evidence storage and invalidation become more complex, but preview and completion authorities consume one consistent proof model.

---

## ADR-188: Separate documentation certification from runtime certification

**Locks:** `CONTRACT.RUNTIME.INVARIANTS`, `CONTRACT.RUNTIME.VERIFICATION`

**Status:** Accepted

**Decision:** Contract-graph and Markdown checks certify documentation structure and addressing only. Separate runtime certification must prove schema compilation, reducer transitions, transactions, leases, Windows process and IPC behavior, provider fixtures, Android builds, emulator/device execution, preview truth, APK inspection, failure injection, restart recovery, hidden-human-dependency handling, and self-development rollback.

**Reasoning:** A clean documentation graph cannot prove that the runtime starts, builds an Android application, survives failure, or exports a valid artifact.

**Trade-off:** Release certification requires more jobs and fixtures, but capability claims become evidence-backed rather than inferred from prose.


---

## ADR-189: Establish one canonical machine-readable schema registry

**Locks:** `CONTRACT.RUNTIME.AUTHORITY`, `CONTRACT.RUNTIME.EVIDENCE`

**Status:** Accepted

**Decision:** `CanonicalSchemaRegistry` is the sole machine-readable ownership index for runtime entities, fields, enum values, invariants, migrations, persistence locations, authorities, and acceptance fixtures. Architecture prose, roadmap entries, and decision records may explain a schema but cannot silently redefine it. Every controller or contract version change requires an explicit `ContractCompatibility` record.

**Rationale:** Repeated prose can remain internally consistent while implementations diverge in field identity or migration behavior. Mechanical ownership and parity are required for durable replay and self-development.

**Consequences:** Schema compilation, parity checks, migration fixtures, and compatibility evidence become prerequisites for runtime promotion.

---

## ADR-190: Model integration operationality as independent dimensions

**Locks:** `CONTRACT.RUNTIME.EVIDENCE`, `CONTRACT.RUNTIME.AUTHORITY`

**Status:** Accepted

**Decision:** Required integrations record connectivity, authentication, availability, functionality, and acceptance state independently, with an aggregate operational state derived from those observations. `CONFIGURED`, `REACHABLE`, `AUTHENTICATED`, `FUNCTIONAL`, and `ACCEPTED` are not interchangeable.

**Rationale:** An endpoint may be reachable while returning `401 Unauthorized`, or functional for a health check while failing the user’s acceptance scenario.

**Consequences:** Integration health fixtures must include authentication failure, partial availability, functional failure, and acceptance failure. Build or launch evidence alone cannot promote an integration.

---

## ADR-191: Bind preview currency to branch and runtime-state fingerprints

**Locks:** `CONTRACT.RUNTIME.EVIDENCE`

**Status:** Accepted

**Decision:** A current preview is bound to active branch, project revision, promotion lineage, source, asset, toolchain, device, application, and environment-state fingerprints. The newest revision is never authoritative by number alone.

**Rationale:** Identical device identity and APK identity can still produce different behavior when permissions, databases, locale, network state, account state, or system settings differ.

**Consequences:** Preview promotion and stale invalidation require state-fingerprint capture and comparison, and the last-known-good preview remains available when any required identity is uncertain.

---

## ADR-192: Separate local, device, and external-effect transactions

**Locks:** `CONTRACT.RUNTIME.RECONCILIATION`

**Status:** Accepted

**Decision:** Local source/artifact mutations, device operations, and external side effects use separate transaction records and rollback semantics. A consumed mutation capability cannot be reused for a new side effect after an unknown response. Reconciliation uses an explicitly scoped reconciliation authority, idempotency key, read-back, or compensation evidence.

**Rationale:** Filesystem rollback can restore local state, but it cannot automatically undo a device installation or a remote request.

**Consequences:** Unknown outcomes are durable states, duplicate external effects are rejected, and local commit never implies remote or device success.

---

## ADR-193: Require deterministic capability promotion and signing identity binding

**Locks:** `CONTRACT.RUNTIME.SUPPLY_CHAIN`, `CONTRACT.RUNTIME.VERIFICATION`

**Status:** Accepted

**Decision:** Capability statuses are promoted only by a deterministic authority after current profile, fixture, environment, and evidence checks. Release signing is valid only when an immutable `SigningIdentityBinding` connects artifact hash, application identity, version code, certificate fingerprint, signing scheme, keystore identity, build variant, policy version, and inspection evidence.

**Rationale:** A worker or model must not be able to promote support or assert release signing through text or an unverified field.

**Consequences:** Capability-promotion and signing-inspection records become part of the release evidence graph and are invalidated by relevant artifact, policy, or environment changes.

## ADR-194: Establish one canonical integration-boundary contract

**Locks:** `CONTRACT.RUNTIME.INTEGRATION_BOUNDARY`, `CONTRACT.RUNTIME.AUTHORITY`, `CONTRACT.RUNTIME.EVIDENCE`, `CONTRACT.RUNTIME.RECONCILIATION`

**Status:** Accepted

**Decision:** Nirman will use one versioned `IntegrationBoundaryContract` as a cross-cutting reference envelope for operations that cross IPC, process, worker, workspace, persistence, provider, device, artifact, credential, signing, external-service, or documentation-verification boundaries. The envelope identifies source and destination, schema and protocol versions, adapter or bridge, deterministic authority, specialized state, operation and transaction references, permissions, credentials, correlation and idempotency, lifecycle policies, observations, evidence, validation, downstream effects, failure/recovery, compatibility, and invalidation dependencies.

The universal `SOURCE → CONTRACT → ADAPTER / BRIDGE → AUTHORITY → STATE → OPERATION → OBSERVATION → EVIDENCE → VALIDATION → DOWNSTREAM EFFECT` chain is mandatory only where a declared boundary exists. An inapplicable stage requires an explicit reason. The boundary envelope references specialized contracts and never redefines their schemas, state machines, authorities, transaction semantics, preview gate, provider context, skill lifecycle, artifact policy, signing identity, completion predicate, or documentation/runtime certification boundary.

**Canonical ownership:** the build specification owns the product invariant and registered contract; the technical architecture owns the implementation schema and protocol; the development plan owns sequencing, fixtures, and exit gates; the decision log owns precedence and rationale; and the verifier owns only documentation graph and semantic checks. No UI, model, worker, adapter, bridge, provider response, preview projection, export operation, or documentation verifier may promote a downstream effect without the applicable deterministic authority.

**Rationale:** Existing Nirman contracts already cover transactions, capabilities, leases and fencing, preview, evidence, validation, providers, workers, runtime execution, signing, and artifact promotion. A single reference envelope closes their correlation gap without creating four divergent wiring architectures or a second authority system.

**Consequences:** Boundary operations require explicit schema compatibility, lifecycle, timeout, cancellation, retry, reconciliation, observation, evidence, and invalidation references. Runtime implementation and fixture evidence are required before any capability or artifact claim is promoted. Android remains the only generated target; supporting services remain declared integrations rather than additional generated products.

## ADR-195: Make preview synchronization event- and reducer-bound

**Locks:** `CONTRACT.RUNTIME.PREVIEW_SYNC`, `CONTRACT.RUNTIME.EVIDENCE`, `CONTRACT.RUNTIME.AUTHORITY`

**Status:** Accepted

**Decision:** Nirman will synchronize chat instructions, autonomous agent activity, source revisions, Android builds, artifacts, emulator/device observations, evidence, validation, promotion, and the live preview panel through one durable `PreviewSyncEvent` sequence and one deterministic `PreviewProjectionReducer`. Agents, workers, models, build processes, device callbacks, evidence producers, and UI components may emit requests or normalized observations but cannot mutate preview projection state directly.

Events are applied by durable sequence and compatible identity, not arrival time. Same-ID same-payload replay is idempotent; conflicting duplicate IDs are quarantined; sequence gaps block advancement and request replay; old, late, stale, or incompatible events cannot overwrite a newer projection; stream loss freezes the panel at the last durable state; reconnect replays and verifies continuity before resuming; and every displayed completed stage carries `PreviewSyncEvidenceRecord` linking event range, reducer version, projection revision, preview revision, branch/candidate identity, runtime session, device identity, artifact fingerprint, state fingerprints, observation references, evidence references, validation references, recovery events, and promotion or completion decisions.

The event authority class limits the projection dimensions that an event may advance: declarative or planned events cannot advance execution, runtime, evidence, validation, or certification state. Every non-root event must retain causal parentage and compatible project, revision, candidate, artifact, runtime-session, and device lineage. For a compatible identity, a current supervised runtime observation reconciles contradictory persisted runtime state; for an incompatible identity, the event becomes stale or invalidated rather than being merged. Events after cancellation, rollback, promotion, or worker fencing remain historical or quarantined unless a new authorized lineage admits them.

`PreviewCoordinator` remains the sole preview mutation and promotion service, and `PreviewPromotionGate` remains the sole promotion predicate. This decision does not create a second preview state machine, evidence authority, completion authority, or artifact authority.

**Rationale:** A panel that receives independent worker, build, device, and model updates can display a visually plausible but causally incorrect state unless one durable sequence and reducer determine what the user sees. Event-bound projection makes chat-to-preview synchronization replayable, stale-safe, reconnectable, and evidence-auditable.

**Consequences:** M108 must prove the complete chat-to-device-to-panel vertical slice, M109 must prove resilience and runtime-certification evidence, and implementation status cannot be inferred from the presence of the schemas or documentation verifier alone.

## ADR-196: Continue autonomous work from durable events with specialist gates

**Locks:** `CONTRACT.RUNTIME.TRIGGER`

**Status:** Accepted

**Decision:** Nirman will continue an approved Android task from durable lifecycle events rather than requiring a new chat action after every step. Saved revisions, completed builds, observed failures, dependency changes, local preview promotion or declared artifact export requests, and stream reconnection may schedule the next authorized operation under the current goal, policy, revision, checkpoint, worker, capability, and evidence context.

The continuation path must capture real diagnostics and stack-trace references, create a stable failure fingerprint and privacy-filtered failure context package, and provide that context to the next authorized diagnostic or coding worker. Dependency and security checks must run before an affected commit or promotion is accepted. Failed health, validation, signing, or export gates preserve last-known-good state and cannot be replaced by model or worker claims.

Specialist workers may handle orchestration, security, consistency, diff-aware patching, diagnostics, validation, memory/index updates, or release preparation, but they remain subordinate to the existing task, policy, workspace, evidence, signing, promotion, and completion authorities. No specialist report creates permission, completion, certification, or deployment authority.

**Rationale:** Event-driven continuation and focused specialists are necessary for long-horizon autonomous work, but independent agents must not become independent sources of truth. Durable triggers, evidence-carrying failure context, bounded strategy changes, and singular deterministic authorities prevent blind retries, silent drift, secret leakage, and false completion.

**Consequences:** M110 must prove the continuation triggers, failure-feedback loop, specialist gates, dependency/security blocking, last-known-good preservation, and replayable autonomous progress. Windows process/workspace isolation remains the local boundary, and Android remains the only generated target.

## ADR-197: Make cost governance a deterministic resource authority

**Locks:** `CONTRACT.RUNTIME.COST_GOVERNANCE`

**Status:** Accepted

**Decision:** Token, request, duration, process, emulator, disk, and estimated monetary budgets are governed by durable reservations and settlements. Cost exhaustion may cause adaptive degradation, an approved policy change, pause, or safe failure, but never false completion, silent permission expansion, or evidence weakening. Cost governance sits beside policy and resource authority and cannot override safety, privacy, signing, validation, or completion authority.

**Rationale:** Without a deterministic budget record, long-running autonomy can exhaust provider, process, or device resources without a truthful outcome or safe recovery path.

**Consequences:** M111 must prove usage accounting, unknown-outcome reconciliation, cap enforcement, and truthful exhaustion behavior.

## ADR-198: Scan and revoke agent-layer extension content

**Locks:** `CONTRACT.RUNTIME.AGENT_TRUST`

**Status:** Accepted

**Decision:** Skills, MCP-compatible tools, plugins, workflow packages, provider-returned tool descriptions, and instruction-bearing extension content are untrusted data until provenance, content hash, version, requested capabilities, destinations, secret access, static findings, behavioral findings, policy admission, and revocation state are recorded. A passing scan grants no permission, and revocation or drift invalidates prior admission.

**Rationale:** Extension content can contain instructions or payloads that impersonate authority, request undeclared access, or exfiltrate secrets; scanning and revocation must therefore precede admission and remain enforceable.

**Consequences:** M112 must prove quarantine and revocation fixtures. Untrusted instructions cannot alter target scope, authority, policy, or completion state.

## ADR-199: Govern context compaction and provider cache reuse

**Locks:** `CONTRACT.RUNTIME.CONTEXT_GOVERNANCE`

**Status:** Accepted

**Decision:** Context compaction and provider cache reuse require explicit policy, protected-context classes, compatibility keys, invalidation events, privacy controls, and telemetry disclosure. Active constraints, locked decisions, evidence lineage, and required source context cannot be evicted for budget. A cache hit is not a fresh observation.

**Rationale:** Compaction and cache reuse can silently remove constraints or reuse stale private context unless their identity, invalidation, and telemetry rules are explicit.

**Consequences:** M113 must prove compaction, cache compatibility, invalidation, privacy, and lineage fixtures.

## ADR-200: Report Android runtime integrity as independent applicable signals

**Locks:** `CONTRACT.RUNTIME.ANDROID_INTEGRITY`

**Status:** Accepted

**Decision:** Play Integrity, ANR, battery, Doze/background restriction, permission, device, and runtime-session signals are collected and validated independently. Play Integrity is conditional on declared support and configuration; unavailable or inapplicable signals are not passes. Local Android completion remains evidence-bound without requiring a production-only service for every preview.

**Rationale:** Android runtime integrity signals have different availability and evidentiary meaning; treating unavailable Play Integrity, ANR, battery, or Doze data as a single pass would create false assurance.

**Consequences:** M114 must prove honest coverage, stale-signal invalidation, and typed unavailable or not-applicable outcomes.

## ADR-201: Make the frontend a typed projection client of the control plane

**Locks:** `CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE`

**Status:** Accepted

**Decision:** The C#/.NET + WinUI 3 frontend communicates with the authoritative local control plane through authenticated, project-scoped, schema-versioned commands and durable event subscriptions. The command registry, response and error envelopes, transaction ownership, projection snapshots, replay cursor, backpressure, idempotency, and optimistic-state separation are canonical. The UI cannot authorize operations, write domain state, fill event gaps, promote artifacts, or advance evidence. Generated Android service adapters are separate from Nirman IPC and own only generated application behavior.

**Rationale:** A frontend that directly manipulates state, assumes successful requests, or reconstructs missing events from local memory becomes a second authority and can display progress that the backend never accepted. Typed envelopes and cursor-atomic replay make the UI reconnectable, diagnosable, and safe without coupling domain persistence to WinUI 3 components.

**Consequences:** M115 must prove the command registry, typed failures, projection reconstruction, replay and backpressure behavior, SQLite transaction ownership, and generated Android service boundary.

## ADR-202: Canonical background continuity state machine
**Locks:** `CONTRACT.RUNTIME.BACKGROUND_CONTINUITY`
**Status:** Accepted
**Decision:** Background autonomy is represented by one durable, versioned continuity record with independently persisted UI, host, device, provider, lease, and reconciliation dimensions. Its aggregate state is derived by deterministic precedence and is an orthogonal substate of the existing product lifecycle; it cannot replace `ProductLifecycleState`, own `CompletionDecision`, or create a second recovery authority. UI closure never cancels eligible work. Recovery must reload durable checkpoints, fence stale leases, reconcile unknown outcomes, and preserve truthful evidence and last-known-good state. The frontend receives continuity only through the typed projection and cannot resume, complete, or clear continuity states.
**Rationale:** Continuity behavior already spans several authorities; orthogonal dimensions and an aggregate precedence rule prevent concurrent host, device, provider, lease, and UI conditions from overwriting one another without transferring authority to the model. Existing lifecycle, recovery, lease, device-session, and provider-operationality authorities remain canonical; continuity names are aliases only.
**Consequences:** M116 must execute interruption and recovery fixtures. Suspended, offline, unreconciled, or invalidated work cannot be presented as verified progress or completion.

## ADR-203: Make local deployment export profile-bound and provenance-complete
**Locks:** `CONTRACT.RUNTIME.APK_EXPORT`
**Status:** Accepted
**Decision:** `ExportVerificationRecord` is strengthened with packaging-profile, artifact-kind, source-revision, checkpoint, source/destination identity, request-fingerprint, idempotency, signing, validation, promotion, reconciliation, failure-evidence, delivery-kind, and destination-kind references. Its copy lifecycle includes `UNKNOWN` and `RECONCILING`, and uncertain copies cannot be retried until destination inspection and identity/hash reconciliation resolve them. Local deployment is restricted to verified declared artifacts on the approved Windows filesystem. The required local deliverable remains APK; AAB remains optional only under an explicitly declared `PackagingProfile`. Workspace, ZIP, and Git access remains available as `SOURCE_ACCESS_ONLY` and never satisfies deployment completion.
**Rationale:** A durable post-copy record must prove not just byte copying but the identity and policy lineage of the delivered artifact, while source access and deployment delivery are distinct user needs.
**Consequences:** M117 must prove hash equality, destination identity, interrupted-copy reconciliation, profile admission, optional declared-AAB behavior, and rejection of external deployment destinations.

## ADR-204: Make local certification authoritative and hosted CI optional

**Locks:** `CONTRACT.RUNTIME.INVARIANTS`
**Status:** Accepted
**Decision:** Under the existing invariants contract, Nirman’s local certification commands are the authoritative engineering validation path. The repository must provide equivalent Unix-like and Windows entry points that run documentation certification, M0 foundation checks, Rust formatting and tests, frontend checks/build, and fixture validation without requiring GitHub, GitHub Actions, hosted CI, or network access to a repository host. Git hosting and hosted CI may be used as optional source-control or convenience services, but they are not runtime authorities, certification authorities, build dependencies, or prerequisites for Nirman to build, test, certify, run, recover, or produce a local Android artifact.
**Rationale:** Nirman is a Windows-first local application whose control plane, execution, evidence, recovery, and delivery must remain functional when GitHub or any hosted CI service is unavailable. A local certification command makes the engineering gate reproducible on the developer machine and keeps hosted automation from becoming an accidental product dependency.
**Consequences:** M0 requires `tools/verify.sh` and `tools/verify.ps1` to remain aligned. Documentation, implementation, and fixture changes must be validated locally before commit. Optional hosted workflows, if ever reintroduced, must call or reproduce the local certification contract and must not be treated as the source of truth.

---

## ADR-205: Nirman requires no account, subscription, license fee, or hosted platform

**Locks:** `CONTRACT.RUNTIME.INVARIANTS`
**Status:** Accepted
**Decision:** Nirman itself will never require a user account, login, subscription, license fee, recurring payment, or mandatory hosted platform/cloud dependency for local use. Distribution consists of a Windows `.exe` installer built from source or obtained from a trusted source. AI provider access (API keys, base URLs, model IDs) is supplied and paid for by the user directly with their chosen provider; Nirman does not proxy, resell, or charge for provider usage. Full source remains available for local build and permitted redistribution per the eventual license.
**Rationale:** Nirman's core distinction is a local desktop application that builds applications on the user's own computer. Introducing a mandatory account, subscription, or hosted dependency would contradict ADR-001 (desktop, not hosted platform), the local-first principle in §3.1, and user ownership of source, builds, and credentials. Provider cost is a separate commercial relationship between the user and their provider(s), not a Nirman product constraint.
**Consequences:** No feature, integration, or workflow may add a mandatory account, subscription, license fee, or hosted-platform dependency for Nirman itself. This invariant is mirrored as a binding product constraint in the Build Spec §1.5 (Distribution and licensing model). Future changes that would weaken it require a new accepted ADR and supersession of this one.

---

## ADR-206: Cross-compilation does not establish native target-runtime support or certification

**Locks:** `CONTRACT.RUNTIME.PLATFORM_CAPABILITY`
**Status:** Accepted
**Decision:** Host environment, target platform, validation platform, and certification status are four distinct state values and must never be collapsed into one build, validation, or completion result. Cross-compilation (or any host-platform compilation) establishes artifact-production capability only. Native target-runtime capability, target-specific validation, and certification require authoritative observation from a matching validation environment held under a durable lease, with evidence bound to the environment fingerprint, target platform, and source revision. Platform capability classification (`AVAILABLE`, `REPAIRABLE`, `USER_REQUIRED`, `UNAVAILABLE`) is decided by the deterministic `EnvironmentCapabilityPlanner` from observed preflight; a model, worker, or skill may never set or raise it. The names `CrossCompilationAuthority` and `NativeRuntimeValidationAuthority` denote existing decision points (the `ToolBroker`/`PolicyAuthority` admission decision point and the `EvidenceAuthority`/completion-evaluator gate, respectively), not new authorities. No container, VM, WSL, or simulated environment may substitute for the declared target platform's native validation.
**Rationale:** Without this distinction, an agent operating on a non-target host can honestly complete a cross-build and then represent it as target-runtime validation — "I launched Nirman and verified Windows ConPTY" — producing certification claims with no underlying observation. The existing invariants (model proposes, authorities decide; evidence is bound to source, toolchain, device, and environment state and invalidates on change; hidden-human dependencies resolve to an authorized action, `USER_REQUIRED`, or a truthful block) already forbid each piece of this behavior in isolation. This ADR locks their combination into one sealed platform capability contract (Build Spec §79, Technical Architecture §84) and makes the four-state invariant machine-checkable by the contract-graph verifier and the runtime fixtures.
**Consequences:** New contract `CONTRACT.RUNTIME.PLATFORM_CAPABILITY` (Build Spec §79, Technical Architecture §84, milestone M118, test family `TEST-PLAT-001`, evidence `EV-PLAT-001`). The task planner resolves environment capability before implementation commits (Build Spec §52.9/§52.10 extended); the planner splits work so independent host-platform work continues while target validation waits as a durable `USER_REQUIRED`/`UNAVAILABLE` node. The existing `EnvironmentCapabilityPlanner` (M71) is extended, not replaced. `EnvironmentCapabilityRecord`, `PlatformCapabilityEntry`, `ValidationEnvironment`, and `BuildGateRecord` join the `CanonicalSchemaRegistry` (TA §36.1); `WorkerContract` gains platform requirement fields. The hidden-human-dependency concept of Build Spec §69.10 is extended to cover an absent target validation environment. Future weakening requires a new accepted ADR and supersession of this one.

---

## ADR-207: Nirman supports cloud AI providers only

**Locks:** `CONTRACT.RUNTIME.INVARIANTS`
**Status:** Accepted
**Decision:** Nirman supports only cloud-hosted, network-reachable AI providers configured by the user with an API key, base URL, and model ID. Local, offline, on-device, and self-hosted model runtimes are out of scope. A provider base URL resolving to localhost, 127.0.0.0/8, ::1, or an RFC-1918 private range MUST be rejected at configuration time. This does not restrict Nirman's own local control plane, supervisor, build tooling, or Android development servers, which remain local by design.
**Rationale:** Local model runtimes have materially different context limits, tool-calling fidelity, structured-output reliability, and vision support. Supporting them as a first-class path would mean every capability claim carries an unstated "depending on your local model" qualifier, which conflicts with the evidence and capability-truth model. A single cloud provider contract keeps capability claims checkable.
**Consequences:** BS §8.3 local-endpoint allowance is removed. BS §8.1 gains base-URL validation. Privacy mitigations may no longer cite local models. AGENTS.md §2 and README provider rows become cloud-only. ADR-019's provider-neutral interface is unaffected — neutrality is across cloud vendors, not across locality. Nirman itself remains local-first and requires no account or subscription (ADR-205, unchanged). Future reversal requires a new accepted ADR superseding this one.
