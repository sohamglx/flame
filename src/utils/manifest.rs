use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustPluginsMode {
    Default,
    Deny,
    Warn,
}

/// Reads the `rust-plugins` (or `plugins`) setting from `[options]` in `flame.toml`.
pub fn get_rust_plugins_mode(start_path: Option<&Path>) -> RustPluginsMode {
    let check_file = |p: &Path| -> Option<RustPluginsMode> {
        if let Ok(content) = std::fs::read_to_string(p) {
            let mut in_options = false;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    in_options = trimmed == "[options]";
                    continue;
                }
                if in_options && (trimmed.starts_with("rust-plugins") || trimmed.starts_with("plugins")) {
                    if let Some(val) = trimmed.split('=').nth(1) {
                        let clean = val.trim().trim_matches('"').trim_matches('\'');
                        match clean {
                            "deny" => return Some(RustPluginsMode::Deny),
                            "warn" => return Some(RustPluginsMode::Warn),
                            "default" | "allow" => return Some(RustPluginsMode::Default),
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    };

    if let Some(p) = start_path {
        if let Some(root) = find_manifest_root(p) {
            if let Some(mode) = check_file(&root.join("flame.toml")) {
                return mode;
            }
            if let Some(mode) = check_file(&root.join("Flame.toml")) {
                return mode;
            }
        }
    }

    if let Some(mode) = check_file(Path::new("flame.toml")) {
        return mode;
    }
    if let Some(mode) = check_file(Path::new("Flame.toml")) {
        return mode;
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

        if let Ok(content) = std::fs::read_to_string(&manifest_path) {
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
                for line in c_content.lines() {
                    let t = line.trim();
                    if t.starts_with("name =") || t.starts_with("name=") {
                        if let Some(val) = t.split('=').nth(1) {
                            let name = val.trim().trim_matches('"').trim_matches('\'');
                            if !name.is_empty() {
                                plugins.insert(name.to_string());
                            }
                        }
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
