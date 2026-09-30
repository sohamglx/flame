use std::fs;
use std::path::{Path, PathBuf};
use super::downloader::*;
use super::manifest::*;
use super::native::*;

pub fn is_rust_plugins_denied() -> bool {
    crate::utils::manifest::is_rust_plugins_denied(None)
}

pub fn is_rust_plugins_warn() -> bool {
    crate::utils::manifest::is_rust_plugins_warn(None)
}

#[cfg(feature = "cli")]
pub fn add_package(args: &[String]) {
    if args.is_empty() {
        println!("\x1b[1;31merror:\x1b[0m please specify package name to add.");
        println!(
            "usage: flame add <package_name> | flame add --plugin <path> (-p) | flame add --native <crate> (-n)"
        );
        return;
    }

    let is_plugin = args.contains(&"--plugin".to_string())
        || args.contains(&"-p".to_string())
        || args.contains(&"@plugin".to_string());
    let is_native = args.contains(&"--native".to_string())
        || args.contains(&"-n".to_string());

    if (is_plugin || is_native) && is_rust_plugins_denied() {
        println!(
            "\x1b[1;31merror:\x1b[0m installation blocked: rust-plugins is set to \"deny\" in flame.toml [options]."
        );
        println!(
            "help: change `rust-plugins` in flame.toml [options] to \"default\" or remove it to allow Rust plugins."
        );
        return;
    }

    if (is_plugin || is_native) && is_rust_plugins_warn() {
        println!(
            "\x1b[1;33mwarning:\x1b[0m adding native dependency: rust-plugins is set to \"warn\" in flame.toml [options]."
        );
    }

    let (manifest_key, manifest_value, section) = if is_plugin {
        let plugin_idx = match args.iter().position(|r| r == "--plugin" || r == "-p" || r == "@plugin") {
            Some(idx) => idx,
            None => {
                println!(
                    "\x1b[1;31merror:\x1b[0m --plugin requires a file path argument (e.g. flame add --plugin ./native)."
                );
                return;
            }
        };
        let plugin_path = match args.get(plugin_idx + 1) {
            Some(p) if !p.starts_with('-') => p.clone(),
            _ => {
                if plugin_idx > 0 && !args[plugin_idx - 1].starts_with('-') {
                    args[plugin_idx - 1].clone()
                } else {
                    println!(
                        "\x1b[1;31merror:\x1b[0m --plugin requires a valid file path argument (e.g. flame add --plugin ./native)."
                    );
                    return;
                }
            }
        };
        let plugin_name = if let Some(name_idx) = args.iter().position(|r| r == "--name") {
            match args.get(name_idx + 1) {
                Some(n) if !n.starts_with("--") => n.clone(),
                _ => {
                    println!(
                        "\x1b[1;31merror:\x1b[0m --name requires a valid name argument (e.g. --name server)."
                    );
                    return;
                }
            }
        } else {
            let cargo_candidates = [
                Path::new(&plugin_path).join("Cargo.toml"),
                Path::new(&plugin_path).join("native").join("Cargo.toml"),
            ];
            let mut extracted = None;
            for cargo_toml_path in &cargo_candidates {
                if cargo_toml_path.exists() {
                    if let Ok(content) = fs::read_to_string(cargo_toml_path) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("name") && trimmed.contains('=') {
                                if let Some(val) = trimmed.split('=').nth(1) {
                                    let clean = val.trim().trim_matches('"').trim_matches('\'');
                                    if !clean.is_empty() {
                                        extracted = Some(clean.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                if extracted.is_some() {
                    break;
                }
            }
            extracted.unwrap_or_else(|| {
                Path::new(&plugin_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("plugin")
                    .to_string()
            })
        };

        // Validate that plugin name is not equal to Flame package name
        let pkg_name = if let Ok(content) = fs::read_to_string("flame.toml") {
            let mut name = String::new();
            for line in content.lines() {
                let t = line.trim();
                if t.starts_with("name =") || t.starts_with("name=") {
                    if let Some(val) = t.split('=').nth(1) {
                        name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                        break;
                    }
                }
            }
            name
        } else {
            String::new()
        };

        if !pkg_name.is_empty() && plugin_name == pkg_name {
            println!(
                "\x1b[1;31merror:\x1b[0m plugin name '{}' cannot be the same as the Flame package name '{}'.",
                plugin_name, pkg_name
            );
            println!(
                "help: rename your plugin in its Cargo.toml or choose a distinct name to avoid Cargo build and linkage collisions."
            );
            return;
        }

        (plugin_name, plugin_path, "[plugins]")
    } else {
        let raw_target = if is_native {
            let native_idx = args.iter().position(|r| r == "--native" || r == "-n").unwrap();
            if let Some(target) = args.get(native_idx + 1).filter(|a| !a.starts_with('-')) {
                target.clone()
            } else if native_idx > 0 && !args[native_idx - 1].starts_with('-') {
                args[native_idx - 1].clone()
            } else if let Some(target) = args.iter().find(|a| !a.starts_with('-')) {
                target.clone()
            } else {
                println!(
                    "\x1b[1;31merror:\x1b[0m --native requires a crate name (e.g. flame add --native serde)."
                );
                return;
            }
        } else {
            match args.iter().find(|a| !a.starts_with('-')) {
                Some(t) => t.clone(),
                None => {
                    println!("\x1b[1;31merror:\x1b[0m please specify package name to add.");
                    return;
                }
            }
        };

        let mut name = if let Some(name_idx) = args.iter().position(|r| r == "--name") {
            match args.get(name_idx + 1) {
                Some(n) if !n.starts_with("--") => n.clone(),
                _ => {
                    println!("\x1b[1;31merror:\x1b[0m --name requires a valid name argument.");
                    return;
                }
            }
        } else {
            String::new()
        };

        if name.is_empty() {
            if raw_target.contains("github.com/") && raw_target.contains("/archive/") {
                let parts: Vec<&str> = raw_target.split('/').collect();
                if let Some(idx) = parts.iter().position(|&p| p == "github.com") {
                    if idx + 2 < parts.len() {
                        name = parts[idx + 2].to_string();
                    }
                }
            }
            if name.is_empty() {
                let base = raw_target
                    .trim_end_matches(".zip")
                    .trim_end_matches(".tar.gz")
                    .trim_end_matches(".git");
                name = base
                    .rsplit('/')
                    .next()
                    .unwrap_or(base)
                    .split('@')
                    .next()
                    .unwrap_or(base)
                    .to_string();
            }
        }

        let val = if raw_target == name {
            "*".to_string()
        } else {
            raw_target.clone()
        };
        let sec = if is_native {
            "[native-dependencies]"
        } else {
            "[dependencies]"
        };
        (name, val, sec)
    };

    println!(
        "\x1b[1;36m    Adding\x1b[0m {} '{}'...",
        if is_plugin {
            "plugin"
        } else if is_native {
            "native dependency"
        } else {
            "dependency"
        },
        manifest_key
    );

    let toml_path = Path::new("flame.toml");
    if toml_path.exists() {
        let mut content = fs::read_to_string(toml_path).unwrap_or_default();
        if !content.contains(&format!("{} =", manifest_key))
            && !content.contains(&format!("{}=", manifest_key))
        {
            if !content.contains(section) {
                content.push_str(&format!("\n{}\n", section));
                content.push_str(&format!("{} = \"{}\"\n", manifest_key, manifest_value));
            } else {
                if let Some(idx) = content.find(section) {
                    let insert_pos = idx + section.len();
                    content.insert_str(
                        insert_pos,
                        &format!("\n{} = \"{}\"", manifest_key, manifest_value),
                    );
                }
            }
            let _ = fs::write(toml_path, content);
        }
    }

    // Only download and compile the specific package or plugin being added
    if is_plugin {
        let local_plugin = Path::new(&manifest_value);
        if local_plugin.join("Cargo.toml").exists()
            || local_plugin.join("native").join("Cargo.toml").exists()
        {
            println!(
                "\x1b[1;36m   Compiling\x1b[0m rust plugin '{}'...",
                manifest_key
            );
            generate_package_fmi(&manifest_key, local_plugin, false);
            inspect_native_plugin(&manifest_key, local_plugin);
        } else {
            println!(
                "\x1b[1;33m   Warning:\x1b[0m local plugin path '{}' does not contain Cargo.toml yet.",
                manifest_value
            );
        }
    } else {
        let pkg_base = Path::new(".flame").join("pkg");
        let target_dir = pkg_base.join(&manifest_key);
        let is_remote = manifest_value.starts_with("http") || manifest_value.contains("github.com");

        let pkg_location = if is_remote {
            let _ = fs::create_dir_all(&pkg_base);
            if !target_dir.exists() {
                if let Err(e) =
                    download_archive_with_loader(&manifest_key, &manifest_value, &target_dir)
                {
                    eprintln!(
                        "\r\x1b[1;31m  ✗ Failed\x1b[0m downloading package '{}': {}\x1b[K",
                        manifest_key, e
                    );
                }
            }
            target_dir
        } else if manifest_value.starts_with('.') || manifest_value.starts_with('/') {
            PathBuf::from(&manifest_value)
        } else {
            target_dir
        };

        if pkg_location.exists() {
            if is_rust_plugins_denied() && (pkg_location.join("Cargo.toml").exists() || pkg_location.join("native").exists()) {
                println!(
                    "   \x1b[1;33m⚠ Warning:\x1b[0m skipping Rust plugin compilation in '{}' (`rust-plugins = \"deny\"` in flame.toml [options]).",
                    manifest_key
                );
            } else {
                build_single_dependency_plugins(&pkg_location, false);
            }
        }
    }

    println!(
        "\x1b[1;32m   Installed\x1b[0m {} '{}' successfully.",
        if is_plugin {
            "plugin"
        } else if is_native {
            "native dependency"
        } else {
            "package"
        },
        manifest_key
    );
}

#[cfg(feature = "cli")]
pub fn remove_package(pkg_name: &str) {
    let toml_path = Path::new("flame.toml");
    if toml_path.exists() {
        if let Ok(content) = fs::read_to_string(toml_path) {
            let lines: Vec<&str> = content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.starts_with(&format!("{} =", pkg_name))
                        && !trimmed.starts_with(&format!("{}=", pkg_name))
                })
                .collect();
            let _ = fs::write(toml_path, lines.join("\n"));
        }
    }

    let pkg_dir = Path::new(".flame").join("pkg").join(pkg_name);
    if pkg_dir.exists() {
        let manifest_path = pkg_dir.join("flame.toml");
        if manifest_path.exists() {
            if let Ok(dep_manifest) = fs::read_to_string(&manifest_path) {
                let plugins = parse_section_entries(&dep_manifest, "[plugins]");
                let native_deps = parse_section_entries(&dep_manifest, "[native-dependencies]");
                for (plugin_name, _) in plugins.into_iter().chain(native_deps.into_iter()) {
                    let plugin_dir = Path::new(".flame").join("pkg").join(&plugin_name);
                    if plugin_dir.exists() && plugin_name != pkg_name {
                        let _ = fs::remove_dir_all(&plugin_dir);
                        println!(
                            "\x1b[1;32m     Removed\x1b[0m local plugin '{}' of package '{}'",
                            plugin_name, pkg_name
                        );
                    }
                }
            }
        }
        let _ = fs::remove_dir_all(&pkg_dir);
    }
    println!("\x1b[1;32m     Removed\x1b[0m package '{}'", pkg_name);
}


#[cfg(feature = "cli")]
pub fn ensure_dependencies_installed(is_release: bool) {
    // Compile std_bridge directly since it uses #[flame_export] now
    let std_bridge_path = Path::new("flame-stdlib").join("native").join("std_bridge");
    if std_bridge_path.exists() {
        let profile_dir = if is_release { "release" } else { "debug" };
        let lib_out = std_bridge_path
            .join("target")
            .join(profile_dir)
            .join("libstd_bridge.rlib");
        let mut needs_build = true;
        if lib_out.exists() {
            if let Ok(out_meta) = fs::metadata(&lib_out) {
                if let Ok(out_mtime) = out_meta.modified() {
                    if let Ok(src_meta) = fs::metadata(std_bridge_path.join("src").join("lib.rs")) {
                        if let Ok(src_mtime) = src_meta.modified() {
                            if out_mtime > src_mtime {
                                needs_build = false;
                            }
                        }
                    }
                }
            }
        }
        if needs_build {
            let mut cmd = std::process::Command::new("cargo");
            cmd.arg("build");
            if is_release {
                cmd.arg("--release");
            }
            let _ = cmd.current_dir(&std_bridge_path).output();
        }
    }
    let toml_path = Path::new("flame.toml");
    if !toml_path.exists() {
        return;
    }

    let content = match fs::read_to_string(toml_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Parse all relevant sections for dependencies
    let deps = parse_section_entries(&content, "[dependencies]");
    let native_deps = parse_section_entries(&content, "[native-dependencies]");
    let plugins = parse_section_entries(&content, "[plugins]");

    // Combine native deps and plugins for compilation
    let mut native_to_compile = native_deps;
    native_to_compile.extend(plugins);

    let fetch_remote = |target: &str, source: &str| -> String {
        let is_local = source.starts_with('.') || source.starts_with('/') || source == "*";
        if is_local {
            if source == "*" {
                target.to_string()
            } else {
                source.to_string()
            }
        } else {
            let pkg_dir = Path::new(".flame").join("pkg");
            let target_dir = pkg_dir.join(&target);

            if !source.starts_with("http") && !source.contains("github.com") {
                return target.to_string();
            }

            if !target_dir.exists() {
                let _ = fs::create_dir_all(&pkg_dir);
                if let Err(e) = download_archive_with_loader(target, source, &target_dir) {
                    eprintln!(
                        "\r\x1b[1;31m  ✗ Failed\x1b[0m downloading package '{}': {}\x1b[K",
                        target, e
                    );
                }
            }
            target_dir.to_string_lossy().into_owned()
        }
    };

    for (target, source) in deps {
        fetch_remote(&target, &source);
    }

    if !is_rust_plugins_denied() {
        for (target, source) in native_to_compile {
            let plugin_path_str = fetch_remote(&target, &source);
            let plugin_path = Path::new(&plugin_path_str);

            if plugin_path.join("Cargo.toml").exists()
                || plugin_path.join("native").join("Cargo.toml").exists()
            {
                generate_package_fmi(&target, plugin_path, is_release);
            }
        }
        build_all_dependency_plugins(is_release);
    }
}


#[cfg(feature = "cli")]
pub fn install_all_packages(args: &[String]) {
    let toml_path = Path::new("flame.toml");
    if !toml_path.exists() {
        println!(
            "\x1b[1;31merror:\x1b[0m no flame.toml manifest file found in the current directory."
        );
        println!("help: run this command inside a valid Flame project directory.");
        return;
    }

    let is_release = args.contains(&"--release".to_string()) || args.contains(&"-r".to_string());
    let force = args.contains(&"--force".to_string()) || args.contains(&"-f".to_string());

    println!();
    println!("\x1b[1;36m  Installing\x1b[0m dependencies declared in flame.toml...\n");

    let content = match fs::read_to_string(toml_path) {
        Ok(c) => c,
        Err(e) => {
            println!("\x1b[1;31merror:\x1b[0m failed to read flame.toml: {}", e);
            return;
        }
    };

    let deps = parse_section_entries(&content, "[dependencies]");
    let native_deps = parse_section_entries(&content, "[native-dependencies]");
    let plugins = parse_section_entries(&content, "[plugins]");

    let pkg_dir = Path::new(".flame").join("pkg");
    let _ = fs::create_dir_all(&pkg_dir);

    let mut successful_installs = 0;
    let mut errors = 0;
    let rust_denied = is_rust_plugins_denied();

    // 1. Process pure Flame dependencies & remote packages
    for (target, source) in deps {
        let is_local = source.starts_with('.') || source.starts_with('/') || source == "*";
        if is_local {
            println!(
                "   \x1b[1;35m•\x1b[0m Linked      local package '{}' ({})",
                target, source
            );
            let local_dir = if source == "*" {
                PathBuf::from(&target)
            } else {
                PathBuf::from(&source)
            };
            generate_package_fmi(&target, &local_dir, is_release);
            if local_dir.join("flame.toml").exists() {
                if let Ok(dep_manifest) = fs::read_to_string(local_dir.join("flame.toml")) {
                    let plugins = parse_section_entries(&dep_manifest, "[plugins]");
                    let native_deps = parse_section_entries(&dep_manifest, "[native-dependencies]");
                    for (plugin_name, plugin_source) in
                        plugins.into_iter().chain(native_deps.into_iter())
                    {
                        let clean_source = plugin_source.trim_matches('"').trim();
                        let p_dir = if clean_source == "*" {
                            local_dir.clone()
                        } else if clean_source.starts_with('.') || clean_source.starts_with('/') {
                            local_dir.join(clean_source)
                        } else {
                            local_dir.join(clean_source)
                        };
                        if p_dir.join("Cargo.toml").exists()
                            || p_dir.join("native").join("Cargo.toml").exists()
                        {
                            if rust_denied {
                                println!(
                                    "   \x1b[1;33m⚠ Warning:\x1b[0m skipping Rust plugin '{}' from '{}' (`rust-plugins = \"deny\"` in flame.toml [options]).",
                                    plugin_name, target
                                );
                            } else {
                                println!(
                                    "   \x1b[1;36m•\x1b[0m Compiling   dependency plugin '{}' from '{}'...",
                                    plugin_name, target
                                );
                                if generate_package_fmi(&plugin_name, &p_dir, is_release) {
                                    successful_installs += 1;
                                }
                            }
                        }
                    }
                }
            }
            successful_installs += 1;
        } else if source.starts_with("http") || source.contains("github.com") {
            let target_dir = pkg_dir.join(&target);
            let mut download_ok = true;
            if !target_dir.exists() || force {
                if target_dir.exists() && force {
                    let _ = fs::remove_dir_all(&target_dir);
                }
                match download_archive_with_loader(&target, &source, &target_dir) {
                    Ok(_) => {
                        successful_installs += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "   \x1b[1;31m✗\x1b[0m Failed downloading package '{}': {}",
                            target, e
                        );
                        errors += 1;
                        download_ok = false;
                    }
                }
            } else {
                println!(
                    "   \x1b[1;34m•\x1b[0m Cached      package '{}' (use --force to re-download)",
                    target
                );
                successful_installs += 1;
            }

            if download_ok && target_dir.exists() {
                generate_package_fmi(&target, &target_dir, is_release);
            }
        } else {
            println!(
                "   \x1b[1;36m•\x1b[0m Resolved    package '{}' version {}",
                target, source
            );
            successful_installs += 1;
        }
    }

    // 2. Process native-dependencies and plugins
    let mut native_to_process = native_deps;
    native_to_process.extend(plugins);

    if rust_denied && !native_to_process.is_empty() {
        println!();
        println!(
            "   \x1b[1;33m⚠ Warning:\x1b[0m skipping native dependencies and plugins: `rust-plugins = \"deny\"` in flame.toml [options]."
        );
    } else {
        if !native_to_process.is_empty() {
            println!();
        }

        for (target, source) in native_to_process {
            let is_local = source.starts_with('.') || source.starts_with('/') || source == "*";
            let target_dir = if is_local {
                if source == "*" {
                    PathBuf::from(&target)
                } else {
                    PathBuf::from(&source)
                }
            } else if source.starts_with("http") || source.contains("github.com") {
                let dest = pkg_dir.join(&target);
                if !dest.exists() || force {
                    if dest.exists() && force {
                        let _ = fs::remove_dir_all(&dest);
                    }
                    if let Err(e) = download_archive_with_loader(&target, &source, &dest) {
                        eprintln!(
                            "   \x1b[1;31m✗\x1b[0m Failed downloading plugin '{}': {}",
                            target, e
                        );
                        errors += 1;
                        continue;
                    }
                } else {
                    println!("   \x1b[1;34m•\x1b[0m Cached      plugin '{}'", target);
                }
                dest
            } else {
                pkg_dir.join(&target)
            };

            if generate_package_fmi(&target, &target_dir, is_release) {
                successful_installs += 1;
            } else if is_local {
                println!(
                    "   \x1b[1;33m⚠\x1b[0m Warning: local plugin '{}' at '{}' does not contain Cargo.toml",
                    target,
                    target_dir.display()
                );
            }
        }
    }

    // 3. Process and build local plugins / native dependencies declared inside all dependency packages
    let dep_plugins_built = if !rust_denied {
        build_all_dependency_plugins(is_release)
    } else {
        0
    };
    successful_installs += dep_plugins_built;

    if errors > 0 {
        println!(
            "\n\x1b[1;31merror:\x1b[0m completed with {} error(s) during installation.\n",
            errors
        );
    } else {
        println!(
            "\n\x1b[1;32m  Finished\x1b[0m successfully installed {} package(s) and plugins.\n",
            successful_installs
        );
    }
}

