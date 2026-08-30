// Pure unit tests for the UI's construction-contract derivation. These run
// with `node --test` (no Tauri host required) and pin the exact contract
// shape the desktop integration tests mirror on the Rust side.
import { strict as assert } from "node:assert";
import { test } from "node:test";
import {
  deriveConstructionPipeline,
  derivePreviewRequest,
  exportDestination,
  DEFAULT_BUILD_VARIANT,
  DEFAULT_GRADLE_TASK,
  DEFAULT_ITERATION_BUDGET,
} from "./contract.ts";

const BASE = { projectId: "project-0001", taskId: "task-ui-1", pipelineId: "ui-1" };

test("derives a valid contract from a plain intent", () => {
  const { contract } = deriveConstructionPipeline({ ...BASE, intent: "Build an offline-first notes app with camera capture and location reminders." });
  assert.equal(contract.schemaVersion, 1);
  assert.equal(contract.contractId, "contract-ui-1");
  assert.deepEqual(contract.projectId, ["project-0001"]);
  assert.deepEqual(contract.taskId, ["task-ui-1"]);
  assert.deepEqual(contract.targetPlatforms, ["android"]);
  assert.equal(contract.userIntent, "Build an offline-first notes app with camera capture and location reminders.");
  assert.ok(contract.features.length >= 1, "features must not be empty");
  assert.ok(contract.ui.length >= 1, "ui requirements must not be empty");
  assert.ok(contract.data.length >= 1, "data requirements must not be empty");
  assert.deepEqual(contract.technologyPlan.taskId, ["task-ui-1"]);
  assert.ok(contract.technologyPlan.requestedCapabilities.length >= 1);
  assert.ok(contract.technologyPlan.requiredToolchains.length >= 1);
  assert.ok(contract.technologyPlan.validationPlan.length >= 1);
  assert.deepEqual(contract.technologyPlan.selectedLanguages, ["kotlin"]);
  assert.deepEqual(contract.technologyPlan.selectedUiFrameworks, ["jetpack-compose"]);
  assert.equal(contract.deviceMatrix.length, 1);
  assert.equal(contract.deviceMatrix[0].apiLevel, 35);
  assert.ok(contract.validationModel.requiredChecks.length >= 1);
  assert.ok(contract.validationModel.acceptanceCriteria.length >= 1);
  assert.equal(contract.artifactModel.requiredArtifact, "apk");
  // Every requirement id is unique (duplicate ids fail domain validation).
  const ids = [
    ...contract.features,
    ...contract.ui,
    ...contract.data,
    ...contract.integrations,
    ...contract.androidRequirements,
  ].map((requirement) => requirement.requirementId);
  assert.equal(new Set(ids).size, ids.length);
  // All requirements are user facts with no dangling source references.
  for (const requirement of [...contract.features, ...contract.ui, ...contract.data]) {
    assert.equal(requirement.origin, "user_fact");
    assert.deepEqual(requirement.sourceReferenceIds, []);
    assert.ok(requirement.statement.trim().length > 0);
  }
});

test("keyword capabilities map to declared device permissions", () => {
  const { contract } = deriveConstructionPipeline({ ...BASE, intent: "A camera scanner that records voice notes and syncs over the network." });
  const capabilities = contract.technologyPlan.requestedCapabilities;
  assert.ok(capabilities.includes("camera-capture"));
  assert.ok(capabilities.includes("audio-capture"));
  assert.ok(capabilities.includes("network-sync"));
  const permissions = contract.deviceMatrix[0].permissions;
  assert.ok(permissions.includes("android.permission.CAMERA"));
  assert.ok(permissions.includes("android.permission.RECORD_AUDIO"));
  assert.ok(permissions.includes("android.permission.INTERNET"));
});

test("offline intents default to offline-storage without device permissions", () => {
  const { contract } = deriveConstructionPipeline({ ...BASE, intent: "Keep my notes readable offline." });
  assert.ok(contract.technologyPlan.requestedCapabilities.includes("offline-storage"));
  assert.deepEqual(contract.deviceMatrix[0].permissions, []);
  assert.equal(contract.deviceMatrix[0].networkProfile, "offline-capable");
});

test("derivation is deterministic", () => {
  const input = { ...BASE, intent: "Track workouts with charts. Log sets. Share summaries." };
  const first = deriveConstructionPipeline(input);
  const second = deriveConstructionPipeline(input);
  assert.deepEqual(first, second);
});

test("multi-sentence intents become distinct feature requirements", () => {
  const { contract } = deriveConstructionPipeline({ ...BASE, intent: "Track workouts. Log sets per exercise. Show weekly charts. Export data." });
  assert.equal(contract.features.length, 4);
  assert.deepEqual(contract.features.map((feature) => feature.statement), [
    "Track workouts.",
    "Log sets per exercise.",
    "Show weekly charts.",
    "Export data.",
  ]);
});

test("empty intents still derive a buildable contract", () => {
  const { contract } = deriveConstructionPipeline({ ...BASE, intent: "   " });
  assert.equal(contract.features.length, 1);
  assert.ok(contract.features[0].statement.length > 0);
  assert.ok(contract.technologyPlan.requestedCapabilities.includes("offline-storage"));
  assert.deepEqual(contract.validationModel.acceptanceCriteria, ["the application builds and launches"]);
});

test("pipeline plan matches the agent-loop defaults", () => {
  const pipeline = deriveConstructionPipeline({ ...BASE, intent: "Anything." });
  assert.equal(pipeline.buildVariant, DEFAULT_BUILD_VARIANT);
  assert.equal(pipeline.gradleTask, DEFAULT_GRADLE_TASK);
  assert.equal(pipeline.iterationBudget, DEFAULT_ITERATION_BUDGET);
  assert.ok(pipeline.buildTimeoutMs > 0);
});

test("export destinations stay inside the authorized workspace", () => {
  const destination = exportDestination("/srv/nirman/workspace", "ui-42", "debug");
  assert.equal(destination, "/srv/nirman/workspace/exports/nirman-ui-42-debug.apk");
  assert.ok(destination.endsWith(".apk"));
  const windowsStyle = exportDestination("C:\\nirman\\workspace\\", "ui-42", "debug");
  assert.equal(windowsStyle, "C:\\nirman\\workspace\\exports\\nirman-ui-42-debug.apk");
});

test("preview requests bind the durable source revision and contract identity", () => {
  const { contract } = deriveConstructionPipeline({ ...BASE, intent: "An offline journal with photo notes." });
  const request = derivePreviewRequest({
    contract,
    pipelineId: "ui-9",
    sourceRevision: 3,
    sourceFingerprint: "sha256:abc123",
    buildVariant: "debug",
    deviceSerial: "emulator-5554",
  });
  assert.equal(request.schema_version, 1);
  assert.equal(request.request_id, "preview-request-ui-9");
  assert.equal(request.project_id, "project-0001");
  assert.equal(request.task_id, "task-ui-1");
  assert.equal(request.project_revision_id, "source-3");
  assert.equal(request.checkpoint_id, "checkpoint-ui-9");
  assert.equal(request.source_fingerprint, "sha256:abc123");
  assert.equal(request.contract_version, contract.contractId);
  assert.equal(request.technology_plan_version, contract.technologyPlan.planId);
  assert.equal(request.build_variant, "debug");
  assert.equal(request.device_id, "emulator-5554");
  assert.equal(request.android_api_level, contract.deviceMatrix[0].apiLevel);
  assert.equal(request.selected_language, "kotlin");
  assert.equal(request.selected_ui_framework, "jetpack-compose");
  assert.equal(request.requested_mode, null);
  assert.deepEqual(request.changed_paths, []);
  assert.ok(request.required_evidence_kinds.includes("DEVICE_EVIDENCE"));
});

test("an absent device serial selects the headless fallback without claiming a device", () => {
  const { contract } = deriveConstructionPipeline({ ...BASE, intent: "Notes." });
  const headless = derivePreviewRequest({
    contract,
    pipelineId: "ui-9",
    sourceRevision: 0,
    sourceFingerprint: "sha256:def456",
    buildVariant: "debug",
    deviceSerial: "",
  });
  assert.equal(headless.device_id, null);
  assert.equal(headless.android_api_level, null);
  // Whitespace-only serials are normalized to the headless path too.
  const padded = derivePreviewRequest({
    contract,
    pipelineId: "ui-9",
    sourceRevision: 0,
    sourceFingerprint: "sha256:def456",
    buildVariant: "debug",
    deviceSerial: "   ",
  });
  assert.equal(padded.device_id, null);
  // A trimmed serial is bound verbatim (the host compares it against adb).
  const trimmed = derivePreviewRequest({
    contract,
    pipelineId: "ui-9",
    sourceRevision: 0,
    sourceFingerprint: "sha256:def456",
    buildVariant: "debug",
    deviceSerial: "  emulator-5554  ",
  });
  assert.equal(trimmed.device_id, "emulator-5554");
});

test("preview derivation is deterministic for identical inputs", () => {
  const { contract } = deriveConstructionPipeline({ ...BASE, intent: "Habit tracker with streaks." });
  const input = {
    contract,
    pipelineId: "ui-77",
    sourceRevision: 5,
    sourceFingerprint: "sha256:777",
    buildVariant: "debug",
    deviceSerial: "emulator-5554" as string | null,
  };
  assert.deepEqual(derivePreviewRequest(input), derivePreviewRequest(input));
});
