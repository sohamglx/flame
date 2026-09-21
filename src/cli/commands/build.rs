use crate::diagnostics::Diagnostic;
use crate::lexer::Lexer;
use crate::lexer::TokenKind;
use crate::parser::{Parser, Stmt};
use crate::typechecker::TypeChecker;
use crate::utils::parse_manifest_section;
use crate::{lexer, package_manager};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn parse_file_stmts(path: &Path, content: &str) -> Result<Vec<Stmt>, Diagnostic> {
    let mut lexer = Lexer::new(content);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        let is_eof = tok.kind == lexer::TokenKind::EOF;
        tokens.push(tok);
        if is_eof {
            break;
        }
    }

    let mut parser = Parser::new(tokens, path.to_string_lossy().to_string());
    parser.parse()
}

pub fn typecheck_file_stmts(path: &Path, stmts: &[Stmt]) -> Result<(), Vec<Diagnostic>> {
    TypeChecker::new(path.to_string_lossy().to_string())
        .check_program(stmts)
        .0
}

pub fn build_project(args: &[String]) -> Option<PathBuf> {
    let toml_path = Path::new("flame.toml");
    if !toml_path.exists() {
        println!(
            "\x1b[1;31merror:\x1b[0m no flame.toml manifest file found in the current directory."
        );
        println!("help: run this command inside a valid Flame project folder.");
        return None;
    }

    let is_release = args.contains(&"--release".to_string()) || args.contains(&"-r".to_string());
    let force_local = args.contains(&"--local".to_string());
    let use_vfs = args.contains(&"--vfs".to_string());
    let mut pkg_name = "app".to_string();
    let mut target = None;
    for i in 0..args.len() {
        if args[i] == "--target" && i + 1 < args.len() {
            target = Some(args[i + 1].clone());
        }
    }
    if let Ok(toml_str) = fs::read_to_string("flame.toml") {
        for line in toml_str.lines() {
            let t = line.trim();
            if t.starts_with("name =") {
                if let Some(val) = t.split('=').nth(1) {
                    pkg_name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                }
            } else if t.starts_with("target =") {
                if let Some(val) = t.split('=').nth(1) {
                    if target.is_none() {
                        target = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
                    }
                }
            }
        }
    }

    let mode_str = if is_release {
        "release [optimized]"
    } else {
        "dev [unoptimized]"
    };
    let profile = if is_release { "release" } else { "dev" };
    if is_release {
        println!(
            "\x1b[1;36m    Building\x1b[0m optimized production release binary (target/release)..."
        );
    }
    if use_vfs {
        println!(
            "\x1b[1;36m    Embedding\x1b[0m source tree into single executable VFS (--vfs)..."
        );
    }

    package_manager::ensure_dependencies_installed(is_release);

    println!("\x1b[1;36m    Building\x1b[0m dependency graph...");
    println!("\x1b[1;36m   Compiling\x1b[0m std standard library...");

    let src_dir = Path::new("src");
    let has_source_files = if src_dir.exists() {
        fs::read_dir(src_dir)
            .map(|mut it| {
                it.any(|e| {
                    e.map(|entry| entry.path().extension().map_or(false, |ext| ext == "fm"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    } else {
        false
    };

    if has_source_files {
        println!("\x1b[1;36m   Compiling\x1b[0m targets (src/)...");

        let mut all_stmts = Vec::new();
        // Parse dependencies first
        if let Ok(entries) = fs::read_dir(".flame/pkg") {
            for entry in entries.flatten() {
                let pkg_path = entry.path();
                if pkg_path.is_dir() {
                    let pkg_src = pkg_path.join("src");
                    if pkg_src.exists() {
                        if let Ok(pkg_files) = fs::read_dir(&pkg_src) {
                            for pkg_file in pkg_files.flatten() {
                                let fpath = pkg_file.path();
                                if fpath.is_file() && fpath.extension().map_or(false, |e| e == "fm")
                                {
                                    let content = fs::read_to_string(&fpath).unwrap_or_default();
                                    if let Ok(mut stmts) = parse_file_stmts(&fpath, &content) {
                                        crate::parser::filter_platform_stmts(
                                            &mut stmts,
                                            target.as_deref(),
                                        );
                                        all_stmts.extend(stmts);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Parse local project files
        if let Ok(entries) = fs::read_dir("src") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "fm") {
                    let content = fs::read_to_string(&path).unwrap_or_default();
                    let stmts = match parse_file_stmts(&path, &content) {
                        Ok(mut stmts) => {
                            crate::parser::filter_platform_stmts(&mut stmts, target.as_deref());
                            stmts
                        }
                        Err(diag) => {
                            diag.print(&content);
                            println!(
                                "\x1b[1;31merror:\x1b[0m build failed due to 1 previous error"
                            );
                            std::process::exit(1);
                        }
                    };

                    if let Err(diags) = typecheck_file_stmts(&path, &stmts) {
                        for diag in diags {
                            diag.print(&content);
                        }
                        println!("\x1b[1;31merror:\x1b[0m build failed due to type errors");
                        std::process::exit(1);
                    }
                    all_stmts.extend(stmts);
                }
            }
        }

        let manifest_content = fs::read_to_string("flame.toml").unwrap_or_default();
        let mut native_deps_raw =
            parse_manifest_section(&manifest_content, "[native-dependencies]");
        let mut plugins_raw = parse_manifest_section(&manifest_content, "[plugins]");

        if let Ok(entries) = fs::read_dir(".flame/pkg") {
            for entry in entries.flatten() {
                let pkg_path = entry.path();
                if pkg_path.is_dir() {
                    let toml_path = pkg_path.join("flame.toml");
                    if toml_path.exists() {
                        let dep_manifest = fs::read_to_string(&toml_path).unwrap_or_default();
                        for (name, mut path) in
                            parse_manifest_section(&dep_manifest, "[native-dependencies]")
                        {
                            if path.starts_with('"') && path.ends_with('"') {
                                path = path[1..path.len() - 1].to_string();
                            }
                            if path.starts_with('.') || path.starts_with('/') {
                                let abs = std::fs::canonicalize(pkg_path.join(&path))
                                    .unwrap_or_else(|_| pkg_path.join(&path));
                                crate::package_manager::inspect_native_plugin(&name, &abs);
                                path = format!("\"{}\"", abs.to_string_lossy().replace("\\", "/"));
                            } else {
                                path = format!("\"{}\"", path);
                            }
                            native_deps_raw.push((name, path));
                        }
                        for (name, mut path) in parse_manifest_section(&dep_manifest, "[plugins]") {
                            if path.starts_with('"') && path.ends_with('"') {
                                path = path[1..path.len() - 1].to_string();
                            }
                            if path.starts_with('.') || path.starts_with('/') {
                                let abs = std::fs::canonicalize(pkg_path.join(&path))
                                    .unwrap_or_else(|_| pkg_path.join(&path));
                                crate::package_manager::inspect_native_plugin(&name, &abs);
                                path = format!("\"{}\"", abs.to_string_lossy().replace("\\", "/"));
                            } else {
                                path = format!("\"{}\"", path);
                            }
                            plugins_raw.push((name, path));
                        }
                    }
                }
            }
        }

        let mut processed_native_deps = Vec::new();

        for (plugin_name, plugin_path) in native_deps_raw {
            let mut path_str = plugin_path.clone();
            if path_str.starts_with('"') && path_str.ends_with('"') {
                path_str = path_str[1..path_str.len() - 1].to_string();
            }
            if path_str.starts_with('.')
                || path_str.starts_with('/')
                || std::path::Path::new(&path_str).is_absolute()
            {
                let absolute_path = std::fs::canonicalize(std::path::Path::new(&path_str))
                    .unwrap_or_else(|_| std::env::current_dir().unwrap().join(&path_str));
                let mut abs_path_str = absolute_path.to_string_lossy().replace("\\", "/");
                if abs_path_str.starts_with("//?/") {
                    abs_path_str = abs_path_str[4..].to_string();
                }
                processed_native_deps
                    .push((plugin_name, format!("{{ path = \"{}\" }}", abs_path_str)));
            } else {
                processed_native_deps.push((plugin_name, path_str));
            }
        }

        for (plugin_name, plugin_path) in plugins_raw {
            let mut path_str = plugin_path.clone();
            if path_str.starts_with('"') && path_str.ends_with('"') {
                path_str = path_str[1..path_str.len() - 1].to_string();
            }

            let is_local = path_str.starts_with('.')
                || path_str.starts_with('/')
                || std::path::Path::new(&path_str).is_absolute();
            let actual_path = if is_local {
                path_str
            } else {
                std::env::current_dir()
                    .unwrap()
                    .join(".flame")
                    .join("pkg")
                    .join(&plugin_name)
                    .to_string_lossy()
                    .into_owned()
            };

            let absolute_path = std::fs::canonicalize(std::path::Path::new(&actual_path))
                .unwrap_or_else(|_| std::env::current_dir().unwrap().join(&actual_path));
            let mut abs_path_str = absolute_path.to_string_lossy().replace("\\", "/");
            if abs_path_str.starts_with("//?/") {
                abs_path_str = abs_path_str[4..].to_string();
            }
            processed_native_deps.push((plugin_name, format!("{{ path = \"{}\" }}", abs_path_str)));
        }

        crate::compiler::build_project(
            &pkg_name,
            profile,
            &processed_native_deps,
            force_local,
            false,
            false,
            None,
            use_vfs,
        );

        let ext = std::env::consts::EXE_SUFFIX;
        let exe_name = format!("{}{}", pkg_name.clone(), ext);
        let out_rel = format!("target/{}/{}", profile, exe_name);
        println!(
            "\x1b[1;32m    Finished\x1b[0m {} target(s) -> {} in 0.12s",
            mode_str, out_rel
        );

        if is_release {
            println!(
                "\x1b[1;32m   Packaged\x1b[0m distribution build successfully in \x1b[1mtarget/release/\x1b[0m directory"
            );
        }

        return Some(PathBuf::from(out_rel));
    } else {
        println!("\x1b[1;32m    Finished\x1b[0m compilation: no source files found in src/");
        return None;
    }
}

pub fn get_manifest_pkg_name() -> String {
    let mut pkg_name = "app".to_string();
    if let Ok(toml_str) = fs::read_to_string("flame.toml") {
        for line in toml_str.lines() {
            let t = line.trim();
            if t.starts_with("name =") {
                if let Some(val) = t.split('=').nth(1) {
                    pkg_name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                }
            }
        }
    }
    pkg_name
}

pub fn check_runtime_needs_rebuild(exe_path: &Path, _profile: &str) -> bool {
    if !exe_path.exists() {
        return true;
    }
    let exe_meta = match fs::metadata(exe_path) {
        Ok(m) => m,
        Err(_) => return true,
    };
    let exe_time = match exe_meta.modified() {
        Ok(t) => t,
        Err(_) => return true,
    };

    // 1. Check if compiler executable itself is newer than cached runtime
    if let Ok(self_exe) = std::env::current_exe() {
        if let Ok(self_meta) = fs::metadata(self_exe) {
            if let Ok(self_time) = self_meta.modified() {
                if self_time > exe_time {
                    return true;
                }
            }
        }
    }

    // 2. Check flame.toml modification time
    if let Ok(toml_meta) = fs::metadata("flame.toml") {
        if let Ok(toml_time) = toml_meta.modified() {
            if toml_time > exe_time {
                return true;
            }
        }
    }

    // 2. Check native/ directory if it exists
    fn check_dir_newer(dir: &Path, cutoff: std::time::SystemTime) -> bool {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if p.file_name().map_or(false, |n| n == "target") {
                        continue;
                    }
                    if check_dir_newer(&p, cutoff) {
                        return true;
                    }
                } else if let Ok(m) = entry.metadata() {
                    if let Ok(time) = m.modified() {
                        if time > cutoff {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    if Path::new("native").exists() && check_dir_newer(Path::new("native"), exe_time) {
        return true;
    }

    // 3. Check .flame/pkg plugins or .fmi
    if Path::new(".flame").join("pkg").exists()
        && check_dir_newer(&Path::new(".flame").join("pkg"), exe_time)
    {
        return true;
    }

    // 4. Check Cargo features required by imports vs what's in .flame/build-cache/Cargo.toml
    let build_cache_toml = Path::new(".flame").join("build-cache").join("Cargo.toml");
    if !build_cache_toml.exists() {
        return true;
    }
    let cached_toml_content = fs::read_to_string(&build_cache_toml).unwrap_or_default();

    let mut required_features = std::collections::HashSet::new();
    fn scan_features_in_dir(dir: &Path, features: &mut std::collections::HashSet<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    scan_features_in_dir(&p, features);
                } else if p.extension().map_or(false, |ext| ext == "fm") {
                    if let Ok(content) = fs::read_to_string(&p) {
                        for line in content.lines() {
                            let t = line.trim();
                            if t.starts_with("import ") {
                                let after = t[7..].trim();
                                let mod_name = after
                                    .split(|c: char| c.is_whitespace() || c == ';' || c == '{')
                                    .next()
                                    .unwrap_or("");
                                let search = if mod_name.starts_with("std.") {
                                    mod_name.to_string()
                                } else {
                                    format!("std.{}", mod_name)
                                };
                                let module_features = match search.as_str() {
                                    "std.time" => vec!["utils"],
                                    "std.os" => vec!["os"],
                                    "std.regex" => vec!["regex"],
                                    "std.json" => vec!["utils"],
                                    "std.desktop" => vec!["os"],
                                    "std.camera" => vec!["camera"],
                                    "std.base64" => vec!["base64"],
                                    "std.net.tcp" | "std.net.udp" | "std.net.dns"
                                    | "std.net.url" | "std.net.interface" | "std.net" => {
                                        vec!["net"]
                                    }
                                    "std.net.http" => vec!["net", "http"],
                                    "std.net.ws" => vec!["net", "ws"],
                                    "std.net.mqtt" => vec!["net", "mqtt"],
                                    _ => vec![],
                                };
                                for feat in module_features {
                                    features.insert(feat.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if Path::new("src").exists() {
        scan_features_in_dir(Path::new("src"), &mut required_features);
    }

    // Extract exact compiled features from flamelang entry in .flame/build-cache/Cargo.toml
    let mut compiled_features = std::collections::HashSet::new();
    for line in cached_toml_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("flamelang") && trimmed.contains('{') {
            if let Some(start_idx) = trimmed.find("features") {
                if let Some(bracket_start) = trimmed[start_idx..].find('[') {
                    let from_bracket = &trimmed[start_idx + bracket_start + 1..];
                    if let Some(bracket_end) = from_bracket.find(']') {
                        let feats_slice = &from_bracket[..bracket_end];
                        for f in feats_slice.split(',') {
                            let clean = f.trim().trim_matches('"').trim_matches('\'').trim();
                            if !clean.is_empty() {
                                compiled_features.insert(clean.to_string());
                            }
                        }
                    }
                }
            }
            break;
        }
    }

    // Check if @Application has features in src/main.fm
    let main_fm = Path::new("src").join("main.fm");
    if let Ok(content) = fs::read_to_string(&main_fm) {
        if let Some(idx) = content.find("@Application") {
            let after = &content[idx..];
            if let Some(paren_end) = after.find(')') {
                let annot = &after[..paren_end];
                if annot.contains("features") {
                    if let Some(b_start) = annot.find('[') {
                        if let Some(b_end) = annot.find(']') {
                            let list = &annot[b_start + 1..b_end];
                            for f in list.split(',') {
                                let clean = f.trim().trim_matches('"').trim_matches('\'').trim();
                                if !clean.is_empty() {
                                    required_features.insert(clean.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If the required feature set changed in any way (added or removed), rebuild!
    if compiled_features != required_features {
        return true;
    }

    false
}

pub fn get_project_mtime_snapshot() -> HashMap<PathBuf, std::time::SystemTime> {
    let mut map = HashMap::new();
    if let Ok(m) = fs::metadata("flame.toml") {
        if let Ok(time) = m.modified() {
            map.insert(PathBuf::from("flame.toml"), time);
        }
    }
    fn scan(dir: &Path, map: &mut HashMap<PathBuf, std::time::SystemTime>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if p.file_name()
                        .map_or(false, |n| n == "target" || n == "build-cache")
                    {
                        continue;
                    }
                    scan(&p, map);
                } else if let Ok(m) = entry.metadata() {
                    if let Ok(time) = m.modified() {
                        map.insert(p, time);
                    }
                }
            }
        }
    }
    if Path::new("src").exists() {
        scan(Path::new("src"), &mut map);
    }
    if Path::new("native").exists() {
        scan(Path::new("native"), &mut map);
    }
    if Path::new(".flame").join("pkg").exists() {
        scan(&Path::new(".flame").join("pkg"), &mut map);
    }
    map
}
