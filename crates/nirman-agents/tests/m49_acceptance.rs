use nirman_agents::{DecisionTrace, ProviderModelProvenance, M49_SCHEMA_VERSION};
use nirman_policy::{govern_resources, ResourceBudget, ResourcePressure, ResourceSnapshot};
use serde_json::json;
use std::{fs, path::PathBuf};

#[test]
fn m49_acceptance_is_evidence_linked_and_safety_preserving() {
    let trace = DecisionTrace {
        schema_version: M49_SCHEMA_VERSION,
        decision_id: "decision-m49".into(),
        session_id: "session-1".into(),
        task_id: "task-1".into(),
        worker_id: None,
        input_references: vec!["preview-request-1".into()],
        constraints: vec!["android-only".into()],
        candidate_actions: vec!["compose-reload".into(), "apk-reinstall".into()],
        selected_action: "compose-reload".into(),
        deterministic_policy_checks: vec!["policy-allow".into()],
        provider_model_provenance: Some(ProviderModelProvenance {
            provider_id: "provider-1".into(),
            model_id: "model-1".into(),
            request_id: Some("request-1".into()),
        }),
        confidence_percent: 90,
        outcome_event: "DecisionRecorded".into(),
        evidence_ids: vec!["evidence-m49".into()],
    };
    trace.validate().expect("valid decision trace");
    let snapshot = ResourceSnapshot {
        cpu_percent: 50,
        memory_mb: 100,
        disk_free_mb: 1000,
        checkpoint_storage_mb: 100,
        emulator_memory_mb: 200,
        gradle_memory_mb: 200,
        worker_concurrency: 8,
        provider_concurrency: 2,
        context_tokens: 5000,
        log_volume_mb: 10,
        build_duration_seconds: 10,
        device_slots_used: 0,
        device_slots_total: 1,
    };
    let budget = ResourceBudget {
        max_cpu_percent: None,
        max_memory_mb: None,
        min_disk_free_mb: None,
        max_checkpoint_storage_mb: None,
        max_emulator_memory_mb: None,
        max_gradle_memory_mb: None,
        max_worker_concurrency: Some(2),
        max_provider_concurrency: None,
        max_context_tokens: Some(2000),
        max_log_volume_mb: None,
        max_build_duration_seconds: None,
    };
    let decision =
        govern_resources("resource-decision-m49", &snapshot, &budget).expect("resource decision");
    assert_eq!(decision.pressure, ResourcePressure::Elevated);
    assert!(decision.safety_gates_preserved);
    let evidence = json!({"schema":"nirman.m49.acceptance.v1","decisionTraceObserved":true,"resourceDecisionObserved":true,"safetyGatesPreservedObserved":decision.safety_gates_preserved,"headlessOnly":true,"nativeWindowsTauriRuntimeObserved":false,"androidRuntimeObserved":false,"m49Status":"M49_HEADLESS_AUTHORITY_TRACE_ONLY"});
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/evidence/m49_acceptance.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
}
