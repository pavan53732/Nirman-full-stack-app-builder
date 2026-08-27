#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

pub const CODE_INTELLIGENCE_SCHEMA_VERSION: u16 = 1;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IndexMode {
    Lightweight,
    FullSemantic,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileKind {
    Kotlin,
    Java,
    Xml,
    AndroidManifest,
    GradleKotlin,
    GradleGroovy,
    TypeScript,
    JavaScript,
    C,
    Cpp,
    Json,
    Yaml,
    Toml,
    Sql,
    Lockfile,
    Resource,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GraphNodeKind {
    File,
    Module,
    Symbol,
    Resource,
    Permission,
    NavigationRoute,
    Dependency,
    Test,
    DeviceProfile,
    Artifact,
    PreviewSurface,
    ApiLevel,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GraphEdgeKind {
    Contains,
    Defines,
    References,
    DependsOn,
    UsesPermission,
    NavigatesTo,
    NativeBoundary,
    Tests,
    CompatibleWithApi,
    ProducesArtifact,
    AffectsPreview,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IndexRequest {
    pub mode: IndexMode,
    pub technology_plan_hash: Option<String>,
    pub toolchain_lock_hash: Option<String>,
    pub relevant_external_state_hash: Option<String>,
}

impl Default for IndexRequest {
    fn default() -> Self {
        Self {
            mode: IndexMode::FullSemantic,
            technology_plan_hash: None,
            toolchain_lock_hash: None,
            relevant_external_state_hash: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IndexedFile {
    pub relative_path: String,
    pub content_hash: String,
    pub kind: FileKind,
    pub module_id: String,
    pub excluded: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub kind: GraphNodeKind,
    pub label: String,
    pub source_file: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub kind: GraphEdgeKind,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct SemanticGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct ImpactAnalysis {
    pub changed_files: Vec<String>,
    pub affected_files: Vec<String>,
    pub affected_modules: Vec<String>,
    pub affected_resources: Vec<String>,
    pub affected_tests: Vec<String>,
    pub affected_permissions: Vec<String>,
    pub affected_preview_surfaces: Vec<String>,
    pub affected_artifacts: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProjectIndex {
    pub schema_version: u16,
    pub mode: IndexMode,
    pub root_identity: String,
    pub project_fingerprint: String,
    pub files: Vec<IndexedFile>,
    pub graph: SemanticGraph,
    pub excluded_paths: Vec<String>,
    pub index_revision: u64,
}

impl ProjectIndex {
    pub fn analyze_change(&self, changed_paths: &[impl AsRef<str>]) -> ImpactAnalysis {
        let changed: BTreeSet<String> = changed_paths
            .iter()
            .map(|path| normalize_relative_path(path.as_ref()))
            .collect();
        let file_ids: BTreeSet<String> = changed
            .iter()
            .map(|path| file_node_id(path))
            .filter(|id| self.graph.nodes.iter().any(|node| node.id == *id))
            .collect();
        let mut affected_ids = file_ids.clone();
        let mut queue: VecDeque<String> = file_ids.into_iter().collect();
        while let Some(target) = queue.pop_front() {
            for edge in &self.graph.edges {
                if edge.to == target && affected_ids.insert(edge.from.clone()) {
                    queue.push_back(edge.from.clone());
                }
                if edge.from == target && affected_ids.insert(edge.to.clone()) {
                    queue.push_back(edge.to.clone());
                }
            }
        }

        let mut result = ImpactAnalysis {
            changed_files: changed.into_iter().collect(),
            ..ImpactAnalysis::default()
        };
        for node in &self.graph.nodes {
            if !affected_ids.contains(&node.id) {
                continue;
            }
            match node.kind {
                GraphNodeKind::File => result.affected_files.push(node.label.clone()),
                GraphNodeKind::Module => result.affected_modules.push(node.label.clone()),
                GraphNodeKind::Resource => result.affected_resources.push(node.label.clone()),
                GraphNodeKind::Test => result.affected_tests.push(
                    node.source_file
                        .clone()
                        .unwrap_or_else(|| node.label.clone()),
                ),
                GraphNodeKind::Permission => result.affected_permissions.push(node.label.clone()),
                GraphNodeKind::PreviewSurface => {
                    result.affected_preview_surfaces.push(node.label.clone())
                }
                GraphNodeKind::Artifact => result.affected_artifacts.push(node.label.clone()),
                _ => {}
            }
        }
        for values in [
            &mut result.affected_files,
            &mut result.affected_modules,
            &mut result.affected_resources,
            &mut result.affected_tests,
            &mut result.affected_permissions,
            &mut result.affected_preview_surfaces,
            &mut result.affected_artifacts,
        ] {
            values.sort();
            values.dedup();
        }
        result
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterOutput {
    symbols: Vec<String>,
    references: Vec<String>,
    dependencies: Vec<String>,
    resources: Vec<String>,
    permissions: Vec<String>,
    routes: Vec<String>,
    api_levels: Vec<String>,
    native_boundary: bool,
}

impl AdapterOutput {
    fn empty() -> Self {
        Self {
            symbols: Vec::new(),
            references: Vec::new(),
            dependencies: Vec::new(),
            resources: Vec::new(),
            permissions: Vec::new(),
            routes: Vec::new(),
            api_levels: Vec::new(),
            native_boundary: false,
        }
    }
}

pub trait AndroidLanguageAdapter: Send + Sync {
    fn kind(&self) -> FileKind;
    fn supports(&self, path: &str) -> bool;
    fn index(&self, path: &str, content: &str) -> AdapterOutput;
}

struct KotlinAdapter;
struct JavaAdapter;
struct XmlAdapter;
struct GradleAdapter;
struct ScriptAdapter;
struct NativeAdapter;
struct DataAdapter;
struct SqlAdapter;
struct LockfileAdapter;

fn common_code_output(content: &str) -> AdapterOutput {
    let mut output = AdapterOutput::empty();
    for line in content.lines() {
        let tokens: Vec<&str> = line
            .split(|character: char| {
                !(character.is_ascii_alphanumeric()
                    || character == '_'
                    || character == '.'
                    || character == ':'
                    || character == '/'
                    || character == '-'
                    || character == '@'
                    || character == '+')
            })
            .filter(|token| !token.is_empty())
            .collect();
        for window in tokens.windows(2) {
            if matches!(
                window[0],
                "class" | "interface" | "object" | "enum" | "fun" | "function"
            ) {
                output.symbols.push(window[1].trim_matches(':').to_owned());
            }
        }
        for token in &tokens {
            if token.starts_with("@+id/")
                || token.starts_with("@id/")
                || token.starts_with("@string/")
                || token.starts_with("@drawable/")
                || token.starts_with("@layout/")
            {
                output.resources.push((*token).to_owned());
            }
            if token.starts_with("android.permission.") {
                output.permissions.push((*token).to_owned());
            }
            if token.starts_with("api-")
                && token[4..]
                    .chars()
                    .all(|character| character.is_ascii_digit())
            {
                output.api_levels.push((*token).to_owned());
            }
        }
        if line.contains("navigate(")
            || line.contains("NavHost")
            || line.contains("composable(")
            || line.contains("NavController")
        {
            output
                .routes
                .push(quoted_value(line).unwrap_or_else(|| "navigation:unresolved".into()));
        }
        if line.contains("System.loadLibrary")
            || line.contains("externalNativeBuild")
            || line.contains("JNI")
            || line.contains("jni/")
        {
            output.native_boundary = true;
        }
    }
    output
}

impl AndroidLanguageAdapter for KotlinAdapter {
    fn kind(&self) -> FileKind {
        FileKind::Kotlin
    }
    fn supports(&self, path: &str) -> bool {
        path.ends_with(".kt")
            || (path.ends_with(".kts")
                && !path.ends_with("build.gradle.kts")
                && !path.ends_with("settings.gradle.kts"))
    }
    fn index(&self, _path: &str, content: &str) -> AdapterOutput {
        common_code_output(content)
    }
}
impl AndroidLanguageAdapter for JavaAdapter {
    fn kind(&self) -> FileKind {
        FileKind::Java
    }
    fn supports(&self, path: &str) -> bool {
        path.ends_with(".java")
    }
    fn index(&self, _path: &str, content: &str) -> AdapterOutput {
        common_code_output(content)
    }
}
impl AndroidLanguageAdapter for XmlAdapter {
    fn kind(&self) -> FileKind {
        FileKind::Xml
    }
    fn supports(&self, path: &str) -> bool {
        path.ends_with(".xml") && !path.ends_with("AndroidManifest.xml")
    }
    fn index(&self, _path: &str, content: &str) -> AdapterOutput {
        let mut output = common_code_output(content);
        for line in content.lines() {
            if line.contains("android:name=") || line.contains("app:destination=") {
                if let Some(value) = quoted_attribute(line) {
                    output.references.push(value);
                }
            }
        }
        output
    }
}
impl AndroidLanguageAdapter for GradleAdapter {
    fn kind(&self) -> FileKind {
        FileKind::GradleGroovy
    }
    fn supports(&self, path: &str) -> bool {
        path.ends_with("build.gradle")
            || path.ends_with("build.gradle.kts")
            || path.ends_with("settings.gradle")
            || path.ends_with("settings.gradle.kts")
            || path.ends_with("gradle.properties")
    }
    fn index(&self, _path: &str, content: &str) -> AdapterOutput {
        let mut output = common_code_output(content);
        for line in content.lines() {
            if [
                "implementation",
                "api",
                "kapt",
                "classpath",
                "compileOnly",
                "testImplementation",
            ]
            .iter()
            .any(|prefix| line.contains(prefix))
            {
                if let Some(value) = quoted_value(line) {
                    output.dependencies.push(value);
                }
            }
            for key in ["minSdk", "targetSdk", "compileSdk"] {
                if let Some(value) = line.split(key).nth(1).and_then(|rest| {
                    rest.split(|character: char| !character.is_ascii_digit())
                        .find(|value| !value.is_empty())
                }) {
                    output.api_levels.push(format!("{key}-{value}"));
                }
            }
        }
        output
    }
}
impl AndroidLanguageAdapter for ScriptAdapter {
    fn kind(&self) -> FileKind {
        FileKind::TypeScript
    }
    fn supports(&self, path: &str) -> bool {
        path.ends_with(".ts")
            || path.ends_with(".tsx")
            || path.ends_with(".js")
            || path.ends_with(".jsx")
    }
    fn index(&self, _path: &str, content: &str) -> AdapterOutput {
        common_code_output(content)
    }
}
impl AndroidLanguageAdapter for NativeAdapter {
    fn kind(&self) -> FileKind {
        FileKind::Cpp
    }
    fn supports(&self, path: &str) -> bool {
        [".c", ".cc", ".cpp", ".h", ".hpp"]
            .iter()
            .any(|suffix| path.ends_with(suffix))
    }
    fn index(&self, _path: &str, content: &str) -> AdapterOutput {
        let mut output = common_code_output(content);
        output.native_boundary = true;
        output
    }
}
impl AndroidLanguageAdapter for DataAdapter {
    fn kind(&self) -> FileKind {
        FileKind::Json
    }
    fn supports(&self, path: &str) -> bool {
        [".json", ".yaml", ".yml", ".toml"]
            .iter()
            .any(|suffix| path.ends_with(suffix))
    }
    fn index(&self, _path: &str, content: &str) -> AdapterOutput {
        common_code_output(content)
    }
}
impl AndroidLanguageAdapter for SqlAdapter {
    fn kind(&self) -> FileKind {
        FileKind::Sql
    }
    fn supports(&self, path: &str) -> bool {
        path.ends_with(".sql")
    }
    fn index(&self, _path: &str, content: &str) -> AdapterOutput {
        let mut output = common_code_output(content);
        for line in content.lines() {
            let upper = line.to_ascii_uppercase();
            if upper.contains("CREATE TABLE") || upper.contains("CREATE VIEW") {
                let name = line
                    .split_whitespace()
                    .last()
                    .unwrap_or("sql-object")
                    .trim_matches(|character| character == '(' || character == ';');
                output.symbols.push(name.into());
            }
        }
        output
    }
}
impl AndroidLanguageAdapter for LockfileAdapter {
    fn kind(&self) -> FileKind {
        FileKind::Lockfile
    }
    fn supports(&self, path: &str) -> bool {
        is_lockfile(path)
    }
    fn index(&self, _path: &str, content: &str) -> AdapterOutput {
        let mut output = AdapterOutput::empty();
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('[') {
                if let Some(value) = quoted_value(trimmed) {
                    output.dependencies.push(value);
                }
            }
        }
        output
    }
}

pub struct LanguageAdapterRegistry {
    adapters: Vec<Box<dyn AndroidLanguageAdapter>>,
}
impl Default for LanguageAdapterRegistry {
    fn default() -> Self {
        Self {
            adapters: vec![
                Box::new(KotlinAdapter),
                Box::new(JavaAdapter),
                Box::new(XmlAdapter),
                Box::new(GradleAdapter),
                Box::new(ScriptAdapter),
                Box::new(NativeAdapter),
                Box::new(SqlAdapter),
                Box::new(LockfileAdapter),
                Box::new(DataAdapter),
            ],
        }
    }
}
impl LanguageAdapterRegistry {
    fn adapter_for(&self, path: &str) -> Option<&dyn AndroidLanguageAdapter> {
        self.adapters
            .iter()
            .find(|adapter| adapter.supports(path))
            .map(|adapter| adapter.as_ref())
    }
    pub fn detect(&self, path: &str) -> Option<FileKind> {
        if path.ends_with("AndroidManifest.xml") {
            return Some(FileKind::AndroidManifest);
        }
        if is_resource_path(path) && path.ends_with(".xml") {
            return Some(FileKind::Resource);
        }
        self.adapter_for(path)
            .map(|adapter| public_kind(adapter.kind(), path))
    }
    fn index(&self, path: &str, content: &str) -> (FileKind, AdapterOutput) {
        if path.ends_with("AndroidManifest.xml") {
            return (FileKind::AndroidManifest, index_manifest(content));
        }
        if is_resource_path(path) && path.ends_with(".xml") {
            return (FileKind::Resource, XmlAdapter.index(path, content));
        }
        if let Some(adapter) = self.adapter_for(path) {
            return (
                public_kind(adapter.kind(), path),
                adapter.index(path, content),
            );
        }
        (fallback_file_kind(path), AdapterOutput::empty())
    }
}

pub struct ProjectIndexer {
    registry: LanguageAdapterRegistry,
}
impl Default for ProjectIndexer {
    fn default() -> Self {
        Self {
            registry: LanguageAdapterRegistry::default(),
        }
    }
}
impl ProjectIndexer {
    pub fn new(registry: LanguageAdapterRegistry) -> Self {
        Self { registry }
    }
    pub fn index_workspace(
        &self,
        root: impl AsRef<Path>,
        request: &IndexRequest,
    ) -> std::io::Result<ProjectIndex> {
        let root = root.as_ref().canonicalize()?;
        let mut candidates = Vec::new();
        collect_files(&root, &mut candidates)?;
        candidates.sort();
        let mut files = Vec::new();
        let mut graph = SemanticGraph::default();
        let mut excluded_paths = Vec::new();
        let mut contents = BTreeMap::new();
        for path in candidates {
            let relative = normalize_relative_path(
                path.strip_prefix(&root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .as_ref(),
            );
            if is_excluded_path(&relative) {
                excluded_paths.push(relative);
                continue;
            }
            let content = fs::read_to_string(&path).unwrap_or_default();
            let (kind, output) = self.registry.index(&relative, &content);
            let module = module_for_path(&relative);
            let hash = sha256_hex(content.as_bytes());
            contents.insert(relative.clone(), content);
            files.push(IndexedFile {
                relative_path: relative.clone(),
                content_hash: hash,
                kind,
                module_id: module.clone(),
                excluded: false,
            });
            add_node(
                &mut graph,
                GraphNode {
                    id: file_node_id(&relative),
                    kind: GraphNodeKind::File,
                    label: relative.clone(),
                    source_file: Some(relative.clone()),
                    metadata: BTreeMap::from([("fileKind".into(), format!("{kind:?}"))]),
                },
            );
            let module_id = module_node_id(&module);
            add_node(
                &mut graph,
                GraphNode {
                    id: module_id.clone(),
                    kind: GraphNodeKind::Module,
                    label: module,
                    source_file: None,
                    metadata: BTreeMap::new(),
                },
            );
            add_edge(
                &mut graph,
                file_node_id(&relative),
                module_id,
                GraphEdgeKind::Contains,
            );
            index_output(&mut graph, &relative, output);
            let preview_id = preview_node_id(&relative);
            add_node(
                &mut graph,
                GraphNode {
                    id: preview_id.clone(),
                    kind: GraphNodeKind::PreviewSurface,
                    label: format!("preview:{relative}"),
                    source_file: Some(relative.clone()),
                    metadata: BTreeMap::new(),
                },
            );
            add_edge(
                &mut graph,
                file_node_id(&relative),
                preview_id,
                GraphEdgeKind::AffectsPreview,
            );
        }
        if request.mode == IndexMode::FullSemantic {
            resolve_symbol_edges(&mut graph, &contents);
            add_artifact_and_device_nodes(&mut graph, &files);
        }
        sort_graph(&mut graph);
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        excluded_paths.sort();
        let project_fingerprint = fingerprint(&root, request, &files, &graph);
        Ok(ProjectIndex {
            schema_version: CODE_INTELLIGENCE_SCHEMA_VERSION,
            mode: request.mode,
            root_identity: root.to_string_lossy().into_owned(),
            project_fingerprint,
            files,
            graph,
            excluded_paths,
            index_revision: 1,
        })
    }
}

fn collect_files(current: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_files(&path, output)?;
        } else if file_type.is_file() {
            output.push(path);
        }
    }
    Ok(())
}
fn public_kind(kind: FileKind, path: &str) -> FileKind {
    match kind {
        FileKind::GradleGroovy if path.ends_with(".kts") => FileKind::GradleKotlin,
        FileKind::TypeScript if path.ends_with(".js") || path.ends_with(".jsx") => {
            FileKind::JavaScript
        }
        FileKind::Cpp if path.ends_with(".c") => FileKind::C,
        FileKind::Json if path.ends_with(".yaml") || path.ends_with(".yml") => FileKind::Yaml,
        FileKind::Json if path.ends_with(".toml") => FileKind::Toml,
        other => other,
    }
}
fn index_manifest(content: &str) -> AdapterOutput {
    let mut output = common_code_output(content);
    for line in content.lines() {
        if line.contains("uses-permission") {
            if let Some(value) = quoted_attribute(line) {
                output.permissions.push(value);
            }
        }
        if line.contains("android:name=") {
            if let Some(value) = quoted_attribute(line) {
                output.references.push(value);
            }
        }
    }
    output
}
fn index_output(graph: &mut SemanticGraph, relative: &str, output: AdapterOutput) {
    let source = file_node_id(relative);
    for symbol in output.symbols {
        let id = symbol_node_id(relative, &symbol);
        add_node(
            graph,
            GraphNode {
                id: id.clone(),
                kind: if is_test_path(relative) {
                    GraphNodeKind::Test
                } else {
                    GraphNodeKind::Symbol
                },
                label: symbol,
                source_file: Some(relative.into()),
                metadata: BTreeMap::new(),
            },
        );
        add_edge(graph, source.clone(), id, GraphEdgeKind::Defines);
    }
    for dependency in output.dependencies {
        let id = dependency_node_id(&dependency);
        add_node(
            graph,
            GraphNode {
                id: id.clone(),
                kind: GraphNodeKind::Dependency,
                label: dependency,
                source_file: Some(relative.into()),
                metadata: BTreeMap::new(),
            },
        );
        add_edge(graph, source.clone(), id, GraphEdgeKind::DependsOn);
    }
    for resource in output.resources {
        let id = resource_node_id(&resource);
        add_node(
            graph,
            GraphNode {
                id: id.clone(),
                kind: GraphNodeKind::Resource,
                label: resource,
                source_file: Some(relative.into()),
                metadata: BTreeMap::new(),
            },
        );
        add_edge(graph, source.clone(), id, GraphEdgeKind::References);
    }
    for permission in output.permissions {
        let id = permission_node_id(&permission);
        add_node(
            graph,
            GraphNode {
                id: id.clone(),
                kind: GraphNodeKind::Permission,
                label: permission,
                source_file: Some(relative.into()),
                metadata: BTreeMap::new(),
            },
        );
        add_edge(graph, source.clone(), id, GraphEdgeKind::UsesPermission);
    }
    for route in output.routes {
        let id = route_node_id(&route);
        add_node(
            graph,
            GraphNode {
                id: id.clone(),
                kind: GraphNodeKind::NavigationRoute,
                label: route,
                source_file: Some(relative.into()),
                metadata: BTreeMap::new(),
            },
        );
        add_edge(graph, source.clone(), id, GraphEdgeKind::NavigatesTo);
    }
    for api in output.api_levels {
        let id = api_node_id(&api);
        add_node(
            graph,
            GraphNode {
                id: id.clone(),
                kind: GraphNodeKind::ApiLevel,
                label: api,
                source_file: Some(relative.into()),
                metadata: BTreeMap::new(),
            },
        );
        add_edge(graph, source.clone(), id, GraphEdgeKind::CompatibleWithApi);
    }
    if output.native_boundary {
        let id = format!("native-boundary:{relative}");
        add_node(
            graph,
            GraphNode {
                id: id.clone(),
                kind: GraphNodeKind::Module,
                label: id.clone(),
                source_file: Some(relative.into()),
                metadata: BTreeMap::new(),
            },
        );
        add_edge(graph, source, id, GraphEdgeKind::NativeBoundary);
    }
}
fn resolve_symbol_edges(graph: &mut SemanticGraph, contents: &BTreeMap<String, String>) {
    let symbols: Vec<(String, String)> = graph
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, GraphNodeKind::Symbol | GraphNodeKind::Test))
        .map(|node| (node.id.clone(), node.label.clone()))
        .collect();
    for (path, content) in contents {
        let file_id = file_node_id(path);
        for (symbol_id, label) in &symbols {
            if symbol_id.starts_with(&file_id) {
                continue;
            }
            if content
                .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
                .any(|token| token == label)
            {
                add_edge(
                    graph,
                    file_id.clone(),
                    symbol_id.clone(),
                    GraphEdgeKind::References,
                );
                if is_test_path(path) {
                    add_edge(
                        graph,
                        file_id.clone(),
                        symbol_id.clone(),
                        GraphEdgeKind::Tests,
                    );
                }
            }
        }
    }
}
fn add_artifact_and_device_nodes(graph: &mut SemanticGraph, files: &[IndexedFile]) {
    let artifact_id = "artifact:debug-apk".to_owned();
    add_node(
        graph,
        GraphNode {
            id: artifact_id.clone(),
            kind: GraphNodeKind::Artifact,
            label: artifact_id.clone(),
            source_file: None,
            metadata: BTreeMap::from([("kind".into(), "apk".into())]),
        },
    );
    let device_id = "device-profile:declared".to_owned();
    add_node(
        graph,
        GraphNode {
            id: device_id,
            kind: GraphNodeKind::DeviceProfile,
            label: "device-profile:declared".into(),
            source_file: None,
            metadata: BTreeMap::new(),
        },
    );
    for file in files {
        if matches!(
            file.kind,
            FileKind::Kotlin
                | FileKind::Java
                | FileKind::Xml
                | FileKind::AndroidManifest
                | FileKind::GradleKotlin
                | FileKind::GradleGroovy
                | FileKind::Cpp
                | FileKind::C
        ) {
            add_edge(
                graph,
                file_node_id(&file.relative_path),
                artifact_id.clone(),
                GraphEdgeKind::ProducesArtifact,
            );
        }
    }
}
fn add_node(graph: &mut SemanticGraph, node: GraphNode) {
    if !graph.nodes.iter().any(|existing| existing.id == node.id) {
        graph.nodes.push(node);
    }
}
fn add_edge(graph: &mut SemanticGraph, from: String, to: String, kind: GraphEdgeKind) {
    if !graph
        .edges
        .iter()
        .any(|edge| edge.from == from && edge.to == to && edge.kind == kind)
    {
        graph.edges.push(GraphEdge {
            from,
            to,
            kind,
            metadata: BTreeMap::new(),
        });
    }
}
fn sort_graph(graph: &mut SemanticGraph) {
    graph.nodes.sort_by(|left, right| left.id.cmp(&right.id));
    graph.edges.sort_by(|left, right| {
        (&left.from, &left.to, &left.kind).cmp(&(&right.from, &right.to, &right.kind))
    });
}
fn fingerprint(
    root: &Path,
    request: &IndexRequest,
    files: &[IndexedFile],
    graph: &SemanticGraph,
) -> String {
    let canonical = format!(
        "{}|{}|{}|{}",
        root.to_string_lossy(),
        serde_json::to_string(request).expect("request serialization"),
        serde_json::to_string(files).expect("file serialization"),
        serde_json::to_string(graph).expect("graph serialization")
    );
    sha256_hex(canonical.as_bytes())
}
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn normalize_relative_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_owned()
}
fn file_node_id(path: &str) -> String {
    format!("file:{}", normalize_relative_path(path))
}
fn module_node_id(module: &str) -> String {
    format!("module:{module}")
}
fn symbol_node_id(path: &str, symbol: &str) -> String {
    format!("symbol:{}::{symbol}", normalize_relative_path(path))
}
fn dependency_node_id(value: &str) -> String {
    format!("dependency:{value}")
}
fn resource_node_id(value: &str) -> String {
    format!("resource:{value}")
}
fn permission_node_id(value: &str) -> String {
    format!("permission:{value}")
}
fn route_node_id(value: &str) -> String {
    format!("route:{value}")
}
fn api_node_id(value: &str) -> String {
    format!("api:{value}")
}
fn preview_node_id(path: &str) -> String {
    format!("preview:{path}")
}
fn module_for_path(path: &str) -> String {
    path.split('/')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("root")
        .to_owned()
}
fn is_test_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("/test/")
        || lower.contains("/androidtest/")
        || lower.ends_with("test.kt")
        || lower.ends_with("test.java")
        || lower.ends_with("test.ts")
}
fn is_resource_path(path: &str) -> bool {
    path.split('/')
        .any(|part| part == "res" || part == "resources")
}
fn is_lockfile(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    matches!(
        name,
        "gradle.lockfile"
            | "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "libs.versions.toml"
            | "Cargo.lock"
    )
}
fn is_excluded_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.split('/').any(|part| {
        matches!(
            part,
            "build"
                | ".gradle"
                | ".idea"
                | "node_modules"
                | "vendor"
                | "generated"
                | "intermediates"
                | "caches"
                | "target"
        )
    }) || lower.ends_with(".keystore")
        || lower.ends_with(".jks")
}
fn fallback_file_kind(path: &str) -> FileKind {
    if path.ends_with(".xml") {
        FileKind::Xml
    } else if path.ends_with(".json") {
        FileKind::Json
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        FileKind::Yaml
    } else if path.ends_with(".toml") {
        FileKind::Toml
    } else {
        FileKind::Resource
    }
}
fn quoted_value(line: &str) -> Option<String> {
    let start = line.find('"').or_else(|| line.find('\''))?;
    let quote = line.as_bytes()[start] as char;
    let rest = &line[start + 1..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_owned())
}
fn quoted_attribute(line: &str) -> Option<String> {
    line.split('=').nth(1).and_then(quoted_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nirman-m45-fixture-{nonce}"));
        fs::create_dir_all(root.join("app/src/main/java/com/example")).expect("java dir");
        fs::create_dir_all(root.join("app/src/main/kotlin/com/example")).expect("kotlin dir");
        fs::create_dir_all(root.join("app/src/main/res/navigation")).expect("resource dir");
        fs::create_dir_all(root.join("app/src/androidTest")).expect("test dir");
        fs::create_dir_all(root.join("app/src/main/cpp")).expect("native dir");
        fs::create_dir_all(root.join("node_modules/ignored")).expect("excluded dir");
        fs::write(root.join("settings.gradle.kts"), "include(\":app\")\n").expect("settings");
        fs::write(root.join("app/build.gradle.kts"), "android { compileSdk = 35; defaultConfig { minSdk = 29; targetSdk = 35 } }\ndependencies { implementation(\"androidx.activity:activity-compose:1.9.0\") }\n").expect("gradle");
        fs::write(root.join("app/src/main/AndroidManifest.xml"), "<manifest><uses-permission android:name=\"android.permission.CAMERA\"/><application android:name=\".MainActivity\"/></manifest>").expect("manifest");
        fs::write(
            root.join("app/src/main/kotlin/com/example/MainActivity.kt"),
            "class MainActivity { fun open() { navController.navigate(\"home\") } }\n",
        )
        .expect("kotlin");
        fs::write(
            root.join("app/src/main/layout.xml"),
            "<layout><view android:name=\"MainActivity\"/></layout>\n",
        )
        .expect("xml");
        fs::write(
            root.join("app/src/main/java/com/example/Repository.java"),
            "public class Repository {}\n",
        )
        .expect("java");
        fs::write(
            root.join("app/src/main/res/navigation/routes.xml"),
            "<navigation app:destination=\"home\"/>\n",
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
        .expect("cpp");
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
        .expect("lock");
        fs::write(
            root.join("node_modules/ignored/index.js"),
            "class Ignored {}\n",
        )
        .expect("ignored");
        root
    }

    #[test]
    fn m45_indexes_all_declared_language_families_and_graph_relationships() {
        let root = fixture_root();
        let index = ProjectIndexer::default()
            .index_workspace(&root, &IndexRequest::default())
            .expect("index");
        assert_eq!(index.schema_version, CODE_INTELLIGENCE_SCHEMA_VERSION);
        assert_eq!(index.mode, IndexMode::FullSemantic);
        let kinds: BTreeSet<_> = index.files.iter().map(|file| file.kind).collect();
        for kind in [
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
        ] {
            assert!(kinds.contains(&kind), "missing {kind:?}");
        }
        assert!(index
            .graph
            .nodes
            .iter()
            .any(|node| node.kind == GraphNodeKind::Permission && node.label.contains("CAMERA")));
        assert!(index
            .graph
            .nodes
            .iter()
            .any(|node| node.kind == GraphNodeKind::NavigationRoute && node.label == "home"));
        assert!(index
            .graph
            .nodes
            .iter()
            .any(|node| node.kind == GraphNodeKind::Dependency
                && node.label.contains("activity-compose")));
        assert!(index
            .graph
            .edges
            .iter()
            .any(|edge| edge.kind == GraphEdgeKind::Tests));
        assert!(index
            .graph
            .edges
            .iter()
            .any(|edge| edge.kind == GraphEdgeKind::NativeBoundary));
        assert!(!index
            .files
            .iter()
            .any(|file| file.relative_path.contains("node_modules")));
        assert!(!index.excluded_paths.is_empty());
        assert!(!index.project_fingerprint.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn m45_fingerprint_and_index_order_are_deterministic() {
        let root = fixture_root();
        let request = IndexRequest {
            technology_plan_hash: Some("plan-hash".into()),
            toolchain_lock_hash: Some("lock-hash".into()),
            ..IndexRequest::default()
        };
        let first = ProjectIndexer::default()
            .index_workspace(&root, &request)
            .expect("first index");
        let second = ProjectIndexer::default()
            .index_workspace(&root, &request)
            .expect("second index");
        assert_eq!(first, second);
        let changed = first.analyze_change(&["app/src/main/kotlin/com/example/MainActivity.kt"]);
        assert!(changed
            .changed_files
            .contains(&"app/src/main/kotlin/com/example/MainActivity.kt".into()));
        assert!(changed
            .affected_files
            .iter()
            .any(|path| path.ends_with("MainActivity.kt")));
        assert!(!changed.affected_modules.is_empty());
        assert!(!changed.affected_preview_surfaces.is_empty());
        assert!(!changed.affected_artifacts.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn m45_lightweight_mode_is_read_only_and_excludes_build_outputs() {
        let root = fixture_root();
        fs::create_dir_all(root.join("app/build/outputs/apk")).expect("build dir");
        fs::write(root.join("app/build/outputs/apk/debug.apk"), "not indexed").expect("apk");
        let index = ProjectIndexer::default()
            .index_workspace(
                &root,
                &IndexRequest {
                    mode: IndexMode::Lightweight,
                    ..IndexRequest::default()
                },
            )
            .expect("index");
        assert_eq!(index.mode, IndexMode::Lightweight);
        assert!(index
            .files
            .iter()
            .all(|file| !file.relative_path.contains("build/outputs")));
        assert!(!index
            .files
            .iter()
            .any(|file| file.relative_path.ends_with("debug.apk")));
        let _ = fs::remove_dir_all(root);
    }
}
