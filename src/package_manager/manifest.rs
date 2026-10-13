use std::path::Path;
use super::meta::PluginSpec;
pub use crate::utils::manifest::{
    find_and_load_manifest, load_manifest_from_file, parse_manifest, parse_manifest_permissions,
    parse_manifest_section, parse_section_entries, FlameManifest, PackageConfig, RustPluginsMode,
};

pub fn list_plugins() -> Vec<PluginSpec> {
    let toml_path = Path::new("flame.toml");
    if !toml_path.exists() {
        return Vec::new();
    }

    let manifest = match load_manifest_from_file(toml_path) {
        Some(m) => m,
        None => return Vec::new(),
    };

    let mut specs = Vec::new();
    if let Some(plugins) = manifest.plugins {
        for (name, val) in plugins {
            let source = match val {
                toml::Value::String(s) => s,
                _ => val.to_string().trim_matches('"').to_string(),
            };
            let version = if source == "*" || source.is_empty() {
                None
            } else if let Some((_, version)) = source.rsplit_once('@') {
                Some(version.to_string())
            } else {
                None
            };
            let is_local = source.starts_with('.') || source.starts_with('/') || source == "*";
            specs.push(PluginSpec {
                name,
                source,
                version,
                is_local,
            });
        }
    }

    specs
}

