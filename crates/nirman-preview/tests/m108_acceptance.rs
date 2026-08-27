use nirman_preview::{
    M108ProjectionState, PreviewEventTruth, PreviewSyncEvent, PreviewSyncEventType,
    M108_SCHEMA_VERSION,
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

#[test]
fn m108_headless_vertical_sequence_reduces_deterministically() {
    let types = [
        PreviewSyncEventType::IntentAccepted,
        PreviewSyncEventType::ContractValidated,
        PreviewSyncEventType::PlanRecorded,
        PreviewSyncEventType::CheckpointCreated,
        PreviewSyncEventType::SourceRevisionCommitted,
        PreviewSyncEventType::BuildRequested,
        PreviewSyncEventType::BuildObserved,
        PreviewSyncEventType::ArtifactObserved,
        PreviewSyncEventType::InstallRequested,
        PreviewSyncEventType::InstallObserved,
        PreviewSyncEventType::LaunchObserved,
        PreviewSyncEventType::InteractionObserved,
        PreviewSyncEventType::ObservationCaptured,
        PreviewSyncEventType::ValidationObserved,
        PreviewSyncEventType::PreviewPromoted,
    ];
    let mut state = nirman_preview::M108ProjectionState::new("p", "t", "preview-1");
    for (index, event_type) in types.into_iter().enumerate() {
        let event = PreviewSyncEvent {
            event_id: format!("event-{index}"),
            event_sequence: index as u64 + 1,
            project_id: "p".into(),
            task_id: "t".into(),
            correlation_id: "corr".into(),
            causation_id: Some(format!("cause-{index}")),
            candidate_preview_revision_id: "preview-1".into(),
            event_type: event_type.clone(),
            event_truth: if event_type == PreviewSyncEventType::PreviewPromoted {
                PreviewEventTruth::Verified
            } else if index >= 6 {
                PreviewEventTruth::Observed
            } else {
                PreviewEventTruth::Requested
            },
            project_revision_id: "source-1".into(),
            checkpoint_id: "checkpoint-1".into(),
            source_fingerprint: "sha256:source".into(),
            artifact_id: Some("apk-1".into()),
            artifact_fingerprint: Some("sha256:apk".into()),
            runtime_session_id: Some("runtime-1".into()),
            device_id: Some("emulator-1".into()),
            operation_ref: format!("operation-{index}"),
            observation_refs: vec![format!("observation-{index}")],
            evidence_refs: vec![format!("evidence-{index}")],
            validation_ref: (index >= 13).then(|| "validation-1".into()),
            payload: "headless authoritative fixture event".into(),
        };
        state.apply(&event).expect("event reduces");
    }
    assert_eq!(state.last_event_sequence, 15);
    assert_eq!(state.projection_revision, 15);
    assert_eq!(state.build_status, "OBSERVED");
    assert_eq!(state.install_status, "OBSERVED");
    assert_eq!(state.launch_status, "OBSERVED");
    assert_eq!(state.runtime_status, "OBSERVED");
    assert_eq!(state.validation_status, "OBSERVED");
    assert_eq!(state.evidence_ids.len(), 15);
    let gap = PreviewSyncEvent {
        event_id: "gap".into(),
        event_sequence: 17,
        project_id: "p".into(),
        task_id: "t".into(),
        correlation_id: "corr".into(),
        causation_id: None,
        candidate_preview_revision_id: "preview-1".into(),
        event_type: PreviewSyncEventType::StreamGap,
        event_truth: PreviewEventTruth::Stale,
        project_revision_id: "source-1".into(),
        checkpoint_id: "checkpoint-1".into(),
        source_fingerprint: "sha256:source".into(),
        artifact_id: None,
        artifact_fingerprint: None,
        runtime_session_id: None,
        device_id: None,
        operation_ref: "op-gap".into(),
        observation_refs: vec![],
        evidence_refs: vec![],
        validation_ref: None,
        payload: "gap".into(),
    };
    assert!(matches!(
        state.apply(&gap),
        Err(nirman_preview::M108ReducerError::SequenceGap { .. })
    ));
}

#[test]
fn m108_integrates_m9_observation_and_m10_artifact_records() {
    let profile = nirman_evidence::AndroidDeviceProfile {
        profile_id: "pixel-phone".into(),
        name: "Pixel phone".into(),
        api_level: 35,
        architecture: "x86_64".into(),
        width: 1080,
        height: 2400,
        density: 420,
        orientation: "portrait".into(),
        locale: "en-US".into(),
    };
    profile.validate().expect("M9 profile");
    let observation = nirman_evidence::AndroidDeviceObservation {
        schema_version: nirman_evidence::M9_SCHEMA_VERSION,
        observation_id: "m9-observation".into(),
        project_id: "p".into(),
        task_id: "t".into(),
        project_revision_id: "source-1".into(),
        device_profile_id: profile.profile_id,
        device_identity: "emulator-1".into(),
        runtime_session_id: "m9-session-m108-observation".into(),
        package_name: "com.example.app".into(),
        apk_sha256: "sha256:observed".into(),
        install_status: "OBSERVED_SUCCESS".into(),
        launch_status: "OBSERVED_SUCCESS".into(),
        interaction_status: "OBSERVED_PASS".into(),
        logcat_reference: Some("logcat-1".into()),
        screenshot_references: vec!["screenshot-1".into()],
        accessibility_reference: Some("a11y-1".into()),
        visual_comparison_reference: Some("visual-1".into()),
        permission_result_reference: Some("permissions-1".into()),
        crash_trace_reference: None,
        observed_at_epoch_seconds: 1,
        synthetic_data_only: true,
    };
    observation.validate().expect("M9 observation");
    let artifact = nirman_artifacts::ApkArtifact {
        schema_version: nirman_artifacts::M10_SCHEMA_VERSION,
        artifact_id: "m10-apk".into(),
        project_id: "p".into(),
        task_id: "t".into(),
        project_revision_id: "source-1".into(),
        source_fingerprint: "sha256:source".into(),
        source_provenance_ref: "android-build-observation:t:1".into(),
        path: "app-debug.apk".into(),
        sha256: "sha256:observed".into(),
        package_name: observation.package_name.clone(),
        inspection: Some(nirman_artifacts::ApkInspection {
            package_name: observation.package_name.clone(),
            version_code: "1".into(),
            version_name: "1.0".into(),
            aapt_output_sha256: "inspection-hash".into(),
        }),
        build_variant: "debug".into(),
        secret_scan_status: "PASS".into(),
        signing_status: "UNSIGNED_DEBUG".into(),
        delivery_status: "READY_LOCAL".into(),
        delivery_sha256: Some("sha256:observed".into()),
        delivery_verified: true,
        copy_uncertain: false,
    };
    nirman_artifacts::validate_apk_delivery(&artifact).expect("M10 artifact");
    let mut state = M108ProjectionState::new("p", "t", "preview-1");
    for (sequence, (event_type, truth, refs)) in [
        (
            PreviewSyncEventType::BuildObserved,
            PreviewEventTruth::Observed,
            vec!["m4-build".into()],
        ),
        (
            PreviewSyncEventType::ArtifactObserved,
            PreviewEventTruth::Observed,
            vec![artifact.artifact_id.clone()],
        ),
        (
            PreviewSyncEventType::InstallObserved,
            PreviewEventTruth::Observed,
            vec![observation.observation_id.clone()],
        ),
        (
            PreviewSyncEventType::LaunchObserved,
            PreviewEventTruth::Observed,
            vec!["launch-1".into()],
        ),
        (
            PreviewSyncEventType::ObservationCaptured,
            PreviewEventTruth::Observed,
            observation.screenshot_references.clone(),
        ),
        (
            PreviewSyncEventType::ValidationObserved,
            PreviewEventTruth::Verified,
            vec!["validation-1".into()],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let event = PreviewSyncEvent {
            event_id: format!("integrated-{sequence}"),
            event_sequence: sequence as u64 + 1,
            project_id: "p".into(),
            task_id: "t".into(),
            correlation_id: "corr".into(),
            causation_id: None,
            candidate_preview_revision_id: "preview-1".into(),
            event_type,
            event_truth: truth,
            project_revision_id: "source-1".into(),
            checkpoint_id: "checkpoint-1".into(),
            source_fingerprint: "sha256:source".into(),
            artifact_id: Some(artifact.artifact_id.clone()),
            artifact_fingerprint: Some(artifact.sha256.clone()),
            runtime_session_id: Some("runtime-1".into()),
            device_id: Some(observation.device_identity.clone()),
            operation_ref: format!("op-{sequence}"),
            observation_refs: vec![observation.observation_id.clone()],
            evidence_refs: refs,
            validation_ref: Some("validation-1".into()),
            payload: "M4/M9/M10 integrated headless observation".into(),
        };
        state.apply(&event).expect("integrated event");
    }
    assert_eq!(state.build_status, "OBSERVED");
    assert_eq!(state.install_status, "OBSERVED");
    assert_eq!(state.launch_status, "OBSERVED");
    assert_eq!(state.runtime_status, "OBSERVED");
    assert_eq!(state.validation_status, "OBSERVED");
    assert!(state.evidence_ids.contains(&"m10-apk".into()));
    assert!(state.evidence_ids.contains(&"screenshot-1".into()));
}

#[test]
fn m108_rejects_simulated_runtime_and_premature_promotion() {
    let mut state = M108ProjectionState::new("p", "t", "preview-1");
    let mut build = PreviewSyncEvent {
        event_id: "event-simulated-build".into(),
        event_sequence: 1,
        project_id: "p".into(),
        task_id: "t".into(),
        correlation_id: "corr".into(),
        causation_id: None,
        candidate_preview_revision_id: "preview-1".into(),
        event_type: PreviewSyncEventType::BuildObserved,
        event_truth: PreviewEventTruth::Simulated,
        project_revision_id: "source-1".into(),
        checkpoint_id: "checkpoint-1".into(),
        source_fingerprint: "sha256:source".into(),
        artifact_id: Some("apk-1".into()),
        artifact_fingerprint: Some("sha256:apk".into()),
        runtime_session_id: None,
        device_id: None,
        operation_ref: "operation-build".into(),
        observation_refs: vec!["observation-build".into()],
        evidence_refs: vec!["evidence-build".into()],
        validation_ref: None,
        payload: "model-only build claim".into(),
    };
    assert_eq!(
        state.apply(&build),
        Err(nirman_preview::M108ReducerError::EvidenceRequired)
    );
    build.event_truth = PreviewEventTruth::Observed;
    state.apply(&build).expect("observed build");

    let promotion = PreviewSyncEvent {
        event_id: "event-premature-promotion".into(),
        event_sequence: 2,
        project_id: "p".into(),
        task_id: "t".into(),
        correlation_id: "corr".into(),
        causation_id: Some("operation-promote".into()),
        candidate_preview_revision_id: "preview-1".into(),
        event_type: PreviewSyncEventType::PreviewPromoted,
        event_truth: PreviewEventTruth::Observed,
        project_revision_id: "source-1".into(),
        checkpoint_id: "checkpoint-1".into(),
        source_fingerprint: "sha256:source".into(),
        artifact_id: Some("apk-1".into()),
        artifact_fingerprint: Some("sha256:apk".into()),
        runtime_session_id: Some("runtime-1".into()),
        device_id: Some("emulator-1".into()),
        operation_ref: "operation-promote".into(),
        observation_refs: vec!["observation-promote".into()],
        evidence_refs: vec!["evidence-promote".into()],
        validation_ref: None,
        payload: "premature promotion claim".into(),
    };
    assert_eq!(
        state.apply(&promotion),
        Err(nirman_preview::M108ReducerError::EvidenceRequired)
    );
}

fn m109_event(sequence: u64, event_id: &str, event_type: PreviewSyncEventType) -> PreviewSyncEvent {
    PreviewSyncEvent {
        event_id: event_id.into(),
        event_sequence: sequence,
        project_id: "p".into(),
        task_id: "t".into(),
        correlation_id: "corr".into(),
        causation_id: Some(format!("cause-{sequence}")),
        candidate_preview_revision_id: "preview-1".into(),
        event_type,
        event_truth: PreviewEventTruth::Requested,
        project_revision_id: "source-1".into(),
        checkpoint_id: "checkpoint-1".into(),
        source_fingerprint: "sha256:source".into(),
        artifact_id: None,
        artifact_fingerprint: None,
        runtime_session_id: None,
        device_id: None,
        operation_ref: format!("operation-{sequence}"),
        observation_refs: vec![],
        evidence_refs: vec![],
        validation_ref: None,
        payload: format!("payload-{sequence}"),
    }
}

#[test]
fn m109_duplicate_replay_is_idempotent_and_conflicting_stale_delivery_is_quarantined() {
    let mut state = M108ProjectionState::new("p", "t", "preview-1");
    let event = m109_event(1, "event-1", PreviewSyncEventType::IntentAccepted);
    state.apply(&event).expect("initial event");
    let before = state.clone();
    state.apply(&event).expect("same duplicate replay");
    assert_eq!(state, before);

    let mut conflicting = event.clone();
    conflicting.payload = "conflicting-old-payload".into();
    assert_eq!(
        state.apply(&conflicting),
        Err(nirman_preview::M108ReducerError::ConflictingDuplicate)
    );
    assert_eq!(state.quarantined_event_ids, vec!["event-1"]);
    assert_eq!(state.last_event_sequence, 1);
}

#[test]
fn m109_gap_is_held_and_replay_then_reconnect_restores_continuity() {
    let mut state = M108ProjectionState::new("p", "t", "preview-1");
    state
        .apply(&m109_event(
            1,
            "event-1",
            PreviewSyncEventType::IntentAccepted,
        ))
        .expect("first event");
    let gap = m109_event(3, "event-3", PreviewSyncEventType::PlanRecorded);
    assert!(matches!(
        state.apply(&gap),
        Err(nirman_preview::M108ReducerError::SequenceGap {
            expected: 2,
            received: 3
        })
    ));
    assert_eq!(state.pending_event_sequences, vec![3]);
    assert_eq!(state.stream_status, "GAP_BLOCKED");
    state
        .apply(&m109_event(
            2,
            "event-2",
            PreviewSyncEventType::StreamReconnected,
        ))
        .expect("reconnect event during replay");
    state
        .apply(&m109_event(
            3,
            "event-3",
            PreviewSyncEventType::PlanRecorded,
        ))
        .expect("pending event after replay");
    state.complete_replay().expect("replay complete");
    assert_eq!(state.stream_status, "CONNECTED");
    assert_eq!(state.last_event_sequence, 3);
}

#[test]
fn m109_stream_loss_freezes_projection_and_late_wrong_device_event_is_stale() {
    let mut state = M108ProjectionState::new("p", "t", "preview-1");
    state
        .apply(&m109_event(
            1,
            "event-1",
            PreviewSyncEventType::IntentAccepted,
        ))
        .expect("initial event");
    state.mark_stream_lost();
    assert_eq!(state.stream_status, "STALE_STREAM");
    assert_eq!(
        state.apply(&m109_event(
            2,
            "event-2",
            PreviewSyncEventType::PlanRecorded
        )),
        Err(nirman_preview::M108ReducerError::StaleEvent)
    );

    let mut reconnect = m109_event(
        2,
        "event-reconnect",
        PreviewSyncEventType::StreamReconnected,
    );
    reconnect.device_id = Some("device-a".into());
    state.apply(&reconnect).expect("reconnect");
    state.complete_replay().expect("complete reconnect replay");
    let mut late = m109_event(3, "event-late", PreviewSyncEventType::LaunchObserved);
    late.event_truth = PreviewEventTruth::Observed;
    late.device_id = Some("device-b".into());
    late.evidence_refs = vec!["evidence-late".into()];
    assert_eq!(
        state.apply(&late),
        Err(nirman_preview::M108ReducerError::StaleEvent)
    );
}

#[test]
fn m109_equal_cursor_snapshot_requires_consistent_projection_dimensions() {
    let mut state = M108ProjectionState::new("p", "t", "preview-1");
    state
        .apply(&m109_event(
            1,
            "event-1",
            PreviewSyncEventType::IntentAccepted,
        ))
        .expect("initial event");
    let mut consistent = state.clone();
    consistent.stream_status = "REPLAYING".into();
    state
        .accept_equal_cursor_snapshot(&consistent)
        .expect("consistent equal cursor enrichment");
    let mut contradictory = consistent;
    contradictory.build_status = "OBSERVED".into();
    assert_eq!(
        state.accept_equal_cursor_snapshot(&contradictory),
        Err(nirman_preview::M108ReducerError::ContradictorySnapshot)
    );
}

#[test]
fn m109_evidence_record_requires_complete_durable_lineage() {
    let valid = nirman_preview::PreviewSyncEvidenceRecord {
        evidence_id: "evidence-1".into(),
        project_id: "p".into(),
        task_id: "t".into(),
        event_sequence_start: 1,
        event_sequence_end: 2,
        projection_revision: 2,
        preview_revision_id: "preview-1".into(),
        project_revision_id: "source-1".into(),
        checkpoint_id: "checkpoint-1".into(),
        branch_id: Some("main".into()),
        artifact_fingerprint: Some("sha256:apk".into()),
        device_id: Some("device-1".into()),
        runtime_session_id: Some("runtime-1".into()),
        state_fingerprints: [("runtime".into(), "running".into())].into_iter().collect(),
        event_ids: vec!["event-1".into(), "event-2".into()],
        observation_refs: vec!["observation-1".into()],
        evidence_refs: vec!["evidence-source-1".into()],
        validation_refs: vec!["validation-1".into()],
        invalidated_evidence_ids: vec![],
        recovery_event_ids: vec![],
        promotion_record_ref: Some("promotion-1".into()),
        certification_decision_ref: Some("certification-1".into()),
        completion_decision_ref: None,
        truth: PreviewEventTruth::Verified,
        captured_at_epoch_seconds: 10,
    };
    valid.validate().expect("complete evidence record");
    let mut invalid = valid;
    invalid.event_ids.clear();
    assert!(invalid.validate().is_err());
}
