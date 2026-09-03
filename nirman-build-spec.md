# Nirman

## Detailed Product and Build Specification

**Document type:** Product requirements and technical architecture specification  
**Application type:** Windows-first desktop application for autonomous Android application development  
**Suggested product name:** **Nirman**  
**Status:** Living product and build specification — accepted scope; implementation status is specification and contract-certification stage
**Primary goal:** Enable a user to create, modify, preview, test, package, and export Android applications through a conversational AI-assisted desktop workspace.

> **Implementation-status boundary:** This document defines accepted requirements and planned implementation. A capability is not a working-product claim until runtime source, fixture execution, and the required evidence are present in the repository and certification output.

**Canonical ownership:** The Build Spec owns product contracts, invariants, and capability/contract registries. The Technical Architecture owns implementation schemas, protocols, and module boundaries. The Development Plan owns sequencing, milestones, fixtures, and exit gates. The Decision Log owns accepted decisions, rationale, and supersession. The README is explanatory only. AGENTS defines agent operating constraints only. The verifier certifies documentation and semantic checks only; it is never a runtime authority.

---

## 1. Product Identity

### 1.1 Product name

The recommended name is **Nirman**.

The name “Nirman” conveys building and creation. It describes a focused Android development desktop application without making the product sound like a hosting service or developer platform.

The product should consistently be described as a **desktop application for building other applications**, not as a platform. It runs on the user’s computer, manages local project workspaces, connects to the user’s selected AI provider, and produces source code and build artifacts that remain under the user’s control.

### 1.2 Product statement

> Nirman is a local-first Windows desktop application that uses configurable AI models to help users design, generate, run, test, preview, repair, and package Android applications through a simple conversational workspace.

### 1.3 Product vision

Nirman should make Android development feel closer to describing a product than manually assembling every implementation detail. A user should be able to explain an Android idea, answer a small number of important questions, watch the application appear in the Nirman-managed local Android emulator preview, request changes through chat, and export the resulting Android source code or installable build.

The application should combine the most useful characteristics of conversational builders and traditional development environments:

| Existing development experience | Nirman response |
|---|---|
| Manual project setup | The application creates and configures the project locally |
| Repetitive file editing | The agent makes reviewable, structured changes |
| Long feedback loops | The local preview and test loop runs continuously |
| Difficult environment setup | Nirman diagnoses required tools and versions |
| Framework complexity | AI-selected Android architectures provide reliable starting points |
| Unclear AI behavior | Every action has a visible plan, diff, log, and result |
| Provider lock-in | Users can supply their own API key, base URL, and model ID |
| Cloud-only execution | Code runs locally on the user’s computer |

### 1.4 Important feasibility boundary

Nirman can become highly autonomous, but “fully autonomous” must be defined carefully. It should be capable of planning, implementing, running, testing, inspecting, and repairing a project within a permissioned local workspace. It should not silently access arbitrary files, publish software, spend money, sign release builds, or transmit private information without user approval.

### 1.5 Distribution and licensing model

Nirman is a **local Windows desktop application** with the following distribution invariants:

| Property | Requirement |
|---|---|
| Account / login | **None required** — no user account, no authentication to Nirman services |
| Subscription / recurring payment | **None** — no subscription, no license fee, no recurring charge to use Nirman |
| Hosted platform dependency | **None** — no mandatory cloud service, no hosted execution, no platform account |
| AI provider costs | **User's own responsibility** — user supplies their own API keys, base URLs, model IDs; Nirman does not proxy, resell, or charge for provider usage |
| Distribution artifact | Windows `.exe` installer built from source; user builds locally or obtains from a trusted source |
| Source access | Full source code available; user may build, modify, redistribute per the eventual license |

These invariants are binding product constraints. No future feature, integration, or workflow may introduce a mandatory account, subscription, license fee, or hosted-platform dependency for Nirman itself. AI provider usage remains the user's separate commercial relationship with their chosen provider(s).

### Runtime authority principle

The target is **autonomous system recovery**, not “the AI becomes the authority.” The model may propose plans, edits, tool calls, recovery strategies, and self-improvements, but deterministic runtime authorities remain in control of execution. Nirman must recover, retry, checkpoint, repair, reconcile, degrade, or fail safely by itself while lifecycle, permission, sandbox, storage, checkpoint, evidence, promotion, rollback, and termination authorities enforce the boundaries.

| Authority | Non-delegable responsibility |
|---|---|
| Lifecycle authority | Starts, pauses, resumes, restarts, and terminates processes and tasks |
| Permission authority | Evaluates allow, ask, and deny decisions for every tool action |
| Sandbox authority | Enforces filesystem, process, network, resource, and workspace isolation |
| Storage authority | Commits durable task state, events, checkpoints, leases, and recovery records |
| Evidence authority | Determines whether actions and completion claims have verifiable evidence |
| Recovery authority | Selects retry, repair, backtracking, delegation, degradation, or safe failure |
| Promotion authority | Controls candidate activation, self-updates, canaries, and rollback |
| Model | Proposes work and interprets results but cannot grant itself authority or override deterministic controls |

A practical definition is:

> **Nirman autonomously performs the complete development loop inside an approved project workspace while preserving user control over credentials, privileged commands, external side effects, and final release decisions.**

The product must not force a user-facing technology shortlist. The user describes the Android application, its behavior, visual references, integrations, device requirements, and delivery needs; the configured AI and framework resolver select, compose, and validate the implementation technologies automatically.

---

## 2. Target Users and Use Cases

### 2.1 Primary users

Nirman is intended for independent developers, startup founders, designers who can describe product requirements, students learning application development, small agencies, internal tools teams, and experienced engineers who want to accelerate repetitive implementation work.

The product should support both technical and semi-technical users. Beginners need guided setup, explanations, and safe defaults. Experienced developers need direct access to files, commands, diffs, logs, provider settings, and project configuration. Nirman must never present a user-facing template catalog or require an app archetype selection.

### 2.2 Core use cases

A user should be able to create an Android application by describing its goal, screens, navigation, data, visual style, device behavior, permissions, and required integrations. They should be able to continue modifying it through natural language without losing manual editing control.

A user should also be able to open an existing Android project, ask Nirman to understand the codebase, and request targeted changes. The application should create a checkpoint before significant work and show a summary of changed files.

The Android generation system must synthesize projects dynamically from the user’s instructions and reference screenshots. It may use internal framework bootstraps, component libraries, or build profiles to improve reliability, but users must not be limited to selecting from a fixed template catalog. The framework resolver should choose or compose the appropriate Android implementation—such as Expo/React Native, native Android, Kotlin/Jetpack Compose, or a mixed project—based on requirements, device capabilities, dependencies, and validation needs.

### 2.3 Example user requests

```text
Build an Android customer-support app with authentication, a ticket list, ticket details, search, status filters, offline caching, dark mode, and accessible mobile layouts.
```

```text
Build an Android habit tracker with onboarding, daily habits, reminders, offline storage, a statistics screen, and a clean dark theme.
```

```text
Build an Android marketplace app with product browsing, search, cart state, checkout flow mockups, push-notification handling, and responsive phone and tablet layouts.
```

```text
The preview is showing a blank screen after the last change. Diagnose the
problem, run the appropriate checks, fix it, and explain the cause.
```

---

## 3. Product Principles

### 3.1 Local-first execution

The generated source code, project files, previews, tests, and builds should run locally whenever possible. Nirman may call cloud AI services when the user configures a cloud provider, but application execution should not depend on a hosted execution environment.

Local-first does not automatically mean that all data remains local. If a user chooses a cloud AI model, relevant prompts and project context may be sent to that provider. Nirman must clearly explain this distinction and provide local-model support as an alternative.

### 3.2 User-owned output

The user should always be able to access the project directory, source files, Git history, configuration, and generated artifacts. The application should support ZIP export and Git repository export without requiring a Nirman account or proprietary hosting service.

### 3.3 Visible autonomy

Autonomous behavior must be inspectable. The user should be able to see the current task, plan, files being changed, commands being executed, test results, and reasons for failure or escalation.

### 3.4 Reversible changes

Every meaningful autonomous task should create a checkpoint. The user should be able to undo the complete task, restore an individual file, compare versions, or continue from a previous checkpoint.

### 3.5 Progressive complexity

Nirman should start with a reliable dynamic project-synthesis loop. Internal bootstraps and framework adapters may provide stable foundations, but the user experience must remain instruction-driven and capable of generating different Android architectures as requirements evolve.

---

## 4. User Experience and Interface

### 4.1 Main application layout

Nirman should use a minimal, focused desktop layout inspired by modern AI coding products without copying their branding or exact interface.

| Area | Purpose |
|---|---|
| Left navigation | Projects, recent workspaces, project inputs, assets, settings, and diagnostics |
| Chat panel | Natural-language requests, planning status, explanations, and approvals |
| File tree | Project folders, generated files, search, and changed-file indicators |
| Main workspace | Code editor, visual preview, diff view, or project specification view |
| Bottom panel | Terminal output, test results, build logs, warnings, and agent activity |
| Top toolbar | Run, stop, preview, checkpoint, undo, build, export, and provider status |

### 4.2 First-run experience

On first launch, Nirman should explain that it is a local desktop application and ask the user to choose an AI provider. The setup flow should offer three paths:

1. Configure a cloud provider with a base URL, API key, and model ID.

2. Continue in planning-only mode without an AI provider.

The setup wizard should check the local environment, detect installed versions of Node.js, package managers, Java, Gradle, Android SDK, platform-tools, emulator tooling, and identify which Android capabilities are available. Missing tools should be reported with an installation guide rather than hidden behind a failed build.

### 4.3 Chat interaction model

Each user request should produce a structured response with the following sections:

```text
Understanding
Plan
Files to change
Commands that may run
Implementation progress
Validation results
Summary and remaining issues
```

When policy reaches a hard or review-gated operation, the chat should show a clear approval card. For example, external-directory access, credential use, emulator access, destructive operations, publishing, or release signing should not be hidden inside ordinary text. Routine reversible operations inside an approved workspace follow the configured execution profile and must not require repeated prompts in `Unattended / Full Autonomy` mode.

A request may include one or more screenshots as visual references. Nirman should analyze layout, typography, color, spacing, components, navigation states, device framing, interaction clues, and visible content. It should convert the analysis into an editable visual specification, identify uncertainty, synthesize the Android implementation, and validate the result against the reference screenshots in the Nirman-managed local Android emulator.

### 4.4 Live preview panel

The live preview MUST use a Nirman-managed local Android emulator as the sole canonical Android runtime. The emulator MUST run locally on the Windows host, MUST be launched headless, and MUST render its actual Android application surface inside Nirman's Preview panel. The user MUST NOT need physical Android hardware to build, install, launch, interact with, validate, or visually inspect the generated application.

No physical Android devices are outside Nirman product scope and MUST NOT be a validation, preview, recovery, completion, or fallback dependency.

The live preview MUST show the selected device, build/install state, Metro or native development-server output, connection status, runtime errors, Logcat output, reload controls, and the current project revision.

The default project workspace should show the running application preview and the live execution surface together. The preview occupies the primary visual area, while a resizable execution panel shows the task graph, nested worker steps, terminal streams, checkpoints, approvals, validation evidence, and current next action. Users may collapse or expand the execution panel, but the relationship between the running application and the work producing it must remain visible without navigating to a separate screen.

Nirman should optionally capture screenshots during autonomous tasks and compare them with user-provided references or generated visual baselines. The selected AI provider may receive screenshots for visual inspection if the user has enabled that capability. The user should be told when an image is being sent to a cloud provider. Screenshots, visual specifications, comparison results, and unresolved visual differences must be attached to the task evidence.

The preview panel MUST show the canonical local emulator session, including emulator identity, Android version, API level, architecture, orientation, density, build/install state, runtime state, Logcat, screenshots, interaction state, and revision identity.

### 4.5 Manual editing

Nirman must not trap users inside the chat. The application should include a full code editor with syntax highlighting, search, multi-file tabs, formatting, diagnostics, and direct editing. After a manual edit, the agent should be able to re-index the project and continue working from the updated state.

---

## 5. Android-Only Application Scope

**ContractId:** `CONTRACT.RUNTIME.SCOPE`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.SCOPE` (see BS §67.8)


### 5.1 Android generation target

Nirman should focus exclusively on Android applications, but it must not behave like a fixed template picker. It should build each application from the user’s natural-language requirements, reference screenshots, supplied assets, existing project files, device assumptions, and requested integrations.

The generator should synthesize the project structure, screens, navigation, state model, data layer, permissions, services, assets, design system, tests, build configuration, and validation plan for each request. Internal bootstraps may accelerate project creation, but they are implementation mechanisms rather than user-facing limitations.

The framework resolver may select or compose Expo/React Native, native Android, Kotlin/Jetpack Compose, or a mixed architecture when the requirements justify it. The user should be able to say what the application must do without first knowing which framework or template to select.

### 5.2 Complete Android technology coverage

Nirman must be designed to build all categories of Android applications end to end. The technology resolver must treat Android technologies as available implementation capabilities, not as future product tiers or user-selected templates. It must be able to choose and combine Java, Kotlin, Jetpack Compose, Android Views, Expo/React Native, custom native modules, Gradle plugins, background services, Bluetooth, NFC, camera and media, location, sensors, widgets, foreground services, WorkManager, push notifications, billing, maps, accessibility services, databases, networking stacks, authentication, offline storage, and complex device APIs.

The resolver selects the architecture from the user’s requirements and screenshots, then creates or modifies the complete project. It may choose a JavaScript layer, a native layer, or a mixed architecture, and it may introduce native modules whenever the requested capability requires them. The user does not need to know which technologies are required before starting.

### 5.3 Android scope boundary

The product core has one generated target: Android applications. The desktop application is only the local development environment and never becomes a generated project target. All project synthesis, previews, validation flows, toolchains, artifacts, and autonomous workflows must resolve to an Android project.

The system may use framework-required local build tooling, such as a JavaScript bundler or development server, only as an internal Android build dependency. These tools are never exposed as independent project-generation profiles.

### 5.4 Complete Android coverage contract

No category of Android application is excluded by product intent. The system must support consumer applications, business applications, marketplaces, media applications, communication applications, productivity tools, offline-first applications, location and sensor applications, device-integrated applications, background-service applications, accessibility-focused applications, games where the selected runtime supports them, and applications requiring mixed JavaScript/native or fully native implementations.

When a requested capability requires a missing SDK, device, vendor tool, native dependency, signing configuration, or external service, the runtime must diagnose and repair the environment where authorized, select an approved alternative, continue with a degraded but explicit mode, or report a precise technical blocker. It must not silently narrow the product scope because the user did not choose a predefined framework.

Product intent excludes no category. Reliable inference from natural language does have limits, and those limits are stated rather than discovered by the user after delivery.

The following categories are known to exceed what a natural-language instruction reliably specifies. When a request depends on one, the runtime MUST classify the affected capability `DEGRADED` or `USER_REQUIRED` with a stated reason and a description of what was and was not implemented. It MUST NOT silently produce a simplified implementation and present it as complete.

| Category | Why natural language under-specifies it |
|---|---|
| Custom animation, physics, or elastic interaction | Timing, easing, and feel are not inferable from prose |
| Performance-critical custom view rendering | Large datasets, real-time updates, and touch-scaled drawing need explicit performance targets |
| Non-standard sensor or peripheral integration | Bluetooth LE profiles and custom hardware protocols require device-specific specification |
| Offline synchronization with conflict resolution | Conflict semantics are application-specific and cannot be assumed |
| Densely interconnected business rules | Interacting edge cases are rarely stated completely in prose |
| Accessibility beyond standard semantics | Announcement order and custom traversal for non-standard layouts require explicit direction |
| Specific cryptographic or key-management methodology | Security design must be specified, never inferred |

This list records where inference is unreliable. It is not a list of unsupported features: each remains implementable when the user supplies the missing specification, and a clarifying question under §69.11's clarification gate is the correct first response when the request is otherwise well-formed.

Declaring the ceiling honestly is required by the capability-status vocabulary of §5.6. Reporting `SUPPORTED` for a capability whose implementation was silently simplified is a contract violation regardless of whether an APK was produced.

---

### 5.5 Generated-target invariant

The generated application target is Android and only Android. This is a machine-checked invariant, not a stated intention:

```text
Project.targetPlatforms == ["android"]        for every project, at every revision
```

The runtime must reject at construction any project whose `targetPlatforms` is empty, contains any value other than `android`, or contains `android` alongside another platform. Generic platform fields exist so the data model is stable, never so a second target can be introduced by configuration.

A field, template, resolver path, worker role, or capability that would produce a non-Android deployable is out of scope regardless of how it is labelled. Framework choices that run on Android — Kotlin, Java, Jetpack Compose, Android Views, React Native, Expo, native modules — are implementation styles selected by the resolver, not additional targets. A JavaScript bundler or development server used by such a framework is an internal Android build dependency per §5.4 and never a web deliverable.

### 5.6 Android Capability Coverage Matrix

Every Android capability supported by product intent must belong to one of:

| Status | Meaning |
|---|---|
| **SUPPORTED** | Explicit implementation contract, validation path, recovery behavior, and fixture evidence exist. |
| **SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS** | Implementation exists but requires SDK, device, vendor, or toolchain capabilities that may be absent. |
| **DEGRADED** | Capability can operate with a documented reduced behavior. |
| **USER_REQUIRED** | Required credential, device, hardware, approval, or external dependency is unavailable. |
| **UNAVAILABLE** | The current runtime cannot safely provide the capability. |
| **PLANNED** | Accepted product scope but lacks an implemented runtime contract. |

`PLANNED` in the product capability registry means product certification maturity, not "no source implementation exists." A capability may have source code and partial implementation but still be `PLANNED` until its complete contract chain, validation path, recovery behavior, and fixture evidence exist.

Certification scope is explicit: `DOCUMENTATION_CERTIFIED` means contract-graph, canonical-identity, and traceability checks pass; `RUNTIME_CERTIFIED` means the applicable executable fixture and runtime evidence pass; and `PRODUCT_COMPLETED` means the user’s `GoalContract` completion predicate passes. The unqualified word “certified” must not be used to conflate these scopes.

A model response, worker claim, template existence, or code generation attempt does not make a capability SUPPORTED. A capability becomes SUPPORTED only when its implementation contract, validation path, recovery behavior, and fixture evidence exist.

These statuses describe evidence for a registered capability profile; they are not a guarantee of success under arbitrary host, provider, SDK, device, vendor, dependency, signing, or policy conditions. When such conditions prevent completion, the runtime must report the applicable environment or policy status rather than silently narrowing the intent or claiming universal success.

The runtime must report the current status of each capability in the preflight report and must not claim SUPPORTED status for any capability whose fixture evidence is missing.


### 5.7 Capability Registry

The §5.6 matrix defines capability *status*. This section defines capability *identity* and is the addressing source for the traceability chain of §67.3 and the reachability rule of §67.10.

Every user-facing product capability has a stable `CapabilityId`. A capability that is not registered here does not exist for certification purposes, and a contract that no registered capability requires is subject to the orphan rule of §67.10.

| CapabilityId | Requirement | Required contracts | Test id | Evidence id | Status |
|---|---|---|---|---|---|
| CAP.ANDROID.GENERATE | Generate a working Android application from product intent | CONTRACT.RUNTIME.SCOPE, CONTRACT.RUNTIME.PROMPT_CONTRACT, CONTRACT.RUNTIME.AUTHORITY, CONTRACT.RUNTIME.EVIDENCE, CONTRACT.RUNTIME.WORKSPACE, CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | TEST-GEN-001 | EV-GEN-001 | PLANNED |
| CAP.ANDROID.LIVE_PREVIEW | Show a revision-bound, evidence-backed Android runtime preview and reconstruct it after interruption | CONTRACT.RUNTIME.PREVIEW_SYNC | TEST-PSYNC-001 | EV-PSYNC-001 | PLANNED |
| CAP.ANDROID.FRONTEND_CONTROL_PLANE | Operate the desktop UI through authenticated commands, durable projections, replay, and typed errors | CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE | TEST-FCP-001 | EV-FCP-001 | PLANNED |
| CAP.ANDROID.APK_DELIVERY | Deliver a locally verified Android artifact with complete signing, validation, promotion, copy, and post-copy provenance | CONTRACT.RUNTIME.APK_EXPORT | TEST-APK-001 | EV-APK-001 | PLANNED |
| CAP.ANDROID.BACKGROUND_CONTINUITY | Continue, recover, reconcile, or safely stop autonomous work across UI, host, device, and provider interruptions | CONTRACT.RUNTIME.BACKGROUND_CONTINUITY | TEST-BG-001 | EV-BG-001 | PLANNED |
| CAP.ANDROID.BUDGETED_AUTONOMY | Continue autonomous Android work under explicit token, duration, cost, and resource governance | CONTRACT.RUNTIME.COST_GOVERNANCE | TEST-COST-001 | EV-COST-001 | PLANNED |
| CAP.ANDROID.TRUSTED_EXTENSIONS | Use skills, MCP-compatible tools, and plugins only after trust, provenance, permission, and revocation checks | CONTRACT.RUNTIME.AGENT_TRUST | TEST-TRUST-001 | EV-TRUST-001 | PLANNED |
| CAP.ANDROID.CONTEXT_GOVERNANCE | Compact and cache context without evicting constraints, corrupting lineage, or hiding provider telemetry | CONTRACT.RUNTIME.CONTEXT_GOVERNANCE | TEST-CONTEXT-001 | EV-CONTEXT-001 | PLANNED |
| CAP.ANDROID.RUNTIME_INTEGRITY | Report applicable Android runtime integrity, ANR, battery, Doze, and device signals with honest coverage | CONTRACT.RUNTIME.ANDROID_INTEGRITY | TEST-INTEGRITY-001 | EV-INTEGRITY-001 | PLANNED |
| CAP.ANDROID.LONG_HORIZON | Continue a multi-session project without losing settled decisions | CONTRACT.RUNTIME.MEMORY, CONTRACT.RUNTIME.CONTEXT | TEST-MEM-001 | EV-MEM-001 | PLANNED |
| CAP.ANDROID.PARALLEL | Run multiple workers on interdependent code without incoherent merges | CONTRACT.RUNTIME.WORKSPACE, CONTRACT.RUNTIME.RESERVATION | TEST-RES-001 | EV-RES-001 | PLANNED |
| CAP.ANDROID.USER_COEDIT | Let the user edit project files during an active autonomous run | CONTRACT.RUNTIME.RECONCILIATION | TEST-RCN-001 | EV-RCN-001 | PLANNED |
| CAP.ANDROID.E2E_VERIFY | Verify stateful application behavior, not first-screen appearance | CONTRACT.RUNTIME.E2E, CONTRACT.RUNTIME.EVIDENCE | TEST-E2E-001 | EV-E2E-001 | PLANNED |
| CAP.ANDROID.QUALITY_GATE | Prevent unverified mutations from reaching a deliverable | CONTRACT.RUNTIME.VERIFICATION, CONTRACT.RUNTIME.SPECULATION | TEST-VER-001 | EV-VER-001 | PLANNED |
| CAP.ANDROID.REGRESSION_REPAIR | Repair a regression at its cause without broad regeneration | CONTRACT.RUNTIME.LOCALIZATION | TEST-LOC-001 | EV-LOC-001 | PLANNED |
| CAP.ANDROID.SECURE_RELEASE | Produce a packaged artifact with verified dependencies and provenance | CONTRACT.RUNTIME.SUPPLY_CHAIN | TEST-SEC-001 | EV-SEC-001 | PLANNED |
| CAP.ANDROID.DEVICE_COVERAGE | Report honest verification coverage across a emulator profile matrix | CONTRACT.RUNTIME.DEVICE_MATRIX | TEST-DEV-001 | EV-DEV-001 | PLANNED |
| CAP.ANDROID.LIVE_STEER | Change direction mid-run, inspect runtime state, and plan within host capacity | CONTRACT.RUNTIME.DIRECTIVE, CONTRACT.RUNTIME.DEBUGGER, CONTRACT.RUNTIME.PROFILING | TEST-DIR-001 | EV-DIR-001 | PLANNED |
| CAP.ANDROID.AUTOMATED_START | Begin work from an authenticated external event | CONTRACT.RUNTIME.TRIGGER | TEST-TRG-001 | EV-TRG-001 | PLANNED |
| CAP.ANDROID.SKILL_WORKFLOW | Apply reusable domain workflows without granting new permissions | CONTRACT.RUNTIME.SKILL | TEST-SKL-001 | EV-SKL-001 | PLANNED |
| CAP.ANDROID.AUTONOMOUS_REASONING | Decide what to do next from evidence, and delegate within bounded authority | CONTRACT.RUNTIME.REASONING | TEST-RSN-001 | EV-RSN-001 | PLANNED |
| CAP.ANDROID.DEEP_PROBLEM_SOLVING | Spend additional bounded reasoning to solve a hard defect instead of guessing | CONTRACT.RUNTIME.DELIBERATION | TEST-DEL-001 | EV-DEL-001 | PLANNED |
| CAP.ANDROID.CERTIFIED_RELEASE | Promote a release only when runtime invariants hold and platform capability states (host, target, validation, certification) are truthful and evidence-bound | CONTRACT.RUNTIME.INVARIANTS, CONTRACT.RUNTIME.INTEGRATION_BOUNDARY, CONTRACT.RUNTIME.PLATFORM_CAPABILITY, CONTRACT.RUNTIME.AGENT_BUILDABILITY | TEST-INV-001 | EV-INV-001 | PLANNED |
| CAP.PLATFORM.CAPABILITY_TRUTH | Classify and report build, cross-build, and target-runtime capability states truthfully, with host, target, validation, and certification kept distinct and evidence-bound | CONTRACT.RUNTIME.PLATFORM_CAPABILITY | TEST-PLAT-001 | EV-PLAT-001 | PLANNED |

Capability status uses the §5.6 vocabulary. `PLANNED` here means the capability has an accepted contract chain but no implemented runtime; it must not be reported as `SUPPORTED` until its test id produces its evidence id, per §67.5.

### 5.7.1 Internal capability-profile identity

Capability status is not sufficient to identify the exact Android environment in which a capability was certified. Every implementation profile used for planning, validation, or support reporting must have a stable internal `ProfileId` and a durable profile record containing:

```text
AndroidCapabilityProfile
- profileId
- capabilityIds
- technologyComposition
- toolchainLock
- androidApiLevels
- deviceMatrix
- requiredEnvironment
- fixtureIds
- knownExclusions
- brandingAndAssetRequirements
- repositoryTrustRequirement
- environmentIdentity
- requiredIntegrationStates
- signingPolicy
- reproducibilityLevel
- testIds
- evidenceReportIds
- status
- adapterId
- adapterVersion
- technologyPlanHash
- buildStrategyId
- previewStrategyId
- runtimeStrategyId
- validationStrategyId
- certifiedRevision
```

`toolchainLock` defines the concrete Android toolchain versions pinned for this profile: AGP, Gradle wrapper, JDK vendor + major, compileSdk, targetSdk, minSdk, Build Tools, Kotlin, Compose BOM, and NDK when applicable. The lock MUST be resolved and recorded per project revision and MUST contribute to the environment fingerprint that binds preview and evidence currentness (CLAUSE.PLATFORM.EVIDENCE_ENV_BINDING, CLAUSE.PREVIEW_SYNC.IDENTITY_MATCH). A pinned JDK MUST be used with a per-process JAVA_HOME; Nirman MUST NOT depend on or mutate the machine-wide JAVA_HOME. These versions have hard mutual constraints — a given AGP requires a minimum Gradle and a specific JDK major and caps usable compileSdk. Incompatible combinations MUST be rejected at preflight naming the violated constraint, before any build starts. Record concrete versions as the CURRENT lock with an explicit revision date, not as permanent truth.

A profile may describe an internally selected composition of Java, Kotlin, Compose, Views, React Native/Expo, native modules, device APIs, or mixed technologies. It is an implementation identity, not a user-facing template, archetype, starter project, or framework picker. `SUPPORTED` or `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS` may be reported only for the declared profile and its evidence; support must not be generalized to every possible Android project.

### 5.7.2 Canonical maturity and operational state separation

Capability maturity, product lifecycle, assurance, integration operationality, signing, artifact, preview, and delivery are separate state dimensions. They MUST NOT be represented by one overloaded status field or inferred from model text.

```text
ProductLifecycleState = CREATED | PLANNING | SYNTHESIZING | IMPLEMENTING |
                        PREVIEWING | VALIDATING | RECOVERING | PACKAGING |
                        COMPLETED | BLOCKED | USER_REQUIRED | CANCELLED |
                        SAFELY_FAILED
AssuranceState        = UNKNOWN | PREDICTED | SIMULATED | OBSERVED | VERIFIED |
                        CERTIFIED | STALE | INVALIDATED
CapabilityMaturity    = SPECIFIED | IMPLEMENTED | VERIFIED | CERTIFIED |
                        DEGRADED | BLOCKED | UNKNOWN
IntegrationState      = NOT_REQUIRED | SPECIFIED | CONFIGURED | REACHABLE |
                        FUNCTIONAL | DEGRADED | USER_REQUIRED | UNAVAILABLE |
                        BLOCKED | UNKNOWN
SigningState          = NOT_REQUIRED | UNSIGNED_DEBUG | CONFIGURED |
                        AUTHORIZED | IN_PROGRESS | SIGNED_OBSERVED |
                        INSPECTED | FAILED | BLOCKED | UNKNOWN
DeliveryState         = NOT_REQUESTED | ELIGIBLE | EXPORTING | EXPORTED |
                        DELIVERED | FAILED | BLOCKED | UNKNOWN
```

`RUNNING` describes lifecycle or process activity; it does not imply `OBSERVED`, `VERIFIED`, or `COMPLETED`. `DELIVERED` proves a successful local handoff, not that every optional integration or release-signing condition passed. `CERTIFIED` is permitted only after the required executable fixtures and evidence gates pass.

### 5.7.3 Canonical artifact and delivery policy

The minimum local Android deliverable is an installable APK. AAB generation is an optional separately declared release artifact and is never implied by APK completion. A task MUST declare its `PackagingProfile` before packaging:

```text
PackagingProfile
- profileId
- requiredArtifacts: APK | APK_AND_AAB
- buildVariant
- signingPolicy
- reproducibilityPolicy
- requiredInstallabilityChecks
- requiredEvidenceKinds
```

Every artifact belongs to an `ArtifactSet` with a shared source revision, asset manifest, toolchain lock, environment identity, validation policy, and evidence ledger. An optional AAB is a separate artifact record with its own signing, inspection, and promotion evidence. The runtime MUST NOT use the phrase “APK” as an undefined completion condition.

### 5.7.4 Evidence dependencies and cascading invalidation

Evidence is a dependency graph rather than an unqualified list. The canonical chain is:

```text
Observation → EvidenceArtifact → ValidationResult → CertificationDecision → CompletionDecision
```

Each evidence node MUST record source event, operation, session, project revision, checkpoint, artifact or preview identity when applicable, device and toolchain identity when applicable, validation-policy version, freshness interval, dependency IDs, supersession, and invalidation reason. If a source revision, asset manifest, toolchain lock, emulator session, dependency snapshot, validation policy, or required integration changes, every dependent evidence and completion claim MUST be invalidated unless independence is proven by the dependency graph.

`PreviewPromotionGate`, `ArtifactAuthority`, `AndroidQualityGate`, and the completion evaluator MUST consume the same dependency and invalidation relation. A model claim, plan record, simulated result, or isolated worker statement is never completion evidence.

### 5.7.5 Required integration operationality

A required outbound API, authentication service, database service, or other external integration MUST declare its minimum acceptable operational state. Every required integration and every operation that crosses a process, IPC, device, provider, credential, workspace, artifact, or external-service boundary MUST also declare an `IntegrationBoundaryContract`. Client code existing in the project, a successful build, or a successful app launch does not prove that the integration is functional.

Operationality is multidimensional. `CONFIGURED` does not imply valid credentials, `REACHABLE` does not imply authentication, `AUTHENTICATED` does not imply functional behavior, and `FUNCTIONAL` does not imply that the user’s acceptance criteria have passed.

```text
IntegrationOperationality
- integrationId
- required: boolean
- endpointIdentity
- credentialReference
- schemaVersion
- policyProfile
- connectivityState: UNKNOWN | UNREACHABLE | REACHABLE
- authenticationState: NOT_REQUIRED | UNKNOWN | INVALID | AUTHENTICATED
- availabilityState: UNKNOWN | UNAVAILABLE | AVAILABLE | DEGRADED
- functionalState: UNKNOWN | NON_FUNCTIONAL | FUNCTIONAL
- acceptanceState: NOT_REQUIRED | UNKNOWN | NOT_ACCEPTED | ACCEPTED
- aggregateState: NOT_REQUIRED | SPECIFIED | CONFIGURED | REACHABLE |
                  FUNCTIONAL | DEGRADED | USER_REQUIRED | UNAVAILABLE |
                  BLOCKED | UNKNOWN
- healthEvidenceId
- authenticationEvidenceId
- functionalEvidenceId
- acceptanceEvidenceId
- lastObservedAt
- invalidatedBy
```

For example, an endpoint returning `401 Unauthorized` may be `CONFIGURED` and `REACHABLE` while remaining unauthenticated and non-functional. When safe test access is unavailable, the runtime MUST report `USER_REQUIRED`, `UNAVAILABLE`, `BLOCKED`, or `UNKNOWN`; it MUST NOT report complete merely because local code compiled.

### 5.7.6 External-effect reconciliation

Every remote or externally visible side effect MUST be represented by an `ExternalEffectRecord` with an idempotency key, target identity, authority grant, request fingerprint, request state, response reference, compensation plan, and local transaction. The record MUST reference the applicable `IntegrationBoundaryContract`. If the response is lost after transmission may have occurred, the runtime MUST reconcile by idempotency key or read-back before retrying or declaring failure. Local rollback MUST NOT be described as undoing a remote effect unless compensation evidence proves it.

### 5.7.7 Completion predicate and illegal-state rules

Certification and completion are different decisions and MUST never be treated as synonyms. `CertificationDecision` answers whether an artifact or revision satisfies the declared technical certification policy. `CompletionDecision` answers whether the user’s goal contract is satisfied, including mandatory integrations and product requirements. Therefore, `CERTIFICATION ≠ COMPLETION`: an APK may be technically certified while goal completion remains `NOT_COMPLETE` because a required backend is unavailable.

The sole completion evaluator MUST require the declared goal conditions, current mandatory evidence, valid dependencies, appropriate capability maturity, required integration operationality, preview gate when required, artifact and signing policy, reproducibility policy, and absence of blocking contradictions. At minimum, the following combinations are illegal and MUST be rejected:

- `COMPLETED` with missing or invalid mandatory evidence;
- `VERIFIED` based only on model, plan, or simulated records;
- `CURRENT` preview with stale source, artifact, asset, toolchain, or emulator identity;
- `SUPPORTED` capability without a matching certified profile or fixture evidence;
- `DELIVERED` artifact without checksum and artifact inspection;
- `FUNCTIONAL` integration without successful functional evidence;
- `SIGNED_OBSERVED` or `INSPECTED` with an unknown signing outcome;
- `CERTIFIED` profile with an expired or invalidated evidence report.

These predicates are normative. Explanatory UI text and reasoning summaries may describe them but cannot replace them.

### 5.7.8 Android target and provider-context boundaries

The generated target predicate is machine-checkable:

```text
TargetPlatformSet == {ANDROID}
project.targetPlatforms == ["android"]
```

Supporting backend services, build tools, native modules, provider adapters, and development utilities MAY exist when required by an Android application, but no resolver path may produce a second generated deployable target. Android-only describes the generated product target, not a prohibition on supporting components. An Android service integration MUST identify its request/response schemas, authentication reference, datastore owner, privacy and network policy, functional scenarios, and required evidence; it remains a supporting dependency rather than a second generated product target. Cloud-provider context transmission is governed by a typed envelope:

```text
ProviderContextEnvelope
- dataClassification
- providerPolicyId
- selectedContextIds
- redactionPolicyId
- userApprovalPolicyId
- allowedPurpose
- retentionPolicy
- transmissionDecision: ALLOWED | REDACTED | USER_REQUIRED |
                         BLOCKED | NOT_TRANSMITTED
- providerRequestId
```

Only the minimum context required for the declared purpose may be transmitted. Secrets, private reasoning, unrelated personal data, protected credentials, and excluded project content MUST be withheld. A provider response cannot broaden the envelope, permissions, target set, or completion authority.

### 5.7.9 Capability promotion, signing identity, and compatibility

Capability maturity follows a deterministic promotion chain:

```text
CapabilityEvidence
  → CapabilityValidation
  → CapabilityCertification
  → CapabilityPromotionAuthority
  → immutable promotion record
```

Workers and models may propose status changes but cannot write `SUPPORTED`, `VERIFIED`, or `CERTIFIED` directly. Promotion requires the matching profile, fixture IDs, current evidence, environment identity, and policy version.

Release signing requires an immutable binding:

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

Contract and controller changes require an explicit compatibility record:

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

A candidate controller or contract migration cannot be promoted until compatibility, migration, replay, restart, rollback, and evidence-invalidation fixtures pass.

## 6. High-Level Architecture

Nirman should be composed as a Windows-first desktop application with independent internal modules. The architecture should allow the project runtime and agent system to evolve without coupling the interface to one particular AI provider.

```text
┌────────────────────────────────────────────────────────────┐
│                      Nirman Desktop App                 │
├────────────────────────────────────────────────────────────┤
│ Chat Workspace │ File Tree │ Editor │ Preview │ Logs │ UI   │
├────────────────────────────────────────────────────────────┤
│             Application State and Project Context            │
├────────────────────────────────────────────────────────────┤
│      Agent Orchestrator and Structured Tool Protocol         │
├────────────────────────────────────────────────────────────┤
│ Provider Adapter │ Project Index │ Policy Engine │ Checkpoint │
├────────────────────────────────────────────────────────────┤
│ Local Runtime: Node │ Package Manager │ Git │ Build Tools    │
├────────────────────────────────────────────────────────────┤
│ Android project │ Android runtime │ Expo/React Native │ APK artifacts │
└────────────────────────────────────────────────────────────┘
```

### 6.1 Desktop shell

The desktop shell should use C#/.NET + WinUI 3. The shell is responsible for opening project folders, communicating with the local runtime, presenting native dialogs, storing secure credentials through the operating-system keychain, and managing application-level settings.

### 6.1.1 Windows host process contract

Nirman has exactly one user-facing product/application identity.

Production deployment consists of:
- `Nirman.exe` — visible Windows desktop client
- `NirmanSupervisor.exe` — headless local runtime

These are not separate user-facing applications.

Required invariants:
1. One installer/package installs both.
2. One product identity is presented to the user.
3. Supervisor has no normal user-facing window.
4. Supervisor requires no manual launch/configuration.
5. Nirman automatically starts/reconnects to the compatible supervisor.
6. UI closure/minimization does not terminate eligible autonomous tasks.
7. UI restart reconnects to durable supervisor state.
8. Supervisor and UI versions must remain compatibility-bound.
9. Supervisor lifecycle failures must be recoverable and visible through Nirman.
10. The user must never need to operate the supervisor independently.

### 6.2 Frontend interface

The frontend should contain the chat workspace, project selector, file tree, editor, preview frame, terminal panel, test panel, provider settings, environment diagnostics, and export controls.

The interface should maintain a clear distinction between generated text and executed actions. A message saying that a command will run is different from a confirmed command result, and the interface must represent those states separately.

### 6.3 Local runtime

The local runtime manages Android project processes and development tools. It should be responsible for starting and stopping Metro or native development servers, managing Gradle and Android build processes, reading Logcat and process output, enforcing timeouts, checking ports, managing emulators and devices, running tests, capturing screenshots, and collecting APK artifacts.

It should never assume that a tool exists. Before invoking a command, it should verify the required executable and display a diagnostic if the environment is incomplete.

### 6.4 Agent orchestrator

The agent orchestrator should be a stateful task engine rather than a single prompt call. It should maintain:

| State category | Examples |
|---|---|
| User intent | Original request, clarifications, acceptance criteria |
| Project context | Framework, files, dependencies, scripts, environment |
| Current plan | Tasks, dependencies, completed steps, blocked steps |
| Execution history | Commands, outputs, errors, screenshots, test results |
| Change history | Checkpoints, diffs, restored versions |
| Provider state | Selected model, capabilities, token limits, failures |
| Safety state | Approved paths, commands, network permissions, budgets |

### 6.5 Project context and indexing

Nirman should not send the entire project to the model on every request. It should maintain a lightweight index of files, symbols, routes, components, configuration, scripts, and recent changes. The context selector should retrieve only the files and summaries relevant to the current task.

The project index should be refreshed after manual edits, generated changes, dependency installation, and branch or checkpoint changes.

### 6.6 Checkpoint and recovery system

Before an autonomous task changes multiple files, Nirman should create a checkpoint. A checkpoint should record the project revision, task description, generated plan, provider identity, and changed-file set.

The user should be able to undo the whole task, restore a previous checkpoint, or inspect a file-by-file diff. Git should be used where available, but Nirman should still provide understandable recovery behavior for projects that are not yet Git repositories.

---

## 7. Agent Tool Protocol

The AI model should interact with Nirman through structured tools. The model should not directly receive an unrestricted terminal or filesystem interface.

### 7.1 Required tools for Version 1

| Tool | Function |
|---|---|
| `inspect_project` | Detect framework, scripts, dependencies, entry points, and project health |
| `search_files` | Find relevant files and symbols using paths, text, or patterns |
| `read_file` | Read selected file content with line ranges |
| `write_file` | Create a new file inside the approved workspace |
| `patch_file` | Apply a targeted modification to an existing file |
| `delete_file` | Remove a file only after approval or policy validation |
| `create_checkpoint` | Save a reversible project state |
| `run_command` | Execute an approved local command with timeout and output capture |
| `start_preview` | Start the development server and return its address |
| `stop_preview` | Stop a Nirman-managed development process |
| `capture_screenshot` | Capture the current preview for visual inspection |
| `run_checks` | Run linting, type checks, tests, and build validation |
| `show_diff` | Return changed files and a human-readable summary |
| `export_project` | Create a source/workspace export or a declared build artifact |

`export_project` does not make a ZIP or Git bundle a deployment artifact. Source and project access remain user-owned workspace operations. Deployment delivery is governed separately by `PackagingProfile`: an installable APK is required for local completion, and AAB is produced only when an explicitly declared packaging profile requires it.

### 7.2 Agent task lifecycle

```text
1. Receive user request.
2. Inspect project and identify the current state.
3. Extract requirements and acceptance criteria.
4. Ask clarifying questions only when necessary.
5. Produce an implementation plan.
6. Create a checkpoint.
7. Apply small, logically grouped file changes.
8. Run the preview or relevant development process.
9. Run checks and inspect errors.
10. Capture and inspect a screenshot when visual behavior matters.
11. Repair detected issues within the configured attempt limit.
12. Present the final diff, checks, warnings, and remaining work.
```

### 7.3 Failure handling

The agent should classify failures instead of blindly retrying. Useful categories include missing tool, dependency failure, syntax error, type error, runtime error, visual issue, permission issue, network failure, provider failure, and ambiguous requirement.

If the same failure appears repeatedly, the agent should stop and explain the problem. It should include the relevant command, error output, suspected cause, attempted fixes, and the specific user action required.

---

## 8. AI Provider Configuration

### 8.1 Provider settings requirements

Nirman should allow users to configure their own AI provider without changing application code.

| Configuration field | Required behavior |
|---|---|
| Provider label | User-defined friendly name |
| Compatibility mode | One of `OPENAI_COMPATIBLE` or `ANTHROPIC_COMPATIBLE`, selected by the user per ADR-208 |
| Base URL | Custom provider endpoint. MUST be a network-reachable cloud endpoint; localhost, loopback, and RFC-1918 private ranges MUST be rejected at configuration time per ADR-207. |
| API key | Stored securely in the operating-system keychain |
| Chat model ID | Model used for planning and code generation |
| Vision model ID | Optional model used for screenshot and preview analysis |
| Embedding model ID | Optional model used for project retrieval |
| Token limit | Provider-specific output limit |
| Temperature | Optional creativity control |
| Reasoning capability | Whether the selected model supports provider-native reasoning |
| Reasoning effort levels | Provider-supported mapping for NORMAL, EXTENDED, DEEP, and EXHAUSTIVE |
| Maximum reasoning tokens | Provider-reported or configured upper bound when supported |
| Reasoning usage reporting | Whether reasoning-token or equivalent effort usage is reported, estimated, or unavailable |
| Reasoning configuration | Provider-specific settings normalized by the ModelGateway |
| Timeout | Maximum provider request duration, bounded by the active deliberation budget |
| Enabled capabilities | Text, vision, structured output, tool calling, reasoning, embeddings |
| Test connection | Sends a safe validation request. MUST pass before Save is permitted per ADR-208; changing key, base URL, model ID, or mode invalidates the prior pass and re-disables Save. |

### 8.2 Provider adapter interface

The internal provider interface must normalize differences between services. It must support text generation, structured JSON output, tool calls, vision input, streaming responses, cancellation, error normalization, capability discovery, reasoning-effort configuration, reasoning-token accounting, context-capacity discovery, and provider-specific continuation behavior.

The adapter resolves the declared compatibility mode to a request family — `OPENAI_COMPATIBLE` to Chat Completions / Responses-style, `ANTHROPIC_COMPATIBLE` to message-oriented — and both normalize to the same internal result. Anthropic-compatible endpoints carry the system prompt as a top-level parameter rather than a message role, and use distinct tool-use and tool-result block shapes; the normalizer MUST account for both without exposing the difference to workers.

The normalized provider capability descriptor must distinguish:

- native reasoning support;
- supported reasoning effort levels;
- maximum reasoning-token capacity when known;
- whether reasoning usage is provider-reported, estimated, or unavailable;
- whether reasoning effort can be changed between requests;
- whether reasoning continues across provider requests;
- whether the provider supports background or asynchronous requests.

Provider-specific reasoning controls must never be passed through as opaque authority-bearing settings. The ModelGateway maps the runtime's normalized effort request to provider-specific parameters and records the mapping.

A provider that cannot represent the requested reasoning effort must not claim that it did so. The runtime must either downgrade the effort level according to policy and record the constraint, select another approved provider/model, or terminate deliberation with a typed capability gap.

A provider adapter should return a normalized result containing the model ID, response text, tool calls, structured output, reasoning usage when available, capability metadata, usage information, finish reason, request duration, and any provider warning.

### 8.3 Privacy behavior

Nirman must clearly communicate whether project content is being sent to a cloud model. The user should be able to configure context policies that exclude selected files, folders, secrets, generated binaries, or sensitive project types.

Only cloud-hosted, network-reachable AI providers are supported. Local, offline, on-device, and self-hosted model runtimes are out of scope per ADR-207.

### 8.4 Credential rules

API keys must not be stored in ordinary JSON settings, project files, logs, prompts, Git commits, or generated source code. The application should mask keys in all displayed output and provide a “remove credentials” action.

---

## 9. Local Execution and Environment Management

### 9.1 Local execution policy

Generated applications should execute locally. Nirman should manage only processes that it started or that the user explicitly connected to the project.

Each managed process should have a process ID, working directory, start time, command, port, environment profile, output stream, and stop action.

### 9.2 Environment diagnostics

Nirman should detect the presence and versions of tools required by the selected project type.

| Project type | Example required tools |
|---|---|
| Android JavaScript project | Node.js, package manager, Metro or bundler runtime |
| Android build | Node.js, package manager, Java, Gradle, Android SDK, platform-tools, Nirman-managed local Android emulator |
| Expo Android | Node.js, package manager, Java, Android SDK, Nirman-managed local Android emulator when used |
| Git export | Git executable and repository permissions |

The diagnostic screen should distinguish between installed, missing, outdated, misconfigured, and inaccessible tools. It should provide a command or official installation reference where appropriate.

Diagnostics are per-tool state. Platform capability state — what this host can build, cross-build, and validate for a declared target platform — is a separate classification defined by §79 and recorded in the `EnvironmentCapabilityRecord`. A tool being installed on the host does not by itself establish target-platform runtime capability, validation capability, or certification capability.

### 9.3 Process controls

The runtime should implement command timeouts, output truncation limits, process termination, port conflict detection, memory safeguards where available, and cancellation from the user interface.

Commands should be categorized as safe, reviewable, or privileged. Safe commands can run automatically within the workspace. Reviewable commands require approval according to the user’s policy. Privileged commands always require explicit approval.

Terminal execution must support persistent per-worker sessions with a working directory, environment snapshot, shell type, process group, and session identifier. The runtime must detect interactive prompts, provide a controlled input channel, apply an unattended prompt policy, and terminate or recover processes that wait for input beyond the configured liveness window. On Windows, the shell profile must explicitly identify PowerShell, `cmd.exe`, Git Bash, or another approved native-Windows shell and record the selected shell in the task evidence. The interface should show multiple worker terminals separately, with searchable rolling logs and preserved raw artifacts for long-running processes.

### 9.4 Network behavior

Network access should be visible. Dependency installation and API calls may require network access, but the user should see when it is being requested. Generated application runtime traffic should be distinguishable from Nirman’s own provider and package-manager traffic.

---

## 10. Security and Trust Model

### 10.1 Workspace boundary

By default, Nirman may read and write only inside the selected project workspace and its approved temporary build directories. Access outside that boundary requires an explicit user decision.

### 10.2 Command safety

The command runner should validate the executable, arguments, working directory, and requested permissions. It should reject suspicious path traversal, unexpected shell chaining, destructive commands, and commands that attempt to access protected locations unless the user explicitly approves them.

### 10.3 Dependency safety

The application should show which dependencies will be installed and why. It should record package versions and update the project lockfile. Dependency installation should be treated as a network operation and should be visible in the activity log.

### 10.4 Secret protection

Nirman should detect likely secrets in files and prevent them from entering model context or logs by default. It should warn before showing environment files in the editor and should never include secrets in generated explanations.

### 10.5 Release and publishing controls

Building a local artifact is different from publishing it. Nirman should require explicit confirmation before release signing, uploading, publishing, distributing, or submitting an application to an external service.

### 10.6 Auditability

The activity log should record the task ID, user request, provider, model ID, files read, files changed, commands executed, network actions, approvals, test results, and artifact paths. The log should be exportable for troubleshooting.

---

## 11. Data Model

Nirman should maintain local metadata separate from generated application source files.

### 11.1 Project record

```text
Project
- id
- name
- rootPath
- projectType
- framework
- targetPlatforms          # invariant: must equal exactly ["android"]
- createdAt
- updatedAt
- activeCheckpointId
- providerProfileId
- autonomyPolicyId
```

### 11.2 Provider profile

```text
ProviderProfile
- id
- label
- baseUrl
- keychainReference
- chatModelId
- visionModelId
- embeddingModelId
- capabilities
- requestSettings
- createdAt
- updatedAt
```

The actual API key should not be stored in this record. The record should contain only a secure keychain reference.

### 11.3 Agent task

```text
AgentTask
- id
- projectId
- userRequest
- specification
- acceptanceCriteria
- plan
- status
- currentStep
- attemptCount
- tokenUsage
- createdAt
- completedAt
- failureReason
```

### 11.4 Action record

```text
ActionRecord
- id
- taskId
- actionType
- argumentsSummary
- approvalStatus
- startedAt
- completedAt
- exitCode
- outputSummary
- affectedFiles
```

### 11.5 Checkpoint

```text
Checkpoint
- id
- projectId
- taskId
- revisionReference
- description
- changedFiles
- createdAt
```

---

## 12. MVP Functional Requirements

### 12.1 Project management

The user must be able to create a new Android project by describing it, open an existing local project, rename a project, close a project, inspect project health, and select the active AI provider.

Project creation is described, not selected from a catalogue. The user states product intent; the technology resolver of §5 chooses the Android implementation stack, and any internal scaffold it uses is an implementation detail of that resolution. Nirman must not present a template picker as the primary creation path, because doing so would move the resolver's decision into the interface.

### 12.2 Chat-driven generation

The user must be able to describe a new application or request a change to an existing project. Nirman must display the agent’s understanding, plan, actions, progress, validation results, and final summary.

### 12.3 Code and diff management

The user must be able to inspect generated files, review diffs, accept or reject grouped changes, manually edit files, create checkpoints, undo a task, and restore an earlier checkpoint.

### 12.4 Preview and validation

Nirman must start a local preview for supported Android projects, show the Nirman-managed local Android emulator preview inside the application, display runtime errors, run linting and type checks, capture screenshots, and present validation results in a readable form.

### 12.5 Provider settings

The user must be able to create, edit, test, select, and delete provider profiles. The interface must support a custom base URL, API key, and model ID as first-class settings.

### 12.6 Export

The user must be able to export source code as a ZIP archive, export or initialize a Git repository, and create a supported local build artifact.

### 12.7 Diagnostics

The user must be able to inspect installed tool versions, missing dependencies, provider connection state, active processes, port conflicts, and recent task failures.

---

## 13. Non-Functional Requirements

| Area | Requirement |
|---|---|
| Reliability | Failed tasks should stop safely and preserve the last valid checkpoint |
| Transparency | File changes and commands must be visible to the user |
| Responsiveness | The interface should remain usable while builds and model requests run |
| Cancellation | Long-running model requests and processes must be cancellable |
| Privacy | Credentials and excluded files must not be exposed in logs or prompts |
| Portability | Generated projects must remain usable outside Nirman |
| Extensibility | New Android technology adapters and AI providers should be addable independently |
| Recoverability | Users must be able to undo autonomous tasks |
| Accessibility | Keyboard navigation, readable contrast, and visible status states are required |
| Maintainability | Agent tools, provider adapters, internal Android bootstraps, and UI should have separate boundaries |

---

## 14. Suggested Application Directory Structure

```text
Nirman/
├── app/
│   ├── desktop-shell/
│   ├── frontend/
│   └── shared-types/
├── agent/
│   ├── orchestrator/
│   ├── tools/
│   ├── policies/
│   ├── context/
│   └── prompts/
├── runtime/
│   ├── process-manager/
│   ├── environment-diagnostics/
│   ├── preview-manager/
│   ├── test-runner/
│   └── artifact-builder/
├── providers/
│   ├── provider-interface/
│   ├── compatible-provider/
│   ├── local-models/
│   └── capability-detection/
├── android_bootstraps/
│   ├── expo-react-native/
│   ├── android-native-compose/
│   └── android-device-profiles/
├── storage/
│   ├── project-metadata/
│   ├── checkpoints/
│   └── activity-logs/
├── docs/
└── tests/
```

The exact repository layout may change during implementation, but the boundaries should remain clear. The desktop interface should not contain the complete agent implementation, and provider-specific behavior should not be scattered through the user interface.

---

## 15. Implementation Roadmap

### Phase 1: Desktop shell and workspace

Create the Windows-first desktop application shell, project picker, basic layout, settings navigation, local metadata storage, and secure credential storage abstraction.

**Exit criteria:** The application launches, creates or opens a local workspace, displays the main layout, and stores non-secret settings correctly.

### Phase 2: Provider configuration

Implement provider profiles with custom base URL, API key, model ID, optional vision model, connection testing, capability detection, masked display, and keychain integration.

**Exit criteria:** A user can configure a compatible cloud provider and receive a validated response without exposing the API key in logs or project files.

### Phase 3: Intent-to-Android contract and dynamic project synthesis

Create the AndroidConstructionContract from the user’s intent, screenshots, assets, constraints, and device requirements. Resolve an Android technology plan and synthesize the required project structure, resources, build configuration, tests, and preview target. Any internal bootstrap is an implementation detail and must not appear as a selectable user-facing template.

**Exit criteria:** A new project can be created locally, installed, started, and opened in the live preview.

### Phase 4: Structured agent tools

Implement project inspection, file search, file reading, file creation, targeted patches, checkpoints, command execution, and diff reporting.

**Exit criteria:** The agent can make a small, reviewable change to a project and show the complete action history.

### Phase 5: Autonomous development loop

Add task planning, acceptance criteria, grouped changes, preview startup, screenshot capture, linting, type checking, tests, error classification, repair attempts, cancellation, and failure escalation.

**Exit criteria:** Nirman can complete a common feature request, validate the result, repair at least common implementation failures, and stop safely when blocked.

### Phase 6: Android packaging and artifact export

Add Git export, Android debug/release build artifacts, APK packaging, signing configuration boundaries, artifact metadata, checksums, and Android build diagnostics.

**Exit criteria:** A supported Android project can produce a validated APK artifact, or an AAB artifact only when the active PackagingProfile requires `APK_AND_AAB`, and the user can locate the result with its build metadata and validation report.

### Phase 7: Android generation

Add autonomous Android technology resolution across native Android, Kotlin/Compose, Java/Views, React Native/Expo, native modules, and mixed architectures, together with environment diagnostics, Nirman-managed local Android emulator connection information, Android logs, and APK build workflows where the local environment supports them.

**Exit criteria:** Nirman can create and build a supported Android project and clearly identify environmental limitations.

### Phase 8: Advanced features

Add visual element selection, project memory, reusable components, Android data and authentication capabilities, multi-agent task specialization, regression screenshots, and more native Android capability profiles.

**Exit criteria:** Advanced capabilities remain optional and do not reduce the reliability of the core Android workflow or the Windows desktop host.

---

## 16. Acceptance Criteria for the First Usable Release

The first usable release should satisfy the following conditions:

1. A new user can install Nirman on Windows and understand its purpose without external documentation.
2. A user can configure a custom AI base URL, API key, and model ID.
3. A user can create an Android project from a natural-language request and optional screenshots without selecting a framework or template.
4. A user can ask Nirman to add a feature through chat.
5. The application shows a plan before performing a multi-file change.
6. The user can approve, reject, undo, and inspect generated changes.
7. The project can run locally and display a live preview inside Nirman.
8. Nirman can run linting, type checks, tests, or a build command and display the result.
9. The application preserves a checkpoint before autonomous changes.
10. The agent stops after repeated failure and explains what is blocked.
11. API keys do not appear in source files, logs, prompts, or exported projects.
12. The user can export the resulting source code independently of Nirman.
13. The application never requires cloud code execution for the supported Android workflow.
14. The application clearly distinguishes cloud AI processing from local AI processing.

---

## 17. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| The generated project becomes inconsistent | High | Use templates, project specifications, incremental patches, and checkpoints |
| The agent enters an infinite repair loop | High | Enforce attempt, time, and token limits; classify repeated failures |
| Local toolchains are missing | High | Provide environment diagnostics and supported-version guidance |
| A generated command is unsafe | High | Use command policies, path restrictions, approvals, and process isolation |
| Cloud providers receive sensitive project data | High | Provide context exclusions, privacy notices, and redaction |
| Different models support different tool protocols | Medium | Normalize providers through adapters and capability discovery |
| The UI looks correct but behavior is broken | Medium | Combine visual screenshots with tests, type checks, and runtime inspection |
| Universal framework support increases complexity too quickly | High | Add broader Android capability fixtures only after the core workflow is reliable |
| Users expect a perfect final product from one prompt | High | Show supported capabilities, validation status, and remaining risks |

---

## 18. Recommended Initial Screens

### Welcome screen

The welcome screen should explain Nirman in one sentence, offer “Create project” and “Open project,” and show whether an AI provider is configured.

### Project workspace

The project workspace should contain the chat, file tree, editor or preview, activity stream, and bottom logs panel. The most important toolbar actions should be Run, Stop, Checkpoint, Undo, Build, and Export.

### Provider settings

The provider settings screen should allow users to add profiles, enter a base URL, API key, and model ID, test the connection, select model capabilities, and remove credentials.

### Environment diagnostics

The diagnostics screen should show required tools, detected versions, missing tools, project health, provider status, active processes, port usage, and recent errors.

### Task review

The task review screen should show the original request, implementation plan, changed files, commands, test results, warnings, and buttons for keeping or restoring the work.

---

## 19. Recommended Development Strategy

The project should be built in vertical slices rather than by completing every subsystem separately. Each slice should produce a usable part of the application.

The first vertical slice should allow the user to open Nirman, configure a provider, describe any supported Android application in chat, optionally attach screenshots, receive a technology-selection plan, synthesize a project, apply a small file change, start an emulator or emulator preview, and inspect the result.

The second slice should add checkpoints, diffs, tests, repair attempts, Android Nirman-managed local Android emulator preview, and cancellation. The third should add Android packaging, APK artifacts, signing boundaries, and emulator validation.

The team should maintain a fixture library of representative projects and tasks. Each agent change should be evaluated against these fixtures for code correctness, preview startup, test results, changed-file scope, and safe failure behavior.

---

## 20. Final Product Direction

Nirman should be a polished, minimal Windows desktop application that puts a controlled autonomous software-development loop inside one workspace. Its differentiator should not be the existence of a chat box. Its differentiator should be the combination of:

- Local project execution.
- User-configurable cloud and local AI providers.
- Reliable structured code changes.
- Live preview and visual inspection.
- Tests and automatic repair attempts.
- Checkpoints, diffs, and recovery.
- Android packaging, emulator validation, and later specialized native Android profiles.
- Clear security and privacy controls.

The most important strategic decision is to build the **local Android runtime, technology resolver, visual synthesis pipeline, and agent execution system first**. The system should become broadly capable by composing Android technologies from requirements rather than by maintaining a narrow template catalog.

---

## 21. Suggested Next Build Sequence

1. Create the Nirman desktop shell.
2. Implement the workspace layout with chat, file tree, editor, preview, and logs.
3. Add provider profiles with custom base URL, API key, and model ID.
4. Add secure keychain storage and connection testing.
5. Implement dynamic Android project synthesis, technology resolution, and screenshot-to-project analysis.
6. Implement the structured file and command tools.
7. Add checkpoints and diff review.
8. Add local preview and runtime diagnostics.
9. Add linting, type checking, tests, and repair loops.
10. Add Android packaging and APK artifact export.
11. Add full Android technology coverage, native integration, and device capabilities.
12. Add advanced visual editing, project memory, technology-plan inspection, and screenshot comparison.

---

## 22. Advanced Autonomous Development and Swarm Execution Capabilities

**ContractId:** `CONTRACT.RUNTIME.WORKSPACE`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.WORKSPACE` (see §67.8)

This section incorporates advanced patterns from modern autonomous agent frameworks—specifically focusing on **parallel agent orchestration (swarms)**, **long-running continuous background execution**, **persistent problem-solving loops**, **anti-thrashing error recovery**, and **shared task state coordination**. All descriptions are tailored to Nirman's local desktop application architecture without referencing external agent brand names.

### 22.1 Parallel Agent Orchestration (Swarm Architecture)

To prevent the latency and scalability bottlenecks of traditional sequential tool execution, Nirman should support a **Parallel Swarm Orchestrator**. When a user requests a complex application feature or multi-module refactor, the main orchestrator decomposes the objective into orthogonal sub-tasks and delegates them to specialized background workers operating concurrently.

| Canonical worker role | Responsibility | Execution Boundary |
|---|---|---|
| Primary Orchestrator | Goal decomposition, routing, synthesis, and task-graph coordination | Main session context; no direct file mutation |
| Repository Scout | Repository, dependency, and environment mapping | Read-only background worker |
| Requirements Planner | Requirements, assumptions, interfaces, and acceptance criteria | Planning artifacts only |
| Architecture Worker | Architecture and integration design | Design artifacts only |
| UI Worker | Frontend screens, components, styling, and interactions | Assigned isolated workspace |
| Android Data and Integration Worker | Generated Android data layer, persistence, service integrations, and business logic | Assigned isolated workspace |
| Test and QA Worker | Tests, fixtures, regression checks, and validation execution | Test paths and approved commands |
| Debugging Worker | Failure diagnosis and scoped repairs | Assigned repair paths |
| Security Worker | Security, permissions, secrets, dependencies, and compliance checks | Read-only by default |
| Visual QA Worker | Visual, device, and accessibility checks | Read-only |
| Performance Worker | Profiling, resource use, and regression analysis | Read-only |
| Documentation Worker | Documentation, decisions, and release notes | Documentation paths |
| Release Worker | Builds, packaging, and release reports | Build and artifact paths |
| Reconciliation Worker | Conflict analysis and integration validation | No direct mutation until integration |

The orchestrator manages these workers through structured task contracts and merges their results using an automated **Reconciliation Worker** that checks for file conflicts and integration errors before applying changes to the main workspace.

### 22.2 Long-Running Continuous Execution (12+ Hour Resilience)

Nirman should support continuous background execution for large-scale development tasks. Unlike simple request-response chats, an autonomous build task can run over extended periods (spanning thousands of automated tool calls, builds, and test cycles). To maintain stability during long runs, the runtime implements:

- **Progressive Context Compaction**: Automatically summarizing historical tool outputs and resolved steps while preserving exact file diffs, active errors, and acceptance criteria in the active context window.
- **Durable Checkpoint State**: Storing task progress, intermediate test results, and file revisions in local metadata storage so that tasks can survive application restarts or system reboots.
- **Live Telemetry & Adaptive Guardrails**: Real-time tracking of token expenditure, API cost, turn counts, elapsed time, and local resources. Ordinary thresholds should warn, throttle, optimize, or change model routing without terminating the goal. Only explicit hard safety, policy, environment, or user-configured stop conditions may end execution.

### 22.3 Persistent Problem-Solving and Anti-Thrashing Loops

A major failure mode of autonomous agents is getting trapped in endless "doom loops"—repeatedly attempting the exact same failing command or file patch without making progress. Nirman addresses this through an active **Anti-Thrashing and Error Recovery Harness**:

1. **Failure Fingerprinting**: The runtime records the exact signature of errors (compiler output, test stack trace, linter exit code).
2. **Repetition Detection**: If an identical tool call or failing error signature occurs three times consecutively, the execution loop is instantly suspended.
3. **Strategy Escalation**: When trapped, the system automatically triggers a recovery protocol:
   - **Context Reset**: Strips out noisy intermediate trace logs and re-injects only the core error message and initial acceptance criteria.
   - **Model Escalation**: Automatically routes the problem to a higher-reasoning model tier configured in the user's provider settings.
   - **Targeted Diagnostic Sub-Agent**: Spawns a specialized debugging worker to isolate the root cause before letting the primary builder resume.
4. **Graceful Escalation**: If automated recovery fails after configured attempts, Nirman pauses execution, presents the user with the exact failure history, and suggests specific corrective paths rather than silently failing or consuming infinite tokens.

### 22.4 Shared Task Ledger and Cross-Agent Coordination

For multi-worker tasks and parallel swarms, Nirman maintains a centralized, machine-readable **Task Ledger** stored locally as a structured state file within the workspace.

- **Atomic Task Units**: Tasks are broken down into discrete, atomic items with defined dependencies (e.g., Task 3 cannot start until Task 1 and Task 2 pass their tests).
- **Claim-and-Update Protocol**: Background workers claim unassigned tasks, mark their progress in real time, and record completion evidence (test logs, file paths).
- **Inter-Agent Handoffs**: Workers can read each other's completion summaries. For instance, the Test Engineer reads the Backend Specialist's implementation notes to write precise integration tests.

---

## 23. Advanced Autonomous Development Capabilities

**ContractId:** `CONTRACT.RUNTIME.SKILL`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.SKILL` (see §67.8)

This section expands Nirman with capabilities observed across mature autonomous software-development workflows. The capabilities are expressed as Nirman requirements and design patterns rather than as references to any other application or product.

### 23.1 Durable project context

Nirman should automatically create a concise project-context file in every managed workspace. This file should contain the project purpose, supported commands, framework conventions, architecture overview, important directories, testing instructions, environment assumptions, and known constraints. It should remain short enough to load frequently and should link to deeper documentation when more context is needed.

The application should also maintain a durable execution plan for long-running tasks. The plan should be stored as a project artifact and updated after each completed step, blocked step, or changed assumption. This prevents a long task from depending entirely on a transient chat history.

A project-context record should contain:

| Context category | Examples |
|---|---|
| Product intent | Application purpose, target users, primary workflows |
| Architecture | Entry points, modules, routes, data flow, external services |
| Commands | Install, development, lint, type check, test, build, package |
| Conventions | Naming, formatting, component patterns, error handling |
| Constraints | Supported platforms, forbidden dependencies, privacy rules |
| Known issues | Existing failures, temporary workarounds, unresolved decisions |
| Validation | Acceptance criteria, smoke tests, visual checks, release checks |

The user should be able to review and edit this context manually. Nirman should show when a task used project context and should never silently treat an outdated context file as authoritative after substantial manual changes.

### 23.2 Repository map and intelligent context retrieval

Nirman should build a compact structural map of the project rather than sending all source files to the model for every request. The map should include file paths, exports, classes, functions, types, routes, components, configuration, test relationships, and dependency edges.

The context engine should first provide the model with a small map of the relevant repository area. It should then expand into specific files, symbols, tests, and documentation only when the task requires them. Relevance should be ranked by dependency relationships, recent changes, user-selected files, active errors, route ownership, and acceptance criteria.

This should be token-aware. The context engine must have a defined budget, track what was included, and explain when content was summarized or excluded. Large logs should be compressed into error-focused summaries, while source code needed for an edit should remain available at full fidelity.

### 23.3 Explicit operating modes

Nirman should make the agent’s authority visible through explicit operating modes. The user should be able to change the mode per task or per project.

| Mode | Read access | File changes | Commands | Best use |
|---|---|---|---|---|
| Plan | Allowed | Denied | Denied or read-only | Requirements, architecture, and task planning |
| Explore | Allowed | Denied | Limited read-only commands | Codebase discovery and dependency research |
| Assisted build | Allowed | Allowed after review | Ask or allow by policy | Normal feature development |
| Autonomous build | Allowed | Allowed in workspace | Allow safe commands, ask for risky actions | Long-running implementation tasks |
| Review | Allowed | Denied | Read-only validation | Diff, security, performance, and quality review |
| Debug | Allowed | Limited to approved files | Allowed within project policy | Diagnosing and repairing a known failure |
| Release | Allowed | Approved build files | Explicit approval required | Packaging and release preparation |

The application should display the current mode in the toolbar and in every task record. Switching to a less restrictive mode should be an explicit user action.

### 23.4 Specialized worker architecture

The main Nirman agent should not perform every task itself. It should delegate focused work to specialized workers with independent context, role instructions, tool permissions, model preferences, memory policy, and budgets.

Recommended built-in workers are shown below.

| Canonical worker | Primary responsibility | Default permissions |
|---|---|---|
| Repository Scout | Map files, symbols, dependencies, entry points, and environment | Read-only |
| Requirements Planner | Convert requests into specifications, assumptions, and acceptance criteria | Read-only |
| Architecture Worker | Design structure, interfaces, data flow, and integration choices | Read-only; design artifacts |
| UI Worker | Build screens, components, styling, interactions, and responsive behavior | Workspace edits; preview |
| Android Data and Integration Worker | Build the Android data layer, persistence, validation, and integrations with external services | Workspace edits; approved commands |
| Test and QA Worker | Create and run unit, integration, regression, and edge-case checks | Test files; test commands |
| Debugging Worker | Diagnose failures and apply minimal or alternative repairs | Approved file edits; diagnostics |
| Security Worker | Inspect vulnerabilities, secret exposure, permissions, and unsafe dependencies | Read-only; security tools |
| Visual QA Worker | Inspect screenshots, layouts, accessibility, responsive behavior, and visual regressions | Read-only; screenshot tools |
| Performance Worker | Measure build/runtime performance, resource usage, bottlenecks, and regressions | Read-only; profiling tools |
| Documentation Worker | Maintain project documentation, decision records, and release notes | Documentation edits only |
| Release Worker | Create local build artifacts, packaging metadata, checksums, and release reports | Build commands; release files |
| Reconciliation Worker | Compare independent changes and prepare a validated integration plan | Read-only until integration |
| Primary Orchestrator | Decompose goals, select workers, coordinate dependencies, and synthesize evidence | Task graph and delegation only |

A worker should return a structured handoff rather than injecting all of its raw logs into the main chat. The handoff should include a concise summary, evidence, files inspected, files changed, tests run, unresolved questions, and recommended next action.

The orchestrator should choose swarm size using task complexity, dependency coupling, changed-file boundaries, target platforms, expected validation work, and available resources. It should prefer one worker for tightly coupled work, parallel read-only workers for exploration and review, and isolated write-capable workers only when their file and interface boundaries are clear.

For genuinely interdependent work, the orchestrator must create an interface agreement before parallel implementation. The agreement may include API shapes, shared types, route contracts, database schemas, event formats, or design tokens. Workers validate against this agreement before reconciliation.

Worker nesting is limited to two levels by default: the Primary Orchestrator may delegate to workers, and a worker may request a narrowly scoped diagnostic child worker. A child worker cannot create further workers, change the parent contract, expand permissions, or integrate changes. Deeper nesting requires an explicit future policy because unrestricted delegation makes ownership, evidence, and recovery ambiguous.

### 23.5 Worker chains and quality gates

Nirman should support sequential worker chains for work that benefits from independent review. A typical feature chain should be:

```text
Explore → Plan → Implement → Test → Review → Repair → Re-test → Summarize
```

The chain should not assume that every stage must run for every task. The planner may skip implementation for a planning request, and the reviewer may require a repair stage only when it identifies a material issue.

Each stage should have a quality gate. For example, implementation cannot be marked complete when the project does not compile, testing cannot be marked complete when required tests were skipped, and release preparation cannot be marked complete when the artifact path or checksum is missing.

### 23.6 Parallel work with isolation

Nirman should support parallel tasks only when each task has an isolated project copy, Git worktree, or equivalent workspace boundary. Two write-capable workers must not edit the same files in the same working directory without a coordination lock.

Parallel execution is appropriate for independent activities such as exploring different parts of a repository, reviewing separate modules, generating alternative designs, or fixing unrelated issues. It is not appropriate for two workers to rewrite the same component concurrently without a reconciliation step.

The parallel-task lifecycle should be:

```text
Create isolated workspaces
    ↓
Assign independent task contracts
    ↓
Run workers concurrently
    ↓
Collect structured results
    ↓
Compare changed files and dependencies
    ↓
Detect conflicts and overlapping assumptions
    ↓
Run reconciliation worker
    ↓
Apply approved integration
    ↓
Run full validation
```

The user should be able to view each worker session, inspect its logs, pause it, cancel it, or open its isolated workspace. Nirman should show the additional disk, token, and time cost of parallel execution.

### 23.7 Permission policy engine

Nirman should implement a three-outcome policy engine: **allow**, **ask**, and **deny**. Policies should be evaluated against the tool, command, path, project, worker role, network destination, and current operating mode.

Example policy behavior is shown below.

| Action | Default decision |
|---|---|
| Read source file inside workspace | Allow |
| Read environment secret file | Deny or ask |
| Edit source file inside workspace | Ask in assisted mode; allow in autonomous mode if approved |
| Run formatter or test command | Allow |
| Install a project dependency inside the workspace | Allow in Autonomous and Unattended profiles; ask otherwise |
| Access an external directory | Deny in Unattended profile; ask in Assisted mode |
| Run a destructive command | Deny by default |
| Use a cloud provider with project context | Allow only after a project-level privacy policy has been explicitly configured; otherwise ask when sensitive files are included |
| Commit changes inside the project repository | Allow in Autonomous and Unattended profiles when the task policy permits it |
| Push changes or publish artifacts | Always ask |
| Push changes or publish artifacts | Always ask |

Policies should support wildcard patterns, project-specific overrides, worker-specific restrictions, session-wide approvals, one-time approvals, and explicit deny rules that cannot be bypassed by automatic mode.

Nirman must provide a named `Unattended / Full Autonomy` policy profile for Goal Mode background tasks. Within the project workspace, this profile allows routine reversible actions such as dependency installation, local commits, formatting, testing, builds, preview restarts, and approved environment repair without pausing for a human. It denies external-directory access, raw credential use, destructive commands, operating-system changes, remote pushes, publishing, release signing, and unapproved sensitive-data transmission. A user configures the project privacy and network policy once; the runtime then applies it without asking again for every ordinary action. The profile must be visible, auditable, project-scoped, and easy to disable.

A repeated-action guard should detect when the same tool call, command, or failed repair is repeated without progress. Nirman should pause with a “possible loop” explanation instead of allowing an agent to continue indefinitely.

### 23.8 Background tasks and session control

Long-running work should continue in the background while the user reviews files, edits the project, or opens another task. The activity panel should show task status, elapsed time, current worker, current step, last output, token usage, estimated cost, and required approvals.

Every task should support pause, resume, cancel, retry from checkpoint, fork into an alternative approach, and open in a focused session. Resuming a task should restore its plan, context summary, worker state, checkpoint reference, and unresolved questions.

When the context becomes too large, Nirman should compact it into a structured summary containing the original goal, completed work, current files, test state, known errors, decisions, and next steps. The user should be able to inspect the summary before the task continues.

### 23.9 Structured output and event streaming

The agent runtime should emit typed events rather than returning only a final text response. Events should include task started, plan created, context loaded, file read, file changed, command proposed, approval requested, command started, command output, preview started, test completed, screenshot captured, worker delegated, worker completed, checkpoint created, warning raised, and task completed.

The interface should render these events in real time and persist them in the activity log. The final task result should be available as both human-readable Markdown and machine-readable JSON.

A normalized task result should contain:

```text
TaskResult
- taskId
- status
- summary
- changedFiles
- createdFiles
- deletedFiles
- commandsRun
- testsRun
- screenshots
- checkpoints
- warnings
- unresolvedIssues
- workerHandoffs
- providerAndModel
- tokenUsage
- estimatedCost
- duration
- confidence
```

Structured results will allow future automation, regression testing, analytics, and reliable UI rendering without parsing free-form model text.

### 23.10 Model routing and fallback

Nirman should route different task types to different model profiles when the user has configured multiple providers or models. Planning and architecture may use a high-reasoning model, repository exploration may use a faster model, visual inspection may use a vision-capable model, and simple documentation updates may use a lower-cost model.

The routing policy should consider task type, required capabilities, context size, latency, cost, current provider health, and user preference. The user should be able to override the route for an individual task.

If a provider is unavailable, rate-limited, or returns an unsupported capability error, Nirman should optionally fall back to an approved alternative. Fallback behavior must be visible in the task record and should never silently send sensitive project context to an unapproved provider.

### 23.11 Skills, extensions, and external tools

Nirman should support reusable skills as version-controlled Markdown instruction packages. A skill should declare its name, description, compatibility, required tools, and intended use cases. Skills should be discoverable by description and loaded on demand rather than injected into every prompt.

Examples include database migrations, accessibility review, design-system implementation, Android build diagnostics, secure API integration, release preparation, documentation maintenance, and visual regression testing.

Every skill must use a structured package contract:

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

A skill is invoked only when the orchestrator selects it from a task requirement, explicit user request, or matching trigger condition. Loading a skill adds instructions and schemas; it never grants permissions automatically. Every skill tool call still passes through the normal policy engine. User or shared skills must be scanned for prompt injection, unsafe commands, secret access, and dependency behavior before activation, and updates must be versioned with rollback. Built-in runtime capabilities take precedence over skills when both provide the same function, while a skill may add workflow instructions around the built-in capability.

The application should also support external tools through an MCP-compatible adapter or equivalent extension interface. External tools may provide design files, issue trackers, documentation search, browser automation, observability data, or test environments. Each external tool must have its own permission scope, provider status, network policy, compatibility record, lifecycle state, and audit trail. Nirman distinguishes a `SkillPackage` (a scanned, versioned, permission-neutral instruction and workflow package) from an `ExternalToolConnection` (a mediated connection to an external service or protocol server). A code-bearing runtime extension is a separate future capability and is not created by the word “plugin”. Discovery, schema compatibility, trust/scan, enablement, session pinning, permission evaluation, invocation, health, disablement, revocation, update, and rollback are explicit lifecycle concerns; loading or connecting never grants permissions automatically.

### 23.12 Hooks and policy interception

Nirman should expose pre-action and post-action hooks. A pre-action hook may validate a command, redact a secret, enforce a path policy, require an approval, or transform tool arguments. A post-action hook may summarize output, detect errors, update the repository map, create a checkpoint, or trigger a reviewer.

Hooks should be deterministic where possible and should run outside the model’s control. A model must not be able to disable a mandatory security hook through ordinary project instructions.

### 23.13 AST, LSP, and semantic editing

Text patches are useful but insufficient for large refactors. Where the language server or parser is available, Nirman should use semantic operations such as rename symbol, find references, extract function, update imports, change interface implementation, and apply workspace-wide type-safe transformations.

The agent should prefer semantic edits for high-impact refactors and use text patches for localized changes. After a semantic edit, Nirman should run the relevant type checks and tests, then show the affected symbol and file graph.

### 23.14 Android device and visual verification

For Android applications, Nirman MUST use a Nirman-managed local Android emulator for runtime and visual verification. The emulator runs headlessly on the Windows host and its actual rendering surface is projected into the Nirman Preview panel.

The device worker may install builds, launch activities, execute synthetic interactions, capture screenshots, inspect Logcat and runtime errors, verify permissions and orientation, test phone/tablet profiles, and collect crash traces.

Physical Android hardware is outside the Nirman product scope and MUST NOT be a validation, preview, recovery, completion, or fallback dependency.

The device runner must use synthetic test data by default. It must not access personal accounts, submit real transactions, use personal credentials, or access private services without explicit user control and approval.

Visual verification should compare screenshots against the requested design requirements, known baseline screenshots, and accessibility expectations. A visual finding should include the screen or navigation state, device profile, screenshot, observed issue, confidence, and recommended change.

### 23.15 Testing and automatic repair

Testing should be treated as part of implementation rather than as a final optional step. Nirman should infer relevant checks from the project and task, including formatting, linting, type checking, unit tests, integration tests, build validation, smoke tests, and visual checks.

When a check fails, the debugger worker should receive the focused failure output, the relevant changed files, the task acceptance criteria, and the latest checkpoint. It should attempt the smallest reasonable repair, rerun the failed check, and stop after the configured retry limit.

The final result should distinguish between passed checks, skipped checks, failed checks, environment failures, and checks that could not be run. “No test command was available” must not be presented as “tests passed.”

### 23.16 Cost, token, and resource telemetry

The application should show token usage, request count, model selection, estimated cost, duration, process time, and disk usage for each task when the provider exposes the relevant data. Users should be able to set maximum task budgets.

Before starting a large task, Nirman should provide an approximate resource forecast based on the number of workers, expected context size, selected models, and validation stages. During execution, it should continuously report usage and adapt by reducing concurrency, compacting context, routing to an approved lower-cost model, batching work, or pausing new optional work. Ordinary token, cost, time, or process thresholds must not terminate an end-to-end goal unless the user explicitly configured them as hard limits. Hard safety limits, destructive-process watchdogs, provider policy limits, and operating-system protection limits may still stop a task when necessary.

### 23.17 Review-only and release workflows

Nirman should include a review-only workflow that analyzes a diff, branch, checkpoint, or pull request without modifying the project. The review should prioritize correctness, security, performance, maintainability, test coverage, accessibility, and release risk.

A release workflow should run a clean validation pass, confirm that required metadata exists, verify that secrets are absent from the artifact, record dependency versions, generate checksums where appropriate, and provide a release report. Publishing, signing, uploading, or submitting the artifact should require explicit confirmation.

### 23.18 Recommended advanced architecture

The expanded runtime should use the following internal components:

```text
Task Controller
    ├── Requirements and Plan Manager
    ├── Context and Repository-Map Engine
    ├── Worker Registry and Delegation Manager
    ├── Policy and Approval Engine
    ├── Tool Gateway
    ├── Process and Workspace Manager
    ├── Checkpoint and Session Manager
    ├── Event Stream and Activity Log
    ├── Validation and Visual QA Manager
    ├── Provider Router and Fallback Manager
    ├── Skill and Extension Registry
    └── Reconciliation and Release Manager
```

The Tool Gateway should be the only component allowed to invoke filesystem, terminal, browser, external-tool, or build actions. The Task Controller should decide what work is needed, while the Policy Engine decides whether a proposed action is permitted. This separation prevents a model response from becoming an uncontrolled system action.

---

## 24. Advanced Roadmap Priorities

The capabilities above should be introduced in the following order.

| Priority | Capability group | Reason |
|---|---|---|
| P0 | Project context, repository map, structured events, checkpoints, permissions | Foundational reliability and safety |
| P1 | Plan/build/review modes, background tasks, session resume, testing and repair | Makes the core app genuinely autonomous |
| P1 | Cost budgets, loop detection, provider routing, fallback, provenance | Controls operational risk and user trust |
| P2 | Specialized workers, worker chains, isolated workspaces, reconciliation | Enables higher-quality parallel development |
| P2 | Skills, project extensions, hooks, external tools | Adds domain-specific capability without hard-coding everything |
| P2 | Screenshots, visual QA, AST/LSP edits | Improves quality beyond text generation |
| P3 | Browser automation (optional, external) | Auxiliary only; never a substitute for Nirman-managed local Android emulator validation |
| P3 | Headless automation, scheduled local tasks, remote worker connections | Expands automation after the local core is stable |
| P3 | Advanced native project profiles and multi-emulator testing | Expands beyond the initial supported stacks |

Nirman should not begin with unrestricted multi-agent parallelism. It should first prove that one worker can reliably inspect, plan, edit, test, and recover within a controlled workspace. Parallel workers should be added only after checkpoints, permissions, event logs, and reconciliation are dependable.

---

## 25. Updated Definition of a High-Quality Autonomous Task

A high-quality Nirman task is not merely a code-generation response. It is a reproducible development record containing the original request, project context, plan, selected worker roles, permissions, model routing, files changed, commands run, tests and screenshots, checkpoints, warnings, resource usage, and unresolved issues.

The task should be considered complete only when the requested acceptance criteria are satisfied or the application has clearly explained why they could not be satisfied. Every declared boundary operation must resolve its applicable integration contract and evidence dependencies before it can contribute to completion. The final result must not hide uncertainty behind confident wording.

> Nirman should optimize for **verified progress**, not maximum autonomous activity.

## 26. Implementation-Level Requirements for the Initial Architecture

The master specification defines product behavior, while this section makes the most important implementation mechanics explicit. The section is intentionally code-free: it describes the components, interfaces, state transitions, limits, and acceptance behavior that the engineering documents must implement.

### 26.1 Local control plane and persistent task daemon

Nirman should separate the desktop user interface from a local control plane. The interface may close, restart, or become unavailable without destroying a running task. A local task daemon should own task execution, worker processes, approvals, checkpoints, logs, and recovery.

The control plane should start when Nirman launches and should be able to continue as a user-scoped background process when the window is minimized or closed. It should not run as a system service by default. The user must be able to stop it from the application and from a visible operating-system process control action.

The daemon should persist task state in the authoritative local SQLite execution ledger or an explicitly accepted equivalent transactional store. Large logs and binary artifacts should be stored in task-specific directories, while the database stores metadata and references. In this specification, “project repository” means the user-owned Git/workspace and source revision, while “execution ledger” means Nirman’s authoritative SQLite store; these are separate persistence domains and MUST NOT be represented by one generic repository abstraction.

| Persistent object | Required information |
|---|---|
| Task | Goal, status, plan, current step, owner, budgets, timestamps |
| Worker | Role, process ID, workspace, model, permissions, heartbeat, status |
| Event | Sequence number, type, payload, timestamp, task and worker IDs |
| Approval | Requested action, policy reason, decision, user, expiry |
| Checkpoint | Revision, changed files, preview state, test state |
| Artifact | Path, type, size, checksum, build profile, creation time |
| Recovery record | Failure fingerprint, attempted strategies, next action |

After a restart, the daemon should rehydrate tasks from the database, verify that worker processes and workspaces still exist, mark lost processes as recoverable failures, and offer resume-from-checkpoint rather than pretending that execution continued uninterrupted.

### 26.2 Worker communication protocol

Workers should communicate through a local event bus and durable task ledger rather than editing shared Markdown files as their only coordination mechanism. Markdown summaries may remain useful for humans, but machine coordination requires typed messages and transactional task updates.

Every worker message should contain the following fields:

```text
WorkerMessage
- messageId
- taskId
- senderWorkerId
- recipientWorkerId or broadcastTopic
- messageType
- correlationId
- sequenceNumber
- payload
- evidenceReferences
- requiresAcknowledgement
- createdAt
- expiresAt
```

Supported message types should include `task_claimed`, `progress_update`, `question`, `dependency_ready`, `implementation_summary`, `test_result`, `review_finding`, `merge_request`, `approval_required`, `worker_failed`, and `task_completed`.

Workers should use heartbeats while active. A worker that misses a configured number of heartbeats should be marked stale, its process should be inspected, and its task should be requeued or escalated. Messages should be idempotent so that replay after a daemon restart does not create duplicate changes.

### 26.3 Worker concurrency and resource limits

Nirman should not permit unlimited background workers. The scheduler should enforce global and per-task limits based on CPU cores, memory, disk availability, provider concurrency, and user configuration.

| Resource | Initial default policy |
|---|---|
| Concurrent write-capable workers per task | 3 |
| Concurrent read-only workers per task | 5 |
| Total active workers | Minimum of 8 or available-resource policy |
| Worker heartbeat interval | 10 seconds |
| Worker stale threshold | 60 seconds, configurable |
| Default task wall-clock policy | Default 200-minute duration budget, user-configurable per task or project. Exhaustion resolved per CLAUSE.COST.EXHAUSTION_EXPLICIT — reduce context, reduce concurrency, change model, pause for approval, continue under renewed policy, safely fail, or degrade classification. NOT a termination trigger and NOT a completion claim. The "no fixed completion lock" principle is preserved: the budget triggers an explicit outcome, it does not lock completion. |
| Default repair attempts per failure | 3 strategy changes, not three identical retries |
| Default task context budget | Provider-dependent with a visible cap |
| Default disk quota per task | 10 GB unless project policy overrides |

The scheduler should reserve resources before launching a worker, release them after completion, and reduce parallelism when the system becomes constrained. A user should be able to pause new workers while allowing active workers to finish.

### 26.4 Deterministic reconciliation of parallel changes

Parallel worker results must never be copied directly into the main workspace without reconciliation. Each write-capable worker should operate in a dedicated worktree or copy-on-write workspace based on a known parent checkpoint.

The reconciliation process should follow these stages:

```text
Freeze parent checkpoint
    ↓
Collect worker commits and structured summaries
    ↓
Compare changed files and dependency manifests
    ↓
Detect overlapping edits and incompatible assumptions
    ↓
Apply non-overlapping changes automatically
    ↓
Send overlapping changes to reconciliation worker
    ↓
Run formatter, type check, tests, and build
    ↓
Create an integration checkpoint
    ↓
Present conflicts and evidence to the user
```

The reconciliation worker must not silently choose a winner. Tie-breaking should prefer the change that satisfies the current acceptance criteria, preserves existing public behavior, passes more validation, and changes fewer unrelated files. When evidence is insufficient, the system should preserve both alternatives in isolated branches and ask the user.

Partial integration should be transactional. If the integrated workspace fails its required quality gates, Nirman should roll back to the parent checkpoint or keep the result isolated for review. The main workspace must never be left in an unknown half-merged state.

### 26.5 Sandbox profiles and operating-system isolation

Path-based permissions are necessary but are not sufficient as a sandbox. Nirman should implement multiple execution profiles.

| Profile | Isolation approach | Intended use |
|---|---|---|
| Trusted local | User process with workspace and command policy | Fast development on trusted repositories |
| Restricted process | Windows job object, restricted token, controlled environment, workspace-only paths | Default autonomous execution |
| High-risk restricted process | Restricted token, isolated workspace ACLs, filtered environment, strict Job Object limits, and disposable emulator snapshot | High-risk builds, untrusted dependencies, or hostile repositories |
| Review-only | No write access and no arbitrary process execution | Diff, security, and architecture analysis |

The exact isolation technology may vary by Windows edition and deployment environment, but the abstraction must be stable. Every worker receives a declared profile, workspace mount, environment variables, network policy, process quota, and cleanup policy.

### 26.6 Resource quota enforcement

The runtime should monitor CPU, memory, disk, process-count, output-size, elapsed time, and network usage. Ordinary usage thresholds should trigger telemetry, throttling, concurrency reduction, context compaction, or an approval request when the user has configured one. Windows Job Objects should be used where appropriate for process-tree accounting and termination. Hard operating-system safety limits, unresponsive-process watchdogs, Windows Job Object limits, restricted-token boundaries, and sandbox protection limits may terminate a process when necessary to protect the computer or workspace.

A quota event should pause the worker, capture diagnostics, and explain whether the task can resume with a larger limit. It should not kill the process without preserving the latest checkpoint and event log.

### 26.7 Dependency and artifact safety scanning

Before running newly downloaded dependencies or build scripts, Nirman should record the package name, version, source, lockfile change, and requested network access. The runtime should run available malware, secret, license, and vulnerability checks before promoting a dependency into an autonomous build profile.

A package that cannot be scanned should be labeled unverified. The user may approve it for a restricted or disposable environment, but it should not silently run with full local privileges.

Generated artifacts should be scanned for embedded secrets, unexpected executables, suspicious network destinations, and files outside the expected output directory. Release reports should record scan results and unresolved warnings.

### 26.8 Browser isolation and visual testing

Browser automation is optional and external to the Android validation path. It may assist auxiliary work such as reading documentation or inspecting a web service the generated application consumes. It is never a validation surface for the generated Android application: Nirman-managed local Android emulator execution per §59 is the only core validation path, and a browser observation must never be cited as evidence that Android behavior is correct.

When enabled, browser automation should use a dedicated Nirman-managed browser profile, separate from the user’s personal browser profile, cookies, extensions, saved passwords, and downloads. Test sessions should use synthetic data and disposable storage by default.

The browser worker should expose only approved routes and local development origins. External navigation should be controlled by the network policy. Screenshots, console logs, network failures, accessibility findings, and interaction traces should be attached to the task record.

### 26.9 Preview state, checkpoints, and rollback

The preview manager should associate every running preview with a project revision and checkpoint ID. When files change, it should report whether the preview hot-reloaded, partially reloaded, or required a full restart.

When the user reverts a checkpoint, Nirman should stop or invalidate the preview if its running revision no longer matches the restored project. It may hot-reload only when the preview runtime confirms that the restored state is safe and complete. The UI should never show a preview as current when it represents a different checkpoint.

### 26.10 Responsive and multi-emulator preview

The Android preview should support named emulator profiles for phone, tablet, portrait, landscape, Android version, architecture, screen density, and API level. A visual test should launch the same flow across selected Nirman-managed emulator profiles, compare screenshots, and record profile-specific findings.

Android preview should use a emulator-manager abstraction that reports Nirman-managed local Android emulator identity, connection state, platform version, architecture, available storage, hot-reload state, logs, and build/install status. The first implementation may support one Nirman-managed local Android emulator at a time, but the protocol should allow multiple emulator sessions later.

### 26.11 Toolchain version management

Nirman should not rely on one globally installed toolchain. Each Android project should declare required versions or compatible ranges for Node.js, package manager, Java, Gradle, Android SDK, platform-tools, emulator images, Expo or React Native tooling, and selected native build dependencies.

The runtime should resolve a project toolchain through a version manager, portable installation, or explicitly configured local path. Each project receives isolated environment variables, cache paths, process scopes, and toolchain bindings so incompatible projects cannot silently change one another’s environment. Two projects with incompatible versions must be able to run without silently changing one another’s environment.

The environment record should contain executable paths, detected versions, source of installation, compatibility result, and reproducibility status. A build must fail with a diagnostic when the requested toolchain cannot be resolved.

### 26.12 Android runtime abstraction

Although Nirman runs as a Windows desktop application, its generated target is Android. Runtime operations should use an Android-focused interface defining process launch, termination, filesystem policy, environment discovery, port management, emulator and emulator control, Logcat capture, Gradle and Metro execution, quotas, and APK artifact handling. The desktop host may use Windows-specific process and sandbox implementations, but the generated-project contract remains Android-specific.

### 26.13 Background approvals and notifications

When a minimized or background task requires approval, the control plane should create a durable approval request with an expiry policy. The desktop application should display it on return and may use an operating-system notification when notifications are enabled.

The user should be able to approve once, approve matching actions for the session, deny once, deny the task, or pause the task until later. An approval must be bound to the exact action, workspace, worker, and policy context that generated it. Expired approvals must not be reused.

### 26.14 Continuous execution state machine

Long-running tasks should use an explicit state machine rather than an informal loop.

```text
QUEUED
  ↓
PLANNING
  ↓
READY
  ↓
RUNNING
  ├── WAITING_APPROVAL
  ├── WAITING_RESOURCE
  ├── RECOVERING
  ├── PAUSED
  ├── FAILED_RETRYABLE
  └── CANCEL_REQUESTED
  ↓
VALIDATING
  ↓
RECONCILING
  ↓
COMPLETED or ESCALATED
```

Every state transition should be persisted with a reason and event reference. A task may continue automatically only from states marked recoverable. A task that reaches `ESCALATED` must require a new user action or explicit retry strategy.

### 26.15 Updated implementation acceptance criteria

The initial architecture is not complete until it can demonstrate the following behavior:

1. A background task survives closing the Nirman window and continues through the control plane.
2. An active Unattended / Full Autonomy task starts the control plane at user login after reboot and resumes from its last validated checkpoint without requiring the UI to be opened.
3. Suspend, resume, and hibernate transitions leave a durable event and eligible processes, ports, emulators, and provider requests are revalidated before continuation.
4. Two independent workers can run in isolated workspaces without changing the main project.
5. A worker timeout or crash produces a durable failure record and a recovery decision.
6. The reconciliation process detects overlapping edits and never silently overwrites one worker.
7. The runtime enforces a configured process, memory, disk, time, and output limit.
8. A restricted worker cannot read a protected environment file or write outside its workspace.
9. Browser tests use a dedicated profile with no personal cookies or credentials.
10. Reverting a checkpoint cannot leave the live preview pointing to an unknown revision.
11. Two projects with different toolchain versions can run without global version contamination.
12. A background approval request remains visible after the application is minimized or restarted.
13. The event log can reconstruct the task’s plan, worker actions, approvals, failures, checkpoints, and final result.
14. A persistent worker terminal preserves working directory and environment state, handles declared safe interactive prompts, records the Windows shell profile, and reconnects with searchable rolling logs.
15. A skill can be scanned, invoked through a declared contract, and denied when it requests undeclared tools or permissions.
16. The repository map updates incrementally, affected tests are computed from dependency evidence, checkpoint retention preserves a restore path, and architectural-drift checks run during long tasks.
17. The live application preview and nested execution surface remain visible together and refer to the same project revision.

## 27. Product Requirements for Goal-Based and Persistent Autonomy

This section defines advanced autonomy requirements for Nirman. These are product and system requirements, not implementation code. The requirements make long-horizon work, background execution, lifecycle automation, checkpoint granularity, recovery, context scaling, and external tool interoperability explicit.

### 27.1 Goal Mode

Nirman must provide a **Goal Mode** for tasks where the user defines a completion condition once and expects the application to continue working without repeated prompts.

A Goal Mode task must contain:

| Goal field | Meaning |
|---|---|
| Goal statement | Natural-language description of the desired result |
| Completion conditions | Testable conditions that determine whether the goal is complete |
| Scope | Project, folders, files, routes, modules, or platform targets included |
| Resource policy | Adaptive time, turns, tokens, estimated cost, disk, and process monitoring; optional user-configured hard caps |
| Autonomy profile | Named allow/ask/deny policy such as Unattended / Full Autonomy |
| Resource budget | Optional user-configured hard limits rather than a default completion lock |
| Allowed autonomy | Permitted operating mode, tools, network, workers, and schedules |
| Stop conditions | Conditions that require pause or escalation |
| Progress state | Completed work, active work, blocked work, and next strategy |
| Validation plan | Tests, builds, screenshots, device checks, and review gates |

The completion condition must be stored as a durable task contract and evaluated after every validation cycle. Nirman must not report success merely because the model stopped generating text. It must show which completion conditions passed, failed, were skipped, or remain unverified.

Goal Mode should support a user instruction such as “continue until the application builds, the required tests pass, the preview has no runtime errors, and all acceptance criteria are satisfied.” The mode should continue working across multiple agent turns and worker handoffs. Ordinary resource signals should trigger adaptation rather than termination; only explicit hard caps, safety stop conditions, provider or environment unavailability, cancellation, or unrecoverable failure may end execution.

For unattended background work, the user should be able to select the named `Unattended / Full Autonomy` profile. It allows routine reversible operations inside the approved workspace without repeated prompts while keeping deployment, signing, credential access, remote pushes, destructive commands, protected paths, and unapproved sensitive-data transmission hard-gated.

### 27.2 Non-blocking background tasks

Nirman must support background tasks that do not block the user from working in the same application or using the rest of the computer. A background task must have its own task panel, workspace state, worker processes, event stream, resource budget, and notification behavior.

The user should be able to start a background task, continue editing another project or task, inspect progress without taking focus, pause or cancel it, approve a pending action, and open the task’s isolated workspace. A background task must never steal keyboard or mouse focus from the user’s active application.

When a task finishes, pauses, fails, or needs approval, the application should use an in-app notification and an optional operating-system notification. The notification must identify the project, task, state, required action, and a direct path to the relevant task view.

The task should also appear in a tray badge, durable task queue, and startup summary after reboot. If operating-system notifications are suppressed, a pending decision must remain visible when the application reconnects and must not silently park the task.

### 27.3 Isolated sub-agent workspaces

Every write-capable sub-agent must receive an isolated Git worktree, branch, copy-on-write workspace, or disposable project workspace. The isolation method, parent revision, allowed files, and cleanup policy must be visible in the task record.

Sub-agents may work concurrently when their task contracts are independent. Nirman must prevent two write-capable sub-agents from modifying the same mutable workspace at the same time. The main project must remain unchanged until the reconciliation contract has completed.

Each sub-agent must return a structured handoff containing its objective, files inspected, files changed, commands run, tests completed, assumptions, unresolved issues, and recommended next action. The parent orchestrator must decide whether the handoff satisfies the dependency and acceptance criteria.

### 27.4 Lifecycle hooks

Nirman must define a named lifecycle-hook system so that policy, validation, automation, and integrations can attach to predictable events. Hooks may invoke a deterministic local action, request approval, call an approved external tool, or start a specialized worker, subject to policy.

| Hook category | Required events |
|---|---|
| Session | `session_started`, `session_resumed`, `session_paused`, `session_ended` |
| Task | `task_created`, `task_planned`, `task_started`, `task_completed`, `task_failed`, `task_escalated` |
| Agent loop | `before_reasoning`, `after_reasoning`, `before_tool`, `after_tool`, `before_validation`, `after_validation`, `agent_stopped` |
| Permission | `approval_requested`, `approval_granted`, `approval_denied`, `approval_expired` |
| Worker | `worker_created`, `worker_started`, `worker_waiting`, `worker_failed`, `worker_completed`, `worker_requeued` |
| Workspace | `workspace_created`, `checkpoint_created`, `checkpoint_restored`, `merge_started`, `merge_completed`, `merge_conflict` |
| Context | `context_loaded`, `context_compacted`, `context_excluded`, `context_budget_reached` |
| Runtime | `process_started`, `process_failed`, `process_terminated`, `quota_reached`, `preview_started`, `preview_stale` |
| Configuration | `provider_changed`, `policy_changed`, `skill_loaded`, `external_tool_connected` |

Every hook invocation must have a timeout, permission scope, correlation ID, and failure policy. A mandatory security hook must not be disabled by model output or ordinary project instructions. Hook failures should pause the affected action when the hook is marked blocking; otherwise, they should be recorded as warnings.

### 27.5 Scheduled automations

Nirman must support recurring local automations independently of chat sessions. A scheduled automation must define a trigger, project, goal, operating mode, resource budget, workspace policy, approval behavior, notification policy, and retention policy.

Supported trigger types should include a fixed interval, a local calendar schedule, project-file change, failed validation, new checkpoint, and user-defined manual trigger. Scheduled tasks should initially be limited to safe local activities such as running tests, checking dependencies, refreshing documentation, generating reports, and preparing review summaries.

Publishing, pushing, signing, submitting, purchasing, or using personal credentials must never be scheduled without an explicit per-run approval policy. The scheduler must show the next run, previous run, current status, failure history, and pause/disable controls.

### 27.6 Two-tier checkpoint model

Nirman must implement two distinct checkpoint tiers.

| Checkpoint tier | Scope | Use case |
|---|---|---|
| File-level checkpoint | Individual file or small file group | Fast undo of an isolated edit or patch |
| Task-level checkpoint | Full project revision, metadata, preview state, and validation state | Revert an autonomous feature or recover a failed task |

File-level checkpoints should be created before targeted patches or manual edits that the user asks the agent to perform. Task-level checkpoints should be created before multi-file autonomous work, worker integration, packaging, or release preparation.

Restoring a file-level checkpoint must not silently restore unrelated files. Restoring a task-level checkpoint must restore the complete project revision and invalidate or restart any preview that does not match the restored state.

### 27.7 Backtracking-based recovery

Repair loops must include a backtracking strategy, not only a retry strategy. When an approach fails, Nirman should be able to return to the last known-good file-level or task-level checkpoint, record the failed strategy, and attempt a materially different approach.

A recovery cycle should follow this pattern:

```text
Detect failure
    ↓
Capture failure fingerprint and evidence
    ↓
Classify whether the approach is locally repairable
    ↓
Attempt a minimal repair when appropriate
    ↓
If repeated or structurally blocked, restore last known-good checkpoint
    ↓
Change strategy, worker role, context, or model profile
    ↓
Run validation again
    ↓
Continue, pause, or escalate with evidence
```

Nirman must distinguish between a new strategy and a repeated variation of the same failed action. The recovery record should explain what changed between attempts. A task should continue through additional strategies and adaptive resource management. It should stop only when it reaches an explicit hard safety or policy limit, an unresolvable requirement, a required user decision, an unavailable environment/provider, user cancellation, or no safe recovery path remains. Ordinary usage thresholds are guardrails, not automatic completion locks.

### 27.8 Context-scaling modes

Nirman must support two context strategies because configured providers may have very different context capacities.

| Context mode | Behavior | Best use |
|---|---|---|
| Indexed retrieval mode | Provide repository map and retrieve relevant files, symbols, tests, and documentation | Small or medium context providers and very large repositories |
| Large-context mode | Provide a near-full repository representation after filtering secrets, binaries, and irrelevant generated files | Providers with large context capacity and repository-scale refactors |

The context planner should select a mode based on provider capability, project size, task type, token budget, privacy policy, and user preference. The task record must show which mode was used, what content was included, what was summarized, and what was excluded.

Large-context mode must not mean sending secrets or unbounded generated files. The exclusion and redaction policy remains active regardless of provider capacity.

### 27.9 Optional MCP-compatible extension layer

Nirman must provide an optional extension layer compatible with the Model Context Protocol or an equivalent standardized external-tool protocol. The internal Tool Gateway remains authoritative for local files, processes, policies, previews, and credentials, while external servers may provide approved capabilities such as issue tracking, design inspection, documentation search, databases, observability, or browser services.

Every external tool connection must declare its server identity, available tools, network destination, data scope, worker access, approval policy, and disable action. External tools must be scoped per project or worker where possible. Their inputs and outputs must be recorded in the activity log.

MCP-compatible tools must not bypass Nirman’s permission engine. A tool request that reads a protected file, accesses an external directory, sends project content outside the configured provider policy, or performs an external side effect must still be evaluated by Nirman.

### 27.10 Completion and continuous-work contract

Nirman should continue working until the goal is complete **or until a defined stop condition is reached**. Defined stop conditions include completed acceptance criteria, an explicit hard safety or policy limit, unrecoverable repeated strategy failure, missing environment capability, required human decision, safety policy denial, provider failure, user cancellation, or no safe recovery path. Reaching an ordinary time, token, cost, or usage threshold should trigger adaptation or a visible warning rather than automatically ending the goal.

The application must never claim that it “worked until complete” if it stopped because of a limit or error. It should present a completion classification:

| Classification | Meaning |
|---|---|
| Completed | All required conditions passed |
| Completed with warnings | Required conditions passed but non-blocking issues remain |
| Blocked | A dependency, environment, permission, or decision is missing |
| Escalated | Automated strategies were exhausted and user input is required |
| Cancelled | The user or policy stopped the task |
| Failed | The task ended without satisfying the goal and without a recoverable next step |

### 27.11 Execution surface, evidence, and continuous validation

The chat interface is the **task launcher**, not the execution engine. After a user starts a task, Nirman must continue independently in the background under the task’s stored goal contract, permissions, budgets, and stop conditions. The user must be able to close or minimize the interface and later reconnect to the same persisted task state.

Every autonomous task must expose a visible execution plan with phases, dependencies, progress, checkpoints, active workers, blocked work, and completion state. The plan must be durable and must update as new evidence changes the implementation strategy. The application must not replace a plan with a generic spinner or imply that a task is complete only because a model response ended.

The task view must support an expandable execution tree. The tree should show the parent goal, phases, sub-tasks, worker handoffs, commands, previews, tests, builds, security checks, visual checks, repair attempts, approvals, and checkpoint operations. Each node must have a state, timestamps, owner, workspace, evidence references, and failure or warning information where applicable.

The task view must expose runtime telemetry sufficient to understand what is happening without opening raw logs:

| Telemetry | Required behavior |
|---|---|
| Current goal | Show the active goal and exact completion conditions |
| Elapsed time | Show task and current-step duration |
| Turns and requests | Show model turns, provider requests, cancellations, and retries |
| Token and resource usage | Show token estimates or usage, cost where available, CPU, memory, disk, and process consumption |
| Active workers | Show worker roles, current actions, heartbeats, workspaces, and states |
| Last checkpoint | Link to the most recent validated file-level or task-level checkpoint |
| Current blocker | Show the dependency, failure, approval, or environment issue blocking progress |
| Next action | Show the next planned action or recovery strategy |
| Completion state | Show passed, failed, skipped, unverified, and remaining conditions |

Status claims must be evidence-backed. A phase may be marked `completed` only when its declared evidence requirements pass. Evidence may include a successful command result, test report, build artifact, screenshot, device result, security scan, review record, or user-approved exception. Model-generated statements must be displayed as explanations, not treated as proof.

For an Android-target profile, the default autonomous validation loop should be:

```text
Launch the Nirman-managed local Android emulator
    ↓
Run focused Android tests and checks
    ↓
Run Android build or package validation
    ↓
Run security, dependency, and reliability checks
    ↓
Run Android device, accessibility, and visual QA
    ↓
Use browser validation only for a declared optional external/auxiliary surface
    ↓
Classify failures and warnings
    ↓
Repair or backtrack to a known-good checkpoint
    ↓
Revalidate the affected and regression checks
    ↓
Evaluate completion conditions through PreviewPromotionGate and evidence rules
```

#### Event-driven continuation and evidence feedback

The runtime must continue from durable events rather than waiting for another chat click. The following triggers are continuations of the existing lifecycle-hook, trigger, validation, and recovery contracts; they do not create a second scheduler or authority:

| Trigger | Automatic continuation | Required gate |
|---|---|---|
| `workspace_file_saved` | Run the affected formatter, lint, typecheck, and focused tests when enabled by the project policy | Changed-file scope and current revision |
| `build_completed` | Inspect the artifact, run the affected and regression checks, then collect runtime prerequisites | Build observation and artifact identity |
| `failure_observed` | Capture diagnostic output and stack-trace references, create a stable failure fingerprint, package the failure context, and schedule diagnosis or repair | RecoveryAuthority and checkpoint lineage |
| `dependency_changed` | Run compatibility, vulnerability, license, provenance, size, and duplicate-class checks before commit or build continuation | DependencyHealthService and policy decision |
| `promotion_or_export_requested` | Run health checks, artifact inspection, required validation, signing/certificate checks, and post-copy verification; retain last-known-good on failure | PreviewPromotionGate, artifact authority, signing authority, and export verification |
| `stream_reconnected` | Replay missing durable events and rebuild projections before resuming display or execution decisions | Event continuity and projection cursor |

A failure continuation must pass the real diagnostic context—failure fingerprint, relevant stack trace or process output, changed files, environment identity, prior attempts, checkpoint, and validation results—to the next authorized diagnostic or coding worker. A retry without new evidence, a changed strategy, or a changed authority context is not a new attempt. Retry budgets are bounded and policy-configurable; reaching a budget triggers strategy change, backtracking, degradation, or a truthful blocker rather than a blind loop.

Nirman uses Windows process and workspace isolation for local execution. The runtime must not imply that a Docker container, virtual machine, WSL environment, or other prohibited external sandbox was used. Nirman also has no generic web or cloud deployment target; `promotion_or_export_requested` refers to local Android preview promotion or declared APK artifact export.

Browser automation is never a required or authoritative completion stage for an Android-target profile. Nirman-managed local Android emulator evidence is authoritative for Android runtime behavior. The exact preview-promotion predicate is the canonical `PreviewPromotionGate` defined in technical architecture §73.5.1.

Nirman should not ask for approval for every small, reversible operation inside an approved workspace. It should request a decision only at defined policy boundaries, including protected-file access, risky dependency installation, external-service access, credential use, destructive operations, publishing, release signing, or any action outside the current workspace and policy scope. The approval request must identify the exact action, reason, worker, workspace, policy, risk, and available choices.

A task must terminate only when one of the following conditions is true: all required completion conditions pass; a required user decision is reached; an explicit hard safety or policy limit is reached; the environment or provider is unavailable; the user cancels the task; an unresponsive or dangerous process must be stopped to protect the computer; or an unrecoverable failure occurs. A routine event such as a saved file, completed build, captured error, dependency change, or successful worker response must not end the task; it must advance the applicable continuation trigger and validation path. Ordinary time, token, cost, process, disk, and retry thresholds should cause adaptation, throttling, warning, or optional approval—not a fixed completion lock. If the screenshot or task view shows extended activity, that demonstrates persistent execution, not a guarantee that every goal can be completed without intervention.

The final task result must expose the requested goal, changed files, checkpoints, worker activity, commands, validation evidence, tests, builds, screenshots or device results where relevant, warnings, blockers, unresolved conditions, resource usage, and the final completion classification. The user should be able to reopen each evidence item from the result.

## 28. Complete Runtime and Self-Improvement Requirements

The Nirman runtime must be treated as the core product, not as a thin wrapper around model requests. It must own the complete development loop from goal intake through requirement extraction, planning, implementation, validation, repair, packaging, evidence-backed completion, and recovery.

### 28.1 Complete runtime control plane

The runtime must provide a stable supervisor, durable control plane, task-graph scheduler, worker lease manager, policy engine, Model Gateway, Tool Gateway, workspace/checkpoint manager, validation/evidence engine, recovery manager, memory manager, artifact/version manager, and self-improvement manager.

The desktop interface is a client of this runtime. Closing or restarting the interface must not destroy a running task. A model response is only one execution step; the runtime must continue through subsequent requests, tool calls, worker handoffs, validation cycles, and recovery strategies until the goal completes or a genuine hard stop condition occurs.

### 28.2 Deep recovery and problem solving

Nirman must use a graduated recovery ladder that begins with transient retry and focused diagnostics, then refreshes context and environment state, changes strategy or worker role, restores a known-good checkpoint, changes model or context profile, delegates to a specialist, and creates an isolated alternative solution before requesting user input.

Every recovery attempt must include a failure fingerprint, evidence, changed strategy, checkpoint reference, and validation result. Repeating the same command, patch, prompt, or model route does not count as a new attempt. The runtime should continue automatically whenever a safe new strategy is available.

### 28.3 Self-observation and self-evaluation

Nirman must create an episode record for every completed, failed, recovered, cancelled, or escalated task. Episode records should summarize the goal class, project profile, provider profile, plan, worker roles, actions, failures, recovery strategies, validation results, resource telemetry, user corrections, and final classification.

The runtime should measure goal completion, evidence completeness, regression rate, recovery success, strategy diversity, repair efficiency, tool reliability, provider reliability, self-update safety, and human intervention rate. These metrics should be visible for diagnosis and should not be optimized at the expense of correctness or safety.

### 28.4 Self-improvement manager

Nirman should identify recurring failure patterns, provider incompatibilities, repeated user corrections, regression clusters, tool failures, and evaluation degradation. It should convert sufficiently repeated patterns into scoped improvement proposals containing evidence, hypothesis, affected components, expected benefit, risks, test plan, and rollback plan.

### 28.5 Autonomy-level capability ladder

Autonomy is a capability ladder, not a model personality claim. Each level is achieved only when the listed capability rows and evidence gates pass:

| Level | Runtime meaning | Required capability evidence |
|---|---|---|
| `ASSISTED` | The system proposes plans and changes while the user initiates each meaningful action | Intent, authority, workspace, and validation records |
| `SUPERVISED` | The system executes approved local work continuously while policy boundaries remain visible | Background execution, worker, trigger, and evidence records |
| `UNATTENDED_LOCAL` | The system continues routine Android work without per-step clicks under an explicit policy profile | Budgeted autonomy, trusted extensions, context governance, and recovery evidence |
| `ADAPTIVE_RECOVERY` | The system diagnoses failures, changes strategy, repairs, and revalidates without blind retries | Failure fingerprints, specialist gates, reconciliation, and runtime evidence |
| `CERTIFIED_AUTONOMY` | The system satisfies the declared Android goal and delivery conditions with complete provenance | Runtime integrity, preview, validation, signing, artifact, and completion decisions |

A higher level cannot be claimed merely because a model is capable of longer responses. The capability registry, policy authority, evidence authority, and completion predicate determine the achieved level. A missing environment, credential, device, provider, or required decision lowers the observed level or produces a truthful blocker.

A self-improvement proposal may modify prompts, task decomposition, model routing, context retrieval, tool schemas, failure classifiers, worker roles, skills, provider adapters, validation rules, or runtime code. Changes to the supervisor, policy engine, sandbox, credentials, updater, database migrations, or evidence engine require the highest validation level and must not be promoted solely from a model-generated proposal.

### 28.6 Candidate evaluation and promotion

Self-improvement must happen in an isolated worktree and candidate runtime. A candidate must pass targeted tests, broad regression fixtures, provider compatibility tests, sandbox and permission tests, migration tests, recovery tests, candidate health checks, smoke tasks, and representative end-to-end task replay.

Promotion should support observe-only, candidate-only, canary, trusted auto-promotion, and manual-promotion modes. Trusted auto-promotion may be enabled for low-risk scoped changes, but stable-controller recovery, rollback artifacts, credential protections, and sandbox boundaries remain non-bypassable.

After promotion, Nirman must monitor candidate outcomes against the previous baseline and automatically roll back or disable the candidate scope when quality, stability, security, or recovery metrics degrade.

### 28.7 Runtime memory boundaries

Nirman should maintain separate task memory, project memory, and runtime-improvement memory. Memory must be generated from validated events and user-confirmed decisions rather than every model statement. The user must be able to inspect, correct, export, and delete memory. Credentials, protected files, and unclassified private content must never enter long-term improvement memory.

### 28.8 End-to-end runtime acceptance criteria

The complete runtime is not considered implemented until it can accept one broad goal, extract requirements, create a durable task graph, run multiple workers, persist events, execute the validation loop, recover from worker/provider/environment failure, survive application restart, produce evidence-backed completion, and continue until the goal is complete or a genuine hard stop condition exists.

The self-improvement loop is not considered implemented until Nirman can observe episodes, detect recurring failures, produce a scoped improvement proposal, build and evaluate a candidate, run a canary, promote it through the stable controller, monitor post-promotion behavior, and automatically roll back without corrupting the active application or user projects.

### 28.9 Core Autonomous Runtime Capabilities

Nirman’s first-class autonomous behavior is defined by the following seven capabilities. These are product-level requirements and must remain visible in the master specification even though their detailed implementation is defined in the technical architecture.

| Core capability | Required behavior |
|---|---|
| **Specialized workers** | Use separate, permission-scoped workers for architecture, coding, debugging, testing, security, visual QA, performance, and release preparation. Workers must communicate through durable task contracts and return evidence-backed handoffs. |
| **Self-healing loop** | Detect failures, classify their causes, change strategy, backtrack to a known-good checkpoint, delegate diagnosis when useful, and continue validation without repeating the same failed approach. |
| **Evidence-based completion** | Never accept a model claim as proof. Require appropriate tests, builds, screenshots, health checks, security results, device results, review findings, or artifacts before marking work complete. |
| **Adaptive resource management** | Compact context, change models, reduce concurrency, retry transient failures, repair the environment, and continue long-running work without arbitrary time or token completion locks. |
| **Self-development mode** | Modify Nirman only in an isolated worktree, build and test a candidate, launch it separately, run health and smoke checks, promote it through the stable controller, and automatically roll back if validation fails. |
| **Project memory** | Remember validated decisions, architecture, conventions, previous fixes, failed strategies, task outcomes, and user preferences across sessions while excluding credentials and protected content. |
| **Environment repair** | Detect missing SDKs, broken dependencies, incompatible versions, occupied ports, unavailable emulators, and toolchain failures, then repair or install project-scoped requirements where policy allows. |

A task is not considered autonomously complete unless the relevant capabilities have either succeeded or been explicitly classified as unnecessary, unavailable, or blocked with evidence. The runtime should continue automatically through these capabilities whenever a safe next action exists.

## 29. End-to-End Android Generation Contract

The primary product promise is that one user instruction and optional screenshots launch one durable Android engineering session. The session must continue through input analysis, visual specification, technology selection, project synthesis, live preview, implementation, testing, repair, validation, packaging, and evidence-backed completion without routine human intervention.

### 29.1 Input fusion

The session combines the user’s chat instruction, screenshots, supplied assets, existing project files, device requirements, integrations, and delivery requirements into three authoritative inputs: an `AndroidApplicationContract`, a `VisualSpecification`, and an `AndroidTechnologyPlan`. The user does not select a framework or template. The configured AI resolves the implementation from these inputs.

### 29.2 Autonomous Android session

```text
AutonomousAndroidSession
- sessionId
- userGoal
- screenshotsAndAssets
- applicationContract
- visualSpecification
- technologyPlan
- taskGraph
- workerRegistry
- terminalSessions
- sandboxProfile
- activeProjectRevision
- previewState
- checkpoints
- validationState
- recoveryState
- artifactState
- completionState
```

The session owns the complete task independently of the chat interface. It remains resumable after the interface closes, the process restarts, or the host resumes from sleep where the operating system permits it.

### 29.3 Live preview and execution synchronization

The live Android Nirman-managed local Android emulator is a first-class execution surface. Every preview state must expose the project revision, checkpoint ID, emulator identity, installation state, reload state, Logcat, runtime errors, latest screenshot, visual comparison result, and the worker or task responsible for the current change.

If a candidate change breaks the application, the preview must show the last valid revision and identify the failed candidate. The execution tree and preview must share a revision identifier so the user can see exactly which work produced the running application.

### 29.4 Progress ledger and stall detection

The runtime must maintain a progress ledger recording changed files, new evidence, preview revision movement, test transitions, worker handoffs, strategy changes, and validated requirements. A stall detector must identify repeated commands, repeated patches, repeated failure fingerprints, unchanged workspaces, unchanged previews, missing evidence, unresponsive processes, and heartbeats without useful progress.

When a stall is detected, the runtime must refresh context, change strategy, change technology, delegate diagnosis, repair the environment, restore a checkpoint, or construct an isolated alternative. It must not repeat the same action indefinitely.

### 29.5 Swarm handoff and reconciliation

Parallel workers must receive explicit contracts and isolated workspaces. Each handoff must include changed files, assumptions, dependencies, tests, evidence, unresolved issues, and recommended next actions. The reconciliation worker integrates only validated outputs, resolves conflicts, runs integrated Android checks, updates the live preview, and creates the next checkpoint.

### 29.6 APK completion gates

A task is complete only when its applicable completion conditions are proven. For Android delivery, the evidence must include a successful build, an APK artifact or an AAB artifact only when the active PackagingProfile requires `APK_AND_AAB`, a checksum, artifact scanning, installation or launch evidence, main-flow results, screenshot or visual validation, required permission behavior, and no unresolved fatal runtime errors. The final artifact must link to the project revision and evidence ledger.

### 29.7 No-routine-intervention policy

Routine project-local actions may continue automatically under the configured Unattended / Full Autonomy policy, including editing, dependency installation, terminal commands, emulator launches, builds, tests, screenshots, repair attempts, checkpoints, worker handoffs, and local artifact creation. Only protected credentials, destructive operations, external publishing, signing policy, protected paths, missing required information, hard safety violations, or unrecoverable technical blockers may interrupt the session.

### 29.8 Full Android capability acceptance

The product must validate AI-selected generation across JavaScript-driven Android projects, Java, Kotlin, Android Views, Jetpack Compose, mixed architectures, custom native modules, background services, WorkManager, notifications, camera and media, location and sensors, Bluetooth and NFC, offline-first storage, API-heavy applications, authentication and permissions, tablet and multi-orientation layouts, device-integrated applications, and APK delivery. These are internal acceptance categories, not user-facing templates.

---

## 30. Android Completion Report

The final completion screen must show the application identity, selected technology plan and reasons, final emulator or emulator state, build and validation results, APK paths and checksums, recovery history, source revision, checkpoints, warnings, and unresolved issues. A model-generated statement that the work is complete is never sufficient evidence.

---

## 31. Final System Principle

> **The user gives one Android application idea and optional screenshots once. The system works continuously in the background, dynamically chooses the Android implementation, updates the live preview, coordinates terminals and workers, heals failures, validates the result, and returns a working APK with evidence.**

The complexity belongs inside the runtime rather than inside the user’s workflow. Deterministic lifecycle, permission, sandbox, storage, evidence, recovery, promotion, rollback, and termination authorities remain in control while the configured AI proposes and executes development work within the approved policy.

---

## 32. Autonomous Runtime Capability Contract

The runtime must provide specialized workers, a self-healing loop, evidence-based completion, adaptive resource management, self-development, project memory, and environment repair as core capabilities. These capabilities are mandatory parts of the end-to-end Android session rather than optional extensions.

**Acceptance statement:** A representative Android task can be launched from one instruction and optional screenshots, continue through background implementation, update the live preview, recover from injected worker/process/provider/device failures, produce evidence for each completion condition, and return a validated APK without routine approval pauses.


## 33. Production Runtime Contract

**ContractId:** `CONTRACT.RUNTIME.AUTHORITY`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.AUTHORITY` (see §67.8)

Nirman must treat the autonomous Android build as a deterministic runtime session rather than a sequence of independent chat responses. The model proposes plans and actions; the runtime owns lifecycle, permissions, filesystem access, process execution, emulator access, persistence, evidence, recovery, promotion, rollback, and termination.

### 33.1 Canonical runtime contracts

The implementation must define versioned, validated contracts for:

| Contract | Responsibility |
|---|---|
| `AutonomousAndroidSession` | Owns the full task from one user request to validated APK output |
| `AndroidApplicationContract` | Captures features, screens, behavior, integrations, devices, permissions, and acceptance conditions |
| `VisualSpecification` | Captures screenshot-derived layouts, states, components, typography, color, spacing, and comparison rules |
| `AndroidTechnologyPlan` | Records AI-selected languages, UI systems, native modules, SDKs, libraries, device APIs, and build strategy |
| `TaskGraph` | Defines phases, dependencies, workers, inputs, outputs, checkpoints, and completion conditions |
| `WorkerContract` | Defines worker purpose, workspace, tools, permissions, inputs, outputs, and validation rules |
| `TerminalSession` | Tracks shell, working directory, environment, process tree, PTY, input policy, output, and recovery |
| `PreviewRevision` | Binds Nirman-managed local Android emulator state to a project revision and checkpoint |
| `EvidenceRecord` | Stores proof from tests, builds, screenshots, Logcat, permissions, scans, and artifacts |
| `RecoveryRecord` | Stores failure fingerprints, attempted strategies, backtracking, and outcomes |
| `ArtifactRecord` | Stores APK metadata, checksum, build profile, signing state, scans, and source revision |
| `ProviderProfile` | Stores endpoint, model ID, protocol, capabilities, privacy policy, and routing role |

All durable contracts require explicit versioning, schema validation, atomic persistence, migration, backup, and rollback. No model output may create undocumented fields or alter authority rules.

### 33.2 Authoritative lifecycle

The session lifecycle must be explicit and persisted:

```text
Created → Understanding → Planning → EnvironmentPreparing
  → ProjectSynthesizing → Implementing → Previewing
  → Testing → Recovering → Revalidating → Packaging → Completed
```

Safe terminal states are `BlockedByPolicy`, `BlockedByMissingInformation`, `ProviderUnavailable`, `EnvironmentUnrecoverable`, `Cancelled`, and `SafelyFailed`. Models, workers, skills, hooks, and UI events may propose transitions but cannot commit them directly.

### 33.3 Renewable leases and operation capabilities

Long-running work must use a renewable session lease rather than a short fixed execution token. The supervisor renews the lease only while heartbeats, progress, and authority checks remain valid. Individual sensitive operations use scoped, single-use operation capabilities bound to the session, worker, workspace, project revision, action type, and expiry.

An operation capability is required for actions such as installing a risky dependency, changing protected configuration, accessing a device capability, signing an artifact, publishing, or promoting a self-update. A model cannot mint, extend, or broaden a capability.

## 34. Android Project Ingestion and Integrity

The project-ingestion layer must understand Android source files, Gradle settings, manifests, resources, assets, fonts, localization, JavaScript package manifests where selected, native-module boundaries, Nirman-managed local Android emulator configuration, generated build directories, secrets, keystores, local properties, environment files, Git state, and uncommitted changes.

The layer must apply hard exclusions, canonical path normalization, project-root boundaries, scope fingerprints, content hashes, and revision checks. Before reconciliation, preview installation, packaging, or self-development promotion, it must detect external changes and revalidate the active project revision. A stale or mismatched revision must be rejected rather than silently overwritten.

## 35. Provider Gateway and Controlled Tool Protocol

The Model Gateway must normalize configured Chat Completions, Responses-style requests, message history, screenshot inputs, structured outputs, tool calls, tool results, streaming task events, cancellation, usage, context limits, provider errors, and model capabilities.

The user owns each endpoint, API key, base URL, and model ID. Nirman must not silently replace a configured model. Explicitly approved role profiles may route planning, coding, visual inspection, debugging, testing, and review to different providers or models.

Every tool call must have a typed name, version, schema-validated arguments, session ID, worker ID, project policy, privacy classification, requested capabilities, and evidence result. Unknown tools, unknown arguments, unapproved routing, secret access, and malformed tool results must be rejected before execution.

## 36. Execution Isolation and Sandbox Boundaries

The runtime must separate the Windows host, control-plane supervisor, worker processes, Android build processes, Nirman-managed local Android emulator processes, preview application, provider network access, project files, credentials, and signing material.

Generated code must not automatically access personal files, browser cookies, SSH keys, API keys, signing keys, unrelated projects, or arbitrary network resources. Each process receives the minimum filesystem, network, process, and device permissions required by its contract. Sandbox policy is enforced by deterministic runtime authorities and cannot be weakened by model output.

## 37. Event, Evidence, and Completion Authority

**ContractId:** `CONTRACT.RUNTIME.EVIDENCE`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.EVIDENCE` (see §67.8)

**ContractId:** `CONTRACT.RUNTIME.AUTHORITY`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.AUTHORITY
- authoritySection: §33
- extendingSection: §37
- extensionType: adds_clauses
- extendedClauses: CLAUSE.EVIDENCE.CLAIM_SEPARATION, CLAUSE.EVIDENCE.FRESHNESS
- nonOverriddenClauses: CLAUSE.AUTHORITY.MODEL_PROPOSES, CLAUSE.AUTHORITY.NO_SELF_ELEVATION

**ContractId:** `CONTRACT.RUNTIME.PLATFORM_CAPABILITY`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.PLATFORM_CAPABILITY
- authoritySection: §79
- extendingSection: §37
- extensionType: adds_component
- extendedClauses: none
- nonOverriddenClauses: CLAUSE.PLATFORM.HOST_TARGET_SEPARATION, CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION, CLAUSE.PLATFORM.EVIDENCE_ENV_BINDING, CLAUSE.PLATFORM.VALIDATION_ENV_RESERVATION, CLAUSE.PLATFORM.NO_SUBSTITUTE_TARGET

Nirman must distinguish among model claims, runtime events, and evidence records. A model statement such as “the login screen is complete” is not completion evidence. Completion requires applicable proof from builds, installation, automated flows, screenshots, visual comparison, Logcat, permissions, security scans, performance checks, and APK metadata.

The final report must identify what passed, what failed, what was repaired, what could not be tested, the source revision, the active checkpoint, the artifact checksum, and any unresolved warnings. No model claim may mark a requirement complete without a corresponding evidence record.

## 38. Privacy-Scoped Memory, Replay, and Recovery History

**ContractId:** `CONTRACT.RUNTIME.MEMORY`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.MEMORY` (see §67.8)

Memory must be divided into session memory, project memory, runtime-improvement memory, and credential storage. Every memory entry must include source, confidence, project scope, timestamp, revision, retention policy, and deletion support. Credentials, signing keys, raw secrets, and unclassified private content must never enter semantic memory.

Users must be able to reopen a completed or failed session, inspect the task and worker timeline, compare preview revisions, rerun validation, fork a failed task into a new strategy, replay a task with an approved provider, restore a checkpoint, download APK evidence, and inspect why the technology resolver selected a particular implementation.

## 39. Production Windows Host Requirements

The desktop host must use backend-only file access, explicit capability permissions, atomic state writes, file locking, versioned migrations, crash recovery, offline startup, prerequisite validation, signed per-user installers, upgrade rollback, state preservation, memory-leak testing, large-project virtualization, local editor assets, and privacy-filtered local logs.

Provider unavailability must not prevent the host from opening projects, history, checkpoints, and settings. Execution must be disabled or marked unavailable until an approved provider is ready.

## 40. User-Facing Productivity Features

The core workspace must provide one-click goal launch, live task tree beside the Android preview, pause/resume/cancel/fork/retry-from-checkpoint, a technology rationale panel, a changed-files timeline, device-matrix testing, visual comparison, build-health status, an APK artifact center, recovery explanations, an editable project-memory view, task replay, a privacy/network context panel, and an environment-repair center.

These features expose the runtime’s state without forcing the user to understand internal worker orchestration. The user gives the goal; Nirman manages the complexity.

## 41. Production Readiness Principle

> **Nirman must be autonomous in execution and recovery, but deterministic in authority.**

The application may continue automatically through routine project-local work, but no model, worker, skill, hook, or external tool may grant permission, bypass the sandbox, delete recovery state, mark work complete without evidence, promote an unvalidated candidate, or suppress a hard safety termination.


---

## 42. Integrated Android Construction and Runtime Contracts

This section incorporates the strongest reusable construction and runtime principles identified in the Sync-AI reference set. It does not change Nirman’s product scope: Nirman remains a Windows-first desktop host that generates Android applications only. No user-facing framework catalog is exposed, and the AI remains responsible for selecting and composing the Android implementation.

### 42.1 AndroidConstructionContract

Every autonomous build session MUST produce a versioned AndroidConstructionContract before implementation begins. The contract is the canonical handoff between user intent, planning, workers, preview, validation, and artifact production.

| Contract area | Required contents |
|---|---|
| Application identity | Display name, package ID, namespace, version, description, branding intent, privacy classification |
| Intent model | Original request, screenshot references, explicit constraints, inferred requirements, assumptions, unresolved ambiguities |
| Feature model | User stories, feature IDs, dependencies, mandatory/optional status, acceptance tests, affected screens |
| UI model | Screens, components, navigation, visual states, interactions, accessibility semantics, localization, theme behavior |
| Data model | Entities, relationships, persistence choice, migrations, caching, synchronization, offline and corruption recovery behavior |
| Integration model | APIs, authentication, notifications, storage, camera, media, sensors, maps, biometrics, payments, and native services |
| Technology plan | Selected languages, UI systems, libraries, native modules, build systems, runtime versions, and rationale |
| Android requirements | Minimum/target/compile SDK, ABI, permissions, manifest entries, background behavior, API-level constraints |
| Device matrix | Nirman-managed local Android emulator profiles, API levels, orientations, densities, tablet/phone coverage |
| Validation model | Unit, integration, UI, visual, accessibility, performance, security, runtime, and release checks |
| Artifact model | APK variants, signing policy, version code, checksums, evidence requirements, export destinations |

The contract MUST use explicit schema versions, reject unknown fields where strict validation is required, record source references for inferred fields, and distinguish user-provided facts from model inferences. A worker MUST NOT invent a contract field absent from the canonical schema.

### 42.2 ConstructionTransaction

Every mutation, dependency change, toolchain repair, preview promotion, signing operation, and artifact promotion MUST be represented by a ConstructionTransaction.

```text
ConstructionTransaction
├── transaction_id
├── session_id
├── task_id
├── worker_id
├── provider_profile_id
├── model_id
├── trace_id
├── base_project_revision
├── pre_mutation_checkpoint_id
├── requested_operations
├── required_permissions
├── policy_decision
├── candidate_revision
├── validation_evidence_ids
├── commit_result
└── rollback_result
```

The transaction lifecycle is:

```text
PROPOSED → SCOPED → SNAPSHOTTED → VALIDATED → APPLIED
        → INDEXED → TESTED → PREVIEWED → COMMITTED
```

Any failed stage transitions to REJECTED, ROLLED_BACK, WAITING, RETRYABLE_FAILURE, or SAFE_FAILURE. A model response is never a commit. The runtime authority owns transaction acceptance, rollback, and promotion.

### 42.3 Pure Reducer and Replayable State

The autonomous session state MUST be reconstructed by a deterministic reducer:

```text
previous durable state + validated runtime event = next durable state
```

The reducer MUST be side-effect free. Filesystem writes, process launch, provider calls, emulator commands, and artifact operations belong to command handlers that emit validated events. This enables crash recovery, deterministic replay, impossible-transition detection, and property-based testing.

The reducer MUST reject events for unknown sessions or tasks, stale project revisions, completion events without required evidence, promotion events without artifact checksums, worker events from expired leases, preview events for unrelated revisions, and transitions that bypass checkpoint, policy, or validation gates.

### 42.4 Recovery Governance

Nirman MUST provide autonomous recovery without uncontrolled mutation loops. Availability waits such as provider outage, emulator offline, device unavailable, or temporary locks may renew under an active session lease. Code, dependency, manifest, resource, and architecture changes use bounded attempts per strategy, checkpoint restoration, strategy changes, and escalation.

| Tier | Failure class | Typical examples |
|---|---|---|
| T1 | Environment/configuration | Missing JDK, SDK, Gradle wrapper, corrupted cache, invalid path |
| T2 | Dependency/toolchain | AGP/Kotlin/Compose mismatch, package conflict, NDK mismatch |
| T3 | Source/build | Kotlin, Java, XML, Gradle, TypeScript, native-module, or resource error |
| T4 | Runtime/integration | Crash, navigation failure, permission denial, API integration failure |
| T5 | Visual/behavioral | Screenshot mismatch, layout overflow, accessibility, orientation, or interaction failure |
| T6 | Structural | Invalid technology plan, missing capability, incompatible architecture, or broken contract |

A worker makes one repair proposal per attempt. The supervisor owns retry count, checkpoint restoration, memory reset, strategy changes, concurrency reduction, and safe terminal-state selection.

> Integrated principle: the model proposes the construction strategy; deterministic Nirman authorities validate, apply, observe, repair, roll back, and promote only evidence-backed Android artifacts.

## 43. Android Code Intelligence and Mutation Contract

### 43.1 Language-Neutral Android Code Intelligence

Nirman MUST use a language-neutral Android code-intelligence layer with adapters for Kotlin, Java, XML, Android manifests, Gradle Kotlin DSL, Gradle Groovy, TypeScript, JavaScript, C/C++ native modules, JSON, YAML, TOML, SQL, and lockfiles.

The graph MUST track files, modules, symbols, references, Gradle dependencies, manifest permissions, resource references, navigation routes, native-module boundaries, test-to-source relationships, API-level compatibility, and generated artifacts. Lightweight indexing may support discovery and browsing; full semantic indexing is required before high-impact mutation, reconciliation, packaging, signing, or promotion.

### 43.2 Structured Mutation Broker

Model output MUST pass through the mutation broker. Direct model writes to project files are forbidden. The broker validates project scope, path normalization, base revision, file ownership, schema, syntax, mutation budget, dependency policy, and evidence requirements.

| File category | Preferred transformation |
|---|---|
| Kotlin/Java | PSI, AST, symbol-aware patch, or validated structured generation |
| XML/manifest/resources | Schema-aware XML transformation |
| Gradle Kotlin DSL/Groovy | Parser-aware or block-aware transformation followed by syntax validation |
| TypeScript/JavaScript | TypeScript AST or parser-aware transformation |
| C/C++ native module | Clang/parser-aware transformation where available |
| JSON/YAML/TOML | Schema-validated serialization |
| Unknown/generated/vendor file | Isolated whole-file replacement followed by syntax, build, and integrity validation |

The broker MUST reject blind search-and-replace mutations for high-risk source files. Whole-file generation is allowed only inside an isolated transaction and only when the resulting file passes syntax, graph, build, test, and content-integrity gates.

### 43.3 Project Impact Graph

Before a refinement, Nirman MUST calculate affected files, modules, resources, tests, permissions, preview surfaces, and artifact outputs. The impact graph MUST support incremental indexing, affected-test selection, dependency conflict analysis, navigation and resource reachability, manifest/API usage correlation, long-horizon map sharding, checkpoint-aware invalidation, and reconciliation conflict detection.

---

## 44. Preview, Branding, and Data-Layer Requirements

### 44.1 Preview Fallback Matrix

The live preview coordinator MUST select a preview mode appropriate to the selected Android technology and current revision.

| Preview mode | Use case | Required evidence |
|---|---|---|
| Incremental emulator install | Native changes that compile successfully | Install result, process health, screenshot |
| Compose reload | Compose-compatible UI change | Reload event, state continuity, screenshot |
| React Native/Expo fast refresh | JavaScript/TypeScript-only change | Metro/Expo health, rendered screen, screenshot |
| Full APK reinstall | Manifest, resource, dependency, native, or major build change | APK hash, install, launch, screenshot |
| Nirman-managed local Android emulator preview | User-approved connected device | Device identity, install, launch, capture, Logcat |
| Headless smoke test | Preview device unavailable | Test output, runtime logs, health result |
| Diagnostic/source preview | Build unavailable during recovery | Diagnostics only; cannot satisfy completion |

Every preview is bound to PreviewRevision, project revision, emulator identity, build variant, and technology plan. A stale preview MUST be visibly labeled and MUST NOT satisfy final completion gates.

### 44.2 Android BrandManifest

Nirman may infer branding from the application contract, screenshots, domain semantics, and user preferences, but it MUST not use Windows-specific visual assumptions. BrandManifest covers display name, semantic description, light/dark colors, typography, spacing, adaptive icon assets, splash assets, notification icons, empty states, density variants, accessibility contrast, provenance, prompt hash, provider/model ID, and output hashes.

AI image seeds are recorded as inputs, but exact reproducibility MUST be verified from output hashes rather than assumed. Content-addressed caching and explicit regeneration records are required.

### 44.3 Android Data-Layer Resolver

Nirman MUST choose a data strategy from the application contract rather than enforcing one fixed database technology. Valid choices include Room with SQLite, direct SQLite, DataStore, encrypted local storage, a justified alternative local store, network cache/synchronization, or a composed strategy.

The resolver MUST produce migration rules, corruption recovery rules, seed-data policy, offline behavior, encryption requirements, test fixtures, and an evidence plan. The selected data strategy becomes part of the technology plan and cannot be changed by a worker without a versioned plan update and reconciliation.

### 44.4 Default visual system

When the user states no brand direction, color, or aesthetic preference, generation starts from a defined baseline rather than an arbitrary one. These are defaults, not contract: any explicit user preference overrides them, and conformance to them is never a certification criterion.

| Dimension | Default |
|---|---|
| Design system | Material 3 baseline, with dynamic color where the target supports it |
| Spacing | 8dp grid for all margins, padding, and component sizing; 4dp permitted only for dense inline elements |
| Typography | Material 3 type scale; system default font family |
| Elevation | Applied only to express hierarchy or interactive affordance, never decoratively |
| Contrast | WCAG AA minimum, 4.5:1 for body text and 3:1 for large text and meaningful non-text elements |
| Component styling | Material 3 default component states before any customization |

Contrast is the only row that is more than a preference: a generated screen that fails the AA minimum is a defect, because it is unreadable for a real population of users rather than merely unattractive.

The resolved values MUST be recorded in `BrandManifest` (§44.2) so that a generated appearance is reproducible and auditable, whether it came from a user preference or from these defaults.

### 44.5 Default application states and motion

A user instruction describes what an application does when it works. It almost never describes what the application shows when it is empty, loading, failing, or launched for the first time. These states are the majority of a real user's experience, so they have defined defaults rather than being left to per-generation improvisation.

These are defaults, not contract: an explicit user preference overrides any row, and conformance is never a certification criterion.

| State | Default |
|---|---|
| Empty collection | Muted illustrative icon, one line naming what belongs here, and the primary creation action. A bare "No items" string is insufficient |
| Loading, known content shape | Skeleton placeholders matching the eventual layout |
| Loading, unknown shape or duration | Indeterminate progress indicator |
| Loading under approximately 300 ms | No indicator at all; a control that flashes and disappears reads as a defect |
| Transient operation failure | Snackbar carrying a retry action |
| Field or input validation failure | Inline message beneath the offending field, with the field marked |
| Blocking or destructive failure | Dialog requiring acknowledgement |
| First launch, no data | Empty shell with its empty state; no seeded example data |
| Irreversible action | Confirmation, plus undo where the operation permits it |

Two rows carry reasoning that must not be lost if the table is ever revised.

First launch MUST NOT seed fabricated example data. Example rows the user must identify and delete are a poor first impression, and they place content in the application's own data store that the user never created. Where a data strategy defines a seed-data policy under §44.3, that policy governs; the default in the absence of one is an empty store. A generated screenshot or preview showing seeded rows MUST identify them as seeded, because an observation that displays fabricated data as user data misrepresents application state.

A user-facing failure message states what failed and what the user can do next. An exception type, stack frame, error code, or raw provider string is not a user-facing message. Diagnostic detail remains available in the evidence record, not in the application's own UI.

Motion defaults, when the instruction says nothing about motion:

| Motion | Default |
|---|---|
| Touch and state feedback (ripple, pressed, focused, disabled) | Always present; absence reads as an unresponsive application |
| Standard navigation transitions | Platform defaults |
| List insertion, removal, and reorder | Animated, so the change is legible |
| Haptic feedback | Destructive or irreversible confirmations only |
| Parallax, hero transitions, spring physics, custom easing, decorative loops | Never added unless requested |

The distinguishing test is whether the motion communicates causality or continuity — what caused this, and where did it come from. Motion that only draws attention to itself is decoration and is not added by default. Motion beyond these defaults falls under §5.4's inference ceiling: timing and feel are not reliably inferable from prose, and a request for elaborate motion warrants a clarifying question under §69.11 rather than an invented interpretation.

Dark theme, orientation change, and larger screens are not optional polish and are not deferred to a later revision:

| Dimension | Default |
|---|---|
| Dark theme | Always generated, through Material 3 color roles. Hardcoded color literals in layouts or composables are the defect that breaks dark theme, not the theme itself |
| Orientation change | Both orientations supported. State survives configuration change through retained state holders and saved instance state |
| Larger screens | Layout adapts at the standard width breakpoints. Full-width single-column form fields and hardcoded single-pane assumptions are the common failure |

Locking an application to portrait, or omitting a dark theme, to avoid handling configuration change is prohibited. It converts a generation defect into a permanent product limitation, and it does so invisibly to the user who never asked for either restriction.

The dominant failure on orientation change is loss of state, not layout distortion. §56's configuration-change scenario class is the evidence path for this behavior; this subsection states the generation default, and does not add, alter, or duplicate any test.

---

## 45. Autonomous UX, Decision Trace, and Resource Governance

### 45.1 Progressive Disclosure

Nirman MUST hide unnecessary implementation complexity by default without hiding truth. The UI provides three levels:

| Mode | Visible information |
|---|---|
| Calm | Current phase, meaningful progress, live preview, latest update, working/waiting state |
| Inspect | Task graph, workers, terminal summaries, changed files, checkpoints, devices, recovery, evidence |
| Developer | Structured diagnostics, provider/model provenance, decision trace, command details, environment snapshot, replay controls |

Raw secrets, private keys, and unfiltered prompts are never displayed. Blocked, waiting, recovering, and safely-failed states MUST be explicit.

### 45.2 DecisionTrace

For each material autonomous decision, Nirman records a concise DecisionTrace containing decision ID, session/task/worker IDs, input references, constraints, candidate actions, selected action, deterministic policy checks, provider/model provenance, confidence, outcome event, and evidence IDs. Hidden chain-of-thought is not stored or exposed.

### 45.3 ResourceGovernor

The resource governor monitors CPU, memory, disk, checkpoint storage, emulator memory, Gradle memory, worker concurrency, provider concurrency, context size, log volume, build duration, and device slots.

Under pressure it may compact context, reduce concurrency, prune safe caches, stop redundant workers, run affected tests, defer nonessential visual checks, or switch to an approved lighter provider profile. It MUST NOT silently weaken sandboxing, permissions, evidence, signing, or artifact gates.

### 45.4 EnvironmentSnapshot

Every substantial build, recovery cycle, and final artifact MUST include an environment snapshot recording operating-system host metadata, toolchain versions and hashes, relevant environment variables, emulator identity/API level, provider profile and model metadata without secrets, workspace revision, lockfile hashes, and build flags.

---

## 46. Non-Goals Preserved by This Integration

The following remain explicitly outside Nirman’s generated-target scope: Windows application generation; web application generation; WinUI, WPF, WinForms, Win32, WinRT, MSBuild, MSIX, MSI, or Windows-manifest target generation; Roslyn, XAML, or EF Core as mandatory implementation technologies; a user-facing framework or template catalog; direct model writes to files; unrestricted model shell authority; unauthenticated local provider access; uncontrolled infinite mutation retries; and completion based solely on model claims.

Internal bootstrap scaffolding is permitted only when required to create a valid Android project; it is not a user-facing template limitation and does not constrain the AI’s technology selection.

### 46.1 Product Acceptance Additions

The integration is complete only when a complete AndroidConstructionContract can be created, versioned, validated, and replayed; every mutation is represented by a ConstructionTransaction with a checkpoint and project revision; the session can be reconstructed after forced process termination; a clean-machine build uses only the locked Android toolchain; provider bridge failures are handled without corrupting the session; multi-language changes pass the structured mutation broker; parallel workers reconcile through a serialized commit barrier; Android permission and requirement drift is detected before artifact promotion; preview is revision-bound; resource pressure changes scheduling without weakening safety; and a completed APK contains checksums, environment snapshot, validation evidence, source revision, and artifact provenance.


---

## 47. Integrated Android Workflow and Quality Intelligence

**ContractId:** `CONTRACT.RUNTIME.EVIDENCE`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.EVIDENCE
- authoritySection: §37
- extendingSection: §47
- extensionType: adds_verification
- extendedClauses: none
- nonOverriddenClauses: CLAUSE.EVIDENCE.CLAIM_SEPARATION, CLAUSE.EVIDENCE.FRESHNESS

**ContractId:** `CONTRACT.RUNTIME.VERIFICATION`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.VERIFICATION
- authoritySection: §57
- extendingSection: §47
- extensionType: adds_verification
- extendedClauses: none
- nonOverriddenClauses: CLAUSE.VERIFY.IN_LOOP, CLAUSE.VERIFY.ASSERTION_ORDER, CLAUSE.VERIFY.NON_VACUITY

### 47.1 IntegratedAndroidWorkflowCoordinator

Nirman MUST provide one canonical coordinator for the complete Android construction lifecycle. This is a runtime service, not a single oversized prompt. It connects user input, screenshots, contract generation, feasibility analysis, technology selection, worker allocation, transactional mutation, build, preview, testing, self-critique, repair, packaging, and evidence-backed promotion.

```text
User request and screenshots
        ↓
Prompt normalization
        ↓
AndroidConstructionContract
        ↓
Preflight risk and feasibility gate
        ↓
AndroidTechnologyPlan
        ↓
Task graph and worker allocation
        ↓
Structured mutation transactions
        ↓
Build, preview, and tests
        ↓
Independent quality and security review
        ↓
Device validation
        ↓
APK packaging and evidence promotion
```

The coordinator MUST persist each boundary as a durable event and MUST be able to resume from the last validated boundary after a supervisor, worker, provider, emulator, or host interruption.

### 47.2 PreflightReport and feasibility gate

Before expensive generation begins, Nirman MUST produce a `PreflightReport`. The report evaluates the selected or candidate technology plan against the local environment, project constraints, provider capabilities, privacy policy, emulator availability, and expected validation work.

| Preflight area | Required checks |
|---|---|
| Provider | Authentication, protocol, model capabilities, context limit, vision/tool support, privacy policy |
| Toolchain | JDK, Gradle, Android Gradle Plugin, Kotlin, SDK, build tools, platform tools, NDK/CMake, Node/Metro/Expo when needed |
| Workspace | Writable scope, disk space, project fingerprint, lockfiles, credentials exclusion, checkpoint capacity |
| Device | Nirman-managed local Android emulator, API level, ABI, storage, ADB health, orientation, required hardware capabilities |
| Dependencies | Availability, compatibility, vulnerability/license policy, lockfile status, native build requirements |
| Requirements | Permissions, manifest entries, background rules, accessibility, localization, offline behavior, signing prerequisites |
| Resource forecast | CPU, memory, disk, emulator memory, worker count, provider concurrency, expected validation stages |

Each risk records severity, probability, affected phase, evidence, mitigation, fallback, and whether autonomous repair is permitted. Routine toolchain or cache repair may proceed under policy. Credentials, privileged access, unavailable required devices, and policy restrictions become explicit waiting or blocked conditions rather than endless retries.

### 47.3 AndroidQualityGate

Before artifact promotion, independent review workers MUST evaluate correctness, architecture, security, dependencies, runtime behavior, visual fidelity, accessibility, performance, test coverage, and release integrity.

| Finding class | Completion behavior |
|---|---|
| Blocking | Must be repaired, independently waived by an allowed policy, or prevent artifact promotion |
| Warning | May proceed only with recorded rationale and evidence |
| Informational | Recorded for improvement and does not block completion |

The quality gate MUST be independent from the worker that produced the implementation. A quality score alone is never completion evidence.

### 47.4 FailureModeRecord

Nirman MUST maintain a proactive Android failure-mode catalogue. Every important failure mode has a trigger, prevention check, classifier, recovery strategy, scope, stop condition, and evidence requirement.

Initial failure families include toolchain incompatibility, missing SDK components, dependency conflicts, lockfile drift, resource linking failures, manifest merge failures, duplicate classes, DEX/R8 errors, native-module failures, emulator and ADB failures, install failures, runtime crashes, ANRs, permission denials, offline-data corruption, visual regressions, inaccessible controls, signing failures, and invalid APK metadata.

### 47.5 Acceptance-test traceability

Every mandatory requirement MUST map to at least one executable acceptance criterion and one validation path.

```text
Requirement → acceptance criterion → test → execution result → evidence → artifact revision
```

The traceability matrix records skipped, blocked, flaky, and passing tests honestly. A final artifact cannot claim complete implementation when a mandatory requirement has no executable validation or has unresolved blocking evidence.

### 47.6 Architecture and contract drift

After every major transaction, Nirman MUST compare the project against the approved `AndroidConstructionContract` and `AndroidTechnologyPlan`. Drift detection identifies missing features, undocumented permissions, unreachable screens, data models without migrations, acceptance criteria without tests, dependencies outside the approved plan, unauthorized architecture changes, stale generated files, and preview or artifact outputs from unrelated revisions.

Drift findings are classified as blocking, repairable, warning, or informational. A worker cannot silently update the contract to make drift disappear; contract changes require a versioned plan update and reconciliation event.

### 47.7 Project handbook and release intelligence

Each managed Android workspace MUST contain a concise generated project handbook describing purpose, selected technology plan, modules, commands, toolchain lock, environment assumptions, privacy rules, permissions, build/test instructions, known limitations, current revision, and recovery notes.

Each promoted APK MUST have a release-intelligence report containing dependency inventory, permission inventory, data-handling summary, test and device results, performance summary, known warnings, artifact hashes, signing status, source revision, toolchain lock, and environment snapshot.

### 47.8 Worker quality metrics and validated repair promotion

Nirman SHOULD measure worker and strategy quality using success rate, regression rate, time-to-evidence, false-positive review rate, repair reuse rate, handoff completeness, affected-test precision, and rollback frequency. Metrics are for routing and improvement; they do not grant permissions.

A learned repair or pattern may enter the trusted registry only after repeated successful validation on the originating project and independent fixtures. The stored record includes failure fingerprint, environment, strategy, changed scope, validation evidence, regression results, and confidence. Model suggestions remain untrusted until promoted by deterministic evidence.

### 47.9 Bounded structured reasoning

Nirman MAY use prompt normalization, self-critique, logical consistency checks, alternative-solution analysis, risk prediction, reflection, and strategy scoring. These services MUST return bounded structured outputs such as assumptions, alternatives, selected action, constraints, confidence, and evidence references. Hidden chain-of-thought MUST NOT be stored or shown. No reasoning service may override the runtime authorities.

---

## 48. Product Scope Decisions from the Integrated Review

The following are explicitly not adopted: web application generation, Windows application generation, PWA delivery, a universal web-wrapper architecture, exposed hidden reasoning transcripts, unbounded recursive worker spawning, automatic remote publication, and completion claims based on module counts or unsupported implementation percentages.

Nirman uses native Windows isolation as its required execution model: restricted tokens, Windows Job Objects, ACL-scoped workspaces, environment filtering, process-tree supervision, resource quotas, toolchain isolation, and disposable Android emulator snapshots. Remote Git operations, publication, store submission, and release signing remain explicit policy-controlled operations.

The central product rule remains:

> **One instruction plus optional screenshots should produce a complete, validated Android application through a durable, recoverable, inspectable, and evidence-backed autonomous workflow.**

### 48.1 Additional acceptance criteria

1. A preflight report identifies blockers before expensive generation.
2. The integrated coordinator resumes from durable boundaries after interruption.
3. Independent quality workers can block promotion with evidence-backed findings.
4. Every mandatory requirement is traceable to a test and evidence record.
5. Architecture or contract drift cannot be silently ignored.
6. A generated project contains a concise handbook and a promoted artifact contains a release-intelligence report.
7. Learned repairs require independent validation before trusted reuse.
8. Resource pressure and model strategy changes never weaken runtime safety or evidence gates.


---

## 49. Private Internal Reasoning and Visible Structured Reasoning Stream

### 49.1 Product decision

Nirman MAY use private internal model reasoning to support planning, hypothesis generation, self-critique, alternative comparison, error diagnosis, and strategy selection. Private reasoning is an internal computation boundary; it is not displayed to users, persisted as a verbatim transcript, treated as evidence, or granted runtime authority.

Nirman MUST provide a separate live `ReasoningStream` so the user can see what the system is doing during long autonomous sessions. The stream contains concise, useful, filtered summaries rather than raw hidden chain-of-thought.

> **Private reasoning may guide the strategy. Visible structured reasoning explains the strategy. Deterministic runtime authorities control execution.**

### 49.2 Visible reasoning event types

| Event type | User-visible purpose | Example |
|---|---|---|
| `UNDERSTANDING` | Summarize the interpreted request | “This app needs offline task storage, reminders, and dark mode.” |
| `CONSTRAINT` | Show relevant requirements and limits | “Camera is not required; notification permission is required.” |
| `PLAN` | Show the next construction stages | “I will create the data layer, reminder service, screens, and tests.” |
| `ALTERNATIVE` | Show bounded alternatives considered | “A native implementation has lower background-service risk for this requirement.” |
| `DECISION` | Explain a selected technology or strategy | “Selected Kotlin, Compose, Room, and WorkManager for this revision.” |
| `ACTION` | Show an operation currently being performed | “Implementing the reminder scheduler.” |
| `OBSERVATION` | Report an environment, build, preview, or test result | “The first build reports a missing Android SDK platform.” |
| `RECOVERY` | Explain a repair or strategy change | “Repairing the SDK, then retrying the affected build.” |
| `EVIDENCE` | Report proof collected | “Reminder instrumentation test passed on the API 35 emulator.” |
| `NEXT_STEP` | State the immediate planned continuation | “Running accessibility and visual validation next.” |
| `WAITING` | Explain a blocked or waiting condition | “Waiting for the Nirman-managed local Android emulator to reconnect.” |
| `DELIBERATION` | Show that the runtime entered bounded additional reasoning | “Deep deliberation started because two competing hypotheses remain unresolved.” |
| `EFFORT` | Show requested versus granted effort | “Requested DEEP; granted DEEP within the task policy and remaining budget.” |
| `HYPOTHESIS` | Show competing diagnostic candidates | “Three plausible causes remain.” |
| `REFUTATION` | Show evidence that eliminated a hypothesis | “The discriminating test ruled out the dependency-initialization hypothesis.” |
| `MODEL_ESCALATION` | Show a provider/model capability change | “Escalating to the approved reasoning-capable model because uncertainty remained above threshold.” |
| `NO_PROGRESS` | Show why the current reasoning approach stopped | “Further reasoning produced no measurable movement; gathering new evidence.” |
| `DELIBERATION_RESUMED` | Show continuation after compaction/failover | “Resumed deliberation with two rejected hypotheses and remaining budget intact.” |
| `COMPLETION` | Summarize validated output | "APK passed the required gates; optional AAB passed only when the declared packaging profile requires it." |

Every event MUST contain a concise title, human-readable summary, event sequence, session/task/worker IDs, project revision, timestamp, status, provenance references, and evidence IDs when applicable.

### 49.3 Stream behavior

The stream MUST be available while the desktop UI is open and MUST remain recoverable after reconnect, minimization, sleep, reboot, provider restart, or control-plane restart. The UI must show the newest event immediately while retaining a scrollable session history.

The user can pause visual auto-scroll without pausing execution, collapse repeated low-value events, filter by worker or phase, expand evidence links, and switch between Calm, Inspect, and Developer presentation levels. The stream must clearly distinguish model reasoning summaries, runtime actions, observations, policy decisions, recovery, and evidence.

Streaming must not imply that a model has authority. A visible `DECISION` event means that a strategy was selected; it does not mean that a tool, mutation, permission, or artifact promotion was authorized. The runtime must emit a separate policy and execution event for those actions.

### 49.4 Privacy and safety filters

Before a reasoning summary reaches the UI or durable history, `ReasoningStreamFilter` MUST remove or mask API keys, access tokens, private keys, passwords, cookies, personally identifying data, complete source-file contents, sensitive user data, hidden system instructions, raw provider messages, and private internal reasoning.

The stream must not reveal unrestricted shell commands when the command contains secrets or sensitive paths. It may show a safe command category, redacted arguments, operation ID, and result. Detailed diagnostics remain available only through policy-controlled Developer mode and still undergo redaction.

### 49.5 Honest status semantics

The stream MUST distinguish:

| Status | Meaning |
|---|---|
| Working | A supervised operation is active and making measurable progress |
| Waiting | Progress is intentionally paused for a provider, device, resource, or policy condition |
| Recovering | The supervisor is applying or evaluating a repair strategy |
| Blocked | A deterministic policy or required input prevents continuation |
| Stale | The displayed information no longer matches the current project revision |
| Complete | Completion gates passed with evidence |
| Safely failed | No safe approved strategy remains; checkpoint and recovery options are available |

An unchanged stream, repeated message, or active spinner is not sufficient evidence of progress. Stall detection must operate independently of the stream.

### 49.6 User controls

The user may hide or show the stream, change its detail level, filter event categories, inspect evidence, pause new autonomous work, cancel the session, request a summary, or open the relevant checkpoint. Hiding the stream does not stop execution or delete history.

The user cannot edit a stream event, mark an unsupported event as evidence, approve a mutation by editing text, or use the stream to bypass policy. Any approval remains a separate explicit runtime action.

### 49.7 Acceptance criteria

1. A long-running session streams understanding, plan, action, observation, recovery, evidence, and next-step events in order.
2. Stream reconnection resumes from the last acknowledged sequence without duplicate or missing events.
3. Private reasoning never appears verbatim in the UI, event store, logs, exports, or provider handoffs.
4. Secrets, sensitive paths, source contents, and raw provider messages are redacted before display and persistence.
5. Visible decisions are linked to runtime policy, execution, and evidence events.
6. Waiting, blocked, stale, complete, and safely-failed conditions are visually distinct from working.
7. Stream presentation can be changed without changing execution behavior.
8. Replay reconstructs the visible stream from durable filtered events without re-running model reasoning.


---

## 50. Mandatory Brand and Asset Completion Gate

### 50.1 Product requirement

Branding and visual assets are first-class Android product requirements. When the user requests a logo, icon, splash screen, notification icon, illustration, branded color system, or visual identity, Nirman MUST generate or safely derive the requested assets, integrate them into the Android project, show them in the live preview, and validate them before the APK can be promoted.

The implementation must not finish at source-code generation while leaving the application with missing, generic, stale, or unintegrated branding.

### 50.2 BrandAssetPipeline

```text
User instruction and screenshots
        ↓
Brand intent extraction
        ↓
BrandManifest creation
        ↓
Asset generation or approved vector fallback
        ↓
Android resource adaptation
        ↓
Project integration
        ↓
Asset validation
        ↓
Live preview verification
        ↓
APK asset inspection
        ↓
BrandAssetCompletionGate
```

The pipeline covers the application label, adaptive launcher icon, legacy launcher variants where required, monochrome icon where supported, splash screen, notification icon, in-app logo, color system, theme tokens, typography intent, empty-state art, onboarding illustrations, and other assets explicitly requested by the user.

### 50.3 BrandManifest and AssetManifest

`BrandManifest` records display name, semantic brand description, logo/icon/splash intent, source screenshot references, light and dark colors, typography and spacing intent, theme behavior, asset requirements, accessibility expectations, and manifest version.

`AssetManifest` records each asset’s ID, type, BrandManifest version, source intent, screenshot references, output path, format, dimensions, density or adaptive variant, content hash, provider/model metadata, generation status, integration status, validation status, and regeneration history.

Provider/model metadata and prompt hashes are retained for provenance, but raw prompts, private data, and secrets are not exposed in the user-facing stream or ordinary logs. A seed may be recorded when available, but exact reproducibility is verified from output hashes rather than assumed.

### 50.4 Asset completion rules

The final artifact MUST NOT be marked complete when a requested asset is missing, references an invalid path, is not packaged, is stale relative to the source revision, fails format/dimension/transparency/contrast checks, or has not been verified in the active preview. A temporary placeholder may be used during recovery, but it cannot silently satisfy the final gate when branded assets were requested.

The gate must inspect the built APK, not only the workspace. It must confirm that launcher resources, splash resources, notification assets, in-app assets, theme resources, and referenced fonts or illustrations are present and reachable in the final artifact.

### 50.5 Asset change behavior

When the user requests a branding change, Nirman creates a new BrandManifest revision, regenerates only affected assets, updates Android resources, refreshes the preview, invalidates stale asset evidence, and reruns the asset gate. Unaffected source code and assets should remain unchanged where impact analysis proves they are independent.

### 50.6 Visible asset progress

The reasoning stream should show safe events such as:

```text
Understanding: “You requested a fitness brand named FitPulse.”
Brand decision: “Using an energetic green palette with a heart-and-lightning symbol.”
Asset action: “Generating adaptive launcher icon variants.”
Asset action: “Integrating the icon into Android resources.”
Validation: “Launcher icon and splash screen verified on the API 35 emulator.”
Next step: “Running final APK asset inspection.”
```

### 50.7 Acceptance criteria

1. A user request for branded assets creates a versioned BrandManifest and AssetManifest.
2. Requested launcher, adaptive, monochrome, splash, notification, in-app, and theme assets are generated or explicitly governed by a fallback record.
3. All assets are integrated into the correct Android resource locations and referenced by the project.
4. The active PreviewRevision displays the current asset revision.
5. The built APK is inspected for asset presence, reachability, and content hashes.
6. Missing, stale, invalid, unintegrated, or placeholder-only requested assets block final completion.
7. Branding changes regenerate only affected assets and invalidate stale evidence.
8. Asset generation, integration, validation, fallback, and release results are visible in the structured reasoning stream and retained in replayable evidence.


---

## 51. Locked Nirman Implementation Stack and Executable Architecture

### 51.1 Stack decision

The following stack is the implementation baseline for Nirman v1. It does not change the Android-only generated target.

| Layer | Locked implementation |
|---|---|
| Windows desktop shell | C#/.NET + WinUI 3 |
| Frontend | C# + XAML (WinUI 3) |
| Styling | WinUI 3 Fluent Design System |
| Presentation state | WinUI 3 MVVM or equivalent presentation-only state layer |
| Core runtime | Rust with Tokio |
| Control plane | Rust authoritative supervisor and runtime services |
| Local database | SQLite with versioned migrations |
| Initial database access | SQLx preferred; rusqlite remains an evaluated alternative if isolated safely |
| UI IPC | Authenticated SupervisorConnection over named pipes |
| Durable event stream | Supervisor-owned durable event log with cursor-based replay; UI transport is an authenticated projection channel |
| Editor | Native WinUI editor surface (AvalonEdit or equivalent) |
| Terminal renderer | Native WinUI terminal surface |
| Windows terminal runtime | Native ConPTY supervised by Rust |
| Worker execution | Rust-supervised child processes with leases and scoped capabilities |
| Windows isolation | Restricted tokens, Job Objects, ACL workspaces, environment filtering, process supervision, quotas |
| Credentials | Windows Credential Manager and DPAPI-backed secure storage |
| Version control | Git and Git worktrees |
| Android toolchain | JDK, Gradle, AGP, Android SDK, ADB, emulator, NDK/CMake when required |
| JavaScript Android toolchain | Node and npm/pnpm/yarn, Metro, Expo/React Native only when selected |
| Packaging | MSIX installer, with optional MSI packaging |

Nirman orchestrates the Android ecosystem; it does not replace JDK, Gradle, AGP, Android SDK, ADB, emulator, Node, Metro, Expo, native compilers, or Git.

### 51.2 Two-executable production architecture

The first vertical slice may embed the control plane in the WinUI 3 Rust backend to reduce initial process complexity. The production durable-autonomy architecture separates presentation from the long-running supervisor:

```text
Nirman.exe
└── C#/.NET + WinUI 3
    ├── Chat
    ├── Project navigation
    ├── Code/editor surface
    ├── Preview presentation
    ├── Task graph and reasoning stream
    ├── Settings and user controls
    └── SupervisorConnection client
              │ authenticated Supervisor protocol (named pipes)
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

`Nirman.exe` is a reconnectable client. It must not own authoritative task state, credentials, lifecycle, worker leases, filesystem authority, process supervision, recovery, evidence, or artifact promotion. `NirmanSupervisor.exe` starts with Windows user login when eligible work exists, survives UI closure, scans SQLite after reboot or sleep/resume, and allows the UI to reconnect later.

### 51.3 User-visible implementation contract

Nirman should feel like one application even when the supervisor is a separate executable. The UI must show supervisor health, connection state, session state, reasoning stream, task progress, terminal summaries, preview revision, evidence, and recovery status. Supervisor installation, update, version handshake, and graceful shutdown are runtime concerns and must not require users to manually operate a second application.

### 51.4 First-release editor and terminal boundaries

A native WinUI editor surface is the first editor implementation because Nirman’s primary product is autonomous construction, preview, validation, recovery, and artifact delivery rather than a full standalone IDE. AvalonEdit or an equivalent native editor surface may be evaluated later without changing the control-plane architecture.

A native WinUI terminal surface is the terminal renderer. Rust owns ConPTY sessions, shell profiles, process trees, input policy, output capture, cancellation, resource limits, and recovery. Supported shells may include PowerShell, `cmd.exe`, Git Bash, or another explicitly approved profile.

### 51.5 Completion invariants

The stack is considered correctly implemented only when the UI can restart without losing a session, the supervisor can continue without the UI, Android toolchains execute through supervised local processes, model proposals pass through ModelGateway, ToolBroker, and PolicyAuthority, and APK promotion remains evidence-backed. No framework selector, web target, Windows generated target, or cloud execution environment is introduced.


---

## 52. Core Agent Execution Kernel and Autonomous Loop Contract

**ContractId:** `CONTRACT.RUNTIME.AUTHORITY`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.AUTHORITY
- authoritySection: §33
- extendingSection: §52
- extensionType: adds_component
- extendedClauses: none
- nonOverriddenClauses: CLAUSE.AUTHORITY.MODEL_PROPOSES, CLAUSE.AUTHORITY.NO_SELF_ELEVATION

**ContractId:** `CONTRACT.RUNTIME.SKILL`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.SKILL
- authoritySection: §23
- extendingSection: §52
- extensionType: adds_component
- extendedClauses: none
- nonOverriddenClauses: CLAUSE.SKILL.NO_PERMISSION_GRANT, CLAUSE.SKILL.SESSION_PINNING

**ContractId:** `CONTRACT.RUNTIME.PLATFORM_CAPABILITY`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.PLATFORM_CAPABILITY
- authoritySection: §79
- extendingSection: §52
- extensionType: adds_component
- extendedClauses: none
- nonOverriddenClauses: CLAUSE.PLATFORM.HOST_TARGET_SEPARATION, CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE, CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION, CLAUSE.PLATFORM.EVIDENCE_ENV_BINDING, CLAUSE.PLATFORM.VALIDATION_ENV_RESERVATION, CLAUSE.PLATFORM.NO_SUBSTITUTE_TARGET

### 52.1 Purpose

Nirman must expose a first-class **AgentExecutionKernel** between the goal/task graph and worker, skill, and tool execution. Existing worker lifecycle states describe whether a process is created, active, waiting, or stopped; the kernel describes how autonomous reasoning and verified execution progress from an observation to the next evidence-backed state.

The kernel must be deterministic at the authority boundary. Models may propose interpretations, plans, actions, delegations, repairs, and validation strategies. The kernel, reducer, policy authority, transaction manager, process supervisor, and evidence authority decide what may actually happen.

The mandatory control invariant is:

```text
Model output
    ↓
Structured proposal
    ↓
Schema validation
    ↓
Revision and scope validation
    ↓
Policy and capability authorization
    ↓
Construction transaction or supervised tool session
    ↓
Observation and evidence
    ↓
State transition
```

Nirman must never implement a direct `model → execute` path.

### 52.2 Agent loop states

The kernel must maintain a separate reasoning/execution state machine from the worker-process lifecycle state machine:

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

Every transition must include the session, task, agent instance, project revision, plan revision, input evidence, output reference, policy decision, and next permitted transition. Impossible transitions must be rejected and recorded as runtime faults.

### 52.3 Progress evaluation

After every meaningful observation, the kernel must determine whether the current goal is progressing, blocked, contradicted, unsafe, stale, or satisfied. Progress evaluation must consider requirement coverage, changed files, test results, preview revision, environment capability state, worker handoffs, unresolved uncertainty, failure fingerprints, resource pressure, and artifact readiness.

Completion is permitted only when the appropriate requirement, test, preview, device, quality, branding, and APK evidence gates pass. A model statement that a task is complete is never sufficient evidence.

### 52.4 SkillRuntime and skill composition

The existing skill registry describes packages and permissions. Nirman must also provide a `SkillRuntime` that performs:

```text
DISCOVER
  ↓
SELECT
  ↓
CHECK_COMPATIBILITY
  ↓
BIND_INPUT
  ↓
ASSEMBLE_CONTEXT
  ↓
EXECUTE
  ↓
MEDIATE_TOOLS
  ↓
VALIDATE_OUTPUT
  ↓
CAPTURE_EVIDENCE
  ↓
RETURN_RESULT
```

A skill lifecycle must include `DISCOVERED`, `INSTALLED`, `SCANNED`, `TRUSTED`, `AVAILABLE`, `SELECTED`, `BOUND`, `RUNNING`, `WAITING_TOOL`, `WAITING_APPROVAL`, `VALIDATING`, `COMPLETED`, `FAILED`, and `ROLLED_BACK`.

Skills may compose into bounded Android workflows. For example, Android UI implementation may compose with accessibility review, visual regression, Android build diagnostics, and release validation. Composition must check skill dependencies, version compatibility, required worker roles, required tools, required Android profiles, input/output schemas, resource limits, and permissions.

Loading or composing a skill never grants a permission. The PolicyAuthority must authorize every capability independently.

Every invocation must produce a `SkillExecutionRecord` containing the skill version, worker, task, input hash, context references, tools used, permissions used, files changed, evidence IDs, duration, model usage, result status, and rollback reference.

### 52.5 Agent profiles and dynamic worker instances

A worker role defines responsibility. An `AgentProfile` defines how a particular instance operates:

```text
AgentProfile
├── model profile
├── reasoning mode
├── context strategy
├── skill set
├── tool set
├── permission profile
├── autonomy level
├── generation parameters
├── maximum child count
├── resource policy
├── recovery policy
├── validation policy
└── memory policy
```

A worker instance must be constructed from a role, task contract, profile, skills, model, tools, workspace lease, permission profile, resource profile, context profile, parent task, and recovery policy. Dynamic creation must remain bounded and must not expand permissions or workspace scope.

### 52.6 SwarmPlanner and DelegationProtocol

Nirman must add a `SwarmPlanner` that decides whether a goal can be parallelized. It must analyze requirements, dependency graph, change surface, file and symbol boundaries, validation cost, workspace availability, tool capability requirements, risk, and resource capacity before selecting workers.

The planner must optimize for correct integration and evidence, not maximum worker count:

```text
Goal
  ↓
Dependency analysis
  ↓
Change-surface analysis
  ↓
Work decomposition
  ↓
Parallelism analysis
  ↓
Worker/profile selection
  ↓
Interface agreement
  ↓
Workspace allocation
  ↓
Swarm execution
  ↓
Reconciliation and validation
```

The typed delegation protocol must support `delegate`, `spawn`, `handoff`, `resume`, `cancel`, `replace`, `retry`, `escalate`, and `merge`. A delegation request must include the required capability, proposed role/profile, task scope, input references, expected outputs, validation requirements, parent task, workspace lease, and cancellation lineage.

### 52.7 KnowledgeLedger and TaskBlackboard

Workers must communicate through typed, scoped knowledge rather than a shared mutable prompt or unbounded common memory. Nirman must maintain a `KnowledgeLedger` and a task-scoped `TaskBlackboard` containing goals, requirements, architecture facts, decisions, constraints, assumptions, active workers, completed work, blocked work, findings, conflicts, evidence, known failures, and next actions.

A `KnowledgeArtifact` may be a finding, decision, constraint, assumption, architecture fact, failure pattern, test result, artifact, or environment fact. It must include the source worker, source task, project revision, confidence, evidence IDs, validity period, and scope.

Workers may read relevant entries, propose artifacts, attach evidence, request changes, and retrieve facts. Only deterministic authorities may commit decisions, mutate the task graph, mark requirements complete, change policy, or promote artifacts.

### 52.8 Workspace leases and stateful ToolSessions

Every isolated worktree, copy-on-write workspace, terminal, ADB session, emulator, debugger, LSP, preview process, and other long-lived execution resource must be represented by an ownership and lifecycle record.

A `WorkspaceLease` must include workspace ID, owner worker, task ID, parent checkpoint, lease state, acquisition time, heartbeat, expiration, cleanup policy, recovery policy, current revision, and stale-owner handling. Lease recovery must prevent orphan worktrees, duplicate ownership, zombie builds, and stale writes.

A `ToolSession` must include session ID, tool type, owner, task and project scope, environment fingerprint, process group, current state, capability scope, input policy, output reference, heartbeat, reconnect policy, cleanup policy, and evidence references. Sessions must support reconnect after worker replacement or UI restart without granting a new scope.

### 52.9 Tool Capability Graph and environment capability planning

Nirman must map goals to required capabilities, then capabilities to skills, workers, tools, and environment prerequisites. For example, an Android BLE application may require BLE APIs, a compatible Android SDK, native modules, Bluetooth permissions, ADB, a Nirman-managed local Android emulator capability, and emulator validation.

Each required environment capability must be classified as `AVAILABLE`, `REPAIRABLE`, `USER_REQUIRED`, or `UNAVAILABLE` before the task commits to a validation path. The planner must surface the distinction early instead of discovering an impossible prerequisite after a long build.

Host platform, target platform, and validation platform are distinct, explicitly recorded fields of the environment record (§79.1 and §79.2). The planner classifies cross-compilation capability and native target-runtime capability as separate prerequisites and must never derive one from the other: a successful cross-build is an artifact-production result, not a runtime-validation result (§79.5 and §79.6).

### 52.10 ValidationPlanner and mutation/regression intelligence

The `ValidationPlanner` must choose checks from changed files, changed symbols, call graph, route graph, dependency graph, requirements, acceptance criteria, project type, risk level, previous failures, emulator profiles, and available resources.

A change to an Android screen, repository, permission, navigation route, data model, manifest, native module, or build file must expand validation to the affected behavior. The planner may select focused checks for low-risk changes and automatically expand to instrumentation, accessibility, security, visual, device, performance, regression, and release checks for high-risk changes.

The planner must emit a traceability chain:

```text
Requirement
  ↓
Acceptance criterion
  ↓
Environment requirement
  ↓
Capability resolution
  ↓
Task graph node
  ↓
Worker contract
  ↓
Skill execution
  ↓
Code change
  ↓
Validation run
  ↓
Evidence
  ↓
APK artifact
```

The environment requirement and capability resolution edges are populated by the platform capability contract of §79 before the task graph is compiled: the declared target platform, the observed host platform, and the classified capability set (`AVAILABLE`, `REPAIRABLE`, `USER_REQUIRED`, `UNAVAILABLE`) are part of the chain, and a plan that commits to a validation path whose capability is not `AVAILABLE` or `REPAIRABLE` without a durable `USER_REQUIRED`/`UNAVAILABLE` node is a planning defect.

### 52.11 Trajectory Replay and Simulation mode

Nirman must provide a side-effect-free `TrajectoryReplayEngine` that can replay a recorded goal, context references, structured model proposals, tool calls, tool results, state changes, observations, and next decisions against a new model, prompt, skill, tool schema, or runtime without touching the real project.

Nirman must also provide a clearly labeled **Simulation/Dry-Run Mode**. It may predict workers, skills, files, commands, permissions, devices, tests, resources, risks, and expected validation, but it must not mutate files, execute commands, start devices, or claim that predicted checks actually ran. Simulation output must be labeled `PREDICTED`, while executed evidence must be labeled `OBSERVED` or `VERIFIED`.

### 52.12 Deadlock, backpressure, cancellation, and pause/resume

The runtime must detect dependency cycles across tasks, workers, resource reservations, approvals, workspace leases, and ToolSessions. A detected deadlock must produce a typed finding and trigger safe recovery, reordering, worker replacement, or a structured decision node.

Swarm execution must apply backpressure when workers compete for Gradle, emulator slots, GPU capacity, emulator slots, storage, or provider concurrency. Reservations, priority, fairness, queues, and resource release must be visible in the task graph.

Cancellation must propagate from goal to task graph, workers, skills, ToolSessions, processes, PTY sessions, emulator operations, and pending provider requests. Each layer must support graceful cancellation, forced termination, cleanup, checkpoint preservation, and rollback semantics.

Workers and skills must support independent pause and resume. Pausing must preserve context references, ToolSessions, leases, checkpoints, and unresolved questions while allowing unrelated work to continue.

### 52.13 Decision nodes, uncertainty, contradiction, and plan recompilation

When multiple valid Android architectures or recovery strategies exist, Nirman must represent a `DecisionNode` containing the question, options, evidence, trade-offs, recommendation, impact, and resume conditions. It is distinct from a generic command approval.

The runtime must track uncertainty as first-class state: `KNOWN`, `PROBABLE`, `ASSUMED`, `UNKNOWN`, `CONTRADICTED`, `VERIFIED`, and `BLOCKED`. Each uncertainty record must identify its scope, source, evidence, confidence, expiration, and next resolution action.

A contradiction detector must identify conflicting requirements, stale assumptions, invalidated decisions, changed device constraints, and architecture drift. It must create a controlled decision revision rather than silently selecting whichever statement appeared most recently.

The `PlanCompiler` and `Replanner` must produce plan revisions when evidence, environment, requirements, toolchain, worker availability, or validation results invalidate the current plan. Each plan revision must record `planRevision`, `supersedesPlan`, reason, trigger evidence, affected nodes, and migration/recovery action.

### 52.14 Execution history tiers

Long-running Android sessions must not retain every event, terminal output, screenshot, failed strategy, intermediate plan, and checkpoint in active memory. The `ExecutionHistoryManager` must provide:

| Tier | Contents | Retrieval behavior |
|---|---|---|
| Hot | Current graph, active workers, current plan, latest evidence, unresolved blockers | Always available to the kernel |
| Warm | Recent events, recent terminal summaries, recent checkpoints, recent preview and test results | Loaded on task or worker request |
| Cold | Older events, completed handoffs, historical failures, superseded plans, old screenshots | Retrieved by indexed query or replay request |
| Archived | Content-addressed logs, full traces, old artifacts, crash dumps, retired sessions | Restored explicitly for audit or investigation |

Compaction must preserve semantic summaries, evidence links, revision identity, and replay references. Garbage collection must never delete required completion evidence, active checkpoint parents, unresolved failure evidence, or artifact provenance.

### 52.15 Product acceptance invariants

The AgentExecutionKernel release is complete only when Nirman can run one Android goal through the loop state machine, execute a skill composition, dynamically configure a worker profile, delegate a typed task, exchange knowledge artifacts, lease a workspace, reconnect a ToolSession, plan environment capabilities, select affected validation, replay the trajectory without side effects, simulate the plan without mutation, detect a deadlock, apply backpressure, propagate cancellation, pause and resume a worker, surface a decision node, track uncertainty, recompile a plan, compact execution history, and deliver an evidence-backed APK.

The user-facing stream must show concise structured events for these transitions without exposing private chain-of-thought. The deterministic runtime remains the only authority over mutation, tools, permissions, lifecycle, evidence, recovery, and artifact promotion.

## 53. Agent Memory and Context Architecture

**ContractId:** `CONTRACT.RUNTIME.CONTEXT`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.CONTEXT` (see §67.8)

**ContractId:** `CONTRACT.RUNTIME.MEMORY`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.MEMORY
- authoritySection: §38
- extendingSection: §53
- extensionType: adds_clauses
- extendedClauses: CLAUSE.CONTEXT.CONSTRAINT_PRIORITY, CLAUSE.CONTEXT.SOURCE_REQUIRED
- nonOverriddenClauses: CLAUSE.MEMORY.SCOPES, CLAUSE.MEMORY.RETENTION_AUTHORITY, CLAUSE.MEMORY.SECRET_EXCLUSION


This section extends §38 (Privacy-Scoped Memory, Replay, and Recovery History). §38 remains the authority on memory scopes, retention, and deletion. This section adds the retrieval and context-assembly contract that §38 does not specify, and does not redefine memory scopes.

### 53.1 Product requirement

A long-horizon Android session must not degrade because earlier decisions fell out of the working context. The runtime must be able to reconstruct why a decision was made after thousands of intervening actions, and must not re-derive a decision that was already locked.

### 53.2 Memory record classes

Every memory write must be classified as exactly one of:

| Class | Contents | Written from |
|---|---|---|
| DECISION | A locked choice and its evidence | Approved decision nodes and validated outcomes |
| CONSTRAINT | A rule the runtime must not violate later | User instruction, policy, or contract |
| FACT | An observed property of the project or environment | Validated tool output |
| FAILURE | A failed approach and its symptom signature | Evidence-backed failure records |
| ARTIFACT | A produced file, build, or report reference | Artifact provenance records |

A model statement is never a memory write. Only validated events, approved decisions, and user confirmations produce memory records, consistent with §37 (Event, Evidence, and Completion Authority).

### 53.3 Context assembly contract

Before any model call, the runtime must assemble a context package that declares:

```text
ContextPackage
- taskId
- assembledAt
- mode: retrieval | large_context
- includedPaths
- excludedPaths
- includedMemoryRecords
- activeConstraints
- lockedDecisions
- tokenEstimate
- tokenBudget
- redactions
- selectionReason
- omittedForBudget
```

Active constraints and locked decisions must never be dropped for budget reasons. If they cannot fit, the runtime must reduce file content, not constraint content, and must record the reduction in `omittedForBudget`.

### 53.4 Re-grounding requirement

At every long-horizon checkpoint the runtime must re-ground the working context by re-reading the original goal, the active constraints, the locked decisions, and the current evidence state. Re-grounding must be an explicit recorded step, not an implicit prompt behavior, and must occur before plan recompilation.

### 53.5 Cross-project isolation

Project memory must never be read across project boundaries. Runtime-improvement memory may cross projects only in anonymized form with no file paths, identifiers, source content, or credentials.

### 53.6 Acceptance criteria

The memory and context contract is satisfied only when a session can be interrupted, resumed after a runtime restart, and continue without re-asking a settled question; when a locked decision is never contradicted by a later action; when every memory record cites its source evidence; and when a context package can be reproduced from the event ledger for any historical model call.

## 54. Swarm Coordination and Concurrent Change Management

**ContractId:** `CONTRACT.RUNTIME.RESERVATION`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.RESERVATION` (see §67.8)

**ContractId:** `CONTRACT.RUNTIME.WORKSPACE`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.WORKSPACE
- authoritySection: §22
- extendingSection: §54
- extensionType: adds_clauses
- extendedClauses: CLAUSE.RESERVATION.GRANT_AUTHORITY, CLAUSE.RESERVATION.STALE_INVALIDATION
- nonOverriddenClauses: CLAUSE.WORKSPACE.SINGLE_WRITER


This section extends §22 and §23 (Advanced Autonomous Development and Swarm Execution) and §52 (Core Agent Execution Kernel). Those sections remain the authority on worker lifecycle, delegation, and workspace leases. This section adds semantic conflict prevention, which lease ownership alone does not provide.

### 54.1 The gap addressed

Workspace leases prevent two workers from writing the same file. They do not prevent two workers from making semantically incompatible changes in different files — for example one worker renaming a data model field while another writes code against the old field name.

### 54.2 Semantic reservations

A worker must reserve the semantic surfaces it intends to change before mutating them:

```text
SemanticReservation
- reservationId
- workerId
- taskId
- surfaceKind: symbol | route | schema_table | resource_id | permission | dependency | build_config
- surfaceIdentifier
- intent: read_stable | modify | delete | create
- grantedAt
- expiresAt
- renewedAt
- state: granted | renewed | released | expired | revoked
```

A `modify` or `delete` reservation conflicts with any other reservation on the same surface. A `read_stable` reservation means the holder has generated code that depends on the surface remaining unchanged.

### 54.3 Stale-contract invalidation

When a surface changes, every `read_stable` reservation on that surface must be invalidated. Each holder must be notified, its affected work marked unvalidated, and its output revalidated before it may be promoted. Silent acceptance of work built against an invalidated contract is prohibited.

### 54.4 Peer coordination without peer authority

Workers may exchange knowledge artifacts, publish reservations, and request handoffs. Workers must never grant permissions to each other, approve each other's evidence, mark each other's work complete, or override an authority decision. All authority remains with the deterministic runtime, consistent with §33 and §52.

### 54.5 Serialized commit barrier

Parallel proposals must be reconciled through a serialized commit barrier. At the barrier the runtime must verify that every reservation held by the proposing worker is still valid, that no dependent surface changed since generation, and that validation evidence postdates the last relevant surface change. A proposal failing any check is rejected and returned for revalidation, not merged.

### 54.6 Acceptance criteria

Coordination is satisfied only when a fixture with two workers changing interdependent Android code produces either a correct merged result or an explicit rejection with a stale-contract reason, and never a build that compiled per-worker but fails after merge.

## 55. User/Edit Reconciliation

**ContractId:** `CONTRACT.RUNTIME.RECONCILIATION`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.RECONCILIATION` (see §67.8)


No existing section covers concurrent human editing. This section is new and does not overlap §22, §23, or §52, which address worker-to-worker coordination only.

### 55.1 Product requirement

The user may edit project files in Nirman's editor or in an external tool while an autonomous run is active. The runtime must never silently overwrite a user edit, and must never treat a user edit as its own output.

### 55.2 External change detection

The runtime must watch the project tree and classify every observed change as:

| Origin | Meaning | Required behavior |
|---|---|---|
| RUNTIME | Written by an authorized mutation | Continue |
| USER | Written by the user | Reconcile before further mutation of that surface |
| EXTERNAL | Written by another process or tool | Reconcile and record provenance as unknown |
| GENERATED | Produced by a build or toolchain step | Ignore for reconciliation, exclude from context |

Origin classification must come from mutation records and file fingerprints, not from timestamps alone.

### 55.3 Reconciliation behavior

On a USER or EXTERNAL change to a surface the runtime holds a reservation on, the runtime must pause mutation of that surface, re-read the changed file, invalidate affected validation evidence, re-derive whether the active plan is still correct, and either continue with the user's version as the new baseline or surface a decision node when the change contradicts a locked decision.

### 55.4 Prohibited behaviors

The runtime must not revert a user edit to reapply its own version, must not include a user edit in its own completion evidence without revalidation, and must not report a requirement complete on the basis of validation that predates a user edit to the same surface.

### 55.5 Acceptance criteria

Reconciliation is satisfied only when a fixture in which the user edits a Kotlin file mid-run results in the user's content preserved, affected validation re-run, and the final report attributing the change to the user rather than to the runtime.

## 56. Stateful End-to-End Verification

**ContractId:** `CONTRACT.RUNTIME.E2E`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.E2E` (see §67.8)

**ContractId:** `CONTRACT.RUNTIME.EVIDENCE`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.EVIDENCE
- authoritySection: §37
- extendingSection: §56
- extensionType: adds_clauses
- extendedClauses: CLAUSE.E2E.DETERMINISM, CLAUSE.E2E.SEED_PROVENANCE
- nonOverriddenClauses: CLAUSE.EVIDENCE.CLAIM_SEPARATION, CLAUSE.EVIDENCE.FRESHNESS


This section extends §29 (End-to-End Android Generation Contract) and §36 (Complete Android Capability Fixture references). Those sections require end-to-end validation; this section specifies stateful scenarios, which single-screen validation cannot cover.

### 56.1 The gap addressed

Launching an app and screenshotting the first screen does not prove the app works. A real Android application has authenticated states, persisted data, navigation depth, process death, and configuration changes. Verification must exercise those states.

### 56.2 Scenario contract

```text
E2EScenario
- scenarioId
- requirementIds
- preconditions
- seedData
- steps: ordered UI, system, and user-like interaction actions
- assertions
- interactionMethod
- observedState
- expectedPersistedState
- teardown
- devices
- deterministic: true | false
```

`steps` MUST execute against the installed/launchable Android application on the declared device. An interaction step is an action performed against the running application, not a source-code inspection or model prediction.

`interactionMethod` MUST identify the mechanism used to drive the application, such as Espresso, Compose UI Test, an out-of-process UI-tree driver, accessibility/input injection, or another approved Android device adapter.

`observedState` MUST identify the runtime state observed after the action. An action without an observed postcondition is not behavioral evidence.

A scenario must be deterministic. Non-deterministic scenarios must be marked and must not be used as completion evidence.

### 56.3 Required scenario classes

| Class | Must verify |
|---|---|
| Cold start | First launch with no data behaves correctly |
| Authenticated flow | Login state reached and persisted across restart |
| Data persistence | Written data survives process death |
| Navigation depth | Deep navigation and back-stack correctness |
| Configuration change | Rotation and theme change preserve state |
| Permission flow | Grant and deny paths both handled |
| Offline behavior | Network-absent path produces defined behavior |
| Process death | System-initiated death and restore |

### 56.4 Data seeding

Seed data must be created through the application's own data layer or an explicit test fixture, never by asserting state the app never produced. Seed provenance must be recorded so evidence cannot be confused with production behavior.

### 56.5 Evidence requirements

Each scenario run MUST produce:
- step execution results;
- action identity and interaction method;
- observed pre-state and post-state where applicable;
- assertion results;
- screenshots at asserted steps;
- UI-hierarchy or equivalent runtime-state evidence where supported;
- Logcat for the run window;
- persisted-state verification where applicable;
- emulator identity;
- installed artifact identity;
- source revision;
- application-state fingerprint.

A scenario without executable interaction results and assertion results is not behavioral evidence.

### 56.6 Acceptance criteria

Stateful verification is satisfied only when every functional requirement maps to at least one deterministic scenario, and when a requirement cannot be marked complete while its scenario is missing, skipped, or non-deterministic.

A model claim, source inspection, successful compilation, static screenshot, or predicted UI state MUST NOT satisfy a behavioral acceptance condition when that condition is executable on the Android runtime.

### 56.7 Per-stack UI validation framework binding

Each technology composition the resolver may select MUST declare its UI validation binding: Android Views → Espresso; Jetpack Compose → Compose UI Test; Expo/React Native → an out-of-process UI-tree driver. The binding is part of `AndroidTechnologyPlan` and MUST be resolved when the technology is resolved, not at test time. A composition with no declared binding MUST report `CAP.ANDROID.E2E_VERIFY` as `UNAVAILABLE` for that composition rather than silently downgrading to screenshot comparison. Test execution MUST route through `AndroidDeviceAdapter` per CLAUSE.PREVIEW_SYNC.ADAPTER_BOUND; the technology adapter MUST NOT execute it. Results enter the evidence chain as an `Observation`; a passing test is evidence, never an independent completion decision. A test MUST NOT be weakened, skipped, or deleted to reach a passing state.

## 57. Advanced Verification Architecture

**ContractId:** `CONTRACT.RUNTIME.VERIFICATION`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.VERIFICATION` (see §67.8)

**ContractId:** `CONTRACT.RUNTIME.EVIDENCE`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.EVIDENCE
- authoritySection: §37
- extendingSection: §57
- extensionType: adds_clauses
- extendedClauses: CLAUSE.VERIFY.IN_LOOP, CLAUSE.VERIFY.ASSERTION_ORDER, CLAUSE.VERIFY.NON_VACUITY
- nonOverriddenClauses: CLAUSE.EVIDENCE.CLAIM_SEPARATION, CLAUSE.EVIDENCE.FRESHNESS


This section extends §47 (Integrated Android Workflow and Quality Intelligence) and §52.10 (ValidationPlanner). Those remain the authority on validation selection. This section adds verification methods that execution-based testing alone does not provide.

### 57.1 Static analysis inside the loop

Static analysis must run inside the generation loop, not as a terminal gate. After each structured mutation the runtime must run compiler diagnostics, lint, and null-safety and type checks on the affected surface before proceeding to the next mutation. A mutation that introduces a new diagnostic must be repaired or reverted before dependent work continues.

### 57.2 Incremental compilation gate

The runtime must compile incrementally at mutation granularity rather than only at task completion. A worker must not accumulate multiple unverified mutations when incremental compilation is available for the affected module.

### 57.3 Test-before-code

For any requirement with observable behavior, the runtime must define the assertion before generating the implementation. The assertion must be recorded, must fail before implementation, and must pass after. An assertion authored after a passing implementation must be marked as `post_hoc` and carries lower evidence weight.

### 57.4 Verification method matrix

| Method | Applies to | Evidence produced |
|---|---|---|
| Compiler diagnostics | Every mutation | Diagnostic set |
| Lint and static rules | Every mutation | Rule violations |
| Unit assertions | Logic and data transformation | Assertion results |
| Instrumentation scenarios | UI and stateful behavior | Scenario results per §56 |
| Screenshot comparison | Visual requirements | Image diff with threshold |
| Mutation probing | Critical logic | Whether assertions detect injected faults |
| Property probing | Input-domain logic | Counterexamples or pass |
| Performance measurement | Non-functional requirements | Measured metrics |

### 57.5 Assertion quality requirement

Assertions that cannot fail are not evidence. For critical logic the runtime must confirm that the assertion set detects at least one injected fault. An assertion set that passes against a deliberately broken implementation must be rejected as vacuous.

### 57.6 Acceptance criteria

Advanced verification is satisfied only when no mutation advances with an unresolved new diagnostic, when behavioral requirements have assertions authored before implementation, and when critical-logic assertion sets are proven non-vacuous.

## 58. Adversarial Security and Supply-Chain Verification

**ContractId:** `CONTRACT.RUNTIME.SUPPLY_CHAIN`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.SUPPLY_CHAIN` (see §67.8)


This section extends §11 (Security and Trust Model) and §36 (Execution Isolation and Sandbox Boundaries). Those remain the authority on host isolation and permissions. This section adds verification of the generated application and its dependencies.

### 58.1 Two distinct security surfaces

| Surface | Threat | Authority |
|---|---|---|
| Host runtime | Nirman itself executing untrusted model output | §11, §36 |
| Generated application | The produced Android app being insecure | This section |
| Dependency supply chain | Malicious or vulnerable third-party packages | This section |

The existing sections protect the host. They do not verify that the generated app is secure. Both are required.

### 58.2 Generated-application security checks

Before packaging, the runtime must verify the generated application for hardcoded secrets and API keys, insecure network configuration including cleartext traffic, exported components without permission guards, insecure data storage of sensitive values, unsafe WebView configuration, unguarded intent handling, over-broad permission requests, debuggable release configuration, and missing certificate handling for pinned endpoints.

### 58.3 Dependency verification

Every declared dependency must be resolved to an exact version with an integrity hash. The runtime must reject a dependency that cannot be resolved reproducibly, that resolves to a different artifact than previously recorded, or whose name closely resembles a known package in a way consistent with substitution attacks.

### 58.4 Artifact provenance and SBOM

Every produced APK artifact, or AAB artifact when the active PackagingProfile requires `APK_AND_AAB`, must have a software bill of materials recording every dependency with version and integrity hash, the toolchain versions used, the source revision, the signing identity class, and the checksum of the produced artifact. An artifact without a complete SBOM must not be promoted as a deliverable.

### 58.5 Findings are blocking or declared

A security finding must either block packaging or be explicitly recorded as an accepted risk with a reason. A finding must never be silently dropped, and the final report must list all findings and their dispositions.

### 58.6 Acceptance criteria

Supply-chain verification is satisfied only when a fixture containing a deliberately hardcoded secret and an unpinned dependency is blocked before packaging, and when every promoted artifact has a reproducible SBOM.

## 59. Android Emulator Scenario Coordination

**ContractId:** `CONTRACT.RUNTIME.DEVICE_MATRIX`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.DEVICE_MATRIX` (see §67.8)

**ContractId:** `CONTRACT.RUNTIME.E2E`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.E2E
- authoritySection: §56
- extendingSection: §59
- extensionType: adds_verification
- extendedClauses: CLAUSE.DEVICE.PRIMARY_REQUIRED
- nonOverriddenClauses: CLAUSE.E2E.DETERMINISM, CLAUSE.E2E.SEED_PROVENANCE


This section extends §11 (Local Execution and Environment Management) and §51 device handling. Those remain the authority on toolchain and device health. This section adds scenario execution across a emulator profile matrix.

### 59.1 Product requirement

An Android application that works on one emulator is not verified. Behavior varies by API level, screen size, density, form factor, and vendor behavior. Verification must state which devices were covered and which were not.

### 59.2 Device matrix declaration

```text
DeviceMatrixEntry
- deviceId
- kind: emulator
- apiLevel
- formFactor: phone | tablet | foldable
- density
- screenSize
- abi
- availability: available | unavailable | user_required
- role: primary | secondary | optional
```

The primary device must be available for a run to proceed. Unavailable secondary devices produce a declared coverage gap, not a silent pass.

### 59.3 Scenario distribution

Scenarios from §56 must be distributed across the matrix. The runtime must record, per scenario and per device, whether the scenario ran, passed, failed, or was skipped with a reason. A pass on the primary device with skips elsewhere must be reported as partial coverage.

### 59.4 Divergence handling

When the same scenario passes on one device and fails on another, the runtime must treat the divergence as a defect, not as device noise, and must record the divergence with both emulator profiles before attempting repair.

### 59.5 Capability status integration

Multi-device coverage must be reported through the capability status vocabulary of §5.6. A capability verified only on the primary device is `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS`, not `SUPPORTED`.

### 59.6 Acceptance criteria

Multi-device coordination is satisfied only when the final report states per-device scenario outcomes, when device unavailability produces a declared gap rather than an implicit pass, and when a device-specific failure is recorded as a defect.

## 60. External Event Trigger Gateway

**ContractId:** `CONTRACT.RUNTIME.TRIGGER`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.TRIGGER` (see §67.8)


This section extends §28 (Product Requirements for Goal-Based and Persistent Autonomy) and its scheduling material. Scheduling covers time-based initiation; this section covers externally originated initiation.

### 60.1 Product requirement

A long-running project may need work initiated by an external event rather than by a user sitting at the application. Any such path must be authenticated, bounded, and auditable, and must never widen the permission surface.

### 60.2 Trigger contract

```text
ExternalTrigger
- triggerId
- source: schedule | filesystem | version_control | manual_api | external_webhook
- authenticationMethod
- projectScope
- allowedGoalKinds
- permissionCeiling
- rateLimit
- requiresApproval: true | false
- enabled
- lastFiredAt
```

### 60.3 Authority constraints

An external trigger may only request work. It may never grant permissions, raise the permission ceiling, approve a decision node, bypass a policy gate, or promote an artifact. A trigger whose requested goal exceeds its permission ceiling must be rejected and recorded.

### 60.4 Default posture

External network-originated triggers must be disabled by default. Enabling one must be an explicit user action recorded in the decision trace with the authentication method and permission ceiling stated.

### 60.5 Auditability

Every trigger firing must record the source, authentication result, requested goal, admission decision, and resulting task identifier. A trigger that fires without an audit record is a defect.

### 60.6 Acceptance criteria

The trigger gateway is satisfied only when a disabled trigger cannot start work, when an over-scoped trigger request is rejected with a recorded reason, and when every admitted trigger produces a task traceable to its originating event.

## 61. Runtime Directives and Live Operational Control

**ContractId:** `CONTRACT.RUNTIME.DIRECTIVE`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.DIRECTIVE` (see §67.8)


This section extends §27 and §28 (implementation-level and goal-based autonomy requirements) and §52.12 (cancellation and pause/resume). Those remain the authority on lifecycle transitions. This section adds mid-run steering without restart.

### 61.1 Product requirement

During a long-horizon run the user must be able to change direction without discarding validated work. Restarting a multi-hour Android build to correct one instruction is unacceptable.

### 61.2 Directive contract

```text
RuntimeDirective
- directiveId
- issuedAt
- issuedBy: user | policy
- kind: constrain | reprioritize | forbid | require | refocus | halt_surface
- target: goal | task | worker | surface | capability
- statement
- bindingScope: remainder_of_session | current_task | until_revoked
- acknowledgedAt
- effectOnPlan
```

### 61.3 Application semantics

A directive takes effect at the next kernel decision point, not mid-mutation. The runtime must acknowledge the directive, record it as an active constraint per §53.2, re-ground context per §53.4, and recompile the plan when the directive invalidates it.

### 61.4 Directive precedence

A user directive outranks a model plan and a learned preference. A user directive may not override a policy gate, a permission ceiling, an evidence requirement, or a safety boundary. A directive requesting prohibited behavior must be rejected with a stated reason.

### 61.5 Work preservation

Applying a directive must preserve validated work that the directive does not invalidate. The runtime must state which completed work remains valid, which becomes unvalidated, and which is abandoned.

### 61.6 Acceptance criteria

Runtime directives are satisfied only when a directive issued mid-run changes subsequent behavior without a restart, when it appears as an active constraint in later context packages, and when the run reports exactly which prior work survived.

## 62. Regression Localization

**ContractId:** `CONTRACT.RUNTIME.LOCALIZATION`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.LOCALIZATION` (see §67.8)


This section extends §52.10 (mutation and regression intelligence) and §47 (quality intelligence). Those predict which validation to run. This section specifies identifying the cause after a regression is observed.

### 62.1 Product requirement

When something that previously passed now fails, the runtime must identify the causing change rather than regenerate broadly. Broad regeneration destroys validated work and hides the defect.

### 62.2 Localization inputs

```text
RegressionCase
- caseId
- failingAssertionOrScenario
- lastKnownPassingRevision
- currentFailingRevision
- candidateChanges: ordered mutation records between the two revisions
- affectedSurfaces
- localizationMethod: impact_graph | history_correlation | bisect
- identifiedCause
- confidence
```

### 62.3 Localization order

The runtime must attempt localization in increasing cost order: first the impact graph to find mutations touching the failing surface, then historical correlation with known failure signatures, then revision bisection using the recorded checkpoint sequence. Bisection must reuse existing checkpoints rather than rebuilding from scratch when checkpoints are available.

### 62.4 Repair constraint

Repair must target the identified cause. When localization fails to identify a cause, the runtime must record an unlocalized regression and escalate rather than rewriting unrelated code. Rewriting code outside the identified cause surface is prohibited without a recorded reason.

### 62.5 Failure signature learning

Each localized regression must produce a failure signature linking the symptom, the cause class, and the successful repair, written to memory as a FAILURE record per §53.2 so later occurrences are localized faster.

### 62.6 Acceptance criteria

Regression localization is satisfied only when an injected regression in a fixture is localized to the causing mutation, when repair is confined to the identified cause, and when an unlocalized regression escalates instead of triggering broad regeneration.

## 63. Agent Runtime Debugger

**ContractId:** `CONTRACT.RUNTIME.DEBUGGER`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.DEBUGGER` (see §67.8)


This section extends §49 (Private Internal Reasoning and Visible Structured Reasoning Stream) and §52.11 (trajectory replay). Those cover user-facing streaming and replay. This section adds operator-grade inspection of a live run.

### 63.1 Product requirement

When an autonomous run behaves wrongly, the user must be able to inspect why without reading private chain-of-thought and without stopping the run.

### 63.2 Inspectable state

The debugger must expose, for any live or completed run: the kernel state machine position, the active plan and its revision, the active constraints and locked decisions, the current context package manifest, the pending and completed tool calls with results, held reservations and leases, the evidence ledger for the current task, the recovery ladder position, and the resource reservations in effect.

### 63.3 Privacy boundary

The debugger exposes structured runtime state, tool inputs and outputs, and decision records. It must not expose private model reasoning tokens. This preserves the §49 boundary while making the runtime diagnosable.

### 63.4 Inspection operations

| Operation | Effect |
|---|---|
| Snapshot | Capture full inspectable state at a point in time |
| Step boundary pause | Pause at the next kernel decision point |
| Surface trace | Show every mutation and validation touching one surface |
| Decision trace | Show why a plan step was selected, with cited evidence |
| Evidence gap query | List requirements lacking evidence and the missing kind |

### 63.5 Read-only default

Debugger operations must be read-only except for explicit pause and resume. Inspection must never mutate project files, alter evidence, or change authority decisions.

### 63.6 Acceptance criteria

The debugger is satisfied only when a live run can be snapshotted and paused at a decision boundary, when the reason for a specific mutation can be traced to a cited requirement and decision, and when inspection produces no project mutation.

## 64. Historical Resource Profiling

**ContractId:** `CONTRACT.RUNTIME.PROFILING`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.PROFILING` (see §67.8)


This section extends §52.12 backpressure and resource reservation. That section reserves scarce capacity. This section supplies the historical measurements that make reservation and estimation accurate.

### 64.1 Product requirement

The runtime must predict from measured history rather than guess. Gradle builds, emulator boots, instrumentation runs, and provider calls have measurable cost profiles that determine whether a plan is feasible on the current host.

### 64.2 Profile record

```text
ResourceProfile
- operationClass: gradle_build | gradle_incremental | emulator_boot | instrumentation_run | apk_package | provider_call | static_analysis
- projectFingerprint
- hostFingerprint
- samples
- medianDuration
- p90Duration
- peakMemory
- peakCpu
- diskDelta
- failureRate
- lastUpdatedAt
```

Profiles are keyed by project and host fingerprint because the same operation costs differently on different projects and machines.

### 64.3 Planning use

Before committing to a plan the runtime must estimate total cost from profiles and compare it against available host capacity and any user-declared time bound. When the estimate exceeds capacity the runtime must reduce scope, sequence work differently, or surface the constraint. It must not begin work it can predict will exhaust the host.

### 64.4 Honest estimation

Estimates must be labeled as estimates with sample counts. An operation class with fewer than a defined minimum of samples must be reported as unprofiled rather than given a fabricated estimate.

### 64.5 Degradation detection

A sustained increase in an operation's duration or failure rate relative to its profile must raise a host or project health signal, since it commonly indicates disk pressure, a corrupted cache, or a degraded emulator.

### 64.6 Acceptance criteria

Resource profiling is satisfied only when repeated fixture runs produce stable profiles, when an over-capacity plan is reduced or declared before execution, and when unprofiled operations are reported as unprofiled rather than estimated.

## 65. Speculative Candidate Branching

**ContractId:** `CONTRACT.RUNTIME.SPECULATION`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.SPECULATION` (see §67.8)


This section extends §29 and §30 self-improvement and candidate-promotion material, and §52.11 simulation. Those cover promoting a validated candidate. This section specifies producing competing candidates safely.

### 65.1 Product requirement

For high-uncertainty work the runtime may attempt more than one approach and keep the one that validates best. This must never double-write the workspace or produce ambiguous evidence.

### 65.2 Candidate branch contract

```text
CandidateBranch
- branchId
- parentRevision
- approach
- isolatedWorkspace
- resourceBudget
- validationPlan
- outcome: pending | validated | failed | abandoned
- comparableMetrics
- selectedAsWinner: true | false
```

Every candidate must run in an isolated workspace with its own revision lineage. Candidates must never share a working tree.

### 65.3 Admission conditions

Speculative branching is permitted only when the task has a declared uncertainty, when host capacity per §64 allows the additional cost, and when the candidates are comparable by an objective validation metric. Otherwise the runtime must execute a single approach.

### 65.4 Selection rules

Selection must be decided by validation evidence, not by model preference. Candidates must be compared on identical validation plans. When candidates tie or all fail, the runtime must record the outcome and escalate rather than selecting arbitrarily.

### 65.5 Discard hygiene

Losing candidates must be discarded from the deliverable path while their evidence and failure signatures are retained in memory per §53.2. A losing candidate's code must never appear in the promoted artifact, and a losing candidate's validation must never be cited as completion evidence.

### 65.6 Acceptance criteria

Speculative branching is satisfied only when parallel candidates leave the primary workspace untouched, when the winner is selected by identical validation evidence, and when discarded candidates contribute learning without contributing code.


## 66. Agent Reasoning, Capability Selection, and Delegation Contract

**ContractId:** `CONTRACT.RUNTIME.REASONING`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.REASONING` (see BS §67.8)

**ContractId:** `CONTRACT.RUNTIME.AUTHORITY`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.AUTHORITY
- authoritySection: §33
- extendingSection: §66
- extensionType: adds_clauses
- extendedClauses: CLAUSE.REASONING.ARTIFACT_ONLY, CLAUSE.REASONING.NO_AUTHORITY, CLAUSE.REASONING.AGENT_INVOCATION, CLAUSE.REASONING.CHILD_CAPABILITY_CEILING, CLAUSE.REASONING.CHILD_RESOURCE_CEILING, CLAUSE.REASONING.HYPOTHESIS_EVIDENCE, CLAUSE.REASONING.MODE_WITHIN_POLICY
- nonOverriddenClauses: CLAUSE.AUTHORITY.MODEL_PROPOSES, CLAUSE.AUTHORITY.NO_SELF_ELEVATION

This section extends §33 (authority) and §52 (Core Agent Execution Kernel and Autonomous Loop Contract). §33 remains the authority on who decides. §52 remains the authority on the execution loop. This section adds the reasoning cycle that drives that loop, the rule that every autonomous capability is agent-invocable, and the constraints that bound recursive delegation. It defines no second execution loop.

### 66.1 The gap addressed

The runtime has an execution loop, skills, workers, swarms, and evidence authorities. What it lacks is a contract for how the agent decides what to do next: how a goal becomes hypotheses, how hypotheses become a selected strategy, how a strategy becomes a capability invocation, and how the observed result revises the plan.

Without that contract, capability invocation defaults to whatever the user interface exposes, and the agent becomes a text generator wired to buttons rather than an autonomous engineer.

### 66.2 Private reasoning boundary

Private reasoning may be used for understanding, constraint identification, hypothesis generation, strategy comparison, risk prediction, diagnosis, and self-critique. Verbatim private reasoning is never persisted, never exposed, never replayed, and never cited as evidence or authority. This preserves the boundary established in §49 and extends it with what the runtime does retain.

What the runtime retains is a structured reasoning artifact:

```text
ReasoningArtifact
- artifactId
- cycleId
- taskId
- producedAtEventId
- objective
- assumptions
- activeConstraints
- lockedDecisions
- hypotheses: HypothesisRef[]
- alternativesConsidered
- selectedStrategy
- selectionBasis
- confidence
- uncertainties
- expectedEffect
- nextAction
- requiredCapabilities
- delegationPlan
- validationPlan
```

`selectionBasis` must cite evidence, constraints, or prior failure signatures. It must not contain reasoning prose offered as its own justification. An artifact whose `selectionBasis` cites nothing is not admissible.

### 66.3 The reasoning cycle

The cycle is a state machine driven by, and subordinate to, the kernel loop of §52:

```text
OBSERVE      -> read current state, evidence, and constraints
UNDERSTAND   -> re-ground against goal, constraints, locked decisions (§53.4)
HYPOTHESIZE  -> generate candidate explanations or approaches
STRATEGIZE   -> generate and compare alternatives
SELECT       -> choose a strategy and emit a ReasoningArtifact
AUTHORIZE    -> submit the proposed action to the deterministic authorities
EXECUTE      -> invoke the granted capability
OBSERVE      -> collect results and evidence
REFLECT      -> compare expected against actual, classify the outcome
UPDATE       -> revise hypotheses, memory, and plan
DECIDE       -> continue | repair | replan | delegate | branch | terminate
```

`AUTHORIZE` is not a formality. The cycle cannot transition from `SELECT` to `EXECUTE` without an authority grant, and a denied action returns to `STRATEGIZE` with the denial recorded as a constraint.

### 66.4 Termination states

A cycle terminates in exactly one of:

| State | Meaning |
|---|---|
| COMPLETED | The objective is satisfied with evidence of an applicable kind |
| BLOCKED | A prerequisite is unavailable and no strategy can proceed |
| WAITING | Progress requires a user decision or an external dependency |
| RECOVERED | The original strategy failed and a later strategy succeeded |
| SAFELY_FAILED | No strategy succeeded; state is consistent and the failure is recorded |
| ESCALATED | The runtime cannot decide and requires a human decision node |

`SAFELY_FAILED` is a legitimate terminal state and must never be reported as completion. A cycle that stops without recording one of these states is a defect.

### 66.5 Reflection record

Every executed action produces a reflection record before the next cycle begins:

```text
ReflectionRecord
- reflectionId
- cycleId
- actionRef
- outcome: SUCCESS | PARTIAL | FAILURE | UNKNOWN
- expected
- observed
- deviation
- evidenceRefs
- rootCauseHypothesis
- confidence
- planImpact: none | revise_step | replan | change_strategy | escalate
- nextAction
```

`UNKNOWN` must be recorded when the result cannot be determined from evidence. Recording `SUCCESS` without an evidence reference is prohibited by §37.

### 66.6 Hypothesis lifecycle

Failure handling is hypothesis-driven rather than retry-driven:

```text
CREATED -> TESTED -> SUPPORTED
                  -> REJECTED
                  -> SUPERSEDED
```

```text
Hypothesis
- hypothesisId
- statement
- predictedObservation
- discriminatingTest
- state: CREATED | TESTED | SUPPORTED | REJECTED | SUPERSEDED
- supportingEvidenceRefs
- refutingEvidenceRefs
- supersededBy
- resultingRepairKind
```

A rejected hypothesis must be recorded with its refuting evidence, not discarded. Rejection is a durable structured record and feeds the failure signatures of §62.5. The runtime must not retest a rejected hypothesis on the same evidence, and must not attempt an untargeted repair while an untested discriminating test remains available.

### 66.7 Agent-invocable capabilities

Every autonomous capability is invocable by the agent through the capability layer. The user interface may request goals, issue directives per §61, and observe the reasoning stream, but it is not the owner or trigger of any capability.

```text
CapabilityInvocation
- invocationId
- cycleId
- capabilityId
- kind: skill | tool | worker | swarm | session | analysis | packaging
- arguments
- requestedPermissions
- authorityDecision: granted | denied | requires_approval
- denialReason
- resourceReservation
- resultRef
```

The set of capabilities is discoverable at runtime rather than hardcoded into the agent. The agent may query available capabilities for an objective and receive those the current environment and policy permit, so a newly installed skill or tool becomes usable without changing the agent.

Discovery reports capability availability using the status vocabulary of §5.6. Discovery grants nothing: an invocation still passes through the policy engine, and a capability reported as available may still be denied.

### 66.8 Recursive delegation ceilings

An agent may instantiate a child agent. Delegation never widens authority:

```text
DelegationGrant
- grantId
- parentAgentId
- childAgentId
- depth
- maxDepth
- capabilityCeiling
- resourceBudget
- timeBudget
- workspaceScope
- terminationPolicy
```

Two invariants bind every grant:

```text
ChildCapabilityCeiling  ⊆  ParentCapabilityCeiling
ChildResourceBudget     ≤  ParentRemainingResourceBudget
```

These are restrictions on delegation, not grants of permission. A child may hold strictly less than its parent and never more, and the sum of outstanding child budgets may never exceed the parent's remaining budget. A delegation request violating either invariant is denied and recorded.

Depth and fan-out are bounded. A grant exceeding `maxDepth`, exceeding the configured child limit, or requesting a workspace outside the parent's scope is denied.

### 66.9 Swarm evolution

A swarm is a live execution graph the agent may revise, not a fixed job queue. On observing that a worker is blocked, has finished, or has produced a conflicting result, the agent may propose spawning a diagnostic worker, cancelling obsolete work, adding a dependency edge, rerouting a task, or adjusting a resource reservation.

Every such revision is a proposal subject to the same authority path as any other action, and every reservation change respects the reservation contract of §54 and the backpressure controls of §52.12. Cross-worker review may inform reconciliation but never substitutes for evidence: one worker's approval of another's output is not evidence, per §54.4.

### 66.10 Execution mode selection

The agent selects the execution strategy for a goal, within policy:

| Mode | Applies when |
|---|---|
| INTERACTIVE | The user is present and iterating |
| BACKGROUND | Work proceeds without attention but is bounded |
| LONG_HORIZON | Work spans sessions and requires durable continuation |
| DEEP_EXECUTION | Many iterations with repeated validation are required |
| SWARM | Independent parallel workstreams are decomposable |
| UNATTENDED | No user is available to answer decisions |
| RECOVERY | The runtime is repairing a failed or interrupted state |
| VERIFICATION | Only validation and evidence collection remain |

Mode selection is a proposal. It never raises a permission ceiling, never suppresses an evidence requirement, and never converts a decision node into an assumption. In `UNATTENDED` mode a required decision produces `WAITING` or `ESCALATED` rather than a guess.

### 66.11 Acceptance criteria

The reasoning contract is satisfied only when a goal produces a recorded reasoning artifact with a cited selection basis before any mutation; when no verbatim private reasoning is persisted; when every executed action produces a reflection record; when a rejected hypothesis is retained with refuting evidence and not retested on the same evidence; when a capability invocation denied by policy returns to strategy selection with the denial recorded; when a delegation violating either ceiling invariant is denied; when a swarm revision passes the same authority path as any other action; and when every cycle terminates in exactly one declared termination state.

## 67. Runtime Safety, Consistency, and Documentation Coverage Contract

**ContractId:** `CONTRACT.RUNTIME.INVARIANTS`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.INVARIANTS` (see §67.8)

**ContractId:** `CONTRACT.RUNTIME.AUTHORITY`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.AUTHORITY
- authoritySection: §33
- extendingSection: §67
- extensionType: adds_verification
- extendedClauses: CLAUSE.INVARIANT.LEDGER_VERIFIABLE
- nonOverriddenClauses: CLAUSE.AUTHORITY.MODEL_PROPOSES, CLAUSE.AUTHORITY.NO_SELF_ELEVATION

**ContractId:** `CONTRACT.RUNTIME.EVIDENCE`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.EVIDENCE
- authoritySection: §37
- extendingSection: §67
- extensionType: adds_verification
- extendedClauses: none
- nonOverriddenClauses: CLAUSE.EVIDENCE.CLAIM_SEPARATION, CLAUSE.EVIDENCE.FRESHNESS


This section defines the invariants that must hold across every preceding section and the documentation coverage gate that governs this document set. It is the certification authority for this specification: every other section, including any added after it, is subject to the invariants of §67.1 and the coverage chain of §67.3.

This section is not the last section by position. Sections added after it extend the contracts it certifies and must register in §67.8, declare their clauses in §67.12, and resolve a complete chain in §67.15. Certification authority does not depend on document order.

### 67.1 Runtime invariants

The following invariants must hold at every point in every run. Any violation is a defect, not a degraded mode.

| Invariant | Statement |
|---|---|
| Authority | The model proposes; deterministic authorities decide mutation, permission, lifecycle, evidence, recovery, and promotion |
| Evidence | No requirement is complete without an evidence record of an applicable kind |
| Provenance | Every mutation is attributable to a worker, a task, and a requirement |
| Reservation | No mutation occurs on a surface without a valid reservation |
| Freshness | No completion evidence predates the last change to the surface it validates |
| Constraint | No active constraint or locked decision is contradicted by a later action |
| Isolation | No project memory crosses a project boundary in identifiable form |
| Honesty | Estimated, simulated, seeded, and unprofiled values are labeled as such |
| Recoverability | Every interrupted run resumes from durable state without re-asking settled questions |
| Ceiling | No component raises its own permission ceiling |

### 67.2 Invariant verification

Invariants must be verifiable from the event ledger, not asserted in prose. The runtime must provide an invariant check that replays a completed session's ledger and reports any violation with the violating event. A release whose certification fixture produces an invariant violation must not be promoted.

### 67.3 Documentation coverage contract

Every capability in this document set must have a complete traceability chain. The chain is:

Capability → Requirement → Build-spec contract → Architecture contract → Schema or state machine → Authority → Persistence → Failure and recovery → ADR → Milestone → Acceptance test → Evidence

### 67.4 Coverage edge definitions

| Edge | Satisfied when |
|---|---|
| Capability → Requirement | The capability is stated as a numbered requirement, not implied |
| Requirement → Build-spec contract | A section of this document defines the required behavior |
| → Architecture contract | The technical architecture defines the component that implements it |
| → Schema or state machine | A typed record or explicit state machine exists, including the adapter operation contracts (`AndroidTechnologyAdapter`, `AndroidDeviceAdapter`, `AndroidBuildAdapter`) and the deterministic preview-mode resolver (`PreviewModeResolverInput`, `PreviewModeResolverOutput`) where preview sync depends on them |
| → Authority | The deciding authority is named |
| → Persistence | What is stored, where, and its retention is stated |
| → Failure and recovery | The failure modes and recovery behavior are stated |
| → ADR | A decision record states what was locked and why |
| → Milestone | A development milestone sequences the implementation |
| → Acceptance test | A named test or fixture proves the behavior |
| → Evidence | The test produces a durable evidence artifact |

### 67.5 Defect rule

Any missing edge in the chain is a documentation defect. A missing edge must be recorded and resolved; it must not be interpreted as out of scope, deferred by omission, or resolved by asserting that the capability is obvious. A capability with a missing edge may not be reported as `SUPPORTED` under §5.6.

### 67.6 Documentation certification

This document set is certified only when every capability in the §5.6 coverage matrix resolves to a complete twelve-edge chain, when no section defines a contract that contradicts another section, when every referenced schema exists in the technical architecture, when every ADR referenced by a section exists in the decision log, and when every milestone referenced by a section exists in the development plan.

### 67.7 Contract Authority Registry

Precedence is not resolved by reading. Every normative contract in this document set has exactly one registered authoritative definition, identified by a stable `ContractId`. All other sections that speak to that contract are extensions and must declare themselves as such.

Each extension must declare:

```text
ExtensionDeclaration
- contractId
- authoritySection
- extendingSection
- extensionType: adds_clauses | adds_schema | adds_component | adds_verification
- extendedClauses
- nonOverriddenClauses
```

The following rules are binding and are verified mechanically, not by inspection:

| Rule | Statement |
|---|---|
| Single authority | A `ContractId` has exactly one authoritative section. Two sections claiming authority over the same `ContractId` is a certification failure. |
| Declared extension | A section addressing a registered contract without declaring `contractId` and `authoritySection` is a certification failure. |
| Acyclicity | Authority and extension relationships must form a directed acyclic graph. Any cycle is a certification failure. |
| No silent override | An extension may add clauses, schemas, components, and verification. It may not redefine an authoritative clause. |
| Versioned supersession | An authoritative clause may change only by creating a new versioned contract that supersedes the previous one through a recorded ADR. The superseded contract is retained and marked `DEPRECATED`. |
| Contradiction | Any extension clause whose value conflicts with the corresponding authoritative clause is a certification failure, regardless of which section is read first. |
| Consumption is not extension | A section that merely consumes an artifact defined by another contract is a consumer, not an extension, and declares no authority relationship. Only a section that adds normative clauses to a contract is an extension. Consumer references form no edge in the authority graph. |

The resulting shape is one authority per contract with N declared extensions, never a pairwise relationship between sections:

```text
ContractId
    |
    +-- AUTHORITATIVE SECTION (exactly one)
          |
          +-- Extension (declared)
          +-- Extension (declared)
          +-- Extension (declared)
```

Where ambiguity exists, certification fails and the document set is corrected. Ambiguity is never resolved by interpretation at implementation time.


### 67.8 Registered contract identifiers

The following `ContractId` values are the registered normative contracts of this document set. Each row names the single authoritative section, the declared extensions, the implementing architecture section, the locking ADR, and the implementing milestone. This table is the resolution source for §67.7 and the addressing source for §67.3.

| ContractId | Authority | Extensions | Architecture | ADR | Milestone | Class |
|---|---|---|---|---|---|---|
| CONTRACT.RUNTIME.SCOPE | BS §5 | BS §69 | TA §47 | ADR-180 | M11 | FOUNDATIONAL |
| CONTRACT.RUNTIME.AUTHORITY | BS §33 | BS §37, BS §52, BS §66, BS §67 | TA §21, TA §27 | ADR-066 | M65 | FOUNDATIONAL |
| CONTRACT.RUNTIME.EVIDENCE | BS §37 | BS §47, BS §56, BS §57, BS §67 | TA §23 | ADR-071 | M65 | FOUNDATIONAL |
| CONTRACT.RUNTIME.MEMORY | BS §38 | BS §53 | TA §31, TA §59 | ADR-140, ADR-141, ADR-155 | M81 | CROSS_CUTTING |
| CONTRACT.RUNTIME.CONTEXT | BS §53 | — | TA §19, TA §59 | ADR-141 | M81 | CROSS_CUTTING |
| CONTRACT.RUNTIME.WORKSPACE | BS §22 | BS §54 | TA §8, TA §46 | ADR-068 | M69 | FOUNDATIONAL |
| CONTRACT.RUNTIME.RESERVATION | BS §54 | — | TA §60 | ADR-142, ADR-143 | M82 | CROSS_CUTTING |
| CONTRACT.RUNTIME.RECONCILIATION | BS §55 | — | TA §61 | ADR-144 | M83 | CROSS_CUTTING |
| CONTRACT.RUNTIME.E2E | BS §56 | BS §59 | TA §62 | ADR-146 | M84 | CROSS_CUTTING |
| CONTRACT.RUNTIME.VERIFICATION | BS §57 | BS §47 | TA §64 | ADR-148 | M85 | CROSS_CUTTING |
| CONTRACT.RUNTIME.LOCALIZATION | BS §62 | — | TA §63 | ADR-147 | M86 | CROSS_CUTTING |
| CONTRACT.RUNTIME.SUPPLY_CHAIN | BS §58 | — | TA §70 | ADR-149 | M87 | CROSS_CUTTING |
| CONTRACT.RUNTIME.DEVICE_MATRIX | BS §59 | — | TA §65 | ADR-150 | M88 | CROSS_CUTTING |
| CONTRACT.RUNTIME.DIRECTIVE | BS §61 | — | TA §66 | ADR-145 | M89 | CROSS_CUTTING |
| CONTRACT.RUNTIME.DEBUGGER | BS §63 | — | TA §67 | ADR-152 | M89 | INTERNAL |
| CONTRACT.RUNTIME.PROFILING | BS §64 | — | TA §69 | ADR-153 | M90 | INTERNAL |
| CONTRACT.RUNTIME.TRIGGER | BS §60 | — | TA §68 | ADR-151 | M91 | CROSS_CUTTING |
| CONTRACT.RUNTIME.SPECULATION | BS §65 | — | TA §51, TA §65 | ADR-156 | M92 | INTERNAL |
| CONTRACT.RUNTIME.SKILL | BS §23 | BS §52 | TA §19 | ADR-154 | M66 | CROSS_CUTTING |
| CONTRACT.RUNTIME.PROMPT_CONTRACT | BS §69 | — | TA §73 | ADR-181 | M96 | CROSS_CUTTING |
| CONTRACT.RUNTIME.REASONING | BS §66 | BS §68 | TA §71 | ADR-167, ADR-168, ADR-169, ADR-170, ADR-171 | M94 | CROSS_CUTTING |
| CONTRACT.RUNTIME.DELIBERATION | BS §68 | — | TA §72 | ADR-172, ADR-173, ADR-174, ADR-175, ADR-176, ADR-177, ADR-178, ADR-179, ADR-184 | M95 | CROSS_CUTTING |
| CONTRACT.RUNTIME.INVARIANTS | BS §67 | BS §80 | all | ADR-157 | M93 | FOUNDATIONAL |
| CONTRACT.RUNTIME.AGENT_BUILDABILITY | BS §80 | — | — | — | — | INTERNAL |
| CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | BS §70 | — | TA §74 | ADR-194 | M107 | CROSS_CUTTING |
| CONTRACT.RUNTIME.PREVIEW_SYNC | BS §71 | — | TA §75 | ADR-195 | M108 | CROSS_CUTTING |
| CONTRACT.RUNTIME.COST_GOVERNANCE | BS §72 | — | TA §77 | ADR-197 | M111 | CROSS_CUTTING |
| CONTRACT.RUNTIME.AGENT_TRUST | BS §73 | — | TA §78 | ADR-198 | M112 | CROSS_CUTTING |
| CONTRACT.RUNTIME.CONTEXT_GOVERNANCE | BS §74 | — | TA §79 | ADR-199 | M113 | CROSS_CUTTING |
| CONTRACT.RUNTIME.ANDROID_INTEGRITY | BS §75 | — | TA §80 | ADR-200 | M114 | CROSS_CUTTING |
| CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE | BS §76 | — | TA §81 | ADR-201 | M115 | CROSS_CUTTING |
| CONTRACT.RUNTIME.BACKGROUND_CONTINUITY | BS §77 | — | TA §82 | ADR-202 | M116 | CROSS_CUTTING |
| CONTRACT.RUNTIME.APK_EXPORT | BS §78 | — | TA §83 | ADR-203 | M117 | CROSS_CUTTING |
| CONTRACT.RUNTIME.PLATFORM_CAPABILITY | BS §79 | BS §37, BS §52 | TA §84 | ADR-206 | M118 | CROSS_CUTTING |
| CONTRACT.RUNTIME.AGENT_BUILDABILITY | BS §80 | — | — | — | — | INTERNAL |

Contract classes are defined as: `FOUNDATIONAL` — required by the runtime regardless of product capability; `CROSS_CUTTING` — serves multiple product capabilities; `INTERNAL` — serves runtime operation rather than a user-facing capability; `DEPRECATED` — superseded by a versioned successor and retained for provenance.

Section references in this table are document-qualified exactly as in §67.15: `BS §n` addresses this build specification and `TA §n` addresses the technical architecture. The authority and extension columns are BS-scoped; the architecture column is TA-scoped. An unqualified reference is not resolvable and is a certification failure.

No `ContractId` may be introduced without a row in this table. A section declaring a `ContractId` absent from this table is a certification failure.

### 67.9 Bidirectional traceability

Certification must traverse the traceability chain in both directions. Forward traversal proves that every capability is implemented. Reverse traversal proves that nothing is implemented without a reason.

Forward direction, per §67.3:

```text
Capability -> Requirement -> Build spec -> Architecture -> Schema -> Authority
          -> Persistence -> Failure/recovery -> ADR -> Milestone -> Test -> Evidence
```

Reverse direction:

```text
Evidence -> Test -> Milestone -> ADR -> ContractId -> Capability or declared class
```

A forward break is an unimplemented capability. A reverse break is architectural dead code at specification level: a contract with schemas, components, decisions, milestones, and tests that no product capability requires. Both are documentation defects and both fail certification.

### 67.10 Orphan contract rule

Every registered contract must be reachable from at least one capability in the §5.6 coverage matrix, or be explicitly classified as `FOUNDATIONAL`, `CROSS_CUTTING`, `INTERNAL`, or `DEPRECATED` in §67.8.

A contract that is neither capability-reachable nor explicitly classified is an orphan contract and fails certification. Classification is a declaration that the contract serves the runtime rather than a user-facing capability; it is not a means of exempting an unused contract from scrutiny. A `DEPRECATED` contract must name its superseding contract and the ADR that recorded the transition.

### 67.11 Contract graph verification

M93 must verify the contract graph programmatically rather than by inspection. The verifier must load the §67.8 registry, resolve every declared `contractId` and `authoritySection`, and report each of the following as a distinct, individually addressable defect:

| Check | Failure condition |
|---|---|
| Duplicate authority | Two sections claim authority over one `ContractId` |
| Unregistered contract | A section declares a `ContractId` absent from §67.8 |
| Undeclared extension | A section addresses a registered contract without an ExtensionDeclaration |
| Authority cycle | The authority/extension graph contains a cycle |
| Clause contradiction | An extension clause conflicts with its authority's clause |
| Unversioned override | An extension redefines an authoritative clause with no superseding contract and ADR |
| Dangling reference | A referenced section, schema, ADR, or milestone does not exist |
| Forward break | A capability lacks any of the twelve chain edges |
| Reverse break | Evidence, test, milestone, or ADR resolves to no capability or class |
| Orphan contract | A contract is neither capability-reachable nor classified |
| Canonical identity | A cross-document reference resolves to the wrong semantic object (INVARIANT.DOCUMENTATION.CANONICAL_IDENTITY) |

The verifier must emit defects with the contract identifier, the sections involved, and the specific violated rule. Certification passes only when the verifier reports zero defects across all eleven checks in both traversal directions.


### 67.12 Clause Registry

Contradiction cannot be detected by reading prose. Every authoritative clause that an extension may touch is registered here with a stable `ClauseId`, a normative value, and a seal state. This table is the comparison source for the contradiction and override checks of §67.11.

| ClauseId | Contract | Authority | Normative value | Sealed |
|---|---|---|---|---|
| CLAUSE.MEMORY.SCOPES | CONTRACT.RUNTIME.MEMORY | §38 | session, project, runtime_improvement, credential | SEALED |
| CLAUSE.MEMORY.RETENTION_AUTHORITY | CONTRACT.RUNTIME.MEMORY | §38 | retention and deletion are user-controlled per entry | SEALED |
| CLAUSE.MEMORY.SECRET_EXCLUSION | CONTRACT.RUNTIME.MEMORY | §38 | credentials, signing keys, raw secrets never enter semantic memory | SEALED |
| CLAUSE.CONTEXT.CONSTRAINT_PRIORITY | CONTRACT.RUNTIME.CONTEXT | §53 | active constraints and locked decisions are never evicted for budget | SEALED |
| CLAUSE.CONTEXT.SOURCE_REQUIRED | CONTRACT.RUNTIME.CONTEXT | §53 | a memory record requires a non-empty source event set | SEALED |
| CLAUSE.WORKSPACE.SINGLE_WRITER | CONTRACT.RUNTIME.WORKSPACE | §22 | one worker holds write ownership of a workspace path at a time | SEALED |
| CLAUSE.RESERVATION.GRANT_AUTHORITY | CONTRACT.RUNTIME.RESERVATION | §54 | only the deterministic runtime grants, revokes, or invalidates a reservation | SEALED |
| CLAUSE.RESERVATION.STALE_INVALIDATION | CONTRACT.RUNTIME.RESERVATION | §54 | a surface change invalidates every read_stable reservation on it | SEALED |
| CLAUSE.RECONCILE.USER_PRECEDENCE | CONTRACT.RUNTIME.RECONCILIATION | §55 | runtime output never overwrites user-authored content | SEALED |
| CLAUSE.RECONCILE.ORIGIN_SOURCE | CONTRACT.RUNTIME.RECONCILIATION | §55 | origin is determined by mutation fingerprint, not timestamp | SEALED |
| CLAUSE.E2E.DETERMINISM | CONTRACT.RUNTIME.E2E | §56 | non-deterministic scenarios are excluded from completion evidence | SEALED |
| CLAUSE.E2E.SEED_PROVENANCE | CONTRACT.RUNTIME.E2E | §56 | seeded state is labeled and never presented as application behavior | SEALED |
| CLAUSE.VERIFY.IN_LOOP | CONTRACT.RUNTIME.VERIFICATION | §57 | a mutation with a new unresolved diagnostic does not advance | SEALED |
| CLAUSE.VERIFY.ASSERTION_ORDER | CONTRACT.RUNTIME.VERIFICATION | §57 | behavioral assertions precede implementation or are marked post_hoc | SEALED |
| CLAUSE.VERIFY.NON_VACUITY | CONTRACT.RUNTIME.VERIFICATION | §57 | critical-logic assertion sets must fail against an injected fault | SEALED |
| CLAUSE.EVIDENCE.CLAIM_SEPARATION | CONTRACT.RUNTIME.EVIDENCE | §37 | a model claim is never completion evidence | SEALED |
| CLAUSE.EVIDENCE.FRESHNESS | CONTRACT.RUNTIME.EVIDENCE | §37 | evidence predating the last change to its surface is invalid | SEALED |
| CLAUSE.LOCALIZE.CAUSE_SCOPE | CONTRACT.RUNTIME.LOCALIZATION | §62 | repair is confined to the identified cause surface | SEALED |
| CLAUSE.SUPPLY.BLOCK_ON_FINDING | CONTRACT.RUNTIME.SUPPLY_CHAIN | §58 | a finding is blocking or accepted with a recorded reason | SEALED |
| CLAUSE.SUPPLY.SBOM_REQUIRED | CONTRACT.RUNTIME.SUPPLY_CHAIN | §58 | an artifact without a complete SBOM is not promotable | SEALED |
| CLAUSE.DEVICE.PRIMARY_REQUIRED | CONTRACT.RUNTIME.DEVICE_MATRIX | §59 | unavailable secondary devices produce declared gaps, never implicit passes | SEALED |
| CLAUSE.DIRECTIVE.BOUNDED_AUTHORITY | CONTRACT.RUNTIME.DIRECTIVE | §61 | a directive may not raise a permission ceiling or bypass an evidence requirement | SEALED |
| CLAUSE.DIRECTIVE.BOUNDARY_APPLICATION | CONTRACT.RUNTIME.DIRECTIVE | §61 | a directive applies at a kernel decision point, never mid-mutation | SEALED |
| CLAUSE.DEBUG.READ_ONLY | CONTRACT.RUNTIME.DEBUGGER | §63 | inspection performs no project mutation and no evidence change | SEALED |
| CLAUSE.DEBUG.REASONING_BOUNDARY | CONTRACT.RUNTIME.DEBUGGER | §63 | private reasoning tokens are never exposed | SEALED |
| CLAUSE.PROFILE.HONEST_ESTIMATE | CONTRACT.RUNTIME.PROFILING | §64 | an operation below minimum sample count reports unprofiled, not an estimate | SEALED |
| CLAUSE.TRIGGER.DEFAULT_DISABLED | CONTRACT.RUNTIME.TRIGGER | §60 | external network triggers are disabled by default | SEALED |
| CLAUSE.TRIGGER.CEILING_CAP | CONTRACT.RUNTIME.TRIGGER | §60 | effective ceiling is the minimum of trigger and policy ceilings | SEALED |
| CLAUSE.SPECULATE.EVIDENCE_SELECTION | CONTRACT.RUNTIME.SPECULATION | §65 | candidate selection is decided by identical validation evidence | SEALED |
| CLAUSE.SPECULATE.DISCARD_HYGIENE | CONTRACT.RUNTIME.SPECULATION | §65 | discarded candidate code never enters a promoted artifact | SEALED |
| CLAUSE.SKILL.NO_PERMISSION_GRANT | CONTRACT.RUNTIME.SKILL | §23 | loading a skill never grants a permission | SEALED |
| CLAUSE.SKILL.SESSION_PINNING | CONTRACT.RUNTIME.SKILL | §23 | a bound skill version is pinned for the session's duration | SEALED |
| CLAUSE.SCOPE.ANDROID_ONLY_TARGET | CONTRACT.RUNTIME.SCOPE | §5 | Project.targetPlatforms must equal exactly ["android"] at every revision | SEALED |
| CLAUSE.SCOPE.NO_NON_ANDROID_DELIVERABLE | CONTRACT.RUNTIME.SCOPE | §5 | no resolver path, worker, or capability may produce a non-Android deployable | SEALED |
| CLAUSE.AUTHORITY.MODEL_PROPOSES | CONTRACT.RUNTIME.AUTHORITY | §33 | the model proposes; deterministic authorities decide | SEALED |
| CLAUSE.AUTHORITY.NO_SELF_ELEVATION | CONTRACT.RUNTIME.AUTHORITY | §33 | no component raises its own permission ceiling | SEALED |
| CLAUSE.REASONING.ARTIFACT_ONLY | CONTRACT.RUNTIME.REASONING | §66 | verbatim private reasoning is never persisted; only structured artifacts are retained | SEALED |
| CLAUSE.REASONING.NO_AUTHORITY | CONTRACT.RUNTIME.REASONING | §66 | reasoning proposes and never decides mutation, permission, evidence, or promotion | SEALED |
| CLAUSE.REASONING.AGENT_INVOCATION | CONTRACT.RUNTIME.REASONING | §66 | every autonomous capability is agent-invocable and the interface owns none | SEALED |
| CLAUSE.REASONING.CHILD_CAPABILITY_CEILING | CONTRACT.RUNTIME.REASONING | §66 | a child capability ceiling is a subset of its parent ceiling | SEALED |
| CLAUSE.REASONING.CHILD_RESOURCE_CEILING | CONTRACT.RUNTIME.REASONING | §66 | a child resource budget never exceeds the parent remaining budget | SEALED |
| CLAUSE.REASONING.HYPOTHESIS_EVIDENCE | CONTRACT.RUNTIME.REASONING | §66 | a rejected hypothesis is retained with its refuting evidence | SEALED |
| CLAUSE.REASONING.MODE_WITHIN_POLICY | CONTRACT.RUNTIME.REASONING | §66 | execution mode is agent-selected and never raises a permission ceiling | SEALED |
| CLAUSE.PROMPT_CONTRACT.NO_TEMPLATE_CATALOG | CONTRACT.RUNTIME.PROMPT_CONTRACT | §69 | no app archetype, framework, or template is presented as a required user-facing choice; the resolver infers the Android implementation from evidence | SEALED |
| CLAUSE.PROMPT_CONTRACT.NO_FAKE_EXECUTION | CONTRACT.RUNTIME.PROMPT_CONTRACT | §69 | prompt and UI layers never label PREDICTED, SIMULATED, REQUESTED, STALE, or INVALIDATED states as VERIFIED, OBSERVED, running, passed, completed, or verified | SEALED |
| CLAUSE.PROMPT_CONTRACT.VERIFIED_ONLY_COMPLETION | CONTRACT.RUNTIME.PROMPT_CONTRACT | §69 | only an independent validator or a supervised observation may produce completion evidence; model statements, predictions, and simulations are proposals | SEALED |
| CLAUSE.DELIBERATE.RUNTIME_GRANTS_BUDGET | CONTRACT.RUNTIME.DELIBERATION | §68 | the agent requests reasoning effort; only the runtime grants it | SEALED |
| CLAUSE.DELIBERATE.SUFFICIENCY_NOT_CONFIDENCE | CONTRACT.RUNTIME.DELIBERATION | §68 | stated model confidence is never sufficient grounds to proceed | SEALED |
| CLAUSE.DELIBERATE.EVIDENCE_PRODUCING | CONTRACT.RUNTIME.DELIBERATION | §68 | consecutive observation-free passes are bounded and force evidence acquisition | SEALED |
| CLAUSE.DELIBERATE.CRITIC_NO_MUTATION | CONTRACT.RUNTIME.DELIBERATION | §68 | the adversarial critic produces findings and never mutates the project | SEALED |
| CLAUSE.DELIBERATE.ESCALATION_NOT_AUTHORITY | CONTRACT.RUNTIME.DELIBERATION | §68 | model escalation never widens the permission ceiling | SEALED |
| CLAUSE.DELIBERATE.CONTINUATION_DURABLE | CONTRACT.RUNTIME.DELIBERATION | §68 | compaction preserves active hypotheses and rejected strategies | SEALED |
| CLAUSE.DELIBERATE.DIMINISHING_RETURN | CONTRACT.RUNTIME.DELIBERATION | §68 | no-progress deliberation changes approach rather than reasoning further | SEALED |
| CLAUSE.DELIBERATE.CAUSAL_ESCALATION | CONTRACT.RUNTIME.DELIBERATION | §68 | an effort escalation must record the observed condition that triggered it | SEALED |
| CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS | CONTRACT.RUNTIME.DELIBERATION | §68 | no project mutation occurs between deliberation entry and the AUTHORIZE grant | SEALED |
| CLAUSE.INVARIANT.LEDGER_VERIFIABLE | CONTRACT.RUNTIME.INVARIANTS | §67 | invariants are verified from the event ledger, not asserted in prose | SEALED |
| CLAUSE.INTEGRATION.REFERENCE_NOT_REDEFINITION | CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | §70 | the boundary envelope references specialized contracts and does not redefine them | SEALED |
| CLAUSE.INTEGRATION.AUTHORITY_EXPLICIT | CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | §70 | every applicable boundary names its deterministic authority references | SEALED |
| CLAUSE.INTEGRATION.NO_FABRICATED_EVIDENCE | CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | §70 | a boundary cannot treat predicted, simulated, requested, stale, or invalidated output as verified evidence | SEALED |
| CLAUSE.INTEGRATION.APPLICABILITY_EXPLICIT | CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | §70 | an inapplicable chain stage requires an explicit applicability value and reason | SEALED |
| CLAUSE.INTEGRATION.INVALIDATION_LINKED | CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | §70 | downstream evidence and effects link to the identities that can invalidate them | SEALED |
| CLAUSE.PREVIEW_SYNC.SINGLE_REDUCER | CONTRACT.RUNTIME.PREVIEW_SYNC | §71 | one canonical reducer applies preview synchronization events | SEALED |
| CLAUSE.PREVIEW_SYNC.ORDERED_REPLAY | CONTRACT.RUNTIME.PREVIEW_SYNC | §71 | duplicate and out-of-order events cannot overwrite a newer projection | SEALED |
| CLAUSE.PREVIEW_SYNC.EVIDENCE_BOUND | CONTRACT.RUNTIME.PREVIEW_SYNC | §71 | displayed completed stages require current evidence bound to the projection | SEALED |
| CLAUSE.PREVIEW_SYNC.NO_LOCAL_ADVANCE | CONTRACT.RUNTIME.PREVIEW_SYNC | §71 | a disconnected UI cannot advance preview truth or evidence locally | SEALED |
| CLAUSE.PREVIEW_SYNC.IDENTITY_MATCH | CONTRACT.RUNTIME.PREVIEW_SYNC | §71 | an event may update only a compatible preview identity and revision | SEALED |
| CLAUSE.PREVIEW_SYNC.ADAPTER_BOUND | CONTRACT.RUNTIME.PREVIEW_SYNC | §71 | every preview operation that performs build, install, launch, observation, screenshot, UI hierarchy, Logcat, validation, or failure-classification work has exactly one execution surface: `AndroidBuildAdapter` for build and artifact operations or `AndroidDeviceAdapter` for device and runtime operations; the `AndroidTechnologyAdapter` resolves those authorities and MUST NOT execute their concrete operations itself; every emitted `PreviewSyncEvent` and corresponding `PreviewSyncEvidenceRecord` MUST carry the `adapterId`, `adapterVersion`, `technologyPlanHash`, and the resolved `buildAdapterIdentity` or `deviceAdapterIdentity` | SEALED |
| CLAUSE.PREVIEW_SYNC.MODE_RESOLVER | CONTRACT.RUNTIME.PREVIEW_SYNC | §71 | the `PreviewRevision.previewMode` is selected only by the deterministic resolver defined in technical architecture §73.11; a model, worker, UI, or prompt MUST NOT select the preview mode directly | SEALED |
| CLAUSE.COST.NO_UNTRACKED_USAGE | CONTRACT.RUNTIME.COST_GOVERNANCE | §72 | every billable or budget-relevant operation records reserved, settled, or rejected usage | SEALED |
| CLAUSE.COST.EXHAUSTION_EXPLICIT | CONTRACT.RUNTIME.COST_GOVERNANCE | §72 | budget exhaustion causes a recorded downgrade, pause, approval request, or safe failure and never silent continuation | SEALED |
| CLAUSE.TRUST.SCAN_BEFORE_EXECUTION | CONTRACT.RUNTIME.AGENT_TRUST | §73 | untrusted skill, MCP, plugin, or instruction content cannot execute before trust assessment and policy admission | SEALED |
| CLAUSE.TRUST.REVOCATION_WINS | CONTRACT.RUNTIME.AGENT_TRUST | §73 | revocation or policy denial invalidates future invocation even when a prior scan passed | SEALED |
| CLAUSE.CONTEXT.POLICY_VISIBLE | CONTRACT.RUNTIME.CONTEXT_GOVERNANCE | §74 | compaction, cache use, exclusion, redaction, and telemetry policy are recorded and visible to runtime governance | SEALED |
| CLAUSE.CONTEXT.CONSTRAINT_PRESERVED | CONTRACT.RUNTIME.CONTEXT_GOVERNANCE | §74 | active constraints, locked decisions, evidence lineage, and required source context are never evicted for budget | SEALED |
| CLAUSE.INTEGRITY.APPLICABILITY_EXPLICIT | CONTRACT.RUNTIME.ANDROID_INTEGRITY | §75 | unsupported or unconfigured integrity signals are recorded as not applicable or unavailable, never as passes | SEALED |
| CLAUSE.INTEGRITY.RUNTIME_SIGNALS_SEPARATE | CONTRACT.RUNTIME.ANDROID_INTEGRITY | §75 | ANR, battery, Doze, Play Integrity, and device signals remain separate observations with independent evidence | SEALED |
| CLAUSE.FCP.AUTHORIZED_COMMANDS | CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE | §76 | every UI command is authenticated, project-scoped, capability-checked, and admitted by the control plane before execution | SEALED |
| CLAUSE.FCP.TYPED_FAILURES | CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE | §76 | a failed UI command returns a typed error with correlation, retryability, diagnostic reference, and authority decision without exposing secrets | SEALED |
| CLAUSE.FCP.REPLAY_CONTINUITY | CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE | §76 | subscription replay uses a durable sequence and snapshot cutover; gaps freeze advancement until continuity is restored | SEALED |
| CLAUSE.FCP.PROJECTION_SEPARATION | CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE | §76 | UI optimistic input and pending-command state cannot mutate authoritative domain, evidence, preview, or policy projection | SEALED |
| CLAUSE.CONTINUITY.NO_UI_DEPENDENCY | CONTRACT.RUNTIME.BACKGROUND_CONTINUITY | §77 | eligible autonomous work does not require an open or connected UI and may continue from durable state | SEALED |
| CLAUSE.CONTINUITY.RECOVER_OR_STOP | CONTRACT.RUNTIME.BACKGROUND_CONTINUITY | §77 | interruption recovery either resumes from a durable checkpoint, reconciles an unknown outcome, requests an explicitly required user decision, or stops safely | SEALED |
| CLAUSE.CONTINUITY.TRUTHFUL_STATE | CONTRACT.RUNTIME.BACKGROUND_CONTINUITY | §77 | a suspended, offline, lost, or unreconciled condition cannot be projected as active progress, verified evidence, or completion | SEALED |
| CLAUSE.EXPORT.PROFILE_BOUND | CONTRACT.RUNTIME.APK_EXPORT | §78 | deployment delivery exports only a verified artifact allowed by the declared PackagingProfile and destination policy; source access remains a separate operation | SEALED |
| CLAUSE.EXPORT.HASH_AND_IDENTITY | CONTRACT.RUNTIME.APK_EXPORT | §78 | local APK delivery records source and destination identity, byte count, hashes, signing binding, validation and promotion decisions, and post-copy verification | SEALED |
| CLAUSE.EXPORT.SOURCE_NOT_DELIVERY | CONTRACT.RUNTIME.APK_EXPORT | §78 | workspace, ZIP, and Git access never satisfies deployment-artifact delivery or Android completion by itself | SEALED |
| CLAUSE.PLATFORM.HOST_TARGET_SEPARATION | CONTRACT.RUNTIME.PLATFORM_CAPABILITY | §79 | host environment, target platform, validation platform, and certification status are distinct state values and are never collapsed into one build, validation, or completion result | SEALED |
| CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE | CONTRACT.RUNTIME.PLATFORM_CAPABILITY | §79 | host-platform compilation or cross-compilation never establishes native target-runtime capability, runtime validation, or certification | SEALED |
| CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION | CONTRACT.RUNTIME.PLATFORM_CAPABILITY | §79 | platform capability state (AVAILABLE, REPAIRABLE, USER_REQUIRED, UNAVAILABLE) is classified by the deterministic EnvironmentCapabilityPlanner from observed preflight; a model, worker, or skill never sets or raises it | SEALED |
| CLAUSE.PLATFORM.EVIDENCE_ENV_BINDING | CONTRACT.RUNTIME.PLATFORM_CAPABILITY | §79 | platform runtime evidence is valid only when bound to the matching EnvironmentCapabilityRecord fingerprint, target platform, and source revision; a mismatch invalidates it | SEALED |
| CLAUSE.PLATFORM.VALIDATION_ENV_RESERVATION | CONTRACT.RUNTIME.PLATFORM_CAPABILITY | §79 | native target-validation tasks execute only after reserving a matching ValidationEnvironment under a durable lease; without the lease there is no validation or certification claim | SEALED |
| CLAUSE.PLATFORM.NO_SUBSTITUTE_TARGET | CONTRACT.RUNTIME.PLATFORM_CAPABILITY | §79 | containers, VMs, WSL, simulated, or remote environments never substitute for the declared target platform's native runtime validation, and are never generated product targets | SEALED |
| CLAUSE.BUILDABILITY.NO_AGENT_HALLUCINATION | CONTRACT.RUNTIME.AGENT_BUILDABILITY | §80 | every schema, procedure, default value, decision criteria, prompt template, test fixture, and implementation sequence MUST be explicitly defined; an agent MUST NEVER have to invent, guess, or infer any of these | SEALED |
| CLAUSE.BUILDABILITY.COMPLETE_PROCEDURES | CONTRACT.RUNTIME.AGENT_BUILDABILITY | §80 | every process described as "the runtime handles it" or "the system should" MUST have a concrete step-by-step procedure defined in §80 | SEALED |
| CLAUSE.BUILDABILITY.EXPLICIT_DEFAULTS | CONTRACT.RUNTIME.AGENT_BUILDABILITY | §80 | every "configurable" parameter MUST have a default value defined in §80.3 | SEALED |
| CLAUSE.BUILDABILITY.DETERMINISTIC_DECISIONS | CONTRACT.RUNTIME.AGENT_BUILDABILITY | §80 | when the runtime has multiple options, it MUST choose using the explicit decision criteria defined in §80.4 | SEALED |
A `SEALED` clause may not be restated with a different value by any extension. An extension referencing a sealed `ClauseId` must list it under `nonOverriddenClauses` in its ExtensionDeclaration, which asserts that the extension adopts the authoritative value unchanged.

Changing a sealed clause requires a new versioned contract, a recorded ADR, and reclassification of the superseded contract as `DEPRECATED` per §67.7. An extension that lists a sealed clause under `extendedClauses` rather than `nonOverriddenClauses` is an unversioned override and fails certification.

### 67.13 ExtensionDeclaration format

An extending section must carry a declaration block in this exact form so it can be parsed without interpretation:

```text
**ContractId:** `<contract being extended>`
**ExtensionDeclaration:**
- authorityContractId: <ContractId whose authority governs>
- authoritySection: §<n>
- extendingSection: §<n>
- extensionType: adds_clauses | adds_schema | adds_component | adds_verification
- extendedClauses: <new ClauseIds introduced by this extension, or none>
- nonOverriddenClauses: <sealed ClauseIds adopted unchanged>
```

A section that declares a `ContractId` for a contract it does not author must carry this block. A `ContractId` with no declaration block, a declaration whose `authoritySection` disagrees with §67.8, or a declaration listing a sealed clause under `extendedClauses` is a certification failure.

Sections that author their own contract declare `**Registry role:** authoritative definition of \u0060<ContractId>\u0060` instead and require no ExtensionDeclaration. The ContractId must be stated explicitly; the bare form is ambiguous in sections that both author one contract and extend another, and is a certification failure.

### 67.14 Contract reachability

A contract is capability-reachable when a registered capability in §5.7 lists it under required contracts, directly or transitively through the extension graph of §67.8.

| Class | Reachability requirement |
|---|---|
| CROSS_CUTTING | Must be capability-reachable from at least one registered capability |
| FOUNDATIONAL | Need not be capability-reachable; must be required by at least two other registered contracts |
| INTERNAL | Need not be capability-reachable; must be referenced by at least one registered capability or one other contract's architecture section |
| DEPRECATED | Must name a superseding ContractId and the ADR that recorded the transition |

Classification is a declaration of the contract's role, not an exemption from reachability. A contract whose class requirement above is unmet is an orphan contract and fails certification regardless of how it is classified.


### 67.15 Twelve-edge resolution table

§67.3 defines the chain. This table makes every edge individually addressable so forward traversal is resolved by lookup rather than by reading. Each row is one registered contract; each column is one edge.

| ContractId | Capability | Requirement | Build spec | Architecture | Schema | Authority | Persistence | Failure/recovery | ADR | Milestone | Test | Evidence |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| CONTRACT.RUNTIME.SCOPE | CAP.ANDROID.GENERATE | BS §5 | BS §5 | TA §47 | TA §47.1 | BS §5 | TA §47.2 | TA §47.3 | ADR-180 | M11 | TEST-GEN-001 | EV-GEN-001 |
| CONTRACT.RUNTIME.PROMPT_CONTRACT | CAP.ANDROID.GENERATE | BS §27 | BS §69 | TA §73 | TA §73.1 | BS §69 | TA §73.4 | TA §73.7 | ADR-181 | M96 | TEST-GEN-001 | EV-GEN-001 |
| CONTRACT.RUNTIME.AUTHORITY | CAP.ANDROID.GENERATE | BS §33 | BS §33 | TA §21 | TA §27.1 | BS §33 | TA §23.1 | TA §28 | ADR-066 | M65 | TEST-GEN-001 | EV-GEN-001 |
| CONTRACT.RUNTIME.EVIDENCE | CAP.ANDROID.GENERATE | BS §37 | BS §37 | TA §23 | TA §23.3 | BS §37 | TA §23.3 | TA §28 | ADR-071 | M65 | TEST-GEN-001 | EV-GEN-001 |
| CONTRACT.RUNTIME.MEMORY | CAP.ANDROID.LONG_HORIZON | BS §38 | BS §38 | TA §59 | TA §59.2 | BS §38 | TA §59.5 | TA §59.6 | ADR-140 | M81 | TEST-MEM-001 | EV-MEM-001 |
| CONTRACT.RUNTIME.CONTEXT | CAP.ANDROID.LONG_HORIZON | BS §53 | BS §53 | TA §59 | TA §59.3 | BS §53 | TA §59.5 | TA §59.6 | ADR-141 | M81 | TEST-MEM-001 | EV-MEM-001 |
| CONTRACT.RUNTIME.WORKSPACE | CAP.ANDROID.PARALLEL | BS §22 | BS §22 | TA §8 | TA §8.1 | BS §22 | TA §8.2 | TA §8.3 | ADR-068 | M69 | TEST-RES-001 | EV-RES-001 |
| CONTRACT.RUNTIME.RESERVATION | CAP.ANDROID.PARALLEL | BS §54 | BS §54 | TA §60 | TA §60.2 | BS §54 | TA §60.4 | TA §60.6 | ADR-143 | M82 | TEST-RES-001 | EV-RES-001 |
| CONTRACT.RUNTIME.RECONCILIATION | CAP.ANDROID.USER_COEDIT | BS §55 | BS §55 | TA §61 | TA §61.2 | BS §55 | TA §61.5 | TA §61.6 | ADR-144 | M83 | TEST-RCN-001 | EV-RCN-001 |
| CONTRACT.RUNTIME.E2E | CAP.ANDROID.E2E_VERIFY | BS §56 | BS §56 | TA §62 | TA §62.2 | BS §56 | TA §62.5 | TA §62.6 | ADR-146 | M84 | TEST-E2E-001 | EV-E2E-001 |
| CONTRACT.RUNTIME.VERIFICATION | CAP.ANDROID.QUALITY_GATE | BS §57 | BS §57 | TA §64 | TA §64.5 | BS §57 | TA §64.5 | TA §64.6 | ADR-148 | M85 | TEST-VER-001 | EV-VER-001 |
| CONTRACT.RUNTIME.LOCALIZATION | CAP.ANDROID.REGRESSION_REPAIR | BS §62 | BS §62 | TA §63 | TA §63.4 | BS §62 | TA §63.4 | TA §63.5 | ADR-147 | M86 | TEST-LOC-001 | EV-LOC-001 |
| CONTRACT.RUNTIME.SUPPLY_CHAIN | CAP.ANDROID.SECURE_RELEASE | BS §58 | BS §58 | TA §70 | TA §70.4 | BS §58 | TA §70.4 | TA §70.6 | ADR-149 | M87 | TEST-SEC-001 | EV-SEC-001 |
| CONTRACT.RUNTIME.DEVICE_MATRIX | CAP.ANDROID.DEVICE_COVERAGE | BS §59 | BS §59 | TA §65 | TA §65.4 | BS §59 | TA §65.4 | TA §65.6 | ADR-150 | M88 | TEST-DEV-001 | EV-DEV-001 |
| CONTRACT.RUNTIME.DIRECTIVE | CAP.ANDROID.LIVE_STEER | BS §61 | BS §61 | TA §66 | TA §66.4 | BS §61 | TA §66.4 | TA §66.6 | ADR-145 | M89 | TEST-DIR-001 | EV-DIR-001 |
| CONTRACT.RUNTIME.DEBUGGER | CAP.ANDROID.LIVE_STEER | BS §63 | BS §63 | TA §67 | TA §67.2 | BS §63 | TA §67.5 | TA §67.6 | ADR-152 | M89 | TEST-DIR-001 | EV-DIR-001 |
| CONTRACT.RUNTIME.PROFILING | CAP.ANDROID.LIVE_STEER | BS §64 | BS §64 | TA §69 | TA §69.3 | BS §64 | TA §69.2 | TA §69.6 | ADR-153 | M90 | TEST-DIR-001 | EV-DIR-001 |
| CONTRACT.RUNTIME.TRIGGER | CAP.ANDROID.AUTOMATED_START | BS §60 | BS §60 | TA §68 | TA §68.4 | BS §60 | TA §68.4 | TA §68.6 | ADR-151 | M91 | TEST-TRG-001 | EV-TRG-001 |
| CONTRACT.RUNTIME.SPECULATION | CAP.ANDROID.QUALITY_GATE | BS §65 | BS §65 | TA §51 | TA §65.4 | BS §65 | TA §51.1 | TA §65.6 | ADR-156 | M92 | TEST-VER-001 | EV-VER-001 |
| CONTRACT.RUNTIME.SKILL | CAP.ANDROID.SKILL_WORKFLOW | BS §23 | BS §23 | TA §19 | TA §19.1 | BS §23 | TA §19.1 | TA §19.1 | ADR-154 | M66 | TEST-SKL-001 | EV-SKL-001 |
| CONTRACT.RUNTIME.REASONING | CAP.ANDROID.AUTONOMOUS_REASONING | BS §66 | BS §66 | TA §71 | TA §71.3 | BS §66 | TA §71.7 | TA §71.9 | ADR-167 | M94 | TEST-RSN-001 | EV-RSN-001 |
| CONTRACT.RUNTIME.DELIBERATION | CAP.ANDROID.DEEP_PROBLEM_SOLVING | BS §68 | BS §68 | TA §72 | TA §72.3 | BS §68 | TA §72.9 | TA §72.10 | ADR-172, ADR-173, ADR-174, ADR-175, ADR-176, ADR-177, ADR-178, ADR-179, ADR-184 | M95 | TEST-DEL-001 | EV-DEL-001 |
| CONTRACT.RUNTIME.INVARIANTS | CAP.ANDROID.CERTIFIED_RELEASE | BS §67 | BS §67 | TA §23 | TA §23.3 | BS §67 | TA §23.3 | BS §67.2 | ADR-157 | M93 | TEST-INV-001 | EV-INV-001 |
| CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | CAP.ANDROID.GENERATE | BS §70 | BS §70 | TA §74 | TA §74.1 | BS §70 | TA §74.2 | TA §74.3 | ADR-194 | M107 | TEST-GEN-001 | EV-GEN-001 |
| CONTRACT.RUNTIME.PREVIEW_SYNC | CAP.ANDROID.LIVE_PREVIEW | BS §71 | BS §71 | TA §75 | TA §75.1 | BS §71 | TA §75.2 | TA §75.3 | ADR-195 | M108 | TEST-PSYNC-001 | EV-PSYNC-001 |
| CONTRACT.RUNTIME.COST_GOVERNANCE | CAP.ANDROID.BUDGETED_AUTONOMY | BS §72 | BS §72 | TA §77 | TA §77.1 | BS §72 | TA §77.2 | TA §77.3 | ADR-197 | M111 | TEST-COST-001 | EV-COST-001 |
| CONTRACT.RUNTIME.AGENT_TRUST | CAP.ANDROID.TRUSTED_EXTENSIONS | BS §73 | BS §73 | TA §78 | TA §78.1 | BS §73 | TA §78.2 | TA §78.3 | ADR-198 | M112 | TEST-TRUST-001 | EV-TRUST-001 |
| CONTRACT.RUNTIME.CONTEXT_GOVERNANCE | CAP.ANDROID.CONTEXT_GOVERNANCE | BS §74 | BS §74 | TA §79 | TA §79.1 | BS §74 | TA §79.2 | TA §79.3 | ADR-199 | M113 | TEST-CONTEXT-001 | EV-CONTEXT-001 |
| CONTRACT.RUNTIME.ANDROID_INTEGRITY | CAP.ANDROID.RUNTIME_INTEGRITY | BS §75 | BS §75 | TA §80 | TA §80.1 | BS §75 | TA §80.2 | TA §80.3 | ADR-200 | M114 | TEST-INTEGRITY-001 | EV-INTEGRITY-001 |
| CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE | CAP.ANDROID.FRONTEND_CONTROL_PLANE | BS §76 | BS §76 | TA §81 | TA §81.1 | BS §76 | TA §81.2 | TA §81.3 | ADR-201 | M115 | TEST-FCP-001 | EV-FCP-001 |
| CONTRACT.RUNTIME.BACKGROUND_CONTINUITY | CAP.ANDROID.BACKGROUND_CONTINUITY | BS §77 | BS §77 | TA §82 | TA §82.1 | BS §77 | TA §82.2 | TA §82.3 | ADR-202 | M116 | TEST-BG-001 | EV-BG-001 |
| CONTRACT.RUNTIME.APK_EXPORT | CAP.ANDROID.APK_DELIVERY | BS §78 | BS §78 | TA §83 | TA §83.1 | BS §78 | TA §83.2 | TA §83.3 | ADR-203 | M117 | TEST-APK-001 | EV-APK-001 |
| CONTRACT.RUNTIME.PLATFORM_CAPABILITY | CAP.PLATFORM.CAPABILITY_TRUTH | BS §79 | BS §79 | TA §84 | TA §84.1 | BS §79 | TA §84.2 | TA §84.4 | ADR-206 | M118 | TEST-PLAT-001 | EV-PLAT-001 |
| CONTRACT.RUNTIME.AGENT_BUILDABILITY | CAP.ANDROID.CERTIFIED_RELEASE | BS §80 | BS §80 | all | all | BS §80 | all | BS §80 | ADR-157 | M93 | TEST-INV-001 | EV-INV-001 |

Every section reference in this table is document-qualified. A reference is written `BS §n` or `BS §n.m` to address this build specification, and `TA §n` or `TA §n.m` to address the technical architecture. The document namespace is part of the reference identity: an unqualified `§n.m` is not resolvable, because the same number exists in both documents with different content.

The authoritative target domain of each edge is fixed:

| Edge | Target domain |
|---|---|
| Capability | `CAP.*` in §5.7 |
| Requirement | BS |
| Build spec | BS |
| Architecture | TA |
| Schema | TA |
| Authority | BS |
| Persistence | TA |
| Failure/recovery | TA, or BS when the recovery contract is normative rather than implemented |
| ADR | decision log |
| Milestone | development plan |
| Test | test id defined in §5.7 and the development plan |
| Evidence | evidence id defined in §5.7 and the development plan |

A reference resolving in a document other than its edge's target domain is a dangling reference even when the target exists in some other document. Existence is not identity.

A row with an empty cell is a forward break. A referenced section, subsection, ADR, milestone, capability, test id, or evidence id that does not exist is a dangling reference. Both fail certification per §67.11.


## 68. Deep Deliberation and Adaptive Reasoning Contract

**ContractId:** `CONTRACT.RUNTIME.DELIBERATION`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.DELIBERATION` (see §67.8)

**ContractId:** `CONTRACT.RUNTIME.REASONING`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.REASONING
- authoritySection: §66
- extendingSection: §68
- extensionType: adds_clauses
- extendedClauses: CLAUSE.DELIBERATE.RUNTIME_GRANTS_BUDGET, CLAUSE.DELIBERATE.SUFFICIENCY_NOT_CONFIDENCE, CLAUSE.DELIBERATE.EVIDENCE_PRODUCING, CLAUSE.DELIBERATE.CRITIC_NO_MUTATION, CLAUSE.DELIBERATE.ESCALATION_NOT_AUTHORITY, CLAUSE.DELIBERATE.CONTINUATION_DURABLE, CLAUSE.DELIBERATE.DIMINISHING_RETURN, CLAUSE.DELIBERATE.CAUSAL_ESCALATION, CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS
- nonOverriddenClauses: CLAUSE.REASONING.ARTIFACT_ONLY, CLAUSE.REASONING.NO_AUTHORITY, CLAUSE.REASONING.AGENT_INVOCATION, CLAUSE.REASONING.MODE_WITHIN_POLICY, CLAUSE.REASONING.CHILD_CAPABILITY_CEILING, CLAUSE.REASONING.CHILD_RESOURCE_CEILING, CLAUSE.REASONING.HYPOTHESIS_EVIDENCE

This section extends §66 (the reasoning cycle) and §52 (the kernel loop). §66 remains the authority on what the reasoning cycle is and on the private-reasoning boundary. §52 remains the authority on the loop and on progress evaluation. This section adds how much reasoning the runtime performs before selecting an action. It defines no third loop and no new authority.

### 68.1 The gap addressed

§66 establishes that the agent reasons, and §52.3 establishes that the kernel evaluates progress. Neither specifies the decision that separates a competent engineer from a fast one: recognising that current understanding is insufficient and spending more effort before acting.

Without that contract, a single model response becomes the unit of intelligence. The agent produces a plausible strategy, executes it, and discovers the problem was misdiagnosed — repeatedly, because nothing required it to test a competing explanation first. Difficult Android defects are lost this way: a blank screen has four plausible causes, and guessing costs more than discriminating.

This section makes deliberation effort an explicit, budgeted, evidence-producing runtime activity.

### 68.2 Deliberation boundary

Deliberation is bounded by the same boundary §66.2 establishes. Additional reasoning passes may be performed; verbatim private reasoning from those passes is never persisted, exposed, replayed, or cited. What the runtime retains per deliberation is a structured record:

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
```

A record whose `passCount` exceeds one must contain one `continuationReasons` entry for each additional pass. Continuation without a stated reason is not admissible. Each continuation reason must identify the condition that justified another pass and must be associated with the pass that consumed the additional deliberation budget. This prevents unbounded thinking presented as diligence.

The runtime must never fabricate provider-reported reasoning usage. If the provider does not expose reasoning-token usage, the record must state `estimated` or `unavailable` in `accountingStatus`. Estimates are telemetry only and cannot satisfy a sufficiency or certification requirement.

This schema is the single canonical `DeliberationRecord` representation; the technical architecture (TA §72.3) implements this exact field set and must not invent alternative representations of any field.

### 68.3 The deliberation decision

At the `HYPOTHESIZE` and `STRATEGIZE` states of the §66.3 cycle, the runtime must decide whether current understanding is sufficient to proceed. The decision produces exactly one of:

| Decision | Meaning |
|---|---|
| PROCEED | Understanding is sufficient; continue to SELECT |
| DELIBERATE_MORE | Another reasoning pass is warranted |
| GATHER_EVIDENCE | A tool observation would resolve more than further reasoning |
| DELEGATE | A specialist worker is better suited to this question |
| BRANCH | Competing strategies are comparable and should be tried per §65 |
| ESCALATE | The question requires a human decision |

Inputs to the decision are goal and requirement uncertainty, hypothesis confidence spread, strategy disagreement, assessed risk, failure history for the surface, available validation evidence, architectural impact, change-surface size, remaining budget, model capability, and task criticality.

### 68.4 Deliberation budget

Deliberation cost is a distinct resource from host and provider resources. A task may have CPU, provider, and wall-clock capacity available and still be required to stop deliberating, and may have tight host capacity and still be required to deliberate further on a high-risk change.

```text
DeliberationBudget
- maxReasoningTime
- maxReasoningPasses
- maxModelRequests
- maxReasoningTokens
- maxToollessPasses
- maxEvidenceAcquisitionPasses
- maxHypotheses
- maxStrategyCandidates
- maxSpecialistConsultations
- maxCandidateBranches
- maxReasoningTokensPerPass
- maxReasoningTimePerPass
- maxProviderRequestsPerPass
- escalationThreshold
- diminishingReturnThreshold
```

`maxToollessPasses` is required: consecutive reasoning passes without new observation are the dominant failure mode of extended thinking, and the runtime must force evidence acquisition rather than permit indefinite unlit reasoning.

Before each provider request, the deterministic runtime must reserve the maximum permitted reasoning expenditure for that request from the remaining deliberation budget. The provider request cannot begin until the reservation succeeds.

When the request completes, the runtime settles the reservation against observed usage when available, or against the configured maximum when usage is unavailable, and returns unused capacity to the deliberation budget when safe to do so.

Budget reservation and settlement are transactional. Two concurrent deliberation requests must never be able to consume the same remaining reasoning budget.

### 68.5 The runtime grants the budget

The agent may request an effort level. It may never grant its own. The deterministic runtime decides the granted level from the request, the remaining budget, policy, host resources, provider capability, and task risk, and records the decision.

A request exceeding what policy or capacity permits is downgraded to the highest permitted level and recorded, never denied silently and never satisfied beyond the ceiling. This is the §33 authority principle applied to reasoning effort: the model proposes how hard to think; the runtime decides.

Every grant above the task's baseline level must record the observed condition that triggered it. An escalation whose `grantDecisionReason` cites no observed condition is not an adaptive escalation and must be rejected: effort level differing before and after is not evidence that the runtime recognised anything. This makes escalation causally auditable rather than merely observable.

Native provider reasoning and runtime deliberation are separate resources.

Provider-native reasoning increases computation within one model request. Runtime deliberation increases the number of bounded reasoning/evidence iterations across requests. Either may be used independently or together.

A DEEP deliberation may therefore use a single provider request with high native reasoning effort, multiple provider requests at a lower native effort, or multiple requests whose native effort is itself escalated, provided the total deliberation budget and provider capability constraints are respected.

The runtime must never treat a provider's native reasoning effort as proof that runtime deliberation occurred.

### 68.6 Reasoning effort levels

| Level | Applies when | Bound |
|---|---|---|
| NORMAL | Routine change on a familiar surface | Single pass, no escalation |
| EXTENDED | Uncertainty remains after the first pass | Bounded additional passes with evidence acquisition |
| DEEP | Competing hypotheses persist, or the change is high-risk | Hypothesis competition and adversarial critique required |
| EXHAUSTIVE | High-risk architectural or destructive change unresolved at DEEP | Candidate branching or escalation required at termination |

Escalation must be justified by a recorded condition, not by preference. De-escalation is permitted when uncertainty resolves. Effort level never alters permissions, evidence requirements, or authority.

The levels are behavioral contracts, not model-name aliases:

```text
NORMAL
    one bounded reasoning pass.

EXTENDED
    additional bounded passes are permitted when uncertainty remains,
    with evidence acquisition required at the configured observation-free bound.

DEEP
    competing hypotheses, discriminating tests, refutation attempts, and
    adversarial critique are mandatory.

EXHAUSTIVE
    DEEP behavior plus candidate branching or specialist escalation when
    unresolved high-risk uncertainty remains at termination.
```

Selecting a higher level does not require a different model. Selecting a stronger model does not automatically increase the effort level.

### 68.7 Sufficiency is not confidence

A stated model confidence value is never sufficient grounds to proceed. Sufficiency is a conjunction:

```text
sufficient = required evidence present
           AND uncertainty below threshold for the risk class
           AND strategy stable across the last pass
           AND validation plan defined
           AND no untested discriminating test available
```

For a high-risk architectural change the required evidence set must include architectural impact, dependency impact, affected-symbol analysis, a regression plan, and a validation plan. Reporting high confidence while any required element is absent is a defect, not a judgement call.

### 68.8 Deliberation passes produce evidence, not only prose

Deliberation is interleaved with observation rather than separated from it. A pass may read code, search symbols, inspect the impact graph, run a diagnostic, query the environment, or execute a discriminating test, then reason over what it observed.

Consecutive passes that acquire no new observation must be counted against `maxToollessPasses`, and reaching that bound forces `GATHER_EVIDENCE` or termination. Evidence acquisition during deliberation is read-only or explicitly non-mutating: deliberation must not mutate project source, and a diagnostic that would mutate requires the ordinary authorization path of §66.7.

### 68.9 Hypothesis competition

At DEEP and above, hypotheses are evaluated in competition rather than sequentially. For a defect with multiple plausible causes the runtime must enumerate candidates, define a discriminating test per candidate, rank candidates by the cost and decisiveness of their test, execute the most decisive affordable test, and reject candidates the evidence refutes.

Untargeted repair while an untested discriminating test remains available is prohibited by §66.6. This section adds that at DEEP the runtime must attempt to *refute* rather than merely to confirm: a pass that only seeks support for the leading hypothesis has not competed it.

### 68.10 Adversarial strategy critique

At DEEP and above, a selected candidate strategy must pass an adversarial critique before authorization. The critique asks what would make the strategy wrong, searches for a counterexample, and produces either a rejection finding or a set of evidence requests.

The critic produces findings and evidence requests only. It has no mutation capability, no authority to approve, and no capability to mark work complete. Critique is mandatory for architectural decisions, destructive migrations, security-relevant changes, concurrency changes, state-machine changes, authentication changes, data migrations, and release packaging.

### 68.11 Model escalation without authority escalation

Deliberation may escalate the model, not the permissions. Routing considers problem complexity, required reasoning effort, context capacity, tool-call capability, vision requirement, coding capability, historical failure rate for the surface, provider health, latency, cost, and privacy policy — extending the routing of §9 rather than replacing it.

A stronger or specialist model receives exactly the same permission ceiling, the same evidence requirements, and the same authority path as the model it replaced. Escalation changes who is asked, never what is allowed.

### 68.12 Deliberation continuation

A provider request is not the unit of deliberation. A `DeliberationSession` spans multiple model requests, tool observations, and context reconstructions, and must survive context compaction, provider failover, and runtime restart.

Continuation state comprises the deliberation revision, the reasoning objective, active hypotheses with their states, evidence acquired so far, strategies already rejected with reasons, the current effort level, and the remaining budget. Compaction must preserve this state, per the constraint-priority rule of §53.3: a compaction that discards active hypotheses or rejected-strategy records has reset the agent's thinking and is a defect.

### 68.13 Diminishing-return detection

Each pass must record measurable movement: uncertainty change, evidence added, hypotheses eliminated, and strategy stability. When movement falls below `diminishingReturnThreshold` across consecutive passes, the runtime must classify the deliberation `NO_PROGRESS` and stop reasoning in place.

`diminishingReturnThreshold` is configuration. No component may hardcode a pass count for `NO_PROGRESS`; the classification is a function of the configured threshold, the measured movement, and consecutive-pass semantics. A runtime whose behavior does not change with the configured value has not implemented detection.

On `NO_PROGRESS` the runtime must acquire evidence, escalate the model, branch candidates, delegate, or escalate to a human decision. Continuing to reason without one of those changes is prohibited. This connects to the stall detection of §29.4 and the progress evaluation of §52.3, which remain the authorities on task-level stall; this section governs stall within a single deliberation.

### 68.14 Termination

Every deliberation terminates in exactly one recorded outcome: `SUFFICIENT`, `BUDGET_EXHAUSTED`, `NO_PROGRESS`, `ESCALATED`, or `ABANDONED`.

`BUDGET_EXHAUSTED` and `NO_PROGRESS` must never be reported as sufficiency, and must never silently permit execution of the leading strategy as though it had been validated. Terminating without sufficiency yields a cycle termination state of `WAITING`, `SAFELY_FAILED`, or `ESCALATED` per §66.4.

### 68.15 Skill reasoning requirements

A skill declares the deliberation it requires, so effort is a property of the work rather than a guess:

```text
SkillDeliberationProfile
- skillId
- minimumEffortLevel
- requiredEvidenceKinds
- requiredCritique: true | false
- preferredModelCapabilities
- maxDeliberationCost
- allowedDelegation
- failureStrategies
```

A skill for a data-layer migration may require DEEP effort with schema analysis, migration compatibility, data-loss analysis, a rollback plan, and a test strategy as required evidence. The runtime must honour a declared minimum effort level, and must refuse to execute a skill whose required evidence kinds are unavailable in the current environment rather than proceeding with less.

### 68.16 Acceptance criteria

The deliberation contract is satisfied only when an agent request for a higher effort level is granted, downgraded, or denied by the runtime and never self-granted; when each additional pass records a reason for continuation; when consecutive observation-free passes are bounded and force evidence acquisition; when a high-risk change cannot proceed on stated confidence while a required evidence element is missing; when competing hypotheses are refuted by discriminating tests rather than confirmed by preference; when an adversarial critique produces findings without mutating the project; when no project mutation occurs anywhere between deliberation entry and the authorization grant; when every escalation records the observed condition that caused it; when a stronger model inherits the identical permission ceiling; when a deliberation session survives context compaction with its hypotheses and rejected strategies intact; when diminishing returns force a change of approach rather than further reasoning; and when a deliberation that ends without sufficiency never presents its leading strategy as validated.

## References

[1]: https://learn.microsoft.com/en-us/windows/apps/winui/ "WinUI 3 Documentation"

[2]: https://react.dev/ "React Documentation"

[3]: https://www.typescriptlang.org/docs/ "TypeScript Documentation"

[4]: https://docs.expo.dev/ "Expo Documentation"

[5]: https://reactnative.dev/docs/getting-started "React Native Documentation"

[6]: https://git-scm.com/doc "Git Documentation"

[7]: https://www.electronjs.org/docs/latest/ "Electron Documentation"

---

**Document owner:** Nirman product team  
**Recommended application name:** Nirman  
**Recommended first release:** Windows desktop application for local Android application generation, Nirman-managed local Android emulator preview, testing, repair, packaging, and APK export




## 69. Intent-Driven Android Synthesis and Truthful Live Preview Contract

**ContractId:** `CONTRACT.RUNTIME.PROMPT_CONTRACT`  
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.PROMPT_CONTRACT` (see BS §67.8)

**ContractId:** `CONTRACT.RUNTIME.SCOPE`  
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.SCOPE
- authoritySection: §5
- extendingSection: §69
- extensionType: adds_clauses
- extendedClauses: CLAUSE.PROMPT_CONTRACT.NO_TEMPLATE_CATALOG, CLAUSE.PROMPT_CONTRACT.NO_FAKE_EXECUTION, CLAUSE.PROMPT_CONTRACT.VERIFIED_ONLY_COMPLETION
- nonOverriddenClauses: CLAUSE.SCOPE.ANDROID_ONLY_TARGET, CLAUSE.SCOPE.NO_NON_ANDROID_DELIVERABLE

**Contract scope:** This section extends the existing Android scope, end-to-end verification, reasoning, and evidence contracts. It does not introduce another generated target or a user-facing template system.

### 69.1 No-template product invariant

Nirman MUST begin every new Android application session from the user’s intent, product concept, natural-language requirements, optional screenshots, supplied assets, device requirements, privacy constraints, and requested integrations. The user MUST NOT be required to choose an app archetype, framework, technology, starter template, or project template.

The technology resolver MUST infer and compose the Android implementation from evidence. It may select native Android, Kotlin/Compose, Java/Views, React Native/Expo, native modules, or a mixed Android architecture when the requirements justify that choice. These are implementation strategies, not user-facing choices or templates.

Internal bootstraps, dependency starters, component libraries, generated resource scaffolds, and build profiles MAY be used to make construction reliable. They MUST remain implementation details, MUST be selected by the runtime, and MUST NOT constrain the user’s app concept or be presented as the source of the product design.

A session is non-compliant if a worker asks the user to select a framework or template merely because the resolver has not completed its analysis. The correct behavior is to continue intent interpretation, request only product-requirement ambiguity, or choose and record a technology plan autonomously.

### 69.2 IntentSynthesisPromptContract

All system, coordinator, worker, skill, and deliberation prompts that can influence Android construction MUST conform to an `IntentSynthesisPromptContract`. The prompt contract MUST require the model to:

1. Extract user goals, user-visible behavior, screens, navigation, data, integrations, device capabilities, accessibility, branding, privacy, and release requirements.
2. Separate user facts from model inferences, assumptions, alternatives, and unresolved uncertainty.
3. Propose an Android technology plan without asking the user to choose a framework or template.
4. Treat any internal bootstrap as replaceable implementation machinery rather than a product limitation.
5. Produce schema-validated proposals for requirements, architecture, mutations, tools, tests, preview actions, recovery, and evidence.
6. Identify the smallest safe next action and the evidence required to evaluate it.
7. Never claim that predicted, simulated, proposed, or model-generated work was executed or verified.
8. Never authorize a tool, permission, mutation, process, preview, or artifact promotion through prompt text.

Worker prompts MUST receive the current contract version, project revision, checkpoint, relevant evidence, assigned scope, allowed capabilities, and unresolved questions. They MUST NOT replace the contract with a template-specific assumption or silently change the generated target.

### 69.3 Construction and preview truth labels

Every plan item, command, file change, preview update, test result, and artifact claim MUST carry one of these execution truth labels:

| Label | Meaning | May satisfy completion evidence? |
|---|---|---:|
| `PREDICTED` | Model or runtime forecast; no action has occurred | No |
| `SIMULATED` | Dry-run result produced without mutation or execution | No |
| `REQUESTED` | An action has been authorized or queued but has not completed | No |
| `OBSERVED` | A supervised process, device, preview, or validator produced a result | Only when the evidence kind allows observation |
| `VERIFIED` | An independent validator confirmed the observed result against a requirement | Yes |
| `STALE` | Evidence belongs to an older revision, checkpoint, emulator state, or environment | No |
| `INVALIDATED` | Previously valid evidence was invalidated by a relevant change | No |

The UI MUST never render `PREDICTED`, `SIMULATED`, or `REQUESTED` as a running application, passed test, completed task, or verified artifact.

### 69.4 Revision-bound PreviewRevision

Every preview panel state MUST be represented by a revision-bound `PreviewRevision` containing at least:

```text
previewRevisionId
projectId
projectRevisionId
activeBranchId
promotionLineage
checkpointId
sourceFingerprint
contractVersion
technologyPlanVersion
assetManifestVersion
buildVariant
artifactId
artifactFingerprint
deviceId
androidApiLevel
deviceStateFingerprint
applicationStateFingerprint
environmentStateFingerprint
previewMode
executionTruth
buildStatus
installStatus
runtimeStatus
validationStatus
createdAt
observedAt
invalidatedAt
invalidatedReason
evidenceIds
```

A preview is current only when its active branch, project revision, promotion lineage, checkpoint, source fingerprint, contract version, technology plan, asset manifest, artifact fingerprint, emulator state fingerprint, application state fingerprint, and environment state fingerprint are compatible with the active session. “Newest revision” is never sufficient to establish authority. A preview with a mismatched or unknown identity MUST be labelled `STALE` and MUST NOT satisfy completion.

### 69.5 Live preview panel layout

The default preview surface MUST show the Android application beside its execution and evidence context. It MUST provide:

| Panel region | Required information |
|---|---|
| Application viewport | Actual frame stream from the Nirman-managed headless local Android emulator rendered inside the WinUI Preview surface; emulator identity; orientation; density; API level; runtime/session identity |
| Revision header | Project revision, checkpoint, PreviewRevision, source fingerprint, artifact ID, and truth label |
| Execution timeline | Contract stage, task, worker, skill, command, observation, and next action |
| Build/install strip | Build variant, build status, install status, package ID, launch status, and timestamps |
| Evidence drawer | Tests, screenshots, Logcat, UI-hierarchy, accessibility, performance, security, and artifact evidence linked to the revision |
| Recovery banner | Candidate failure, last-known-good revision, recovery strategy, and current recovery state |
| Preview controls | Start, stop, reload, reinstall, capture, device selection, compare revision, and open evidence |

The panel MUST distinguish the last-known-good preview from a broken or incomplete candidate. It MUST never silently replace a valid preview with a predicted screen or a failed candidate.

### 69.6 Evidence-based preview transitions

Every preview update MUST be admitted through the canonical `PreviewSyncEvent` and applied by one `PreviewProjectionReducer`; agents, workers, build services, device services, evidence producers, and the UI may emit or consume events but cannot mutate the preview projection directly.

A preview update follows this sequence:

The canonical Preview viewport MUST be an embedded projection of the running Android application, not a screenshot simulation, HTML recreation, source-code rendering, or detached emulator window.

The emulator rendering surface and its input channel MUST remain inside the Nirman Preview experience. A user must be able to see and interact with the generated Android application without opening a separate emulator window or connecting a physical phone.

```text
Intent/contract accepted
    → plan and mutation authorized
    → transaction checkpoint created
    → source revision committed
    → Android build observed
    → install observed
    → process launch observed
    → emulator rendering session established
    → embedded Preview viewport observed
    → user-like interaction channel established
    → runtime interaction observed
    → screenshot/Logcat/test evidence captured
    → revision validated
    → PreviewRevision promoted
```

A preview cannot become `LIVE` merely because an APK launched. Nirman MUST establish that the running application's rendering surface is being projected into the Nirman Preview viewport and that the declared interaction channel targets that running application instance.

Each transition MUST produce a durable event and evidence reference. A model statement such as “the app is now running” is not sufficient. `RUNNING` may be displayed only after a supervised launch or reload has been observed for the declared device and revision.

### 69.7 Step-by-step preview stages

Nirman SHOULD expose meaningful validated stages rather than streaming every token or unverified file prediction:

| Stage | Minimum proof before display as completed |
|---|---|
| Intent understood | Contract schema validation and extracted requirement record |
| Product shell | Source revision committed; build/install/launch observed |
| Branding | AssetManifest integrated; asset preview observed; asset checks passed |
| Navigation | Declared routes or destinations exercised on the device/emulator |
| Core behavior | Acceptance scenarios observed and required assertions pass |
| Data/integrations | Declared request/response schemas, local or authorized integration tests, operationality dimensions, and error states observed |
| Android capabilities | Relevant permission, device API, background, or service behavior observed |
| Quality revision | Independent visual, accessibility, security, performance, and regression results |
| Release candidate | APK exists, checksum and artifact inspection pass, launch evidence is linked; optional AAB is included only when the declared packaging profile requires it |

An incomplete stage MAY be shown as in progress, predicted, simulated, blocked, or recovering, but MUST NOT be shown as completed.

### 69.8 Last-known-good and stale-candidate behavior

Before a candidate preview is installed or promoted, Nirman MUST retain the last-known-good `PreviewRevision` and its checkpoint. If build, install, launch, runtime, visual, or validation evidence fails, the candidate MUST remain visible as failed or recovering while the last-known-good preview remains available.

Rollback or repair MUST invalidate only the affected candidate evidence and MUST preserve the known-good evidence. A new preview may be promoted only after it satisfies the revision identity checks and the declared preview evidence gate.

### 69.9 Acceptance criteria

1. A new Android session can be created from an intent and optional screenshots without exposing a template or framework picker.
2. Prompt and worker contract fixtures reject template-selection instructions and non-Android target proposals.
3. An internal bootstrap, if used, is not exposed as a user-facing app archetype or technology requirement.
4. Every preview state identifies project revision, checkpoint, source fingerprint, device, artifact, truth label, and evidence.
5. Predicted, simulated, requested, stale, and invalidated states cannot satisfy completion.
6. The live panel shows the actual observed Nirman-managed local Android emulator state beside the execution timeline and evidence.
7. A failed candidate cannot replace the last-known-good preview.
8. Closing or reconnecting the UI does not change preview truth or revision identity.
9. A user can compare preview revisions and open the evidence that caused a promotion, invalidation, recovery, or rollback.
10. The final APK release report proves that the promoted preview corresponds to the packaged source revision and current asset manifest.

### 69.10 Runtime-certification and hidden-human-dependency boundary

The documentation contract and its verifier certify documentation identity, authority, traceability, and selected semantic rules only. They MUST NOT be presented as proof that the WinUI 3 host, Rust control plane, Windows isolation, provider bridge, Android toolchain, Nirman-managed local Android emulator workflow, preview, recovery loop, or APK artifact is implemented.

Runtime certification is a separate evidence class. It MUST include schema and migration tests, reducer and illegal-state tests, transaction and lease tests, Windows process and IPC tests, provider fixtures, Android build and Nirman-managed local Android emulator fixtures, preview truth tests, APK inspection, failure injection, restart recovery, self-development rollback, and hidden-human-dependency fixtures.

A hidden-human dependency includes an unclassified terminal prompt, provider login, device unlock, emulator dialog, package-manager confirmation, signing selection, missing environment variable, GUI-only installer, external-service acceptance, or suppressed approval notification. An unattended task MUST complete through an explicitly authorized automatic action, create a durable `USER_REQUIRED` decision, or enter a truthful blocked state; it MUST NOT remain silently running.

### 69.11 Clarification gate and assumption recording

Ambiguity in a user instruction is resolved by one of exactly two outcomes: a clarifying question asked before generation begins, or a recorded assumption in the intent model. Silent resolution is prohibited — an ambiguity that is neither asked nor recorded is a defect.

Nirman MUST ask a clarifying question when the ambiguity affects any of:

| Category | Example |
|---|---|
| Primary user goal | What the application is fundamentally for |
| Critical navigation structure | Tabs, drawer, or discrete screens |
| The application's distinguishing behavior | What "streak" means for a habit tracker |
| Security, authentication, or personal data | Whether accounts or sensitive data are involved |

Nirman MUST NOT ask, and MUST default silently while recording the assumption, for:

| Category | Default |
|---|---|
| Secondary color shades | Derived from the resolved theme |
| Icon selection where function is clear | Standard Material icon for the role |
| Non-critical label phrasing | Conventional Android wording |
| Spacing within the established grid | The grid value |

Clarifying questions MUST be batched and asked once, before generation begins, with a maximum of four questions. Interrupting mid-run to ask what could have been asked at the start is a defect, because it converts an autonomous session into an attended one.

Every ambiguity resolved by default MUST be written to the intent model's `assumptions` field with the alternative that was not chosen, so the user can see and revise it. Every ambiguity that is neither asked nor defaulted MUST be written to `unresolved ambiguities` and MUST NOT be treated as settled.

A clarifying question is a proposal, not an authority act. The model proposes the question; it does not thereby acquire the right to decide the answer, widen scope, or treat an unanswered question as permission (CLAUSE.AUTHORITY.MODEL_PROPOSES applies unchanged).

A domain term whose meaning materially changes the data model — "streak", "active user", "recent", "nearby", "archived" — is a primary-goal ambiguity, not a labeling detail, and belongs in the MUST-ask set.

## 70. Integration Boundary Contract

**ContractId:** `CONTRACT.RUNTIME.INTEGRATION_BOUNDARY`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.INTEGRATION_BOUNDARY` (see §67.8)

This cross-cutting contract applies to operations that cross an IPC, process, worker, workspace, persistence, provider, device, artifact, credential, signing, external-service, or documentation-verification boundary. It is a reference envelope, not a replacement for any specialized contract. It MUST reference the authoritative payload schema, lifecycle state, authority, transaction, evidence, validation, and downstream-effect contracts rather than redefining them.

```text
IntegrationBoundaryContract
- boundaryId
- integrationBoundaryVersion
- capabilityId
- sourceEntityRef
- destinationEntityRef
- boundaryKind: ipc | process | worker | workspace | persistence |
                 provider | device | artifact | external_service |
                 credential | signing | documentation
- sourceContractRef
- payloadSchemaRef
- responseSchemaRef
- protocolVersion
- adapterOrBridgeRef
- adapterOrBridgeVersion
- authorityRefs
- stateProjectionRefs
- operationRef
- transactionDomain: local | device | external_effect | none
- correlationId
- causationId
- idempotencyKey
- permissionProfileRef
- credentialReference
- lifecyclePolicyRef
- timeoutPolicy
- cancellationPolicy
- retryPolicy
- compatibilityRef
- observationRefs
- evidenceRequirements
- validationPolicyVersion
- downstreamEffectRefs
- invalidationDependencyRefs
- failureRecoveryRef
- applicability: required | optional | not_applicable
- notApplicableReason
```

For an applicable boundary operation, the contract resolves this chain:

```text
SOURCE
  → CONTRACT
  → ADAPTER / BRIDGE
  → AUTHORITY
  → STATE
  → OPERATION
  → OBSERVATION
  → EVIDENCE
  → VALIDATION
  → DOWNSTREAM EFFECT
```

`SOURCE`, `CONTRACT`, `ADAPTER / BRIDGE`, `AUTHORITY`, `STATE`, and `OPERATION` are references to the participating entities and their authoritative contracts. `OBSERVATION` and `EVIDENCE` MUST preserve the execution-truth distinction between `PREDICTED`, `SIMULATED`, `REQUESTED`, `OBSERVED`, `VERIFIED`, `STALE`, and `INVALIDATED`. `VALIDATION` resolves to the applicable independent validator and policy. `DOWNSTREAM EFFECT` records the resulting local commit, emulator state, external effect, projection update, artifact transition, or documentation-certification result.

An inapplicable stage MUST be represented by `applicability: not_applicable` and a reason. A read-only in-process helper need not manufacture an external-effect record, but no boundary may use inapplicability to avoid a required permission, authority, evidence, compatibility, or invalidation link. A boundary envelope references `IntegrationOperationality`, `ExternalEffectRecord`, `PreviewRevision`, `OperationCapability`, `ProviderContextEnvelope`, `ArtifactSet`, `SigningIdentityBinding`, and `DocumentationCertificationReport` when those specialized contracts apply. It does not create a second lifecycle, transaction, evidence, preview, provider, skill, artifact, signing, or completion authority.

Every boundary operation MUST be idempotent or explicitly non-idempotent with a declared compensation/reconciliation rule. Unknown outcomes MUST remain durable and MUST be reconciled before retrying an external or device effect. Cancellation propagates through the operation’s descendants. Timeout, retry, failure, recovery, validation, and invalidation references are required for every applicable boundary. The UI, model, worker, adapter, bridge, and verifier may propose or report outcomes but cannot approve a boundary effect or promote its downstream state.

### 70.1 Acceptance criteria

A boundary fixture must prove that source and destination identity, schema and protocol version, adapter or bridge, authority, operation, transaction domain, correlation and idempotency, lifecycle policy, observation, evidence, validation, downstream effect, failure/recovery, compatibility, and invalidation references are all resolvable. It must reject an envelope that redefines a specialized authority, fabricates verified evidence, retries an unknown external outcome without reconciliation, omits a required applicability reason, or allows a stale source or contract version to produce a current downstream effect.

## 71. Preview Synchronization Protocol

**ContractId:** `CONTRACT.RUNTIME.PREVIEW_SYNC`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.PREVIEW_SYNC` (see BS §67.8)

This contract defines how the user’s chat instruction and autonomous agent activity become a truthful live Android preview projection. It extends the existing intent, execution, evidence, preview, and integration-boundary contracts. It creates no second preview authority: `PreviewCoordinator` remains the sole service that creates, reloads, installs, invalidates, rolls back, or promotes a preview, and the deterministic runtime remains the sole lifecycle and evidence authority.

### 71.0 Adapter binding

`CONTRACT.RUNTIME.PREVIEW_SYNC` requires the selected `AndroidTechnologyPlan` to resolve through exactly one registered `AndroidTechnologyAdapter` (technical architecture §73.10). The `AndroidTechnologyAdapter` resolves execution authorities; it does not constitute an additional execution authority. Each concrete preview operation has exactly one execution surface: `AndroidBuildAdapter` for build and artifact operations, or `AndroidDeviceAdapter` for device and runtime operations. The technology adapter only exposes selection, composition, validation, planning, and failure-classification operations; it never executes concrete build, install, launch, observation, screenshot, UI hierarchy, Logcat, validation, or failure-classification work.

Every preview operation that performs build, install, launch, observation, screenshot, UI hierarchy, Logcat, validation, or failure-classification work MUST carry the `adapterId`, `adapterVersion`, `technologyPlanHash`, and the resolved `buildAdapterIdentity` or `deviceAdapterIdentity` on the emitted `PreviewSyncEvent` and on the corresponding `PreviewSyncEvidenceRecord`. Lifecycle, policy, evidence, preview, artifact, recovery, promotion, and completion decisions remain with the existing specialized authorities. The deterministic preview-mode resolver defined in technical architecture §73.11 is the sole normative selector for the `PreviewRevision.previewMode` field, including the `CONSERVATIVE_FULL_REINSTALL` refinement introduced in §73.11; a model, worker, UI, or prompt MUST NOT select the preview mode directly.

### 71.1 Canonical synchronization schemas

```text
PreviewSyncEvent
- eventId
- eventSchemaVersion
- eventSequence
- occurredAt
- projectId
- goalId
- taskId
- sessionId
- workerRunId
- correlationId
- causationId
- boundaryId
- branchId
- candidatePreviewRevisionId
- eventType: INTENT_ACCEPTED | CONTRACT_VALIDATED | PLAN_RECORDED |
             CHECKPOINT_CREATED | SOURCE_REVISION_COMMITTED |
             BUILD_REQUESTED | BUILD_OBSERVED | ARTIFACT_OBSERVED |
             INSTALL_REQUESTED | INSTALL_OBSERVED | LAUNCH_OBSERVED |
             INTERACTION_OBSERVED | OBSERVATION_CAPTURED |
             VALIDATION_OBSERVED | RECOVERY_STARTED | CANDIDATE_FAILED |
             PREVIEW_INVALIDATED | PREVIEW_PROMOTED | STREAM_GAP |
             STREAM_RECONNECTED
- eventTruth: PREDICTED | SIMULATED | REQUESTED | OBSERVED | VERIFIED |
               STALE | INVALIDATED
- projectRevisionId
- checkpointId
- sourceFingerprint
- assetManifestVersion
- contractVersion
- technologyPlanVersion
- artifactId
- artifactFingerprint
- runtimeSessionId
- deviceId
- deviceStateFingerprint
- applicationStateFingerprint
- environmentStateFingerprint
- operationRef
- observationRefs
- evidenceRefs
- validationRef
- failureRecoveryRef
- emittedBy
- authorityClass: DECLARATIVE | PLANNED | EXECUTION_OBSERVED |
                  RUNTIME_OBSERVED | EVIDENCE_BACKED | VALIDATED | CERTIFIED
- payload
- emittedAt
- previewSurfaceId
- previewSurfaceSessionId
- renderTransportId
- renderTransportVersion
- inputChannelId
- inputChannelVersion
- viewportStateFingerprint

The fields identify the actual embedded rendering and interaction projection. They do not constitute a new authority. PreviewCoordinator remains responsible for promotion and PreviewProjectionReducer remains the sole projection reducer.

PreviewProjection
- projectionRevision
- goalState
- executionState
- sourceState
- buildState
- artifactState
- installationState
- runtimeState
- deviceState
- interactionState
- validationState
- evidenceState
- recoveryState
- promotionState
- displayState

PreviewProjectionReducer
- reducerId
- reducerVersion
- projectId
- taskId
- lastAppliedEventSequence
- projectionRevision
- activePreviewRevisionId
- lastKnownGoodPreviewRevisionId
- candidatePreviewRevisionId
- lifecycleStage
- executionTruth
- buildStatus
- installStatus
- launchStatus
- runtimeStatus
- validationStatus
- streamStatus: CONNECTED | REPLAYING | STALE_STREAM | GAP_BLOCKED
- pendingEventSequences
- rejectedEventIds
- projectionDimensions
- quarantinedEventIds
- evidenceIds
- invalidationIds
- updatedAt

PreviewSyncEvidenceRecord
- evidenceId
- projectId
- taskId
- eventSequenceStart
- eventSequenceEnd
- projectionRevision
- previewRevisionId
- projectRevisionId
- checkpointId
- branchId
- deviceId
- runtimeSessionId
- artifactFingerprint
- stateFingerprints
- eventIds
- observationRefs
- evidenceRefs
- validationRefs
- invalidatedEvidenceIds
- recoveryEventIds
- promotionRecordRef
- certificationDecisionRef
- completionDecisionRef
- truth
- capturedAt
```

`PreviewSyncEvent` is the only event shape that can update the preview projection. A worker result, model message, terminal output, raw device callback, or UI action must first be normalized into this schema or remain informational. `PreviewProjectionReducer` is the only component that derives the panel’s preview state from durable events. `PreviewSyncEvidenceRecord` proves which event range and identity produced a displayed stage; it is not a substitute for the underlying device, process, visual, test, artifact, or promotion evidence.

`PreviewProjection` is evaluated as independent dimensions rather than a single success value: goal, execution, source, build, artifact, installation, runtime, device, interaction, validation, evidence, recovery, promotion, and display. For example, `buildState: SUCCEEDED`, `artifactState: APK_AVAILABLE`, `installationState: INSTALLED`, `runtimeState: RUNNING`, `validationState: IN_PROGRESS`, `evidenceState: PARTIAL`, and `promotionState: NOT_PROMOTED` may coexist. No dimension implies completion of another dimension.

The projection MUST retain causal provenance for displayed claims. A panel field that says an application is running or validated must be traceable through its `PreviewSyncEvent`, runtime observation, evidence record, validation result, and applicable promotion or completion decision. A panel screenshot is a display artifact, not proof that this provenance exists.

The `authorityClass` establishes the maximum truth level an event may advance. `DECLARATIVE` and `PLANNED` events can update intent, plan, or queued-operation fields only. `EXECUTION_OBSERVED` requires supervised process evidence. `RUNTIME_OBSERVED` requires matching Nirman-managed local Android emulator, package, process, and runtime-session observation. `EVIDENCE_BACKED` requires durable evidence references, `VALIDATED` requires an independent validator, and `CERTIFIED` requires the applicable certification authority. An event cannot advance a field beyond its authority class.

Every non-root event MUST identify its `causationId` and compatible identity lineage. A build, install, launch, interaction, observation, validation, or promotion event without a matching project revision, candidate, artifact, runtime session, device, and predecessor lineage is rejected or retained as non-authoritative history; it cannot update the current projection.

### 71.2 Event-to-preview field ownership

| Event type | Fields it may update | Required authority or evidence | Panel effect |
|---|---|---|---|
| `INTENT_ACCEPTED` | intent reference, lifecycle stage | contract admission | Shows intent received; no running preview |
| `CONTRACT_VALIDATED` | contract version, requirement references | schema/contract validator | Shows validated intent; no emulator state |
| `PLAN_RECORDED` | plan reference, technology-plan version | planner record | Shows planned work only |
| `CHECKPOINT_CREATED` | checkpoint and project revision | transaction authority | Establishes candidate baseline |
| `SOURCE_REVISION_COMMITTED` | source fingerprint, revision, branch | commit barrier and workspace authority | Marks previous preview stale when incompatible |
| `BUILD_REQUESTED` | operation reference, build status | operation capability | Shows queued/building state only |
| `BUILD_OBSERVED` | build status, artifact reference, toolchain identity | supervised process evidence | Allows build-observed stage |
| `ARTIFACT_OBSERVED` | artifact ID and fingerprint | artifact inspection evidence | Binds artifact to candidate |
| `INSTALL_REQUESTED` | device operation reference | device operation authority | Shows install requested only |
| `INSTALL_OBSERVED` | install status, emulator identity | emulator evidence | Allows installed stage |
| `LAUNCH_OBSERVED` | launch/runtime status | supervised device/process evidence | Allows `RUNNING_OBSERVED` |
| `INTERACTION_OBSERVED` | application state and interaction refs | device/test evidence | Allows interaction stage |
| `OBSERVATION_CAPTURED` | screenshot, Logcat, UI-hierarchy evidence refs | observation service | Adds evidence, not promotion by itself |
| `VALIDATION_OBSERVED` | validation status and result refs | independent validator | Allows validation stage |
| `RECOVERY_STARTED` | recovery state and last-known-good ref | RecoveryAuthority | Shows recovery without replacing known-good |
| `CANDIDATE_FAILED` | candidate failure and invalidation refs | failure/recovery authority | Keeps candidate failed; preserves known-good |
| `PREVIEW_INVALIDATED` | invalidation state and reason | invalidation authority | Marks dependent projection/evidence stale |
| `PREVIEW_PROMOTED` | active preview revision | `PreviewPromotionGate` | Replaces candidate only after all gates pass |
| `STREAM_GAP` | stream status, pending sequence range | event-store/replay authority | Freezes panel advancement and shows stale stream |
| `STREAM_RECONNECTED` | stream status and replay cursor | authenticated supervisor connection | Replays before resuming projection |

No event may update a field outside its ownership row. A promotion event cannot be emitted by a model, worker, UI, build process, or device callback; it is emitted only after the canonical promotion gate commits the decision.

### 71.3 Ordering, duplicate, stale, and reconnect rules

The reducer applies events by the durable per-project/task `eventSequence`. Reapplying an event with the same `eventId` and payload hash is idempotent and produces no second state transition. An event with a duplicate ID and a different payload is a protocol violation and is quarantined. An event whose sequence is greater than the next expected sequence is held in `pendingEventSequences`; the panel enters `GAP_BLOCKED` or `STALE_STREAM` and requests replay rather than applying the event out of order.

An event whose sequence is older than `lastAppliedEventSequence` is accepted only as a replay match. It cannot overwrite a newer projection. An event with a project revision, checkpoint, source fingerprint, artifact fingerprint, emulator state fingerprint, application state fingerprint, environment state fingerprint, contract version, or branch identity incompatible with the active candidate is marked `STALE` or `INVALIDATED` and cannot update current preview fields.

When the UI or event stream disconnects, the panel keeps the last durable projection, sets `streamStatus: STALE_STREAM`, and cannot advance lifecycle, execution truth, evidence, or promotion locally. On reconnect, the authenticated supervisor sends a snapshot and replays the missing event range. The reducer verifies the snapshot cursor, event continuity, and projection revision before returning to `CONNECTED`.

A late build, install, launch, screenshot, test, or worker event may contribute historical evidence to its matching candidate only. It cannot replace the active preview or last-known-good preview. Events received after cancellation, rollback, promotion, or worker fencing are historical or quarantined unless a new operation explicitly re-authorizes them under a new lineage.

Preview truth reconciliation compares the durable projection with the current supervised runtime observation. For a compatible project revision, artifact fingerprint, emulator identity, and runtime session, a current process/device observation can move runtime state from a previously persisted `RUNNING` claim to `STOPPED`, `CRASHED`, `DISCONNECTED`, or `UNKNOWN`; a persisted projection cannot override a contradictory current observation. If the identities do not match, the projection becomes `STALE` or `INVALIDATED` rather than combining the records. Recovery, rollback, source edits, toolchain changes, device changes, application-state changes, environment changes, contract changes, and policy changes invalidate affected projection and evidence dependencies through the existing evidence graph.

### 71.4 Runtime-certification evidence

Runtime certification must retain a `PreviewSyncEvidenceRecord` for every displayed completed stage and must prove the event sequence, reducer version, projection revision, preview revision, source revision, checkpoint, artifact fingerprint, emulator identity, state fingerprints, evidence references, and validation result. A panel screenshot alone is not runtime evidence of synchronization.

The runtime fixture must exercise a complete path:

```text
chat instruction
  → intent and contract events
  → authorized agent proposal
  → source mutation and checkpoint
  → build and artifact observations
  → install and launch observations
  → interaction and screenshot/Logcat/UI-hierarchy observations
  → validation
  → PreviewPromotionGate
  → durable PreviewSyncEvent sequence
  → PreviewProjectionReducer
  → panel projection
```

The fixture must also inject duplicate events, out-of-order events, missing event ranges, stale revisions, a failed candidate, UI disconnect, supervisor restart, stream replay, and a late device observation. The expected result is deterministic projection reconstruction, no false current state, preserved last-known-good preview, and complete evidence lineage.

### 71.5 Acceptance criteria

1. A chat instruction creates a durable task and intent record before agent execution begins.
2. Agent and worker events cannot update the preview panel except through `PreviewSyncEvent` and `PreviewProjectionReducer`.
3. Every displayed completed stage has a `PreviewSyncEvidenceRecord` with event range, projection revision, preview revision, emulator identity, artifact fingerprint, and evidence references.
4. A successful build without install and launch observation cannot display `RUNNING_OBSERVED`.
5. Duplicate events are idempotent; conflicting duplicate payloads are quarantined.
6. Out-of-order or missing events freeze advancement and initiate replay rather than being applied speculatively.
7. Stale or incompatible source, artifact, device, application, environment, branch, or contract identities cannot update the current preview.
8. UI disconnection, supervisor restart, and event-stream loss preserve durable truth and do not advance the panel locally.
9. A failed candidate cannot replace last-known-good, and a late candidate event cannot overwrite a newer projection.
10. Preview promotion remains exclusive to `PreviewCoordinator` through `PreviewPromotionGate`.
11. The complete chat-to-APK-to-device-to-evidence-to-panel fixture passes before the preview synchronization capability is reported as runtime-certified.
12. Event authority classes prevent declarative or planned messages from advancing execution, runtime, evidence, validation, or certification dimensions.
13. A contradictory current runtime observation reconciles a compatible persisted projection, while an incompatible observation becomes stale or invalidated instead of being merged.
14. The panel can answer why a displayed running or validated claim is current by exposing its event range, projection revision, preview revision, runtime session, emulator identity, artifact fingerprint, evidence references, validation references, and promotion or completion decision references.

## 72. Cost Governance Authority

**ContractId:** `CONTRACT.RUNTIME.COST_GOVERNANCE`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.COST_GOVERNANCE`

Cost governance is a deterministic policy authority placed beside permission and resource policy. It governs token budgets, provider request budgets, duration, CPU, memory, disk, emulator, and estimated monetary cost without turning ordinary budget thresholds into false completion.

The canonical `CostGovernanceRecord` contains `budgetId`, `taskId`, `sessionId`, `policyVersion`, `tokenBudget`, `requestBudget`, `durationBudget`, `resourceBudgets`, `costCap`, `reservedUsage`, `settledUsage`, `usageEventIds`, `remainingBudget`, `exhaustionOutcome`, `degradationPolicy`, `approvalPolicy`, and `evidenceIds`. The `durationBudget` defaults to 200 minutes and is user-configurable per task or project; exhaustion follows CLAUSE.COST.EXHAUSTION_EXPLICIT. `costCap` remains optional with no default value. Every operation reserves usage before execution and settles actual or provider-reported usage afterward; unknown usage remains unreconciled until resolved.

Budget exhaustion must produce one explicit outcome: reduce context, reduce concurrency, change an approved model or provider, pause for approval, continue under a renewed policy, safely fail, or degrade the task classification. Exhaustion cannot silently authorize a broader permission, discard required evidence, or mark a goal complete. The authority hierarchy is policy authority, then cost governance for resource admission, then operation capability and lifecycle authority; cost governance cannot override safety, privacy, signing, evidence, or completion authority.

### 72.1 Acceptance criteria

A fixture must prove reservation, settlement, provider-reported or estimated usage, cap exhaustion, adaptive degradation, renewal approval, cancellation, unknown-outcome reconciliation, and truthful completion classification.

## 73. Agent Trust Boundary Authority

**ContractId:** `CONTRACT.RUNTIME.AGENT_TRUST`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.AGENT_TRUST`

Agent-layer supply-chain content includes skills, MCP-compatible tools, plugins, provider-returned tool descriptions, workflow packages, and instruction files encountered inside an extension boundary. These inputs are data, not authority. The runtime must scan and assess them before loading, invocation, or instruction interpretation.

The canonical `AgentTrustAssessment` contains `assessmentId`, `subjectType`, `subjectId`, `sourceIdentity`, `contentHash`, `declaredVersion`, `provenance`, `scanProfile`, `staticFindings`, `behavioralFindings`, `requestedCapabilities`, `networkDestinations`, `secretAccessClaims`, `permissionDecisionId`, `trustState`, `revocationState`, `expiry`, `evidenceIds`, and `invalidatedBy`. Scanning must detect prompt-injection instructions, hidden tool escalation, secret exfiltration patterns, unsafe path access, undeclared network destinations, dependency or binary payload risk, and authority-impersonation claims.

A passing scan does not grant permission. The capability registry, policy authority, operation capability, credential policy, and external-effect authority still decide what may run. Revocation, hash drift, version drift, policy change, or a new requested capability invalidates prior admission and requires reassessment. Untrusted instructions must never rewrite the goal, policy, authority, or completion state.

### 73.1 Acceptance criteria

Fixtures must cover clean and malicious skill packages, MCP tool declarations, plugin instruction files, hash or version drift, revocation, secret-access attempts, undeclared network access, authority impersonation, and safe rejection with durable evidence.

## 74. Context and Cache Governance

**ContractId:** `CONTRACT.RUNTIME.CONTEXT_GOVERNANCE`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.CONTEXT_GOVERNANCE`

Context compaction and provider caching are governed independently from memory, retrieval, and provider selection. The canonical `ContextCachePolicy` contains `policyId`, `providerProfileId`, `taskId`, `contextMode`, `compactionTriggers`, `reservedTokenFloor`, `protectedContextClasses`, `cacheBreakpointPolicy`, `cacheKeyInputs`, `cacheInvalidationEvents`, `redactionPolicy`, `excludedContentClasses`, `telemetryDisclosure`, `retention`, and `evidenceIds`.

Compaction may summarize logs and ordinary context, but it must preserve active constraints, locked decisions, source and revision identity, acceptance criteria, evidence lineage, unresolved failures, required tool results, and signing or privacy restrictions. Cache reuse is valid only when provider, model, policy, context selection, project revision, relevant files, tool results, and privacy classification are compatible. A cache hit must be visible in telemetry and must not be represented as a fresh observation.

Compaction triggers include context utilization thresholds, phase boundaries, provider continuation limits, failure boundaries, and explicit policy requests. Cache invalidation occurs after source changes, policy changes, credential changes, provider or model changes, tool-result changes, evidence invalidation, or privacy classification changes. Context governance may reduce or defer work but cannot evict mandatory constraints or weaken evidence requirements.

### 74.1 Acceptance criteria

Fixtures must prove protected-constraint retention, compaction at a threshold, cache reuse and invalidation, provider/model mismatch, privacy exclusion, cache telemetry, failed compaction recovery, and preservation of causal and evidence lineage.

## 75. Android Runtime Integrity Contract

**ContractId:** `CONTRACT.RUNTIME.ANDROID_INTEGRITY`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.ANDROID_INTEGRITY`

Android runtime integrity is a set of independent observations, not a single boolean. The canonical `AndroidRuntimeIntegrityObservation` contains `observationId`, `projectRevisionId`, `artifactId`, `packageName`, `deviceId`, `runtimeSessionId`, `appIntegritySignal`, `playIntegrityApplicability`, `playIntegrityEvidenceId`, `anrEvidenceIds`, `batteryObservationIds`, `dozeObservationIds`, `permissionObservationIds`, `coverageClass`, `state`, `observedAt`, `source`, and `invalidatedBy`.

Play Integrity is conditional: it is recorded only when the application, device, credentials, network, and configured service support it. An emulator without a valid applicable signal is not treated as a Play Integrity pass. ANR, startup/crash, battery-sensitive behavior, background restrictions, Doze, permission, and device-availability observations remain separate. Missing or unsupported signals produce `NOT_APPLICABLE`, `UNAVAILABLE`, or `USER_REQUIRED` records and cannot silently satisfy a required acceptance criterion.

The integrity contract does not expand Nirman into a cloud deployment system or require production-only services for every local preview. It makes declared runtime-integrity requirements explicit and links each applicable signal to the Android artifact, device, runtime session, evidence, validation, and invalidation graph.

### 75.1 Acceptance criteria

Fixtures must cover applicable and inapplicable Play Integrity, ANR capture, startup and crash evidence, battery-sensitive checks, Doze or background restriction behavior, permission behavior, emulator session loss, stale observations, and honest coverage reporting.

## 76. Frontend–Control-Plane Protocol Contract

**ContractId:** `CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE`

The desktop frontend is a presentation client of the authoritative local control plane. The canonical path is:

```text
UI input or user command
  → UICommandEnvelope
  → authenticated connection and project-scope check
  → UICommandRegistry entry
  → use-case handler and deterministic authorities
  → owned SQLite transaction
  → durable domain events
  → ProjectionSnapshot and UIResponseEnvelope
  → frontend projection reducer
```

The frontend may own view preferences, form input, selection, filters, scroll position, and pending-command display. It cannot own task, worker, process, build, preview, artifact, evidence, policy, signing, or completion truth. The control plane is the only component that authorizes operations, persists domain state, emits authoritative events, and derives projections.

### 76.1 UICommandRegistry

Every command must be registered with `commandKind`, `requestSchemaRef`, `responseSchemaRef`, `requiredAuthority`, `requiredCapability`, `projectScope`, `transactionDomain`, `idempotencyPolicy`, `timeoutPolicy`, `cancellationPolicy`, `emittedEventTypes`, `projectionEffects`, `errorCodes`, and `sensitiveFields`. The registry (mirrored by `command_registry()` in `nirman-ipc`) is:

| Command kind | Domain use case | Required authority | Transaction domain | Projection effect |
|---|---|---|---|---|
| `project.open` | Open and inspect a project | Workspace and policy authority | Local | Project and health projection |
| `task.start` | Start an approved Android goal | Lifecycle, policy, and capability authority | Local | Task and worker projection |
| `task.cancel` | Request cancellation | Lifecycle authority | Local | Cancellation and recovery projection |
| `task.resume` | Resume an eligible task | Lifecycle and recovery authority | Local | Task and continuation projection |
| `task.submit_instruction` | Submit a natural-language build instruction that opens the task's background run | Lifecycle authority | Local | Task and instruction projection |
| `task.pause` | Pause an active task while preserving the background run | Lifecycle authority | Local | Task projection |
| `connection.reconnect` | Rebind a UI session to the running host after a disconnect | Recovery authority | Local | Continuity projection |
| `workspace.apply_patch` | Admit a worker or user patch | Workspace, reconciliation, and policy authority | Local | Revision and diff projection |
| `preview.start` | Start a revision-bound preview | Preview and lifecycle authority | Device | Preview candidate projection |
| `preview.stop` | Stop a managed preview session | Lifecycle authority | Device | Preview lifecycle projection |
| `preview.promote` | Request candidate promotion | Preview promotion and evidence authority | Device | Promotion decision projection |
| `validation.run` | Run declared checks | Verification and policy authority | Local or device | Validation and evidence projection |
| `artifact.build` | Build a declared Android artifact | Toolchain and artifact authority | Local | Build and artifact projection |
| `artifact.export` | Export a verified source or declared artifact; deployment delivery is profile-bound | Artifact and external-effect authority | Local | Delivery and export projection |
| `provider.test` | Test a configured provider profile | Provider and credential policy authority | External | Provider operationality projection |
| `provider.execute` | Execute a provider request through the M44 bridge under a locked environment | Provider bridge authority | External | Provider execution projection |
| `settings.update_provider` | Update a provider profile | Credential and policy authority | Local | Settings and provider projection |
| `android.construction.create` | Create and validate the Android construction contract (M39/M47) | Construction contract authority | Local | Android construction contract projection |
| `android.toolchain.preflight` | Preflight and lock the Android toolchain environment (M43) | Toolchain authority | Local | Android toolchain and environment projection |
| `android.requirements.evaluate` | Evaluate Android requirements and select repairs (M47) | Android requirement authority | Local | Android requirement manifest and repair-selection projection |
| `android.synthesis.build` | Record the Android synthesis plan and build provenance (M4) | Android synthesis authority | Local | Android synthesis and build provenance projection |
| `android.project.scaffold` | Scaffold the real Android Gradle project workspace (M4b) | Android synthesis authority | Local | Android project workspace and revision projection |
| `agent.loop.run` | Drive the agent loop from synthesis through validated APK (M58) | Lifecycle authority | Local | Agent loop record and build projection |
| `worker.task.claim` | Claim a coordination task under an expiring lease (M8) | Worker coordination authority | Local | Worker lease and coordination projection |
| `worker.handoff.submit` | Submit a worker handoff for integration (M8) | Worker coordination authority | Local | Worker handoff projection |
| `worker.handoff.acknowledge` | Acknowledge a worker handoff outcome (M8) | Worker coordination authority | Local | Worker handoff acknowledgement projection |
| `worker.reconcile` | Reconcile a worker integration transactionally (M8) | Worker coordination authority | Local | Transactional integration checkpoint projection |
| `worker.step` | Execute one worker stage with declared capability and evidence (M5) | Worker execution authority | Local | Single-worker stage and evidence projection |

The lifecycle commands additionally accept the UI-level aliases `PauseTask`, `CancelTask`, `ResumeTask`, and `SubmitInstruction` (same authority, transaction domain, and projection effect as their canonical forms). The registry above is the complete set of thirty command kinds admitted by the authenticated boundary; commands not listed are rejected before a domain transaction begins.

For `artifact.export`, source/workspace access and deployment delivery are distinct branches. The deployment branch requires a verified declared artifact, an immutable `PackagingProfile`, `deploymentDelivery` consistent with that profile, and `destinationKind: LOCAL_WINDOWS_FILESYSTEM`; external deployment destinations are rejected. The source-access branch may produce a user-approved workspace, ZIP, or Git export, but it cannot create deployment evidence or completion. Unknown commands, commands missing a schema or authority, and commands outside the authenticated project scope are rejected before a domain transaction begins.

### 76.2 Response and error envelopes

```text
UIResponseEnvelope
- responseId
- commandId
- correlationId
- causationId
- projectId
- taskIdOptional
- status: ACCEPTED | COMPLETED | REJECTED | DUPLICATE | STALE | CANCELLED | FAILED
- resultSchemaRefOptional
- resultRefOptional
- projectionSnapshotRefOptional
- eventRangeOptional
- authorityDecisionRef
- diagnosticRefOptional
- createdAt
```

```text
UIErrorEnvelope
- errorId
- commandId
- correlationId
- causationId
- code
- category: AUTHENTICATION | AUTHORIZATION | SCOPE | VALIDATION |
            STALE_PROJECTION | IDEMPOTENCY | NOT_FOUND | CONFLICT |
            ENVIRONMENT | PROVIDER | DEVICE | TIMEOUT | CANCELLATION |
            UNAVAILABLE | INTERNAL
- safeMessage
- retryable
- retryAfterOptional
- recoveryActionOptional
- diagnosticRef
- authorityDecisionRef
- sensitiveDataOmitted: boolean
- createdAt
```

Raw stack traces, API keys, credentials, private reasoning, and unrestricted command output remain in protected diagnostic artifacts referenced by `diagnosticRef`; they are never copied into the UI error message.

### 76.3 Subscription, replay, and snapshot cutover

```text
EventSubscription
- subscriptionId
- connectionId
- projectId
- taskIdOptional
- fromEventSequence
- snapshotRevisionOptional
- requestedProjectionKinds
- acknowledgedEventSequence
- heartbeatInterval
- maxBatchSize
- backpressurePolicy
- status: REQUESTED | ACTIVE | PAUSED | GAP | CLOSED
```

The backend authenticates the subscription, returns a `ProjectionSnapshot`, then replays events strictly after the snapshot cursor. The UI acknowledges the highest contiguous sequence. Duplicate acknowledgements are harmless. A sequence gap, incompatible schema, slow consumer, supervisor restart, or retention boundary pauses projection advancement and returns a typed recovery response; the UI cannot fill the gap from local state. Snapshot cutover is atomic from the reducer’s perspective: either the snapshot and its cursor are accepted together or neither advances the projection.

### 76.4 Frontend/backend ownership and optimistic state

The frontend application contains a ViewModel or presentation controller, an API/IPC client, and a projection reducer. The control plane contains command handlers, domain use cases, deterministic authorities, repositories, transaction managers, event persistence, and read-model projectors. A repository maps domain records to SQLite and never returns raw database rows as UI state.

The transaction owner is the backend use-case handler. Local task, checkpoint, provider, and project changes use the local transaction domain; emulator installation, launch, and observation use the device transaction domain; provider or other externally visible calls use the external-effect domain with idempotency and reconciliation. The UI may show a pending command, but authoritative state advances only after a durable response or replayed event.

### 76.5 Android service-integration adapter

A generated Android application that uses a supporting API, authentication service, or datastore must declare `AndroidServiceIntegration` with `requestSchemaRef`, `responseSchemaRef`, `errorSchemaRef`, `authState`, `credentialReference`, `baseEndpointIdentity`, `datastoreOwner`, `offlinePolicy`, `retryPolicy`, `timeoutPolicy`, `idempotencyPolicy`, `tokenRefreshPolicy`, `privacyPolicy`, `networkPolicy`, and functional scenario IDs. Its generated API client and adapter are separate from Nirman’s desktop IPC client. Android integration failures become application evidence or declared blockers; they cannot mutate Nirman’s control-plane authority.

### 76.6 Acceptance criteria

The protocol is accepted only when fixtures prove authenticated command admission, project-scope rejection, idempotent duplicate handling, stale projection rejection, typed error rendering, cancellation, timeout, reconnect, snapshot cutover, event-gap recovery, backpressure, SQLite transaction ownership, projection reconstruction, and generated Android service error normalization.

## 77. Background Continuity Contract
**ContractId:** `CONTRACT.RUNTIME.BACKGROUND_CONTINUITY`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.BACKGROUND_CONTINUITY` (see §67.8)

This contract makes background autonomy explicit across user-interface closure, UI restart, supervisor restart, host reboot, sleep or hibernation, shutdown, Android emulator session loss, and provider or network outage. It centralizes continuity truth that is otherwise distributed across the supervisor, event ledger, lease, checkpoint, recovery, and reconnect contracts. `BackgroundContinuityState` is an orthogonal interruption and availability substate; it does not replace `ProductLifecycleState`, does not own `CompletionDecision`, and does not create a second recovery or completion authority. It does not make the model an authority and it does not permit progress to be inferred from elapsed time.

The existing product lifecycle remains authoritative for `Created`, `Planning`, `Recovering`, `Packaging`, `Completed`, cancellation, and terminal failure. Continuity records whether that lifecycle can currently advance and how it must recover. `BackgroundContinuityState.COMPLETED` is permitted only as a derived mirror after the existing completion authority commits `CompletionDecision=COMPLETED`; continuity alone can never complete a task.

### 77.1.1 Orthogonal continuity dimensions and aggregate precedence
```text
ContinuityDimensions
- uiConnectionState: CONNECTED | DISCONNECTED
- hostState: ONLINE | SUSPENDED | OFFLINE | RECOVERING
- deviceAvailabilityState: AVAILABLE | UNAVAILABLE | REATTACHING
- providerAvailabilityState: AVAILABLE | DEGRADED | UNAVAILABLE | USER_REQUIRED
- leaseState: HELD | EXPIRED | FENCED | REACQUIRING
- reconciliationState: NOT_REQUIRED | REQUIRED | IN_PROGRESS | RESOLVED | BLOCKED
```

The dimensions change independently. The displayed aggregate is deterministic and cannot hide a stronger condition. Aggregate precedence is:

```text
SAFELY_FAILED
> USER_REQUIRED
> RECONCILING
> RECOVERING
> HOST_OFFLINE
> HOST_SUSPENDED
> EMULATOR_UNAVAILABLE
> PROVIDER_UNAVAILABLE
> UI_DISCONNECTED
> ACTIVE_BACKGROUND
> COMPLETED (only when ProductLifecycleState and CompletionDecision agree)
```

A lower-precedence condition may remain recorded while a higher-precedence condition is active. Clearing one condition recomputes the aggregate from all current dimensions; it never blindly returns to `ACTIVE_BACKGROUND`.

### 77.1 Canonical continuity record and state machine
```text
BackgroundContinuityRecord
- continuityId
- projectId
- taskId
- branchId
- lastDurableEventId
- lastCheckpointId
- supervisorInstanceId
- hostSessionId
- deviceSessionId
- providerSessionId
- productLifecycleStateRef
- continuityDimensions
- aggregateState: ACTIVE_BACKGROUND | UI_DISCONNECTED | HOST_SUSPENDED |
                  HOST_OFFLINE | EMULATOR_UNAVAILABLE | PROVIDER_UNAVAILABLE |
                  RECOVERING | RECONCILING | USER_REQUIRED | SAFELY_FAILED |
                  COMPLETED
- interruptionCause
- resumeEligibility: ELIGIBLE | WAIT_FOR_HOST | WAIT_FOR_EMULATOR |
                     WAIT_FOR_PROVIDER | RECONCILE_REQUIRED | USER_REQUIRED |
                     NOT_ELIGIBLE
- requiredRecoveryActions
- leaseReference
- fencingToken
- transitionEventId
- authorityDecisionId
- checkpointReference
- reconciliationReference
- lastKnownGoodReference
- evidenceStatus
- evidenceReferences
- stateVersion
- updatedAt
```

The continuity transition rules are:

```text
Any aggregate state
  → dimension update
  → recompute aggregate by precedence
  → RECOVERING when resumeEligibility permits recovery
  → RECONCILING when an effect, lease, emulator session, or provider response is unknown
  → ACTIVE_BACKGROUND only after all required dimensions are healthy and reconciliation is resolved
  → USER_REQUIRED or SAFELY_FAILED when deterministic authorities cannot safely continue

ProductLifecycleState=Packaging
  + CompletionDecision=COMPLETED
  → aggregateState=COMPLETED
```

A continuity transition cannot directly change product lifecycle, completion, artifact, evidence, or promotion truth. It emits a transition event and a typed request to the existing lifecycle or recovery authority.

`COMPLETED` is reachable only through the existing evidence, validation, artifact, signing, preview, and completion authorities. `USER_REQUIRED` is reserved for an actual policy, credential, permission, or product decision that cannot be resolved from declared authority; it is not a timer-based escalation. `SAFELY_FAILED` preserves the last checkpoint, diagnostics, leases, fencing state, and evidence gap.

### 77.2 Interruption and recovery rules
UI closure or UI crash disconnects presentation only; it MUST NOT cancel eligible autonomous work. Reconnect reconstructs the UI from a cursor-atomic snapshot and durable event replay. Supervisor restart, host reboot, or process replacement requires lease fencing, checkpoint reload, descendant reconciliation, and duplicate-effect prevention before resuming. Sleep, hibernation, or shutdown records the last durable state and resumes only after host identity and required tools are revalidated. Device loss invalidates device-bound observations and waits for a new emulator session or records an honest unavailable result. Provider or network outage uses bounded retry and provider operationality rules; it never converts an unobserved model response into a successful action.

An unknown outcome remains `RECONCILING` until the authoritative ledger, process supervisor, emulator session, provider operation, or external-effect record resolves it. A retry is permitted only after idempotency and fencing checks. Late events from an old supervisor, device, branch, provider session, or lease cannot advance the current continuity state.

### 77.3 Authority, projection, evidence, and acceptance
The canonical authority mapping is: the existing supervisor/process-supervision authority owns `hostState`; `WorkspaceLeaseManager` and its lease/fencing authority own `leaseState`; the existing `RecoveryAuthority` owns recovery and reconciliation transitions; the existing device-session/device-operation authority owns `deviceAvailabilityState`; and the existing integration/provider operationality authority owns `providerAvailabilityState`. `SupervisorAuthority`, `LeaseAuthority`, `DeviceAuthority`, and `ProviderOperationalityAuthority` are aliases only and are not new authorities. The frontend renders `BackgroundContinuityRecord` as a projection and cannot resume, mark complete, clear an outage, or suppress a user-required state.

Every continuity transition must reference the owning canonical authority, its decision ID, the prior and next dimension values, and the resulting aggregate state. No continuity authority can override `LifecycleAuthority`, `PolicyAuthority`, `EvidenceAuthority`, `ArtifactAuthority`, `PreviewPromotionGate`, or `CompletionDecision`.

Every state transition records the causation event, prior and next state, authority decision, checkpoint or reconciliation reference, recovery action, and evidence status. The preview panel may show a truthful continuity label such as disconnected, recovering, stale, unavailable, or safely failed, but it cannot show active verified progress while the underlying state is suspended, offline, unreconciled, or invalidated. `BackgroundContinuityRecord` is an explicit field of the authoritative `ProjectionSnapshot`; its transition events are replayed through the same cursor, state-version, branch, session, and fencing checks as other control-plane events. The event crosswalk is: `UI_DISCONNECTED` updates only UI connection state; host suspend/offline/restart events update host state; emulator session loss/reattachment updates emulator availability and invalidates device-bound preview/evidence; provider operationality events update provider availability; checkpoint/reconciliation events update recovery and reconciliation state; and no continuity event directly writes completion, promotion, or verification truth. `task.resume` and recovery results return the resulting continuity transition reference, while `task.cancel` records the product-lifecycle decision separately.

Acceptance requires executable fixtures for UI closure and reconnect, supervisor restart, host reboot, sleep or hibernation, shutdown recovery, emulator session loss and reattachment, provider/network outage, stale-event rejection, unknown-outcome reconciliation, lease fencing, checkpoint resume, safe failure, and preservation of last-known-good evidence.

## 78. APK Export Provenance Contract
**ContractId:** `CONTRACT.RUNTIME.APK_EXPORT`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.APK_EXPORT` (see §67.8)

This contract governs local deployment delivery of a verified Android artifact. It specializes the integration-boundary and artifact contracts without creating a second signing, validation, promotion, or completion authority. Source/workspace, ZIP, and Git access remain available, but they are explicitly classified as `SOURCE_ACCESS_ONLY` and cannot satisfy deployment delivery.

### 78.1 Deployment admission
A deployment export is admitted only when an immutable `PackagingProfile` is identified by `packagingProfileId`, declares the artifact kind, the artifact has passed the required build, signing, validation, and promotion authorities, and the destination is `LOCAL_WINDOWS_FILESYSTEM`. The required local deliverable is an installable APK. AAB is optional only when the profile is `APK_AND_AAB`; it is never implied by source export or by a generic artifact request. External deployment destinations are rejected in the current product scope.

### 78.2 Canonical provenance and lifecycle
`ExportVerificationRecord` in TA §74.3 is the canonical durable record. For an APK deployment it is exposed as an `APKExportRecord` view containing `artifactKind`, `packagingProfileId`, source revision, checkpoint, source and destination file identities, request fingerprint, idempotency key, signing identity binding, validation decision, promotion decision, reconciliation reference, source and destination hashes, byte count, copy state, `deploymentDelivery: REQUIRED_APK`, `destinationKind: LOCAL_WINDOWS_FILESYSTEM`, post-copy verification, failure evidence, evidence references, and timestamp. The lifecycle is `REQUESTED → COPYING → COPIED → UNKNOWN → RECONCILING → VERIFIED` or `FAILED | BLOCKED`; interrupted or unknown copies remain durable and must be reconciled before retry.

### 78.3 Source-access separation and completion
`SOURCE_ACCESS_ONLY` may produce a user-approved workspace, ZIP, or Git export, but it cannot create deployment evidence, satisfy the required APK gate, or advance Android completion. Deployment completion requires source/destination identity and hash equality, approved destination scope, durable post-copy verification, matching packaging-profile, artifact, signing, validation, promotion, and evidence references, and a resolved `reconciliationReference` whenever the copy entered `UNKNOWN` or `RECONCILING`. Export success does not independently prove preview currency, integration functionality, runtime integrity, or user-goal completion.

## 79. Platform and Target Environment Contract
**ContractId:** `CONTRACT.RUNTIME.PLATFORM_CAPABILITY`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.PLATFORM_CAPABILITY` (see §67.8)

This contract separates the machine where work is performed from the machine the work must prove. It governs every artifact Nirman builds or validates — including Nirman's own Windows desktop host (C#/.NET + WinUI 3 + Windows App SDK + Rust/Tokio, Windows installer) and the generated Android application — and every capability claim, gate, or certification derived from that work. It extends the environment prerequisite classification of §52.9 and the tool diagnostics of §9.2 with an explicit host/target dimension; it does not replace them and creates no new runtime authority.

### 79.1 The four-state invariant

```text
HOST ENVIRONMENT     = the machine on which a command executes
TARGET PLATFORM      = the platform the artifact or capability is declared for
VALIDATION PLATFORM  = the platform on which runtime behavior is observed
CERTIFICATION STATUS = the gate result over accumulated, bound evidence
```

These are distinct state values and MUST never be collapsed into one `BUILD=SUCCESS`, one completion claim, or one capability status. A task running on a Linux x64 host for a Windows x64 target MUST be represented, after preflight, as:

```text
host_platform:             linux x86_64
target_platform:           windows x86_64
cross_compilation:         AVAILABLE   (only when toolchain preflight proves it)
native_target_execution:   UNAVAILABLE
target_runtime_validation: USER_REQUIRED or UNAVAILABLE
certification:             cannot complete without target-platform runtime evidence
```

A worker, model, skill, or report that merges these states into a single result fails this contract and every gate that consumes it.

### 79.2 Environment Capability Contract

Every environment preflight MUST produce a durable `EnvironmentCapabilityRecord` (schema: TA §84.1) before a task commits to a build or validation path:

```text
EnvironmentCapabilityRecord
- environment_id
- host_platform
- host_architecture
- target_platform
- target_architecture
- shell
- compiler, linker, sdk, runtime, build_tools, installer_tools
- native_dependencies
- tool_versions
- environment_fingerprint
- capability_results
- repair_attempts
- required_user_actions
- runtime_validation_available
- cross_compilation_available
- evidence_ids
```

Host and target are explicit fields of the record. No worker, skill, or model may infer host or target platform from `uname` output, toolchain heuristics, directory layout, or conversation context; the planner and the evidence authority consume only the recorded values. Tool and dependency state inside the record uses the §9.2 diagnostic vocabulary (`installed`, `missing`, `outdated`, `misconfigured`, `inaccessible`); platform capability state uses the §52.9 classification (`AVAILABLE`, `REPAIRABLE`, `USER_REQUIRED`, `UNAVAILABLE`).

### 79.3 Platform Capability Matrix

The runtime maintains a canonical platform capability matrix (`PlatformCapabilityEntry` in TA §84.1) mapping (host platform, capability) to an expected result class. The matrix is a prior for preflight, not a truth source: the environment preflight observes the actual environment, and the observed classification wins. Cells that depend on the concrete environment MUST be recorded as `environment_dependent` and MUST be classified from observation at preflight time. The matrix MUST NOT hard-code a tool or capability as universally unavailable on a host platform when an authorized toolchain can make it available — for example, Windows cross-compilation from Linux with a proven Rust target, linker, and Windows SDK is `environment_dependent`, not a fixed `unavailable_by_platform`.

At minimum the matrix covers, for each host platform: source compilation; dependency installation; static analysis; host-native test execution; cross-compilation to each declared target; target installer generation; artifact inspection; target native execution; target-specific runtime facilities (for the Windows host target: ConPTY, Job Objects, restricted tokens, ACL workspaces, Credential Manager/DPAPI, native IPC); process supervision and recovery validation; and emulator-dependent validation (Android Nirman-managed local Android emulator, per TA §49 and §50).

### 79.4 Environment Capability Resolution

Resolution is capability-driven, not model-driven:

```text
Task declares required capabilities
        ↓
ToolCapabilityGraph maps capabilities → required tools + environment prerequisites
        ↓
EnvironmentCapabilityPlanner classifies each prerequisite against the
observed EnvironmentCapabilityRecord
        ↓
AVAILABLE | REPAIRABLE | USER_REQUIRED | UNAVAILABLE
        ↓
TaskGraphCompiler schedules work only inside the admitted capability set
```

A model may propose a build, repair, or validation plan. It cannot set, raise, or waive a capability classification. A classification that is not backed by an observed preflight, by a successful repair executed through the normal policy/transaction path, or by an explicit user action is a contract violation (CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION).

When a required capability is absent, the planner MUST split the work rather than block everything: independent implementation, static analysis, host-native tests, cross-build, and artifact inspection continue, while the blocked capability becomes a durable `USER_REQUIRED` or `UNAVAILABLE` node with a stated reason, a resume condition, and the two lists defined in §79.11.

### 79.5 Cross-Compilation Policy and Build Gates

Cross-compilation is permitted when the toolchain preflight proves the required target toolchain (target Rust triple, linker, Windows SDK or equivalent, bundler, installer toolchain). Cross-compilation is **artifact production only**: it establishes that a target-platform artifact can be produced. It does not establish that the artifact runs, that target-specific behavior works, or that any runtime capability is validated.

Build stages and runtime stages are separate evidence gates. A target-platform build MUST pass each stage with its own evidence before the next stage is admitted:

```text
Source
  → Compile
  → Target build
  → Bundle
  → Artifact inspection
  → Install            (target host)
  → Launch             (target host)
  → Runtime validation (target host)
  → Platform-specific validation (target host)
  → Recovery validation (target host)
  → Certification
```

Stages executed on the host platform (compile, target build, bundle, artifact inspection) produce host-platform evidence. Stages that require the target platform (install through certification) produce target-platform evidence only when observed on a matching `ValidationEnvironment` (§79.8). A cross-built Windows installer produced on a Linux host may therefore close `Artifact inspection` and nothing beyond it: `Native launch`, `ConPTY`, `Job Objects`, `native IPC`, `recovery`, and `Windows certification` remain `UNAVAILABLE` or `USER_REQUIRED`, and the honest aggregate status is `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS`, never `SUPPORTED`.

### 79.6 Native Runtime Validation Policy

A platform runtime capability may be claimed only with authoritative observation from the matching target platform. Each capability's matrix entry declares its required evidence; for the Windows host target this includes at minimum a Windows host fingerprint, a process-launch observation with executable path and process identity, runtime output, IPC observation, the required Windows API behavior (ConPTY, Job Objects, restricted tokens, Credential Manager/DPAPI, as applicable), and recovery behavior.

```text
no matching-platform observation
        ↓
no evidence
        ↓
no capability promotion
        ↓
no certification
```

Evidence that does not bind to the `EnvironmentCapabilityRecord` fingerprint, target platform, and source revision that produced it is invalid (CLAUSE.PLATFORM.EVIDENCE_ENV_BINDING, §5.7.4). A model statement, worker report, or simulation asserting target-runtime behavior without such bound evidence is a rejected completion claim; the completion evaluator MUST reject it and the rejection is durable.

### 79.7 Platform-Specific Build and Validation Skills

Platform behavior is carried by dedicated implementation skills, not by a generic AI-coding skill. A `UniversalCodingSkill` or equivalent catch-all prompt is prohibited: it cannot encode the host/target distinction and it cannot be evidence.

Each platform skill is a `SkillPackage` (§23) declaring `requiredTools`, `requiredCapabilities`, `triggerConditions`, `permissionRequests`, `inputSchema`, `outputSchema`, and its fixture set. Skills remain permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every execution they describe still passes through ToolBroker and PolicyAuthority, and a skill whose `requiredCapabilities` resolve to `UNAVAILABLE` or `USER_REQUIRED` MUST NOT execute the gated steps and MUST report the blocked state. The v1 platform skill set:

| Skill | Scope | Gated by |
|---|---|---|
| `environment-preflight` | Identify host and target; inspect toolchain, SDKs, runtimes, native dependencies; classify executable and validation capabilities; produce the environment fingerprint | runs before implementation; host tools |
| `environment-repair` | Authorized repairs: missing tool, wrong tool version, missing target, broken PATH, missing SDK or dependency, incorrect configuration | repair capability + policy approval through the normal transaction path |
| `windows-desktop-build` | C#/.NET / WinUI 3 / Windows App SDK / XAML + Rust control-plane integration; Nirman.exe packaging, NirmanSupervisor.exe packaging, named-pipe SupervisorConnection, native Windows runtime integration | cross-compilation capability or Windows host; **never claims runtime validation** |
| `windows-runtime-validation` | Nirman.exe and NirmanSupervisor startup, IPC, ConPTY, process supervision, Job Objects, isolation, restart/recovery, credential storage, installer/uninstaller behavior | `target_platform = windows` AND `native_execution = AVAILABLE`; otherwise `USER_REQUIRED`/`UNAVAILABLE`, never a simulated pass |
| `cross-platform-build-diagnostics` | Determine what can be cross-built, which artifacts can be produced, and which validation evidence necessarily remains missing for a host→target pair | host toolchain observation |
| `android-toolchain` | Node, package manager, Java, Gradle, Android SDK, platform tools, emulator, native dependencies, signing | Android toolchain authority (TA §49); independent of host-target build capability |

A skill MUST NOT hard-code a capability as unavailable on a host platform; it declares the required capability and consumes the preflight classification.

### 79.8 Validation Environment as a First-Class Resource

Native target validation consumes a `ValidationEnvironment` (schema: TA §84.1) as a first-class resource:

```text
ValidationEnvironment
- environment_id
- platform
- architecture
- toolchain
- runtime
- available_tools
- available_devices
- isolation_profile
- network_policy
- fingerprint
- health
- lease
```

A target validation task reserves the matching validation environment before execution:

```text
target validation task
  → ValidationEnvironment reservation (durable lease via WorkspaceLeaseManager)
  → tool sessions (ToolSessionRegistry)
  → target processes
  → observations
  → bound evidence
```

No native-validation or certification claim exists without the lease and the observation set it produced. A lease, fingerprint, toolchain, device, or policy change invalidates the evidence produced under it (CLAUSE.PLATFORM.VALIDATION_ENV_RESERVATION).

### 79.9 No Substitute Execution Targets

A container, virtual machine, WSL, Windows Sandbox, remote build farm, or simulated environment MUST NOT substitute for the declared target platform in native runtime validation, and no such environment may be introduced as a generated product target (BS §2, ADR-001). Nirman's correct behavior when the target validation environment is absent is to recognize the absence, classify the capability `UNAVAILABLE` or `USER_REQUIRED`, continue independent work, and wait or escalate truthfully.

### 79.10 Host Development and Target Certification Are Different Lanes

Development on a non-target host is permitted and expected where the toolchain exists: reading and editing source, static analysis, host-native Rust and frontend tests, cross-build of the target artifact, artifact inspection, documentation verification, and platform-independent fixtures. The target environment then performs install, launch, platform-specific runtime tests, process supervision and recovery tests, and installer tests. Certification combines evidence: host-platform evidence plus target-platform evidence, each bound to its own record. Neither lane may report the other lane's result.

### 79.11 Unavailable Validation Environment as a Hidden Human Dependency

An absent or unrecoverable target validation environment is a hidden human dependency in the sense of §69.10. An unattended task MUST resolve it by exactly one of: (a) an explicitly authorized automatic action that provisions or reattaches the environment, (b) a durable `USER_REQUIRED` decision naming the required environment and the reason, or (c) a truthful blocked state. The pending node MUST state both lists:

```text
WAITING / USER_REQUIRED
Reason: native Windows runtime environment required for target validation.
Can continue: cross-build and platform-independent checks.
Cannot continue: Windows runtime certification.
```

Silent continuation that skips the gate, or a claim that the gate passed, is a certification failure.

### 79.12 WorkerContract Platform Requirements

`WorkerContract` (registry: TA §36.1) gains platform requirement fields, extended in TA §84.1: `requiredHostPlatforms`, `requiredTargetPlatforms`, `requiredArchitectures`, `requiredCapabilities`, `requiredSkills`, `requiredToolchain`, `requiredValidationEnvironment`, `crossCompilationAllowed`, `nativeExecutionRequired`, and `evidenceRequirements`. A worker whose contract sets `nativeExecutionRequired = true` for a platform that is `UNAVAILABLE` in the current `EnvironmentCapabilityRecord` MUST NOT be scheduled for the gated steps; the scheduler places the node in the §79.11 blocked state. A worker running on a host platform outside `requiredHostPlatforms` cannot claim host-specific results for that contract.

### 79.13 Hallucination-Prevention Fixtures

The following are mandatory runtime-certification fixtures (test family `TEST-PLAT-001`, evidence `EV-PLAT-001`; implementation: TA §84.5, M118):

| Fixture | Setup | Required behavior |
|---|---|---|
| A — host mismatch | host = Linux, target = Windows; task: "build and validate" | cross-build may execute; native Windows validation MUST NOT be claimed; the blocked node records `USER_REQUIRED`/`UNAVAILABLE` with the two §79.11 lists |
| B — successful cross-build | a Windows `.exe`/installer is produced from a non-Windows host | `ARTIFACT_BUILD = VERIFIED` and `WINDOWS_RUNTIME = UNVERIFIED` are both recorded; the aggregate status is `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS`, never `SUPPORTED` |
| C — fake completion | a model or worker reports "Windows runtime tests passed" with no target observation | the completion claim is durably rejected by the completion evaluator and the rejection cites the missing evidence |
| D — stale target evidence | target-platform evidence exists, then the source revision, toolchain identity, or environment fingerprint changes | the prior target evidence is `INVALIDATED`; the certification gate re-closes until re-validation on the target platform |

### 79.14 Filesystem path-length capability

Workspace roots MUST be allocated under a short deterministic prefix (pattern `C:\<root>\p\<8-char-id>\`), never under the user profile, Desktop, or a OneDrive-synced path. `EnvironmentCapabilityPlanner` MUST probe effective maximum path length at preflight and classify `AVAILABLE` / `REPAIRABLE` / `USER_REQUIRED` / `UNAVAILABLE` per CLAUSE.PLATFORM.DETERMINISTIC_CLASSIFICATION. Enabling the long-paths registry policy requires elevation and a reboot; it is `USER_REQUIRED` and MUST NOT be performed silently. Path-length exhaustion MUST classify as a distinct failure with a stated remedy, not a generic build failure. Route it through the existing `AndroidRepairRegistry` (ADR-075). The probe result MUST be part of `environment_fingerprint`, so a change invalidates prior evidence per CLAUSE.PLATFORM.EVIDENCE_ENV_BINDING.

### 79.15 Host security-software interference

Preflight MUST detect active real-time scanning over the workspace root, Gradle home, and toolchain directory, and record it in `EnvironmentCapabilityRecord`. Requesting exclusions is `USER_REQUIRED` and MUST be an explicit consented action displaying the exact paths. Nirman MUST NOT modify host security policy autonomously — this is a privileged command per BS §9.3. Absent exclusions the classification is `DEGRADED` with a stated performance consequence: not a failure, and not silence. Scanner file-lock contention MUST be distinguishable from a compilation error and separately retryable, since retrying the wrong class wastes budget.

### 79.16 Hypervisor availability and arbitration

Preflight MUST classify firmware virtualization enabled, hypervisor platform present, and conflicting hypervisor consumers, recording each in `EnvironmentCapabilityRecord`. Without acceleration, emulator-backed validation is `UNAVAILABLE`. Per CLAUSE.PLATFORM.NO_RUNTIME_INFERENCE the completion evaluator MUST then report at most `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS` and MUST NOT substitute a successful build for runtime validation. A Nirman-managed local Android emulator is the documented alternative path and MUST be offered before the work is blocked, consistent with BS §79.4 work splitting. Hypervisor-contention start failure MUST be its own classification with a plain-language remedy naming the conflicting software. A container, VM, or WSL Android environment does NOT satisfy emulator validation (CLAUSE.PLATFORM.NO_SUBSTITUTE_TARGET).

---

## 80. Agent-Buildability Contract

**ContractId:** `CONTRACT.RUNTIME.AGENT_BUILDABILITY`
**Registry role:** authoritative definition of `CONTRACT.RUNTIME.AGENT_BUILDABILITY` (see BS §67.8)

**ContractId:** `CONTRACT.RUNTIME.INVARIANTS`
**ExtensionDeclaration:**
- authorityContractId: CONTRACT.RUNTIME.INVARIANTS
- authoritySection: §67
- extendingSection: §80
- extensionType: adds_clauses
- extendedClauses: CLAUSE.BUILDABILITY.NO_AGENT_HALLUCINATION, CLAUSE.BUILDABILITY.COMPLETE_PROCEDURES, CLAUSE.BUILDABILITY.EXPLICIT_DEFAULTS, CLAUSE.BUILDABILITY.DETERMINISTIC_DECISIONS
- nonOverriddenClauses: CLAUSE.INVARIANT.LEDGER_VERIFIABLE

This section defines the contract that every canonical document MUST be complete enough for an AI agent to build Nirman without hallucination, inference, or guessing. It resolves all ambiguities in the existing specification by providing concrete procedures, default values, decision criteria, and complete schemas.

### 80.1 The agent-buildability invariant

An AI agent building Nirman from these docs MUST NEVER have to:
1. Invent a schema field that is not defined
2. Guess a default value that is not specified
3. Choose between options without explicit decision criteria
4. Infer a procedure that is described only in vague terms
5. Create a prompt template that is not provided
6. Determine implementation sequencing without explicit ordering
7. Define a test fixture without a concrete fixture specification
8. Resolve a "should" without explicit criteria for when it applies

If any of these conditions occurs, the docs are incomplete and MUST be corrected.

### 80.2 "Should" resolution table

Every "should" in the canonical documents is resolved here with explicit criteria:

| Section | "Should" statement | Resolution | Criteria |
|---|---|---|---|
| BS §3.1 | "should run locally whenever possible" | MUST run locally; cloud AI is the only exception | When cloud AI is configured, model calls go to cloud; all build/test/preview runs local |
| BS §3.2 | "should always be able to access" | MUST provide access | User can always access project dir, source, Git, config, artifacts |
| BS §3.3 | "should be able to see" | MUST display | Current task, plan, files changed, commands, test results, failure reasons |
| BS §3.4 | "should create a checkpoint" | MUST create checkpoint | Before every multi-file autonomous task |
| BS §3.5 | "should start with reliable synthesis" | MUST start with synthesis | First vertical slice MUST be synthesis loop |
| BS §4.1 | "should use minimal focused layout" | MUST use minimal layout | Layout MUST have: left nav, chat, file tree, workspace, bottom panel, toolbar |
| BS §4.2 | "should explain it is local" | MUST explain on first launch | First-run screen MUST explain local-first nature |
| BS §4.3 | "should produce structured response" | MUST produce structured response | Every response MUST have: Understanding, Plan, Files to change, Commands, Progress, Validation, Summary |
| BS §4.4 | "should support emulator preview first" | MUST support emulator first | Emulator is the primary preview path |
| BS §4.5 | "should include full code editor" | MUST include editor | Editor MUST have: syntax highlighting, search, tabs, formatting, diagnostics |
| BS §5.1 | "should focus exclusively on Android" | MUST focus on Android | Generated target is Android only |
| BS §5.2 | "should be designed to build all categories" | MUST build all categories | Technology resolver MUST support all listed categories |
| BS §6.1 | "should use C#/.NET + WinUI 3" | MUST use C#/.NET + WinUI 3 | Desktop shell is C#/.NET + WinUI 3 |
| BS §6.2 | "should contain chat workspace" | MUST contain all listed areas | Frontend MUST have all listed areas |
| BS §6.3 | "should never assume tool exists" | MUST verify before invoke | Runtime MUST verify executable before invoking |
| BS §6.4 | "should be stateful task engine" | MUST be stateful | Orchestrator MUST maintain all listed state categories |
| BS §6.5 | "should not send entire project" | MUST use index | Context selector MUST use project index |
| BS §6.6 | "should create checkpoint" | MUST create checkpoint | Before multi-file changes |
| BS §7.1 | "should interact through structured tools" | MUST use structured tools | Model MUST use only defined tools |
| BS §7.2 | "should classify failures" | MUST classify | Agent MUST classify every failure |
| BS §7.3 | "should stop and explain" | MUST stop and explain | After repeated failure, MUST stop and explain |
| BS §8.1 | "should allow users to configure" | MUST allow configuration | Users MUST be able to configure all listed fields |
| BS §8.2 | "should return normalized result" | MUST return normalized result | Adapter MUST return normalized result |
| BS §8.3 | "should clearly communicate" | MUST communicate | MUST tell user when content is sent to cloud |
| BS §8.4 | "should mask keys" | MUST mask keys | Keys MUST be masked in all output |
| BS §9.1 | "should execute locally" | MUST execute locally | Generated apps run locally |
| BS §9.2 | "should detect presence and versions" | MUST detect | Runtime MUST detect all listed tools |
| BS §9.3 | "should implement command timeouts" | MUST implement | Runtime MUST implement all listed controls |
| BS §9.4 | "should be visible" | MUST be visible | Network access MUST be visible |
| BS §10.1 | "should read/write only inside workspace" | MUST stay inside workspace | Default: workspace-only access |
| BS §10.2 | "should validate executable" | MUST validate | Command runner MUST validate before execution |
| BS §10.3 | "should show which dependencies" | MUST show | MUST show dependencies before install |
| BS §10.4 | "should detect likely secrets" | MUST detect | MUST detect and protect secrets |
| BS §10.5 | "should require explicit confirmation" | MUST require confirmation | Release/signing/publishing requires confirmation |
| BS §10.6 | "should record task ID" | MUST record | Activity log MUST record all listed fields |
| BS §12.1 | "should be able to create" | MUST support creation | User MUST be able to create by describing |
| BS §12.2 | "should be able to describe" | MUST support description | User MUST be able to describe new app |
| BS §12.3 | "should be able to inspect" | MUST support inspection | User MUST be able to inspect files |
| BS §12.4 | "should start local preview" | MUST start preview | MUST start preview for supported projects |
| BS §12.5 | "should be able to create/edit" | MUST support provider CRUD | User MUST manage provider profiles |
| BS §12.6 | "should be able to export" | MUST support export | User MUST be able to export source/APK |
| BS §12.7 | "should be able to inspect" | MUST support diagnostics | User MUST be able to inspect diagnostics |

### 80.3 Default values for all configurable parameters

Every "configurable" parameter in the specification has a default value defined here. An agent MUST use these defaults unless the user explicitly overrides them.

| Parameter | Default | Range | Override |
|---|---|---|---|
| Worker stale threshold | 60 seconds | 30-300 seconds | Per project |
| Worker heartbeat interval | 10 seconds | 5-30 seconds | Per project |
| Concurrent write-capable workers per task | 3 | 1-5 | Per project |
| Concurrent read-only workers per task | 5 | 1-10 | Per project |
| Total active workers | 8 | 4-16 | Per project |
| Default task wall-clock budget | 200 minutes | 30-1440 minutes | Per task |
| Default repair attempts per failure | 3 | 1-10 | Per task |
| Default task context budget | Provider-dependent | 16K-200K tokens | Per task |
| Default disk quota per task | 10 GB | 1-100 GB | Per project |
| Default token budget | Provider-dependent | 100K-10M tokens | Per task |
| Default request budget | 1000 requests | 100-10000 | Per task |
| Default duration budget | 200 minutes | 30-1440 minutes | Per task |
| Default cost cap | None (unlimited) | $0.01-$1000 | Per task |
| Checkpoint retention (recent) | 10 | 3-50 | Per project |
| Checkpoint retention (initial) | Always keep | N/A | Never deleted |
| Checkpoint retention (last known-good) | Always keep | N/A | Never deleted |
| Context compaction threshold | 80% of context limit | 60-95% | Per project |
| Context compaction minimum retention | 20% of context limit | 10-40% | Per project |
| Telemetry sampling interval | 30 seconds | 5-300 seconds | Per project |
| Retry budget (transient failures) | 3 | 1-10 | Per task |
| Retry backoff initial | 1 second | 0.1-10 seconds | Per project |
| Retry backoff max | 60 seconds | 10-300 seconds | Per project |
| Retry backoff multiplier | 2.0 | 1.1-3.0 | Per project |
| Deliberation diminishing return threshold | 3 passes | 2-10 passes | Per task |
| Deliberation max passes (NORMAL) | 1 | 1-3 | Per task |
| Deliberation max passes (EXTENDED) | 3 | 2-5 | Per task |
| Deliberation max passes (DEEP) | 5 | 3-10 | Per task |
| Deliberation max passes (EXHAUSTIVE) | 10 | 5-20 | Per task |
| Screenshot comparison threshold | 0.95 similarity | 0.80-0.99 | Per project |
| Visual diff threshold | 5% pixel diff | 1-20% | Per project |
| Uncertainty threshold (high risk) | 0.1 | 0.05-0.3 | Per task |
| Uncertainty threshold (medium risk) | 0.2 | 0.1-0.5 | Per task |
| Uncertainty threshold (low risk) | 0.4 | 0.2-0.7 | Per task |
| Stall detection window | 300 seconds | 60-1800 seconds | Per project |
| Stall detection min progress | 1 event | 0-5 events | Per project |
| Approval expiry | 24 hours | 1-168 hours | Per project |
| Notification cooldown | 60 seconds | 5-600 seconds | Per project |
| Log retention | 30 days | 7-365 days | Per project |
| Artifact retention | 90 days | 7-365 days | Per project |
| Session memory retention | Project lifetime | N/A | Until project deleted |
| Project memory retention | Project lifetime | N/A | Until project deleted |
| Runtime-improvement memory retention | 365 days | 30-3650 days | Per project |

### 80.4 Decision criteria for runtime choices

When the runtime has multiple options, it MUST choose using these explicit criteria:

#### 80.4.1 Recovery strategy selection

When a failure occurs, the runtime MUST select a recovery strategy using this ordered priority:

1. **Transient retry** — If the failure is transient (network timeout, rate limit, temporary lock), retry with exponential backoff
2. **Focused diagnostic** — If the failure is localized, spawn a diagnostic worker to isolate the cause
3. **Context refresh** — If the failure may be due to stale context, compact and refresh context
4. **Strategy change** — If the current strategy has failed twice, switch to a different strategy
5. **Worker role change** — If the current worker role is inappropriate, delegate to a different role
6. **Checkpoint restore** — If the failure is structural, restore last known-good checkpoint
7. **Model change** — If the model is incapable, route to a different model
8. **Specialist delegation** — If the failure is domain-specific, delegate to a specialist
9. **Isolated alternative** — If the current approach is fundamentally flawed, create an isolated alternative
10. **User escalation** — If all automated strategies are exhausted, escalate to user

The runtime MUST attempt each strategy in order. It MAY skip a strategy if the failure class is clearly incompatible with that strategy. The runtime MUST record which strategies were attempted and why each was selected or skipped.

#### 80.4.2 Worker selection

When selecting a worker for a task, the runtime MUST use this ordered criteria:

1. **Role match** — The worker's role MUST match the task type
2. **Availability** — The worker MUST be available (not at capacity)
3. **Capability** — The worker MUST have the required capabilities
4. **Model suitability** — The worker's model MUST be suitable for the task type
5. **Resource fit** — The worker MUST fit within the task's resource budget
6. **Historical performance** — Prefer workers with higher success rates for this task type
7. **Workspace isolation** — The worker MUST have an isolated workspace

#### 80.4.3 Model routing

When selecting a model for a task, the runtime MUST use this ordered criteria:

1. **Capability requirement** — The model MUST have the required capabilities (vision, reasoning, tool calling)
2. **Task type suitability** — The model MUST be suitable for the task type (planning, coding, visual)
3. **Context capacity** — The model MUST have sufficient context capacity for the task
4. **Cost efficiency** — Prefer lower-cost models when capability is equivalent
5. **Latency** — Prefer lower-latency models when capability is equivalent
6. **Historical performance** — Prefer models with higher success rates for this task type
7. **Provider health** — Prefer providers with better current health metrics

#### 80.4.4 Context compaction

When compacting context, the runtime MUST use this ordered priority for what to retain:

1. **Active constraints** — MUST never be evicted
2. **Locked decisions** — MUST never be evicted
3. **Current goal** — MUST never be evicted
4. **Current plan** — MUST never be evicted
5. **Active errors** — MUST never be evicted
6. **Recent evidence** — Retain from last 5 turns
7. **Recent file changes** — Retain from last 5 turns
8. **Recent commands** — Retain from last 3 turns
9. **Earlier context** — Summarize into structured summary
10. **Historical context** — Archive to cold storage

#### 80.4.5 Checkpoint selection

When selecting a checkpoint to restore, the runtime MUST use this ordered criteria:

1. **Last known-good** — The most recent checkpoint that passed all validation
2. **Task boundary** — The checkpoint at the start of the current task
3. **Pre-mutation** — The checkpoint immediately before the failing mutation
4. **Initial** — The initial project checkpoint (last resort)

### 80.5 Complete schema definitions

All schemas referenced in the specification are fully defined here. An agent MUST use these exact field definitions.

#### 80.5.1 AndroidTechnologyPlan

```text
AndroidTechnologyPlan
- planId: string (uuid)
- projectId: string (uuid)
- revision: string (hash)
- selectedLanguages: ("kotlin" | "java" | "typescript" | "javascript" | "cpp" | "c")[]
- uiSystem: ("jetpack_compose" | "android_views" | "react_native" | "expo" | "mixed")?
- nativeModules: string[] (Maven coordinates or npm package names)
- buildSystem: ("gradle_kotlin" | "gradle_groovy")?
- gradleVersion: string (semver)?
- agpVersion: string (semver)?
- kotlinVersion: string (semver)?
- compileSdk: integer?
- targetSdk: integer?
- minSdk: integer?
- ndkVersion: string (semver)?
- cmakeVersion: string (semver)?
- packageId: string (reverse-domain)?
- versionCode: integer?
- versionName: string (semver)?
- permissions: string[] (Android permission names)
- features: string[] (Android feature names)
- services: string[] (service class names)
- dependencies: string[] (Maven coordinates or npm package names)
- testFrameworks: string[] (e.g., "junit", "espresso", "compose_ui_test")
- rationale: string (human-readable explanation of technology choices)
- confidence: float (0.0-1.0)
- alternativesConsidered: { technology: string, rejectionReason: string }[]
- lockedAt: timestamp
- lockedBy: string (worker or authority ID)
```

#### 80.5.2 VisualSpecification

```text
VisualSpecification
- specId: string (uuid)
- projectId: string (uuid)
- revision: string (hash)
- sourceScreenshotRefs: string[] (screenshot IDs)
- screens: ScreenSpec[]
- colorSystem: ColorSystem?
- typography: TypographySpec?
- spacing: SpacingSpec?
- componentLibrary: string?
- interactionPatterns: string[]
- accessibilityRequirements: string[]
- uncertainty: { area: string, confidence: float, question: string }[]
- lockedAt: timestamp
- lockedBy: string (worker or authority ID)

ScreenSpec
- screenId: string (uuid)
- name: string
- route: string?
- components: ComponentSpec[]
- layout: LayoutSpec?
- interactions: InteractionSpec[]
- states: ScreenState[]

ComponentSpec
- componentId: string (uuid)
- type: string (e.g., "button", "text", "image", "list")
- label: string?
- position: { x: float, y: float, width: float, height: float }
- style: string (style reference)
- behavior: string?
- accessibilityLabel: string?

InteractionSpec
- interactionId: string (uuid)
- trigger: string (e.g., "tap", "swipe", "long_press")
- action: string (e.g., "navigate", "toggle", "submit")
- target: string (screen or component ID)

ScreenState
- stateId: string (uuid)
- name: string (e.g., "loading", "empty", "error", "loaded")
- conditions: string[]
- components: ComponentSpec[] (overrides for this state)
```

#### 80.5.3 AndroidApplicationContract

```text
AndroidApplicationContract
- contractId: string (uuid)
- projectId: string (uuid)
- revision: string (hash)
- displayName: string
- packageId: string (reverse-domain)
- namespace: string
- versionCode: integer
- versionName: string (semver)
- description: string
- brandingIntent: string?
- privacyClassification: ("public" | "internal" | "confidential" | "restricted")
- originalRequest: string
- screenshotRefs: string[]
- explicitConstraints: string[]
- inferredRequirements: string[]
- assumptions: string[]
- unresolvedAmbiguities: string[]
- features: FeatureModel[]
- screens: ScreenSpec[]
- dataModel: DataModel?
- integrations: IntegrationSpec[]
- technologyPlanRef: string (planId)
- validationModel: ValidationModel?
- artifactModel: ArtifactModel?
- lockedAt: timestamp
- lockedBy: string (worker or authority ID)

FeatureModel
- featureId: string (uuid)
- name: string
- description: string
- userStory: string
- dependencies: string[] (feature IDs)
- mandatory: boolean
- acceptanceTests: string[]
- affectedScreens: string[] (screen IDs)

DataModel
- entities: EntitySpec[]
- relationships: RelationshipSpec[]
- persistenceStrategy: ("room" | "sqlite" | "datastore" | "encrypted" | "network_cache" | "composed")
- migrationRules: string[]
- corruptionRecovery: string?
- seedDataPolicy: ("empty" | "fixture" | "none")
- encryptionRequirements: string?

IntegrationSpec
- integrationId: string (uuid)
- name: string
- kind: ("api" | "auth" | "notification" | "storage" | "camera" | "location" | "bluetooth" | "nfc" | "payment" | "maps" | "biometric")
- endpointIdentity: string?
- authState: ("not_required" | "configured" | "authenticated")
- credentialReference: string? (keychain ref)
- requestSchemaRef: string?
- responseSchemaRef: string?
- errorSchemaRef: string?
- offlinePolicy: string?
- retryPolicy: string?
- timeoutPolicy: string?
- idempotencyPolicy: string?
- privacyPolicy: string?
- networkPolicy: string?
- functionalScenarioIds: string[]
```

#### 80.5.4 TaskGraph

```text
TaskGraph
- graphId: string (uuid)
- projectId: string (uuid)
- sessionId: string (uuid)
- revision: string (hash)
- phases: TaskPhase[]
- dependencies: { fromPhase: string, toPhase: string }[]
- workers: WorkerAssignment[]
- completionConditions: string[]
- createdAt: timestamp
- updatedAt: timestamp
- lockedAt: timestamp
- lockedBy: string (worker or authority ID)

TaskPhase
- phaseId: string (uuid)
- name: string
- description: string
- order: integer
- status: ("pending" | "active" | "completed" | "failed" | "blocked")
- tasks: TaskNode[]
- entryCriteria: string[]
- exitCriteria: string[]

TaskNode
- taskId: string (uuid)
- phaseId: string (uuid)
- name: string
- description: string
- role: string (worker role)
- status: ("pending" | "active" | "completed" | "failed" | "blocked" | "waiting_approval")
- dependencies: string[] (task IDs)
- inputRefs: string[]
- outputRefs: string[]
- validationPlan: string?
- attemptCount: integer
- maxAttempts: integer
- failureFingerprint: string?
- assignedWorker: string? (worker ID)
- startedAt: timestamp?
- completedAt: timestamp?

WorkerAssignment
- assignmentId: string (uuid)
- workerId: string (uuid)
- taskId: string (uuid)
- role: string
- workspaceLease: string (lease ID)
- modelProfile: string (profile ID)
- status: ("assigned" | "active" | "completed" | "failed" | "released")
```

#### 80.5.5 ProviderProfile

```text
ProviderProfile
- id: string (uuid)
- label: string
- baseUrl: string (URL)
- keychainReference: string (credential ref, NOT the actual key)
- chatModelId: string
- visionModelId: string?
- embeddingModelId: string?
- capabilities: ("text" | "vision" | "structured_output" | "tool_calling" | "reasoning" | "embeddings")[]
- requestSettings: RequestSettings
- compatibilityMode: ("openai_compatible" | "anthropic_compatible")
- reasoningSupport: boolean
- reasoningEffortLevels: ("normal" | "extended" | "deep" | "exhaustive")[]
- maxReasoningTokens: integer?
- reasoningUsageReporting: ("reported" | "estimated" | "unavailable")
- contextCapacity: integer (tokens)
- status: ("configured" | "reachable" | "authenticated" | "degraded" | "unavailable")
- createdAt: timestamp
- updatedAt: timestamp

RequestSettings
- temperature: float (0.0-2.0, default 0.7)
- maxTokens: integer?
- timeoutSeconds: integer (default 120)
- retryPolicy: RetryPolicy?

RetryPolicy
- maxRetries: integer (default 3)
- initialBackoffSeconds: float (default 1.0)
- maxBackoffSeconds: float (default 60.0)
- backoffMultiplier: float (default 2.0)
```

### 80.6 Concrete test fixtures

Every test fixture referenced in the specification is defined here. An agent MUST implement these exact fixtures.

#### 80.6.1 FIX-PROG-01: Tip Calculator

```text
Fixture: FIX-PROG-01
Name: Tip Calculator
Prompt: "A tip calculator"
Primary stress: Pure UI and state
Acceptance criteria:
  1. App launches without runtime errors
  2. User can enter a bill amount
  3. User can select a tip percentage
  4. App calculates and displays the tip amount
  5. App calculates and displays the total amount
  6. All calculations are correct within 0.01 tolerance
  7. UI adapts to different screen sizes
  8. Dark theme is supported
  9. State survives configuration change (rotation)
  10. No hardcoded secrets in the generated code
```

#### 80.6.2 FIX-PROG-02: Todo List with Local Persistence

```text
Fixture: FIX-PROG-02
Name: Todo List with Local Persistence
Prompt: "A todo list with local persistence"
Primary stress: Local storage
Acceptance criteria:
  1. App launches without runtime errors
  2. User can add a todo item
  3. User can mark a todo item as complete
  4. User can delete a todo item
  5. Todo items persist across app restart
  6. Todo items survive process death
  7. Data is stored using Room or DataStore
  8. UI shows empty state when no todos exist
  9. UI adapts to different screen sizes
  10. Dark theme is supported
```

#### 80.6.3 FIX-PROG-03: Habit Tracker with Streaks and 8pm Reminders

```text
Fixture: FIX-PROG-03
Name: Habit Tracker with Streaks and 8pm Reminders
Prompt: "A habit tracker with streaks and 8pm reminders"
Primary stress: Scheduling and notifications
Acceptance criteria:
  1. App launches without runtime errors
  2. User can create a habit with a name
  3. User can mark a habit as complete for the day
  4. App calculates and displays the current streak
  5. App schedules a daily reminder at 8pm
  6. Notification is delivered at the scheduled time
  7. User can view habit history
  8. Streaks persist across app restart
  9. UI adapts to different screen sizes
  10. Dark theme is supported
```

#### 80.6.4 FIX-PROG-04: Weather App Using a Public REST API

```text
Fixture: FIX-PROG-04
Name: Weather App Using a Public REST API
Prompt: "A weather app using a public REST API"
Primary stress: Network and error states
Acceptance criteria:
  1. App launches without runtime errors
  2. App fetches weather data from a public API
  3. App displays current weather conditions
  4. App handles network errors gracefully
  5. App shows loading state while fetching
  6. App shows error state with retry action
  7. App caches data for offline viewing
  8. UI adapts to different screen sizes
  9. Dark theme is supported
  10. No hardcoded API keys in the generated code
```

#### 80.6.5 FIX-PROG-05: Note-Taking App with Search

```text
Fixture: FIX-PROG-05
Name: Note-Taking App with Search
Prompt: "A note-taking app with search"
Primary stress: Query and list state
Acceptance criteria:
  1. App launches without runtime errors
  2. User can create a note with title and body
  3. User can edit a note
  4. User can delete a note
  5. User can search notes by title or body content
  6. Search results update as the user types
  7. Notes persist across app restart
  8. UI shows empty state when no notes exist
  9. UI adapts to different screen sizes
  10. Dark theme is supported
```

#### 80.6.6 FIX-PROG-06: Photo Gallery Reading Device Storage

```text
Fixture: FIX-PROG-06
Name: Photo Gallery Reading Device Storage
Prompt: "A photo gallery reading device storage"
Primary stress: Runtime permissions
Acceptance criteria:
  1. App launches without runtime errors
  2. App requests storage permission
  3. App handles permission denial gracefully
  4. App displays photos from device storage
  5. User can view photos in full screen
  6. App handles empty gallery gracefully
  7. UI adapts to different screen sizes
  8. Dark theme is supported
  9. No photos are uploaded to any external service
  10. Permission rationale is clearly explained
```

#### 80.6.7 FIX-PROG-07: Pomodoro Timer with a Foreground Service

```text
Fixture: FIX-PROG-07
Name: Pomodoro Timer with a Foreground Service
Prompt: "A pomodoro timer with a foreground service"
Primary stress: Background execution
Acceptance criteria:
  1. App launches without runtime errors
  2. User can start a pomodoro timer
  3. Timer continues when app is in background
  4. Foreground service notification is displayed
  5. User is notified when the timer completes
  6. User can pause and reset the timer
  7. App tracks completed pomodoros
  8. UI adapts to different screen sizes
  9. Dark theme is supported
  10. Foreground service is properly declared in manifest
```

#### 80.6.8 FIX-PROG-08: Expense Tracker with a Chart and CSV Export

```text
Fixture: FIX-PROG-08
Name: Expense Tracker with a Chart and CSV Export
Prompt: "An expense tracker with a chart and CSV export"
Primary stress: Data visualization and file output
Acceptance criteria:
  1. App launches without runtime errors
  2. User can add an expense with amount, category, and date
  3. User can view expenses in a list
  4. App displays a chart of expenses by category
  5. User can export expenses as CSV
  6. CSV file is saved to device storage
  7. App handles empty state gracefully
  8. UI adapts to different screen sizes
  9. Dark theme is supported
  10. No expenses are uploaded to any external service
```

### 80.7 Implementation sequencing within milestones

Each milestone's work items MUST be implemented in the order listed. Dependencies between work items are specified.

#### 80.7.1 M0 sequencing

1. Repository layout (must exist before any code)
2. TypeScript and Rust conventions (must exist before any code)
3. Configuration model (needed by all subsequent work)
4. Logging standard (needed by all subsequent work)
5. Test fixtures (needed for exit gate)
6. Security baseline (needed for exit gate)
7. Local certification pipeline (needed for exit gate)

#### 80.7.2 M1 sequencing

1. C#/.NET + WinUI 3 shell (foundation for all UI)
2. Welcome, create-project, open-project screens (user entry points)
3. Main workspace layout (core UI structure)
4. Project metadata storage (needed by all project operations)
5. Application-level error handling (needed for reliability)
6. Keyboard navigation and accessibility (needed for acceptance)

#### 80.7.3 M2 sequencing

1. IPC API (foundation for UI-supervisor communication)
2. SQLite store (foundation for all persistence)
3. Event bus (foundation for all event-driven behavior)
4. Task scheduler (needed for all task operations)
5. Process registry (needed for all process management)
6. Recovery scanner (needed for restart resilience)
7. Notification adapter (needed for user notifications)

#### 80.7.4 M3 sequencing

1. ProviderProfile definition (foundation for provider runtime)
2. Secure credential storage (needed before any provider calls)
3. Authenticated provider request (core provider functionality)
4. Normalized response (needed by all provider consumers)
5. Timeout and cancellation (needed for reliability)
6. Failure classification (needed for error handling)
7. Durable usage record (needed for telemetry)

#### 80.7.5 M4 sequencing

1. AndroidConstructionContract definition (foundation for synthesis)
2. targetPlatforms enforcement (invariant check)
3. Technology-plan resolver (needed for project creation)
4. Environment diagnostics (needed for preflight)
5. Project workspace creation (core synthesis output)
6. Preflight record (needed for capability classification)
7. Durable checkpoint (needed for recovery)

#### 80.7.6 M5 sequencing

1. Authorized tools (foundation for agent loop)
2. Plan production (needed before any work)
3. Checkpoint creation (needed before any mutation)
4. File mutation (core agent capability)
5. Build execution (needed for validation)
6. Install/launch (needed for preview)
7. Observation (needed for evidence)
8. Validation (needed for completion)
9. Repair (needed for recovery)
10. Diff reporting (needed for user visibility)

### 80.8 Prompt templates

Every system prompt used by the runtime is defined here. An agent MUST use these exact templates.

#### 80.8.1 System prompt for planning

```
You are an autonomous Android development agent. Your goal is to plan and execute the following task:

Task: {task_description}

Project context:
- Framework: {framework}
- Package: {package_id}
- Current revision: {revision}

Constraints:
{constraints}

Locked decisions:
{locked_decisions}

Available capabilities:
{capabilities}

You MUST:
1. Produce a plan with discrete, ordered steps
2. Identify files to change and commands to run
3. Specify acceptance criteria for each step
4. Cite evidence for every claim
5. Stop and escalate if blocked

You MUST NOT:
1. Execute any action without authorization
2. Claim completion without evidence
3. Access files outside the approved workspace
4. Send project content to unapproved providers
5. Bypass policy gates

Output your plan in this format:
## Understanding
## Plan
## Files to change
## Commands that may run
## Acceptance criteria
## Risks and mitigations
```

#### 80.8.2 System prompt for code generation

```
You are an autonomous Android development agent. Your goal is to implement the following change:

Task: {task_description}
Plan step: {step_description}

File: {file_path}
Current content:
{file_content}

Constraints:
{constraints}

You MUST:
1. Produce a minimal, targeted change
2. Preserve all existing functionality
3. Follow project conventions
4. Include necessary imports
5. Handle errors appropriately

You MUST NOT:
1. Add unrelated changes
2. Remove existing functionality
3. Introduce security vulnerabilities
4. Hardcode secrets
5. Add dependencies not in the approved plan

Output your change as a unified diff or complete file replacement.
```

#### 80.8.3 System prompt for validation

```
You are an autonomous Android development agent. Your goal is to validate the following change:

Task: {task_description}
Change: {change_summary}

Validation plan:
{validation_plan}

You MUST:
1. Run all specified checks
2. Report pass/fail for each check
3. Include evidence for each result
4. Stop and report if a check fails
5. NOT claim success without evidence

You MUST NOT:
1. Skip any specified check
2. Weaken a check to make it pass
3. Report predicted results as observed
4. Claim completion without passing checks

Output your validation in this format:
## Checks performed
## Results
## Evidence
## Remaining issues
```

#### 80.8.4 System prompt for repair

```
You are an autonomous Android development agent. Your goal is to repair the following failure:

Task: {task_description}
Failure: {failure_description}
Error output: {error_output}
Changed files: {changed_files}

Previous attempts:
{previous_attempts}

You MUST:
1. Identify the root cause from evidence
2. Propose a minimal repair
3. Explain why the repair addresses the cause
4. NOT repeat a failed strategy
5. Escalate if the cause is unclear

You MUST NOT:
1. Regenerate the entire file without cause
2. Apply the same fix that already failed
3. Ignore the error output
4. Claim success without revalidation

Output your repair in this format:
## Root cause analysis
## Proposed repair
## Why this addresses the cause
## Validation plan
```

#### 80.8.5 Context compaction prompt

```
The following context has exceeded the compaction threshold. Produce a structured summary that retains:

MUST RETAIN:
- Active constraints: {constraints}
- Locked decisions: {decisions}
- Current goal: {goal}
- Current plan: {plan}
- Active errors: {errors}
- Recent evidence (last 5 turns): {recent_evidence}
- Recent file changes (last 5 turns): {recent_changes}

SUMMARIZE:
- Earlier context into a structured summary
- Historical commands and results
- Superseded plans

ARCHIVE:
- Full traces to cold storage
- Old screenshots
- Completed handoffs

Output the summary in this format:
## Goal
## Active constraints
## Locked decisions
## Current plan
## Active errors
## Recent evidence summary
## Historical summary
## Archived references
```

### 80.9 Acceptance criteria

The agent-buildability contract is satisfied only when:

1. Every "should" in the specification has explicit criteria
2. Every "configurable" parameter has a default value
3. Every vague procedure has a concrete step-by-step replacement
4. Every referenced schema has a complete field definition
5. Every system prompt has a defined template
6. Every runtime decision has explicit criteria
7. Every adapter has a complete method signature
8. Every test fixture has a concrete definition
9. Every milestone has explicit implementation sequencing
10. An AI agent can build Nirman from these docs without hallucination
