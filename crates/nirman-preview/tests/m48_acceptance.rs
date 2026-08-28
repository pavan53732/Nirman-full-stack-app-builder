use nirman_preview::{
    bind_preview_revision, select_fallback, transition_preview, PreviewExecutionTruth,
    PreviewLifecycleState, PreviewMode, PreviewProjection, PreviewRequest, PreviewStatus,
    M48_SCHEMA_VERSION,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

fn request(framework: &str, device: Option<&str>, changed_paths: Vec<&str>) -> PreviewRequest {
    PreviewRequest {
        schema_version: M48_SCHEMA_VERSION,
        request_id: format!("request-{framework}"),
        project_id: "project-m48".into(),
        task_id: "task-m48".into(),
        project_revision_id: "source-1".into(),
        checkpoint_id: "checkpoint-1".into(),
        source_fingerprint: "source-fingerprint-1".into(),
        contract_version: "contract-1".into(),
        technology_plan_version: "technology-1".into(),
        asset_manifest_version: "assets-1".into(),
        build_variant: "debug".into(),
        device_id: device.map(String::from),
        android_api_level: device.map(|_| 35),
        requested_mode: None,
        selected_language: "kotlin".into(),
        selected_ui_framework: framework.into(),
        changed_paths: changed_paths.into_iter().map(String::from).collect(),
        required_evidence_kinds: vec!["PROCESS_EVIDENCE".into()],
        policy_decision_id: "policy-1".into(),
        workspace_root: None,
        build_identity: None,
    }
}

#[test]
fn m48_acceptance_proves_fallback_matrix_binding_and_truthful_promotion_gate() {
    let compose = request(
        "Jetpack Compose",
        Some("emulator-1"),
        vec!["app/src/main/res/values/strings.xml"],
    );
    let compose_selection = select_fallback(&compose).expect("Compose selection");
    assert_eq!(compose_selection.mode, PreviewMode::ComposeReload);

    let react = request(
        "React Native / Expo",
        Some("emulator-1"),
        vec!["app/src/main/App.tsx"],
    );
    let react_selection = select_fallback(&react).expect("React selection");
    assert_eq!(react_selection.mode, PreviewMode::ReactNativeExpoRefresh);

    let native = request(
        "Views",
        Some("emulator-1"),
        vec!["app/src/main/MainActivity.kt"],
    );
    let native_selection = select_fallback(&native).expect("native selection");
    assert_eq!(
        native_selection.mode,
        PreviewMode::IncrementalEmulatorInstall
    );

    let headless = request("Views", None, vec!["app/src/main/MainActivity.kt"]);
    let headless_selection = select_fallback(&headless).expect("headless selection");
    assert_eq!(headless_selection.mode, PreviewMode::HeadlessSmokeTest);
    assert!(!headless_selection.runtime_observation_required);

    let mut diagnostic = headless.clone();
    diagnostic.requested_mode = Some(PreviewMode::Diagnostic);
    let diagnostic_selection = select_fallback(&diagnostic).expect("diagnostic selection");
    assert_eq!(diagnostic_selection.mode, PreviewMode::Diagnostic);

    let revision = bind_preview_revision(&compose, &compose_selection, "preview-revision-1", 1)
        .expect("bound revision");
    assert_eq!(revision.project_revision_id, compose.project_revision_id);
    assert_eq!(revision.checkpoint_id, compose.checkpoint_id);
    assert_eq!(revision.source_fingerprint, compose.source_fingerprint);
    assert!(!revision.can_promote().eligible);

    let mut stale_request = compose.clone();
    stale_request.source_fingerprint = "source-fingerprint-2".into();
    stale_request.asset_manifest_version = "assets-2".into();
    let stale_reasons = revision.stale_reasons(&stale_request);
    assert!(stale_reasons.contains(&"source-fingerprint".into()));
    assert!(stale_reasons.contains(&"asset-manifest".into()));
    let mut projection = PreviewProjection::new("project-m48", "task-m48");
    assert!(projection
        .apply_candidate(revision.clone(), &stale_request)
        .is_err());
    projection
        .apply_candidate(revision.clone(), &compose)
        .expect("current candidate");
    assert_eq!(
        projection
            .promote_candidate()
            .expect_err("no observed promotion"),
        nirman_preview::PreviewError::RuntimeObservationRequired
    );

    let mut observed = revision.clone();
    for next in [
        PreviewLifecycleState::Building,
        PreviewLifecycleState::BuildObserved,
        PreviewLifecycleState::Installing,
        PreviewLifecycleState::InstallObserved,
        PreviewLifecycleState::Launching,
        PreviewLifecycleState::RunningObserved,
        PreviewLifecycleState::InteractionObserved,
        PreviewLifecycleState::Validating,
    ] {
        transition_preview(&mut observed, next).expect("valid observed transition");
    }
    observed.validation_status = "OBSERVED_PASS".into();
    assert_eq!(observed.execution_truth, PreviewExecutionTruth::Observed);
    projection.candidate = Some(observed);
    projection.promote_candidate().expect("observed promotion");
    assert_eq!(
        projection.active_last_known_good.as_ref().unwrap().status,
        PreviewStatus::Observed
    );

    let evidence = json!({
        "schema": "nirman.m48.preview_authority.v1",
        "composeReloadObserved": compose_selection.mode == PreviewMode::ComposeReload,
        "reactNativeExpoRefreshObserved": react_selection.mode == PreviewMode::ReactNativeExpoRefresh,
        "incrementalEmulatorInstallObserved": native_selection.mode == PreviewMode::IncrementalEmulatorInstall,
        "headlessSmokeFallbackObserved": headless_selection.mode == PreviewMode::HeadlessSmokeTest,
        "diagnosticFallbackObserved": diagnostic_selection.mode == PreviewMode::Diagnostic,
        "revisionIdentityBindingObserved": revision.project_revision_id == "source-1" && revision.checkpoint_id == "checkpoint-1" && revision.source_fingerprint == "source-fingerprint-1",
        "staleIdentityRejectedObserved": stale_reasons.len() == 2,
        "predictedPromotionRejectedObserved": true,
        "observedPromotionObserved": projection.active_last_known_good.is_some(),
        "lastKnownGoodPreservedObserved": projection.active_last_known_good.as_ref().unwrap().preview_revision_id == "preview-revision-1",
        "buildObserved": false,
        "installObserved": false,
        "launchObserved": false,
        "androidDeviceObserved": false,
        "nativeWindowsTauriRuntimeObserved": false,
        "m48Status": "M48_HEADLESS_PREVIEW_AUTHORITY_TRACE_ONLY"
    });
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/evidence/m48_preview_authority.json");
    fs::create_dir_all(path.parent().expect("evidence directory")).expect("evidence directory");
    fs::write(
        path,
        serde_json::to_vec_pretty(&evidence).expect("evidence JSON"),
    )
    .expect("evidence");
}
