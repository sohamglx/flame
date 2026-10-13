use crate::parser::Stmt;
use super::build::parse_file_stmts;
use crate::package_manager;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::utils::copy_dir_all;

pub fn create_new_project(name: &str) {
    let root = Path::new(name);
    if root.exists() {
        println!(
            "\x1b[1;31merror:\x1b[0m directory '{}' already exists",
            name
        );
        return;
    }

    println!(
        "Scaffolding a brand new Flame package: \x1b[1;32m{}\x1b[0m",
        name
    );

    // Create directories
    let dirs = vec!["src"];
    for d in dirs {
        let path = root.join(d);
        if let Err(e) = fs::create_dir_all(&path) {
            println!(
                "\x1b[1;31merror:\x1b[0m failed to create directory {:?}: {}",
                path, e
            );
            return;
        }
    }

    let clean_name = root
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(name);

    // Write flame.toml
    let toml_content = format!(
r#"[package]
name = "{}"
version = "0.1.0"
edition = "2026"
type = "executable"

[options]
# rust-plugins: "default" allows Rust plugins and native dependencies; "warn" allows them with compiler warnings; "deny" blocks all native code installation, import, and execution
rust-plugins = "default"
# closure-types: "default" provides flexible dynamic closure checking with full IDE autocomplete; "strict" enforces compile-time signature checks
closure-types = "default"

[dependencies]
"#,
        clean_name
    );
    fs::write(root.join("flame.toml"), toml_content).unwrap();

    // Write src/main.fm
    let main_flame = r#"

println("Hello, world!")

"#;
    fs::write(root.join("src/main.fm"), main_flame).unwrap();

    println!(
        "\x1b[1;32mCreated\x1b[0m binary (application) `{}` package",
        name
    );
}


pub fn create_new_library(name: &str) {
    let root = Path::new(name);
    if root.exists() {
        println!("\x1b[1;31merror:\x1b[0m directory '{}' already exists", name);
        return;
    }

    println!(
        "Scaffolding a brand new Flame library: \x1b[1;32m{}\x1b[0m",
        name
    );

    let _ = fs::create_dir_all(root.join("src"));
    let _ = fs::create_dir_all(root.join("tests"));

    let clean_name = root
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(name);

    let toml_content = format!(
r#"[package]
name = "{}"
version = "0.1.0"
edition = "2026"
type = "library"

[lib]
path = "src/lib.fm"

[options]
rust-plugins = "default"
closure-types = "default"

[dependencies]
"#,
        clean_name
    );
    fs::write(root.join("flame.toml"), toml_content).unwrap();

    let lib_flame = format!(
r#"package {}

@Docs("Computes the sum of two numbers.")
export fn add(a: Int, b: Int) -> Int {{
    a + b
}}
"#,
        clean_name
    );
    fs::write(root.join("src/lib.fm"), lib_flame).unwrap();

    let test_flame = format!(
r#"import {}

@Test
fn test_{}_api() {{
    if {}.add(2, 3) != 5 {{
        panic("Expected 5")
    }}
}}
"#,
        clean_name, clean_name, clean_name
    );
    fs::write(root.join("tests/test_api.fm"), test_flame).unwrap();

    println!(
        "\x1b[1;32mCreated\x1b[0m library `{}` package with entry point src/lib.fm",
        name
    );
}

fn scan_package_exports(src_dir: &Path) -> (Vec<PathBuf>, Vec<(String, String, PathBuf)>) {
    let mut files = Vec::new();
    let mut exports = Vec::new();

    fn collect_fm_files(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    collect_fm_files(&p, files);
                } else if p.extension().map_or(false, |e| e == "fm") {
                    files.push(p);
                }
            }
        }
    }

    if src_dir.exists() {
        collect_fm_files(src_dir, &mut files);
    }

    for file in &files {
        if let Ok(content) = fs::read_to_string(file) {
            if let Ok(stmts) = parse_file_stmts(file, &content) {
                for stmt in &stmts {
                    if let Stmt::ExportDecl(inner, _) = stmt {
                        match inner.as_ref() {
                            Stmt::FuncDecl { name, .. } => {
                                exports.push((name.clone(), "fn".to_string(), file.clone()));
                            }
                            Stmt::StructDecl { name, .. } => {
                                exports.push((name.clone(), "struct".to_string(), file.clone()));
                            }
                            Stmt::EnumDecl { name, .. } => {
                                exports.push((name.clone(), "enum".to_string(), file.clone()));
                            }
                            Stmt::TraitDecl { name, .. } => {
                                exports.push((name.clone(), "trait".to_string(), file.clone()));
                            }
                            Stmt::ConstDecl { name, .. } => {
                                exports.push((name.clone(), "const".to_string(), file.clone()));
                            }
                            Stmt::LetDecl { name, .. } => {
                                exports.push((name.clone(), "let".to_string(), file.clone()));
                            }
                            Stmt::AnnotationDecl { name, .. } => {
                                exports.push((name.clone(), "annotation".to_string(), file.clone()));
                            }
                            Stmt::ExprStmt(crate::parser::Expr::Identifier(name, _)) => {
                                exports.push((name.clone(), "re-export".to_string(), file.clone()));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    (files, exports)
}

#[cfg(feature = "cli")]
fn create_fmp_archive(src_dir: &Path, out_file: &Path) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = out_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = fs::File::create(out_file)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    fn add_dir_to_zip(
        zip: &mut zip::ZipWriter<fs::File>,
        base_dir: &Path,
        current_dir: &Path,
        options: zip::write::SimpleFileOptions,
    ) -> std::io::Result<()> {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path
                .strip_prefix(base_dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                zip.add_directory(&relative, options)?;
                add_dir_to_zip(zip, base_dir, &path, options)?;
            } else {
                zip.start_file(&relative, options)?;
                let content = fs::read(&path)?;
                zip.write_all(&content)?;
            }
        }
        Ok(())
    }

    add_dir_to_zip(&mut zip, src_dir, src_dir, options)?;
    zip.finish()?;
    Ok(())
}

pub fn package_project(args: &[String]) {
    let toml_path = Path::new("flame.toml");
    if !toml_path.exists() {
        println!("\x1b[1;31merror:\x1b[0m no flame.toml manifest file found.");
        return;
    }

    let manifest = match crate::utils::manifest::load_manifest_from_file(toml_path) {
        Some(m) => m,
        None => {
            println!("\x1b[1;31merror:\x1b[0m failed to parse flame.toml.");
            return;
        }
    };

    let pkg_name = manifest
        .package_name()
        .unwrap_or("app")
        .to_string();
    let is_library = manifest.is_library();
    let entry_point = manifest.entry_point();

    let is_check = args.contains(&"--check".to_string());
    let is_list = args.contains(&"--list".to_string());
    let out_override = if let Some(idx) = args.iter().position(|r| r == "--out") {
        args.get(idx + 1).map(PathBuf::from)
    } else {
        None
    };

    let (src_files, exports) = scan_package_exports(Path::new("src"));

    if is_list {
        println!("\x1b[1;36mPackage:\x1b[0m {} (type: {})", pkg_name, manifest.package_type());
        println!("Entry point: {}", entry_point);
        println!("\nSource files ({}):", src_files.len());
        for f in &src_files {
            println!("  • {}", f.display());
        }
        println!("\nPublic exports ({}):", exports.len());
        for (name, kind, file) in &exports {
            println!("  • export {} {} ({})", kind, name, file.display());
        }
        return;
    }

    if is_check {
        println!("\x1b[1;36mChecking package\x1b[0m {} ...", pkg_name);
        if !Path::new(&entry_point).exists() {
            println!(
                "\x1b[1;31merror:\x1b[0m declared entry point '{}' does not exist.",
                entry_point
            );
            return;
        }
        if is_library && exports.is_empty() {
            println!(
                "\x1b[1;33mwarning:\x1b[0m library package '{}' has no declared public exports.",
                pkg_name
            );
        } else {
            println!(
                "\x1b[1;32m✓\x1b[0m Validated {} public exports across {} source files.",
                exports.len(),
                src_files.len()
            );
        }
        println!("\x1b[1;32m✓ Package check passed successfully.\x1b[0m");
        return;
    }

    println!("\x1b[1;36m Packaging\x1b[0m {} ...", pkg_name);

    if is_library && exports.is_empty() {
        println!(
            "\x1b[1;33mwarning:\x1b[0m package '{}' does not export anything. Is this intentional?",
            pkg_name
        );
    } else {
        println!(
            "\x1b[1;32m  Verified\x1b[0m package '{}' ({} public exports in {} source files).",
            pkg_name,
            exports.len(),
            src_files.len()
        );
    }

    let pkg_out_dir = Path::new("target").join("pkg").join(&pkg_name);
    let _ = fs::create_dir_all(&pkg_out_dir);

    let _ = fs::copy("flame.toml", pkg_out_dir.join("flame.toml"));
    if Path::new("src").exists() {
        let _ = copy_dir_all("src", pkg_out_dir.join("src"));
    }
    if Path::new("native").exists() {
        let _ = copy_dir_all("native", pkg_out_dir.join("native"));
    }
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("fmi") {
                let _ = fs::copy(&path, pkg_out_dir.join(path.file_name().unwrap()));
            }
        }
    }

    let out_archive = out_override.unwrap_or_else(|| {
        Path::new("target")
            .join("pkg")
            .join(format!("{}.fmp", pkg_name))
    });

    #[cfg(feature = "cli")]
    if let Err(e) = create_fmp_archive(&pkg_out_dir, &out_archive) {
        println!(
            "\x1b[1;33mwarning:\x1b[0m failed creating .fmp archive: {}",
            e
        );
    }

    println!(
        "\x1b[1;32m  Packaged\x1b[0m successfully to {} and {}",
        pkg_out_dir.display(),
        out_archive.display()
    );
}


pub fn init_native_bridge(plugin_name: &str) {
    let toml_path = Path::new("flame.toml");
    if !toml_path.exists() {
        println!(
            "\x1b[1;31merror:\x1b[0m no flame.toml manifest file found in the current directory."
        );
        println!("help: run this command inside a valid Flame project folder.");
        return;
    }

    // Check package name in flame.toml - plugin name MUST be different from package name!
    let pkg_name = if let Ok(content) = fs::read_to_string(toml_path) {
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
            "help: choose a distinct plugin name (e.g. 'server', 'native_core', or 'bridge') to avoid Cargo workspace and linkage naming collisions."
        );
        return;
    }

    println!(
        "\x1b[1;36mInitializing\x1b[0m Rust plugin '{}' environment...",
        plugin_name
    );

    // Create native directory and native/src directory
    let native_dir = Path::new("native");
    let src_dir = native_dir.join("src");
    if !src_dir.exists() {
        if let Err(e) = fs::create_dir_all(&src_dir) {
            println!(
                "\x1b[1;31merror:\x1b[0m failed to create 'native/src/' directory: {}",
                e
            );
            return;
        }
    }

    // Write native/src/lib.rs
    let lib_rs = r#"pub fn rust_add(a: i64, b: i64) -> i64 {
    a + b
}
"#;
    let lib_path = src_dir.join("lib.rs");
    if !lib_path.exists() {
        fs::write(&lib_path, lib_rs).unwrap();
        println!("\x1b[1;32mCreated\x1b[0m {:?}", lib_path);
    }

    // Write native/Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]

[profile.dev]
split-debuginfo = "unpacked"

[profile.release]
opt-level = 3
lto = "thin"
strip = true
codegen-units = 1
"#,
        plugin_name
    );
    let cargo_path = native_dir.join("Cargo.toml");
    if !cargo_path.exists() {
        fs::write(&cargo_path, cargo_toml).unwrap();
        println!("\x1b[1;32mCreated\x1b[0m {:?}", cargo_path);
    }

    // Update flame.toml to append [plugins] if not present
    let mut toml_content = fs::read_to_string(toml_path).unwrap();
    if !toml_content.contains(&format!("{} =", plugin_name))
        && !toml_content.contains(&format!("{}=", plugin_name))
    {
        if !toml_content.contains("[plugins]") {
            toml_content.push_str(&format!("\n[plugins]\n{} = \"./native\"\n", plugin_name));
        } else if let Some(idx) = toml_content.find("[plugins]") {
            let insert_pos = idx + "[plugins]".len();
            toml_content.insert_str(insert_pos, &format!("\n{} = \"./native\"", plugin_name));
        }
        fs::write(toml_path, toml_content).unwrap();
        println!(
            "\x1b[1;32mUpdated\x1b[0m flame.toml to reference native plugin '{}'.",
            plugin_name
        );
    }

    println!("\x1b[1;32mFinished\x1b[0m native initialization. Run `flame build` to compile.");
}
