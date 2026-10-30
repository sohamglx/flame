use std::fs;
use std::io::Write;
use std::path::Path;
#[cfg(feature = "cli")]
use super::downloader::download_archive_with_loader;
use super::manifest::*;
use super::meta::*;
use super::rustdoc::*;

pub fn inspect_native_plugin(target: &str, plugin_path: &Path) {
    inspect_native_plugin_opt(target, plugin_path, false);
}

pub fn inspect_native_plugin_forced(target: &str, plugin_path: &Path) {
    inspect_native_plugin_opt(target, plugin_path, true);
}

pub fn inspect_native_plugin_opt(target: &str, plugin_path: &Path, force: bool) {
    let pkg_dir = Path::new(".flame").join("pkg").join(target);
    let fmi_path = pkg_dir.join(format!("{}.fmi", target));

    let mut needs_update = true;
    if !force && fmi_path.exists() {
        if let Ok(fmi_meta) = fs::metadata(&fmi_path) {
            if let Ok(fmi_mtime) = fmi_meta.modified() {
                needs_update = false;

                if let Ok(src_meta) = fs::metadata(plugin_path.join("src").join("lib.rs")) {
                    if let Ok(src_mtime) = src_meta.modified() {
                        if src_mtime > fmi_mtime {
                            needs_update = true;
                        }
                    }
                }

                if let Ok(cargo_meta) = fs::metadata(plugin_path.join("Cargo.toml")) {
                    if let Ok(cargo_mtime) = cargo_meta.modified() {
                        if cargo_mtime > fmi_mtime {
                            needs_update = true;
                        }
                    }
                }
            }
        }
    }

    if !needs_update {
        return;
    }

    println!(
        "\x1b[1;36m  Inspecting\x1b[0m rust plugin '{}' via fmi parser...",
        target
    );

    let cargo_toml_path = plugin_path.join("Cargo.toml");
    let mut crate_name = target.to_string();
    if let Ok(cargo_content) = fs::read_to_string(&cargo_toml_path) {
        for line in cargo_content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("name") {
                if let Some(idx) = trimmed.find('=') {
                    crate_name = trimmed[idx + 1..].trim().trim_matches('"').to_string();
                    break;
                }
            }
        }
    }

    // Convert crate_name hyphens to underscores as rustc outputs library names this way
    let lib_crate_name = crate_name.replace('-', "_");

    let lib_filename = if cfg!(target_os = "windows") {
        format!("{}.dll", lib_crate_name)
    } else if cfg!(target_os = "macos") {
        format!("lib{}.dylib", lib_crate_name)
    } else {
        format!("lib{}.so", lib_crate_name)
    };

    let mut meta = FlameMeta {
        module: target.to_string(),
        kind: "native".to_string(),
        lib: Some(lib_filename),
        functions: Vec::new(),
        structs: Vec::new(),
        docs: None,
    };

    enrich_with_syn(&mut meta, plugin_path);

    let _ = fs::create_dir_all(&pkg_dir);
    if let Ok(meta_str) = serde_json::to_string_pretty(&meta) {
        let _ = fs::write(&fmi_path, meta_str);
    }
}

#[cfg(feature = "cli")]
pub fn generate_package_fmi(target: &str, target_dir: &Path, is_release: bool) -> bool {
    let pkg_fmi_dir = Path::new(".flame").join("pkg").join(target);
    let fmi_path = pkg_fmi_dir.join(format!("{}.fmi", target));

    // Case 1: Package or plugin with Rust native code (Cargo.toml or native/Cargo.toml)
    let native_path = if target_dir.join("Cargo.toml").exists() {
        Some(target_dir.to_path_buf())
    } else if target_dir.join("native").join("Cargo.toml").exists() {
        Some(target_dir.join("native"))
    } else {
        None
    };

    if let Some(npath) = native_path {
        let profile_dir = if is_release { "release" } else { "debug" };
        let target_build_dir = npath.join("target").join(profile_dir);
        let mut is_up_to_date = false;
        if fmi_path.exists() && target_build_dir.exists() {
            if let Ok(fmi_meta) = fs::metadata(&fmi_path) {
                if let Ok(fmi_time) = fmi_meta.modified() {
                    fn check_plugin_src_newer(dir: &Path, fmi_time: std::time::SystemTime) -> bool {
                        if let Ok(entries) = fs::read_dir(dir) {
                            for entry in entries.flatten() {
                                let p = entry.path();
                                if p.is_dir() {
                                    if p.file_name().map_or(false, |n| n == "target") {
                                        continue;
                                    }
                                    if check_plugin_src_newer(&p, fmi_time) {
                                        return true;
                                    }
                                } else if p
                                    .extension()
                                    .map_or(false, |ext| ext == "rs" || ext == "toml")
                                {
                                    if let Ok(m) = entry.metadata() {
                                        if let Ok(mtime) = m.modified() {
                                            if mtime > fmi_time {
                                                return true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        false
                    }
                    if !check_plugin_src_newer(&npath, fmi_time) {
                        is_up_to_date = true;
                    }
                }
            }
        }

        if is_up_to_date {
            println!("   \x1b[1;32m✓\x1b[0m Up-to-date  rust plugin '{}'", target);
            return true;
        }

        let start_build = std::time::Instant::now();
        let (tx, rx) = std::sync::mpsc::channel();
        let target_name = target.to_string();
        let spin_handle = std::thread::spawn(move || {
            let spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut i = 0;
            while rx.try_recv().is_err() {
                let elapsed = start_build.elapsed().as_secs_f64();
                print!(
                    "\r   \x1b[1;36m⚙\x1b[0m \x1b[1;36m{}\x1b[0m Compiling   rust plugin '{}' ({:.1}s)...\x1b[K",
                    spinners[i], target_name, elapsed
                );
                let _ = std::io::stdout().flush();
                std::thread::sleep(std::time::Duration::from_millis(80));
                i = (i + 1) % spinners.len();
            }
        });

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("build");
        if is_release {
            cmd.arg("--release");
        }
        let res = cmd.current_dir(&npath).output();
        let _ = tx.send(());
        let _ = spin_handle.join();

        let elapsed_str = if start_build.elapsed().as_secs_f64() < 1.0 {
            format!("{}ms", start_build.elapsed().as_millis())
        } else {
            format!("{:.2}s", start_build.elapsed().as_secs_f64())
        };

        if let Ok(out) = res {
            if !out.status.success() {
                eprintln!(
                    "\r   \x1b[1;31m✗\x1b[0m Failed to compile native plugin '{}': {}\x1b[K",
                    target,
                    String::from_utf8_lossy(&out.stderr)
                );
                return false;
            }
        }
        print!(
            "\r   \x1b[1;32m✓\x1b[0m Compiled    rust plugin '{}' in {}\x1b[K\n",
            target, elapsed_str
        );
        let _ = std::io::stdout().flush();

        inspect_native_plugin_forced(target, &npath);
        println!(
            "   \x1b[1;32m✓\x1b[0m Generated   .fmi interface for '{}' at {}",
            target,
            fmi_path.display()
        );
        return true;
    }

    // Case 2: Pure Flame package - inspect .fm files to generate cached .fmi interface
    let src_dir = if target_dir.join("src").exists() {
        target_dir.join("src")
    } else {
        target_dir.to_path_buf()
    };

    let mut functions = Vec::new();
    let mut structs = Vec::new();

    if let Ok(entries) = fs::read_dir(&src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("fm") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let mut lexer = crate::lexer::Lexer::new(&content);
                    let mut tokens = Vec::new();
                    loop {
                        let tok = lexer.next_token();
                        let is_eof = tok.kind == crate::lexer::TokenKind::EOF;
                        tokens.push(tok);
                        if is_eof {
                            break;
                        }
                    }
                    let mut parser =
                        crate::parser::Parser::new(tokens, path.to_string_lossy().to_string());
                    if let Ok(stmts) = parser.parse() {
                        for stmt in stmts {
                            let unwrapped = match &stmt {
                                crate::parser::Stmt::ExportDecl(inner, _) => &**inner,
                                other => other,
                            };
                            match unwrapped {
                                crate::parser::Stmt::FuncDecl {
                                    name,
                                    params,
                                    return_type,
                                    is_async,
                                    ..
                                } => {
                                    functions.push(FlameFunctionMeta {
                                        name: name.clone(),
                                        flame_name: name.clone(),
                                        params: params
                                            .iter()
                                            .map(|p| FlameParamMeta {
                                                name: p.name.clone(),
                                                type_name: if p.type_name.is_empty() {
                                                    "any".to_string()
                                                } else {
                                                    p.type_name.clone()
                                                },
                                                is_callback: false,
                                                is_ref: p.is_ref,
                                                is_mut: p.is_mut,
                                            })
                                            .collect(),
                                        return_type: return_type
                                            .clone()
                                            .unwrap_or_else(|| "void".to_string()),
                                        is_static: false,
                                        is_generic: false,
                                        is_async: *is_async,
                                        is_constructor: false,
                                        persistent_runtime: false,
                                        receiver: None,
                                        docs: None,
                                        requires: Vec::new(),
                                        permissions: Vec::new(),
                                    });
                                }
                                crate::parser::Stmt::StructDecl { name, fields, .. } => {
                                    structs.push(FlameStructMeta {
                                        name: name.clone(),
                                        flame_name: name.clone(),
                                        methods: Vec::new(),
                                        fields: fields
                                            .iter()
                                            .map(|(f_name, f_type)| FlameStructFieldMeta {
                                                name: f_name.clone(),
                                                type_name: if f_type.is_empty() {
                                                    "any".to_string()
                                                } else {
                                                    f_type.clone()
                                                },
                                                docs: None,
                                            })
                                            .collect(),
                                        docs: None,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    if !functions.is_empty() || !structs.is_empty() {
        let meta = FlameMeta {
            module: target.to_string(),
            kind: "flame".to_string(),
            lib: None,
            functions,
            structs,
            docs: None,
        };
        let _ = fs::create_dir_all(&pkg_fmi_dir);
        if let Ok(meta_str) = serde_json::to_string_pretty(&meta) {
            let _ = fs::write(&fmi_path, meta_str);
            println!(
                "   \x1b[1;32m✓\x1b[0m Generated   .fmi interface for '{}' at {}",
                target,
                fmi_path.display()
            );
            return true;
        }
    }

    false
}

#[cfg(feature = "cli")]
pub fn build_single_dependency_plugins(pkg_path: &Path, is_release: bool) -> usize {
    let mut built_count = 0;
    let pkg_name = pkg_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let pkg_base = Path::new(".flame").join("pkg");

    // 1. Direct Rust native plugin (Cargo.toml at root or in native/)
    if pkg_path.join("Cargo.toml").exists() || pkg_path.join("native").join("Cargo.toml").exists() {
        if generate_package_fmi(&pkg_name, pkg_path, is_release) {
            built_count += 1;
        }
        return built_count;
    }

    // 2. Package with flame.toml declaring sub-plugins or native dependencies
    let toml_path = pkg_path.join("flame.toml");
    if toml_path.exists() {
        if let Ok(dep_manifest) = fs::read_to_string(&toml_path) {
            let plugins = parse_section_entries(&dep_manifest, "[plugins]");
            let native_deps = parse_section_entries(&dep_manifest, "[native-dependencies]");
            for (plugin_name, plugin_source) in plugins.into_iter().chain(native_deps.into_iter()) {
                let clean_source = plugin_source.trim_matches('"').trim();
                let is_local = clean_source.starts_with('.')
                    || clean_source.starts_with('/')
                    || clean_source == "*";
                let plugin_dir = if is_local {
                    if clean_source == "*" {
                        pkg_path.to_path_buf()
                    } else {
                        pkg_path.join(clean_source)
                    }
                } else if clean_source.starts_with("http") || clean_source.contains("github.com") {
                    let dest = pkg_base.join(&plugin_name);
                    if !dest.exists() {
                        let _ = download_archive_with_loader(&plugin_name, clean_source, &dest);
                    }
                    dest
                } else {
                    pkg_base.join(&plugin_name)
                };

                if plugin_dir.join("Cargo.toml").exists()
                    || plugin_dir.join("native").join("Cargo.toml").exists()
                {
                    println!(
                        "   \x1b[1;36m•\x1b[0m Compiling   dependency plugin '{}' from '{}'...",
                        plugin_name, pkg_name
                    );
                    if generate_package_fmi(&plugin_name, &plugin_dir, is_release) {
                        built_count += 1;
                    }
                }
            }
        }
    }

    // 3. Pure Flame package - inspect .fm files to generate cached .fmi interface if not already present
    let fmi_path = pkg_base.join(&pkg_name).join(format!("{}.fmi", pkg_name));
    if !fmi_path.exists() && (pkg_path.join("src").exists() || pkg_path.exists()) {
        if generate_package_fmi(&pkg_name, pkg_path, is_release) {
            built_count += 1;
        }
    }

    built_count
}

#[cfg(feature = "cli")]
pub fn build_all_dependency_plugins(is_release: bool) -> usize {
    let mut built_count = 0;
    let pkg_base = Path::new(".flame").join("pkg");
    if let Ok(entries) = fs::read_dir(&pkg_base) {
        for entry in entries.flatten() {
            let pkg_path = entry.path();
            if pkg_path.is_dir() {
                built_count += build_single_dependency_plugins(&pkg_path, is_release);
            }
        }
    }
    built_count
}


pub fn gen_fmi_from_rust_file(rust_file_path: &std::path::Path) {
    if !rust_file_path.exists() {
        println!(
            "\x1b[1;31merror:\x1b[0m file '{}' not found",
            rust_file_path.display()
        );
        return;
    }

    let module_name = rust_file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module")
        .to_string();

    let plugin_path = rust_file_path.parent().unwrap().parent().unwrap();

    let mut meta = FlameMeta {
        module: module_name.clone(),
        kind: "native".to_string(),
        lib: None,
        functions: Vec::new(),
        structs: Vec::new(),
        docs: None,
    };

    enrich_with_syn(&mut meta, plugin_path);

    if let Ok(meta_str) = serde_json::to_string_pretty(&meta) {
        let out_filename = format!("{}.fmi", module_name);
        if std::fs::write(&out_filename, meta_str).is_ok() {
            println!("\x1b[1;32mGenerated\x1b[0m {}", out_filename);
        } else {
            println!(
                "\x1b[1;31merror:\x1b[0m failed to write to {}",
                out_filename
            );
        }
    } else {
        println!("\x1b[1;31merror:\x1b[0m failed to serialize metadata");
    }
}

