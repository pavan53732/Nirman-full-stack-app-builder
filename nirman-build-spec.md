# Nirman

## Detailed Product and Build Specification

**Document type:** Product requirements and technical architecture specification  
**Application type:** Windows-first desktop application for autonomous Android application development  
**Suggested product name:** **Nirman**  
**Status:** Initial product definition  
**Primary goal:** Enable a user to create, modify, preview, test, package, and export Android applications through a conversational AI-assisted desktop workspace.

---

## 1. Product Identity

### 1.1 Product name

The recommended name is **Nirman**.

The name “Nirman” conveys building and creation. It describes a focused Android development desktop application without making the product sound like a hosting service or developer platform.

The product should consistently be described as a **desktop application for building other applications**, not as a platform. It runs on the user’s computer, manages local project workspaces, connects to the user’s selected AI provider, and produces source code and build artifacts that remain under the user’s control.

### 1.2 Product statement

> Nirman is a local-first Windows desktop application that uses configurable AI models to help users design, generate, run, test, preview, repair, and package Android applications through a simple conversational workspace.

### 1.3 Product vision

Nirman should make Android development feel closer to describing a product than manually assembling every implementation detail. A user should be able to explain an Android idea, answer a small number of important questions, watch the application appear in an emulator or connected-device preview, request changes through chat, and export the resulting Android source code or installable build.

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

The product should support both technical and semi-technical users. Beginners need guided setup, explanations, templates, and safe defaults. Experienced developers need direct access to files, commands, diffs, logs, provider settings, and project configuration.

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
2. Connect to a local compatible model server.
3. Continue in planning-only mode without an AI provider.

The setup wizard should check the local environment, detect installed versions of Node.js, package managers, Java, Gradle, Android SDK, platform-tools, emulator tooling, and device tooling, and identify which Android capabilities are available. Missing tools should be reported with an installation guide rather than hidden behind a failed build.

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

Before a potentially risky operation, the chat should show a clear approval card. For example, package installation, network access, writing outside the workspace, credential use, device access, or release signing should not be hidden inside ordinary text.

A request may include one or more screenshots as visual references. Nirman should analyze layout, typography, color, spacing, components, navigation states, device framing, interaction clues, and visible content. It should convert the analysis into an editable visual specification, identify uncertainty, synthesize the Android implementation, and validate the result against the reference screenshots in the emulator or connected device.

### 4.4 Live preview panel

The live preview should support Android emulator and connected-device preview first. It should show the selected device, build/install state, Metro or native development-server output, connection status, runtime errors, Logcat output, reload controls, and the current project revision.

The default project workspace should show the running application preview and the live execution surface together. The preview occupies the primary visual area, while a resizable execution panel shows the task graph, nested worker steps, terminal streams, checkpoints, approvals, validation evidence, and current next action. Users may collapse or expand the execution panel, but the relationship between the running application and the work producing it must remain visible without navigating to a separate screen.

Nirman should optionally capture screenshots during autonomous tasks and compare them with user-provided references or generated visual baselines. The selected AI provider may receive screenshots for visual inspection if the user has enabled that capability. The user should be told when an image is being sent to a cloud provider. Screenshots, visual specifications, comparison results, and unresolved visual differences must be attached to the task evidence.

The preview panel should support Android emulator and physical-device connection status, device identity, Android version, architecture, available storage, hot reload state, Logcat output, install status, screenshots, and links to generated APK or AAB artifacts. Multiple devices may be added later, but the first stable workflow may use one active device or emulator.

### 4.5 Manual editing

Nirman must not trap users inside the chat. The application should include a full code editor with syntax highlighting, search, multi-file tabs, formatting, diagnostics, and direct editing. After a manual edit, the agent should be able to re-index the project and continue working from the updated state.

---

## 5. Android-Only Application Scope

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

---

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
│ Android project │ Android runtime │ Expo/React Native │ APK/AAB artifacts │
└────────────────────────────────────────────────────────────┘
```

### 6.1 Desktop shell

The desktop shell should use Tauri with a React and TypeScript interface. The shell is responsible for opening project folders, communicating with the local runtime, presenting native dialogs, storing secure credentials through the operating-system keychain, and managing application-level settings.

### 6.2 Frontend interface

The frontend should contain the chat workspace, project selector, file tree, editor, preview frame, terminal panel, test panel, provider settings, environment diagnostics, and export controls.

The interface should maintain a clear distinction between generated text and executed actions. A message saying that a command will run is different from a confirmed command result, and the interface must represent those states separately.

### 6.3 Local runtime

The local runtime manages Android project processes and development tools. It should be responsible for starting and stopping Metro or native development servers, managing Gradle and Android build processes, reading Logcat and process output, enforcing timeouts, checking ports, managing emulators and devices, running tests, capturing screenshots, and collecting APK/AAB artifacts.

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
| `export_project` | Create a ZIP, Git bundle, or build artifact |

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
| Base URL | Custom provider endpoint, including compatible API endpoints |
| API key | Stored securely in the operating-system keychain |
| Chat model ID | Model used for planning and code generation |
| Vision model ID | Optional model used for screenshot and preview analysis |
| Embedding model ID | Optional model used for project retrieval |
| Token limit | Provider-specific output limit |
| Temperature | Optional creativity control |
| Reasoning configuration | Optional provider capability setting |
| Timeout | Maximum request duration |
| Enabled capabilities | Text, vision, structured output, tool calling, embeddings |
| Test connection | Sends a safe validation request before saving |

### 8.2 Provider adapter interface

The internal provider interface should normalize differences between services. It should support text generation, structured JSON output, tool calls, vision input, streaming responses, cancellation, error normalization, and capability discovery.

A provider adapter should return a normalized result containing the model ID, response text, tool calls, usage information when available, finish reason, request duration, and any provider warning.

### 8.3 Privacy behavior

Nirman must clearly communicate whether project content is being sent to a cloud model. The user should be able to configure context policies that exclude selected files, folders, secrets, generated binaries, or sensitive project types.

For local models, Nirman should support a local compatible API endpoint. Local-model compatibility should be treated as an optional capability because local models may have different context limits, tool-calling behavior, coding quality, and visual-inspection support.

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
| Android build | Node.js, package manager, Java, Gradle, Android SDK, platform-tools, emulator or device |
| Expo Android | Node.js, package manager, Java, Android SDK, emulator or device when used |
| Git export | Git executable and repository permissions |

The diagnostic screen should distinguish between installed, missing, outdated, misconfigured, and inaccessible tools. It should provide a command or official installation reference where appropriate.

### 9.3 Process controls

The runtime should implement command timeouts, output truncation limits, process termination, port conflict detection, memory safeguards where available, and cancellation from the user interface.

Commands should be categorized as safe, reviewable, or privileged. Safe commands can run automatically within the workspace. Reviewable commands require approval according to the user’s policy. Privileged commands always require explicit approval.

Terminal execution must support persistent per-worker sessions with a working directory, environment snapshot, shell type, process group, and session identifier. The runtime must detect interactive prompts, provide a controlled input channel, apply an unattended prompt policy, and terminate or recover processes that wait for input beyond the configured liveness window. On Windows, the shell profile must explicitly identify PowerShell, `cmd.exe`, Git Bash, WSL, or another approved shell and record the selected shell in the task evidence. The interface should show multiple worker terminals separately, with searchable rolling logs and preserved raw artifacts for long-running processes.

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
- targetPlatforms
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

The user must be able to create a project from a supported template, open an existing local project, rename a project, close a project, inspect project health, and select the active AI provider.

### 12.2 Chat-driven generation

The user must be able to describe a new application or request a change to an existing project. Nirman must display the agent’s understanding, plan, actions, progress, validation results, and final summary.

### 12.3 Code and diff management

The user must be able to inspect generated files, review diffs, accept or reject grouped changes, manually edit files, create checkpoints, undo a task, and restore an earlier checkpoint.

### 12.4 Preview and validation

Nirman must start a local preview for supported Android projects, show the emulator or connected-device preview inside the application, display runtime errors, run linting and type checks, capture screenshots, and present validation results in a readable form.

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
| Maintainability | Agent tools, provider adapters, templates, and UI should have separate boundaries |

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
├── templates/
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

**Exit criteria:** A user can configure a compatible cloud or local provider and receive a validated response without exposing the API key in logs or project files.

### Phase 3: Project templates

Create the initial React and TypeScript web template. Add project metadata, scripts, formatting, linting, type checking, and a clear convention for components, routes, assets, and configuration.

**Exit criteria:** A new project can be created locally, installed, started, and opened in the live preview.

### Phase 4: Structured agent tools

Implement project inspection, file search, file reading, file creation, targeted patches, checkpoints, command execution, and diff reporting.

**Exit criteria:** The agent can make a small, reviewable change to a project and show the complete action history.

### Phase 5: Autonomous development loop

Add task planning, acceptance criteria, grouped changes, preview startup, screenshot capture, linting, type checking, tests, error classification, repair attempts, cancellation, and failure escalation.

**Exit criteria:** Nirman can complete a common feature request, validate the result, repair at least common implementation failures, and stop safely when blocked.

### Phase 6: Android packaging and artifact export

Add Git export, Android debug/release build artifacts, APK/AAB packaging, signing configuration boundaries, artifact metadata, checksums, and Android build diagnostics.

**Exit criteria:** A supported Android project can produce a validated APK or AAB artifact and the user can locate the resulting artifact with its build metadata and validation report.

### Phase 7: Android generation

Add Expo and React Native templates, Android environment diagnostics, emulator or device connection information, Android logs, and APK/AAB build workflows where the local environment supports them.

**Exit criteria:** Nirman can create and build a supported Android project and clearly identify environmental limitations.

### Phase 8: Advanced features

Add visual element selection, project memory, reusable components, database and authentication templates, multi-agent task specialization, regression screenshots, and more native project profiles.

**Exit criteria:** Advanced capabilities remain optional and do not reduce the reliability of the core web and desktop workflows.

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
| Cloud providers receive sensitive project data | High | Provide context exclusions, privacy notices, local models, and redaction |
| Different models support different tool protocols | Medium | Normalize providers through adapters and capability discovery |
| The UI looks correct but behavior is broken | Medium | Combine visual screenshots with tests, type checks, and runtime inspection |
| Universal framework support increases complexity too quickly | High | Add templates only after the core workflow is reliable |
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

The first vertical slice should allow the user to open Nirman, configure a provider, describe any supported Android application in chat, optionally attach screenshots, receive a technology-selection plan, synthesize a project, apply a small file change, start an emulator or device preview, and inspect the result.

The second slice should add checkpoints, diffs, tests, repair attempts, Android emulator/device preview, and cancellation. The third should add Android packaging, APK/AAB artifacts, signing boundaries, and device validation.

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
- Android packaging, device validation, and later specialized native Android profiles.
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
10. Add Android packaging and APK/AAB artifact export.
11. Add full Android technology coverage, native integration, and device capabilities.
12. Add advanced visual editing, project memory, technology-plan inspection, and screenshot comparison.

---

## 22. Advanced Autonomous Development and Swarm Execution Capabilities

This section incorporates advanced patterns from modern autonomous agent frameworks—specifically focusing on **parallel agent orchestration (swarms)**, **long-running continuous background execution**, **persistent problem-solving loops**, **anti-thrashing error recovery**, and **shared task state coordination**. All descriptions are tailored to Nirman's local desktop application architecture without referencing external agent brand names.

### 22.1 Parallel Agent Orchestration (Swarm Architecture)

To prevent the latency and scalability bottlenecks of traditional sequential tool execution, Nirman should support a **Parallel Swarm Orchestrator**. When a user requests a complex application feature or multi-module refactor, the main orchestrator decomposes the objective into orthogonal sub-tasks and delegates them to specialized background workers operating concurrently.

| Swarm Role | Responsibility | Execution Boundary |
|---|---|---|
| Primary Orchestrator | High-level planning, task decomposition, agent routing, and final synthesis | Main session context |
| Architecture Scout | Repository-wide exploration, dependency mapping, and upstream research | Read-only background worker |
| UI/Frontend Specialist | Component generation, responsive layout, styling, and visual asset integration | Isolated branch/worktree |
| Backend/Logic Specialist | API routes, database schemas, server actions, and business logic | Isolated branch/worktree |
| Test & QA Engineer | Automated test suite generation, edge-case coverage, and execution | Isolated branch/worktree |
| Security & Reviewer | Vulnerability scanning, diff analysis, and compliance verification | Non-mutating verification worker |

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

For multi-worker tasks and parallel swarms, Nirman maintains a centralized, machine-readable **Task Ledger** (stored locally as `TODO.md` or a structured state file within the workspace). 

- **Atomic Task Units**: Tasks are broken down into discrete, atomic items with defined dependencies (e.g., Task 3 cannot start until Task 1 and Task 2 pass their tests).
- **Claim-and-Update Protocol**: Background workers claim unassigned tasks, mark their progress in real time, and record completion evidence (test logs, file paths).
- **Inter-Agent Handoffs**: Workers can read each other's completion summaries. For instance, the Test Engineer reads the Backend Specialist's implementation notes to write precise integration tests.

---

## 23. Advanced Autonomous Development Capabilities

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
| Backend Worker | Build APIs, data access, validation, and integrations | Workspace edits; approved commands |
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

The application should also support external tools through an MCP-compatible adapter or equivalent extension interface. External tools may provide design files, issue trackers, documentation search, browser automation, observability data, or test environments. Each external tool must have its own permission scope, provider status, network policy, and audit trail.

### 23.12 Hooks and policy interception

Nirman should expose pre-action and post-action hooks. A pre-action hook may validate a command, redact a secret, enforce a path policy, require an approval, or transform tool arguments. A post-action hook may summarize output, detect errors, update the repository map, create a checkpoint, or trigger a reviewer.

Hooks should be deterministic where possible and should run outside the model’s control. A model must not be able to disable a mandatory security hook through ordinary project instructions.

### 23.13 AST, LSP, and semantic editing

Text patches are useful but insufficient for large refactors. Where the language server or parser is available, Nirman should use semantic operations such as rename symbol, find references, extract function, update imports, change interface implementation, and apply workspace-wide type-safe transformations.

The agent should prefer semantic edits for high-impact refactors and use text patches for localized changes. After a semantic edit, Nirman should run the relevant type checks and tests, then show the affected symbol and file graph.

### 23.14 Android device and visual verification

For Android applications, Nirman should optionally connect to a controlled emulator or physical-device runner. The device worker may install builds, launch activities, tap and fill synthetic data, capture screenshots, inspect Logcat and runtime errors, verify permissions and orientation, test phone and tablet layouts, and collect crash traces.

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
| P2 | Browser automation, screenshots, visual QA, AST/LSP edits | Improves quality beyond text generation |
| P3 | Headless automation, scheduled local tasks, remote worker connections | Expands automation after the local core is stable |
| P3 | Advanced native project profiles and multi-device testing | Expands beyond the initial supported stacks |

Nirman should not begin with unrestricted multi-agent parallelism. It should first prove that one worker can reliably inspect, plan, edit, test, and recover within a controlled workspace. Parallel workers should be added only after checkpoints, permissions, event logs, and reconciliation are dependable.

---

## 25. Updated Definition of a High-Quality Autonomous Task

A high-quality Nirman task is not merely a code-generation response. It is a reproducible development record containing the original request, project context, plan, selected worker roles, permissions, model routing, files changed, commands run, tests and screenshots, checkpoints, warnings, resource usage, and unresolved issues.

The task should be considered complete only when the requested acceptance criteria are satisfied or the application has clearly explained why they could not be satisfied. The final result must not hide uncertainty behind confident wording.

> Nirman should optimize for **verified progress**, not maximum autonomous activity.

## 26. Implementation-Level Requirements for the Initial Architecture

The master specification defines product behavior, while this section makes the most important implementation mechanics explicit. The section is intentionally code-free: it describes the components, interfaces, state transitions, limits, and acceptance behavior that the engineering documents must implement.

### 26.1 Local control plane and persistent task daemon

Nirman should separate the desktop user interface from a local control plane. The interface may close, restart, or become unavailable without destroying a running task. A local task daemon should own task execution, worker processes, approvals, checkpoints, logs, and recovery.

The control plane should start when Nirman launches and should be able to continue as a user-scoped background process when the window is minimized or closed. It should not run as a system service by default. The user must be able to stop it from the application and from a visible operating-system process control action.

The daemon should persist task state in a local SQLite database or an equivalent transactional store. Large logs and binary artifacts should be stored in task-specific directories, while the database stores metadata and references.

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
| Default task wall-clock policy | No fixed completion lock; adaptive monitoring with optional user-configured hard safety cap |
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

Browser automation should use a dedicated Nirman-managed browser profile, separate from the user’s personal browser profile, cookies, extensions, saved passwords, and downloads. Test sessions should use synthetic data and disposable storage by default.

The browser worker should expose only approved routes and local development origins. External navigation should be controlled by the network policy. Screenshots, console logs, network failures, accessibility findings, and interaction traces should be attached to the task record.

### 26.9 Preview state, checkpoints, and rollback

The preview manager should associate every running preview with a project revision and checkpoint ID. When files change, it should report whether the preview hot-reloaded, partially reloaded, or required a full restart.

When the user reverts a checkpoint, Nirman should stop or invalidate the preview if its running revision no longer matches the restored project. It may hot-reload only when the preview runtime confirms that the restored state is safe and complete. The UI should never show a preview as current when it represents a different checkpoint.

### 26.10 Responsive and multi-device preview

The Android preview should support named device profiles for phone, tablet, portrait, landscape, Android version, architecture, screen density, and API level. A visual test should launch the same flow across selected emulator or device profiles, compare screenshots, and record device-specific findings.

Android preview should use a device-manager abstraction that reports emulator or physical-device identity, connection state, platform version, architecture, available storage, hot-reload state, logs, and build/install status. The first implementation may support one connected device or emulator at a time, but the protocol should allow multiple devices later.

### 26.11 Toolchain version management

Nirman should not rely on one globally installed toolchain. Each Android project should declare required versions or compatible ranges for Node.js, package manager, Java, Gradle, Android SDK, platform-tools, emulator images, Expo or React Native tooling, and selected native build dependencies.

The runtime should resolve a project toolchain through a version manager, portable installation, or explicitly configured local path. Each project receives isolated environment variables, cache paths, process scopes, and toolchain bindings so incompatible projects cannot silently change one another’s environment. Two projects with incompatible versions must be able to run without silently changing one another’s environment.

The environment record should contain executable paths, detected versions, source of installation, compatibility result, and reproducibility status. A build must fail with a diagnostic when the requested toolchain cannot be resolved.

### 26.12 Android runtime abstraction

Although Nirman runs as a Windows desktop application, its generated target is Android. Runtime operations should use an Android-focused interface defining process launch, termination, filesystem policy, environment discovery, port management, emulator and device control, Logcat capture, Gradle and Metro execution, quotas, and APK/AAB artifact handling. The desktop host may use Windows-specific process and sandbox implementations, but the generated-project contract remains Android-specific.

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

For supported application targets, the default autonomous validation loop should be:

```text
Preview or launch target
    ↓
Run focused tests and checks
    ↓
Run build or package validation
    ↓
Run security, dependency, and reliability checks
    ↓
Run browser, device, accessibility, and visual QA where applicable
    ↓
Classify failures and warnings
    ↓
Repair or backtrack to a known-good checkpoint
    ↓
Revalidate the affected and regression checks
    ↓
Evaluate completion conditions
```

Nirman should not ask for approval for every small, reversible operation inside an approved workspace. It should request a decision only at defined policy boundaries, including protected-file access, risky dependency installation, external-service access, credential use, destructive operations, publishing, release signing, or any action outside the current workspace and policy scope. The approval request must identify the exact action, reason, worker, workspace, policy, risk, and available choices.

A task must terminate only when one of the following conditions is true: all required completion conditions pass; a required user decision is reached; an explicit hard safety or policy limit is reached; the environment or provider is unavailable; the user cancels the task; an unresponsive or dangerous process must be stopped to protect the computer; or an unrecoverable failure occurs. Ordinary time, token, cost, process, disk, and retry thresholds should cause adaptation, throttling, warning, or optional approval—not a fixed completion lock. If the screenshot or task view shows extended activity, that demonstrates persistent execution, not a guarantee that every goal can be completed without intervention.

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

A self-improvement proposal may modify prompts, task decomposition, model routing, context retrieval, tool schemas, failure classifiers, worker roles, skills, provider adapters, validation rules, or runtime code. Changes to the supervisor, policy engine, sandbox, credentials, updater, database migrations, or evidence engine require the highest validation level and must not be promoted solely from a model-generated proposal.

### 28.5 Candidate evaluation and promotion

Self-improvement must happen in an isolated worktree and candidate runtime. A candidate must pass targeted tests, broad regression fixtures, provider compatibility tests, sandbox and permission tests, migration tests, recovery tests, candidate health checks, smoke tasks, and representative end-to-end task replay.

Promotion should support observe-only, candidate-only, canary, trusted auto-promotion, and manual-promotion modes. Trusted auto-promotion may be enabled for low-risk scoped changes, but stable-controller recovery, rollback artifacts, credential protections, and sandbox boundaries remain non-bypassable.

After promotion, Nirman must monitor candidate outcomes against the previous baseline and automatically roll back or disable the candidate scope when quality, stability, security, or recovery metrics degrade.

### 28.6 Runtime memory boundaries

Nirman should maintain separate task memory, project memory, and runtime-improvement memory. Memory must be generated from validated events and user-confirmed decisions rather than every model statement. The user must be able to inspect, correct, export, and delete memory. Credentials, protected files, and unclassified private content must never enter long-term improvement memory.

### 28.7 End-to-end runtime acceptance criteria

The complete runtime is not considered implemented until it can accept one broad goal, extract requirements, create a durable task graph, run multiple workers, persist events, execute the validation loop, recover from worker/provider/environment failure, survive application restart, produce evidence-backed completion, and continue until the goal is complete or a genuine hard stop condition exists.

The self-improvement loop is not considered implemented until Nirman can observe episodes, detect recurring failures, produce a scoped improvement proposal, build and evaluate a candidate, run a canary, promote it through the stable controller, monitor post-promotion behavior, and automatically roll back without corrupting the active application or user projects.

### 28.8 Core Autonomous Runtime Capabilities

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

## References

[1]: https://tauri.app/ "Tauri Documentation"

[2]: https://react.dev/ "React Documentation"

[3]: https://www.typescriptlang.org/docs/ "TypeScript Documentation"

[4]: https://docs.expo.dev/ "Expo Documentation"

[5]: https://reactnative.dev/docs/getting-started "React Native Documentation"

[6]: https://git-scm.com/doc "Git Documentation"

[7]: https://www.electronjs.org/docs/latest/ "Electron Documentation"

---

**Document owner:** Nirman product team  
**Recommended application name:** Nirman  
**Recommended first release:** Windows desktop application for local Android application generation, emulator/device preview, testing, repair, packaging, and APK/AAB export


## 28. End-to-End Android Generation Contract

The primary product promise is that one user instruction and optional screenshots launch one durable Android engineering session. The session must continue through input analysis, visual specification, technology selection, project synthesis, live preview, implementation, testing, repair, validation, packaging, and evidence-backed completion without routine human intervention.

### 24.1 Input fusion

The session combines the user’s chat instruction, screenshots, supplied assets, existing project files, device requirements, integrations, and delivery requirements into three authoritative inputs: an `AndroidApplicationContract`, a `VisualSpecification`, and an `AndroidTechnologyPlan`. The user does not select a framework or template. The configured AI resolves the implementation from these inputs.

### 24.2 Autonomous Android session

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

### 24.3 Live preview and execution synchronization

The live Android emulator or connected device is a first-class execution surface. Every preview state must expose the project revision, checkpoint ID, device identity, installation state, reload state, Logcat, runtime errors, latest screenshot, visual comparison result, and the worker or task responsible for the current change.

If a candidate change breaks the application, the preview must show the last valid revision and identify the failed candidate. The execution tree and preview must share a revision identifier so the user can see exactly which work produced the running application.

### 24.4 Progress ledger and stall detection

The runtime must maintain a progress ledger recording changed files, new evidence, preview revision movement, test transitions, worker handoffs, strategy changes, and validated requirements. A stall detector must identify repeated commands, repeated patches, repeated failure fingerprints, unchanged workspaces, unchanged previews, missing evidence, unresponsive processes, and heartbeats without useful progress.

When a stall is detected, the runtime must refresh context, change strategy, change technology, delegate diagnosis, repair the environment, restore a checkpoint, or construct an isolated alternative. It must not repeat the same action indefinitely.

### 24.5 Swarm handoff and reconciliation

Parallel workers must receive explicit contracts and isolated workspaces. Each handoff must include changed files, assumptions, dependencies, tests, evidence, unresolved issues, and recommended next actions. The reconciliation worker integrates only validated outputs, resolves conflicts, runs integrated Android checks, updates the live preview, and creates the next checkpoint.

### 24.6 APK/AAB completion gates

A task is complete only when its applicable completion conditions are proven. For Android delivery, the evidence must include a successful build, an APK or AAB artifact, a checksum, artifact scanning, installation or launch evidence, main-flow results, screenshot or visual validation, required permission behavior, and no unresolved fatal runtime errors. The final artifact must link to the project revision and evidence ledger.

### 24.7 No-routine-intervention policy

Routine project-local actions may continue automatically under the configured Unattended / Full Autonomy policy, including editing, dependency installation, terminal commands, emulator launches, builds, tests, screenshots, repair attempts, checkpoints, worker handoffs, and local artifact creation. Only protected credentials, destructive operations, external publishing, signing policy, protected paths, missing required information, hard safety violations, or unrecoverable technical blockers may interrupt the session.

### 24.8 Full Android capability acceptance

The product must validate AI-selected generation across JavaScript-driven Android projects, Java, Kotlin, Android Views, Jetpack Compose, mixed architectures, custom native modules, background services, WorkManager, notifications, camera and media, location and sensors, Bluetooth and NFC, offline-first storage, API-heavy applications, authentication and permissions, tablet and multi-orientation layouts, device-integrated applications, and APK/AAB delivery. These are internal acceptance categories, not user-facing templates.

---

## 29. Android Completion Report

The final completion screen must show the application identity, selected technology plan and reasons, final emulator or device state, build and validation results, APK/AAB paths and checksums, recovery history, source revision, checkpoints, warnings, and unresolved issues. A model-generated statement that the work is complete is never sufficient evidence.

---

## 30. Final System Principle

> **The user gives one Android application idea and optional screenshots once. The system works continuously in the background, dynamically chooses the Android implementation, updates the live preview, coordinates terminals and workers, heals failures, validates the result, and returns a working APK/AAB with evidence.**

The complexity belongs inside the runtime rather than inside the user’s workflow. Deterministic lifecycle, permission, sandbox, storage, evidence, recovery, promotion, rollback, and termination authorities remain in control while the configured AI proposes and executes development work within the approved policy.

---

## 31. Autonomous Runtime Capability Contract

The runtime must provide specialized workers, a self-healing loop, evidence-based completion, adaptive resource management, self-development, project memory, and environment repair as core capabilities. These capabilities are mandatory parts of the end-to-end Android session rather than optional extensions.

**Acceptance statement:** A representative Android task can be launched from one instruction and optional screenshots, continue through background implementation, update the live preview, recover from injected worker/process/provider/device failures, produce evidence for each completion condition, and return a validated APK/AAB without routine approval pauses.


## 32. Production Runtime Contract

Nirman must treat the autonomous Android build as a deterministic runtime session rather than a sequence of independent chat responses. The model proposes plans and actions; the runtime owns lifecycle, permissions, filesystem access, process execution, device access, persistence, evidence, recovery, promotion, rollback, and termination.

### 32.1 Canonical runtime contracts

The implementation must define versioned, validated contracts for:

| Contract | Responsibility |
|---|---|
| `AutonomousAndroidSession` | Owns the full task from one user request to validated APK/AAB output |
| `AndroidApplicationContract` | Captures features, screens, behavior, integrations, devices, permissions, and acceptance conditions |
| `VisualSpecification` | Captures screenshot-derived layouts, states, components, typography, color, spacing, and comparison rules |
| `AndroidTechnologyPlan` | Records AI-selected languages, UI systems, native modules, SDKs, libraries, device APIs, and build strategy |
| `TaskGraph` | Defines phases, dependencies, workers, inputs, outputs, checkpoints, and completion conditions |
| `WorkerContract` | Defines worker purpose, workspace, tools, permissions, inputs, outputs, and validation rules |
| `TerminalSession` | Tracks shell, working directory, environment, process tree, PTY, input policy, output, and recovery |
| `PreviewRevision` | Binds emulator/device state to a project revision and checkpoint |
| `EvidenceRecord` | Stores proof from tests, builds, screenshots, Logcat, permissions, scans, and artifacts |
| `RecoveryRecord` | Stores failure fingerprints, attempted strategies, backtracking, and outcomes |
| `ArtifactRecord` | Stores APK/AAB metadata, checksum, build profile, signing state, scans, and source revision |
| `ProviderProfile` | Stores endpoint, model ID, protocol, capabilities, privacy policy, and routing role |

All durable contracts require explicit versioning, schema validation, atomic persistence, migration, backup, and rollback. No model output may create undocumented fields or alter authority rules.

### 32.2 Authoritative lifecycle

The session lifecycle must be explicit and persisted:

```text
Created → Understanding → Planning → EnvironmentPreparing
  → ProjectSynthesizing → Implementing → Previewing
  → Testing → Recovering → Revalidating → Packaging → Completed
```

Safe terminal states are `BlockedByPolicy`, `BlockedByMissingInformation`, `ProviderUnavailable`, `EnvironmentUnrecoverable`, `Cancelled`, and `SafelyFailed`. Models, workers, skills, hooks, and UI events may propose transitions but cannot commit them directly.

### 32.3 Renewable leases and operation capabilities

Long-running work must use a renewable session lease rather than a short fixed execution token. The supervisor renews the lease only while heartbeats, progress, and authority checks remain valid. Individual sensitive operations use scoped, single-use operation capabilities bound to the session, worker, workspace, project revision, action type, and expiry.

An operation capability is required for actions such as installing a risky dependency, changing protected configuration, accessing a device capability, signing an artifact, publishing, or promoting a self-update. A model cannot mint, extend, or broaden a capability.

## 33. Android Project Ingestion and Integrity

The project-ingestion layer must understand Android source files, Gradle settings, manifests, resources, assets, fonts, localization, JavaScript package manifests where selected, native-module boundaries, emulator/device configuration, generated build directories, secrets, keystores, local properties, environment files, Git state, and uncommitted changes.

The layer must apply hard exclusions, canonical path normalization, project-root boundaries, scope fingerprints, content hashes, and revision checks. Before reconciliation, preview installation, packaging, or self-development promotion, it must detect external changes and revalidate the active project revision. A stale or mismatched revision must be rejected rather than silently overwritten.

## 34. Provider Gateway and Controlled Tool Protocol

The Model Gateway must normalize configured Chat Completions, Responses-style requests, message history, screenshot inputs, structured outputs, tool calls, tool results, streaming task events, cancellation, usage, context limits, provider errors, and model capabilities.

The user owns each endpoint, API key, base URL, and model ID. Nirman must not silently replace a configured model. Explicitly approved role profiles may route planning, coding, visual inspection, debugging, testing, and review to different providers or models.

Every tool call must have a typed name, version, schema-validated arguments, session ID, worker ID, project policy, privacy classification, requested capabilities, and evidence result. Unknown tools, unknown arguments, unapproved routing, secret access, and malformed tool results must be rejected before execution.

## 35. Execution Isolation and Sandbox Boundaries

The runtime must separate the Windows host, control-plane supervisor, worker processes, Android build processes, emulator/device processes, preview application, provider network access, project files, credentials, and signing material.

Generated code must not automatically access personal files, browser cookies, SSH keys, API keys, signing keys, unrelated projects, or arbitrary network resources. Each process receives the minimum filesystem, network, process, and device permissions required by its contract. Sandbox policy is enforced by deterministic runtime authorities and cannot be weakened by model output.

## 36. Event, Evidence, and Completion Authority

Nirman must distinguish among model claims, runtime events, and evidence records. A model statement such as “the login screen is complete” is not completion evidence. Completion requires applicable proof from builds, installation, automated flows, screenshots, visual comparison, Logcat, permissions, security scans, performance checks, and APK/AAB metadata.

The final report must identify what passed, what failed, what was repaired, what could not be tested, the source revision, the active checkpoint, the artifact checksum, and any unresolved warnings. No model claim may mark a requirement complete without a corresponding evidence record.

## 37. Privacy-Scoped Memory, Replay, and Recovery History

Memory must be divided into session memory, project memory, runtime-improvement memory, and credential storage. Every memory entry must include source, confidence, project scope, timestamp, revision, retention policy, and deletion support. Credentials, signing keys, raw secrets, and unclassified private content must never enter semantic memory.

Users must be able to reopen a completed or failed session, inspect the task and worker timeline, compare preview revisions, rerun validation, fork a failed task into a new strategy, replay a task with an approved provider, restore a checkpoint, download APK/AAB evidence, and inspect why the technology resolver selected a particular implementation.

## 38. Production Windows Host Requirements

The desktop host must use backend-only file access, explicit capability permissions, atomic state writes, file locking, versioned migrations, crash recovery, offline startup, prerequisite validation, signed per-user installers, upgrade rollback, state preservation, memory-leak testing, large-project virtualization, local editor assets, and privacy-filtered local logs.

Provider unavailability must not prevent the host from opening projects, history, checkpoints, and settings. Execution must be disabled or marked unavailable until an approved provider is ready.

## 39. User-Facing Productivity Features

The core workspace must provide one-click goal launch, live task tree beside the Android preview, pause/resume/cancel/fork/retry-from-checkpoint, a technology rationale panel, a changed-files timeline, device-matrix testing, visual comparison, build-health status, an APK/AAB artifact center, recovery explanations, an editable project-memory view, task replay, a privacy/network context panel, and an environment-repair center.

These features expose the runtime’s state without forcing the user to understand internal worker orchestration. The user gives the goal; Nirman manages the complexity.

## 40. Production Readiness Principle

> **Nirman must be autonomous in execution and recovery, but deterministic in authority.**

The application may continue automatically through routine project-local work, but no model, worker, skill, hook, or external tool may grant permission, bypass the sandbox, delete recovery state, mark work complete without evidence, promote an unvalidated candidate, or suppress a hard safety termination.


---

# 33. Integrated Android Construction and Runtime Contracts

This section incorporates the strongest reusable construction and runtime principles identified in the Sync-AI reference set. It does not change Nirman’s product scope: Nirman remains a Windows-first desktop host that generates Android applications only. No user-facing framework catalog is exposed, and the AI remains responsible for selecting and composing the Android implementation.

## 33.1 AndroidConstructionContract

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
| Device matrix | Emulator profiles, physical devices, API levels, orientations, densities, tablet/phone coverage |
| Validation model | Unit, integration, UI, visual, accessibility, performance, security, runtime, and release checks |
| Artifact model | APK/AAB variants, signing policy, version code, checksums, evidence requirements, export destinations |

The contract MUST use explicit schema versions, reject unknown fields where strict validation is required, record source references for inferred fields, and distinguish user-provided facts from model inferences. A worker MUST NOT invent a contract field absent from the canonical schema.

## 33.2 ConstructionTransaction

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

## 33.3 Pure Reducer and Replayable State

The autonomous session state MUST be reconstructed by a deterministic reducer:

```text
previous durable state + validated runtime event = next durable state
```

The reducer MUST be side-effect free. Filesystem writes, process launch, provider calls, emulator commands, and artifact operations belong to command handlers that emit validated events. This enables crash recovery, deterministic replay, impossible-transition detection, and property-based testing.

The reducer MUST reject events for unknown sessions or tasks, stale project revisions, completion events without required evidence, promotion events without artifact checksums, worker events from expired leases, preview events for unrelated revisions, and transitions that bypass checkpoint, policy, or validation gates.

## 33.4 Recovery Governance

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

---

## References

[1]: /home/ubuntu/upload/AI_RUNTIME_MODEL.md "AI Runtime Model reference"
[2]: /home/ubuntu/upload/ORCHESTRATION_ENGINE.md "Orchestration Engine reference"
[3]: /home/ubuntu/upload/AGENT_EXECUTION_CONTRACT.md "Agent Execution Contract reference"
[4]: /home/ubuntu/upload/CODE_INTELLIGENCE.md "Code Intelligence reference"
[5]: /home/ubuntu/upload/EXECUTION_ENVIRONMENT.md "Execution Environment reference"
[6]: /home/ubuntu/upload/TOOLCHAIN_MANIFEST.md "Toolchain Manifest reference"
[7]: /home/ubuntu/upload/TOOLCHAIN_ISOLATION.md "Toolchain Isolation reference"
[8]: /home/ubuntu/upload/PREVIEW_SYSTEM.md "Preview System reference"
[9]: /home/ubuntu/upload/REPAIR_PATTERNS.md "Repair Patterns reference"
[10]: /home/ubuntu/upload/PLATFORM_REQUIREMENTS_ENGINE.md "Platform Requirements Engine reference"
[11]: /home/ubuntu/upload/BRANDING_INFERENCE_HEURISTICS.md "Branding Inference reference"
[12]: /home/ubuntu/upload/AI_SERVICE_LAYER.md "AI Service Layer reference"
[13]: /home/ubuntu/upload/AI_MINI_SERVICE_IMPLEMENTATION.md "AI Mini Service Implementation reference"
[14]: /home/ubuntu/upload/UI_IMPLEMENTATION.md "UI Implementation reference"
[15]: /home/ubuntu/upload/USER_WORKFLOWS.md "User Workflows reference"
[16]: /home/ubuntu/upload/STRUCTURED_SPEC_FORMAT.md "Structured Spec Format reference"
[17]: /home/ubuntu/upload/TARGET_APP_ARCHITECTURE.md "Target App Architecture reference"
[18]: /home/ubuntu/upload/DATA_LAYER_GENERATION.md "Data Layer Generation reference"
[19]: /home/ubuntu/upload/UI_GENERATION_RULES.md "UI Generation Rules reference"
[20]: /home/ubuntu/upload/SYSTEM_ARCHITECTURE.md "System Architecture reference"
[21]: /home/ubuntu/upload/PROJECT_ARCHETYPE_RESOLUTION.md "Project Archetype Resolution reference"
[22]: /home/ubuntu/upload/PROJECT_HANDBOOK.md "Project Handbook reference"
[23]: /home/ubuntu/upload/WINDOWS_PACKAGING_AND_PERMISSION_AUTOMATION.md "Packaging reference"
[24]: /home/ubuntu/upload/AI_AGENTS_AND_PLANNING.md "AI Agents and Planning reference"

The Windows-specific implementation details from the reference documents are not generated-target requirements for Nirman and are intentionally excluded from this Android-only specification.

---

# 34. Android Code Intelligence and Mutation Contract

## 34.1 Language-Neutral Android Code Intelligence

Nirman MUST use a language-neutral Android code-intelligence layer with adapters for Kotlin, Java, XML, Android manifests, Gradle Kotlin DSL, Gradle Groovy, TypeScript, JavaScript, C/C++ native modules, JSON, YAML, TOML, SQL, and lockfiles.

The graph MUST track files, modules, symbols, references, Gradle dependencies, manifest permissions, resource references, navigation routes, native-module boundaries, test-to-source relationships, API-level compatibility, and generated artifacts. Lightweight indexing may support discovery and browsing; full semantic indexing is required before high-impact mutation, reconciliation, packaging, signing, or promotion.

## 34.2 Structured Mutation Broker

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

## 34.3 Project Impact Graph

Before a refinement, Nirman MUST calculate affected files, modules, resources, tests, permissions, preview surfaces, and artifact outputs. The impact graph MUST support incremental indexing, affected-test selection, dependency conflict analysis, navigation and resource reachability, manifest/API usage correlation, long-horizon map sharding, checkpoint-aware invalidation, and reconciliation conflict detection.

---

# 35. Preview, Branding, and Data-Layer Requirements

## 35.1 Preview Fallback Matrix

The live preview coordinator MUST select a preview mode appropriate to the selected Android technology and current revision.

| Preview mode | Use case | Required evidence |
|---|---|---|
| Incremental emulator install | Native changes that compile successfully | Install result, process health, screenshot |
| Compose reload | Compose-compatible UI change | Reload event, state continuity, screenshot |
| React Native/Expo fast refresh | JavaScript/TypeScript-only change | Metro/Expo health, rendered screen, screenshot |
| Full APK reinstall | Manifest, resource, dependency, native, or major build change | APK hash, install, launch, screenshot |
| Physical device preview | User-approved connected device | Device identity, install, launch, capture, Logcat |
| Headless smoke test | Preview device unavailable | Test output, runtime logs, health result |
| Diagnostic/source preview | Build unavailable during recovery | Diagnostics only; cannot satisfy completion |

Every preview is bound to PreviewRevision, project revision, device identity, build variant, and technology plan. A stale preview MUST be visibly labeled and MUST NOT satisfy final completion gates.

## 35.2 Android BrandManifest

Nirman may infer branding from the application contract, screenshots, domain semantics, and user preferences, but it MUST not use Windows-specific visual assumptions. BrandManifest covers display name, semantic description, light/dark colors, typography, spacing, adaptive icon assets, splash assets, notification icons, empty states, density variants, accessibility contrast, provenance, prompt hash, provider/model ID, and output hashes.

AI image seeds are recorded as inputs, but exact reproducibility MUST be verified from output hashes rather than assumed. Content-addressed caching and explicit regeneration records are required.

## 35.3 Android Data-Layer Resolver

Nirman MUST choose a data strategy from the application contract rather than enforcing one fixed database technology. Valid choices include Room with SQLite, direct SQLite, DataStore, encrypted local storage, a justified alternative local store, network cache/synchronization, or a composed strategy.

The resolver MUST produce migration rules, corruption recovery rules, seed-data policy, offline behavior, encryption requirements, test fixtures, and an evidence plan. The selected data strategy becomes part of the technology plan and cannot be changed by a worker without a versioned plan update and reconciliation.

---

# 36. Autonomous UX, Decision Trace, and Resource Governance

## 36.1 Progressive Disclosure

Nirman MUST hide unnecessary implementation complexity by default without hiding truth. The UI provides three levels:

| Mode | Visible information |
|---|---|
| Calm | Current phase, meaningful progress, live preview, latest update, working/waiting state |
| Inspect | Task graph, workers, terminal summaries, changed files, checkpoints, devices, recovery, evidence |
| Developer | Structured diagnostics, provider/model provenance, decision trace, command details, environment snapshot, replay controls |

Raw secrets, private keys, and unfiltered prompts are never displayed. Blocked, waiting, recovering, and safely-failed states MUST be explicit.

## 36.2 DecisionTrace

For each material autonomous decision, Nirman records a concise DecisionTrace containing decision ID, session/task/worker IDs, input references, constraints, candidate actions, selected action, deterministic policy checks, provider/model provenance, confidence, outcome event, and evidence IDs. Hidden chain-of-thought is not stored or exposed.

## 36.3 ResourceGovernor

The resource governor monitors CPU, memory, disk, checkpoint storage, emulator memory, Gradle memory, worker concurrency, provider concurrency, context size, log volume, build duration, and device slots.

Under pressure it may compact context, reduce concurrency, prune safe caches, stop redundant workers, run affected tests, defer nonessential visual checks, or switch to an approved lighter provider profile. It MUST NOT silently weaken sandboxing, permissions, evidence, signing, or artifact gates.

## 36.4 EnvironmentSnapshot

Every substantial build, recovery cycle, and final artifact MUST include an environment snapshot recording operating-system host metadata, toolchain versions and hashes, relevant environment variables, device identity/API level, provider profile and model metadata without secrets, workspace revision, lockfile hashes, and build flags.

---

# 37. Non-Goals Preserved by This Integration

The following remain explicitly outside Nirman’s generated-target scope: Windows application generation; web application generation; WinUI, WPF, WinForms, Win32, WinRT, MSBuild, MSIX, MSI, or Windows-manifest target generation; Roslyn, XAML, or EF Core as mandatory implementation technologies; a user-facing framework or template catalog; direct model writes to files; unrestricted model shell authority; unauthenticated local provider access; uncontrolled infinite mutation retries; and completion based solely on model claims.

Internal bootstrap scaffolding is permitted only when required to create a valid Android project; it is not a user-facing template limitation and does not constrain the AI’s technology selection.

## 37.1 Product Acceptance Additions

The integration is complete only when a complete AndroidConstructionContract can be created, versioned, validated, and replayed; every mutation is represented by a ConstructionTransaction with a checkpoint and project revision; the session can be reconstructed after forced process termination; a clean-machine build uses only the locked Android toolchain; provider bridge failures are handled without corrupting the session; multi-language changes pass the structured mutation broker; parallel workers reconcile through a serialized commit barrier; Android permission and requirement drift is detected before artifact promotion; preview is revision-bound; resource pressure changes scheduling without weakening safety; and a completed APK/AAB contains checksums, environment snapshot, validation evidence, source revision, and artifact provenance.


---

# 38. Integrated Android Workflow and Quality Intelligence

## 38.1 IntegratedAndroidWorkflowCoordinator

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
APK/AAB packaging and evidence promotion
```

The coordinator MUST persist each boundary as a durable event and MUST be able to resume from the last validated boundary after a supervisor, worker, provider, emulator, or host interruption.

## 38.2 PreflightReport and feasibility gate

Before expensive generation begins, Nirman MUST produce a `PreflightReport`. The report evaluates the selected or candidate technology plan against the local environment, project constraints, provider capabilities, privacy policy, device availability, and expected validation work.

| Preflight area | Required checks |
|---|---|
| Provider | Authentication, protocol, model capabilities, context limit, vision/tool support, privacy policy |
| Toolchain | JDK, Gradle, Android Gradle Plugin, Kotlin, SDK, build tools, platform tools, NDK/CMake, Node/Metro/Expo when needed |
| Workspace | Writable scope, disk space, project fingerprint, lockfiles, credentials exclusion, checkpoint capacity |
| Device | Emulator/physical device, API level, ABI, storage, ADB health, orientation, required hardware capabilities |
| Dependencies | Availability, compatibility, vulnerability/license policy, lockfile status, native build requirements |
| Requirements | Permissions, manifest entries, background rules, accessibility, localization, offline behavior, signing prerequisites |
| Resource forecast | CPU, memory, disk, emulator memory, worker count, provider concurrency, expected validation stages |

Each risk records severity, probability, affected phase, evidence, mitigation, fallback, and whether autonomous repair is permitted. Routine toolchain or cache repair may proceed under policy. Credentials, privileged access, unavailable required devices, and policy restrictions become explicit waiting or blocked conditions rather than endless retries.

## 38.3 AndroidQualityGate

Before artifact promotion, independent review workers MUST evaluate correctness, architecture, security, dependencies, runtime behavior, visual fidelity, accessibility, performance, test coverage, and release integrity.

| Finding class | Completion behavior |
|---|---|
| Blocking | Must be repaired, independently waived by an allowed policy, or prevent artifact promotion |
| Warning | May proceed only with recorded rationale and evidence |
| Informational | Recorded for improvement and does not block completion |

The quality gate MUST be independent from the worker that produced the implementation. A quality score alone is never completion evidence.

## 38.4 FailureModeRecord

Nirman MUST maintain a proactive Android failure-mode catalogue. Every important failure mode has a trigger, prevention check, classifier, recovery strategy, scope, stop condition, and evidence requirement.

Initial failure families include toolchain incompatibility, missing SDK components, dependency conflicts, lockfile drift, resource linking failures, manifest merge failures, duplicate classes, DEX/R8 errors, native-module failures, emulator and ADB failures, install failures, runtime crashes, ANRs, permission denials, offline-data corruption, visual regressions, inaccessible controls, signing failures, and invalid APK/AAB metadata.

## 38.5 Acceptance-test traceability

Every mandatory requirement MUST map to at least one executable acceptance criterion and one validation path.

```text
Requirement → acceptance criterion → test → execution result → evidence → artifact revision
```

The traceability matrix records skipped, blocked, flaky, and passing tests honestly. A final artifact cannot claim complete implementation when a mandatory requirement has no executable validation or has unresolved blocking evidence.

## 38.6 Architecture and contract drift

After every major transaction, Nirman MUST compare the project against the approved `AndroidConstructionContract` and `AndroidTechnologyPlan`. Drift detection identifies missing features, undocumented permissions, unreachable screens, data models without migrations, acceptance criteria without tests, dependencies outside the approved plan, unauthorized architecture changes, stale generated files, and preview or artifact outputs from unrelated revisions.

Drift findings are classified as blocking, repairable, warning, or informational. A worker cannot silently update the contract to make drift disappear; contract changes require a versioned plan update and reconciliation event.

## 38.7 Project handbook and release intelligence

Each managed Android workspace MUST contain a concise generated project handbook describing purpose, selected technology plan, modules, commands, toolchain lock, environment assumptions, privacy rules, permissions, build/test instructions, known limitations, current revision, and recovery notes.

Each promoted APK/AAB MUST have a release-intelligence report containing dependency inventory, permission inventory, data-handling summary, test and device results, performance summary, known warnings, artifact hashes, signing status, source revision, toolchain lock, and environment snapshot.

## 38.8 Worker quality metrics and validated repair promotion

Nirman SHOULD measure worker and strategy quality using success rate, regression rate, time-to-evidence, false-positive review rate, repair reuse rate, handoff completeness, affected-test precision, and rollback frequency. Metrics are for routing and improvement; they do not grant permissions.

A learned repair or pattern may enter the trusted registry only after repeated successful validation on the originating project and independent fixtures. The stored record includes failure fingerprint, environment, strategy, changed scope, validation evidence, regression results, and confidence. Model suggestions remain untrusted until promoted by deterministic evidence.

## 38.9 Bounded structured reasoning

Nirman MAY use prompt normalization, self-critique, logical consistency checks, alternative-solution analysis, risk prediction, reflection, and strategy scoring. These services MUST return bounded structured outputs such as assumptions, alternatives, selected action, constraints, confidence, and evidence references. Hidden chain-of-thought MUST NOT be stored or shown. No reasoning service may override the runtime authorities.

---

# 39. Product Scope Decisions from the Integrated Review

The following are explicitly not adopted: web application generation, Windows application generation, PWA delivery, a universal web-wrapper architecture, exposed hidden reasoning transcripts, unbounded recursive worker spawning, automatic remote publication, and completion claims based on module counts or unsupported implementation percentages.

Nirman uses native Windows isolation as its required execution model: restricted tokens, Windows Job Objects, ACL-scoped workspaces, environment filtering, process-tree supervision, resource quotas, toolchain isolation, and disposable Android emulator snapshots. Remote Git operations, publication, store submission, and release signing remain explicit policy-controlled operations.

The central product rule remains:

> **One instruction plus optional screenshots should produce a complete, validated Android application through a durable, recoverable, inspectable, and evidence-backed autonomous workflow.**

## 39.1 Additional acceptance criteria

1. A preflight report identifies blockers before expensive generation.
2. The integrated coordinator resumes from durable boundaries after interruption.
3. Independent quality workers can block promotion with evidence-backed findings.
4. Every mandatory requirement is traceable to a test and evidence record.
5. Architecture or contract drift cannot be silently ignored.
6. A generated project contains a concise handbook and a promoted artifact contains a release-intelligence report.
7. Learned repairs require independent validation before trusted reuse.
8. Resource pressure and model strategy changes never weaken runtime safety or evidence gates.


---

# 40. Private Internal Reasoning and Visible Structured Reasoning Stream

## 40.1 Product decision

Nirman MAY use private internal model reasoning to support planning, hypothesis generation, self-critique, alternative comparison, error diagnosis, and strategy selection. Private reasoning is an internal computation boundary; it is not displayed to users, persisted as a verbatim transcript, treated as evidence, or granted runtime authority.

Nirman MUST provide a separate live `ReasoningStream` so the user can see what the system is doing during long autonomous sessions. The stream contains concise, useful, filtered summaries rather than raw hidden chain-of-thought.

> **Private reasoning may guide the strategy. Visible structured reasoning explains the strategy. Deterministic runtime authorities control execution.**

## 40.2 Visible reasoning event types

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
| `WAITING` | Explain a blocked or waiting condition | “Waiting for the approved physical device to reconnect.” |
| `COMPLETION` | Summarize validated output | “APK and AAB passed the required gates and are ready for export.” |

Every event MUST contain a concise title, human-readable summary, event sequence, session/task/worker IDs, project revision, timestamp, status, provenance references, and evidence IDs when applicable.

## 40.3 Stream behavior

The stream MUST be available while the desktop UI is open and MUST remain recoverable after reconnect, minimization, sleep, reboot, provider restart, or control-plane restart. The UI must show the newest event immediately while retaining a scrollable session history.

The user can pause visual auto-scroll without pausing execution, collapse repeated low-value events, filter by worker or phase, expand evidence links, and switch between Calm, Inspect, and Developer presentation levels. The stream must clearly distinguish model reasoning summaries, runtime actions, observations, policy decisions, recovery, and evidence.

Streaming must not imply that a model has authority. A visible `DECISION` event means that a strategy was selected; it does not mean that a tool, mutation, permission, or artifact promotion was authorized. The runtime must emit a separate policy and execution event for those actions.

## 40.4 Privacy and safety filters

Before a reasoning summary reaches the UI or durable history, `ReasoningStreamFilter` MUST remove or mask API keys, access tokens, private keys, passwords, cookies, personally identifying data, complete source-file contents, sensitive user data, hidden system instructions, raw provider messages, and private internal reasoning.

The stream must not reveal unrestricted shell commands when the command contains secrets or sensitive paths. It may show a safe command category, redacted arguments, operation ID, and result. Detailed diagnostics remain available only through policy-controlled Developer mode and still undergo redaction.

## 40.5 Honest status semantics

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

## 40.6 User controls

The user may hide or show the stream, change its detail level, filter event categories, inspect evidence, pause new autonomous work, cancel the session, request a summary, or open the relevant checkpoint. Hiding the stream does not stop execution or delete history.

The user cannot edit a stream event, mark an unsupported event as evidence, approve a mutation by editing text, or use the stream to bypass policy. Any approval remains a separate explicit runtime action.

## 40.7 Acceptance criteria

1. A long-running session streams understanding, plan, action, observation, recovery, evidence, and next-step events in order.
2. Stream reconnection resumes from the last acknowledged sequence without duplicate or missing events.
3. Private reasoning never appears verbatim in the UI, event store, logs, exports, or provider handoffs.
4. Secrets, sensitive paths, source contents, and raw provider messages are redacted before display and persistence.
5. Visible decisions are linked to runtime policy, execution, and evidence events.
6. Waiting, blocked, stale, complete, and safely-failed conditions are visually distinct from working.
7. Stream presentation can be changed without changing execution behavior.
8. Replay reconstructs the visible stream from durable filtered events without re-running model reasoning.


---

# 41. Mandatory Brand and Asset Completion Gate

## 41.1 Product requirement

Branding and visual assets are first-class Android product requirements. When the user requests a logo, icon, splash screen, notification icon, illustration, branded color system, or visual identity, Nirman MUST generate or safely derive the requested assets, integrate them into the Android project, show them in the live preview, and validate them before the APK/AAB can be promoted.

The implementation must not finish at source-code generation while leaving the application with missing, generic, stale, or unintegrated branding.

## 41.2 BrandAssetPipeline

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
APK/AAB asset inspection
        ↓
BrandAssetCompletionGate
```

The pipeline covers the application label, adaptive launcher icon, legacy launcher variants where required, monochrome icon where supported, splash screen, notification icon, in-app logo, color system, theme tokens, typography intent, empty-state art, onboarding illustrations, and other assets explicitly requested by the user.

## 41.3 BrandManifest and AssetManifest

`BrandManifest` records display name, semantic brand description, logo/icon/splash intent, source screenshot references, light and dark colors, typography and spacing intent, theme behavior, asset requirements, accessibility expectations, and manifest version.

`AssetManifest` records each asset’s ID, type, BrandManifest version, source intent, screenshot references, output path, format, dimensions, density or adaptive variant, content hash, provider/model metadata, generation status, integration status, validation status, and regeneration history.

Provider/model metadata and prompt hashes are retained for provenance, but raw prompts, private data, and secrets are not exposed in the user-facing stream or ordinary logs. A seed may be recorded when available, but exact reproducibility is verified from output hashes rather than assumed.

## 41.4 Asset completion rules

The final artifact MUST NOT be marked complete when a requested asset is missing, references an invalid path, is not packaged, is stale relative to the source revision, fails format/dimension/transparency/contrast checks, or has not been verified in the active preview. A temporary placeholder may be used during recovery, but it cannot silently satisfy the final gate when branded assets were requested.

The gate must inspect the built APK/AAB, not only the workspace. It must confirm that launcher resources, splash resources, notification assets, in-app assets, theme resources, and referenced fonts or illustrations are present and reachable in the final artifact.

## 41.5 Asset change behavior

When the user requests a branding change, Nirman creates a new BrandManifest revision, regenerates only affected assets, updates Android resources, refreshes the preview, invalidates stale asset evidence, and reruns the asset gate. Unaffected source code and assets should remain unchanged where impact analysis proves they are independent.

## 41.6 Visible asset progress

The reasoning stream should show safe events such as:

```text
Understanding: “You requested a fitness brand named FitPulse.”
Brand decision: “Using an energetic green palette with a heart-and-lightning symbol.”
Asset action: “Generating adaptive launcher icon variants.”
Asset action: “Integrating the icon into Android resources.”
Validation: “Launcher icon and splash screen verified on the API 35 emulator.”
Next step: “Running final APK asset inspection.”
```

## 41.7 Acceptance criteria

1. A user request for branded assets creates a versioned BrandManifest and AssetManifest.
2. Requested launcher, adaptive, monochrome, splash, notification, in-app, and theme assets are generated or explicitly governed by a fallback record.
3. All assets are integrated into the correct Android resource locations and referenced by the project.
4. The active PreviewRevision displays the current asset revision.
5. The built APK/AAB is inspected for asset presence, reachability, and content hashes.
6. Missing, stale, invalid, unintegrated, or placeholder-only requested assets block final completion.
7. Branding changes regenerate only affected assets and invalidate stale evidence.
8. Asset generation, integration, validation, fallback, and release results are visible in the structured reasoning stream and retained in replayable evidence.


---

# 42. Locked Nirman Implementation Stack and Executable Architecture

## 42.1 Stack decision

The following stack is the implementation baseline for Nirman v1. It does not change the Android-only generated target.

| Layer | Locked implementation |
|---|---|
| Windows desktop shell | Tauri 2 |
| Frontend | React, TypeScript, and Vite |
| Styling | Tailwind CSS and shadcn/ui |
| Presentation state | Zustand or equivalent presentation-only state layer |
| Core runtime | Rust with Tokio |
| Control plane | Rust authoritative supervisor and runtime services |
| Local database | SQLite with versioned migrations |
| Initial database access | SQLx preferred; rusqlite remains an evaluated alternative if isolated safely |
| Initial IPC | Typed Tauri commands and events |
| Durable event stream | Tauri events first, authenticated reconnectable loopback transport where required |
| Editor | CodeMirror 6 for the first implementation |
| Terminal renderer | xterm.js |
| Windows terminal runtime | Native ConPTY supervised by Rust |
| Worker execution | Rust-supervised child processes with leases and scoped capabilities |
| Windows isolation | Restricted tokens, Job Objects, ACL workspaces, environment filtering, process supervision, quotas |
| Credentials | Windows Credential Manager and DPAPI-backed secure storage |
| Version control | Git and Git worktrees |
| Android toolchain | JDK, Gradle, AGP, Android SDK, ADB, emulator, NDK/CMake when required |
| JavaScript Android toolchain | Node and npm/pnpm/yarn, Metro, Expo/React Native only when selected |
| Packaging | Tauri Windows `.exe` installer, with optional MSI packaging |

Nirman orchestrates the Android ecosystem; it does not replace JDK, Gradle, AGP, Android SDK, ADB, emulator, Node, Metro, Expo, native compilers, or Git.

## 42.2 Two-executable production architecture

The first vertical slice may embed the control plane in the Tauri Rust backend to reduce initial process complexity. The production durable-autonomy architecture separates presentation from the long-running supervisor:

```text
Nirman.exe
├── chat and project navigation
├── files and CodeMirror editor
├── preview presentation
├── task graph and reasoning stream
├── settings and user controls
└── authenticated supervisor connection

NirmanSupervisor.exe
├── lifecycle authority
├── SQLite execution ledger
├── task scheduler and worker registry
├── policy and tool broker
├── provider gateway
├── persistent terminals and ConPTY
├── Android toolchain and device runtime
├── checkpoints and Git worktrees
├── recovery and resource governance
├── evidence and artifact authorities
└── preview and Android workflow coordinator
```

`Nirman.exe` is a reconnectable client. It must not own authoritative task state, credentials, lifecycle, worker leases, filesystem authority, process supervision, recovery, evidence, or artifact promotion. `NirmanSupervisor.exe` starts with Windows user login when eligible work exists, survives UI closure, scans SQLite after reboot or sleep/resume, and allows the UI to reconnect later.

## 42.3 User-visible implementation contract

Nirman should feel like one application even when the supervisor is a separate executable. The UI must show supervisor health, connection state, session state, reasoning stream, task progress, terminal summaries, preview revision, evidence, and recovery status. Supervisor installation, update, version handshake, and graceful shutdown are runtime concerns and must not require users to manually operate a second application.

## 42.4 First-release editor and terminal boundaries

CodeMirror 6 is the first editor implementation because Nirman’s primary product is autonomous construction, preview, validation, recovery, and artifact delivery rather than a full standalone IDE. Monaco may be evaluated later without changing the control-plane architecture.

xterm.js is only a terminal renderer. Rust owns ConPTY sessions, shell profiles, process trees, input policy, output capture, cancellation, resource limits, and recovery. Supported shells may include PowerShell, `cmd.exe`, Git Bash, or another explicitly approved profile.

## 42.5 Completion invariants

The stack is considered correctly implemented only when the UI can restart without losing a session, the supervisor can continue without the UI, Android toolchains execute through supervised local processes, model proposals pass through ModelGateway, ToolBroker, and PolicyAuthority, and APK/AAB promotion remains evidence-backed. No framework selector, web target, Windows generated target, or cloud execution environment is introduced.


---

# 43. Core Agent Execution Kernel and Autonomous Loop Contract

## 43.1 Purpose

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

## 43.2 Agent loop states

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

## 43.3 Progress evaluation

After every meaningful observation, the kernel must determine whether the current goal is progressing, blocked, contradicted, unsafe, stale, or satisfied. Progress evaluation must consider requirement coverage, changed files, test results, preview revision, environment capability state, worker handoffs, unresolved uncertainty, failure fingerprints, resource pressure, and artifact readiness.

Completion is permitted only when the appropriate requirement, test, preview, device, quality, branding, and APK/AAB evidence gates pass. A model statement that a task is complete is never sufficient evidence.

## 43.4 SkillRuntime and skill composition

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

## 43.5 Agent profiles and dynamic worker instances

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

## 43.6 SwarmPlanner and DelegationProtocol

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

## 43.7 KnowledgeLedger and TaskBlackboard

Workers must communicate through typed, scoped knowledge rather than a shared mutable prompt or unbounded common memory. Nirman must maintain a `KnowledgeLedger` and a task-scoped `TaskBlackboard` containing goals, requirements, architecture facts, decisions, constraints, assumptions, active workers, completed work, blocked work, findings, conflicts, evidence, known failures, and next actions.

A `KnowledgeArtifact` may be a finding, decision, constraint, assumption, architecture fact, failure pattern, test result, artifact, or environment fact. It must include the source worker, source task, project revision, confidence, evidence IDs, validity period, and scope.

Workers may read relevant entries, propose artifacts, attach evidence, request changes, and retrieve facts. Only deterministic authorities may commit decisions, mutate the task graph, mark requirements complete, change policy, or promote artifacts.

## 43.8 Workspace leases and stateful ToolSessions

Every isolated worktree, copy-on-write workspace, terminal, ADB session, emulator, debugger, LSP, preview process, and other long-lived execution resource must be represented by an ownership and lifecycle record.

A `WorkspaceLease` must include workspace ID, owner worker, task ID, parent checkpoint, lease state, acquisition time, heartbeat, expiration, cleanup policy, recovery policy, current revision, and stale-owner handling. Lease recovery must prevent orphan worktrees, duplicate ownership, zombie builds, and stale writes.

A `ToolSession` must include session ID, tool type, owner, task and project scope, environment fingerprint, process group, current state, capability scope, input policy, output reference, heartbeat, reconnect policy, cleanup policy, and evidence references. Sessions must support reconnect after worker replacement or UI restart without granting a new scope.

## 43.9 Tool Capability Graph and environment capability planning

Nirman must map goals to required capabilities, then capabilities to skills, workers, tools, and environment prerequisites. For example, an Android BLE application may require BLE APIs, a compatible Android SDK, native modules, Bluetooth permissions, ADB, a physical device or emulator capability, and device validation.

Each required environment capability must be classified as `AVAILABLE`, `REPAIRABLE`, `USER_REQUIRED`, or `UNAVAILABLE` before the task commits to a validation path. The planner must surface the distinction early instead of discovering an impossible prerequisite after a long build.

## 43.10 ValidationPlanner and mutation/regression intelligence

The `ValidationPlanner` must choose checks from changed files, changed symbols, call graph, route graph, dependency graph, requirements, acceptance criteria, project type, risk level, previous failures, device profiles, and available resources.

A change to an Android screen, repository, permission, navigation route, data model, manifest, native module, or build file must expand validation to the affected behavior. The planner may select focused checks for low-risk changes and automatically expand to instrumentation, accessibility, security, visual, device, performance, regression, and release checks for high-risk changes.

The planner must emit a traceability chain:

```text
Requirement
  ↓
Acceptance criterion
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
APK/AAB artifact
```

## 43.11 Trajectory Replay and Simulation mode

Nirman must provide a side-effect-free `TrajectoryReplayEngine` that can replay a recorded goal, context references, structured model proposals, tool calls, tool results, state changes, observations, and next decisions against a new model, prompt, skill, tool schema, or runtime without touching the real project.

Nirman must also provide a clearly labeled **Simulation/Dry-Run Mode**. It may predict workers, skills, files, commands, permissions, devices, tests, resources, risks, and expected validation, but it must not mutate files, execute commands, start devices, or claim that predicted checks actually ran. Simulation output must be labeled `PREDICTED`, while executed evidence must be labeled `OBSERVED` or `VERIFIED`.

## 43.12 Deadlock, backpressure, cancellation, and pause/resume

The runtime must detect dependency cycles across tasks, workers, resource reservations, approvals, workspace leases, and ToolSessions. A detected deadlock must produce a typed finding and trigger safe recovery, reordering, worker replacement, or a structured decision node.

Swarm execution must apply backpressure when workers compete for Gradle, emulator slots, GPU capacity, physical devices, storage, or provider concurrency. Reservations, priority, fairness, queues, and resource release must be visible in the task graph.

Cancellation must propagate from goal to task graph, workers, skills, ToolSessions, processes, PTY sessions, emulator operations, and pending provider requests. Each layer must support graceful cancellation, forced termination, cleanup, checkpoint preservation, and rollback semantics.

Workers and skills must support independent pause and resume. Pausing must preserve context references, ToolSessions, leases, checkpoints, and unresolved questions while allowing unrelated work to continue.

## 43.13 Decision nodes, uncertainty, contradiction, and plan recompilation

When multiple valid Android architectures or recovery strategies exist, Nirman must represent a `DecisionNode` containing the question, options, evidence, trade-offs, recommendation, impact, and resume conditions. It is distinct from a generic command approval.

The runtime must track uncertainty as first-class state: `KNOWN`, `PROBABLE`, `ASSUMED`, `UNKNOWN`, `CONTRADICTED`, `VERIFIED`, and `BLOCKED`. Each uncertainty record must identify its scope, source, evidence, confidence, expiration, and next resolution action.

A contradiction detector must identify conflicting requirements, stale assumptions, invalidated decisions, changed device constraints, and architecture drift. It must create a controlled decision revision rather than silently selecting whichever statement appeared most recently.

The `PlanCompiler` and `Replanner` must produce plan revisions when evidence, environment, requirements, toolchain, worker availability, or validation results invalidate the current plan. Each plan revision must record `planRevision`, `supersedesPlan`, reason, trigger evidence, affected nodes, and migration/recovery action.

## 43.14 Execution history tiers

Long-running Android sessions must not retain every event, terminal output, screenshot, failed strategy, intermediate plan, and checkpoint in active memory. The `ExecutionHistoryManager` must provide:

| Tier | Contents | Retrieval behavior |
|---|---|---|
| Hot | Current graph, active workers, current plan, latest evidence, unresolved blockers | Always available to the kernel |
| Warm | Recent events, recent terminal summaries, recent checkpoints, recent preview and test results | Loaded on task or worker request |
| Cold | Older events, completed handoffs, historical failures, superseded plans, old screenshots | Retrieved by indexed query or replay request |
| Archived | Content-addressed logs, full traces, old artifacts, crash dumps, retired sessions | Restored explicitly for audit or investigation |

Compaction must preserve semantic summaries, evidence links, revision identity, and replay references. Garbage collection must never delete required completion evidence, active checkpoint parents, unresolved failure evidence, or artifact provenance.

## 43.15 Product acceptance invariants

The AgentExecutionKernel release is complete only when Nirman can run one Android goal through the loop state machine, execute a skill composition, dynamically configure a worker profile, delegate a typed task, exchange knowledge artifacts, lease a workspace, reconnect a ToolSession, plan environment capabilities, select affected validation, replay the trajectory without side effects, simulate the plan without mutation, detect a deadlock, apply backpressure, propagate cancellation, pause and resume a worker, surface a decision node, track uncertainty, recompile a plan, compact execution history, and deliver an evidence-backed APK/AAB.

The user-facing stream must show concise structured events for these transitions without exposing private chain-of-thought. The deterministic runtime remains the only authority over mutation, tools, permissions, lifecycle, evidence, recovery, and artifact promotion.

## References

[1]: /home/ubuntu/upload/pasted_content.txt "User-provided Nirman runtime architecture review"
