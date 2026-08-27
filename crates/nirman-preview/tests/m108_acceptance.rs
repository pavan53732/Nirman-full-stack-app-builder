use nirman_preview::{
    PreviewEventTruth, PreviewSyncEvent, PreviewSyncEventType, M108_SCHEMA_VERSION,
};

#[test]
fn m108_sync_event_requires_causal_identity_and_evidence_lineage() {
    let event = PreviewSyncEvent {
        event_id: "event-m108-1".into(),
        event_sequence: 1,
        project_id: "project-1".into(),
        task_id: "task-1".into(),
        correlation_id: "corr-1".into(),
        causation_id: None,
        candidate_preview_revision_id: "preview-1".into(),
        event_type: PreviewSyncEventType::BuildObserved,
        event_truth: PreviewEventTruth::Observed,
        project_revision_id: "source-1".into(),
        checkpoint_id: "checkpoint-1".into(),
        source_fingerprint: "sha256:source".into(),
        artifact_id: Some("apk-1".into()),
        artifact_fingerprint: Some("sha256:apk".into()),
        runtime_session_id: None,
        device_id: None,
        operation_ref: "operation-1".into(),
        observation_refs: vec!["observation-build-1".into()],
        evidence_refs: vec!["evidence-build-1".into()],
        validation_ref: None,
        payload: "build observed by executor".into(),
    };
    event.validate().expect("valid M108 event");
    assert_eq!(M108_SCHEMA_VERSION, 1);
    let json = serde_json::to_string(&event).expect("event serialization");
    let restored: PreviewSyncEvent = serde_json::from_str(&json).expect("event reload");
    assert_eq!(restored, event);
}

#[test]
fn m108_rejects_events_without_identity() {
    let mut event = PreviewSyncEvent {
        event_id: "".into(),
        event_sequence: 1,
        project_id: "p".into(),
        task_id: "t".into(),
        correlation_id: "c".into(),
        causation_id: None,
        candidate_preview_revision_id: "r".into(),
        event_type: PreviewSyncEventType::IntentAccepted,
        event_truth: PreviewEventTruth::Requested,
        project_revision_id: "s".into(),
        checkpoint_id: "c".into(),
        source_fingerprint: "f".into(),
        artifact_id: None,
        artifact_fingerprint: None,
        runtime_session_id: None,
        device_id: None,
        operation_ref: "o".into(),
        observation_refs: vec![],
        evidence_refs: vec![],
        validation_ref: None,
        payload: "intent".into(),
    };
    assert!(event.validate().is_err());
    event.event_id = "event-1".into();
    event.event_sequence = 0;
    assert!(event.validate().is_err());
}
