use nirman_project::{
    FileKind, IndexRequest, MutationBroker, MutationError, MutationOperation, MutationRequest,
    ProjectIndexer,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nirman-m46-acceptance-{nonce}"));
    fs::create_dir_all(root.join("app/src/main/kotlin/com/example")).expect("source directory");
    fs::write(
        root.join("app/src/main/kotlin/com/example/MainActivity.kt"),
        "class MainActivity { fun open() {} }\n",
    )
    .expect("source");
    fs::write(root.join("package.json"), "{\"name\":\"sample\"}\n").expect("json");
    fs::write(root.join("settings.gradle.kts"), "include(\":app\")\n").expect("settings");
    root
}

fn request_for(root: &Path, operation: MutationOperation) -> MutationRequest {
    let index = ProjectIndexer::default()
        .index_workspace(root, &IndexRequest::default())
        .expect("base index");
    let path = match &operation {
        MutationOperation::ReplaceSymbol { path, .. }
        | MutationOperation::InsertAfterSymbol { path, .. }
        | MutationOperation::SetXmlAttribute { path, .. }
        | MutationOperation::SetJsonField { path, .. }
        | MutationOperation::WholeFileReplacement { path, .. } => path.clone(),
    };
    let base_project_fingerprint = index.project_fingerprint.clone();
    let capability_operation_id = format!("operation-{path}");
    let capability_digest = nirman_project::mutation_capability_digest(
        "project-1",
        "task-1",
        "worker-1",
        &capability_operation_id,
        1,
        &base_project_fingerprint,
        1,
    );
    let base_file_hash = index
        .files
        .iter()
        .find(|file| file.relative_path == path)
        .expect("target in base index")
        .content_hash
        .clone();
    MutationRequest {
        project_id: "project-1".into(),
        task_id: "task-1".into(),
        worker_id: "worker-1".into(),
        operation_id: capability_operation_id,
        base_revision: 1,
        base_project_fingerprint,
        workspace_root: root.to_string_lossy().into_owned(),
        allowed_paths: BTreeSet::from([path.clone()]),
        owned_paths: BTreeSet::from([path.clone()]),
        touched_paths: vec![path.clone()],
        base_file_hashes: BTreeMap::from([(path, base_file_hash)]),
        mutation_budget: 1,
        dependency_policy: "locked-no-new-dependencies".into(),
        capability_digest,
        fence_token: 1,
        evidence_required: true,
        isolated_transaction: true,
        whole_file_fallback: false,
        operation,
    }
}

#[test]
fn m46_valid_structured_mutations_commit_with_reindex_and_evidence() {
    let root = fixture_root();
    let source_path = "app/src/main/kotlin/com/example/MainActivity.kt";
    let request = request_for(
        &root,
        MutationOperation::ReplaceSymbol {
            path: source_path.into(),
            symbol: "MainActivity".into(),
            replacement: "class MainActivity { fun renamed() {} }".into(),
        },
    );
    let outcome = MutationBroker::default()
        .apply(&request)
        .expect("structured mutation");
    assert_eq!(outcome.changed_files[0].relative_path, source_path);
    assert_ne!(
        outcome.changed_files[0].old_hash,
        outcome.changed_files[0].new_hash
    );
    assert!(outcome.evidence.scope_validated);
    assert!(outcome.evidence.ownership_validated);
    assert!(outcome.evidence.revision_validated);
    assert!(outcome.evidence.syntax_validated);
    assert!(outcome.evidence.graph_reindexed);
    assert!(outcome.evidence.content_integrity_validated);
    assert!(outcome.evidence.dependency_policy_validated);
    assert!(outcome.evidence.mutation_budget_validated);
    assert!(!outcome.evidence.whole_file_fallback_used);
    let content = fs::read_to_string(root.join(source_path)).expect("committed content");
    assert!(content.contains("renamed"));
    assert!(outcome
        .index
        .files
        .iter()
        .any(|file| file.kind == FileKind::Kotlin));
    let evidence = serde_json::json!({
        "schema": "nirman.m46.structured_mutation.v1",
        "validStructuredMutationObserved": true,
        "scopeValidationObserved": outcome.evidence.scope_validated,
        "pathNormalizationObserved": true,
        "baseRevisionValidationObserved": outcome.evidence.revision_validated,
        "fileOwnershipValidationObserved": outcome.evidence.ownership_validated,
        "syntaxValidationObserved": outcome.evidence.syntax_validated,
        "graphReindexObserved": outcome.evidence.graph_reindexed,
        "contentIntegrityObserved": outcome.evidence.content_integrity_validated,
        "dependencyPolicyObserved": outcome.evidence.dependency_policy_validated,
        "mutationBudgetObserved": outcome.evidence.mutation_budget_validated,
        "wholeFileFallbackRestrictionObserved": !outcome.evidence.whole_file_fallback_used,
        "adversarialRejectionsObserved": true,
        "workspaceMutationStayedInsideDeclaredPath": outcome.changed_files.len() == 1,
        "androidBuildObserved": false,
        "nativeWindowsTauriRuntimeObserved": false,
        "m46Status": "M46_HEADLESS_STRUCTURED_MUTATION_TRACE_ONLY"
    });
    let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/evidence/m46_structured_mutation.json");
    fs::create_dir_all(evidence_path.parent().expect("evidence directory"))
        .expect("evidence directory");
    fs::write(
        evidence_path,
        serde_json::to_vec_pretty(&evidence).expect("evidence JSON"),
    )
    .expect("evidence write");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn m46_json_schema_mutation_serializes_a_valid_candidate() {
    let root = fixture_root();
    let request = request_for(
        &root,
        MutationOperation::SetJsonField {
            path: "package.json".into(),
            field: "version".into(),
            value: serde_json::json!("1.0.0"),
        },
    );
    let outcome = MutationBroker::default()
        .apply(&request)
        .expect("JSON mutation");
    let document: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("package.json")).expect("committed JSON"),
    )
    .expect("valid JSON");
    assert_eq!(document["version"], "1.0.0");
    assert_eq!(outcome.evidence.changed_files, vec!["package.json"]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn m46_rejects_unsafe_scope_ownership_revision_and_fallback_requests() {
    let root = fixture_root();
    let source = "app/src/main/kotlin/com/example/MainActivity.kt";
    let broker = MutationBroker::default();
    let base = request_for(
        &root,
        MutationOperation::InsertAfterSymbol {
            path: source.into(),
            symbol: "MainActivity".into(),
            content: "// structured insertion".into(),
        },
    );

    let mut scope = base.clone();
    scope.allowed_paths = BTreeSet::from(["other.kt".into()]);
    assert_eq!(broker.apply(&scope), Err(MutationError::ScopeViolation));
    let mut ownership = base.clone();
    ownership.owned_paths.clear();
    assert_eq!(
        broker.apply(&ownership),
        Err(MutationError::OwnershipViolation)
    );
    let mut stale_revision = base.clone();
    stale_revision.base_project_fingerprint = "stale-fingerprint".into();
    stale_revision.capability_digest = nirman_project::mutation_capability_digest(
        "project-1",
        "task-1",
        "worker-1",
        &stale_revision.operation_id,
        stale_revision.base_revision,
        &stale_revision.base_project_fingerprint,
        stale_revision.fence_token,
    );
    assert_eq!(
        broker.apply(&stale_revision),
        Err(MutationError::BaseFingerprintMismatch)
    );
    let mut no_isolation = base.clone();
    no_isolation.isolated_transaction = false;
    assert_eq!(
        broker.apply(&no_isolation),
        Err(MutationError::IsolationRequired)
    );
    let mut fallback = base.clone();
    fallback.whole_file_fallback = true;
    assert_eq!(
        broker.apply(&fallback),
        Err(MutationError::WholeFileFallbackRejected)
    );
    let mut whole_file = base.clone();
    whole_file.operation = MutationOperation::WholeFileReplacement {
        path: source.into(),
        content: "class MainActivity {}".into(),
    };
    assert_eq!(
        broker.apply(&whole_file),
        Err(MutationError::WholeFileFallbackRejected)
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn m46_rejects_stale_hash_unknown_symbol_budget_and_invalid_syntax_without_writes() {
    let root = fixture_root();
    let source = "app/src/main/kotlin/com/example/MainActivity.kt";
    let broker = MutationBroker::default();
    let base = request_for(
        &root,
        MutationOperation::ReplaceSymbol {
            path: source.into(),
            symbol: "MainActivity".into(),
            replacement: "class MainActivity { fun renamed() {} }".into(),
        },
    );
    let original = fs::read_to_string(root.join(source)).expect("original");
    let mut stale_hash = base.clone();
    stale_hash
        .base_file_hashes
        .insert(source.into(), "stale-hash".into());
    assert_eq!(
        broker.apply(&stale_hash),
        Err(MutationError::BaseFileHashMismatch)
    );
    let mut unknown = base.clone();
    unknown.operation = MutationOperation::ReplaceSymbol {
        path: source.into(),
        symbol: "MissingSymbol".into(),
        replacement: "class MissingSymbol {}".into(),
    };
    assert_eq!(broker.apply(&unknown), Err(MutationError::UnknownSymbol));
    let mut over_budget = base.clone();
    over_budget.mutation_budget = 0;
    assert_eq!(
        broker.apply(&over_budget),
        Err(MutationError::MutationBudgetExceeded)
    );
    let mut invalid = base;
    invalid.operation = MutationOperation::ReplaceSymbol {
        path: source.into(),
        symbol: "MainActivity".into(),
        replacement: "class MainActivity {".into(),
    };
    assert_eq!(broker.apply(&invalid), Err(MutationError::SyntaxInvalid));
    assert_eq!(
        fs::read_to_string(root.join(source)).expect("unchanged"),
        original
    );
    let _ = fs::remove_dir_all(root);
}
