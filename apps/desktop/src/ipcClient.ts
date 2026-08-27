import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const PROTOCOL_SCHEMA_VERSION = 1;

export type PreviewTruth = "Predicted" | "Requested" | "Observed" | "Verified" | "Stale" | "Invalidated";
export type ProductLifecycleState = "Created" | "Planning" | "Implementing" | "Paused" | "Previewing" | "Validating" | "Recovering" | "Packaging" | "Completed" | "UserRequired" | "SafelyFailed" | "Cancelled";
export type BackgroundContinuityState = "ActiveBackground" | "UiDisconnected" | "HostSuspended" | "HostOffline" | "DeviceUnavailable" | "ProviderUnavailable" | "Recovering" | "Reconciling" | "UserRequired" | "SafelyFailed" | "Completed";
export type CommandKind = "ProjectOpen" | "TaskStart" | "TaskCancel" | "TaskResume" | "WorkspaceApplyPatch" | "PreviewStart" | "PreviewStop" | "PreviewPromote" | "ValidationRun" | "ArtifactBuild" | "ArtifactExport" | "ProviderTest" | "SettingsUpdateProvider" | "AndroidConstructionCreate" | "AndroidToolchainPreflight" | "SubmitInstruction" | "Reconnect" | "PauseTask" | "ResumeTask" | "CancelTask";
export type ResponseStatus = "Accepted" | "Completed" | "Rejected" | "Duplicate" | "Stale" | "Cancelled" | "Failed";
export type SubscriptionStatus = "Requested" | "Active" | "Paused" | "Gap" | "Closed";

export type ProjectId = [string];
export interface AuthContext { installation_id: string; user_scope: string; project_scope: string; schema_version: number }
export interface ProjectionSnapshot {
  project_id: ProjectId;
  projection_revision: [number];
  task_state: ProductLifecycleState;
  continuity_state: BackgroundContinuityState;
  preview_truth: PreviewTruth;
  current_source_revision: [number];
  last_event_sequence: number;
  last_known_good_ref: string | null;
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
