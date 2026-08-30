// Deterministic construction-contract derivation for the Nirman UI.
//
// This module is PURE (no Tauri imports) so it can be unit-tested with
// node:test and mirrored by the desktop integration tests. The derivation
// turns one user instruction into the AndroidConstructionContract the
// control plane validates (nirman-domain) and the agent loop consumes.
import type {
  AndroidConstructionContract,
  AndroidDeviceProfile,
  AndroidTechnologyPlan,
  ConstructionRequirement,
  PreviewRequest,
} from "./ipcClient.js";

/** Capability keywords recognized in user intents, mirroring the scaffold's
 * permission derivation so the declared device permissions and the generated
 * AndroidManifest stay aligned. */
const CAPABILITY_KEYWORDS: ReadonlyArray<{
  keywords: string[];
  capability: string;
  permission: string;
}> = [
  { keywords: ["camera", "photo", "picture", "scan"], capability: "camera-capture", permission: "android.permission.CAMERA" },
  { keywords: ["location", "gps", "map", "nearby"], capability: "location-awareness", permission: "android.permission.ACCESS_FINE_LOCATION" },
  { keywords: ["network", "online", "sync", "cloud", "api"], capability: "network-sync", permission: "android.permission.INTERNET" },
  { keywords: ["storage", "file", "document", "download", "gallery"], capability: "local-storage", permission: "android.permission.READ_MEDIA_IMAGES" },
  { keywords: ["record", "audio", "voice", "microphone"], capability: "audio-capture", permission: "android.permission.RECORD_AUDIO" },
  { keywords: ["bluetooth", "wearable", "pair"], capability: "bluetooth-peripherals", permission: "android.permission.BLUETOOTH_CONNECT" },
  { keywords: ["notification", "remind", "alert"], capability: "notifications", permission: "android.permission.POST_NOTIFICATIONS" },
  { keywords: ["offline", "local-first", "local first", "without network"], capability: "offline-storage", permission: "" },
];

/** Splits an intent into at most `limit` trimmed, non-empty sentences. */
function intentStatements(intent: string, limit: number): string[] {
  return intent
    .split(/(?<=[.!?])\s+|\n+/)
    .map((sentence) => sentence.trim())
    .filter((sentence) => sentence.length > 0)
    .slice(0, limit);
}

function requirement(id: string, statement: string): ConstructionRequirement {
  return { requirementId: id, statement, origin: "user_fact", sourceReferenceIds: [] };
}

export interface DeriveContractInput {
  projectId: string;
  taskId: string;
  intent: string;
  /** Stable identifier for this pipeline run (e.g. `ui-<uuid>`). */
  pipelineId: string;
}

export interface DerivedPipeline {
  contract: AndroidConstructionContract;
  buildVariant: string;
  gradleTask: string;
  iterationBudget: number;
  buildTimeoutMs: number;
}

export const DEFAULT_BUILD_VARIANT = "debug";
export const DEFAULT_GRADLE_TASK = "assembleDebug";
export const DEFAULT_ITERATION_BUDGET = 8;
export const DEFAULT_BUILD_TIMEOUT_MS = 900_000;

/** Derives the full construction pipeline (contract + build plan) from one
 * user instruction. Deterministic: the same input always yields the same
 * contract, so duplicate dispatches stay idempotent. */
export function deriveConstructionPipeline(input: DeriveContractInput): DerivedPipeline {
  const intent = input.intent.trim();
  const lowered = intent.toLowerCase();
  const matched = CAPABILITY_KEYWORDS.filter((entry) =>
    entry.keywords.some((keyword) => lowered.includes(keyword)),
  );
  const capabilities = matched.map((entry) => entry.capability);
  if (!capabilities.includes("offline-storage") && lowered.includes("note")) {
    capabilities.push("offline-storage");
  }
  if (capabilities.length === 0) capabilities.push("offline-storage");

  const statements = intentStatements(intent, 5);
  const features: ConstructionRequirement[] = statements.map((statement, index) =>
    requirement(`feature-${input.pipelineId}-${index + 1}`, statement),
  );
  if (features.length === 0) {
    features.push(requirement(`feature-${input.pipelineId}-1`, intent || "Build the requested Android application"));
  }
  const ui: ConstructionRequirement[] = [
    requirement(`ui-${input.pipelineId}-1`, "Present each feature as a distinct screen with Material 3 components"),
  ];
  const data: ConstructionRequirement[] = [
    requirement(
      `data-${input.pipelineId}-1`,
      capabilities.includes("offline-storage")
        ? "Persist all user content on device so the app works without network access"
        : "Persist all user content on device",
    ),
  ];

  const device: AndroidDeviceProfile = {
    deviceId: "pixel-api-35",
    name: "Pixel API 35",
    platformVersion: "Android 15",
    apiLevel: 35,
    architecture: "x86_64",
    width: 1080,
    height: 2400,
    density: 420,
    orientation: "portrait",
    locale: "en-US",
    permissions: matched.map((entry) => entry.permission).filter((permission) => permission !== ""),
    networkProfile: capabilities.includes("network-sync") ? "online-capable" : "offline-capable",
  };

  const technologyPlan: AndroidTechnologyPlan = {
    planId: `plan-${input.pipelineId}`,
    taskId: [input.taskId],
    requestedCapabilities: capabilities,
    visualRequirements: ["material-3", "adaptive-layout"],
    selectedLanguages: ["kotlin"],
    selectedUiFrameworks: ["jetpack-compose"],
    selectedRuntimeLayers: [],
    selectedNativeModules: [],
    selectedBuildPlugins: ["android-gradle-plugin"],
    selectedDeviceApis: matched.map((entry) => entry.capability),
    selectedLibraries: capabilities.includes("offline-storage") ? ["room"] : [],
    compatibilityConstraints: ["android-api-29-plus"],
    rejectedAlternatives: ["web-target", "flutter-target"],
    requiredToolchains: ["jdk", "gradle", "android-sdk"],
    validationPlan: ["unit-tests", "android-build"],
    confidence: "high",
    revision: [1],
  };

  const contract: AndroidConstructionContract = {
    schemaVersion: 1,
    contractId: `contract-${input.pipelineId}`,
    projectId: [input.projectId],
    targetPlatforms: ["android"],
    taskId: [input.taskId],
    userIntent: intent,
    screenshots: [],
    assets: [],
    features,
    ui,
    data,
    integrations: [],
    technologyPlan,
    androidRequirements: [
      requirement(`android-${input.pipelineId}-1`, "Support the declared Android API range and device permissions"),
    ],
    deviceMatrix: [device],
    validationModel: {
      requiredChecks: ["compile", "android-build"],
      acceptanceCriteria: statements.length > 0 ? statements : ["the application builds and launches"],
    },
    artifactModel: { requiredArtifact: "apk", aabDeclared: false },
  };

  return {
    contract,
    buildVariant: DEFAULT_BUILD_VARIANT,
    gradleTask: DEFAULT_GRADLE_TASK,
    iterationBudget: DEFAULT_ITERATION_BUDGET,
    buildTimeoutMs: DEFAULT_BUILD_TIMEOUT_MS,
  };
}

/** The export destination for a loop-built APK, inside the authorized
 * workspace so the export policy never needs an external-directory grant. */
export function exportDestination(workspaceRoot: string, pipelineId: string, buildVariant: string): string {
  const root = workspaceRoot.replace(/[\\/]+$/, "");
  const separator = root.includes("\\") && !root.includes("/") ? "\\" : "/";
  return `${root}${separator}exports${separator}nirman-${pipelineId}-${buildVariant}.apk`;
}

export interface DerivePreviewInput {
  /** The construction contract already accepted by the control plane. */
  contract: AndroidConstructionContract;
  /** The pipeline identifier used for the contract (`ui-<uuid>`). */
  pipelineId: string;
  /** The durable source revision the preview binds to (`source-<n>`). */
  sourceRevision: number;
  /** The fingerprint of the source that produced the exported APK. */
  sourceFingerprint: string;
  buildVariant: string;
  /** Real adb device serial (e.g. `emulator-5554`). Empty/null selects the
   * headless smoke-test fallback — no device session is claimed. */
  deviceSerial: string | null;
}

/** Derives the M48 PreviewStart request from an accepted construction
 * contract. Deterministic per pipeline: the same inputs always produce the
 * same request, so retries stay idempotent through the control plane.
 * Device binding requires the REAL adb serial because the host compares it
 * against `adb get-serialno` during the device session. */
export function derivePreviewRequest(input: DerivePreviewInput): PreviewRequest {
  const device = input.contract.deviceMatrix[0];
  const serial = input.deviceSerial?.trim() ?? "";
  return {
    schema_version: 1,
    request_id: `preview-request-${input.pipelineId}`,
    project_id: input.contract.projectId[0],
    task_id: input.contract.taskId[0],
    project_revision_id: `source-${input.sourceRevision}`,
    checkpoint_id: `checkpoint-${input.pipelineId}`,
    source_fingerprint: input.sourceFingerprint,
    contract_version: input.contract.contractId,
    technology_plan_version: input.contract.technologyPlan.planId,
    asset_manifest_version: `assets-${input.pipelineId}`,
    build_variant: input.buildVariant,
    device_id: serial.length > 0 ? serial : null,
    android_api_level: serial.length > 0 ? (device?.apiLevel ?? 35) : null,
    requested_mode: null,
    selected_language: input.contract.technologyPlan.selectedLanguages[0] ?? "kotlin",
    selected_ui_framework: input.contract.technologyPlan.selectedUiFrameworks[0] ?? "jetpack-compose",
    changed_paths: [],
    required_evidence_kinds: ["PROCESS_EVIDENCE", "DEVICE_EVIDENCE", "VISUAL_EVIDENCE"],
    policy_decision_id: `policy-${input.pipelineId}`,
    workspace_root: null,
    build_identity: null,
  };
}
