# Nirman Schemas

This document is the single location of every fenced field-list schema of the Nirman specification (ADR-220). A schema block holds no authority of its own: the **Owner** line names the Build Spec (BS) or Technical Architecture (TA) section that owns the schema, and the block carries that section's precedence. **Contract** names the ContractId of the owning section in the build spec §67.8 registry, or `—` when the section is not a contract authority. **Projected at** lists the other sections that carried this block before the migration; each of them, and the owner, now carries a projection line of the form `> **Schema projection:** ` naming this section. Where the Build Spec and the Technical Architecture both carried a fence for one name, the block below is the merged one: the larger field set, the more specific type annotations, and every comment of the other copy kept in parentheses. A field line ending with `(<document> §x addition; build spec §67.11)` names a field that only the other document's copy carried; it is part of the block, and a field count stated by the owning document (build spec §80.2) refers to the lines without that marker. Field names, types, comments, and the field-count claims of build spec §80.2 are verified against these blocks by `tools/verify_contract_graph.py`. A citation of the form `SCHEMAS §N.k` resolves against this document; a bare `§N` inside a block was qualified with its origin document when the block moved.

Group 1 holds schemas owned by the Build Spec in owner-section order; group 2 holds schemas owned by the Technical Architecture; group 3 holds the `CanonicalSchemaRegistry` list (ADR-189).

## 1. Schemas owned by the Build Spec

### 1.1 AndroidCapabilityProfile

**Owner:** BS §5.7.1 · **Contract:** CONTRACT.RUNTIME.SCOPE · **Projected at:** —

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
- status: derived build spec §5.6 status (SUPPORTED | SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS | DEGRADED | USER_REQUIRED | UNAVAILABLE | PLANNED)
- adapterId
- adapterVersion
- technologyPlanHash
- buildStrategyId
- previewStrategyId
- runtimeStrategyId
- validationStrategyId
- certifiedRevision
```

### 1.2 PackagingProfile

**Owner:** BS §5.7.3 · **Contract:** CONTRACT.RUNTIME.SCOPE · **Projected at:** —

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

### 1.3 IntegrationOperationality

**Owner:** BS §5.7.5 · **Contract:** CONTRACT.RUNTIME.SCOPE · **Projected at:** TA §36.4

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

### 1.4 ProviderContextEnvelope

**Owner:** BS §5.7.8 · **Contract:** CONTRACT.RUNTIME.SCOPE · **Projected at:** —

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

### 1.5 SigningIdentityBinding

**Owner:** BS §5.7.9 · **Contract:** CONTRACT.RUNTIME.SCOPE · **Projected at:** TA §36.5

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

### 1.6 ContractCompatibility

**Owner:** BS §5.7.9 · **Contract:** CONTRACT.RUNTIME.SCOPE · **Projected at:** TA §36.1

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

### 1.7 Project

**Owner:** BS §11.1 · **Contract:** — · **Projected at:** —

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

### 1.8 AgentTask

**Owner:** BS §11.3 · **Contract:** — · **Projected at:** —

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

### 1.9 ActionRecord

**Owner:** BS §11.4 · **Contract:** — · **Projected at:** —

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

### 1.10 Checkpoint

**Owner:** BS §11.5 · **Contract:** — · **Projected at:** —

```text
Checkpoint
- id
- projectId
- taskId
- tier: FILE | TASK
- parentCheckpointId
- workspaceId
- leaseId
- revisionReference
- sourceFingerprint
- filePaths
- contentHashes
- previewRevisionId
- validationSnapshotId
- restoreReference
- validity: VALID | STALE | INVALIDATED
- knownGood: boolean
- retentionClass: INITIAL | LAST_KNOWN_GOOD | RECOVERY_REFERENCED | RECENT | PRUNABLE
- evidenceIds
- description
- createdAt
```

### 1.11 TaskResult

**Owner:** BS §23.9 · **Contract:** CONTRACT.RUNTIME.SKILL · **Projected at:** —

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

### 1.12 SkillPackage

**Owner:** BS §23.11 · **Contract:** CONTRACT.RUNTIME.SKILL · **Projected at:** TA §19.1

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

### 1.13 WorkerMessage

**Owner:** BS §26.2 · **Contract:** — · **Projected at:** TA §6.2

```text
WorkerMessage
- messageId
- taskId
- contractId: (technical architecture §6.2 addition; build spec §67.11)
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

### 1.14 AutonomousAndroidSession

**Owner:** BS §29.2 · **Contract:** — · **Projected at:** TA §34

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
- completionState: CompletionState (build spec §5.7.2)
- providerMode: SessionProviderMode (build spec §5.7.2)
```

### 1.15 StructuredPatch

**Owner:** BS §43.2 · **Contract:** — · **Projected at:** —

```text
StructuredPatch
- patchId
- contextId
- baseRevision
- targetSymbolIds
- anchorHashes
- premises
- operations
- proposedBy
```

### 1.16 ContextPackage

**Owner:** BS §53.3 · **Contract:** CONTRACT.RUNTIME.CONTEXT · **Projected at:** —

```text
ContextPackage
- contextId
- contextRevision
- goalRevision
- planRevision
- projectRevision
- evidenceRevision
- workingSetId
- requiredItems
- activeItems
- supportingItems
- historicalItems
- excludedItems
- fidelityMap
- semanticAnchors
- temporalAnchors
- evidenceFrontier
- uncertainties
- failureFingerprints
- capacityAllocation
- selectionReasons
- omittedForCapacity
- attentionProfileRef
- placementPlan
- attendabilityMap
- recallProbes
- cacheReferences
- integrityHash
```

### 1.17 WorkingSet

**Owner:** BS §53.5 · **Contract:** CONTRACT.RUNTIME.CONTEXT · **Projected at:** —

```text
WorkingSet
- required context
- active context
- supporting context
- historical context
- excluded context
- semantic anchors
- temporal anchors
- evidence anchors
```

### 1.18 AttentionReliabilityProfile

**Owner:** BS §53.11 · **Contract:** CONTRACT.RUNTIME.CONTEXT · **Projected at:** TA §19.2

```text
AttentionReliabilityProfile
- profileId: string
- providerProfileId: string
- modelId: string
- source: DECLARED | PROBED | LEARNED | UNPROFILED
- declaredContextTokens: usize
- reliableLiteralSpanTokens: usize?
- reliableGistSpanTokens: usize?
- positionalRecall: PositionalRecallCell[]
- multiNeedleRecall: { needleCount: usize, passRate: float }?
- distractorSensitivity: LOW | MEDIUM | HIGH | UNKNOWN
- postCompactionRetention: float?
- toolResultRecallDecay: float?
- supportsPrefixCaching: bool
- supportsStructuredCache: bool
- lastProbedAt: timestamp?
- probeFixtureId: string?
- evidenceIds: string[]
- confidence: HIGH | MEDIUM | LOW
```

### 1.19 SemanticReservation

**Owner:** BS §54.2 · **Contract:** CONTRACT.RUNTIME.RESERVATION · **Projected at:** —

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

### 1.20 E2EScenario

**Owner:** BS §56.2 · **Contract:** CONTRACT.RUNTIME.E2E · **Projected at:** —

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

### 1.21 DeviceMatrixEntry

**Owner:** BS §59.2 · **Contract:** CONTRACT.RUNTIME.DEVICE_MATRIX · **Projected at:** —

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

### 1.22 ExternalTrigger

**Owner:** BS §60.2 · **Contract:** CONTRACT.RUNTIME.TRIGGER · **Projected at:** —

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

### 1.23 RuntimeDirective

**Owner:** BS §61.2 · **Contract:** CONTRACT.RUNTIME.DIRECTIVE · **Projected at:** —

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

### 1.24 RegressionCase

**Owner:** BS §62.2 · **Contract:** CONTRACT.RUNTIME.LOCALIZATION · **Projected at:** —

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

### 1.25 ResourceProfile

**Owner:** BS §64.2 · **Contract:** CONTRACT.RUNTIME.PROFILING · **Projected at:** —

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

### 1.26 CandidateBranch

**Owner:** BS §65.2 · **Contract:** CONTRACT.RUNTIME.SPECULATION · **Projected at:** TA §88.2

```text
CandidateBranch
- branchId
- parentRevision
- approach
- isolatedWorkspace
- resourceRequirements
- validationPlan
- outcome: pending | validated | failed | abandoned
- comparableMetrics
- selectedAsWinner: true | false
```

### 1.27 ReasoningArtifact

**Owner:** BS §66.2 · **Contract:** CONTRACT.RUNTIME.REASONING · **Projected at:** TA §71.3

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
- hypotheses: hypothesisId[] (build spec §66.2: HypothesisRef[])
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

### 1.28 ReflectionRecord

**Owner:** BS §66.5 · **Contract:** CONTRACT.RUNTIME.REASONING · **Projected at:** —

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

### 1.29 Hypothesis

**Owner:** BS §66.6 · **Contract:** CONTRACT.RUNTIME.REASONING · **Projected at:** TA §71.5

```text
Hypothesis
- hypothesisId
- taskId: (technical architecture §71.5 addition; build spec §67.11)
- statement
- predictedObservation
- discriminatingTest: { method, targetSurface }
- state: CREATED | TESTED | SUPPORTED | REJECTED | SUPERSEDED
- supportingEvidenceRefs
- refutingEvidenceRefs
- supersededBy
- resultingRepairKind
- createdAtEventId: (technical architecture §71.5 addition; build spec §67.11)
```

### 1.30 CapabilityInvocation

**Owner:** BS §66.7 · **Contract:** CONTRACT.RUNTIME.REASONING · **Projected at:** TA §71.7

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
- evidenceRefs: (technical architecture §71.7 addition; build spec §67.11)
- startedAt: (technical architecture §71.7 addition; build spec §67.11)
- endedAt: (technical architecture §71.7 addition; build spec §67.11)
```

### 1.31 DelegationGrant

**Owner:** BS §66.8 · **Contract:** CONTRACT.RUNTIME.REASONING · **Projected at:** TA §71.7

```text
DelegationGrant
- grantId
- parentAgentId
- childAgentId
- depth
- maxDepth
- capabilityCeiling: capabilityId[]
- resourceRequirements
- executionTimeout
- workspaceScope
- terminationPolicy
- issuedAtEventId: (technical architecture §71.7 addition; build spec §67.11)
- revokedAtEventId: (technical architecture §71.7 addition; build spec §67.11)
```

### 1.32 ExtensionDeclaration

**Owner:** BS §67.7 · **Contract:** CONTRACT.RUNTIME.INVARIANTS · **Projected at:** —

```text
ExtensionDeclaration
- contractId
- authoritySection
- extendingSection
- extensionType: adds_clauses | adds_schema | adds_component | adds_verification
- extendedClauses
- nonOverriddenClauses
```

### 1.33 DeliberationRecord

**Owner:** BS §68.2 · **Contract:** CONTRACT.RUNTIME.DELIBERATION · **Projected at:** TA §72.3

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
- evidenceAcquisitionTriggers: { pass, trigger }[]
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
- outcome: SUFFICIENT | NO_PROGRESS | ESCALATED | ABANDONED
```

### 1.34 SkillDeliberationProfile

**Owner:** BS §68.15 · **Contract:** CONTRACT.RUNTIME.DELIBERATION · **Projected at:** —

```text
SkillDeliberationProfile
- skillId
- minimumEffortLevel
- requiredEvidenceKinds
- requiredCritique: true | false
- preferredModelCapabilities
- requiredExecutionCapacity
- allowedDelegation
- failureStrategies
```

### 1.35 PreviewRevision

**Owner:** BS §69.4 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** TA §73.3

```text
PreviewRevision
- previewRevisionId
- projectId
- projectRevisionId
- activeBranchId
- promotionLineage
- checkpointId
- sourceFingerprint
- contractVersion
- technologyPlanVersion
- assetManifestVersion
- buildVariant
- artifactId
- artifactFingerprint
- deviceId
- androidApiLevel
- deviceStateFingerprint
- applicationStateFingerprint
- environmentStateFingerprint
- previewMode: RN_EXPO_FAST_REFRESH | COMPOSE_RELOAD | INCREMENTAL_APK_INSTALL | FULL_APK_REINSTALL | CONSERVATIVE_FULL_REINSTALL | HEADLESS_SMOKE | DIAGNOSTIC_SOURCE_ONLY | USER_REQUIRED | BLOCKED
- executionTruth
- buildStatus
- installStatus
- launchStatus
- runtimeStatus
- validationStatus
- createdAt
- observedAt
- invalidatedAt
- invalidatedReason
- evidenceIds
```

### 1.36 IntegrationBoundaryContract

**Owner:** BS §70 · **Contract:** CONTRACT.RUNTIME.INTEGRATION_BOUNDARY · **Projected at:** TA §74

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

### 1.37 PreviewSyncEvent

**Owner:** BS §71.1 · **Contract:** CONTRACT.RUNTIME.PREVIEW_SYNC · **Projected at:** —

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
```

### 1.38 PreviewProjection

**Owner:** BS §71.1 · **Contract:** CONTRACT.RUNTIME.PREVIEW_SYNC · **Projected at:** —

```text
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
```

### 1.39 PreviewProjectionReducer

**Owner:** BS §71.1 · **Contract:** CONTRACT.RUNTIME.PREVIEW_SYNC · **Projected at:** —

```text
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
```

### 1.40 PreviewSyncEvidenceRecord

**Owner:** BS §71.1 · **Contract:** CONTRACT.RUNTIME.PREVIEW_SYNC · **Projected at:** —

```text
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

### 1.41 UIResponseEnvelope

**Owner:** BS §76.2 · **Contract:** CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE · **Projected at:** —

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

### 1.42 UIErrorEnvelope

**Owner:** BS §76.2 · **Contract:** CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE · **Projected at:** —

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

### 1.43 EventSubscription

**Owner:** BS §76.3 · **Contract:** CONTRACT.RUNTIME.FRONTEND_CONTROL_PLANE · **Projected at:** —

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

### 1.44 ContinuityDimensions

**Owner:** BS §77.1 · **Contract:** CONTRACT.RUNTIME.BACKGROUND_CONTINUITY · **Projected at:** —

```text
ContinuityDimensions
- uiConnectionState: CONNECTED | DISCONNECTED
- hostState: ONLINE | SUSPENDED | OFFLINE | RECOVERING
- deviceAvailabilityState: AVAILABLE | UNAVAILABLE | REATTACHING
- providerAvailabilityState: AVAILABLE | DEGRADED | UNAVAILABLE | USER_REQUIRED
- leaseState: HELD | EXPIRED | FENCED | REACQUIRING
- reconciliationState: NOT_REQUIRED | REQUIRED | IN_PROGRESS | RESOLVED | BLOCKED
```

### 1.45 BackgroundContinuityRecord

**Owner:** BS §77.2 · **Contract:** CONTRACT.RUNTIME.BACKGROUND_CONTINUITY · **Projected at:** —

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

### 1.46 EnvironmentCapabilityRecord

**Owner:** BS §79.2 · **Contract:** CONTRACT.RUNTIME.PLATFORM_CAPABILITY · **Projected at:** —

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

### 1.47 ValidationEnvironment

**Owner:** BS §79.8 · **Contract:** CONTRACT.RUNTIME.PLATFORM_CAPABILITY · **Projected at:** —

```text
ValidationEnvironment
- environment_id
- platform
- architecture
- toolchain
- runtime
- available_tools
- available_emulator_profiles
- isolation_profile
- network_policy
- fingerprint
- health
- lease
```

### 1.48 AndroidTechnologyPlan

**Owner:** BS §80.5.1 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** TA §10.5

```text
AndroidTechnologyPlan
- planId: string (uuid)
- projectId: string (uuid)
- revision: string (hash)
- capabilityProfileId: string (AndroidCapabilityProfile identity, build spec §5.7.1; the composition and toolchainLock this plan resolves to)
- requestedCapabilities: string[] (CAP.ANDROID.* identifiers the goal requires)
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
- validationPlan: string? (ValidationPlanner plan identity, TA §58.9)
- rationale: string (human-readable explanation of technology choices)
- confidence: float (0.0-1.0)
- alternativesConsidered: { technology: string, rejectionReason: string }[]
- lockedAt: timestamp
- lockedBy: string (worker or authority ID)
```

### 1.49 VisualSpecification

**Owner:** BS §80.5.2 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

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
```

### 1.50 ScreenSpec

**Owner:** BS §80.5.2 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
ScreenSpec
- screenId: string (uuid)
- name: string
- route: string?
- components: ComponentSpec[]
- layout: LayoutSpec?
- interactions: InteractionSpec[]
- states: ScreenState[]
```

### 1.51 ComponentSpec

**Owner:** BS §80.5.2 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
ComponentSpec
- componentId: string (uuid)
- type: string (e.g., "button", "text", "image", "list")
- label: string?
- position: { x: float, y: float, width: float, height: float }
- style: string (style reference)
- behavior: string?
- accessibilityLabel: string?
```

### 1.52 InteractionSpec

**Owner:** BS §80.5.2 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
InteractionSpec
- interactionId: string (uuid)
- trigger: string (e.g., "tap", "swipe", "long_press")
- action: string (e.g., "navigate", "toggle", "submit")
- target: string (screen or component ID)
```

### 1.53 ScreenState

**Owner:** BS §80.5.2 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
ScreenState
- stateId: string (uuid)
- name: string (e.g., "loading", "empty", "error", "loaded")
- conditions: string[]
- components: ComponentSpec[] (overrides for this state)
```

### 1.54 AndroidConstructionContract

**Owner:** BS §80.5.3 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
AndroidConstructionContract
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
```

### 1.55 FeatureModel

**Owner:** BS §80.5.3 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
FeatureModel
- featureId: string (uuid)
- name: string
- description: string
- userStory: string
- dependencies: string[] (feature IDs)
- mandatory: boolean
- acceptanceTests: string[]
- affectedScreens: string[] (screen IDs)
```

### 1.56 DataModel

**Owner:** BS §80.5.3 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
DataModel
- entities: EntitySpec[]
- relationships: RelationshipSpec[]
- persistenceStrategy: ("room" | "sqlite" | "datastore" | "encrypted" | "network_cache" | "composed")
- migrationRules: string[]
- corruptionRecovery: string?
- seedDataPolicy: ("empty" | "fixture" | "none")
- encryptionRequirements: string?
```

### 1.57 IntegrationSpec

**Owner:** BS §80.5.3 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
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

### 1.58 TaskGraph

**Owner:** BS §80.5.4 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** TA §23.1

```text
TaskGraph
- graphId: string (uuid)
- projectId: string (uuid)
- sessionId: string (uuid)
- goalContractId: string (AndroidConstructionContract identity)
- revision: string (hash)
- phases: TaskPhase[]
- dependencies: { fromPhase: string, toPhase: string }[]
- workers: WorkerAssignment[]
- completionConditions: string[]
- lastValidatedCheckpoint: string? (checkpoint id)
- completionEvaluation: string? (CompletionDecision reference)
- createdAt: timestamp
- updatedAt: timestamp
- lockedAt: timestamp
- lockedBy: string (worker or authority ID)
```

### 1.59 TaskPhase

**Owner:** BS §80.5.4 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
TaskPhase
- phaseId: string (uuid)
- name: string
- description: string
- order: integer
- status: ("pending" | "active" | "completed" | "failed" | "blocked")
- tasks: TaskNode[]
- entryCriteria: string[]
- exitCriteria: string[]
```

### 1.60 TaskNode

**Owner:** BS §80.5.4 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
TaskNode
- taskId: string (uuid)
- phaseId: string (uuid)
- kind: ("goal" | "requirement" | "worker_task" | "validation" | "checkpoint" | "approval" | "recovery_attempt" | "evidence")
- name: string
- description: string
- role: string (worker role)
- status: ("pending" | "ready" | "running" | "waiting_approval" | "waiting_resource" | "completed" | "failed" | "blocked" | "skipped" | "cancelled")
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
```

### 1.61 WorkerAssignment

**Owner:** BS §80.5.4 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
WorkerAssignment
- assignmentId: string (uuid)
- workerId: string (uuid)
- taskId: string (uuid)
- role: string
- workspaceLease: string (lease ID)
- modelProfile: string (profile ID)
- status: ("assigned" | "active" | "completed" | "failed" | "released")
```

### 1.62 ProviderProfile

**Owner:** BS §80.5.5 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** BS §11.2, TA §24.2

```text
ProviderProfile
- providerProfileId: string (uuid)
- displayName: string
- compatibilityMode: ("openai_compatible" | "anthropic_compatible") (technical architecture §24.2: OPENAI_COMPATIBLE | ANTHROPIC_COMPATIBLE)
- protocol: ("chat_completions" | "responses" | "messages" | "custom")
- baseUrl: string (URL)
- apiKeySecretRef: string (credential ref, NOT the actual key)
- customHeadersSecretRefs: string[] (credential refs for sensitive headers, never header values)
- modelId: string
- visionModelId: string?
- embeddingModelId: string?
- rerankerModelId: string?
- reasoningModelId: string?
- organizationId: string?
- projectId: string?
- capabilities: ("text" | "vision" | "structured_output" | "tool_calling" | "reasoning" | "embeddings")[]
- capabilityOverrides: { capability: string, enabled: boolean }[] (user overrides of discovered capabilities)
- attentionCapabilities: AttentionReliabilityProfile (TA §19.2; BS §53.11; carries declaredContextTokens, the physical context capacity)
- reasoningCapabilityProfile: ReasoningCapabilityProfile
- defaultReasoningEffort: ("normal" | "extended" | "deep" | "exhaustive")
- requestSettings: RequestSettings
- privacyPolicy: string?
- networkPolicy: string?
- enabled: boolean
- status: ("configured" | "reachable" | "authenticated" | "degraded" | "unavailable")
- lastConnectionTest: timestamp?
- createdAt: timestamp
- updatedAt: timestamp
```

### 1.63 ReasoningCapabilityProfile

**Owner:** BS §80.5.5 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** TA §24.2

```text
ReasoningCapabilityProfile
- supportsNativeReasoning: (true | false | "unknown")
- supportedEffortLevels: ("normal" | "extended" | "deep" | "exhaustive")[] (technical architecture §24.2: NORMAL | EXTENDED | DEEP | EXHAUSTIVE[])
- maxReasoningTokens: integer? (provider capability metadata only — never a Nirman execution budget, authorization ceiling, or termination condition; usage against it is telemetry per build spec §72)
- reasoningUsage: ("reported" | "estimated" | "unavailable")
- effortParameterMapping: { effortLevel: string, providerParameters: object }[] (configuration metadata, never authority) (technical architecture §24.2: provider-specific normalized mapping)
- supportsPerRequestEffortChange: boolean
- supportsContinuation: (true | false | "unknown")
```

### 1.64 RequestSettings

**Owner:** BS §80.5.5 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
RequestSettings
- temperature: float (0.0-2.0, default 0.7)
- maxTokens: integer?
- timeoutSeconds: integer (default 120)
- retryPolicy: RetryPolicy?
```

### 1.65 RetryPolicy

**Owner:** BS §80.5.5 · **Contract:** CONTRACT.RUNTIME.AGENT_BUILDABILITY · **Projected at:** —

```text
RetryPolicy
- maxRetries: integer (default 3)
- initialBackoffSeconds: float (default 1.0)
- maxBackoffSeconds: float (default 60.0)
- backoffMultiplier: float (default 2.0)
```

### 1.66 Content

**Owner:** BS §81.1 · **Contract:** CONTRACT.RUNTIME.CONTENT_INTELLIGENCE · **Projected at:** TA §85.1

```text
Content
- contentId
- projectId
- canonicalKey
- contentType
- sourceLocale
- supportedLocales
- currentRevisionId
- createdAt
- updatedAt
```

### 1.67 ContentRevision

**Owner:** BS §81.1 · **Contract:** CONTRACT.RUNTIME.CONTENT_INTELLIGENCE · **Projected at:** TA §85.1

```text
ContentRevision
- contentRevisionId
- contentId
- projectRevisionId
- requirementIds
- contentType
- locale
- key
- previousValue
- proposedValue
- placeholderSchema
- pluralizationModel
- localeFallback
- sourceLocale
- translationStatus
- terminologyReferences
- toneProfile
- brandVoiceProfile
- accessibilityContext
- sourceEvidenceIds
- transactionId
- validationStatus
- invalidatedBy
- contentProvenance
- approvalState
```

### 1.68 ContentDependency

**Owner:** BS §81.1 · **Contract:** CONTRACT.RUNTIME.CONTENT_INTELLIGENCE · **Projected at:** TA §85.1

```text
ContentDependency
- dependencyId
- contentId
- dependencyType
- dependencyIdentity
- dependencyRevision
- invalidationPolicy
```

### 1.69 Conversation

**Owner:** BS §82 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** TA §86.1

```text
Conversation
- conversationId
- projectId
- messages
- attachments
- requirements: List<ConversationRequirementIndex>  // lineage index referencing canonical MemoryStore/ConstraintRegistry records
- decisions: List<ConversationDecisionIndex>        // lineage index referencing canonical ConstraintRegistry/MemoryStore records
- acceptedSuggestions
- rejectedSuggestions
- activeGoal
- projectRevisionId
- expectedProjectRevision
- conversationRevision
- taskLineage
- createdAt
- updatedAt
```

### 1.70 ConversationRequirement

**Owner:** BS §82 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** TA §86.1

```text
ConversationRequirement
- requirementId
- status
- sourceMessageId
- sourceEvidenceIds
- supersedes
- supersededBy
```

### 1.71 ConversationDecision

**Owner:** BS §82 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** TA §86.1

```text
ConversationDecision
- decisionId
- status
- sourceMessageId
- sourceEvidenceIds
- supersedes
- locked
```

### 1.72 ConversationSuggestion

**Owner:** BS §82 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** TA §86.1

```text
ConversationSuggestion
- suggestionId
- status
- proposedBy
- acceptedAt
- rejectedAt
- resultingTaskIds
```

### 1.73 ConversationAttachment

**Owner:** BS §82 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** TA §86.1

```text
ConversationAttachment
- attachmentId
- contentHash
- mimeType
- sizeBytes
- storageOwner
- privacyClassification
- deletionStatus
- projectIsolation
- providerTransmissionPolicy
- revisionBinding
- createdAt
```

### 1.74 ChangeReportRecord

**Owner:** BS §83.1 · **Contract:** CONTRACT.RUNTIME.CHANGE_INTELLIGENCE · **Projected at:** TA §87.1

```text
ChangeReportRecord
- recordId
- transactionId
- projectRevisionAfter
- status: INCOMPLETE | COMPLETE | UNRESOLVED
- report: ChangeImpactReport | null
- failureDiagnostics: string | null
- createdAt
- updatedAt
```

### 1.75 ChangeImpactReport

**Owner:** BS §83.1 · **Contract:** CONTRACT.RUNTIME.CHANGE_INTELLIGENCE · **Projected at:** TA §87.1

```text
ChangeImpactReport
- reportId
- transactionId
- projectionStatus: COMPLETE
- projectRevisionBefore
- projectRevisionAfter
- requirementIds
- causeType: REQUIREMENT | GOAL | DIRECTIVE | REPAIR_CAUSE | APPROVED_ACTION
- causeId
- changed
- why
- files
- runtimeEffects
- testsAffected
- testsRun
- previewAffected
- evidenceInvalidated
- evidenceRetained
- verified
- unresolvedIssues
- recoveryActions
- recommendedNextStep
- recommendationSource
- recommendationBasis
- requiredAuthority
- generatedAt
- projectionVersion
```

### 1.76 ClarificationRecord

**Owner:** BS §69.11 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** —

```text
ClarificationRecord
- clarificationId
- sessionId
- taskId
- category: PRIMARY_GOAL | NAVIGATION_STRUCTURE | DISTINGUISHING_BEHAVIOR | SECURITY_OR_PERSONAL_DATA
- question
- options
- recordedDefault
- defaultBasis: CONSERVATIVE | CONVENTIONAL | DERIVED_FROM_REFERENCE
- dependentRequirementIds
- askedAt
- answerWaitPolicyId
- answeredAt
- answer
- outcome: ANSWERED | PROCEEDED_ON_DEFAULT | USER_REQUIRED | WITHDRAWN
- proceededAt
- replanDirectiveId
- createdAt
```

## 2. Schemas owned by the Technical Architecture

### 2.1 TaskContract

**Owner:** TA §6.1 · **Contract:** — · **Projected at:** —

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
- resourceRequirements
- inputReferences
- dependencyContracts
- expectedOutputSchema
- deadline
```

### 2.2 AndroidDeviceProfile

**Owner:** TA §10.2 · **Contract:** — · **Projected at:** —

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

### 2.3 InteractionExecutor

**Owner:** TA §10.2 · **Contract:** — · **Projected at:** —

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

### 2.4 Device

**Owner:** TA §10.3 · **Contract:** — · **Projected at:** —

```text
Device
- id
- name
- kind: emulator
- platformVersion
- architecture
- connectionState
- availableStorage
- hotReloadState
- logStream
- installState
```

### 2.5 VisualReference

**Owner:** TA §10.4 · **Contract:** — · **Projected at:** —

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

### 2.6 PreviewSurface

**Owner:** TA §10.8 · **Contract:** — · **Projected at:** —

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
- drivenBy: USER | SCENARIO | EXPLORATION | RESTORE
- drivenByScenarioId
- queuedUserInputCount
- status
- createdAt
```

### 2.7 PreviewInteraction

**Owner:** TA §10.8 · **Contract:** — · **Projected at:** —

```text
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

### 2.8 EnvironmentRecord

**Owner:** TA §11.1 · **Contract:** — · **Projected at:** —

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

### 2.9 TerminalSession

**Owner:** TA §11.4 · **Contract:** — · **Projected at:** —

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

### 2.10 GoalContract

**Owner:** TA §16.1 · **Contract:** — · **Projected at:** —

```text
GoalContract
- goalId
- taskId
- statement
- completionConditions
- validationPlan
- scope
- autonomyPolicy
- resourceRequirements
- stopConditions
- progressSummary
- lastEvaluatedAt
- status
```

### 2.11 Schedule

**Owner:** TA §16.4 · **Contract:** — · **Projected at:** —

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
- resourceRequirements
- notificationPolicy
- lastRunId
- nextRunAt
- failureCount
```

### 2.12 FileCheckpoint

**Owner:** TA §18 · **Contract:** — · **Projected at:** —

```text
FileCheckpoint
- checkpointId
- taskId
- filePaths
- contentHashes
- parentRevision
- createdAt
```

### 2.13 TaskCheckpoint

**Owner:** TA §18 · **Contract:** — · **Projected at:** —

```text
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

### 2.14 RecoveryAttempt

**Owner:** TA §18 · **Contract:** — · **Projected at:** —

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

### 2.15 SkillAdmission

**Owner:** TA §19.1 · **Contract:** CONTRACT.RUNTIME.SKILL · **Projected at:** —

```text
SkillAdmission
- admissionId
- skillId
- skillVersion
- sessionId
- taskId
- environmentFingerprint
- environmentCapabilityRecordId
- requiredCapabilities
- capabilityResolution: per required capability, AVAILABLE | REPAIRABLE | USER_REQUIRED | UNAVAILABLE
- decision: ADMITTED | BLOCKED | NOT_FOUND | NOT_INVOCABLE
- decisionReason
- scanStatus
- trustStatus
- policyDecisionId
- decidedAt
```

### 2.16 SkillInvocationRecord

**Owner:** TA §19.1 · **Contract:** CONTRACT.RUNTIME.SKILL · **Projected at:** —

```text
SkillInvocationRecord
- invocationId
- admissionId
- skillId
- skillVersion
- sessionId
- taskId
- workerId
- projectRevision
- environmentFingerprint
- inputRef
- outputRef
- toolCallIds
- evidenceIds
- outcome: COMPLETED | FAILED | CANCELLED | BLOCKED
- invalidatedBy
- startedAt
- completedAt
```

### 2.17 PositionalRecallCell

**Owner:** TA §19.2 · **Contract:** CONTRACT.RUNTIME.CONTEXT · **Projected at:** —

```text
PositionalRecallCell
- fillBucket: float
- positionBucket: HEAD | EARLY | MIDDLE | LATE | TAIL
- literalPassRate: float
- gistPassRate: float
- samples: usize
```

### 2.18 ExternalToolConnection

**Owner:** TA §20 · **Contract:** — · **Projected at:** —

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

### 2.19 EvidenceRecord

**Owner:** TA §23.3 · **Contract:** CONTRACT.RUNTIME.EVIDENCE · **Projected at:** —

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
- sourceEventId
- operationId
- sessionId
- projectRevision
- checkpointId
- artifactId
- previewRevisionId
- deviceIdentity
- toolchainLockId
- environmentIdentityId
- validationPolicyVersion
- freshnessInterval
- dependencyIds
- supersedes
- supersededBy
- invalidationReason
```

### 2.20 ModelRequest

**Owner:** TA §24.4 · **Contract:** — · **Projected at:** —

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

### 2.21 ReasoningSettings

**Owner:** TA §24.4 · **Contract:** — · **Projected at:** —

```text
ReasoningSettings
- effortLevel: NORMAL | EXTENDED | DEEP | EXHAUSTIVE
- maxReasoningTokensOptional
- maxReasoningTimeMsOptional
- providerNativeParameters
- deliberationId
- passNumber
- effortGrantId
```

### 2.22 ModelEvent

**Owner:** TA §24.4 · **Contract:** — · **Projected at:** —

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

### 2.23 ToolCallRequest

**Owner:** TA §24.5 · **Contract:** — · **Projected at:** —

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

### 2.24 SelfDevContract

**Owner:** TA §25.3 · **Contract:** — · **Projected at:** —

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

### 2.25 EpisodeRecord

**Owner:** TA §29.1 · **Contract:** — · **Projected at:** —

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

### 2.26 ImprovementProposal

**Owner:** TA §30.2 · **Contract:** — · **Projected at:** —

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

### 2.27 DeviceStateFingerprint

**Owner:** TA §34.2 · **Contract:** — · **Projected at:** —

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
```

### 2.28 ApplicationStateFingerprint

**Owner:** TA §34.2 · **Contract:** — · **Projected at:** —

```text
ApplicationStateFingerprint
- packageName
- processState
- databaseSnapshot
- preferencesSnapshot
- appPermissions
- accountSessionState
```

### 2.29 EnvironmentStateFingerprint

**Owner:** TA §34.2 · **Contract:** — · **Projected at:** —

```text
EnvironmentStateFingerprint
- toolchainLock
- environmentIdentity
- dependencySnapshot
- providerProfile
- validationPolicyVersion
```

### 2.30 EvidenceDependency

**Owner:** TA §36.4 · **Contract:** — · **Projected at:** —

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
```

### 2.31 ArtifactSet

**Owner:** TA §36.4 · **Contract:** — · **Projected at:** —

```text
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
```

### 2.32 ExternalEffectRecord

**Owner:** TA §36.4 · **Contract:** — · **Projected at:** —

```text
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
```

### 2.33 UsageRecord

**Owner:** TA §36.4 · **Contract:** — · **Projected at:** —

```text
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

### 2.34 LocalTransaction

**Owner:** TA §36.5 · **Contract:** — · **Projected at:** —

```text
LocalTransaction
- stagedSourceRevision
- changedPaths
- checkpointId
- commitState: STAGED | VALIDATED | COMMITTED | ROLLED_BACK
```

### 2.35 DeviceTransaction

**Owner:** TA §36.5 · **Contract:** — · **Projected at:** —

```text
DeviceTransaction
- deviceSessionId
- installedArtifactFingerprint
- appStateFingerprint
- observationState: REQUESTED | INSTALLED | LAUNCHED | OBSERVED | UNKNOWN
- cleanupPolicy
```

### 2.36 ExternalEffectTransaction

**Owner:** TA §36.5 · **Contract:** — · **Projected at:** —

```text
ExternalEffectTransaction
- externalEffectId
- idempotencyKey
- requestState
- reconciliationState
- compensationState
```

### 2.37 SigningOperation

**Owner:** TA §36.5 · **Contract:** — · **Projected at:** —

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
```

### 2.38 CertificateInspection

**Owner:** TA §36.5 · **Contract:** — · **Projected at:** —

```text
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

### 2.39 AndroidLanguageAdapter

**Owner:** TA §47.3 · **Contract:** CONTRACT.RUNTIME.SCOPE · **Projected at:** —

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
  - params: patch: StructuredPatch (BS §43.2: contextId, baseRevision, targetSymbolIds, anchorHashes, premises, operations)
  - returns: valid: bool, violations: list, affectedNodes: list, premiseMismatches: list
  - errors: PatchValidationError, PremiseMismatchError
- format_or_serialize(updated_unit: ParsedUnit) -> SerializedUnit
  - params: updated_unit: ParsedUnit
  - returns: content: str, format: str, encoding: str
  - errors: SerializationError
```

### 2.40 ProviderAdapter

**Owner:** TA §57.8.1 · **Contract:** — · **Projected at:** —

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
- capabilityProfile: the `capabilities`, `capabilityOverrides`, `attentionCapabilities`, and `reasoningCapabilityProfile` fields of the bound `ProviderProfile` (build spec §80.5.5), refreshed by `detectCapability` (no separate `ProviderCapabilityProfile` record)

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
```

### 2.41 AgentExecutionKernel

**Owner:** TA §58.2 · **Contract:** — · **Projected at:** —

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

### 2.42 AgentLoopRecord

**Owner:** TA §58.3 · **Contract:** — · **Projected at:** —

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
```

### 2.43 AgentProposal

**Owner:** TA §58.3 · **Contract:** — · **Projected at:** —

```text
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
```

### 2.44 AgentProfile

**Owner:** TA §58.3 · **Contract:** — · **Projected at:** —

```text
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

### 2.45 SkillExecutionRecord

**Owner:** TA §58.4 · **Contract:** — · **Projected at:** —

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

### 2.46 KnowledgeRelation

**Owner:** TA §58.6 · **Contract:** — · **Projected at:** —

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

### 2.47 KnowledgeArtifact

**Owner:** TA §58.6 · **Contract:** — · **Projected at:** —

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

### 2.48 ToolSession

**Owner:** TA §58.7 · **Contract:** — · **Projected at:** —

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

### 2.49 ValidationPlan

**Owner:** TA §58.9 · **Contract:** — · **Projected at:** —

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

### 2.50 MemoryRecord

**Owner:** TA §59.5 · **Contract:** CONTRACT.RUNTIME.MEMORY · **Projected at:** —

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

### 2.51 ScenarioStep

**Owner:** TA §62.2 · **Contract:** CONTRACT.RUNTIME.E2E · **Projected at:** —

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

### 2.52 FailureSignature

**Owner:** TA §63.4 · **Contract:** CONTRACT.RUNTIME.LOCALIZATION · **Projected at:** —

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

### 2.53 VerificationRun

**Owner:** TA §64.5 · **Contract:** CONTRACT.RUNTIME.VERIFICATION · **Projected at:** —

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

### 2.54 ScenarioDivergence

**Owner:** TA §65.4 · **Contract:** CONTRACT.RUNTIME.DEVICE_MATRIX · **Projected at:** —

```text
ScenarioDivergence
- divergenceId
- scenarioId
- passingEmulatorProfiles
- failingEmulatorProfiles
- differingAttributes: apiLevel | density | formFactor | abi | orientation | permissions | networkProfile
- classification: defect | environment_limitation
- evidenceRefs
```

### 2.55 DirectiveEffect

**Owner:** TA §66.4 · **Contract:** CONTRACT.RUNTIME.DIRECTIVE · **Projected at:** —

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

### 2.56 RuntimeSnapshot

**Owner:** TA §67.2 · **Contract:** CONTRACT.RUNTIME.DEBUGGER · **Projected at:** —

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

### 2.57 TriggerFiring

**Owner:** TA §68.4 · **Contract:** CONTRACT.RUNTIME.TRIGGER · **Projected at:** —

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

### 2.58 ResourceExecutionProfile

**Owner:** TA §69.3 · **Contract:** CONTRACT.RUNTIME.PROFILING · **Projected at:** —

```text
ResourceExecutionProfile
- planRevision
- perOperationProfiles
- expectedCpu
- expectedMemory
- expectedDisk
- expectedEmulatorSlots
- expectedConcurrency
- expectedBuildPressure
- expectedDurationObserved
- confidence: profiled | sparse | unprofiled
- sampleCounts
- capacityVerdict: fits | exceeds_declared_time_bound | exceeds_memory | exceeds_disk |
                   exceeds_emulator_slots | exceeds_concurrency
```

### 2.59 ResolvedDependency

**Owner:** TA §70.2 · **Contract:** CONTRACT.RUNTIME.SUPPLY_CHAIN · **Projected at:** —

```text
ResolvedDependency
- coordinate
- resolvedVersion
- integrityHash
- resolutionSource
- previouslyRecordedHash
- verdict: verified | hash_mismatch | unresolvable | substitution_suspected
```

### 2.60 ArtifactProvenance

**Owner:** TA §70.4 · **Contract:** CONTRACT.RUNTIME.SUPPLY_CHAIN · **Projected at:** —

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

### 2.61 CapabilityDescriptor

**Owner:** TA §71.6 · **Contract:** CONTRACT.RUNTIME.REASONING · **Projected at:** —

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

### 2.62 DeliberationSession

**Owner:** TA §72.3 · **Contract:** CONTRACT.RUNTIME.DELIBERATION · **Projected at:** —

```text
DeliberationSession
- sessionId
- deliberationId
- revision
- activeHypotheses: hypothesisId[]
- rejectedStrategies: { strategy, refutingEvidenceRef }[]
- evidenceAcquired: evidenceRef[]
- effortLevelGranted
- effortGrantId
- pendingEvidenceAcquisitionTrigger: trigger | null
- providerContinuationState
- lastCheckpointEventId
```

### 2.63 PreviewRequest

**Owner:** TA §73.3 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** —

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

### 2.64 AndroidTechnologyAdapter

**Owner:** TA §73.10 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** —

```text
AndroidTechnologyAdapter
- adapterId
- adapterVersion
- technologyIds
- supportedCompositions
- requiredToolchainCapabilities
- requiredDeviceCapabilities
- compatibilityRules
```

### 2.65 AndroidTechnologyAdapterResolution

**Owner:** TA §73.10 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** —

```text
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

### 2.66 PreviewModeResolverInput

**Owner:** TA §73.11 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** —

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
```

### 2.67 PreviewModeResolverOutput

**Owner:** TA §73.11 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** —

```text
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

### 2.68 AndroidDeviceAdapter

**Owner:** TA §73.12 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** —

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

### 2.69 AndroidBuildAdapter

**Owner:** TA §73.13 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** —

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
```

### 2.70 AndroidBuildObservation

**Owner:** TA §73.13 · **Contract:** CONTRACT.RUNTIME.PROMPT_CONTRACT · **Projected at:** —

```text
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

### 2.71 BoundaryOperationProjection

**Owner:** TA §74 · **Contract:** CONTRACT.RUNTIME.INTEGRATION_BOUNDARY · **Projected at:** —

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

### 2.72 AndroidServiceIntegration

**Owner:** TA §74.1 · **Contract:** CONTRACT.RUNTIME.INTEGRATION_BOUNDARY · **Projected at:** —

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
- requiredOperationality: IntegrationState (build spec §5.7.2; the minimum acceptable `IntegrationOperationality.aggregateState`, build spec §5.7.5)
- functionalScenarioRefs
- acceptanceEvidenceRefs
- privacyAndNetworkPolicy
```

### 2.73 UiHierarchyObservation

**Owner:** TA §74.2 · **Contract:** CONTRACT.RUNTIME.INTEGRATION_BOUNDARY · **Projected at:** —

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

### 2.74 ExportVerificationRecord

**Owner:** TA §74.3 · **Contract:** CONTRACT.RUNTIME.INTEGRATION_BOUNDARY · **Projected at:** —

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

### 2.75 DocumentationCertificationReport

**Owner:** TA §74.5 · **Contract:** CONTRACT.RUNTIME.INTEGRATION_BOUNDARY · **Projected at:** —

```text
DocumentationCertificationReport
- reportId
- documentSnapshotHash
- verifierVersion
- registryVersion
- checksExecuted
- graphClassesChecked
- semanticRulesChecked
- checksUnevaluated
- unevaluatedSubjects
- defectCount
- defects
- result: FAIL | DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS | DOCUMENTATION_CERTIFIED
- evidenceId
- generatedAt
```

### 2.76 ContentRevisionDraft

**Owner:** TA §85.1 · **Contract:** CONTRACT.RUNTIME.CONTENT_INTELLIGENCE · **Projected at:** —

```text
ContentRevisionDraft
- contentType
- locale
- key
- proposedValue
- placeholderSchema
- pluralizationModel
- localeFallback
- sourceLocale
- terminologyReferences
- toneProfile
- brandVoiceProfile
- accessibilityContext
```

### 2.77 ContentMutation

**Owner:** TA §85.1 · **Contract:** CONTRACT.RUNTIME.CONTENT_INTELLIGENCE · **Projected at:** —

```text
ContentMutation
- mutationId
- contentId
- baseProjectRevision
- proposedContentRevision: ContentRevisionDraft
- requestedBy
- requirementIds
- transactionId
```

### 2.78 ContentValidationResult

**Owner:** TA §85.1 · **Contract:** CONTRACT.RUNTIME.CONTENT_INTELLIGENCE · **Projected at:** —

```text
ContentValidationResult
- validationId
- contentRevisionId
- status
- checks
- evidenceIds
- projectRevision
- createdAt
```

### 2.79 ContentPropagationPlan

**Owner:** TA §85.1 · **Contract:** CONTRACT.RUNTIME.CONTENT_INTELLIGENCE · **Projected at:** —

```text
ContentPropagationPlan
- propagationId
- contentRevisionId
- affectedSurfaceIds
- affectedLocaleIds
- affectedResourceIds
- affectedTestIds
- affectedPreviewIds
- invalidationIds
```

### 2.80 TerminologyProfile

**Owner:** TA §85.1 · **Contract:** CONTRACT.RUNTIME.CONTENT_INTELLIGENCE · **Projected at:** —

```text
TerminologyProfile
- profileId
- projectId
- terms
- forbiddenTerms
- aliases
- locale
- version
```

### 2.81 ContentEvidence

**Owner:** TA §85.1 · **Contract:** CONTRACT.RUNTIME.CONTENT_INTELLIGENCE · **Projected at:** —

```text
ContentEvidence
- evidenceId
- contentRevisionId
- observationId
- validationId
- projectRevision
- freshness
- invalidationState
```

### 2.82 ConversationMessage

**Owner:** TA §86.1 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** —

```text
ConversationMessage
- messageId
- conversationId
- sequence
- role
- contentReference
- attachmentIds
- sourceEventId
- createdAt
```

### 2.83 ConversationTaskLink

**Owner:** TA §86.1 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** —

```text
ConversationTaskLink
- linkId
- conversationId
- taskId
- relationship
- projectRevision
- createdAt
```

### 2.84 ConversationRequirementIndex

**Owner:** TA §86.1 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** —

```text
ConversationRequirementIndex
- requirementId
- sourceMessageId
- canonicalRequirementId
- status
```

### 2.85 ConversationDecisionIndex

**Owner:** TA §86.1 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** —

```text
ConversationDecisionIndex
- decisionId
- sourceMessageId
- canonicalDecisionId
- locked
```

### 2.86 ConversationRebaseRecord

**Owner:** TA §86.1 · **Contract:** CONTRACT.RUNTIME.CONVERSATION_CONTEXT · **Projected at:** —

```text
ConversationRebaseRecord
- recordId
- conversationId
- fromProjectRevision
- toProjectRevision
- reason
- affectedTaskIds
- conflictingRequirementIds
- resolution
- createdAt
```

### 2.87 ToolchainProvisioningManifest

**Owner:** TA §49.4 · **Contract:** — · **Projected at:** —

```text
ToolchainProvisioningManifest
- manifestVersion
- nirmanReleaseVersion
- signature
- toolchainRoot
- baselineApiLevel
- baselineAbi: x86_64
- baselineImageVariant: google_apis
- components
  - componentId
  - kind: JDK | CMDLINE_TOOLS | PLATFORM_TOOLS | BUILD_TOOLS | PLATFORM | EMULATOR | SYSTEM_IMAGE | HYPERVISOR_DRIVER
  - sourceKind: SDK_REPOSITORY | VENDOR_URL
  - sourceRef
  - version
  - sha256
  - byteSize
  - licenseId
  - licenseHash
  - installPath
  - onDemand: bool
- deviceProfiles
  - profileId
  - hardwareProfile
  - resolution
  - density
  - ramMb
  - systemImageComponentId
- totalDownloadBytes
- requiredFreeBytes
- createdAt
```

### 2.88 ToolchainProvisioningRecord

**Owner:** TA §49.4 · **Contract:** — · **Projected at:** —

```text
ToolchainProvisioningRecord
- provisioningRunId
- manifestVersion
- environmentId
- windowsAccountSid
- hostArchitecture: X64 | ARM64
- networkPath: DIRECT | SYSTEM_PROXY | PAC
- proxyHost
- state: NOT_PROVISIONED | CONSENT_REQUIRED | WAITING_NETWORK | DOWNLOADING | VERIFYING | INSTALLING | HYPERVISOR_REQUIRED | AVD_CREATING | FIRST_BOOT | SNAPSHOT_SAVED | READY | PROVISIONED_UNVERIFIED | FAILED_INTEGRITY | FAILED_DISK | USER_REQUIRED | UNAVAILABLE
- capabilityClassification: AVAILABLE | REPAIRABLE | USER_REQUIRED | UNAVAILABLE
- componentResults
  - componentId
  - source
  - expectedSha256
  - observedSha256
  - installedPath
  - result: INSTALLED | FAILED_INTEGRITY | SKIPPED_PRESENT | HOST_UNSUPPORTED | FAILED
- licenseAcceptance
  - licenseHash
  - manifestVersion
  - acceptedAt
  - acceptedByAccountSid
- consent
  - downloadBytesShown
  - requiredFreeBytesShown
  - freeBytesObserved
  - acceptedAt
- hypervisorAction: NONE | WHPX_ENABLED | AEHD_INSTALLED | FIRMWARE_BLOCKED
- elevationPerformed: bool
- restartRequired: bool
- detectedNotUsed
  - kind
  - path
  - version
- avdId
- snapshotId
- readinessEvidenceId
- environmentFingerprintAfter
- userRequiredDecisionIds
- startedAt
- completedAt
```

### 2.89 RenderTransport

**Owner:** TA §10.7 · **Contract:** — · **Projected at:** —

```text
RenderTransport
- renderTransportId
- renderTransportVersion
- previewSurfaceId
- deviceId
- deviceSessionId
- runtimeSessionId
- transportKind: SHARED_MEMORY_RING | WEBRTC_LOOPBACK
- controlEndpoint
- pixelFormat: RGBA8888
- width
- height
- maxFrameRate
- ringDepth
- backpressurePolicy: DROP_OLDEST
- staleAfterMs
- gpuMode: HOST_GPU | SWIFTSHADER
- presentationSurface: SWAPCHAIN_PANEL | WRITEABLE_BITMAP
- state: OPENING | STREAMING | IDLE | LOST | CLOSED
- lastFrameSequence
- lastFrameAt
- frameNotice
  - previewSurfaceId
  - ringSlot
  - frameStamp
- frameStamp
  - frameSequence
  - capturedAt
  - width
  - height
  - pixelFormat
  - deviceId
  - previewRevisionId
  - artifactFingerprint
  - deviceStateFingerprint
  - interactionId
- createdAt
- closedAt
```

### 2.90 WorkerConnection

**Owner:** TA §57.11 · **Contract:** — · **Projected at:** —

```text
WorkerConnection
- workerConnectionId
- protocolVersion
- workerId
- workerLeaseId
- attemptId
- taskId
- nodeId
- workerRole
- executionProfile: TRUSTED_LOCAL | RESTRICTED_PROCESS | HIGH_RISK_RESTRICTED_PROCESS | DISPOSABLE_ISOLATED | REVIEW_ONLY
- modelProfileId
- pipeName
- launchTokenDigest
- workerProcessId
- jobObjectName
- containerSid
- memoryLimitBytes
- heartbeatIntervalMs
- staleThresholdMs
- state: LAUNCHING | HANDSHAKING | CONNECTED | PAUSED | CANCELLING | CLOSED
- lastHeartbeatSequence
- lastHeartbeatAt
- workerMessageKinds: HELLO | HEARTBEAT | MODEL_CALL | PROPOSAL | CAPABILITY_QUERY | REASONING_ARTIFACT | DELIBERATION_RECORD | CANCEL_ACK | EXIT
- supervisorMessageKinds: WELCOME | CYCLE_INPUT | MODEL_EVENT | PROPOSAL_RESULT | CAPABILITY_ANSWER | DECISION | PAUSE | RESUME | CANCEL | CLOSE
- exitKind: COMPLETED | FAILED | TIMED_OUT | CANCELLED | CRASHED | STALE_TERMINATED
- exitCode
- openedAt
- closedAt
```

### 2.91 ScreenModel

**Owner:** TA §74.2 · **Contract:** CONTRACT.RUNTIME.E2E · **Projected at:** —

```text
ScreenModel
- screenModelId
- observationId
- deviceSessionId
- previewRevisionId
- applicationStateFingerprint
- packageId
- activityName
- windowKind: APP | SYSTEM_DIALOG | KEYGUARD | LAUNCHER | INPUT_METHOD
- elements: ordered list of ScreenElement
  - elementIndex
  - resourceId
  - className
  - text
  - contentDescription
  - bounds
  - clickable: true | false
  - longClickable: true | false
  - scrollable: true | false
  - editable: true | false
  - checkable: true | false
  - checked: true | false
  - enabled: true | false
  - focused: true | false
  - depth
  - parentIndex
- actionableElementCount
- screenFingerprint
- redactionPolicyId
- capturedAt
```

### 2.92 ScreenGraph

**Owner:** TA §62.1 · **Contract:** CONTRACT.RUNTIME.E2E · **Projected at:** —

```text
ScreenGraph
- screenGraphId
- taskId
- projectRevisionId
- artifactFingerprint
- deviceSessionId
- goldenSnapshotId
- explorationPolicyId
- maxDepth
- maxActionsPerScreen
- nodes: list of ScreenGraphNode
  - screenFingerprint
  - screenModelId
  - activityName
  - firstReachedAt
  - unexploredActionCount
- edges: list of ScreenGraphEdge
  - fromScreenFingerprint
  - action
  - targetIdentity
  - toScreenFingerprint
  - observedResult: NAVIGATED | UNCHANGED | DIALOG | CRASHED | ANR | EXTERNAL_INTENT
  - interactionId
- coveredRequirementIds
- uncoveredRequirementIds
- status: EXPLORING | COMPLETE | BOUNDED | ABORTED
- createdAt
- completedAt
```

### 2.93 DeviceHygienePolicy

**Owner:** TA §10.3 · **Contract:** CONTRACT.RUNTIME.E2E · **Projected at:** —

```text
DeviceHygienePolicy
- deviceHygienePolicyId
- policyVersion
- deviceSessionId
- animationScale: 0
- keyguardDisabled: true
- stayAwake: true
- setupWizardSkipped: true
- locale
- timezone
- fontScale
- densityOverride
- autoRotateDisabled: true
- systemDialogHandling: list of SystemDialogRule
  - dialogKind: RUNTIME_PERMISSION | APP_CRASH | ANR | KEYGUARD | SETUP_WIZARD | SYSTEM_UPDATE | EXTERNAL_INTENT_CHOOSER
  - handling: ANSWER_PER_SCENARIO | CAPTURE_AND_DISMISS | DISMISS | CANCEL
  - evidenceRequired: true | false
- appliedAt
- verificationFingerprint
- status: PENDING | APPLIED | VERIFIED | FAILED
```

### 2.94 GoldenSnapshot

**Owner:** TA §10.3 · **Contract:** CONTRACT.RUNTIME.E2E · **Projected at:** —

```text
GoldenSnapshot
- goldenSnapshotId
- deviceSessionId
- deviceProfileId
- systemImageDigest
- deviceHygienePolicyId
- snapshotRef
- deviceStateFingerprint
- takenAt
- restoreCount
- lastRestoredAt
- lastRestoreDurationMs
- status: TAKING | READY | STALE | INVALID
```

### 2.95 ContractDouble

**Owner:** TA §74.1 · **Contract:** CONTRACT.RUNTIME.INTEGRATION_BOUNDARY · **Projected at:** —

```text
ContractDouble
- contractDoubleId
- sessionId
- taskId
- integrationId
- requestSchemaRef
- responseSchemaRef
- errorSchemaRef
- listenAddress: loopback only
- listenPort
- guestEndpoint
- fixtureSetRef
- recordedExchangeCount
- evidenceLabel: DOUBLE_BACKED
- status: STARTING | SERVING | STOPPED | FAILED
- startedAt
- stoppedAt
```

### 2.96 RepairPattern

**Owner:** TA §51.1 · **Contract:** CONTRACT.RUNTIME.VERIFICATION · **Projected at:** —

```text
RepairPattern
- repairPatternId
- patternVersion
- failureFamily
- fingerprintMatcher
- classifier
- severity
- likelyCause
- allowedScope
- preconditions
- operationType
- repairSteps
- recoveryAttemptPolicy
- checkpointRule
- validationCommand
- evidenceRequirements
- trust: BUILT_IN | PROMOTED | CANDIDATE
- promotionEvidenceRefs
- sourceImprovementProposalId
- successCount
- failureCount
- createdAt
- updatedAt
```

## 3. Canonical schema registry

### 3.1 CanonicalSchemaRegistry

**Owner:** TA §36.1 · **Contract:** — · **Projected at:** —

The registered schema identities (ADR-189). Registry metadata — owner, version, lifecycle, persistence, authority, acceptance fixtures — stays in technical architecture §36.1.

```text
CanonicalSchemaRegistry
AutonomousAndroidSession
AndroidConstructionContract
VisualSpecification
AndroidTechnologyPlan
AndroidCapabilityProfile
TaskGraph
WorkerContract
TerminalSession
PreviewRevision
Checkpoint
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
ResourceIntegrityRecord
AgentTrustAssessment
ContextCachePolicy
ContextPackage
AttentionReliabilityProfile
StructuredPatch
AndroidRuntimeIntegrityObservation
ContinuityDimensions
BackgroundContinuityRecord
ExportVerificationRecord
PackagingProfile
SkillPackage
SkillInvocationRecord
SkillAdmission
EnvironmentCapabilityRecord
PlatformCapabilityEntry
ValidationEnvironment
BuildGateRecord
Content
ContentRevision
ContentRevisionDraft
ContentMutation
ContentValidationResult
ContentPropagationPlan
TerminologyProfile
ContentEvidence
ContentDependency
Conversation
ConversationMessage
ConversationAttachment
ConversationRequirement
ConversationDecision
ConversationSuggestion
ConversationTaskLink
ConversationRequirementIndex
ConversationDecisionIndex
ConversationRebaseRecord
ChangeReportRecord
ChangeImpactReport
ScreenModel
ScreenGraph
DeviceHygienePolicy
GoldenSnapshot
ClarificationRecord
ContractDouble
RepairPattern
```

## References

[1]: nirman-build-spec.md
[2]: nirman-technical-architecture.md
[3]: nirman-adrs.md
