use crate::blaze;
use crate::utils::parse_manifest_section;
use crate::lexer::TokenKind;
use crate::package_manager;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::diagnostics::Diagnostic;
use crate::lexer::Lexer;
use crate::parser::{Parser, Stmt};
use crate::typechecker::TypeChecker;
use super::build::{build_project, parse_file_stmts};

pub fn collect_fm_files(dir: &Path, list: &mut Vec<PathBuf>, is_root: bool) {
    if !is_root && dir.join("flame.toml").exists() {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_fm_files(&p, list, false);
            } else if p.extension().and_then(|s| s.to_str()) == Some("fm") {
                list.push(p);
            }
        }
    }
}

pub fn has_test_annotations(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| {
        let check_stmt = if let Stmt::ExportDecl(inner, _) = stmt {
            inner.as_ref()
        } else {
            stmt
        };

        if let Stmt::FuncDecl { annotations, .. } = check_stmt {
            annotations.iter().any(|anno| {
                matches!(
                    anno.name.as_str(),
                    "Test"
                        | "Benchmark"
                        | "Parameterized"
                        | "ExpectPanic"
                        | "Ignore"
                        | "Only"
                        | "BeforeAll"
                        | "AfterAll"
                        | "Setup"
                        | "Cleanup"
                )
            })
        } else {
            false
        }
    })
}

pub fn run_tests(args: &[String]) {
    println!("\x1b[1;36mFlame Test & Benchmark Engine\x1b[0m");

    let manifest_content = fs::read_to_string("flame.toml").unwrap_or_default();
    let mut pkg_name = "app".to_string();
    for line in manifest_content.lines() {
        if line.starts_with("name =") {
            if let Some(val) = line.split('=').nth(1) {
                pkg_name = val.trim().trim_matches('"').trim_matches('\'').to_string();
            }
        }
    }

    let mut native_deps_raw = parse_manifest_section(&manifest_content, "[native-dependencies]");
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

    let mut files_to_test = Vec::new();
    if args.len() >= 3 && !args[2].starts_with('-') {
        let p = PathBuf::from(&args[2]);
        if p.exists() {
            if p.is_dir() {
                collect_fm_files(&p, &mut files_to_test, true);
            } else {
                files_to_test.push(p);
            }
        } else {
            println!(
                "\x1b[1;31merror:\x1b[0m test target '{}' does not exist.",
                args[2]
            );
            return;
        }
    } else {
        if Path::new("tests").exists() {
            collect_fm_files(Path::new("tests"), &mut files_to_test, false);
        }
        if Path::new("examples/tests").exists() {
            collect_fm_files(Path::new("examples/tests"), &mut files_to_test, false);
        }
        if Path::new("examples").exists() && !Path::new("examples/tests").exists() {
            collect_fm_files(Path::new("examples"), &mut files_to_test, false);
        }
        if Path::new("src").exists() {
            collect_fm_files(Path::new("src"), &mut files_to_test, false);
        }
        if files_to_test.is_empty() && Path::new("main.fm").exists() {
            files_to_test.push(PathBuf::from("main.fm"));
        }
    }

    // Deduplicate
    let mut seen = std::collections::HashSet::new();
    files_to_test.retain(|p| seen.insert(p.clone()));

    if !native_deps_raw.is_empty() || !plugins_raw.is_empty() {
        println!(
            "\x1b[1;36m     Blaze Testing\x1b[0m Native plugins detected. Compiling test suite natively..."
        );
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
            "dev",
            &processed_native_deps,
            false,
            false,
            true, // is_test_mode
            Some(files_to_test.clone()),
            false,
        );

        let exe_name = format!("{}_test{}", pkg_name, std::env::consts::EXE_SUFFIX);
        let target_exe = Path::new("target").join("dev").join(&exe_name);

        if target_exe.exists() {
            println!(
                "\x1b[1;32m    Finished\x1b[0m test executable -> {}",
                target_exe.display()
            );
            let status = std::process::Command::new(&target_exe).status();
            if let Ok(st) = status {
                if !st.success() {
                    std::process::exit(1);
                }
            }
        }
        return;
    }

    if files_to_test.is_empty() {
        println!("No `.fm` test files found.");
        return;
    }

    let mut filtered_files = Vec::new();
    for path in files_to_test {
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let stmts = match parse_file_stmts(&path, &content) {
            Ok(stmts) => stmts,
            Err(diag) => {
                println!(
                    "  \x1b[1;31mparse error in {}:{}:{}\x1b[0m: {}",
                    path.display(),
                    diag.span.line,
                    diag.span.col,
                    diag.message
                );
                continue;
            }
        };
        if has_test_annotations(&stmts) {
            filtered_files.push(path);
        }
    }
    files_to_test = filtered_files;

    if files_to_test.is_empty() {
        println!("No annotated test files found.");
        return;
    }

    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut total_ignored = 0;
    let mut total_measured = 0;
    let mut total_filtered = 0;
    let total_start = std::time::Instant::now();

    for path in &files_to_test {
        println!("\nrunning tests in \x1b[1m{}\x1b[0m:", path.display());
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                println!("  \x1b[1;31mfatal:\x1b[0m failed to read file: {}", e);
                continue;
            }
        };

        let mut lexer = Lexer::new(&content);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == TokenKind::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = Parser::new(tokens, path.to_string_lossy().to_string());
        let stmts = match parser.parse() {
            Ok(s) => s,
            Err(e) => {
                println!("  \x1b[1;31mparse error:\x1b[0m {}", e.message);
                total_failed += 1;
                continue;
            }
        };

        if !has_test_annotations(&stmts) {
            continue;
        }

        let mut runner = crate::runner::Runner::new(path.clone());
        if let Ok(content) = fs::read_to_string("flame.toml") {
            runner.granted_permissions =
                crate::package_manager::parse_manifest_permissions(&content);
        }
        runner.interactive = false;
        runner.test_mode = true;
        if let Err(e) = runner.run(&stmts) {
            println!("  \x1b[1;31m[FAIL]\x1b[0m \x1b[1;36mGlobal\x1b[0m setup failed");
            let span = runner.current_span.clone().unwrap_or(crate::lexer::Span {
                start: 0,
                end: 0,
                line: 1,
                col: 1,
            });
            crate::diagnostics::Diagnostic::new_error(
                e,
                runner.filepath.display().to_string(),
                span,
                None,
                None,
            )
            .print(&std::fs::read_to_string(&runner.filepath).unwrap_or_default());
        }

        let stats = crate::test_engine::execute_test_suite(
            &mut runner,
            &stmts,
            &path.display().to_string(),
        );
        total_passed += stats.passed;
        total_failed += stats.failed;
        total_ignored += stats.ignored;
        total_measured += stats.measured;
        total_filtered += stats.filtered;
    }

    let total_elapsed = total_start.elapsed().as_secs_f64() * 1000.0;
    let result_str = if total_failed == 0 {
        "\x1b[1;32mok.\x1b[0m"
    } else {
        "\x1b[1;31mFAILED.\x1b[0m"
    };
    println!(
        "\n\x1b[1;32mtest result:\x1b[0m {} {} passed; {} failed; {} ignored; {} measured; {} filtered out; finished in {:.2}ms",
        result_str,
        total_passed,
        total_failed,
        total_ignored,
        total_measured,
        total_filtered,
        total_elapsed
    );
}

pub fn has_benchmark_annotations(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| {
        let check_stmt = if let Stmt::ExportDecl(inner, _) = stmt {
            inner.as_ref()
        } else {
            stmt
        };

        if let Stmt::FuncDecl { annotations, .. } = check_stmt {
            annotations.iter().any(|anno| {
                matches!(
                    anno.name.as_str(),
                    "Benchmark" | "benchmark"
                )
            })
        } else {
            false
        }
    })
}

pub fn run_benchmarks(args: &[String]) {
    println!("\x1b[1;36mFlame Benchmark Engine\x1b[0m");

    let manifest_content = fs::read_to_string("flame.toml").unwrap_or_default();
    let mut _pkg_name = "app".to_string();
    for line in manifest_content.lines() {
        if line.starts_with("name =") {
            if let Some(val) = line.split('=').nth(1) {
                _pkg_name = val.trim().trim_matches('"').trim_matches('\'').to_string();
            }
        }
    }

    let mut candidate_files = Vec::new();

    // Check if user passed a specific file or directory
    let filter_arg = args.iter().skip(2).find(|a| !a.starts_with('-'));
    if let Some(target) = filter_arg {
        let p = Path::new(target);
        if p.is_file() {
            candidate_files.push(p.to_path_buf());
        } else if p.is_dir() {
            collect_fm_files(p, &mut candidate_files, true);
        }
    }

    if candidate_files.is_empty() {
        if Path::new("tests").exists() {
            collect_fm_files(Path::new("tests"), &mut candidate_files, true);
        }
        if Path::new("benchmarks").exists() {
            collect_fm_files(Path::new("benchmarks"), &mut candidate_files, true);
        }
        if Path::new("src").exists() {
            collect_fm_files(Path::new("src"), &mut candidate_files, true);
        }
    }

    let mut files_to_bench = Vec::new();
    for path in candidate_files {
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let stmts = match parse_file_stmts(&path, &content) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if has_benchmark_annotations(&stmts) {
            files_to_bench.push(path);
        }
    }

    if files_to_bench.is_empty() {
        println!("No files with @Benchmark annotations found.");
        return;
    }

    let mut total_measured = 0;
    let mut total_failed = 0;
    let total_start = std::time::Instant::now();

    for path in &files_to_bench {
        println!("\nrunning benchmarks in \x1b[1m{}\x1b[0m:", path.display());
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                println!("  \x1b[1;31mfatal:\x1b[0m failed to read file: {}", e);
                continue;
            }
        };

        let mut lexer = Lexer::new(&content);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == TokenKind::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = Parser::new(tokens, path.to_string_lossy().to_string());
        let stmts = match parser.parse() {
            Ok(s) => s,
            Err(e) => {
                println!("  \x1b[1;31mparse error:\x1b[0m {}", e.message);
                total_failed += 1;
                continue;
            }
        };

        let mut runner = crate::runner::Runner::new(path.clone());
        if let Ok(content) = fs::read_to_string("flame.toml") {
            runner.granted_permissions =
                crate::package_manager::parse_manifest_permissions(&content);
        }
        runner.interactive = false;
        runner.test_mode = true;
        let _ = runner.run(&stmts);

        let stats = crate::test_engine::execute_test_suite(
            &mut runner,
            &stmts,
            &path.display().to_string(),
        );
        total_measured += stats.measured;
        total_failed += stats.failed;
    }

    let total_elapsed = total_start.elapsed().as_secs_f64() * 1000.0;
    let result_str = if total_failed == 0 {
        "\x1b[1;32mok.\x1b[0m"
    } else {
        "\x1b[1;31mFAILED.\x1b[0m"
    };
    println!(
        "\n\x1b[1;32mbenchmark result:\x1b[0m {} {} measured; {} failed; finished in {:.2}ms",
        result_str,
        total_measured,
        total_failed,
        total_elapsed
    );
}


