use nirman_domain::{
    AndroidConstructionContract, AndroidConstructionContractError, AndroidDeviceProfile,
    AndroidTechnologyPlan, ArtifactKind, ArtifactModel, AssetReferenceInput,
    ConstructionRequirement, ProjectId, RequirementOrigin, Revision, TaskId, ValidationModel,
    VisualReferenceInput,
};

fn valid_contract() -> AndroidConstructionContract {
    let task_id = TaskId("task-m39".into());
    AndroidConstructionContract {
        schema_version: 1,
        contract_id: "contract-m39".into(),
        project_id: ProjectId("project-m39".into()),
        target_platforms: vec!["android".into()],
        task_id: task_id.clone(),
        user_intent: "Build an offline-first notes application".into(),
        screenshots: vec![VisualReferenceInput {
            reference_id: "screen-reference-1".into(),
            source_path: "inputs/notes.png".into(),
            image_hash: "sha256:screen".into(),
        }],
        assets: vec![AssetReferenceInput {
            asset_id: "asset-logo-1".into(),
            source_path: "inputs/logo.svg".into(),
            content_hash: "sha256:asset".into(),
        }],
        features: vec![ConstructionRequirement {
            requirement_id: "feature-offline".into(),
            statement: "Notes remain readable without network access".into(),
            origin: RequirementOrigin::UserFact,
            source_reference_ids: vec![],
        }],
        ui: vec![ConstructionRequirement {
            requirement_id: "ui-calm".into(),
            statement: "Use a calm dark visual hierarchy".into(),
            origin: RequirementOrigin::ModelProposal,
            source_reference_ids: vec!["screen-reference-1".into()],
        }],
        data: vec![ConstructionRequirement {
            requirement_id: "data-local".into(),
            statement: "Persist notes locally".into(),
            origin: RequirementOrigin::UserFact,
            source_reference_ids: vec![],
        }],
        integrations: vec![],
        technology_plan: AndroidTechnologyPlan {
            plan_id: "plan-m39".into(),
            task_id,
            requested_capabilities: vec!["offline-storage".into(), "accessible-ui".into()],
            visual_requirements: vec!["dark-theme".into()],
            selected_languages: vec!["kotlin".into()],
            selected_ui_frameworks: vec!["jetpack-compose".into()],
            selected_runtime_layers: vec![],
            selected_native_modules: vec![],
            selected_build_plugins: vec!["android-gradle-plugin".into()],
            selected_device_apis: vec![],
            selected_libraries: vec!["room".into()],
            compatibility_constraints: vec!["android-api-29-plus".into()],
            rejected_alternatives: vec!["web-target".into()],
            required_toolchains: vec!["jdk".into(), "gradle".into(), "android-sdk".into()],
            validation_plan: vec!["unit-tests".into(), "android-build".into()],
            confidence: Some("high".into()),
            revision: Revision(1),
        },
        android_requirements: vec![ConstructionRequirement {
            requirement_id: "android-min-api".into(),
            statement: "Support the declared Android API range".into(),
            origin: RequirementOrigin::UserFact,
            source_reference_ids: vec![],
        }],
        device_matrix: vec![AndroidDeviceProfile {
            device_id: "pixel-api-35".into(),
            name: "Pixel API 35".into(),
            platform_version: "Android 15".into(),
            api_level: 35,
            architecture: "x86_64".into(),
            width: 1080,
            height: 2400,
            density: 420,
            orientation: "portrait".into(),
            locale: "en-US".into(),
            permissions: vec![],
            network_profile: "offline-capable".into(),
        }],
        validation_model: ValidationModel {
            required_checks: vec!["compile".into(), "unit-tests".into()],
            acceptance_criteria: vec!["notes persist offline".into()],
        },
        artifact_model: ArtifactModel {
            required_artifact: ArtifactKind::Apk,
            aab_declared: false,
        },
    }
}

#[test]
fn valid_android_construction_contract_round_trips_deterministically() {
    let contract = valid_contract();
    contract.validate().expect("valid contract");
    let json = serde_json::to_string(&contract).expect("serialize");
    assert!(json.contains("\"targetPlatforms\":[\"android\"]"));
    assert_eq!(
        AndroidConstructionContract::from_json(&json).expect("parse"),
        contract
    );
    assert_eq!(
        AndroidConstructionContract::migrated_json(&json).expect("migration"),
        json
    );
}

#[test]
fn target_platforms_are_exactly_android_and_reject_non_android_targets() {
    let mut contract = valid_contract();
    contract.target_platforms = vec!["android".into(), "windows".into()];
    assert_eq!(
        contract.validate(),
        Err(AndroidConstructionContractError::InvalidTargetPlatforms)
    );
    contract.target_platforms.clear();
    assert_eq!(
        contract.validate(),
        Err(AndroidConstructionContractError::InvalidTargetPlatforms)
    );
}

#[test]
fn required_fields_and_source_attribution_are_enforced() {
    let mut contract = valid_contract();
    contract.user_intent.clear();
    assert_eq!(
        contract.validate(),
        Err(AndroidConstructionContractError::EmptyField("userIntent"))
    );

    let mut contract = valid_contract();
    contract.ui[0].source_reference_ids.clear();
    assert_eq!(
        contract.validate(),
        Err(AndroidConstructionContractError::ProposalMissingSource)
    );

    let mut contract = valid_contract();
    contract.ui[0].source_reference_ids = vec!["missing-reference".into()];
    assert_eq!(
        contract.validate(),
        Err(AndroidConstructionContractError::InvalidReference)
    );
}

#[test]
fn unknown_fields_and_unsupported_versions_are_rejected_deterministically() {
    let contract = valid_contract();
    let mut value = serde_json::to_value(contract).expect("value");
    value["unexpectedField"] = serde_json::json!(true);
    let malformed = serde_json::to_string(&value).expect("malformed json");
    assert_eq!(
        AndroidConstructionContract::from_json(&malformed),
        Err(AndroidConstructionContractError::InvalidJson)
    );

    let mut versioned = valid_contract();
    versioned.schema_version = 99;
    let first = versioned.validate();
    let second = versioned.validate();
    assert_eq!(first, second);
    assert_eq!(
        first,
        Err(AndroidConstructionContractError::UnsupportedSchemaVersion)
    );
}

#[test]
fn m4_plan_and_noop_edit_events_preserve_android_intent_identity() {
    let contract = valid_contract();
    contract.validate().expect("valid Android intent");
    assert_eq!(contract.target_platforms, vec!["android"]);
    assert_eq!(contract.technology_plan.task_id, contract.task_id);

    let plan_payload = serde_json::to_string(&contract.technology_plan).expect("plan payload");
    let plan_event = nirman_domain::ControlEvent {
        event_id: "m4-domain-plan-event".into(),
        sequence: 1,
        project_id: contract.project_id.clone(),
        task_id: Some(contract.task_id.clone()),
        kind: "AndroidSynthesisBuild".into(),
        payload: plan_payload.clone(),
        source_revision: Revision(1),
    };
    let restored_plan: AndroidTechnologyPlan =
        serde_json::from_str(&plan_event.payload).expect("plan event payload");
    assert_eq!(restored_plan, contract.technology_plan);
    assert_eq!(plan_event.task_id, Some(contract.task_id.clone()));
    assert_eq!(plan_event.source_revision, Revision(1));

    let noop_event = nirman_domain::ControlEvent {
        event_id: "m4-domain-noop-edit".into(),
        sequence: 2,
        project_id: contract.project_id,
        task_id: Some(contract.task_id),
        kind: "WorkspaceApplyPatch".into(),
        payload: serde_json::json!({
            "operation": "NO_OP",
            "planPayload": plan_payload,
            "changedPaths": []
        })
        .to_string(),
        source_revision: Revision(2),
    };
    let noop: serde_json::Value = serde_json::from_str(&noop_event.payload).expect("no-op event");
    assert_eq!(noop["operation"], "NO_OP");
    assert_eq!(noop["changedPaths"], serde_json::json!([]));
    assert_eq!(noop_event.sequence, 2);
}
