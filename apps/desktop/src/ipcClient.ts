import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const PROTOCOL_SCHEMA_VERSION = 1;

export type PreviewTruth = "Predicted" | "Simulated" | "Requested" | "Observed" | "Verified" | "Stale" | "Invalidated";
export type ProductLifecycleState = "Created" | "Planning" | "Synthesizing" | "Implementing" | "Paused" | "Previewing" | "Validating" | "Recovering" | "Packaging" | "Completed" | "Blocked" | "UserRequired" | "SafelyFailed" | "Cancelled";
export type BackgroundContinuityState = "ActiveBackground" | "UiDisconnected" | "HostSuspended" | "HostOffline" | "DeviceUnavailable" | "ProviderUnavailable" | "Recovering" | "Reconciling" | "UserRequired" | "SafelyFailed" | "Completed";
export type CommandKind = "ProjectOpen" | "TaskStart" | "TaskCancel" | "TaskResume" | "WorkspaceApplyPatch" | "PreviewStart" | "PreviewStop" | "PreviewPromote" | "ValidationRun" | "ArtifactBuild" | "ArtifactExport" | "ProviderTest" | "ProviderExecute" | "SettingsUpdateProvider" | "AndroidConstructionCreate" | "AndroidToolchainPreflight" | "AndroidRequirementEvaluate" | "AndroidSynthesisBuild" | "AndroidProjectScaffold" | "AgentLoopRun" | "SubmitInstruction" | "Reconnect" | "PauseTask" | "ResumeTask" | "CancelTask" | "WorkerTaskClaim" | "WorkerHandoffSubmit" | "WorkerHandoffAcknowledge" | "WorkerReconcile" | "WorkerStep";
export type ResponseStatus = "Accepted" | "Completed" | "Rejected" | "Duplicate" | "Stale" | "Cancelled" | "Failed";
export type SubscriptionStatus = "Requested" | "Active" | "Paused" | "Gap" | "Closed";

export type ProjectId = [string];
export interface AuthContext { installation_id: string; user_scope: string; project_scope: string; schema_version: number }
export interface WorkerProjectionSummary {
  task_count: number;
  claim_count: number;
  handoff_count: number;
  acknowledged_handoff_count: number;
  roles: string[];
  open_task_ids: string[];
}
export interface ArtifactProjectionSummary {
  task_id: string;
  source_revision: number;
  build_variant: string;
  build_success: boolean;
  timed_out: boolean;
  cancelled: boolean;
  artifact_path: string | null;
  artifact_sha256: string | null;
  project_fingerprint: string;
}
export interface EvidenceProjectionSummary {
  m108_event_count: number;
  m108_evidence_count: number;
  device_observation_count: number;
  latest_observation_id: string | null;
  latest_device_identity: string | null;
}
export interface DeliveryProjectionSummary {
  delivery_id: string;
  task_id: string;
  source_revision: number;
  state: string;
  delivery_kind: string;
  destination_kind: string;
  destination_path: string;
  artifact_fingerprint: string | null;
  post_copy_verified: boolean;
  copy_uncertain: boolean;
  reconciliation_reference: string | null;
  failure_evidence_id: string | null;
  deployment_delivery: string | null;
  checkpoint_id: string | null;
}
export interface ProjectionSnapshot {
  project_id: ProjectId;
  projection_revision: [number];
  task_state: ProductLifecycleState;
  continuity_state: BackgroundContinuityState;
  preview_truth: PreviewTruth;
  current_source_revision: [number];
  last_event_sequence: number;
  last_known_good_ref: string | null;
  worker_projection?: WorkerProjectionSummary | null;
  artifact_projection?: ArtifactProjectionSummary | null;
  evidence_projection?: EvidenceProjectionSummary | null;
  delivery_projection?: DeliveryProjectionSummary | null;
}
export interface CommandEnvelope {
  command_id: string;
  project_id: ProjectId;
  task_id: ProjectId | null;
  kind: CommandKind;
  payload: string;
  expected_projection_revision: [number];
  idempotency_key: string | null;
}
export interface CommandRequest { protocol_schema_version: number; auth: AuthContext; command: CommandEnvelope; correlation_id: string; causation_id: string | null; deadline_epoch_seconds?: number | null }
export interface EventRange { first_sequence: number; last_sequence: number }
export type ErrorCategory = "Authentication" | "Authorization" | "Scope" | "Validation" | "StaleProjection" | "Idempotency" | "NotFound" | "Conflict" | "Environment" | "Provider" | "Device" | "Timeout" | "Cancellation" | "ReplayGap" | "Unavailable" | "Internal";
export interface ErrorEnvelope {
  error_id: string;
  command_id: string | null;
  correlation_id: string;
  causation_id: string | null;
  code: string;
  category: ErrorCategory;
  safe_message: string;
  retryable: boolean;
  retry_after_seconds: number | null;
  recovery_action: string | null;
  diagnostic_ref: string | null;
  authority_decision_ref: string;
  sensitive_data_omitted: boolean;
  created_at_epoch_seconds: number;
}

export interface ProviderProfile {
  provider_id: string;
  display_name: string;
  protocol: "chat_completions" | "responses" | "messages" | "custom";
  base_url: string;
  api_key_secret_ref: string;
  model_id: string;
  timeout_ms: number;
  enabled: boolean;
}
export interface ProviderTestCommandPayload { provider_id: string; prompt: string; max_output_tokens: number | null }
export interface ProviderExecuteCommandPayload { provider_id: string; worker_id: string; prompt: string; max_output_tokens: number | null; max_context_tokens: number; privacy_classification: string; tool_policy: string; stream: boolean }
export interface ProviderExecuteResultPayload { execution_id: string; request_id: string; correlation_id: string; provider_id: string; model_id: string; environment_lock_hash: string; environment_snapshot_id: string; state: string; outcome: string; text: string | null; error_kind: string | null; events: unknown[] }
export interface SettingsUpdateProviderCommandPayload { profile: ProviderProfile }
export interface ProviderTestResultPayload { provider_id: string; model_id: string; request_id: string; correlation_id: string; provider_request_id: string | null; text: string; input_tokens: number | null; output_tokens: number | null; total_tokens: number | null }
export type RequirementOrigin = "user_fact" | "model_proposal";
export interface VisualReferenceInput { referenceId: string; sourcePath: string; imageHash: string }
export interface AssetReferenceInput { assetId: string; sourcePath: string; contentHash: string }
export interface ConstructionRequirement { requirementId: string; statement: string; origin: RequirementOrigin; sourceReferenceIds: string[] }
export interface AndroidTechnologyPlan { planId: string; taskId: ProjectId; requestedCapabilities: string[]; visualRequirements: string[]; selectedLanguages: string[]; selectedUiFrameworks: string[]; selectedRuntimeLayers: string[]; selectedNativeModules: string[]; selectedBuildPlugins: string[]; selectedDeviceApis: string[]; selectedLibraries: string[]; compatibilityConstraints: string[]; rejectedAlternatives: string[]; requiredToolchains: string[]; validationPlan: string[]; confidence: string | null; revision: [number] }
export interface AndroidDeviceProfile { deviceId: string; name: string; platformVersion: string; apiLevel: number; architecture: string; width: number; height: number; density: number; orientation: string; locale: string; permissions: string[]; networkProfile: string }
export interface ValidationModel { requiredChecks: string[]; acceptanceCriteria: string[] }
export interface ArtifactModel { requiredArtifact: "apk" | "aab"; aabDeclared: boolean }
export interface AndroidConstructionContract { schemaVersion: number; contractId: string; projectId: ProjectId; targetPlatforms: ["android"]; taskId: ProjectId; userIntent: string; screenshots: VisualReferenceInput[]; assets: AssetReferenceInput[]; features: ConstructionRequirement[]; ui: ConstructionRequirement[]; data: ConstructionRequirement[]; integrations: ConstructionRequirement[]; technologyPlan: AndroidTechnologyPlan; androidRequirements: ConstructionRequirement[]; deviceMatrix: AndroidDeviceProfile[]; validationModel: ValidationModel; artifactModel: ArtifactModel }
export interface AndroidConstructionCommandPayload { contract: AndroidConstructionContract }
export interface AndroidToolchainPreflightCommandPayload { buildVariant: string }
export interface AndroidToolchainPreflightResultPayload { preflightId: string; status: "AVAILABLE" | "REPAIRABLE" | "USER_REQUIRED" | "UNAVAILABLE"; lockHash: string | null; environmentSnapshotId: string; capabilityCount: number }

export interface AgentLoopRunCommandPayload {
  contract: AndroidConstructionContract;
  source_revision: number;
  workspace_root: string;
  build_variant: string;
  gradle_task: string;
  iteration_budget: number;
  build_timeout_ms: number;
}
export type AgentLoopState = "Running" | "Suspended" | "Complete" | "Failed" | "Exhausted" | "Cancelled";
export type AgentLoopPhase = "Observe" | "Understand" | "Plan" | "SelectAction" | "Authorize" | "Execute" | "ObserveResult" | "UpdateState" | "EvaluateProgress";
export type ProgressStatus = "NotStarted" | "OnTrack" | "Recovering" | "Replanning" | "Complete" | "Failed" | "Exhausted" | "Cancelled";
export interface AgentLoopRecord {
  schema_version: number;
  loop_id: string;
  session_id: string;
  task_id: string;
  agent_instance_id: string;
  state: AgentLoopState;
  state_version: number;
  goal_revision: number;
  plan_revision: number;
  project_revision: number;
  last_observation_id: string | null;
  last_proposal_id: string | null;
  progress_status: ProgressStatus;
  retry_strategy: string;
  cancellation_scope: string;
  created_at_epoch_seconds: number;
  updated_at_epoch_seconds: number;
  phase: AgentLoopPhase;
  iteration: number;
  iteration_budget: number;
  consecutive_failures: number;
  max_consecutive_failures: number;
  last_failed_action: string | null;
  last_failed_action_fingerprint: string | null;
  variation_attempts: number;
  pending_variation: string | null;
  completed_action_count: number;
}
export interface AndroidBuildObservation {
  schema_version: number;
  execution_id: string;
  command_id: string;
  project_id: string;
  task_id: string;
  source_revision: number;
  project_fingerprint: string;
  workspace_root: string;
  build_variant: string;
  gradle_task: string;
  executable: string;
  exit_code: number | null;
  success: boolean;
  timed_out: boolean;
  cancelled: boolean;
  stdout_sha256: string;
  stderr_sha256: string;
  stdout_bytes: number;
  stderr_bytes: number;
  artifact_path: string | null;
  artifact_sha256: string | null;
  started_at_epoch_seconds: number;
  completed_at_epoch_seconds: number;
}
export interface ScaffoldSummary {
  scaffold_id: string;
  contract_id: string;
  project_id: string;
  task_id: string;
  package_name: string;
  application_name: string;
  language: string;
  ui_framework: string;
  min_sdk: number;
  target_sdk: number;
  compile_sdk: number;
  version_code: number;
  version_name: string;
  permissions: string[];
  file_count: number;
  scaffold_fingerprint: string;
}
export interface AgentLoopRunResultPayload {
  loop_record: AgentLoopRecord;
  outcome: "COMPLETE" | "FAILED" | "EXHAUSTED" | "CANCELLED" | "INTERRUPTED";
  build_observation: AndroidBuildObservation | null;
  scaffold: ScaffoldSummary | null;
  resulting_project_fingerprint: string | null;
  toolchain_lock_hash: string | null;
  environment_snapshot_id: string | null;
}
export interface ArtifactBuildCommandPayload {
  source_revision: number;
  workspace_root: string;
  project_fingerprint: string;
  build_variant: string;
  gradle_task: string;
}
export interface ArtifactBuildResultPayload { observation: AndroidBuildObservation }
export interface ArtifactExportCommandPayload {
  source_revision: number;
  destination_path: string;
  packaging_profile_id: string;
  artifact_kind: string;
  request_fingerprint: string;
  idempotency_key: string;
  deployment_delivery: string;
  destination_kind: string;
}
export interface ApkArtifact {
  schema_version: number;
  artifact_id: string;
  project_id: string;
  task_id: string;
  project_revision_id: string;
  source_fingerprint: string;
  source_provenance_ref: string;
  path: string;
  sha256: string;
  package_name: string;
  inspection: unknown | null;
  build_variant: string;
  secret_scan_status: string;
  signing_status: string;
  delivery_status: string;
  delivery_sha256: string | null;
  delivery_verified: boolean;
  copy_uncertain: boolean;
}
export interface ApkDeliveryRecord {
  schemaVersion: number;
  deliveryId: string;
  artifactId: string;
  projectId: string;
  taskId: string;
  sourceRevision: number;
  packagingProfileId: string;
  artifactKind: string;
  destinationPath: string;
  destinationKind: string;
  requestFingerprint: string;
  idempotencyKey: string;
  sha256: string;
  byteCount: number;
  state: "PENDING" | "COPYING" | "COPIED" | "RECONCILING" | "VERIFIED" | "FAILED" | "BLOCKED" | "UNKNOWN";
  sourcePath: string | null;
  sourceSha256: string | null;
  postCopyVerified: boolean;
  reconciliationReference: string | null;
  failureEvidenceId: string | null;
  deploymentDelivery: string | null;
  checkpointId: string | null;
  createdAtEpochSeconds: number;
  completedAtEpochSeconds: number | null;
  errorMessage: string | null;
}
export interface ArtifactExportResultPayload {
  artifact: ApkArtifact;
  delivery_record: ApkDeliveryRecord;
  signing_config: unknown | null;
}
export interface WorkspaceDescriptor { workspace_root: string | null }

export interface PreviewStartCommandPayload { request: PreviewRequest }
export interface PreviewRequest {
  schema_version: number;
  request_id: string;
  project_id: string;
  task_id: string;
  project_revision_id: string;
  checkpoint_id: string;
  source_fingerprint: string;
  contract_version: string;
  technology_plan_version: string;
  asset_manifest_version: string;
  build_variant: string;
  device_id: string | null;
  android_api_level: number | null;
  requested_mode: string | null;
  selected_language: string;
  selected_ui_framework: string;
  changed_paths: string[];
  required_evidence_kinds: string[];
  policy_decision_id: string;
  workspace_root: string | null;
  build_identity: string | null;
}
export interface PreviewFallbackSelection {
  schema_version: number;
  request_id: string;
  mode: string;
  reason: string;
  selection_rank: number;
  runtime_observation_required: boolean;
  evidence_kinds: string[];
}
export interface PreviewRevision {
  schema_version: number;
  preview_revision_id: string;
  project_id: string;
  task_id: string;
  project_revision_id: string;
  checkpoint_id: string;
  source_fingerprint: string;
  artifact_id: string | null;
  artifact_fingerprint: string | null;
  device_id: string | null;
  android_api_level: number | null;
  build_variant: string;
  preview_mode: string;
  technology_plan_version: string;
  asset_manifest_version: string;
  lifecycle_state: string;
  execution_truth: string;
  status: string;
  build_status: string;
  install_status: string;
  launch_status: string;
  runtime_status: string;
  validation_status: string;
  evidence_ids: string[];
  created_at_epoch_seconds: number;
  observed_at_epoch_seconds: number | null;
  invalidated_at_epoch_seconds: number | null;
  invalidated_reason: string | null;
}
export interface AndroidDeviceObservation {
  schema_version: number;
  observation_id: string;
  project_id: string;
  task_id: string;
  project_revision_id: string;
  device_profile_id: string;
  device_identity: string;
  runtime_session_id: string;
  package_name: string;
  apk_sha256: string;
  install_status: string;
  launch_status: string;
  interaction_status: string;
  logcat_reference: string | null;
  screenshot_references: string[];
  accessibility_reference: string | null;
  visual_comparison_reference: string | null;
  permission_result_reference: string | null;
  crash_trace_reference: string | null;
  observed_at_epoch_seconds: number;
  synthetic_data_only: boolean;
}
export interface PreviewStartResultPayload {
  selection: PreviewFallbackSelection;
  revision: PreviewRevision;
  device_observation: AndroidDeviceObservation | null;
}
export interface PreviewEvidenceReadResult {
  kind: "image" | "text";
  mime: string;
  data_base64: string | null;
  text: string | null;
  byte_count: number;
}

export interface CommandResponse {
  response_id: string;
  command_id: string;
  correlation_id: string;
  causation_id: string | null;
  project_id: ProjectId;
  task_id: ProjectId | null;
  status: ResponseStatus;
  result_schema_ref: string | null;
  projection_snapshot_ref: string | null;
  projection_revision: [number];
  snapshot: ProjectionSnapshot;
  event_range: EventRange | null;
  result_payload: unknown | null;
  authority_decision_ref: string;
  created_at_epoch_seconds: number;
}
export interface SessionHandshake { auth: AuthContext; correlation_id: string; schema_version: number; expires_at_epoch_seconds: number }
export interface ControlEvent { event_id: string; sequence: number; project_id: ProjectId; task_id: ProjectId | null; kind: string; payload: string; source_revision: [number] }
export interface EventSubscription {
  subscription_id: string;
  connection_id: string;
  auth: AuthContext;
  project_id: string;
  task_id: string | null;
  from_event_sequence: number;
  snapshot_revision: [number] | null;
  requested_projection_kinds: string[];
  acknowledged_event_sequence: number;
  heartbeat_interval_seconds: number;
  max_batch_size: number;
  backpressure_policy: "PauseOnLimit" | "RejectOverLimit";
  status: SubscriptionStatus;
  correlation_id: string;
}
export interface EventBatch {
  subscription_id: string;
  projection_revision: [number];
  from_event_sequence: number;
  next_event_sequence: number;
  events: ControlEvent[];
  has_gap: boolean;
  status: SubscriptionStatus;
}
export interface SubscriptionBootstrap { subscription: EventSubscription; snapshot: ProjectionSnapshot; batch: EventBatch }
export interface SubscriptionAcknowledgement { auth: AuthContext; subscription_id: string; acknowledged_event_sequence: number; correlation_id: string }
export interface SubscriptionControl { auth: AuthContext; subscription_id: string; correlation_id: string }

export type ProjectionEventResult =
  | { accepted: true; events: ControlEvent[] }
  | { accepted: false; reason: "snapshot-required" | "gap" | "stale" | "duplicate" | "conflict" | "identity" | "range" | "metadata" | "lifecycle" };

export class ProjectionStore {
  private current: ProjectionSnapshot | null = null;
  private subscriptionId: string | null = null;
  private subscriptionStatus: SubscriptionStatus | null = null;
  private acknowledgedSequence = 0;
  private events = new Map<number, ControlEvent>();
  private eventFingerprints = new Map<number, string>();
  private eventIds = new Map<string, number>();
  private taskId: ProjectId | null = null;

  snapshot(): ProjectionSnapshot | null { return this.current; }
  acknowledgedCursor(): number { return this.acknowledgedSequence; }
  subscriptionState(): SubscriptionStatus | null { return this.subscriptionStatus; }

  acceptBootstrap(bootstrap: SubscriptionBootstrap): boolean {
    const { subscription, snapshot, batch } = bootstrap;
    if (subscription.status !== "Active") return false;
    if (subscription.project_id !== snapshot.project_id[0] || subscription.auth.project_scope !== snapshot.project_id[0]) return false;
    if (subscription.auth.schema_version !== PROTOCOL_SCHEMA_VERSION) return false;
    if (subscription.snapshot_revision && subscription.snapshot_revision[0] !== snapshot.projection_revision[0]) return false;
    if (subscription.acknowledged_event_sequence > snapshot.last_event_sequence) return false;
    if (subscription.max_batch_size < 1 || subscription.max_batch_size > 256) return false;
    if (batch.subscription_id !== subscription.subscription_id || batch.status !== "Active") return false;
    if (!this.acceptSnapshot(snapshot)) return false;
    this.subscriptionId = subscription.subscription_id;
    this.subscriptionStatus = "Active";
    this.acknowledgedSequence = subscription.acknowledged_event_sequence;
    const result = this.acceptEventBatch(batch);
    return result.accepted || (batch.events.length === 0 && batch.from_event_sequence === batch.next_event_sequence && batch.next_event_sequence === snapshot.last_event_sequence);
  }

  acceptSnapshot(next: ProjectionSnapshot): boolean {
    if (this.current) {
      const revisionRewound = next.projection_revision[0] < this.current.projection_revision[0];
      const cursorRewoundOrDuplicated = next.projection_revision[0] === this.current.projection_revision[0]
        && next.last_event_sequence <= this.current.last_event_sequence;
      const sourceRewound = next.current_source_revision[0] < this.current.current_source_revision[0];
      if (revisionRewound || cursorRewoundOrDuplicated || sourceRewound || next.project_id[0] !== this.current.project_id[0]) return false;
    }
    this.current = next;
    this.events.clear();
    if (!this.taskId) this.taskId = null;
    return true;
  }

  acceptAuthoritativeSnapshot(next: ProjectionSnapshot): boolean {
    if (!this.current) {
      this.current = next;
      return true;
    }
    if (next.project_id[0] !== this.current.project_id[0]) return false;
    const revision = next.projection_revision[0];
    const currentRevision = this.current.projection_revision[0];
    if (revision < currentRevision || next.last_event_sequence < this.current.last_event_sequence || next.current_source_revision[0] < this.current.current_source_revision[0]) return false;
    if (revision === currentRevision && next.last_event_sequence === this.current.last_event_sequence) {
      const consistent = next.task_state === this.current.task_state
        && next.continuity_state === this.current.continuity_state
        && next.preview_truth === this.current.preview_truth
        && next.current_source_revision[0] === this.current.current_source_revision[0]
        && (this.current.last_known_good_ref === null || next.last_known_good_ref === this.current.last_known_good_ref);
      if (!consistent) return false;
      if (this.current.last_known_good_ref === null && next.last_known_good_ref !== null) this.current = { ...this.current, last_known_good_ref: next.last_known_good_ref };
      return true;
    }
    this.current = next;
    return true;
  }

  transitionSubscription(next: SubscriptionStatus): boolean {
    const current = this.subscriptionStatus;
    if (!current) return next === "Active";
    if (current === next) return true;
    const allowed: Record<SubscriptionStatus, SubscriptionStatus[]> = {
      Requested: ["Active", "Closed"],
      Active: ["Paused", "Gap", "Closed"],
      Paused: ["Active", "Closed"],
      Gap: ["Active", "Closed"],
      Closed: [],
    };
    if (!allowed[current].includes(next)) return false;
    this.subscriptionStatus = next;
    return true;
  }

  acknowledge(sequence: number): boolean {
    if (this.subscriptionStatus !== null && this.subscriptionStatus !== "Active") return false;
    if (!this.current || sequence < this.acknowledgedSequence || sequence > this.current.last_event_sequence) return false;
    this.acknowledgedSequence = sequence;
    return true;
  }

  acceptEventBatch(batch: EventBatch): ProjectionEventResult {
    if (!this.current) return { accepted: false, reason: "snapshot-required" };
    if (this.subscriptionStatus !== null && this.subscriptionStatus !== "Active") return { accepted: false, reason: "lifecycle" };
    if (this.subscriptionId && batch.subscription_id !== this.subscriptionId) return { accepted: false, reason: "identity" };
    if (batch.has_gap || batch.status === "Gap") return { accepted: false, reason: "gap" };
    if (batch.status !== "Active") return { accepted: false, reason: "lifecycle" };
    if (!Number.isInteger(batch.from_event_sequence) || !Number.isInteger(batch.next_event_sequence) || batch.from_event_sequence < 0 || batch.next_event_sequence < batch.from_event_sequence) return { accepted: false, reason: "range" };
    if (batch.events.length > 256 || batch.events.length > 0 && batch.next_event_sequence !== batch.events[batch.events.length - 1].sequence) return { accepted: false, reason: "range" };
    if (batch.events.length === 0 && batch.next_event_sequence !== batch.from_event_sequence) return { accepted: false, reason: "range" };
    if (batch.events.length > 0 && batch.events[0].sequence !== batch.from_event_sequence + 1) return { accepted: false, reason: "range" };
    if (batch.projection_revision[0] < this.current.projection_revision[0]) return { accepted: false, reason: "stale" };
    if (batch.from_event_sequence > this.current.last_event_sequence) return { accepted: false, reason: "gap" };
    if (batch.from_event_sequence < this.acknowledgedSequence) return { accepted: false, reason: "stale" };

    let previousSequence = batch.from_event_sequence;
    const fresh: ControlEvent[] = [];
    for (const event of batch.events) {
      if (!Number.isInteger(event.sequence) || event.sequence <= previousSequence || event.sequence > batch.next_event_sequence || !event.event_id.trim() || !Number.isInteger(event.source_revision[0]) || event.source_revision[0] < 0 || event.source_revision[0] > batch.projection_revision[0]) return { accepted: false, reason: "metadata" };
      previousSequence = event.sequence;
      if (event.project_id[0] !== this.current.project_id[0]) return { accepted: false, reason: "identity" };
      const knownTask = this.taskId?.[0];
      if (knownTask && event.task_id && event.task_id[0] !== knownTask) return { accepted: false, reason: "identity" };
      const fingerprint = JSON.stringify(event);
      const knownFingerprint = this.eventFingerprints.get(event.sequence);
      const knownEventIdSequence = this.eventIds.get(event.event_id);
      if (knownEventIdSequence !== undefined && knownEventIdSequence !== event.sequence) return { accepted: false, reason: "metadata" };
      if (event.sequence <= this.current.last_event_sequence) {
        if (!knownFingerprint) return { accepted: false, reason: "stale" };
        if (knownFingerprint !== fingerprint) return { accepted: false, reason: "conflict" };
      } else {
        if (knownFingerprint && knownFingerprint !== fingerprint) return { accepted: false, reason: "conflict" };
        fresh.push(event);
      }
    }
    if (fresh.length === 0) return { accepted: false, reason: "duplicate" };
    if (batch.projection_revision[0] <= this.current.projection_revision[0]) return { accepted: false, reason: "stale" };
    let expected = this.current.last_event_sequence + 1;
    for (const event of fresh) {
      if (event.sequence !== expected) return { accepted: false, reason: "gap" };
      expected += 1;
    }
    for (const event of fresh) {
      this.events.set(event.sequence, event);
      this.eventFingerprints.set(event.sequence, JSON.stringify(event));
      this.eventIds.set(event.event_id, event.sequence);
      if (!this.taskId && event.task_id) this.taskId = event.task_id;
    }
    this.current = this.applyEvents(fresh, batch.projection_revision);
    return { accepted: true, events: fresh };
  }

  private applyEvents(events: ControlEvent[], projectionRevision: [number]): ProjectionSnapshot {
    if (!this.current) throw new Error("projection snapshot required");
    const next = { ...this.current, projection_revision: projectionRevision, last_event_sequence: events[events.length - 1].sequence };
    for (const event of events) {
      if (event.kind === "SubmitInstruction") {
        next.task_state = "Planning";
        next.preview_truth = "Requested";
        next.current_source_revision = event.source_revision;
      } else if (event.kind === "Reconnect") {
        next.continuity_state = "ActiveBackground";
      } else if (event.kind === "PauseTask") {
        next.task_state = "Paused";
      } else if (event.kind === "ResumeTask") {
        next.task_state = "Planning";
      } else if (event.kind === "CancelTask") {
        next.task_state = "Cancelled";
      }
    }
    return next;
  }
}

export function isTauriHost(): boolean { return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window; }
export function hostUnavailable(): Error { return new Error("Nirman desktop host is unavailable; browser preview is non-authoritative"); }
export function safeErrorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "safe_message" in error && typeof (error as { safe_message?: unknown }).safe_message === "string") return (error as { safe_message: string }).safe_message;
  if (error instanceof Error) return error.message;
  return "The local control plane rejected the request";
}
export function makeClientId(prefix: string): string { return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`; }

export async function getHandshake(): Promise<SessionHandshake> {
  if (!isTauriHost()) throw hostUnavailable();
  return invoke<SessionHandshake>("handshake");
}
export async function getWorkspace(): Promise<WorkspaceDescriptor> {
  if (!isTauriHost()) throw hostUnavailable();
  return invoke<WorkspaceDescriptor>("workspace");
}
export async function getProjection(handshake: SessionHandshake): Promise<CommandResponse> {
  if (!isTauriHost()) throw hostUnavailable();
  return invoke<CommandResponse>("projection", { auth: handshake.auth, correlation_id: handshake.correlation_id });
}
export async function dispatchCommand(request: CommandRequest): Promise<CommandResponse> {
  if (!isTauriHost()) throw hostUnavailable();
  return invoke<CommandResponse>("dispatch", { request });
}

function providerCommandRequest(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  kind: "ProviderTest" | "SettingsUpdateProvider",
  payload: ProviderTestCommandPayload | SettingsUpdateProviderCommandPayload,
): CommandRequest {
  return {
    protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
    auth: handshake.auth,
    correlation_id: handshake.correlation_id,
    causation_id: null,
    deadline_epoch_seconds: null,
    command: {
      command_id: makeClientId(kind === "ProviderTest" ? "provider-test" : "provider-settings"),
      project_id: snapshot.project_id,
      task_id: null,
      kind,
      payload: JSON.stringify(payload),
      expected_projection_revision: snapshot.projection_revision,
      idempotency_key: makeClientId("ui"),
    },
  };
}

export async function testProvider(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  payload: ProviderTestCommandPayload,
): Promise<CommandResponse> {
  return dispatchCommand(providerCommandRequest(handshake, snapshot, "ProviderTest", payload));
}

export async function executeProvider(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  taskId: ProjectId,
  payload: ProviderExecuteCommandPayload,
): Promise<CommandResponse> {
  const request: CommandRequest = {
    protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
    auth: handshake.auth,
    correlation_id: handshake.correlation_id,
    causation_id: null,
    deadline_epoch_seconds: null,
    command: {
      command_id: makeClientId("provider-execute"),
      project_id: snapshot.project_id,
      task_id: taskId,
      kind: "ProviderExecute",
      payload: JSON.stringify(payload),
      expected_projection_revision: snapshot.projection_revision,
      idempotency_key: makeClientId("provider-execute-idempotency"),
    },
  };
  return dispatchCommand(request);
}

function taskScopedCommandRequest(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  taskId: ProjectId,
  commandId: string,
  kind: CommandKind,
  payload: string,
): CommandRequest {
  return {
    protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
    auth: handshake.auth,
    correlation_id: handshake.correlation_id,
    causation_id: null,
    deadline_epoch_seconds: null,
    command: {
      command_id: commandId,
      project_id: snapshot.project_id,
      task_id: taskId,
      kind,
      payload,
      expected_projection_revision: snapshot.projection_revision,
      idempotency_key: makeClientId("ui"),
    },
  };
}

export async function runAgentLoop(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  taskId: ProjectId,
  payload: AgentLoopRunCommandPayload,
  commandId = makeClientId("agent-loop-run"),
): Promise<CommandResponse> {
  return dispatchCommand(
    taskScopedCommandRequest(handshake, snapshot, taskId, commandId, "AgentLoopRun", JSON.stringify(payload)),
  );
}

export async function buildArtifact(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  taskId: ProjectId,
  payload: ArtifactBuildCommandPayload,
  commandId = makeClientId("artifact-build"),
): Promise<CommandResponse> {
  return dispatchCommand(
    taskScopedCommandRequest(handshake, snapshot, taskId, commandId, "ArtifactBuild", JSON.stringify(payload)),
  );
}

export async function exportArtifact(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  taskId: ProjectId,
  payload: ArtifactExportCommandPayload,
  commandId = makeClientId("artifact-export"),
): Promise<CommandResponse> {
  return dispatchCommand(
    taskScopedCommandRequest(handshake, snapshot, taskId, commandId, "ArtifactExport", JSON.stringify(payload)),
  );
}

/** Starts the M48 preview pipeline for the current source revision. With a
 * device serial bound the host runs a real adb install/launch/observation
 * session against the exported APK; without one the host selects the
 * headless smoke-test fallback. */
export async function startPreview(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  taskId: ProjectId,
  payload: PreviewStartCommandPayload,
  commandId = makeClientId("preview-start"),
): Promise<CommandResponse> {
  return dispatchCommand(
    taskScopedCommandRequest(handshake, snapshot, taskId, commandId, "PreviewStart", JSON.stringify(payload)),
  );
}

/** Reads one persisted preview-evidence file (screenshot, logcat, UI dump)
 * recorded by a real device session. The host verifies the session and keeps
 * reads inside the workspace `.nirman-evidence` tree. */
export async function readPreviewEvidence(
  handshake: SessionHandshake,
  path: string,
): Promise<PreviewEvidenceReadResult> {
  if (!isTauriHost()) throw hostUnavailable();
  return invoke<PreviewEvidenceReadResult>("read_preview_evidence", {
    request: { auth: handshake.auth, correlation_id: handshake.correlation_id, path },
  });
}

export async function createAndroidConstructionContract(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  taskId: ProjectId,
  contract: AndroidConstructionContract,
): Promise<CommandResponse> {
  const request: CommandRequest = {
    protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
    auth: handshake.auth,
    correlation_id: handshake.correlation_id,
    causation_id: null,
    deadline_epoch_seconds: null,
    command: {
      command_id: makeClientId("android-construction"),
      project_id: snapshot.project_id,
      task_id: taskId,
      kind: "AndroidConstructionCreate",
      payload: JSON.stringify({ contract } satisfies AndroidConstructionCommandPayload),
      expected_projection_revision: snapshot.projection_revision,
      idempotency_key: makeClientId("ui"),
    },
  };
  return dispatchCommand(request);
}

export async function preflightAndroidToolchain(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  taskId: ProjectId,
  buildVariant: string,
): Promise<CommandResponse> {
  const request: CommandRequest = {
    protocol_schema_version: PROTOCOL_SCHEMA_VERSION,
    auth: handshake.auth,
    correlation_id: handshake.correlation_id,
    causation_id: null,
    deadline_epoch_seconds: null,
    command: {
      command_id: makeClientId("android-toolchain-preflight"),
      project_id: snapshot.project_id,
      task_id: taskId,
      kind: "AndroidToolchainPreflight",
      payload: JSON.stringify({ buildVariant } satisfies AndroidToolchainPreflightCommandPayload),
      expected_projection_revision: snapshot.projection_revision,
      idempotency_key: makeClientId("ui"),
    },
  };
  return dispatchCommand(request);
}

export async function updateProviderProfile(
  handshake: SessionHandshake,
  snapshot: ProjectionSnapshot,
  profile: ProviderProfile,
): Promise<CommandResponse> {
  return dispatchCommand(providerCommandRequest(handshake, snapshot, "SettingsUpdateProvider", { profile }));
}
export async function subscribeEvents(handshake: SessionHandshake): Promise<SubscriptionBootstrap> {
  if (!isTauriHost()) throw hostUnavailable();
  const subscription: EventSubscription = {
    subscription_id: makeClientId("sub"), connection_id: makeClientId("connection"), auth: handshake.auth, project_id: handshake.auth.project_scope, task_id: null,
    from_event_sequence: 0, snapshot_revision: null, requested_projection_kinds: ["task", "preview", "backgroundContinuity", "evidence"], acknowledged_event_sequence: 0,
    heartbeat_interval_seconds: 15, max_batch_size: 64, backpressure_policy: "RejectOverLimit", status: "Requested", correlation_id: handshake.correlation_id,
  };
  return invoke<SubscriptionBootstrap>("subscribe_events", { subscription });
}
export async function replayEvents(subscription: EventSubscription, afterSequence: number): Promise<EventBatch> {
  if (!isTauriHost()) throw hostUnavailable();
  return invoke<EventBatch>("replay_events", { subscription: { ...subscription, from_event_sequence: afterSequence, acknowledged_event_sequence: afterSequence } });
}
export async function acknowledgeSubscription(handshake: SessionHandshake, subscriptionId: string, sequence: number): Promise<void> {
  if (!isTauriHost()) throw hostUnavailable();
  const acknowledgement: SubscriptionAcknowledgement = { auth: handshake.auth, subscription_id: subscriptionId, acknowledged_event_sequence: sequence, correlation_id: handshake.correlation_id };
  return invoke<void>("acknowledge_subscription", { acknowledgement });
}
export async function heartbeatSubscription(handshake: SessionHandshake, subscriptionId: string): Promise<EventBatch> {
  if (!isTauriHost()) throw hostUnavailable();
  const control: SubscriptionControl = { auth: handshake.auth, subscription_id: subscriptionId, correlation_id: handshake.correlation_id };
  return invoke<EventBatch>("heartbeat_subscription", { control });
}
export async function closeSubscription(handshake: SessionHandshake, subscriptionId: string): Promise<void> {
  if (!isTauriHost()) throw hostUnavailable();
  const control: SubscriptionControl = { auth: handshake.auth, subscription_id: subscriptionId, correlation_id: handshake.correlation_id };
  return invoke<void>("close_subscription", { control });
}
export async function subscribeToControlEvents(onBatch: (batch: EventBatch) => void): Promise<UnlistenFn> {
  if (!isTauriHost()) throw hostUnavailable();
  return listen<EventBatch>("nirman://control-event", (event) => onBatch(event.payload));
}
