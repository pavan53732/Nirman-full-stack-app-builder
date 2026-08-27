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
            event_type,
            event_truth: if index >= 6 {
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
        package_name: "com.example.app".into(),
        install_status: "OBSERVED_SUCCESS".into(),
        launch_status: "OBSERVED_SUCCESS".into(),
        interaction_status: "OBSERVED_PASS".into(),
        logcat_reference: Some("logcat-1".into()),
        screenshot_references: vec!["screenshot-1".into()],
        accessibility_reference: Some("a11y-1".into()),
        visual_comparison_reference: Some("visual-1".into()),
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
        path: "app-debug.apk".into(),
        sha256: "sha256:observed".into(),
        package_name: observation.package_name.clone(),
        build_variant: "debug".into(),
        secret_scan_status: "PASS".into(),
        signing_status: "UNSIGNED_DEBUG".into(),
        delivery_status: "READY_LOCAL".into(),
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
