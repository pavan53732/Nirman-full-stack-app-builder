# Nirman Glossary

> Reference only (ADR-220). Each entry is one sentence of orientation followed by the section that defines the term; the definition lives there, not here. This document holds no authority: it cannot create, weaken, or reinterpret a contract, and a conflict between an entry and its cited section is resolved by the section. Entries are alphabetical within each group. Citations use the repository's qualifiers: `BS §` = `nirman-build-spec.md`, `TA §` = `nirman-technical-architecture.md`, `SCHEMAS §` = `nirman-schemas.md`, `Mnn` = a milestone block in `nirman-milestones.md`, `ADR-nnn` = a record in `nirman-adrs.md`.

## 1. Product and scope

**Android-only target** — Every generated application targets Android alone; `Project.targetPlatforms` is invariantly `["android"]` and the rule is machine-checked. — BS §5; ADR-180.

**Embedded emulator preview** — The generated app renders inside Nirman's own window through a Nirman-managed local Android emulator (the Google Android Emulator, provisioned by Nirman on first launch, never bundled or built by Nirman); no physical device plays any role. — BS §69; TA §10; TA §49.4; ADR-221.

**Nirman** — A Windows desktop application that autonomously plans, builds, previews, tests, repairs, packages, and exports Android applications from product intent using a user-configured cloud AI provider. — BS §1; BS §20.

**Nirman.exe** — The C#/.NET WinUI 3 user-facing process; a presentation-only projection client of the control plane. — TA §57; ADR-108; ADR-201.

**NirmanSupervisor.exe** — The Rust/Tokio control-plane process that owns durable truth, process supervision, worker leases, checkpoints, recovery, evidence, and preview and artifact promotion. — TA §57; ADR-111.

**NirmanWorker.exe** — The Rust reasoning host the supervisor spawns once per worker lease: an AppContainer process in its own Job Object with no authority, credential, file, socket, or child, whose only input and output is its `WorkerConnection`. — TA §3.5; TA §57.11; ADR-222.

**WorkerConnection** — The per-lease authenticated named pipe between the supervisor's `WorkerRuntime` and one `NirmanWorker.exe`, carrying `MODEL_CALL`, `PROPOSAL`, results, artifacts, heartbeats, and cancellation. — TA §57.11; SCHEMAS §2.90; ADR-222.

**SupervisorConnection** — The authenticated named-pipe channel through which `Nirman.exe` talks to `NirmanSupervisor.exe`. — TA §14; ADR-117.

## 2. Registries and document machinery

**ADR (architecture decision record)** — An accepted, numbered decision with rationale, consequences, and (from ADR-140) the contracts it locks; records live in `nirman-adrs.md`, the writing process in `nirman-decisions.md`. — ADR-220; `nirman-decisions.md`.

**CanonicalSchemaRegistry** — The single list of registered schema identities; owned by TA §36.1 and held as the list block at SCHEMAS §3.1. — TA §36.1; SCHEMAS §3.1; ADR-189.

**Capability registry** — The table of capability identifiers (`CAP.ANDROID.GENERATE` and its peers) with their required contracts, test identity, evidence identity, and maturity. — BS §5.7.

**ClauseId / Clause Registry** — A stable identifier for one normative clause, with its owning contract, authority section, value, and seal state. — BS §67.12.

**Machine-answerable by default** — The ADR-225 presumption that every situation the autonomous emulator loop reaches is answered by the runtime — perception, scenario synthesis, device hygiene and dialogs, golden snapshots, answer-or-proceed, contract doubles, repair patterns, single-writer shared surfaces, and the preview device rule — so that a `USER_REQUIRED` decision must record the automatic paths that did not apply. — BS §69.10; BS §69.11; ADR-225.

**ScreenModel / ScreenGraph** — The text-native perception channel of the autonomous loop: a normalized, fingerprinted element list derived from the running application's UI hierarchy, and the bounded exploration graph built from it, from which `ScenarioSynthesizer` derives scenarios so no validation waits for a human author. — TA §74.2; TA §62.1; ADR-225.

**Component and authority registry** — The TA §57.12 table that gives every authority and every cross-document component name its kind, crate, owned decisions, committed records, and defining section; alias rows name the owner they stand for and carry no crate. — TA §57.12; ADR-223.

**ContractId / Contract Authority Registry** — A stable identifier for one normative contract with exactly one authoritative section; every other section that addresses it is a declared extension. — BS §67.7; BS §67.8.

**Documentation certification** — The terminal status of `tools/verify_contract_graph.py`: `FAIL`, `DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS`, or `DOCUMENTATION_CERTIFIED`; never a runtime or completion claim. — BS §67.6; BS §67.11; TA §74.5.

**ExtensionDeclaration** — The block a section carries when it adds clauses, schemas, components, or verification to a contract it does not own. — BS §67.13; SCHEMAS §1.32.

**INDEX.md** — Navigation generated by the verifier (`--emit-index`); a stale copy fails certification and the file holds no authority. — ADR-220.

**Owner line** — The `**Owner:** … · **Contract:** … · **Projected at:** …` line above every block in `nirman-schemas.md`; the block inherits the precedence of the owner section it names. — ADR-220; `nirman-schemas.md` introduction.

**Schema projection** — The block-quote line that stands where a schema fence used to be and names the schema, its `nirman-schemas.md` section, and its owner section. — ADR-220.

**Twelve-edge resolution table** — The per-contract row that resolves capability, requirement, build-spec, architecture, schema, test, evidence, milestone, ADR, and failure edges in both traversal directions. — BS §67.9; BS §67.15.

## 3. Lifecycle and state vocabularies

**AssuranceState** — The observation-strength vocabulary (`UNKNOWN` … `VERIFIED` and beyond) kept separate from maturity and lifecycle. — BS §5.7.2.

**Capability status (BS §5.6)** — The derived capability status (`SUPPORTED`, `SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS`, `DEGRADED`, `USER_REQUIRED`, `UNAVAILABLE`, `PLANNED`) computed from maturity and operational inputs, never asserted directly. — BS §5.6; BS §5.7.1.

**CapabilityMaturity** — The `SPECIFIED | IMPLEMENTED | VERIFIED | CERTIFIED | …` vocabulary; a capability is never promoted without runtime evidence. — BS §5.7.2; BS §5.7.9.

**CompletionState / CompletionDecision** — The recorded outcome of the completion predicate, including `NOT_COMPLETE`; certification is not completion. — BS §5.7.2; BS §5.7.7; TA §36.4.

**DeliveryState** — The export lifecycle from `NOT_REQUESTED` to an exported, provenance-complete artifact. — BS §5.7.2; BS §78.

**IntegrationState** — The operationality vocabulary of an external integration (`NOT_REQUIRED` … `BLOCKED`), aggregated in `IntegrationOperationality`. — BS §5.7.2; BS §5.7.5; SCHEMAS §1.3.

**ProductLifecycleState** — The authoritative session lifecycle enum, mapped name by name to the TA §36.2 state machine. — BS §5.7.2; BS §33.2; TA §36.2.

**ReproducibilityLevel** — The vocabulary that grades how repeatable a build or validation is; named as its own field on the evidence contracts. — BS §5.7.2; TA §36.4.

**SessionProviderMode** — The sole vocabulary for a session's provider situation: `PLANNING_ONLY`, `PROVIDER_CONFIGURED`, `PROVIDER_VALIDATED`, `OFFLINE`. — BS §5.7.2; TA §41.

**SigningState** — The signing lifecycle of a build output, from `NOT_REQUIRED` and `UNSIGNED_DEBUG` upward. — BS §5.7.2; BS §5.7.9.

## 4. Runtime records

**AndroidCapabilityProfile** — The internal capability-profile identity (`profileId`, toolchain lock, device matrix, evidence and fixture identities, derived status). — BS §5.7.1; SCHEMAS §1.1.

**AndroidConstructionContract** — The canonical contract that binds a construction session's intent, technology plan, and acceptance to one revision. — BS §42; SCHEMAS §1.54; ADR-158.

**AndroidTechnologyPlan** — The record that resolves a session to one technology composition and toolchain lock through exactly one registered `AndroidTechnologyAdapter`. — BS §80.5.1; TA §73.10; SCHEMAS §1.48.

**AttentionReliabilityProfile** — The per-model measurement of how reliably a provider attends to placed context (`reliableLiteralSpanTokens`, `attendabilityMap`, recall probes). — BS §53.11; TA §19.2; SCHEMAS §1.18; ADR-219.

**AutonomousAndroidSession** — The end-to-end session record from user goal to completion state and provider mode. — BS §29; TA §34; SCHEMAS §1.14.

**BackgroundContinuityRecord / ContinuityDimensions** — The canonical record of the background continuity state machine (`ACTIVE_BACKGROUND`, `UI_DISCONNECTED`, `HOST_SUSPENDED`, …). — BS §77; TA §82; SCHEMAS §1.45; ADR-202.

**CandidateBranch** — A speculative candidate revision explored by the speculation runtime and promoted only with recorded lineage. — BS §65; TA §88; SCHEMAS §1.26.

**ChangeReportRecord / ChangeImpactReport** — Exactly one durable report record per committed transaction and the typed impact projection it exposes. — BS §83; TA §87; SCHEMAS §1.74.

**Checkpoint** — The one canonical checkpoint record with `FILE` and `TASK` tiers, validity, and known-good marking; TA §18 tiers are projections of it. — BS §11; TA §18; SCHEMAS §1.10.

**Content / ContentRevision / ContentRevisionDraft / ContentMutation** — The persisted product-content resource, its admitted revisions, and the draft-only proposal a mutation carries. — BS §81; TA §85; SCHEMAS §1.66; ADR-211.

**ContextPackage** — The placed, gated, and recall-verified context assembled for a provider request. — BS §53; TA §59; SCHEMAS §1.16.

**Conversation / ConversationDecision / ConversationRequirement** — The durable development aggregate with revision-bound Continue semantics and evidence-referencing requirements and decisions. — BS §82; TA §86; SCHEMAS §1.69; ADR-212.

**DeliberationRecord** — The record of a deep-deliberation pass; deliberation depth is adaptive and carries no fixed pass ceiling. — BS §68; TA §72; SCHEMAS §1.33; ADR-218.

**EvidenceRecord / EvidenceDependency** — The canonical evidence node with its identity and dependency fields, and the typed dependency whose invalidation cascades. — BS §5.7.4; BS §37; TA §23.3; SCHEMAS §2.19; SCHEMAS §2.30.

**ExportVerificationRecord** — The single export record (its `APKExportRecord` read-model view has no fields of its own) carrying delivery kind, destination policy, and signing lineage. — BS §78; TA §83; SCHEMAS §2.74; ADR-203.

**ExternalEffectRecord** — The record every external side effect writes, whose `reconciliationState` (`UNKNOWN → RECONCILING → RESOLVED`) forbids retrying an unconfirmed effect. — BS §5.7.6; TA §36.4; SCHEMAS §2.32.

**IntegrationBoundaryContract** — The persisted envelope that identifies source, destination, adapter, authority, operation, and transaction domain for an integration boundary. — BS §70; TA §74; SCHEMAS §1.36.

**IntegrationOperationality** — The aggregated connectivity, authentication, availability, functional, and acceptance states of one integration. — BS §5.7.5; SCHEMAS §1.3.

**PackagingProfile** — The canonical artifact and delivery policy (required APK, optionally declared AAB). — BS §5.7.3; SCHEMAS §1.2.

**PreviewRevision** — The revision-bound record of every preview panel state, with the closed `previewMode` enumeration. — BS §69.4; TA §73; SCHEMAS §1.35.

**PreviewSyncEvent / PreviewProjection / PreviewProjectionReducer** — The durable preview events, the projection they reduce into, and the sole reducer; events apply by sequence and identity, not arrival time. — BS §71; TA §75; SCHEMAS §1.37.

**ProviderProfile / ReasoningCapabilityProfile** — The stored provider configuration (secret references only, never keys) and its discovered reasoning capability; `maxReasoningTokens` is capability metadata, never a budget. — BS §80.5.5; TA §24.2; SCHEMAS §1.62; ADR-208.

**ReasoningArtifact / Hypothesis / CapabilityInvocation / DelegationGrant** — The records of the agent reasoning runtime: what was reasoned, hypothesised, invoked, and delegated under which grant. — BS §66; TA §71; SCHEMAS §1.27.

**RenderTransport / FrameNotice** — The supervisor-owned, per-emulator-session frame transport: loopback gRPC control channel, stamped frames in a shared-memory ring with drop-oldest backpressure, announced by volatile `FrameNotice` messages that are never logged or replayed (frames are pixels; `PreviewSyncEvent`s mark only stream-state changes), presented by PreviewHost on a `SwapChainPanel` and painted live only under a `CONNECTED` projection with a bound stamp. — TA §10.7; BS §71.1; SCHEMAS §2.89.

**SigningIdentityBinding** — The binding between a capability promotion and the signing identity that produced its evidence. — BS §5.7.9; SCHEMAS §1.5.

**SkillPackage / SkillInvocationRecord / SkillAdmission** — A registered platform skill (six v1 bodies under `crates/nirman-skills/skills/`), its invocation record, and its fail-closed admission. — BS §23; BS §79.7; TA §19.1; SCHEMAS §1.12; M119.

**TaskContract / TaskGraph / WorkerMessage** — The declared contract every worker receives, the phased graph of task nodes, and the inter-worker message envelope. — TA §6; BS §80.5.4; SCHEMAS §2.1; SCHEMAS §1.58; SCHEMAS §1.13.

**ToolchainProvisioningManifest / ToolchainProvisioningRecord** — The pinned, signed component list Nirman downloads on first launch, and the evidence record of one provisioning run with its state, licence acceptance, hypervisor action, and readiness frame. — TA §49.4; SCHEMAS §2.87; SCHEMAS §2.88.

**UICommandEnvelope / UIResponseEnvelope / UIErrorEnvelope / ProjectionSnapshot** — The typed frontend–control-plane protocol: command, response, error, and snapshot-plus-event replay. — BS §76; TA §81; ADR-201.

## 5. Authorities, services, and runtime concepts

**AndroidTechnologyAdapter** — The resolution-only adapter (six operations) through which a technology plan reaches a concrete `AndroidBuildAdapter` or `AndroidDeviceAdapter`; it performs no build, install, or observation work itself. — TA §73.10; TA §73.12; TA §73.13.

**Attention placement / recall probes** — Placing context where the model measurably attends (`PlacementPlanner`) and verifying recall with runtime-known probes (`RecallProbeService`) instead of trusting model recall. — BS §53.11; TA §59; ADR-219.

**Authority hierarchy** — The fixed order of who may decide what during execution and recovery; consumers of an artifact form no authority edge. — TA §21; BS §67.7.

**Backtracking / two-tier checkpoints** — File-tier and task-tier restore points that let the runtime rewind without losing evidence lineage. — TA §18; BS §11.

**ConversationResolver** — The single authority that commits a conversation revision (also written `ConversationContinuationResolver`). — TA §86; BS §82.

**Deep deliberation** — Adaptive multi-pass reasoning whose depth is decided by the runtime, never by a pass counter or an AI-usage budget. — BS §68; TA §72; ADR-218.

**Evidence ledger / Task Ledger** — The SQLite execution ledger owned by `NirmanSupervisor.exe`; files are projections of it. — TA §23.3; TA §57.5; ADR-110.

**Execution profiles** — Exactly five sandbox profiles (trusted local, restricted process, high-risk restricted process, disposable/isolated, review-only) applied through native Windows isolation. — BS §26.5; TA §9.

**Local certification** — `tools/verify.sh` / `tools/verify.ps1` and the verifier pair are the authoritative gate; hosted CI is optional and never a certification authority. — ADR-204; M0.

**Planning-only mode / Offline Mode** — Operation without a validated provider, bound to `SessionProviderMode` values and never a global prerequisite. — BS §4; TA §41.

**PreviewCoordinator** — The service that owns preview promotion; `PreviewProjectionReducer` remains the sole projection reducer. — TA §50; TA §75.

**Recovery ladder** — The escalating problem-solving depth applied when work repeats or stalls; repetition feeds the ladder rather than raising a stop verdict. — TA §28; ADR-218.

**Resource integrity (`ResourceIntegrityAuthority`, also `ResourceGovernor`)** — The deterministic authority over physical host resources; AI usage is telemetry only. — BS §72; TA §77; ADR-217; ADR-218.

**Speculation runtime** — Exploration of candidate branches implemented only by TA §88 under `CONTRACT.RUNTIME.SPECULATION`. — BS §65; TA §88.

**Toolchain lock / AndroidToolchainManifest** — The pinned Android toolchain identity recorded per capability profile and project. — BS §5.7.1; TA §49; ADR-163.

**ToolchainProvisioner** — The supervisor service that turns a Windows machine with no JDK, Android SDK, emulator, or system image into a ready toolchain and a snapshotted, frame-proven emulator, with at most three user actions and no installation guide. — TA §49.4; BS §4.2; ADR-221.

**WorkflowCoordinator (also `IntegratedAndroidWorkflowCoordinator`, `AndroidWorkflowCoordinator`)** — The single control-plane service connecting the autonomous Android workflow to the quality-intelligence services. — TA §53; BS §47.

**Worker lease / operation capability** — The renewable lease that fences a session's workers and the single-use capability that authorises a sensitive operation. — TA §36.3; TA §46.

## 6. Testing and evidence identities

**Fixture (`FIX-*`)** — A concrete test fixture (for example `FIX-PROG-01`, the tip calculator) with exact acceptance criteria. — BS §80.6; `nirman-milestones.md` evaluation matrices.

**Milestone (`Mnn`)** — A contract-gated unit of the build plan with deliverables, tests, evidence, and exit gates; every milestone block lives in `nirman-milestones.md`. — `nirman-milestones.md`; ADR-220.

**Test identity (`TEST-*`) / evidence identity (`EV-*`)** — The stable identifiers that tie a capability to the test that proves it and the evidence the test emits. — BS §5.7; BS §67.15.
