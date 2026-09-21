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

    // Write flame.toml
    let toml_content = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2026\"\ntype = \"executable\"\n\n[dependencies]\n",
        name
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


pub fn package_project(_args: &[String]) {
    let toml_path = Path::new("flame.toml");
    if !toml_path.exists() {
        println!("\x1b[1;31merror:\x1b[0m no flame.toml manifest file found.");
        return;
    }

    let mut pkg_name = "app".to_string();
    let mut is_pkg = false;
    if let Ok(toml_str) = fs::read_to_string("flame.toml") {
        for line in toml_str.lines() {
            let t = line.trim();
            if t.starts_with("name =") {
                if let Some(val) = t.split('=').nth(1) {
                    pkg_name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                }
            } else if t.starts_with("type =") {
                if let Some(val) = t.split('=').nth(1) {
                    let parsed_type = val
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_lowercase();
                    if parsed_type == "pkg" || parsed_type == "lib" {
                        is_pkg = true;
                    }
                }
            }
        }
    }

    if !is_pkg {
        println!("\x1b[1;33mwarning:\x1b[0m project type is not 'pkg'. packaging anyway.");
    }

    println!("\x1b[1;36m Packaging\x1b[0m {} ...", pkg_name);
    let src_dir = Path::new("src");
    let mut has_exports = false;
    if src_dir.exists() {
        if let Ok(entries) = fs::read_dir(src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |e| e == "fm") {
                    let content = fs::read_to_string(&path).unwrap_or_default();
                    if let Ok(stmts) = parse_file_stmts(&path, &content) {
                        for stmt in &stmts {
                            if matches!(stmt, Stmt::ExportDecl(_, _)) {
                                has_exports = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if !has_exports {
        println!(
            "\x1b[1;33mwarning:\x1b[0m package '{}' does not export anything. Is this intentional?",
            pkg_name
        );
    } else {
        println!(
            "\x1b[1;32m  Verified\x1b[0m package '{}' exports valid symbols.",
            pkg_name
        );
    }

    let pkg_out_dir = Path::new("target").join("pkg").join(&pkg_name);
    let _ = fs::create_dir_all(&pkg_out_dir);

    if Path::new("flame.toml").exists() {
        let _ = fs::copy("flame.toml", pkg_out_dir.join("flame.toml"));
    }
    if Path::new("src").exists() {
        let _ = copy_dir_all("src", pkg_out_dir.join("src"));
    }
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("fmi") {
                let _ = fs::copy(&path, pkg_out_dir.join(path.file_name().unwrap()));
            }
        }
    }
    println!(
        "\x1b[1;32m  Packaged\x1b[0m successfully to target/pkg/{}",
        pkg_name
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
