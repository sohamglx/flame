use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::diagnostics::Diagnostic;
use crate::lexer::Lexer;
use crate::parser::{Parser, Stmt};
use crate::runner::Runner;
use crate::typechecker::TypeChecker;
use super::build::{build_project, check_runtime_needs_rebuild, get_manifest_pkg_name, get_project_mtime_snapshot, parse_file_stmts};
use super::test::collect_fm_files;

pub fn run_file(path_str: &str, force_local: bool, script_args: &[String]) {
    let start_time = std::time::Instant::now();
    let path = Path::new(path_str);
    if !path.exists() {
        println!(
            "\x1b[1;31merror:\x1b[0m source file '{}' not found",
            path_str
        );
        return;
    }

    let pkg_name = get_manifest_pkg_name();
    let profile = "dev";
    let ext = std::env::consts::EXE_SUFFIX;
    let exe_name = format!("{}{}", pkg_name, ext);
    let dev_exe = Path::new("target").join(profile).join(&exe_name);

    let exe_path = if !force_local
        && Path::new("flame.toml").exists()
        && dev_exe.exists()
        && !check_runtime_needs_rebuild(&dev_exe, profile)
    {
        dev_exe
    } else {
        let build_args = if force_local {
            vec!["--local".to_string()]
        } else {
            vec![]
        };
        match build_project(&build_args) {
            Some(p) => p,
            None => return,
        }
    };

    let mut child = Command::new(&exe_path)
        .env("FLAME_ENTRY_FILE", path_str)
        .args(script_args)
        .spawn()
        .expect("Failed to execute generated binary");

    let status = child.wait().expect("Failed to wait on child");
    let elapsed = start_time.elapsed();

    if !status.success() {
        println!(
            "\x1b[1;31mruntime error:\x1b[0m process exited with code {:?}",
            status.code()
        );
    }

    if elapsed.as_secs_f64() < 0.1 {
        println!(
            "\x1b[1;32m    Finished\x1b[0m execution in {:.2}ms",
            elapsed.as_secs_f64() * 1000.0
        );
    } else {
        println!(
            "\x1b[1;32m    Finished\x1b[0m execution in {:.2}s",
            elapsed.as_secs_f64()
        );
    }
}

pub fn run_file_watch(path_str: &str, force_local: bool, script_args: &[String]) {
    println!("\x1b[1;36m    Watching\x1b[0m for changes in src/ and flame.toml (Ctrl+C to exit)...");

    let pkg_name = get_manifest_pkg_name();
    let profile = "dev";
    let ext = std::env::consts::EXE_SUFFIX;
    let exe_name = format!("{}{}", pkg_name, ext);
    let dev_exe = Path::new("target").join(profile).join(&exe_name);

    let mut current_exe = if !force_local
        && Path::new("flame.toml").exists()
        && dev_exe.exists()
        && !check_runtime_needs_rebuild(&dev_exe, profile)
    {
        Some(dev_exe.clone())
    } else {
        let build_args = if force_local {
            vec!["--local".to_string()]
        } else {
            vec![]
        };
        build_project(&build_args)
    };

    let spawn_process = |exe: &Path| -> Option<std::process::Child> {
        Command::new(exe)
            .env("FLAME_ENTRY_FILE", path_str)
            .args(script_args)
            .spawn()
            .ok()
    };

    let mut running_child: Option<std::process::Child> = current_exe.as_ref().and_then(|p| spawn_process(p));
    let mut snapshot = get_project_mtime_snapshot();

    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));

        let new_snapshot = get_project_mtime_snapshot();
        if new_snapshot != snapshot {
            snapshot = new_snapshot;

            // Kill running child if still active
            if let Some(mut child) = running_child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }

            println!("\n\x1b[1;36m    [watch]\x1b[0m Change detected...");

            let needs_rebuild = force_local
                || current_exe.is_none()
                || !dev_exe.exists()
                || check_runtime_needs_rebuild(&dev_exe, profile);

            if needs_rebuild {
                println!("\x1b[1;36m    [watch]\x1b[0m Rebuilding host runner...");
                let build_args = if force_local {
                    vec!["--local".to_string()]
                } else {
                    vec![]
                };
                current_exe = build_project(&build_args);
            } else {
                current_exe = Some(dev_exe.clone());
            }

            if let Some(ref exe) = current_exe {
                running_child = spawn_process(exe);
            }
        }
    }
}

