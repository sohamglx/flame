use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustPluginsMode {
    Default,
    Deny,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlameManifest {
    #[serde(default)]
    pub package: Option<PackageConfig>,
    #[serde(default)]
    pub lib: Option<LibConfig>,
    #[serde(default)]
    pub application: Option<ApplicationConfig>,
    #[serde(default)]
    pub options: Option<OptionsConfig>,
    #[serde(default)]
    pub permissions: Option<toml::Value>,
    #[serde(default)]
    pub dependencies: Option<HashMap<String, toml::Value>>,
    #[serde(rename = "native-dependencies", default)]
    pub native_dependencies: Option<HashMap<String, toml::Value>>,
    #[serde(default)]
    pub plugins: Option<HashMap<String, toml::Value>>,
    #[serde(rename = "dev-dependencies", default)]
    pub dev_dependencies: Option<HashMap<String, toml::Value>>,
    #[serde(default)]
    pub build: Option<BuildConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageConfig {
    pub name: Option<String>,
    pub version: Option<String>,
    pub edition: Option<String>,
    #[serde(rename = "type")]
    pub package_type: Option<String>,
    pub entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibConfig {
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApplicationConfig {
    pub entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OptionsConfig {
    #[serde(rename = "rust-plugins")]
    pub rust_plugins: Option<String>,
    pub plugins: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildConfig {
    pub target: Option<String>,
}

impl FlameManifest {
    pub fn package_name(&self) -> Option<&str> {
        self.package.as_ref().and_then(|p| p.name.as_deref())
    }

    pub fn package_type(&self) -> &str {
        self.package
            .as_ref()
            .and_then(|p| p.package_type.as_deref())
            .unwrap_or("executable")
    }

    pub fn is_library(&self) -> bool {
        let t = self.package_type();
        t == "library" || t == "lib" || t == "pkg" || self.lib.is_some()
    }

    pub fn library_entry(&self) -> Option<String> {
        if let Some(lib) = &self.lib {
            if let Some(path) = &lib.path {
                return Some(path.clone());
            }
        }
        if self.is_library() {
            Some("src/lib.fm".to_string())
        } else {
            None
        }
    }

    pub fn app_entry(&self) -> Option<String> {
        if let Some(app) = &self.application {
            if let Some(entry) = &app.entry {
                return Some(entry.clone());
            }
        }
        if let Some(pkg) = &self.package {
            if let Some(entry) = &pkg.entry {
                return Some(entry.clone());
            }
        }
        Some("src/main.fm".to_string())
    }

    pub fn entry_point(&self) -> String {
        if self.is_library() {
            self.library_entry().unwrap_or_else(|| "src/lib.fm".to_string())
        } else {
            self.app_entry().unwrap_or_else(|| "src/main.fm".to_string())
        }
    }

    pub fn rust_plugins_mode(&self) -> RustPluginsMode {
        if let Some(opts) = &self.options {
            let mode_str = opts
                .rust_plugins
                .as_deref()
                .or_else(|| opts.plugins.as_deref());
            if let Some(val) = mode_str {
                match val.trim().trim_matches('"').trim_matches('\'') {
                    "deny" => return RustPluginsMode::Deny,
                    "warn" => return RustPluginsMode::Warn,
                    "default" | "allow" => return RustPluginsMode::Default,
                    _ => {}
                }
            }
        }
        RustPluginsMode::Default
    }

    pub fn permissions_set(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        if let Some(perms) = &self.permissions {
            match perms {
                toml::Value::Table(tbl) => {
                    for (k, v) in tbl {
                        if v.as_bool().unwrap_or(true) {
                            set.insert(k.clone());
                        }
                    }
                }
                toml::Value::Array(arr) => {
                    for v in arr {
                        if let Some(s) = v.as_str() {
                            set.insert(s.to_string());
                        }
                    }
                }
                toml::Value::String(s) => {
                    set.insert(s.clone());
                }
                _ => {}
            }
        }
        set
    }

    pub fn declared_plugins(&self) -> HashSet<String> {
        let mut plugins = HashSet::new();
        if let Some(plugs) = &self.plugins {
            for k in plugs.keys() {
                plugins.insert(k.clone());
            }
        }
        if let Some(native_deps) = &self.native_dependencies {
            for k in native_deps.keys() {
                plugins.insert(k.clone());
            }
        }
        plugins
    }
}

pub fn parse_manifest(content: &str) -> Result<FlameManifest, toml::de::Error> {
    toml::from_str::<FlameManifest>(content)
}

pub fn load_manifest_from_file(path: &Path) -> Option<FlameManifest> {
    if let Ok(content) = std::fs::read_to_string(path) {
        parse_manifest(&content).ok()
    } else {
        None
    }
}

pub fn find_and_load_manifest(start: Option<&Path>) -> Option<(PathBuf, FlameManifest)> {
    let root = match start {
        Some(p) => find_manifest_root(p)?,
        None => {
            if Path::new("flame.toml").exists() || Path::new("Flame.toml").exists() {
                PathBuf::from(".")
            } else {
                return None;
            }
        }
    };

    let p1 = root.join("flame.toml");
    if p1.exists() {
        if let Some(m) = load_manifest_from_file(&p1) {
            return Some((root, m));
        }
    }
    let p2 = root.join("Flame.toml");
    if p2.exists() {
        if let Some(m) = load_manifest_from_file(&p2) {
            return Some((root, m));
        }
    }
    None
}

/// Reads the `rust-plugins` (or `plugins`) setting from `[options]` in `flame.toml`.
pub fn get_rust_plugins_mode(start_path: Option<&Path>) -> RustPluginsMode {
    if let Some((_, manifest)) = find_and_load_manifest(start_path) {
        return manifest.rust_plugins_mode();
    }
    if let Some((_, manifest)) = find_and_load_manifest(None) {
        return manifest.rust_plugins_mode();
    }
    RustPluginsMode::Default
}

pub fn is_rust_plugins_denied(start_path: Option<&Path>) -> bool {
    get_rust_plugins_mode(start_path) == RustPluginsMode::Deny
}

pub fn is_rust_plugins_warn(start_path: Option<&Path>) -> bool {
    get_rust_plugins_mode(start_path) == RustPluginsMode::Warn
}

/// Returns declared plugin and native dependency names from the project's `flame.toml`.
pub fn get_declared_plugins(start_path: Option<&Path>) -> HashSet<String> {
    let mut plugins = HashSet::new();

    let root_opt = start_path
        .and_then(|p| find_manifest_root(p))
        .or_else(|| {
            if Path::new("flame.toml").exists() || Path::new("Flame.toml").exists() {
                Some(PathBuf::from("."))
            } else {
                None
            }
        });

    if let Some(root) = root_opt {
        let manifest_path = if root.join("flame.toml").exists() {
            root.join("flame.toml")
        } else {
            root.join("Flame.toml")
        };

        if let Some(manifest) = load_manifest_from_file(&manifest_path) {
            plugins.extend(manifest.declared_plugins());
        } else if let Ok(content) = std::fs::read_to_string(&manifest_path) {
            for (name, _) in parse_manifest_section(&content, "[plugins]") {
                plugins.insert(name);
            }
            for (name, _) in parse_manifest_section(&content, "[native-dependencies]") {
                plugins.insert(name);
            }
        }

        // Also check if root has a native/ directory with Cargo.toml or src/lib.rs
        let native_dir = root.join("native");
        if native_dir.exists() {
            let cargo_toml = native_dir.join("Cargo.toml");
            if let Ok(c_content) = std::fs::read_to_string(&cargo_toml) {
                if let Ok(cargo_val) = toml::from_str::<toml::Value>(&c_content) {
                    if let Some(name) = cargo_val
                        .get("package")
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        plugins.insert(name.to_string());
                    }
                }
            }
            plugins.insert("native".to_string());
        }
    }

    plugins
}

/// Finds the root directory of a Flame project by locating `flame.toml` or `Flame.toml`.
pub fn find_manifest_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if current.join("flame.toml").exists() || current.join("Flame.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Parses key-value pairs inside a specified TOML section (e.g. `[plugins]`).
pub fn parse_manifest_section(content: &str, section_name: &str) -> Vec<(String, String)> {
    let clean_section = section_name
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');

    if let Ok(val) = toml::from_str::<toml::Value>(content) {
        if let Some(tbl) = val.get(clean_section).and_then(|v| v.as_table()) {
            let mut entries = Vec::new();
            for (k, v) in tbl {
                let v_str = match v {
                    toml::Value::String(s) => s.clone(),
                    _ => v.to_string().trim_matches('"').to_string(),
                };
                entries.push((k.clone(), v_str));
            }
            return entries;
        }
    }

    // Fallback to line-by-line parsing if TOML structure is partial
    let mut entries = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == section_name {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_section = false;
            continue;
        }
        if !in_section || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, value)) = trimmed.split_once('=') {
            entries.push((
                name.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            ));
        }
    }

    entries
}

/// Alias for `parse_manifest_section` for backward compatibility.
pub fn parse_section_entries(content: &str, section_name: &str) -> Vec<(String, String)> {
    parse_manifest_section(content, section_name)
}

/// Parses declared permissions from `[permissions]` section of a manifest.
pub fn parse_manifest_permissions(content: &str) -> HashSet<String> {
    if let Ok(manifest) = parse_manifest(content) {
        let set = manifest.permissions_set();
        if !set.is_empty() {
            return set;
        }
    }

    let mut perms = HashSet::new();
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[permissions]" {
            in_section = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_section = false;
            continue;
        }
        if !in_section || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_idx) = trimmed.find('=') {
            let key = trimmed[..eq_idx].trim().to_string();
            let val = trimmed[eq_idx + 1..].trim();
            if val == "true" {
                perms.insert(key);
            }
        } else {
            perms.insert(trimmed.to_string());
        }
    }
    perms
}
