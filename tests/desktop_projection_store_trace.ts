import { ProjectionStore, PROTOCOL_SCHEMA_VERSION, type EventBatch, type EventSubscription, type ProjectionSnapshot, type SubscriptionBootstrap } from "../apps/desktop/src/ipcClient.ts";

const snapshot: ProjectionSnapshot = {
  project_id: ["project-0001"], projection_revision: [0], task_state: "Created", continuity_state: "ActiveBackground", preview_truth: "Predicted", current_source_revision: [0], last_event_sequence: 0, last_known_good_ref: null,
};
const subscription: EventSubscription = {
  subscription_id: "sub-1", connection_id: "connection-1", auth: { installation_id: "install-1", user_scope: "local-user", project_scope: "project-0001", schema_version: PROTOCOL_SCHEMA_VERSION }, project_id: "project-0001", task_id: null,
  from_event_sequence: 0, snapshot_revision: null, requested_projection_kinds: ["task", "preview"], acknowledged_event_sequence: 0, heartbeat_interval_seconds: 15, max_batch_size: 64, backpressure_policy: "RejectOverLimit", status: "Active", correlation_id: "corr-1",
};
const event = (sequence: number, payload = "Build an Android notes app", overrides: Record<string, unknown> = {}) => ({
  event_id: `event-${sequence}`, sequence, project_id: ["project-0001"] as [string], task_id: ["task-0001"] as [string], kind: "SubmitInstruction", payload, source_revision: [1] as [number], ...overrides,
});
const batch = (events: ReturnType<typeof event>[], from = 0, next = events.length ? events[events.length - 1].sequence : from, revision = 1, overrides: Partial<EventBatch> = {}): EventBatch => ({
  subscription_id: "sub-1", projection_revision: [revision], from_event_sequence: from, next_event_sequence: next, events, has_gap: false, status: "Active", ...overrides,
});
const expectReason = (result: ReturnType<ProjectionStore["acceptEventBatch"]>, reason: string, label: string) => {
  if (result.accepted || result.reason !== reason) throw new Error(`${label}: expected ${reason}, received ${result.accepted ? "accepted" : result.reason}`);
};

const store = new ProjectionStore();
const bootstrap: SubscriptionBootstrap = {
  subscription,
  snapshot,
  batch: { subscription_id: "sub-1", projection_revision: [0], from_event_sequence: 0, next_event_sequence: 0, events: [], has_gap: false, status: "Active" },
};
if (!store.acceptBootstrap(bootstrap)) throw new Error("authoritative subscription bootstrap was rejected");
if (!store.acceptEventBatch(batch([event(1)])).accepted) throw new Error("ordered host event was rejected");
if (!store.acceptAuthoritativeSnapshot({ ...snapshot, projection_revision: [1], task_state: "Planning", preview_truth: "Requested", current_source_revision: [1], last_event_sequence: 1 })) throw new Error("authoritative command snapshot was rejected");
expectReason(store.acceptEventBatch(batch([event(1)])), "duplicate", "duplicate event");
expectReason(store.acceptEventBatch(batch([event(1, "conflicting payload")])), "conflict", "conflicting duplicate event");
expectReason(store.acceptEventBatch(batch([event(3)], 2, 3, 2)), "gap", "sequence gap");
expectReason(store.acceptEventBatch(batch([event(1)], 0, 1, 1, { subscription_id: "other-sub" })), "identity", "subscription identity");
if (store.acceptSnapshot({ ...snapshot, projection_revision: [0], last_event_sequence: 0 })) throw new Error("stale snapshot was accepted");

const resetStore = new ProjectionStore();
if (!resetStore.acceptBootstrap(bootstrap)) throw new Error("reset-store bootstrap was rejected");
if (!resetStore.acceptEventBatch(batch([event(1)])).accepted) throw new Error("reset-store event was rejected");
if (!resetStore.acceptSnapshot({ ...snapshot, projection_revision: [2], task_state: "Planning", preview_truth: "Requested", current_source_revision: [1], last_event_sequence: 1 })) throw new Error("newer snapshot was rejected");
expectReason(resetStore.acceptEventBatch(batch([event(1, "stale conflicting payload")], 0, 1, 2)), "conflict", "stale conflicting event after snapshot reset");

expectReason(store.acceptEventBatch(batch([event(2)], 0, 2, 2)), "range", "event does not begin at declared cursor");
expectReason(store.acceptEventBatch(batch([event(3)], 2, 3, 2)), "gap", "missing sequence after valid from cursor");
const rangeStore = new ProjectionStore();
if (!rangeStore.acceptBootstrap(bootstrap)) throw new Error("range-store bootstrap was rejected");
expectReason(rangeStore.acceptEventBatch(batch([], 0, 1, 2)), "range", "empty inconsistent range");
expectReason(store.acceptEventBatch(batch([event(2)], 0, 1, 2)), "range", "next cursor does not match event");
expectReason(store.acceptEventBatch(batch([event(2)], 1, 2, 2, { projection_revision: [1] })), "stale", "non-advancing projection revision");
expectReason(store.acceptEventBatch(batch([{ ...event(2), event_id: "" }], 1, 2, 2)), "metadata", "missing event ID");
expectReason(store.acceptEventBatch(batch([{ ...event(2), source_revision: [3] as [number] }], 1, 2, 2)), "metadata", "invalid source revision");
expectReason(store.acceptEventBatch(batch([{ ...event(2), event_id: "event-1" }], 1, 2, 2)), "metadata", "event ID sequence mismatch");
expectReason(store.acceptEventBatch(batch([{ ...event(2), task_id: ["other-task"] as [string] }], 1, 2, 2)), "identity", "task identity mismatch");

if (!store.transitionSubscription("Paused")) throw new Error("valid Active to Paused transition rejected");
expectReason(store.acceptEventBatch(batch([event(2)], 1, 2, 2)), "lifecycle", "event accepted while paused");
if (store.acknowledge(1)) throw new Error("acknowledgement accepted while paused");
if (store.transitionSubscription("Requested")) throw new Error("invalid Paused to Requested transition accepted");
if (!store.transitionSubscription("Active")) throw new Error("valid Paused to Active transition rejected");
if (!store.acknowledge(1)) throw new Error("valid acknowledgement rejected");
if (store.acknowledge(0)) throw new Error("acknowledgement moved the cursor backwards");

const contradictory = { ...store.snapshot()!, task_state: "Completed" as const };
if (store.acceptAuthoritativeSnapshot(contradictory)) throw new Error("contradictory equal-cursor snapshot was accepted");
if (!store.acceptAuthoritativeSnapshot({ ...store.snapshot()!, last_known_good_ref: "preview-ref-1" })) throw new Error("consistent equal-cursor enrichment was rejected");
if (store.acceptAuthoritativeSnapshot({ ...store.snapshot()!, last_known_good_ref: "different-ref" })) throw new Error("conflicting equal-cursor enrichment was accepted");

console.log(JSON.stringify({
  schema: "nirman.react_projection_trace.v3",
  status: "HEADLESS_PROJECTION_REDUCER_TRACE_ONLY",
  snapshotBootstrap: true,
  orderedEventDelivery: true,
  authoritativeCommandSnapshotCutover: true,
  duplicateRejected: true,
  conflictingDuplicateRejected: true,
  staleConflictingEventAfterSnapshotResetRejected: true,
  batchRangeIntegrityRejected: true,
  eventMetadataRejected: true,
  sequenceGapRejected: true,
  subscriptionIdentityRejected: true,
  subscriptionLifecycleRejected: true,
  acknowledgedCursorProtected: true,
  contradictoryEqualCursorSnapshotRejected: true,
  consistentEqualCursorEnrichmentAccepted: true,
  staleSnapshotRejected: true,
  productionReactDomRuntime: false,
}));
