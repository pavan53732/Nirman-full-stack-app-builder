//! Real Android project scaffolding (M4b).
//!
//! Turns a validated [`AndroidConstructionContract`] into a complete,
//! buildable Gradle Android project: Gradle build files, the Android
//! manifest with derived permissions, launcher activity sources, feature
//! screens derived from contract requirements, and Android resources.
//!
//! The generator is pure: it returns an in-memory [`AndroidProjectScaffold`]
//! and never touches the filesystem. [`AndroidProjectScaffold::apply`] is the
//! only side-effecting entry point and enforces workspace containment for
//! every generated file.

use super::AndroidTechnologyPlan;
use nirman_domain::{AndroidConstructionContract, ConstructionRequirement};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

pub const SCAFFOLD_SCHEMA_VERSION: u16 = 1;
pub const SCAFFOLD_SCHEMA_REF: &str = "nirman.android_project_scaffold.v1";

pub const DEFAULT_MIN_SDK: u32 = 24;
pub const DEFAULT_TARGET_SDK: u32 = 34;
pub const COMPILE_SDK: u32 = 35;
pub const GRADLE_VERSION: &str = "8.9";
pub const AGP_VERSION: &str = "8.5.2";
pub const KOTLIN_VERSION: &str = "2.0.20";
pub const COMPOSE_BOM_VERSION: &str = "2024.09.03";
pub const ACTIVITY_COMPOSE_VERSION: &str = "1.9.2";
pub const CORE_KTX_VERSION: &str = "1.13.1";
pub const LIFECYCLE_VERSION: &str = "2.8.6";
pub const APPCOMPAT_VERSION: &str = "1.7.0";
pub const MATERIAL_VERSION: &str = "1.12.0";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ScaffoldFile {
    pub relative_path: String,
    pub contents: String,
    pub language: ScaffoldLanguage,
    pub purpose: String,
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaffoldLanguage {
    GradleKts,
    Kotlin,
    Java,
    Xml,
    Properties,
    Proguard,
    GitIgnore,
    Markdown,
}

impl ScaffoldLanguage {
    fn as_str(self) -> &'static str {
        match self {
            Self::GradleKts => "gradle-kotlin-dsl",
            Self::Kotlin => "kotlin",
            Self::Java => "java",
            Self::Xml => "xml",
            Self::Properties => "properties",
            Self::Proguard => "proguard",
            Self::GitIgnore => "gitignore",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ScaffoldSummary {
    pub scaffold_id: String,
    pub contract_id: String,
    pub project_id: String,
    pub task_id: String,
    pub package_name: String,
    pub application_name: String,
    pub language: String,
    pub ui_framework: String,
    pub min_sdk: u32,
    pub target_sdk: u32,
    pub compile_sdk: u32,
    pub version_code: u64,
    pub version_name: String,
    pub permissions: Vec<String>,
    pub feature_screens: Vec<String>,
    pub file_count: usize,
    pub scaffold_fingerprint: String,
    pub resulting_project_fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AndroidProjectScaffold {
    pub schema_version: u16,
    pub scaffold_id: String,
    pub contract_id: String,
    pub project_id: String,
    pub task_id: String,
    pub package_name: String,
    pub application_name: String,
    pub language: String,
    pub ui_framework: String,
    pub min_sdk: u32,
    pub target_sdk: u32,
    pub compile_sdk: u32,
    pub version_code: u64,
    pub version_name: String,
    pub permissions: Vec<String>,
    pub feature_screens: Vec<FeatureScreen>,
    pub files: Vec<ScaffoldFile>,
    pub scaffold_fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct FeatureScreen {
    pub screen_id: String,
    pub route: String,
    pub title: String,
    pub statement: String,
    pub requirement_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScaffoldError {
    InvalidContract(String),
    UnsupportedPlatform,
    EmptyField(&'static str),
    UnsupportedLanguage(String),
    UnsupportedUiFramework(String),
    NoScreens,
    InvalidPath(String),
    WriteFailed(String),
    OutsideWorkspace(String),
}

impl fmt::Display for ScaffoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract(reason) => {
                write!(f, "M4b construction contract is invalid: {reason}")
            }
            Self::UnsupportedPlatform => f.write_str("M4b scaffolding supports Android only"),
            Self::EmptyField(field) => write!(f, "M4b scaffold field is empty: {field}"),
            Self::UnsupportedLanguage(language) => {
                write!(f, "M4b unsupported scaffold language: {language}")
            }
            Self::UnsupportedUiFramework(framework) => {
                write!(f, "M4b unsupported UI framework: {framework}")
            }
            Self::NoScreens => {
                f.write_str("M4b scaffold requires at least one feature requirement")
            }
            Self::InvalidPath(path) => write!(f, "M4b scaffold path is unsafe: {path}"),
            Self::WriteFailed(path) => write!(f, "M4b scaffold file could not be written: {path}"),
            Self::OutsideWorkspace(path) => {
                write!(f, "M4b scaffold file escapes the workspace: {path}")
            }
        }
    }
}
impl std::error::Error for ScaffoldError {}

/// Mapping from requirement keywords to Android manifest permissions.
/// Order is deterministic: the manifest emits permissions sorted by name.
fn permission_for_keyword(keyword: &str) -> Option<&'static str> {
    Some(match keyword {
        "camera" => "android.permission.CAMERA",
        "location" | "gps" | "maps" => "android.permission.ACCESS_COARSE_LOCATION",
        "microphone" | "audio record" | "voice" => "android.permission.RECORD_AUDIO",
        "internet" | "network" | "api" | "online" | "sync" | "cloud" => {
            "android.permission.INTERNET"
        }
        "contacts" => "android.permission.READ_CONTACTS",
        "storage" | "files" | "photos" | "offline" => "android.permission.READ_EXTERNAL_STORAGE",
        "bluetooth" => "android.permission.BLUETOOTH",
        "biometric" | "fingerprint" => "android.permission.USE_BIOMETRIC",
        "sensor" | "accelerometer" | "pedometer" => "android.permission.BODY_SENSORS",
        "sms" | "message" => "android.permission.SEND_SMS",
        "phone" | "call" => "android.permission.CALL_PHONE",
        _ => return None,
    })
}

fn derive_permissions(contract: &AndroidConstructionContract) -> Vec<String> {
    let mut permissions: BTreeMap<String, ()> = BTreeMap::new();
    let mut statements: Vec<&str> = vec![contract.user_intent.as_str()];
    for requirement in contract
        .features
        .iter()
        .chain(contract.integrations.iter())
        .chain(contract.android_requirements.iter())
    {
        statements.push(requirement.statement.as_str());
    }
    for statement in statements {
        let normalized = statement.to_ascii_lowercase();
        for word in normalized.split_whitespace() {
            let cleaned: String = word
                .chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .collect();
            if let Some(permission) = permission_for_keyword(&cleaned) {
                permissions.insert(permission.to_owned(), ());
            }
        }
    }
    for profile in &contract.device_matrix {
        for permission in &profile.permissions {
            if !permission.trim().is_empty() {
                permissions.insert(permission.clone(), ());
            }
        }
    }
    permissions.into_keys().collect()
}

fn derive_min_sdk(contract: &AndroidConstructionContract) -> u32 {
    contract
        .device_matrix
        .iter()
        .map(|profile| profile.api_level)
        .min()
        .unwrap_or(DEFAULT_MIN_SDK)
        .clamp(21, 35)
}

fn derive_target_sdk(contract: &AndroidConstructionContract) -> u32 {
    contract
        .device_matrix
        .iter()
        .map(|profile| profile.api_level)
        .max()
        .unwrap_or(DEFAULT_TARGET_SDK)
        .clamp(21, 35)
}

fn sanitize_slug(value: &str) -> String {
    let slug: String = value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = slug.trim_matches('-').to_owned();
    let mut cleaned = String::new();
    let mut previous_dash = false;
    for character in trimmed.chars() {
        if character == '-' {
            if !previous_dash && !cleaned.is_empty() {
                cleaned.push('-');
            }
            previous_dash = true;
        } else {
            cleaned.push(character);
            previous_dash = false;
        }
    }
    if cleaned.is_empty() {
        "app".into()
    } else {
        cleaned
    }
}

fn derive_package_name(contract: &AndroidConstructionContract) -> String {
    let slug = sanitize_slug(&contract.project_id.0);
    let mut segments: Vec<String> = vec!["com".into(), "nirman".into()];
    for segment in slug.split('-') {
        if segment.is_empty() {
            continue;
        }
        let starts_with_digit = segment
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit());
        if starts_with_digit {
            // Java/Kotlin package segments cannot start with a digit; merge
            // the numeric segment into the previous one ("app" + "42" -> "app42"),
            // or prefix it when no previous segment can absorb it.
            match segments.last_mut() {
                Some(previous) if previous != "com" && previous != "nirman" => {
                    previous.push_str(segment);
                }
                _ => segments.push(format!("x{segment}")),
            }
            continue;
        }
        segments.push(segment.to_owned());
    }
    segments.join(".")
}

fn derive_application_name(contract: &AndroidConstructionContract) -> String {
    let intent_words: Vec<&str> = contract
        .user_intent
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .take(4)
        .collect();
    let mut name = intent_words.join(" ");
    for (pattern, replacement) in [
        ("build ", ""),
        ("build", ""),
        ("create ", ""),
        ("an ", ""),
        ("a ", ""),
        ("the ", ""),
    ] {
        if name.to_ascii_lowercase().starts_with(pattern) {
            let suffix = &name[pattern.len()..];
            name = replacement.to_owned() + suffix;
            break;
        }
    }
    let cleaned: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == ' ' || character == '-' {
                character
            } else {
                ' '
            }
        })
        .collect();
    let collapsed: Vec<&str> = cleaned.split_whitespace().collect();
    let joined = collapsed.join(" ");
    if joined.trim().is_empty() {
        "Nirman App".into()
    } else {
        let mut characters = joined.chars();
        match characters.next() {
            Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
            None => "Nirman App".into(),
        }
    }
}

fn pascal_case(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect();
    sanitized
        .split_whitespace()
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + &characters.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn screen_name(requirement: &ConstructionRequirement, index: usize) -> String {
    let derived = pascal_case(&requirement.statement);
    let base = if derived.chars().count() > 28 {
        let truncated: String = derived.chars().take(28).collect();
        truncated.trim_end().to_owned()
    } else if derived.is_empty() {
        format!("Feature{index}")
    } else {
        derived
    };
    let base = base.replace(' ', "");
    format!("{base}Screen")
}

fn route_name(requirement: &ConstructionRequirement, index: usize) -> String {
    let name = screen_name(requirement, index);
    let mut route = String::new();
    for (index, character) in name.char_indices() {
        if index == 0 {
            route.extend(character.to_lowercase());
        } else {
            route.push(character);
        }
    }
    route
}

fn derive_feature_screens(contract: &AndroidConstructionContract) -> Vec<FeatureScreen> {
    let mut screens = Vec::new();
    for (index, requirement) in contract.features.iter().enumerate() {
        screens.push(FeatureScreen {
            screen_id: format!("screen-{}", requirement.requirement_id),
            route: route_name(requirement, index + 1),
            title: title_from_statement(&requirement.statement),
            statement: requirement.statement.clone(),
            requirement_id: requirement.requirement_id.clone(),
        });
    }
    if screens.is_empty() {
        for (index, requirement) in contract.ui.iter().enumerate() {
            screens.push(FeatureScreen {
                screen_id: format!("screen-{}", requirement.requirement_id),
                route: route_name(requirement, index + 1),
                title: title_from_statement(&requirement.statement),
                statement: requirement.statement.clone(),
                requirement_id: requirement.requirement_id.clone(),
            });
        }
    }
    if screens.is_empty() {
        screens.push(FeatureScreen {
            screen_id: "screen-0001".into(),
            route: "homeScreen".into(),
            title: "Home".into(),
            statement: contract.user_intent.clone(),
            requirement_id: "derived-intent".into(),
        });
    }
    screens
}

fn title_from_statement(statement: &str) -> String {
    let words: Vec<&str> = statement.split_whitespace().take(4).collect();
    let joined = words.join(" ");
    if joined.is_empty() {
        "Feature".into()
    } else {
        let mut characters = joined.chars();
        match characters.next() {
            Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
            None => "Feature".into(),
        }
    }
}

fn escape_kotlin_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 8);
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '$' => escaped.push_str("\\$"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn escape_xml(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 8);
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn kotlin_string_literal(value: &str) -> String {
    format!("\"{}\"", escape_kotlin_string(value))
}

/// Generates a complete Android Gradle project from a validated contract and
/// its resolved technology plan. Pure: performs no filesystem access.
pub fn scaffold_android_project(
    contract: &AndroidConstructionContract,
    technology_plan: &AndroidTechnologyPlan,
) -> Result<AndroidProjectScaffold, ScaffoldError> {
    if contract.target_platforms != vec!["android".to_string()] {
        return Err(ScaffoldError::UnsupportedPlatform);
    }
    contract
        .validate()
        .map_err(|error| ScaffoldError::InvalidContract(error.to_string()))?;
    for (field, value) in [
        ("planId", technology_plan.plan_id.as_str()),
        ("language", technology_plan.language.as_str()),
        ("uiFramework", technology_plan.ui_framework.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ScaffoldError::EmptyField(field));
        }
    }
    let language = technology_plan.language.to_ascii_lowercase();
    let ui_framework = technology_plan.ui_framework.to_ascii_lowercase();
    if language != "kotlin" && language != "java" {
        return Err(ScaffoldError::UnsupportedLanguage(language));
    }
    if ui_framework != "jetpack-compose" && ui_framework != "android-views" {
        return Err(ScaffoldError::UnsupportedUiFramework(ui_framework));
    }
    // Jetpack Compose requires Kotlin; a Java contract falls back to Views.
    let (language, ui_framework) = if language == "java" && ui_framework == "jetpack-compose" {
        ("java".to_owned(), "android-views".to_owned())
    } else {
        (language, ui_framework)
    };

    let package_name = derive_package_name(contract);
    let application_name = derive_application_name(contract);
    let min_sdk = derive_min_sdk(contract);
    let target_sdk = derive_target_sdk(contract);
    let permissions = derive_permissions(contract);
    let feature_screens = derive_feature_screens(contract);
    if feature_screens.is_empty() {
        return Err(ScaffoldError::NoScreens);
    }

    let scaffold_id = format!("scaffold-{}", contract.contract_id);
    let package_path = package_name.replace('.', "/");
    let use_compose = ui_framework == "jetpack-compose";
    let is_java = language == "java";

    let mut files: Vec<ScaffoldFile> = Vec::new();
    let mut push =
        |path: String, contents: String, file_language: ScaffoldLanguage, purpose: &str| {
            let sha256 = format!("{:x}", Sha256::digest(contents.as_bytes()));
            files.push(ScaffoldFile {
                relative_path: path,
                contents,
                language: file_language,
                purpose: purpose.into(),
                sha256,
            });
        };

    // ---- Root Gradle project -------------------------------------------------
    push(
        "settings.gradle.kts".into(),
        settings_gradle(&application_name),
        ScaffoldLanguage::GradleKts,
        "Gradle settings: repositories and :app module inclusion",
    );
    push(
        "build.gradle.kts".into(),
        root_build_gradle(),
        ScaffoldLanguage::GradleKts,
        "Root Gradle build: Android/Kotlin plugin versions",
    );
    push(
        "gradle.properties".into(),
        gradle_properties(),
        ScaffoldLanguage::Properties,
        "Gradle and AndroidX build properties",
    );
    push(
        "gradle/wrapper/gradle-wrapper.properties".into(),
        gradle_wrapper_properties(),
        ScaffoldLanguage::Properties,
        "Pinned Gradle wrapper distribution",
    );
    push(
        ".gitignore".into(),
        android_gitignore(),
        ScaffoldLanguage::GitIgnore,
        "Version-control ignores for build outputs and IDE files",
    );

    // ---- :app module --------------------------------------------------------
    push(
        "app/build.gradle.kts".into(),
        app_build_gradle(&package_name, min_sdk, target_sdk, use_compose, is_java),
        ScaffoldLanguage::GradleKts,
        "Application module build configuration",
    );
    push(
        "app/proguard-rules.pro".into(),
        proguard_rules(),
        ScaffoldLanguage::Proguard,
        "Release shrinking keep rules",
    );
    push(
        "app/src/main/AndroidManifest.xml".into(),
        android_manifest(&application_name, &permissions),
        ScaffoldLanguage::Xml,
        "Android manifest with derived permissions and launcher activity",
    );

    // ---- Resources ----------------------------------------------------------
    push(
        "app/src/main/res/values/strings.xml".into(),
        strings_xml(&application_name, &feature_screens),
        ScaffoldLanguage::Xml,
        "Localized application strings",
    );
    push(
        "app/src/main/res/values/colors.xml".into(),
        colors_xml(),
        ScaffoldLanguage::Xml,
        "Application color palette",
    );
    push(
        "app/src/main/res/values/themes.xml".into(),
        themes_xml(use_compose),
        ScaffoldLanguage::Xml,
        "Application theme",
    );

    // ---- Sources ------------------------------------------------------------
    if use_compose {
        let main_activity_path = format!("app/src/main/java/{package_path}/MainActivity.kt");
        push(
            main_activity_path,
            compose_main_activity(&package_name, &application_name, &feature_screens),
            ScaffoldLanguage::Kotlin,
            "Jetpack Compose launcher activity with feature navigation",
        );
        let theme_path = format!("app/src/main/java/{package_path}/ui/Theme.kt");
        push(
            theme_path,
            compose_theme(&package_name),
            ScaffoldLanguage::Kotlin,
            "Material3 Compose theme",
        );
        for screen in &feature_screens {
            let path = format!("app/src/main/java/{package_path}/ui/{}.kt", screen.route);
            push(
                path,
                compose_screen(&package_name, screen),
                ScaffoldLanguage::Kotlin,
                "Feature screen rendered from a contract requirement",
            );
        }
    } else {
        push(
            "app/src/main/res/layout/activity_main.xml".into(),
            views_main_layout(&application_name),
            ScaffoldLanguage::Xml,
            "Main activity layout with feature list",
        );
        push(
            "app/src/main/res/layout/item_feature.xml".into(),
            views_feature_item_layout(),
            ScaffoldLanguage::Xml,
            "Feature list item layout",
        );
        if is_java {
            let path = format!("app/src/main/java/{package_path}/MainActivity.java");
            push(
                path,
                java_main_activity(&package_name),
                ScaffoldLanguage::Java,
                "Launcher activity hosting the feature list",
            );
        } else {
            let path = format!("app/src/main/java/{package_path}/MainActivity.kt");
            push(
                path,
                views_main_activity(&package_name),
                ScaffoldLanguage::Kotlin,
                "Launcher activity hosting the feature list",
            );
        }
    }

    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let scaffold_fingerprint = scaffold_fingerprint(&files);
    Ok(AndroidProjectScaffold {
        schema_version: SCAFFOLD_SCHEMA_VERSION,
        scaffold_id,
        contract_id: contract.contract_id.clone(),
        project_id: contract.project_id.0.clone(),
        task_id: contract.task_id.0.clone(),
        package_name,
        application_name,
        language,
        ui_framework,
        min_sdk,
        target_sdk,
        compile_sdk: COMPILE_SDK,
        version_code: 1,
        version_name: "1.0.0".into(),
        permissions,
        feature_screens,
        files,
        scaffold_fingerprint,
    })
}

fn scaffold_fingerprint(files: &[ScaffoldFile]) -> String {
    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(file.relative_path.as_bytes());
        hasher.update(b"\0");
        hasher.update(file.sha256.as_bytes());
        hasher.update(b"\0");
    }
    format!("sha256:{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Gradle templates
// ---------------------------------------------------------------------------

fn settings_gradle(application_name: &str) -> String {
    format!(
        "pluginManagement {{\n    repositories {{\n        google()\n        mavenCentral()\n        gradlePluginPortal()\n    }}\n}}\ndependencyResolutionManagement {{\n    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)\n    repositories {{\n        google()\n        mavenCentral()\n    }}\n}}\n\nrootProject.name = \"{}\"\ninclude(\":app\")\n",
        escape_kotlin_string(application_name)
    )
}

fn root_build_gradle() -> String {
    format!(
        "// Generated by Nirman. Plugin versions are pinned by the toolchain lock.\nplugins {{\n    id(\"com.android.application\") version \"{AGP_VERSION}\" apply false\n    id(\"org.jetbrains.kotlin.android\") version \"{KOTLIN_VERSION}\" apply false\n}}\n"
    )
}

fn gradle_properties() -> String {
    "org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8\norg.gradle.parallel=true\nandroid.useAndroidX=true\nandroid.nonTransitiveRClass=true\nkotlin.code.style=official\n"
        .into()
}

fn gradle_wrapper_properties() -> String {
    format!(
        "distributionBase=GRADLE_USER_HOME\ndistributionPath=wrapper/dists\ndistributionUrl=https\\://services.gradle.org/distributions/gradle-{GRADLE_VERSION}-bin.zip\nnetworkTimeout=10000\nvalidateDistributionUrl=true\nzipStoreBase=GRADLE_USER_HOME\nzipStorePath=wrapper/dists\n"
    )
}

fn android_gitignore() -> String {
    "*.iml\n.gradle\n/local.properties\n/.idea\n.DS_Store\n/build\n/captures\n.externalNativeBuild\n.cxx\nlocal.properties\napp/build\n"
        .into()
}

#[allow(clippy::too_many_arguments)]
fn app_build_gradle(
    package_name: &str,
    min_sdk: u32,
    target_sdk: u32,
    use_compose: bool,
    is_java: bool,
) -> String {
    let plugins = if is_java {
        "plugins {\n    id(\"com.android.application\")\n}\n".to_owned()
    } else {
        "plugins {\n    id(\"com.android.application\")\n    id(\"org.jetbrains.kotlin.android\")\n}\n"
            .to_owned()
    };
    let compose_block = if use_compose {
        format!(
            "    buildFeatures {{\n        compose = true\n    }}\n    composeOptions {{\n        kotlinCompilerExtensionVersion = \"1.5.15\"\n    }}\n"
        )
    } else {
        String::new()
    };
    let dependencies = if use_compose {
        format!(
            "dependencies {{\n    implementation(\"androidx.core:core-ktx:{CORE_KTX_VERSION}\")\n    implementation(\"androidx.lifecycle:lifecycle-runtime-ktx:{LIFECYCLE_VERSION}\")\n    implementation(\"androidx.activity:activity-compose:{ACTIVITY_COMPOSE_VERSION}\")\n    implementation(platform(\"androidx.compose:compose-bom:{COMPOSE_BOM_VERSION}\"))\n    implementation(\"androidx.compose.ui:ui\")\n    implementation(\"androidx.compose.ui:ui-graphics\")\n    implementation(\"androidx.compose.ui:ui-tooling-preview\")\n    implementation(\"androidx.compose.material3:material3\")\n    debugImplementation(\"androidx.compose.ui:ui-tooling\")\n}}\n"
        )
    } else if is_java {
        format!(
            "dependencies {{\n    implementation(\"androidx.appcompat:appcompat:{APPCOMPAT_VERSION}\")\n    implementation(\"com.google.android.material:material:{MATERIAL_VERSION}\")\n    implementation(\"androidx.constraintlayout:constraintlayout:2.1.4\")\n}}\n"
        )
    } else {
        format!(
            "dependencies {{\n    implementation(\"androidx.core:core-ktx:{CORE_KTX_VERSION}\")\n    implementation(\"androidx.appcompat:appcompat:{APPCOMPAT_VERSION}\")\n    implementation(\"com.google.android.material:material:{MATERIAL_VERSION}\")\n    implementation(\"androidx.constraintlayout:constraintlayout:2.1.4\")\n}}\n"
        )
    };
    let packaging = if use_compose {
        "    packaging {\n        resources {\n            excludes += \"/META-INF/{AL2.0,LGPL2.1}\"\n        }\n    }\n"
    } else {
        ""
    };
    format!(
        "{plugins}\nandroid {{\n    namespace = \"{package_name}\"\n    compileSdk = {COMPILE_SDK}\n\n    defaultConfig {{\n        applicationId = \"{package_name}\"\n        minSdk = {min_sdk}\n        targetSdk = {target_sdk}\n        versionCode = 1\n        versionName = \"1.0.0\"\n    }}\n\n    buildTypes {{\n        release {{\n            isMinifyEnabled = false\n            proguardFiles(\n                getDefaultProguardFile(\"proguard-android-optimize.txt\"),\n                \"proguard-rules.pro\"\n            )\n        }}\n    }}\n{compose_block}{packaging}    compileOptions {{\n        sourceCompatibility = JavaVersion.VERSION_17\n        targetCompatibility = JavaVersion.VERSION_17\n    }}\n    kotlinOptions {{\n        jvmTarget = \"17\"\n    }}\n}}\n\n{dependencies}"
    )
}

fn proguard_rules() -> String {
    "# Generated by Nirman.\n# Add application-specific keep rules here.\n".into()
}

// ---------------------------------------------------------------------------
// Manifest and resources
// ---------------------------------------------------------------------------

fn android_manifest(application_name: &str, permissions: &[String]) -> String {
    let mut manifest = String::new();
    manifest.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    manifest.push_str("<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\">\n");
    for permission in permissions {
        manifest.push_str(&format!(
            "    <uses-permission android:name=\"{}\" />\n",
            escape_xml(permission)
        ));
    }
    manifest.push_str("\n    <application\n");
    manifest.push_str("        android:allowBackup=\"true\"\n");
    manifest.push_str("        android:label=\"@string/app_name\"\n");
    manifest.push_str("        android:supportsRtl=\"true\"\n");
    manifest.push_str("        android:theme=\"@style/Theme.NirmanApp\">\n");
    manifest.push_str(&format!("        <!-- {application_name} -->\n"));
    manifest.push_str("        <activity\n");
    manifest.push_str("            android:name=\".MainActivity\"\n");
    manifest.push_str("            android:exported=\"true\">\n");
    manifest.push_str("            <intent-filter>\n");
    manifest.push_str("                <action android:name=\"android.intent.action.MAIN\" />\n");
    manifest.push_str(
        "                <category android:name=\"android.intent.category.LAUNCHER\" />\n",
    );
    manifest.push_str("            </intent-filter>\n");
    manifest.push_str("        </activity>\n");
    manifest.push_str("    </application>\n");
    manifest.push_str("</manifest>\n");
    manifest
}

fn strings_xml(application_name: &str, screens: &[FeatureScreen]) -> String {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    xml.push_str("<resources>\n");
    xml.push_str(&format!(
        "    <string name=\"app_name\">{}</string>\n",
        escape_xml(application_name)
    ));
    for screen in screens {
        xml.push_str(&format!(
            "    <string name=\"screen_{}_title\">{}</string>\n",
            escape_xml(&screen.route),
            escape_xml(&screen.title)
        ));
    }
    xml.push_str("</resources>\n");
    xml
}

fn colors_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <color name=\"nirman_primary\">#0B57D0</color>\n    <color name=\"nirman_on_primary\">#FFFFFF</color>\n    <color name=\"nirman_secondary\">#E8F0FE</color>\n    <color name=\"nirman_surface\">#FDFBFF</color>\n</resources>\n".into()
}

fn themes_xml(use_compose: bool) -> String {
    if use_compose {
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <style name=\"Theme.NirmanApp\" parent=\"android:Theme.Material.Light.NoActionBar\" />\n</resources>\n".into()
    } else {
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n    <style name=\"Theme.NirmanApp\" parent=\"Theme.Material3.DayNight.NoActionBar\">\n        <item name=\"colorPrimary\">@color/nirman_primary</item>\n        <item name=\"colorOnPrimary\">@color/nirman_on_primary</item>\n        <item name=\"android:statusBarColor\">?attr/colorPrimary</item>\n    </style>\n</resources>\n".into()
    }
}

// ---------------------------------------------------------------------------
// Jetpack Compose sources
// ---------------------------------------------------------------------------

fn compose_main_activity(
    package_name: &str,
    application_name: &str,
    screens: &[FeatureScreen],
) -> String {
    let mut imports = String::new();
    imports.push_str(&format!(
        "package {package_name}\n\nimport android.os.Bundle\nimport androidx.activity.ComponentActivity\nimport androidx.activity.compose.setContent\nimport androidx.compose.foundation.layout.Arrangement\nimport androidx.compose.foundation.layout.Column\nimport androidx.compose.foundation.layout.fillMaxSize\nimport androidx.compose.foundation.layout.padding\nimport androidx.compose.foundation.lazy.LazyColumn\nimport androidx.compose.foundation.lazy.items\nimport androidx.compose.material3.ExperimentalMaterial3Api\nimport androidx.compose.material3.MaterialTheme\nimport androidx.compose.material3.Scaffold\nimport androidx.compose.material3.Text\nimport androidx.compose.material3.TopAppBar\nimport androidx.compose.runtime.Composable\nimport androidx.compose.runtime.mutableStateOf\nimport androidx.compose.runtime.remember\nimport androidx.compose.runtime.getValue\nimport androidx.compose.runtime.setValue\nimport androidx.compose.ui.Modifier\nimport androidx.compose.ui.unit.dp\nimport {package_name}.ui.theme.NirmanAppTheme\n"
    ));
    for screen in screens {
        imports.push_str(&format!("import {package_name}.ui.{}\n", screen.route));
    }
    let mut body = String::new();
    body.push_str(&format!(
        "\nclass MainActivity : ComponentActivity() {{\n    override fun onCreate(savedInstanceState: Bundle?) {{\n        super.onCreate(savedInstanceState)\n        setContent {{\n            NirmanAppTheme {{\n                NirmanApp()\n            }}\n        }}\n    }}\n}}\n\n@OptIn(ExperimentalMaterial3Api::class)\n@Composable\nfun NirmanApp() {{\n    var currentRoute by remember {{ mutableStateOf(\"home\") }}\n    Scaffold(\n        topBar = {{\n            TopAppBar(\n                title = {{ Text({}) }}\n            )\n        }},\n    ) {{ innerPadding ->\n        Column(\n            modifier = Modifier\n                .fillMaxSize()\n                .padding(innerPadding)\n        ) {{\n            when (currentRoute) {{\n",
        kotlin_string_literal(application_name)
    ));
    for screen in screens {
        body.push_str(&format!(
            "                \"{}\" -> {}(onBack = {{ currentRoute = \"home\" }})\n",
            escape_kotlin_string(&screen.route),
            pascal_case(&screen.route)
        ));
    }
    body.push_str("                else -> HomeScreen(\n                    screens = screenRoutes,\n                    onScreenSelected = { route -> currentRoute = route },\n                )\n");
    body.push_str("            }\n        }\n    }\n}\n\nprivate val screenRoutes = listOf(\n");
    for screen in screens {
        body.push_str(&format!(
            "    {} to {},\n",
            kotlin_string_literal(&screen.route),
            kotlin_string_literal(&screen.title)
        ));
    }
    body.push_str(
        ")\n\n@Composable\nfun HomeScreen(\n    screens: List<Pair<String, String>>,\n    onScreenSelected: (String) -> Unit,\n) {\n    LazyColumn(\n        verticalArrangement = Arrangement.spacedBy(8.dp),\n        modifier = Modifier.padding(16.dp),\n    ) {\n        items(screens) { (route, title) ->\n            Text(\n                text = title,\n                style = MaterialTheme.typography.titleMedium,\n                modifier = Modifier\n                    .padding(vertical = 12.dp)\n                    .clickable { onScreenSelected(route) },\n            )\n        }\n    }\n}\n",
    );
    // `clickable` import needed by HomeScreen.
    imports.push_str("import androidx.compose.foundation.clickable\n");
    format!("{imports}\n{body}")
}

fn compose_theme(package_name: &str) -> String {
    format!(
        "package {package_name}.ui.theme\n\nimport androidx.compose.foundation.isSystemInDarkTheme\nimport androidx.compose.material3.MaterialTheme\nimport androidx.compose.material3.darkColorScheme\nimport androidx.compose.material3.lightColorScheme\nimport androidx.compose.runtime.Composable\nimport androidx.compose.ui.graphics.Color\n\nprivate val LightColors = lightColorScheme(\n    primary = Color(0xFF0B57D0),\n    onPrimary = Color(0xFFFFFFFF),\n    secondary = Color(0xFFE8F0FE),\n    surface = Color(0xFFFDFBFF),\n)\n\nprivate val DarkColors = darkColorScheme(\n    primary = Color(0xFFA8C7FA),\n    onPrimary = Color(0xFF002E69),\n    secondary = Color(0xFF455A64),\n    surface = Color(0xFF111C2E),\n)\n\n@Composable\nfun NirmanAppTheme(\n    darkTheme: Boolean = isSystemInDarkTheme(),\n    content: @Composable () -> Unit,\n) {{\n    val colors = if (darkTheme) DarkColors else LightColors\n    MaterialTheme(\n        colorScheme = colors,\n        content = content,\n    )\n}}\n"
    )
}

fn compose_screen(package_name: &str, screen: &FeatureScreen) -> String {
    let screen_type = pascal_case(&screen.route);
    format!(
        "package {package_name}.ui\n\nimport androidx.compose.foundation.layout.Arrangement\nimport androidx.compose.foundation.layout.Column\nimport androidx.compose.foundation.layout.fillMaxSize\nimport androidx.compose.foundation.layout.padding\nimport androidx.compose.material3.MaterialTheme\nimport androidx.compose.material3.Text\nimport androidx.compose.material3.Button\nimport androidx.compose.runtime.Composable\nimport androidx.compose.ui.Modifier\nimport androidx.compose.ui.unit.dp\n\n@Composable\nfun {screen_type}(\n    onBack: () -> Unit,\n) {{\n    Column(\n        verticalArrangement = Arrangement.spacedBy(16.dp),\n        modifier = Modifier\n            .fillMaxSize()\n            .padding(16.dp),\n    ) {{\n        Text(\n            text = {},\n            style = MaterialTheme.typography.headlineSmall,\n        )\n        Text(\n            text = {},\n            style = MaterialTheme.typography.bodyMedium,\n        )\n        Button(onClick = onBack) {{\n            Text(text = \"Back\")\n        }}\n    }}\n}}\n",
        kotlin_string_literal(&screen.title),
        kotlin_string_literal(&screen.statement),
    )
}

// ---------------------------------------------------------------------------
// Android Views sources
// ---------------------------------------------------------------------------

fn views_main_layout(application_name: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<androidx.coordinatorlayout.widget.CoordinatorLayout\n    xmlns:android=\"http://schemas.android.com/apk/res/android\"\n    xmlns:app=\"http://schemas.android.com/apk/res-auto\"\n    android:layout_width=\"match_parent\"\n    android:layout_height=\"match_parent\">\n\n    <com.google.android.material.appbar.AppBarLayout\n        android:layout_width=\"match_parent\"\n        android:layout_height=\"wrap_content\">\n        <com.google.android.material.appbar.MaterialToolbar\n            android:id=\"@+id/toolbar\"\n            android:layout_width=\"match_parent\"\n            android:layout_height=\"?attr/actionBarSize\"\n            app:title=\"{}\" />\n    </com.google.android.material.appbar.AppBarLayout>\n\n    <androidx.recyclerview.widget.RecyclerView\n        android:id=\"@+id/feature_list\"\n        android:layout_width=\"match_parent\"\n        android:layout_height=\"match_parent\"\n        app:layout_behavior=\"@string/appbar_scrolling_view_behavior\" />\n\n</androidx.coordinatorlayout.widget.CoordinatorLayout>\n",
        escape_xml(application_name)
    )
}

fn views_feature_item_layout() -> String {
    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<com.google.android.material.card.MaterialCardView\n    xmlns:android=\"http://schemas.android.com/apk/res/android\"\n    android:layout_width=\"match_parent\"\n    android:layout_height=\"wrap_content\"\n    android:layout_margin=\"8dp\">\n    <TextView\n        android:id=\"@+id/feature_title\"\n        android:layout_width=\"match_parent\"\n        android:layout_height=\"wrap_content\"\n        android:padding=\"16dp\"\n        android:textAppearance=\"?attr/textAppearanceTitleMedium\" />\n</com.google.android.material.card.MaterialCardView>\n".into()
}

fn views_main_activity(package_name: &str) -> String {
    let mut source = String::new();
    source.push_str(&format!(
        "package {package_name}\n\nimport android.os.Bundle\nimport android.view.LayoutInflater\nimport android.view.ViewGroup\nimport android.widget.TextView\nimport androidx.appcompat.app.AppCompatActivity\nimport androidx.recyclerview.widget.LinearLayoutManager\nimport androidx.recyclerview.widget.RecyclerView\nimport {package_name}.databinding.ActivityMainBinding\n\n"
    ));
    source.push_str(
        "class MainActivity : AppCompatActivity() {\n    override fun onCreate(savedInstanceState: Bundle?) {\n        super.onCreate(savedInstanceState)\n        val binding = ActivityMainBinding.inflate(layoutInflater)\n        setContentView(binding.root)\n        binding.featureList.layoutManager = LinearLayoutManager(this)\n        binding.featureList.adapter = FeatureAdapter(featureTitles())\n    }\n\n    private fun featureTitles(): List<String> = listOf(\n",
    );
    // Feature titles come from string resources so translations stay native.
    source.push_str("        getString(R.string.app_name),\n");
    source.push_str("    )\n}\n\nclass FeatureAdapter(\n    private val titles: List<String>,\n) : RecyclerView.Adapter<FeatureAdapter.FeatureViewHolder>() {\n    class FeatureViewHolder(view: android.view.View) : RecyclerView.ViewHolder(view) {\n        val title: TextView = view.findViewById(R.id.feature_title)\n    }\n\n    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): FeatureViewHolder {\n        val view = LayoutInflater.from(parent.context)\n            .inflate(R.layout.item_feature, parent, false)\n        return FeatureViewHolder(view)\n    }\n\n    override fun onBindViewHolder(holder: FeatureViewHolder, position: Int) {\n        holder.title.text = titles[position]\n    }\n\n    override fun getItemCount(): Int = titles.size\n}\n");
    source
}

fn java_main_activity(package_name: &str) -> String {
    format!(
        "package {package_name};\n\nimport android.os.Bundle;\nimport android.view.LayoutInflater;\nimport android.view.ViewGroup;\nimport android.widget.TextView;\nimport androidx.appcompat.app.AppCompatActivity;\nimport androidx.recyclerview.widget.LinearLayoutManager;\nimport androidx.recyclerview.widget.RecyclerView;\n\npublic class MainActivity extends AppCompatActivity {{\n    @Override\n    protected void onCreate(Bundle savedInstanceState) {{\n        super.onCreate(savedInstanceState);\n        setContentView(R.layout.activity_main);\n        RecyclerView featureList = findViewById(R.id.feature_list);\n        featureList.setLayoutManager(new LinearLayoutManager(this));\n        featureList.setAdapter(new FeatureAdapter(new String[]{{getString(R.string.app_name)}}));\n    }}\n\n    private static final class FeatureAdapter extends RecyclerView.Adapter<FeatureAdapter.FeatureViewHolder> {{\n        private final String[] titles;\n\n        FeatureAdapter(String[] titles) {{\n            this.titles = titles;\n        }}\n\n        static final class FeatureViewHolder extends RecyclerView.ViewHolder {{\n            final TextView title;\n\n            FeatureViewHolder(android.view.View view) {{\n                super(view);\n                title = view.findViewById(R.id.feature_title);\n            }}\n        }}\n\n        @Override\n        public FeatureViewHolder onCreateViewHolder(ViewGroup parent, int viewType) {{\n            android.view.View view = LayoutInflater.from(parent.getContext())\n                    .inflate(R.layout.item_feature, parent, false);\n            return new FeatureViewHolder(view);\n        }}\n\n        @Override\n        public void onBindViewHolder(FeatureViewHolder holder, int position) {{\n            holder.title.setText(titles[position]);\n        }}\n\n        @Override\n        public int getItemCount() {{\n            return titles.length;\n        }}\n    }}\n}}\n"
    )
}

// ---------------------------------------------------------------------------
// Application and validation
// ---------------------------------------------------------------------------

impl AndroidProjectScaffold {
    /// Validates every generated file path is relative, normalised, and
    /// contained within the workspace when applied.
    pub fn validate(&self) -> Result<(), ScaffoldError> {
        if self.schema_version != SCAFFOLD_SCHEMA_VERSION {
            return Err(ScaffoldError::InvalidContract(
                "unsupported scaffold schema version".into(),
            ));
        }
        for (field, value) in [
            ("scaffoldId", self.scaffold_id.as_str()),
            ("contractId", self.contract_id.as_str()),
            ("projectId", self.project_id.as_str()),
            ("taskId", self.task_id.as_str()),
            ("packageName", self.package_name.as_str()),
            ("applicationName", self.application_name.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ScaffoldError::EmptyField(field));
            }
        }
        if self.files.is_empty() {
            return Err(ScaffoldError::EmptyField("files"));
        }
        for file in &self.files {
            validate_relative_path(&file.relative_path)?;
            if file.contents.is_empty() {
                return Err(ScaffoldError::EmptyField("file contents"));
            }
        }
        let fingerprint = scaffold_fingerprint(&self.files);
        if fingerprint != self.scaffold_fingerprint {
            return Err(ScaffoldError::InvalidContract(
                "scaffold fingerprint does not match generated files".into(),
            ));
        }
        Ok(())
    }

    /// Writes every scaffold file under `workspace_root`, creating parent
    /// directories as needed. Each path is re-validated against workspace
    /// escape before the first byte is written.
    pub fn apply(&self, workspace_root: &Path) -> Result<Vec<ScaffoldFile>, ScaffoldError> {
        self.validate()?;
        let canonical_root = workspace_root
            .canonicalize()
            .map_err(|_| ScaffoldError::WriteFailed(workspace_root.to_string_lossy().into()))?;
        if !canonical_root.is_dir() {
            return Err(ScaffoldError::WriteFailed(
                canonical_root.to_string_lossy().into(),
            ));
        }
        let mut written = Vec::with_capacity(self.files.len());
        for file in &self.files {
            validate_relative_path(&file.relative_path)?;
            let destination = canonical_root.join(&file.relative_path);
            let parent = destination
                .parent()
                .ok_or_else(|| ScaffoldError::InvalidPath(file.relative_path.clone()))?
                .to_path_buf();
            // Path safety is already guaranteed by validate_relative_path
            // (no traversal, no absolute, no symlink components); directories
            // are created before the defensive canonical containment check.
            fs::create_dir_all(&parent)
                .map_err(|_| ScaffoldError::WriteFailed(file.relative_path.clone()))?;
            let canonical_parent = parent
                .canonicalize()
                .map_err(|_| ScaffoldError::InvalidPath(file.relative_path.clone()))?;
            if !canonical_parent.starts_with(&canonical_root) {
                return Err(ScaffoldError::OutsideWorkspace(file.relative_path.clone()));
            }
            fs::write(&destination, &file.contents)
                .map_err(|_| ScaffoldError::WriteFailed(file.relative_path.clone()))?;
            written.push(file.clone());
        }
        Ok(written)
    }

    pub fn summary(&self, resulting_project_fingerprint: &str) -> ScaffoldSummary {
        ScaffoldSummary {
            scaffold_id: self.scaffold_id.clone(),
            contract_id: self.contract_id.clone(),
            project_id: self.project_id.clone(),
            task_id: self.task_id.clone(),
            package_name: self.package_name.clone(),
            application_name: self.application_name.clone(),
            language: self.language.clone(),
            ui_framework: self.ui_framework.clone(),
            min_sdk: self.min_sdk,
            target_sdk: self.target_sdk,
            compile_sdk: self.compile_sdk,
            version_code: self.version_code,
            version_name: self.version_name.clone(),
            permissions: self.permissions.clone(),
            feature_screens: self
                .feature_screens
                .iter()
                .map(|screen| screen.route.clone())
                .collect(),
            file_count: self.files.len(),
            scaffold_fingerprint: self.scaffold_fingerprint.clone(),
            resulting_project_fingerprint: resulting_project_fingerprint.into(),
        }
    }

    pub fn language_labels(&self) -> BTreeMap<String, &'static str> {
        self.files
            .iter()
            .map(|file| (file.relative_path.clone(), file.language.as_str()))
            .collect()
    }
}

fn validate_relative_path(path: &str) -> Result<(), ScaffoldError> {
    if path.trim().is_empty() {
        return Err(ScaffoldError::InvalidPath(path.into()));
    }
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Err(ScaffoldError::InvalidPath(path.into()));
    }
    for component in candidate.components() {
        match component {
            std::path::Component::Normal(_) => {}
            _ => return Err(ScaffoldError::InvalidPath(path.into())),
        }
    }
    if path.contains('\\') || path.contains("..") || path.starts_with('/') {
        return Err(ScaffoldError::InvalidPath(path.into()));
    }
    Ok(())
}

impl FeatureScreen {
    pub fn screen_type_name(&self) -> String {
        pascal_case(&self.route)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nirman_domain::{
        AndroidDeviceProfile, AndroidTechnologyPlan as DomainTechnologyPlan, ArtifactKind,
        ArtifactModel, ConstructionRequirement, ProjectId, RequirementOrigin, TaskId,
        ValidationModel, VisualReferenceInput,
    };

    fn requirement(id: &str, statement: &str) -> ConstructionRequirement {
        ConstructionRequirement {
            requirement_id: id.into(),
            statement: statement.into(),
            origin: RequirementOrigin::UserFact,
            source_reference_ids: vec![],
        }
    }

    fn contract() -> AndroidConstructionContract {
        AndroidConstructionContract {
            schema_version: 1,
            contract_id: "contract-m4b".into(),
            project_id: ProjectId("notes-app-42".into()),
            target_platforms: vec!["android".into()],
            task_id: TaskId("task-m4b".into()),
            user_intent: "Build an offline-first notes app with camera capture".into(),
            screenshots: vec![VisualReferenceInput {
                reference_id: "ref-1".into(),
                source_path: "inputs/screen.png".into(),
                image_hash: "sha256:screen".into(),
            }],
            assets: vec![],
            features: vec![
                requirement("req-notes", "Create and edit notes"),
                requirement("req-camera", "Attach camera photos to a note"),
            ],
            ui: vec![requirement("req-ui-list", "Show a list of saved notes")],
            data: vec![],
            integrations: vec![requirement("req-sync", "Sync notes over the network")],
            technology_plan: DomainTechnologyPlan {
                plan_id: "plan-m4b".into(),
                task_id: TaskId("task-m4b".into()),
                requested_capabilities: vec!["notes".into()],
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
                required_toolchains: vec!["jdk".into()],
                validation_plan: vec!["compile".into()],
                confidence: None,
                revision: nirman_domain::Revision(1),
            },
            android_requirements: vec![],
            device_matrix: vec![AndroidDeviceProfile {
                device_id: "device-1".into(),
                name: "Pixel 8".into(),
                platform_version: "14".into(),
                api_level: 34,
                architecture: "arm64-v8a".into(),
                width: 1080,
                height: 2400,
                density: 420,
                orientation: "portrait".into(),
                locale: "en-US".into(),
                permissions: vec![],
                network_profile: "wifi".into(),
            }],
            validation_model: ValidationModel {
                required_checks: vec!["compile".into()],
                acceptance_criteria: vec!["application builds an APK".into()],
            },
            artifact_model: ArtifactModel {
                required_artifact: ArtifactKind::Apk,
                aab_declared: false,
            },
        }
    }

    fn plan(language: &str, ui_framework: &str) -> AndroidTechnologyPlan {
        AndroidTechnologyPlan {
            schema_version: 1,
            plan_id: "plan-m4b".into(),
            language: language.into(),
            ui_framework: ui_framework.into(),
            data_strategy: "local-first".into(),
            source_revision: 1,
            rationale: "test".into(),
        }
    }

    #[test]
    fn compose_scaffold_generates_complete_project() {
        let scaffold = scaffold_android_project(&contract(), &plan("kotlin", "jetpack-compose"))
            .expect("scaffold");
        scaffold.validate().expect("valid scaffold");
        assert_eq!(scaffold.package_name, "com.nirman.notes.app42");
        assert_eq!(scaffold.min_sdk, 34);
        assert_eq!(scaffold.target_sdk, 34);
        assert_eq!(scaffold.ui_framework, "jetpack-compose");

        let paths: Vec<&str> = scaffold
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect();
        for expected in [
            "settings.gradle.kts",
            "build.gradle.kts",
            "gradle.properties",
            "gradle/wrapper/gradle-wrapper.properties",
            "app/build.gradle.kts",
            "app/proguard-rules.pro",
            "app/src/main/AndroidManifest.xml",
            "app/src/main/res/values/strings.xml",
            "app/src/main/res/values/colors.xml",
            "app/src/main/res/values/themes.xml",
        ] {
            assert!(
                paths.contains(&expected),
                "missing scaffold file: {expected}"
            );
        }
        assert!(paths.contains(&"app/src/main/java/com/nirman/notes/app42/MainActivity.kt"));
        assert!(paths.contains(&"app/src/main/java/com/nirman/notes/app42/ui/Theme.kt"));

        // Camera + internet permissions derived from requirements.
        assert!(scaffold
            .permissions
            .contains(&"android.permission.CAMERA".to_owned()));
        assert!(scaffold
            .permissions
            .contains(&"android.permission.INTERNET".to_owned()));
        let manifest = scaffold
            .files
            .iter()
            .find(|file| file.relative_path == "app/src/main/AndroidManifest.xml")
            .unwrap();
        assert!(manifest.contents.contains("android.permission.CAMERA"));
        assert!(manifest.contents.contains("android:name=\".MainActivity\""));

        // Every feature screen has a Composable referenced from MainActivity.
        let main_activity = scaffold
            .files
            .iter()
            .find(|file| {
                file.relative_path == "app/src/main/java/com/nirman/notes/app42/MainActivity.kt"
            })
            .unwrap();
        for screen in &scaffold.feature_screens {
            assert!(main_activity
                .contents
                .contains(&format!("\"{}\"", screen.route)));
            let screen_path = format!(
                "app/src/main/java/com/nirman/notes/app42/ui/{}.kt",
                screen.route
            );
            assert!(paths.contains(&screen_path.as_str()));
        }
    }

    #[test]
    fn views_scaffold_generates_java_project_when_java_requested() {
        let scaffold = scaffold_android_project(&contract(), &plan("java", "jetpack-compose"))
            .expect("scaffold");
        assert_eq!(scaffold.language, "java");
        assert_eq!(scaffold.ui_framework, "android-views");
        assert!(scaffold.files.iter().any(|file| file.relative_path
            == "app/src/main/java/com/nirman/notes/app42/MainActivity.java"));
        assert!(scaffold
            .files
            .iter()
            .any(|file| file.relative_path == "app/src/main/res/layout/activity_main.xml"));
    }

    #[test]
    fn scaffold_is_deterministic() {
        let first = scaffold_android_project(&contract(), &plan("kotlin", "jetpack-compose"))
            .expect("first");
        let second = scaffold_android_project(&contract(), &plan("kotlin", "jetpack-compose"))
            .expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn scaffold_rejects_unsafe_paths_and_bad_contracts() {
        let error = scaffold_android_project(&contract(), &plan("python", "jetpack-compose"))
            .expect_err("unsupported language");
        assert!(matches!(error, ScaffoldError::UnsupportedLanguage(_)));
        let error = scaffold_android_project(&contract(), &plan("kotlin", "flutter"))
            .expect_err("unsupported framework");
        assert!(matches!(error, ScaffoldError::UnsupportedUiFramework(_)));

        let mut non_android = contract();
        non_android.target_platforms = vec!["ios".into()];
        let error = scaffold_android_project(&non_android, &plan("kotlin", "jetpack-compose"))
            .expect_err("platform");
        assert_eq!(error, ScaffoldError::UnsupportedPlatform);

        assert!(validate_relative_path("../escape.kt").is_err());
        assert!(validate_relative_path("/absolute.kt").is_err());
        assert!(validate_relative_path("a/../../escape.kt").is_err());
        assert!(validate_relative_path("app/src/main/AndroidManifest.xml").is_ok());
    }

    #[test]
    fn apply_writes_files_and_reports_workspace_escape() {
        let scaffold = scaffold_android_project(&contract(), &plan("kotlin", "jetpack-compose"))
            .expect("scaffold");
        let root = std::env::temp_dir().join(format!(
            "nirman-scaffold-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("root");
        let written = scaffold.apply(&root).expect("applied");
        assert_eq!(written.len(), scaffold.files.len());
        let manifest = root.join("app/src/main/AndroidManifest.xml");
        assert!(manifest.is_file());
        let settings = std::fs::read_to_string(root.join("settings.gradle.kts")).unwrap();
        assert!(settings.contains("include(\":app\")"));
        assert!(settings.contains("rootProject.name ="));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn user_content_is_escaped_in_kotlin_and_xml() {
        let mut malicious = contract();
        malicious.user_intent = "Notes with \\ \"quotes\" and $dollar and <tag>".into();
        let scaffold = scaffold_android_project(&malicious, &plan("kotlin", "jetpack-compose"))
            .expect("scaffold");
        for file in &scaffold.files {
            if matches!(file.language, ScaffoldLanguage::Kotlin) {
                assert!(!file.contents.contains("$dollar"));
            }
            if matches!(file.language, ScaffoldLanguage::Xml) {
                assert!(!file.contents.contains("<tag>"));
            }
        }
    }
}
