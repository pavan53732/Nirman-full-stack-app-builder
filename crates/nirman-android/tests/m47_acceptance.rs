use nirman_android::{
    infer_android_requirement_manifest, validate_android_workspace, AndroidRepairRegistry,
    AndroidRequirementKind, AndroidRequirementStatus, RepairFailureFingerprint, RepairPattern,
};
use nirman_domain::{
    AndroidConstructionContract, AndroidDeviceProfile, AndroidTechnologyPlan, ArtifactKind,
    ArtifactModel, ConstructionRequirement, ProjectId, RequirementOrigin, Revision, TaskId,
    ValidationModel,
};
use nirman_project::{IndexRequest, ProjectIndexer};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn contract(project_id: &str, task_id: &str) -> AndroidConstructionContract {
    AndroidConstructionContract {
        schema_version: 1,
        contract_id: "m47-contract".into(),
        project_id: ProjectId(project_id.into()),
        target_platforms: vec!["android".into()],
        task_id: TaskId(task_id.into()),
        user_intent: "Build an offline Android notes application".into(),
        screenshots: vec![],
        assets: vec![],
        features: vec![ConstructionRequirement {
            requirement_id: "notes".into(),
            statement: "create and edit notes".into(),
            origin: RequirementOrigin::UserFact,
            source_reference_ids: vec![],
        }],
        ui: vec![ConstructionRequirement {
            requirement_id: "editor".into(),
            statement: "show an editor".into(),
            origin: RequirementOrigin::UserFact,
            source_reference_ids: vec![],
        }],
        data: vec![],
        integrations: vec![],
        technology_plan: AndroidTechnologyPlan {
            plan_id: "plan-m47".into(),
            task_id: TaskId(task_id.into()),
            requested_capabilities: vec!["offline-storage".into()],
            visual_requirements: vec![],
            selected_languages: vec!["kotlin".into()],
            selected_ui_frameworks: vec!["jetpack-compose".into()],
            selected_runtime_layers: vec![],
            selected_native_modules: vec![],
            selected_build_plugins: vec![],
            selected_device_apis: vec![],
            selected_libraries: vec![],
            compatibility_constraints: vec![],
            rejected_alternatives: vec![],
            required_toolchains: vec!["jdk".into(), "gradle".into(), "android-sdk".into()],
            validation_plan: vec!["compile".into()],
            confidence: None,
            revision: Revision(1),
        },
        android_requirements: vec![],
        device_matrix: vec![AndroidDeviceProfile {
            device_id: "pixel".into(),
            name: "Pixel".into(),
            platform_version: "Android".into(),
            api_level: 35,
            architecture: "x86_64".into(),
            width: 1080,
            height: 2400,
            density: 420,
            orientation: "portrait".into(),
            locale: "en-US".into(),
            permissions: vec!["android.permission.INTERNET".into()],
            network_profile: "offline".into(),
        }],
        validation_model: ValidationModel {
            required_checks: vec!["compile".into()],
            acceptance_criteria: vec!["notes can be edited".into()],
        },
        artifact_model: ArtifactModel {
            required_artifact: ArtifactKind::Apk,
            aab_declared: false,
        },
    }
}

fn fixture_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("nirman-m47-{label}-{nonce}"))
}

#[test]
fn m47_acceptance_is_file_backed_deterministic_and_observation_derived() {
    let root = fixture_root("requirements");
    let manifest_path = root.join("app/src/main/AndroidManifest.xml");
    fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("fixture");
    fs::write(
        &manifest_path,
        r#"<manifest package="com.example.notes" xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <application android:label="Notes" />
</manifest>"#,
    )
    .expect("manifest");
    fs::write(
        root.join("app/build.gradle.kts"),
        "plugins { id(\"com.android.application\") }\nandroid { namespace = \"com.example.notes\" }\n",
    )
    .expect("gradle");
    fs::write(root.join("README.md"), "fixture metadata").expect("metadata");
    fs::create_dir_all(root.join("app/src/main/res/values")).expect("resource directory");
    fs::write(
        root.join("app/src/main/res/values/strings.xml"),
        "<resources><string name=\"app_name\">Notes</string></resources>",
    )
    .expect("resource");
    let index = ProjectIndexer::default()
        .index_workspace(&root, &IndexRequest::default())
        .expect("M45 index");
    let valid_contract = contract("project-m47", "task-m47");
    let manifest =
        infer_android_requirement_manifest(&valid_contract, &index, 7).expect("M47 manifest");
    let workspace_validation =
        validate_android_workspace(&root, &index).expect("workspace validation");
    assert!(workspace_validation.manifest_present);
    assert!(workspace_validation.manifest_well_formed);
    assert!(workspace_validation.invalid_resource_files.is_empty());
    assert!(workspace_validation
        .resource_files
        .iter()
        .any(|path| path.ends_with("strings.xml")));
    manifest.validate().expect("valid requirement manifest");
    let manifest_json = serde_json::to_string(&manifest).expect("manifest JSON");
    let reloaded: nirman_android::AndroidRequirementManifest =
        serde_json::from_str(&manifest_json).expect("manifest reload");
    assert_eq!(manifest, reloaded);
    assert!(manifest
        .requirements
        .iter()
        .any(|item| item.requirement_id == "android.manifest.present"
            && item.status == AndroidRequirementStatus::Satisfied));
    assert!(manifest.requirements.iter().any(|item| {
        item.subject == "android.permission.INTERNET"
            && item.status == AndroidRequirementStatus::Satisfied
    }));
    assert!(manifest
        .requirements
        .iter()
        .any(|item| item.kind == AndroidRequirementKind::Sdk));
    for kind in [
        AndroidRequirementKind::Sdk,
        AndroidRequirementKind::Abi,
        AndroidRequirementKind::Manifest,
        AndroidRequirementKind::Permission,
        AndroidRequirementKind::Service,
        AndroidRequirementKind::Resource,
        AndroidRequirementKind::Accessibility,
        AndroidRequirementKind::Localization,
        AndroidRequirementKind::BackgroundBehavior,
        AndroidRequirementKind::Release,
    ] {
        assert!(manifest.requirements.iter().any(|item| item.kind == kind));
    }

    let mut excessive_contract = valid_contract.clone();
    excessive_contract.device_matrix[0].permissions.clear();
    let excessive = infer_android_requirement_manifest(&excessive_contract, &index, 7)
        .expect("excessive permission manifest");
    assert!(excessive.requirements.iter().any(|item| {
        item.subject == "android.permission.INTERNET"
            && item.status == AndroidRequirementStatus::Excessive
    }));
    let mut invalid_target = valid_contract.clone();
    invalid_target.target_platforms = vec!["web".into()];
    assert!(infer_android_requirement_manifest(&invalid_target, &index, 7).is_err());

    let missing_root = fixture_root("missing-manifest");
    fs::create_dir_all(&missing_root).expect("missing fixture");
    let missing_index = ProjectIndexer::default()
        .index_workspace(&missing_root, &IndexRequest::default())
        .expect("missing index");
    let missing = infer_android_requirement_manifest(
        &contract("project-m47-missing", "task-m47-missing"),
        &missing_index,
        8,
    )
    .expect("missing manifest is still a valid requirement set");
    assert!(missing.requirements.iter().any(|item| {
        item.requirement_id == "android.manifest.present"
            && item.status == AndroidRequirementStatus::Missing
    }));

    let registry = AndroidRepairRegistry::default();
    registry.validate().expect("default registry");
    assert!(registry.patterns.len() >= 10);
    let selection = registry
        .select(&RepairFailureFingerprint {
            classifier: "dependency.conflict".into(),
            detail: "incompatible dependency versions".into(),
        })
        .expect("deterministic repair selection");
    assert_eq!(selection.pattern_id, "repair.dependency.conflict");
    assert_eq!(selection.retry_budget, 3);
    assert!(selection
        .preconditions
        .iter()
        .any(|item| item == "active-checkpoint"));
    assert!(registry
        .select(&RepairFailureFingerprint {
            classifier: "unregistered.failure".into(),
            detail: "unknown".into(),
        })
        .is_err());
    let mut invalid_registry = registry.clone();
    invalid_registry.patterns[0] = RepairPattern {
        retry_budget: 0,
        ..invalid_registry.patterns[0].clone()
    };
    assert!(invalid_registry.validate().is_err());
    let mut invalid_manifest = manifest.clone();
    invalid_manifest.schema_version = 99;
    assert!(invalid_manifest.validate().is_err());

    let evidence = serde_json::json!({
        "schema": "nirman.m47.requirements-repair.v1",
        "fileBackedIndexObserved": true,
        "androidOnlyContractObserved": true,
        "manifestPresentSatisfiedObserved": manifest.requirements.iter().any(|item| item.requirement_id == "android.manifest.present" && item.status == AndroidRequirementStatus::Satisfied),
        "permissionSatisfiedObserved": manifest.requirements.iter().any(|item| item.subject == "android.permission.INTERNET" && item.status == AndroidRequirementStatus::Satisfied),
        "excessivePermissionObserved": excessive.requirements.iter().any(|item| item.subject == "android.permission.INTERNET" && item.status == AndroidRequirementStatus::Excessive),
        "manifestResourceValidationObserved": workspace_validation.manifest_well_formed && workspace_validation.invalid_resource_files.is_empty(),
        "missingManifestObserved": missing.requirements.iter().any(|item| item.status == AndroidRequirementStatus::Missing),
        "deterministicSerializationReloadObserved": manifest == reloaded,
        "repairFamiliesObserved": registry.patterns.len() >= 10,
        "allowedRepairSelectionObserved": selection.pattern_id == "repair.dependency.conflict",
        "retryBudgetObserved": selection.retry_budget == 3,
        "checkpointRuleObserved": selection.checkpoint_rule == "restore-before-repair-and-revalidate",
        "unknownFailureRejectedObserved": true,
        "androidWorkspaceMutation": false,
        "androidBuildObserved": false,
        "androidDeviceObserved": false,
        "nativeWindowsTauriRuntimeObserved": false,
        "m47Status": "M47_HEADLESS_REQUIREMENT_REPAIR_TRACE_ONLY"
    });
    let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/evidence/m47_requirements_repair.json");
    fs::create_dir_all(evidence_path.parent().expect("evidence directory")).expect("evidence dir");
    fs::write(
        evidence_path,
        serde_json::to_vec_pretty(&evidence).expect("evidence JSON"),
    )
    .expect("evidence");
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(missing_root);
}
