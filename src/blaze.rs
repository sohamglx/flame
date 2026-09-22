use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub const EMBEDDED_BLAZE_STD: &[(&str, &str)] = &[
    ("annotations.fm", include_str!("../Blaze/std/annotations.fm")),
    ("builtins.fm", include_str!("../Blaze/std/builtins.fm")),
    ("byte.fm", include_str!("../Blaze/std/byte.fm")),
    ("camera.fm", include_str!("../Blaze/std/camera.fm")),
    ("desktop.fm", include_str!("../Blaze/std/desktop.fm")),
    ("env.fm", include_str!("../Blaze/std/env.fm")),
    ("fs.fm", include_str!("../Blaze/std/fs.fm")),
    ("json.fm", include_str!("../Blaze/std/json.fm")),
    ("math.fm", include_str!("../Blaze/std/math.fm")),
    ("net.fm", include_str!("../Blaze/std/net.fm")),
    ("os.fm", include_str!("../Blaze/std/os.fm")),
    ("process.fm", include_str!("../Blaze/std/process.fm")),
    ("thread.fm", include_str!("../Blaze/std/thread.fm")),
    ("time.fm", include_str!("../Blaze/std/time.fm")),
    ("unit.fm", include_str!("../Blaze/std/unit.fm")),
    ("web.fm", include_str!("../Blaze/std/web.fm")),
    ("window.fm", include_str!("../Blaze/std/window.fm")),
];

pub fn get_blaze_target_std_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. Explicit BLAZE_HOME environment variable
    if let Ok(val) = std::env::var("BLAZE_HOME") {
        let p = PathBuf::from(val);
        if p.ends_with("std") {
            dirs.push(p);
        } else {
            dirs.push(p.join("std"));
        }
    }

    // 2. Explicit FLAME_BLAZE_DIR environment variable
    if let Ok(val) = std::env::var("FLAME_BLAZE_DIR") {
        let p = PathBuf::from(val);
        if p.ends_with("std") {
            dirs.push(p);
        } else {
            dirs.push(p.join("std"));
        }
    }

    // 3. User standard locations
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local_app).join("Blaze").join("std"));
        }
        if let Ok(user_prof) = std::env::var("USERPROFILE") {
            dirs.push(PathBuf::from(user_prof).join(".blaze").join("std"));
        }
        if let Ok(prog_files) = std::env::var("ProgramFiles") {
            let p = PathBuf::from(prog_files).join("Blaze").join("std");
            if p.exists() {
                dirs.push(p);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".blaze").join("std"));
        }
        let usr_local = PathBuf::from("/usr/local/share/blaze/std");
        if usr_local.exists() {
            dirs.push(usr_local);
        }
        let usr_share = PathBuf::from("/usr/share/blaze/std");
        if usr_share.exists() {
            dirs.push(usr_share);
        }
    }

    // Ensure at least one primary target directory exists in candidate list
    if dirs.is_empty() {
        #[cfg(target_os = "windows")]
        {
            if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
                dirs.push(PathBuf::from(local_app).join("Blaze").join("std"));
            } else if let Ok(user_prof) = std::env::var("USERPROFILE") {
                dirs.push(PathBuf::from(user_prof).join(".blaze").join("std"));
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(home) = std::env::var("HOME") {
                dirs.push(PathBuf::from(home).join(".blaze").join("std"));
            }
        }
    }

    // Deduplicate directories by canonical or normalized path string
    let mut unique_dirs = Vec::new();
    for d in dirs {
        let norm = d.to_string_lossy().to_lowercase().replace('\\', "/");
        if !unique_dirs.iter().any(|existing: &PathBuf| {
            existing.to_string_lossy().to_lowercase().replace('\\', "/") == norm
        }) {
            unique_dirs.push(d);
        }
    }

    unique_dirs
}

/// Automatically ensures standard library definitions exist on the machine.
/// Returns the root Blaze directory if successful.
pub fn ensure_blaze_definitions_installed() -> Option<PathBuf> {
    let targets = get_blaze_target_std_dirs();
    let primary_std = targets.first()?.clone();
    let primary_blaze_root = primary_std.parent()?.to_path_buf();

    if !primary_std.exists() {
        let _ = fs::create_dir_all(&primary_std);
    }

    // Check if files are already populated
    let has_files = fs::read_dir(&primary_std)
        .ok()
        .map(|entries| entries.flatten().any(|e| e.path().extension().map_or(false, |ext| ext == "fm")))
        .unwrap_or(false);

    if !has_files {
        for (name, content) in EMBEDDED_BLAZE_STD {
            let target_file = primary_std.join(name);
            let _ = fs::write(&target_file, content);
        }
    }

    if primary_std.exists() {
        Some(primary_blaze_root)
    } else {
        None
    }
}

/// Updates all Blaze standard library directories on the system with the latest definitions.
/// Returns the number of directories successfully updated.
pub fn update_blaze_definitions(prefer_remote: bool) -> Result<usize, String> {
    let target_dirs = get_blaze_target_std_dirs();
    if target_dirs.is_empty() {
        return Err("No candidate Blaze definition directories could be resolved.".to_string());
    }

    let mut files_to_write: HashMap<String, Vec<u8>> = HashMap::new();

    // 1. If not explicitly preferring remote, check local workspace first
    if !prefer_remote {
        let local_candidates = [
            PathBuf::from("Blaze").join("std"),
            PathBuf::from("std"),
            if let Ok(exe) = std::env::current_exe() {
                exe.parent().map(|p| p.join("Blaze").join("std")).unwrap_or_default()
            } else {
                PathBuf::new()
            },
            if let Ok(exe) = std::env::current_exe() {
                exe.parent().and_then(|p| p.parent()).map(|p| p.join("Blaze").join("std")).unwrap_or_default()
            } else {
                PathBuf::new()
            },
        ];

        for cand in &local_candidates {
            if cand.exists() && cand.is_dir() {
                if let Ok(entries) = fs::read_dir(cand) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("fm") {
                            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                                if let Ok(content) = fs::read(&path) {
                                    files_to_write.insert(file_name.to_string(), content);
                                }
                            }
                        }
                    }
                }
                if !files_to_write.is_empty() {
                    println!(
                        "  \x1b[1;32mLoaded\x1b[0m {} definitions from local workspace: \x1b[1m{}\x1b[0m",
                        files_to_write.len(),
                        cand.display()
                    );
                    break;
                }
            }
        }
    }

    // 2. If files_to_write is still empty, fetch latest from GitHub repository main branch archive
    #[cfg(feature = "cli")]
    if files_to_write.is_empty() {
        use std::io::Write;
        print!("  \x1b[1;36mFetching\x1b[0m latest definitions from GitHub (shoya-129/flame:main)... ");
        let _ = std::io::stdout().flush();

        let client = reqwest::blocking::Client::builder()
            .user_agent("Flame-Toolchain")
            .timeout(std::time::Duration::from_secs(20))
            .build();

        if let Ok(client) = client {
            let archive_url = "https://github.com/shoya-129/flame/archive/refs/heads/main.zip";
            if let Ok(resp) = client.get(archive_url).send() {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes() {
                        let cursor = std::io::Cursor::new(bytes);
                        if let Ok(mut archive) = zip::ZipArchive::new(cursor) {
                            for i in 0..archive.len() {
                                if let Ok(mut file) = archive.by_index(i) {
                                    let name = file.name().replace('\\', "/");
                                    if (name.contains("/Blaze/std/") || name.contains("/std/"))
                                        && name.ends_with(".fm")
                                    {
                                        if let Some(file_name) = std::path::Path::new(&name).file_name().and_then(|s| s.to_str()) {
                                            let mut content = Vec::new();
                                            use std::io::Read;
                                            if file.read_to_end(&mut content).is_ok() && !content.is_empty() {
                                                files_to_write.insert(file_name.to_string(), content);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !files_to_write.is_empty() {
            println!("\x1b[1;32m✓\x1b[0m ({} files downloaded)", files_to_write.len());
        } else {
            println!("\x1b[1;33m(remote archive unavailable, using bundled definitions)\x1b[0m");
        }
    }

    // 3. Fallback: Bundled toolchain definitions
    if files_to_write.is_empty() {
        for (name, content) in EMBEDDED_BLAZE_STD {
            files_to_write.insert(name.to_string(), content.as_bytes().to_vec());
        }
        println!(
            "  \x1b[1;32mUsing\x1b[0m bundled toolchain standard library definitions ({} files)",
            files_to_write.len()
        );
    }

    if files_to_write.is_empty() {
        return Err("Could not locate any standard library definitions to install.".to_string());
    }

    // 4. Install files to each destination directory
    let mut updated_directories = 0;
    for dir in &target_dirs {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!(
                "  \x1b[1;33mwarning:\x1b[0m could not create directory '{}': {}",
                dir.display(),
                e
            );
            continue;
        }

        let mut file_count = 0;
        for (name, content) in &files_to_write {
            let target_file = dir.join(name);
            if fs::write(&target_file, content).is_ok() {
                file_count += 1;
            }
        }

        if file_count > 0 {
            println!(
                "  \x1b[1;32m✓\x1b[0m Updated {} standard library definitions in: \x1b[1m{}\x1b[0m",
                file_count,
                dir.display()
            );
            updated_directories += 1;
        }
    }

    Ok(updated_directories)
}

use std::sync::OnceLock;

static STD_DOCS_CACHE: OnceLock<(HashMap<(String, String), String>, HashMap<String, String>)> =
    OnceLock::new();

fn init_std_docs_cache() -> (HashMap<(String, String), String>, HashMap<String, String>) {
    let mut func_docs = HashMap::new();
    let mut mod_docs = HashMap::new();

    let re = regex::Regex::new(
        r#"(?s)@Docs\(\s*"((?:[^"\\]|\\.)*)"\s*\)\s*(?:export\s+)?(?:(package|struct|fn|const|enum))\s+([a-zA-Z_][\w]*)"#,
    )
    .unwrap();

    let impl_re = regex::Regex::new(r#"(?s)impl\s+([a-zA-Z_][\w]*)\s*\{(.*?)\}"#).unwrap();

    for (file_name, content) in EMBEDDED_BLAZE_STD {
        let base_name = file_name.strip_suffix(".fm").unwrap_or(file_name);
        let mod_prefixes = [
            base_name.to_string(),
            format!("std.{}", base_name),
        ];

        // 1. Match top-level annotated declarations
        for cap in re.captures_iter(content) {
            let raw_doc = &cap[1];
            let unescaped = raw_doc
                .replace("\\n", "\n")
                .replace("\\\"", "\"")
                .replace("\\\\", "\\");
            let kind = &cap[2];
            let name = &cap[3];

            if kind == "package" {
                for p in &mod_prefixes {
                    mod_docs.insert(p.clone(), unescaped.clone());
                }
            } else {
                for p in &mod_prefixes {
                    func_docs.insert((p.clone(), name.to_string()), unescaped.clone());
                }
            }
        }

        // 2. Match methods inside impl blocks
        for impl_cap in impl_re.captures_iter(content) {
            let struct_name = &impl_cap[1];
            let impl_body = &impl_cap[2];

            for cap in re.captures_iter(impl_body) {
                let raw_doc = &cap[1];
                let unescaped = raw_doc
                    .replace("\\n", "\n")
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
                let name = &cap[3];

                for p in &mod_prefixes {
                    func_docs.insert((p.clone(), name.to_string()), unescaped.clone());
                    func_docs.insert(
                        (format!("{}.{}", p, struct_name), name.to_string()),
                        unescaped.clone(),
                    );
                    func_docs.insert(
                        (format!("{}.{}", p, struct_name.to_lowercase()), name.to_string()),
                        unescaped.clone(),
                    );
                }
                func_docs.insert((struct_name.to_string(), name.to_string()), unescaped.clone());
                func_docs.insert((struct_name.to_lowercase(), name.to_string()), unescaped.clone());
                func_docs.insert(
                    (format!("std.{}", struct_name.to_lowercase()), name.to_string()),
                    unescaped.clone(),
                );
            }
        }
    }

    (func_docs, mod_docs)
}

pub fn get_std_function_doc(module: &str, function: &str) -> Option<String> {
    let (func_docs, _) = STD_DOCS_CACHE.get_or_init(init_std_docs_cache);
    let mod_clean = module.strip_prefix("std.").unwrap_or(module);
    func_docs
        .get(&(mod_clean.to_string(), function.to_string()))
        .or_else(|| func_docs.get(&(format!("std.{}", mod_clean), function.to_string())))
        .or_else(|| func_docs.get(&(module.to_string(), function.to_string())))
        .cloned()
}

pub fn get_std_module_doc(module: &str) -> Option<String> {
    let (_, mod_docs) = STD_DOCS_CACHE.get_or_init(init_std_docs_cache);
    let mod_clean = module.strip_prefix("std.").unwrap_or(module);
    mod_docs
        .get(mod_clean)
        .or_else(|| mod_docs.get(&format!("std.{}", mod_clean)))
        .or_else(|| mod_docs.get(module))
        .cloned()
}
