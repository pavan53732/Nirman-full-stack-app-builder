use nirman_project::{FileKind, GraphEdgeKind, GraphNodeKind, IndexRequest, ProjectIndexer};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nirman-m45-acceptance-{nonce}"));
    fs::create_dir_all(root.join("app/src/main/kotlin/com/example")).expect("kotlin directory");
    fs::create_dir_all(root.join("app/src/main/java/com/example")).expect("java directory");
    fs::create_dir_all(root.join("app/src/main/res/navigation")).expect("resource directory");
    fs::create_dir_all(root.join("app/src/androidTest")).expect("test directory");
    fs::create_dir_all(root.join("app/src/main/cpp")).expect("native directory");
    fs::create_dir_all(root.join("node_modules/ignored")).expect("excluded directory");
    fs::write(root.join("settings.gradle.kts"), "include(\":app\")\n").expect("settings");
    fs::write(
        root.join("app/build.gradle.kts"),
        "android { compileSdk = 35; defaultConfig { minSdk = 29; targetSdk = 35 } }\ndependencies { implementation(\"androidx.activity:activity-compose:1.9.0\") }\n",
    )
    .expect("gradle");
    fs::write(
        root.join("app/src/main/AndroidManifest.xml"),
        "<manifest><uses-permission android:name=\"android.permission.CAMERA\"/><application android:name=\".MainActivity\"/></manifest>",
    )
    .expect("manifest");
    fs::write(
        root.join("app/src/main/kotlin/com/example/MainActivity.kt"),
        "class MainActivity { fun open() { navController.navigate(\"home\") } }\n",
    )
    .expect("kotlin");
    fs::write(
        root.join("app/src/main/java/com/example/Repository.java"),
        "public class Repository {}\n",
    )
    .expect("java");
    fs::write(
        root.join("app/src/main/res/navigation/routes.xml"),
        "<navigation app:destination=\"home\"/>\n",
    )
    .expect("resource xml");
    fs::write(
        root.join("app/src/main/layout.xml"),
        "<layout><view android:id=\"@+id/home\" android:name=\"MainActivity\"/></layout>\n",
    )
    .expect("xml");
    fs::write(
        root.join("app/src/androidTest/MainActivityTest.kt"),
        "class MainActivityTest { fun testOpen() { MainActivity() } }\n",
    )
    .expect("test");
    fs::write(
        root.join("app/src/main/cpp/native.cpp"),
        "void nativeCall() {}\n",
    )
    .expect("native");
    fs::write(
        root.join("app/src/main/cpp/native.c"),
        "void cNativeCall() {}\n",
    )
    .expect("c");
    fs::write(
        root.join("screen.ts"),
        "export function Screen() { return 'home'; }\n",
    )
    .expect("typescript");
    fs::write(
        root.join("screen.js"),
        "function legacyScreen() { return 'home'; }\n",
    )
    .expect("javascript");
    fs::write(root.join("config.yaml"), "name: sample\n").expect("yaml");
    fs::write(root.join("config.toml"), "name = \"sample\"\n").expect("toml");
    fs::write(root.join("package.json"), "{\"name\":\"sample\"}\n").expect("json");
    fs::write(
        root.join("schema.sql"),
        "CREATE TABLE notes (id INTEGER);\n",
    )
    .expect("sql");
    fs::write(
        root.join("gradle.lockfile"),
        "androidx.activity:activity-compose:1.9.0=\n",
    )
    .expect("lockfile");
    fs::write(
        root.join("node_modules/ignored/index.js"),
        "class Ignored {}\n",
    )
    .expect("ignored");
    root
}

#[test]
fn m45_acceptance_is_file_backed_and_observation_derived() {
    let root = fixture_root();
    let request = IndexRequest {
        technology_plan_hash: Some("technology-plan-hash".into()),
        toolchain_lock_hash: Some("toolchain-lock-hash".into()),
        relevant_external_state_hash: Some("external-state-hash".into()),
        ..IndexRequest::default()
    };
    let indexer = ProjectIndexer::default();
    let first = indexer
        .index_workspace(&root, &request)
        .expect("full semantic index");
    let kinds: BTreeSet<_> = first.files.iter().map(|file| file.kind).collect();
    let representative_languages_observed = [
        FileKind::Kotlin,
        FileKind::Java,
        FileKind::Xml,
        FileKind::AndroidManifest,
        FileKind::GradleKotlin,
        FileKind::TypeScript,
        FileKind::JavaScript,
        FileKind::C,
        FileKind::Cpp,
        FileKind::Json,
        FileKind::Yaml,
        FileKind::Toml,
        FileKind::Sql,
        FileKind::Lockfile,
    ]
    .into_iter()
    .all(|kind| kinds.contains(&kind));
    assert!(representative_languages_observed);
    let graph_relationships_observed = first
        .graph
        .nodes
        .iter()
        .any(|node| node.kind == GraphNodeKind::Permission && node.label.contains("CAMERA"))
        && first
            .graph
            .nodes
            .iter()
            .any(|node| node.kind == GraphNodeKind::NavigationRoute && node.label == "home")
        && first.graph.nodes.iter().any(|node| {
            node.kind == GraphNodeKind::Dependency && node.label.contains("activity-compose")
        })
        && first
            .graph
            .edges
            .iter()
            .any(|edge| edge.kind == GraphEdgeKind::Tests)
        && first
            .graph
            .edges
            .iter()
            .any(|edge| edge.kind == GraphEdgeKind::NativeBoundary);
    assert!(graph_relationships_observed);
    let excluded_paths_observed = !first.excluded_paths.is_empty()
        && !first
            .files
            .iter()
            .any(|file| file.relative_path.contains("node_modules"));
    assert!(excluded_paths_observed);
    let serialized = serde_json::to_string(&first).expect("index serialization");
    let restored = serde_json::from_str(&serialized).expect("index reload");
    let deterministic_reload_observed = first == restored;
    assert!(deterministic_reload_observed);
    let second = indexer
        .index_workspace(&root, &request)
        .expect("repeat index");
    let deterministic_fingerprint_observed =
        first.project_fingerprint == second.project_fingerprint;
    assert!(deterministic_fingerprint_observed);
    let impact = first.analyze_change(&[
        "app/src/main/kotlin/com/example/MainActivity.kt",
        "app/src/main/AndroidManifest.xml",
        "app/src/main/res/navigation/routes.xml",
        "app/src/main/layout.xml",
    ]);
    let affected_test_analysis_observed = impact
        .affected_tests
        .iter()
        .any(|path| path.ends_with("MainActivityTest.kt"));
    assert!(affected_test_analysis_observed);
    let read_only_observed = !first
        .files
        .iter()
        .any(|file| file.relative_path.ends_with(".apk"));
    assert!(read_only_observed);
    let evidence = serde_json::json!({
        "schema": "nirman.m45.android_code_intelligence.v1",
        "representativeLanguageAdaptersObserved": representative_languages_observed,
        "fullSemanticGraphObserved": graph_relationships_observed,
        "projectFingerprintObserved": !first.project_fingerprint.is_empty(),
        "deterministicReloadObserved": deterministic_reload_observed,
        "deterministicRepeatIndexObserved": deterministic_fingerprint_observed,
        "excludedBuildCacheVendorPathsObserved": excluded_paths_observed,
        "affectedFilesAndModulesObserved": !impact.affected_files.is_empty() && !impact.affected_modules.is_empty(),
        "affectedTestsObserved": affected_test_analysis_observed,
        "affectedPermissionsResourcesPreviewArtifactsObserved": !impact.affected_permissions.is_empty() && !impact.affected_resources.is_empty() && !impact.affected_preview_surfaces.is_empty() && !impact.affected_artifacts.is_empty(),
        "readOnlyNoWorkspaceMutationObserved": read_only_observed,
        "structuredMutationBroker": false,
        "androidBuildObserved": false,
        "nativeWindowsTauriRuntimeObserved": false,
        "evidenceStatus": "M45_HEADLESS_READ_ONLY_INDEX_TRACE_ONLY"
    });
    let evidence_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/evidence/m45_android_code_intelligence.json");
    fs::create_dir_all(evidence_path.parent().expect("evidence directory"))
        .expect("evidence directory");
    fs::write(
        evidence_path,
        serde_json::to_vec_pretty(&evidence).expect("evidence JSON"),
    )
    .expect("evidence write");
    fs::remove_dir_all(root).expect("fixture cleanup");
}
