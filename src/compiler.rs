use crate::package_manager;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn build_project(
    pkg_name: &str,
    profile: &str,
    native_deps: &[(String, String)],
    force_local: bool,
    is_pkg: bool,
    is_test_mode: bool,
    files_to_test: Option<Vec<PathBuf>>,
    use_vfs: bool,
) {
    let build_cache = Path::new(".flame").join("build-cache");
    let _ = fs::create_dir_all(&build_cache);
    let src_dir = build_cache.join("src");
    let _ = fs::create_dir_all(&src_dir);
    if !is_pkg {
        let _ = fs::write(src_dir.join("main.rs"), "fn main() {}");
    } else {
        let _ = fs::write(src_dir.join("lib.rs"), "");
    }

    let current_exe = std::env::current_exe().unwrap();
    let mut is_local_dev = false;
    let mut flame_source_dir = std::path::PathBuf::new();

    if let (Some(parent1), Some(parent2), Some(parent3)) = (
        current_exe.parent(),
        current_exe.parent().and_then(|p| p.parent()),
        current_exe
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent()),
    ) {
        if (parent1.ends_with("debug") || parent1.ends_with("release"))
            && parent2.ends_with("target")
            && parent3.join("Cargo.toml").exists()
        {
            let cargo_content = fs::read_to_string(parent3.join("Cargo.toml")).unwrap_or_default();
            if cargo_content.contains("name = \"flamelang\"")
                || cargo_content.contains("name = \"flame\"")
            {
                is_local_dev = true;
                flame_source_dir = parent3.to_path_buf();
            }
        }
    }

    if let Ok(dev_path) = std::env::var("FLAME_DEV_PATH") {
        is_local_dev = true;
        flame_source_dir = std::path::PathBuf::from(dev_path);
    }

    if force_local {
        is_local_dev = true;
        flame_source_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    }

    // Feature extraction based on user imports and @Application
    let mut features = std::collections::HashSet::new();
    let src_scan_dir = std::env::current_dir().unwrap().join("src");

    fn scan_imports(dir: &Path, features: &mut std::collections::HashSet<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    scan_imports(&path, features);
                } else if path.extension().and_then(|s| s.to_str()) == Some("fm") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        // Map imports to features for BlazeVm compilation
                        let import_re = regex::Regex::new(r"import\s+([a-zA-Z0-9_\.]+)").unwrap();
                        for cap in import_re.captures_iter(&content) {
                            let mod_name = &cap[1];
                            let search = if mod_name.starts_with("std.") {
                                mod_name.to_string()
                            } else {
                                format!("std.{}", mod_name)
                            };

                            // Map standard libraries to their Cargo features
                            let module_features = match search.as_str() {
                                "std.time" => vec!["utils"],
                                "std.os" => vec!["os"],
                                "std.regex" => vec!["regex"],
                                "std.json" => vec!["utils"],
                                "std.desktop" => vec!["os"],
                                "std.window" => vec!["automation"],
                                "std.camera" => vec!["camera"],
                                "std.base64" => vec!["base64"],
                                "std.net.tcp" | "std.net.udp" | "std.net.dns" | "std.net.url"
                                | "std.net.interface" | "std.net" => vec!["net"],
                                "std.net.http" => vec!["net", "http"],
                                "std.net.ws" => vec!["net", "ws"],
                                "std.net.mqtt" => vec!["net", "mqtt"],
                                _ => vec![],
                            };

                            for feature in module_features {
                                features.insert(format!("\"{}\"", feature));
                            }
                        }
                    }
                }
            }
        }
    }

    scan_imports(&src_scan_dir, &mut features);
    let tests_scan_dir = std::env::current_dir().unwrap().join("tests");
    if tests_scan_dir.exists() {
        scan_imports(&tests_scan_dir, &mut features);
    }
    let examples_tests_scan_dir = std::env::current_dir()
        .unwrap()
        .join("examples")
        .join("tests");
    if examples_tests_scan_dir.exists() {
        scan_imports(&examples_tests_scan_dir, &mut features);
    }

    // Also properly parse main.fm to extract @Application features
    let main_fm = std::env::current_dir().unwrap().join("src").join("main.fm");
    if let Ok(content) = fs::read_to_string(&main_fm) {
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
        let mut parser = crate::parser::Parser::new(tokens, "src/main.fm".to_string());
        if let Ok(stmts) = parser.parse() {
            for stmt in stmts {
                if let crate::parser::Stmt::FuncDecl { annotations, .. } = stmt {
                    for ann in annotations {
                        if ann.name == "Application" {
                            // Extract features from @Application(features=["http", "tcp"])
                            // The arguments to annotations are strings in the AST right now?
                            // Wait, parser.rs says `pub args: Vec<String>` for Annotation!
                            // The args are joined strings from the AST tokens.
                            // Let's just do a simple string match on the raw args string since we know it's something like `features=["http", "tcp"]`
                            for arg in &ann.args {
                                if arg.contains("features") && arg.contains("[") {
                                    if let Some(start) = arg.find('[') {
                                        if let Some(end) = arg.find(']') {
                                            let feats = &arg[start + 1..end];
                                            for f in feats.split(',') {
                                                let clean =
                                                    f.trim().trim_matches('"').trim_matches('\'');
                                                if !clean.is_empty() {
                                                    let cargo_feature = match clean {
                                                        "http" => "http",
                                                        "ws" => "ws",
                                                        "mqtt" => "mqtt",
                                                        "tcp" | "udp" | "dns" | "url" | "net" => {
                                                            "net"
                                                        }
                                                        _ => clean,
                                                    };
                                                    features
                                                        .insert(format!("\"{}\"", cargo_feature));
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
        }
    }

    let mut feature_list = features.into_iter().collect::<Vec<_>>().join(", ");
    if is_test_mode && !feature_list.contains("\"base64\"") {
        if !feature_list.is_empty() {
            feature_list.push_str(", ");
        }
        feature_list.push_str("\"base64\"");
    }
    if !feature_list.is_empty() {
        feature_list = format!(", features = [{}]", feature_list);
    }

    let flame_dep = if is_local_dev {
        format!(
            r#"flamelang = {{ path = "{}", default-features = false{} }}"#,
            flame_source_dir.to_string_lossy().replace("\\", "/"),
            feature_list
        )
    } else {
        format!(
            r#"flamelang = {{ version = "{}", default-features = false{} }}"#,
            env!("CARGO_PKG_VERSION"),
            feature_list
        )
    };

    let mut deps_str = String::new();
    for (name, version) in native_deps {
        if version.starts_with('{') {
            deps_str.push_str(&format!("{} = {}\n", name, version));
        } else {
            deps_str.push_str(&format!("{} = \"{}\"\n", name, version));
        }
    }

    let lib_section = if is_pkg {
        format!(
            "\n[lib]\nname = \"{}\"\ncrate-type = [\"rlib\"]\n",
            pkg_name
        )
    } else {
        String::new()
    };

    let cargo_toml = format!(
        r#"[package]
name = "{pkg_name}"
version = "0.1.0"
edition = "2021"{lib_section}

[dependencies]
tokio = {{ version = "1", features = ["rt-multi-thread", "macros", "time", "net", "sync"] }}
{flame_dep}
{deps}

[profile.dev]
split-debuginfo = "unpacked"
codegen-units = 256

[profile.release]
opt-level = 3
strip = true
lto = "fat"
codegen-units = 1
panic = "abort"
"#,
        pkg_name = pkg_name,
        lib_section = lib_section,
        flame_dep = flame_dep,
        deps = deps_str
    );

    fs::write(build_cache.join("Cargo.toml"), cargo_toml).unwrap();

    let mut main_rs = String::new();
    main_rs.push_str("#![allow(unused_variables, dead_code, unused_imports, non_snake_case)]\n");
    main_rs.push_str("use flamelang::runner::{Runner, CValue};\n");
    main_rs.push_str("use std::path::PathBuf;\n\n");
    main_rs.push_str("use flamelang::vm;\n");

    // Generate docs for dependencies to extract metadata
    for (name, version) in native_deps {
        let mut package_spec = name.clone();
        if !version.starts_with('{') && version != "*" {
            package_spec = format!("{}@{}", name, version);
        } else if version.starts_with('{') {
            if let Some(idx) = version.find("version = \"") {
                let rest = &version[idx + 11..];
                if let Some(end_idx) = rest.find("\"") {
                    let v = &rest[..end_idx];
                    if v != "*" {
                        package_spec = format!("{}@{}", name, v);
                    }
                }
            } else if let Some(idx) = version.find("version=\"") {
                let rest = &version[idx + 9..];
                if let Some(end_idx) = rest.find("\"") {
                    let v = &rest[..end_idx];
                    if v != "*" {
                        package_spec = format!("{}@{}", name, v);
                    }
                }
            }
        }
        let json_path = build_cache
            .join("target/doc")
            .join(format!("{}.json", name.replace("-", "_")));
        let meta_dir = Path::new(".flame").join("pkg").join(name);
        fs::create_dir_all(&meta_dir).unwrap();
        let meta_path = meta_dir.join(format!("{}.fmi", name));

        if meta_path.exists() {
            // Already generated FMI, skip rustdoc to drastically decrease build time
            continue;
        }

        let mut retry = true;

        while retry {
            retry = false;
            use std::sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
            };
            let done = Arc::new(AtomicBool::new(false));
            let done_clone = done.clone();
            use std::io::Write;
            print!(
                "\x1b[1;36m   Extracting\x1b[0m bindings for {}...  ",
                package_spec
            );
            let _ = std::io::stdout().flush();
            let spinner_thread = std::thread::spawn(move || {
                let chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let colors = ["\x1b[37m", "\x1b[33m", "\x1b[38;2;255;0;104m"]; // white, yellow, flame
                let mut i = 0;
                let mut color_idx = 0;
                use std::io::Write;
                while !done_clone.load(Ordering::SeqCst) {
                    if i > 0 && i % chars.len() == 0 {
                        color_idx = (color_idx + 1) % colors.len();
                    }
                    let color = colors[color_idx];
                    print!("\x08{}{}\x1b[0m", color, chars[i % chars.len()]);
                    let _ = std::io::stdout().flush();
                    i += 1;
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                print!("\x08 \n");
                let _ = std::io::stdout().flush();
            });

            let output = Command::new("cargo")
                .args([
                    "+nightly",
                    "rustdoc",
                    "-p",
                    &package_spec,
                    "--",
                    "--output-format",
                    "json",
                    "-Zunstable-options",
                ])
                .current_dir(&build_cache)
                .output();

            done.store(true, Ordering::SeqCst);
            let _ = spinner_thread.join();

            if let Ok(out) = output {
                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    println!(
                        "WARNING: rustdoc failed for {}. Stderr: {}",
                        package_spec, stderr
                    );
                    if stderr.contains("is ambiguous")
                        && stderr.contains("following specifications")
                    {
                        // Extract the first specification
                        if let Some(spec_idx) = stderr.find("following specifications") {
                            let rest = &stderr[spec_idx..];
                            if let Some(spec_line) = rest.lines().nth(1) {
                                let spec = spec_line.trim();
                                println!("WARNING: Found ambiguous spec suggestion: '{}'", spec);
                                if !spec.is_empty() && spec != package_spec {
                                    package_spec = spec.to_string();
                                    retry = true;
                                    println!("WARNING: Retrying rustdoc for {}", package_spec);
                                }
                            }
                        }
                    }
                }
            }
        }

        if json_path.exists() {
            let mut meta = crate::package_manager::parse_rustdoc_json(&json_path, name);
            if let Some(local_plugin_path) = extract_local_dependency_path(version) {
                crate::package_manager::enrich_with_syn(&mut meta, &local_plugin_path);
            }
            if let Ok(meta_str) = serde_json::to_string_pretty(&meta) {
                fs::write(&meta_path, meta_str).unwrap();
            }
        }
    }

    for (name, _) in native_deps {
        let meta_path = Path::new(".flame")
            .join("pkg")
            .join(name)
            .join(format!("{}.fmi", name));
        if meta_path.exists() {
            if let Ok(meta_content) = fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<package_manager::FlameMeta>(&meta_content)
                {
                    main_rs.push_str(&format!("// Wrapper for crate {}\n", name));
                    main_rs.push_str(&format!("mod bridge_{} {{\n", name));
                    main_rs.push_str("    use super::*;\n");
                    main_rs.push_str("    #[allow(unused_imports, dead_code)]\n");
                    main_rs.push_str(&format!("    use {}::*;\n", name));
                    main_rs.push_str("    type NativeObject = std::ffi::c_void;\n");
                    let mut generated_methods = std::collections::HashSet::new();

                    for func in meta.functions {
                        if !generated_methods.insert(func.name.clone())
                            || should_skip_bridge_function(&func)
                        {
                            continue;
                        }

                        let f_name = if func.flame_name.is_empty() {
                            &func.name
                        } else {
                            &func.flame_name
                        };
                        main_rs.push_str(&format!(
                            "    pub fn {}(_args: *const CValue, _len: usize) -> CValue {{\n",
                            f_name
                        ));
                        // Start generated function body
                        main_rs.push_str("        let c_args = unsafe { std::slice::from_raw_parts(_args, _len) };\n");
                        let mut call_args = Vec::new();
                        for (idx, p) in func.params.iter().enumerate() {
                            let (ext_code, var_name) =
                                generate_param_extraction(p, idx, &func.name);
                            main_rs.push_str(&ext_code);
                            call_args.push(var_name);
                        }

                        let args_str = call_args.join(", ");
                        let requires_prim =
                            func.is_generic && (func.name == "gen" || func.params.is_empty());
                        let is_persistent_async = func.persistent_runtime;
                        let is_async = func.is_async
                            || is_persistent_async
                            || func.return_type.contains("Future")
                            || func.return_type.contains("async");

                        if requires_prim {
                            let generic_idx = func.params.len();
                            main_rs.push_str(&format!("        let generic_type_cstr = unsafe {{ std::ffi::CStr::from_ptr(c_args[{}].string_ptr) }};\n", generic_idx));
                            main_rs.push_str("        let generic_type = generic_type_cstr.to_str().unwrap_or_default();\n");
                            main_rs.push_str("        match generic_type {\n");
                            let primitives = vec!["i64", "f64", "bool"];
                            for prim in primitives {
                                main_rs.push_str(&format!("            \"{}\" => {{\n", prim));
                                main_rs.push_str(&format!(
                                    "                let res = {}::{}::<{}>({});\n",
                                    name, func.name, prim, args_str
                                ));
                                if prim == "bool" {
                                    main_rs
                                        .push_str("                let mut cv = CValue::null();\n");
                                    main_rs.push_str("                cv.tag = flamelang::runner::CValueTag::Bool;\n");
                                    main_rs.push_str("                cv.bool_val = res;\n");
                                    main_rs.push_str("                return cv;\n");
                                } else if prim == "f32" || prim == "f64" {
                                    main_rs
                                        .push_str("                let mut cv = CValue::null();\n");
                                    main_rs.push_str("                cv.tag = flamelang::runner::CValueTag::Float;\n");
                                    main_rs
                                        .push_str("                cv.float_val = res as f64;\n");
                                    main_rs.push_str("                return cv;\n");
                                } else {
                                    main_rs
                                        .push_str("                let mut cv = CValue::null();\n");
                                    main_rs.push_str("                cv.tag = flamelang::runner::CValueTag::Int;\n");
                                    main_rs.push_str("                cv.int_val = res as i64;\n");
                                    main_rs.push_str("                return cv;\n");
                                }
                                main_rs.push_str("            }\n");
                            }
                            main_rs.push_str("            _ => return CValue::null(),\n");
                            main_rs.push_str("        }\n");
                        } else {
                            if is_async {
                                if is_persistent_async {
                                    main_rs.push_str(
                                        "        flamelang::vm::set_event_loop_active(true);\n",
                                    );
                                    main_rs.push_str(&format!("        std::thread::spawn(move || {{ let rt = tokio::runtime::Runtime::new().unwrap(); rt.block_on(async move {{ {}::{}({}).await }}); flamelang::vm::set_event_loop_active(false); }});\n", name, func.name, args_str));
                                    main_rs.push_str("        CValue::null()\n");
                                } else {
                                    main_rs.push_str(&format!("        let res = tokio::runtime::Runtime::new().unwrap().block_on(async move {{ {}::{}({}).await }});\n", name, func.name, args_str));
                                    main_rs.push_str(&generate_return_conversion(
                                        &func.return_type,
                                        "",
                                    ));
                                }
                            } else {
                                main_rs.push_str(&format!(
                                    "        let res = {}::{}({});\n",
                                    name, func.name, args_str
                                ));
                                main_rs
                                    .push_str(&generate_return_conversion(&func.return_type, ""));
                            }
                        }
                        main_rs.push_str("    }\n");
                    }

                    for struct_meta in meta.structs {
                        let s_name = if struct_meta.flame_name.is_empty() {
                            &struct_meta.name
                        } else {
                            &struct_meta.flame_name
                        };
                        let s_rust_name = &struct_meta.name;
                        for func in struct_meta.methods {
                            let f_name = if func.flame_name.is_empty() {
                                &func.name
                            } else {
                                &func.flame_name
                            };
                            let combined_name = format!("{}_{}", s_name, f_name);
                            if !generated_methods.insert(combined_name.clone())
                                || should_skip_bridge_function(&func)
                            {
                                continue;
                            }

                            main_rs.push_str(&format!(
                                "    pub fn {}(_args: *const CValue, _len: usize) -> CValue {{\n",
                                combined_name
                            ));
                            main_rs.push_str("        let c_args = unsafe { std::slice::from_raw_parts(_args, _len) };\n");
                            let is_persistent_async = func.persistent_runtime;
                            let is_async = func.is_async
                                || is_persistent_async
                                || func.return_type.contains("Future")
                                || func.return_type.contains("async");
                            if !func.is_static {
                                main_rs.push_str("        // Self is arg 0, cast from obj_ptr\n");
                                main_rs.push_str("        if c_args.is_empty() {\n");
                                main_rs.push_str(&format!("            eprintln!(\"\\x1b[1;31mRuntime error:\\x1b[0m argument mismatch in rust function '{}': expected at least 1 argument (self), got 0\");\n", func.name));
                                main_rs.push_str("            std::process::exit(1);\n");
                                main_rs.push_str("        }\n");
                                if func.receiver == Some("self".to_string())
                                    || (is_async && is_persistent_async)
                                {
                                    main_rs.push_str(&format!("        let obj = *unsafe {{ Box::from_raw(c_args[0].obj_ptr as *mut {}::{}) }};\n", name, s_rust_name));
                                } else {
                                    main_rs.push_str(&format!("        let obj = unsafe {{ &mut *(c_args[0].obj_ptr as *mut {}::{}) }};\n", name, s_rust_name));
                                }
                            }

                            let mut call_args = Vec::new();
                            for (idx, p) in func.params.iter().enumerate() {
                                let c_idx = if func.is_static { idx } else { idx + 1 };
                                let (ext_code, var_name) =
                                    generate_param_extraction(p, c_idx, &func.name);
                                main_rs.push_str(&ext_code);
                                call_args.push(var_name);
                            }

                            let args_str = call_args.join(", ");
                            let requires_prim =
                                func.is_generic && (func.name == "gen" || func.params.is_empty());
                            if requires_prim {
                                let generic_idx = if func.is_static {
                                    func.params.len()
                                } else {
                                    func.params.len() + 1
                                };
                                main_rs.push_str(&format!("        let generic_type_cstr = unsafe {{ std::ffi::CStr::from_ptr(c_args[{}].string_ptr) }};\n", generic_idx));
                                main_rs.push_str("        let generic_type = generic_type_cstr.to_str().unwrap_or_default();\n");
                                main_rs.push_str("        match generic_type {\n");
                                let primitives = vec!["i64", "f64", "bool"];
                                for prim in primitives {
                                    main_rs.push_str(&format!("            \"{}\" => {{\n", prim));
                                    if func.is_static {
                                        main_rs.push_str(&format!(
                                            "                let res = {}::{}::{}::<{}>({});\n",
                                            name, s_name, func.name, prim, args_str
                                        ));
                                    } else {
                                        main_rs.push_str(&format!(
                                            "                let res = obj.{}::<{}>({});\n",
                                            func.name, prim, args_str
                                        ));
                                    }
                                    if prim == "bool" {
                                        main_rs.push_str(
                                            "                let mut cv = CValue::null();\n",
                                        );
                                        main_rs.push_str("                cv.tag = flamelang::runner::CValueTag::Bool;\n");
                                        main_rs.push_str("                cv.bool_val = res;\n");
                                        main_rs.push_str("                return cv;\n");
                                    } else if prim == "f32" || prim == "f64" {
                                        main_rs.push_str(
                                            "                let mut cv = CValue::null();\n",
                                        );
                                        main_rs.push_str("                cv.tag = flamelang::runner::CValueTag::Float;\n");
                                        main_rs.push_str(
                                            "                cv.float_val = res as f64;\n",
                                        );
                                        main_rs.push_str("                return cv;\n");
                                    } else {
                                        main_rs.push_str(
                                            "                let mut cv = CValue::null();\n",
                                        );
                                        main_rs.push_str("                cv.tag = flamelang::runner::CValueTag::Int;\n");
                                        main_rs
                                            .push_str("                cv.int_val = res as i64;\n");
                                        main_rs.push_str("                return cv;\n");
                                    }
                                    main_rs.push_str("            }\n");
                                }
                                main_rs.push_str("            _ => return CValue::null(),\n");
                                main_rs.push_str("        }\n");
                            } else {
                                let call_expr = if func.is_static {
                                    format!(
                                        "{}::{}::{}({})",
                                        name, s_rust_name, func.name, args_str
                                    )
                                } else {
                                    format!("obj.{}({})", func.name, args_str)
                                };

                                if is_async {
                                    if is_persistent_async {
                                        main_rs.push_str(
                                            "        flamelang::vm::set_event_loop_active(true);\n",
                                        );
                                        main_rs.push_str(&format!("        std::thread::spawn(move || {{ let rt = tokio::runtime::Runtime::new().unwrap(); rt.block_on(async move {{ {}.await }}); flamelang::vm::set_event_loop_active(false); }});\n", call_expr));
                                        main_rs.push_str("        CValue::null()\n");
                                    } else {
                                        main_rs.push_str(&format!("        let res = tokio::runtime::Runtime::new().unwrap().block_on(async move {{ {}.await }});\n", call_expr));
                                        main_rs.push_str(&generate_return_conversion(
                                            &func.return_type,
                                            s_name,
                                        ));
                                    }
                                } else {
                                    main_rs
                                        .push_str(&format!("        let res = {};\n", call_expr));
                                    main_rs.push_str(&generate_return_conversion(
                                        &func.return_type,
                                        s_name,
                                    ));
                                }
                            }
                            main_rs.push_str("    }\n");
                        }
                    }
                    main_rs.push_str("}\n\n");
                }
            }
        }
    }

    main_rs.push_str("fn main() {\n");
    main_rs.push_str("    let handle = std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(move || {\n");

    if use_vfs {
        // Inject VFS only when explicitly requested (e.g. flame build --vfs)
        main_rs.push_str("    let mut vfs = std::collections::HashMap::new();\n");
        fn collect_vfs(dir: &Path, main_rs: &mut String, base_dir: &Path, prefix: &str) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        collect_vfs(&path, main_rs, base_dir, prefix);
                    } else if path.extension().and_then(|s| s.to_str()) == Some("fm")
                        || path.extension().and_then(|s| s.to_str()) == Some("flame")
                        || path.extension().and_then(|s| s.to_str()) == Some("fmi")
                    {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let rel_path = path
                                .strip_prefix(base_dir)
                                .unwrap_or(&path)
                                .to_string_lossy()
                                .replace("\\", "/");
                            let full_rel_path = format!("{}/{}", prefix, rel_path);
                            main_rs.push_str(&format!("    vfs.insert(\"{}\".to_string(), r########\"{}\"########.to_string());\n", full_rel_path, content));
                        }
                    }
                }
            }
        }
        let base_dir = std::env::current_dir().unwrap().join("src");
        collect_vfs(&base_dir, &mut main_rs, &base_dir, "src");

        let pkg_dir = std::env::current_dir().unwrap().join(".flame").join("pkg");
        if pkg_dir.exists() {
            collect_vfs(&pkg_dir, &mut main_rs, &pkg_dir, ".flame/pkg");
        }

        main_rs.push_str("    let mut base_runner = Runner::new(PathBuf::from(\"src/main.fm\"));\n");
        main_rs.push_str("    base_runner.vfs = Some(vfs);\n");
    } else {
        // Normal builds operate directly on the real filesystem without embedding VFS
        main_rs.push_str("    let mut base_runner = Runner::new(PathBuf::from(\"src/main.fm\"));\n");
        main_rs.push_str("    base_runner.vfs = None;\n");
    }

    let mut perms = std::collections::HashSet::new();
    if Path::new("flame.toml").exists() {
        if let Ok(content) = fs::read_to_string("flame.toml") {
            perms = crate::package_manager::parse_manifest_permissions(&content);
        }
    }
    for p in perms {
        main_rs.push_str(&format!(
            "    base_runner.granted_permissions.insert(\"{}\".to_string());\n",
            p
        ));
    }
    if is_test_mode {
        main_rs.push_str("    base_runner.interactive = false;\n");
    }

    for (name, _) in native_deps {
        let meta_path = Path::new(".flame")
            .join("pkg")
            .join(name)
            .join(format!("{}.fmi", name));
        if meta_path.exists() {
            if let Ok(meta_content) = fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<package_manager::FlameMeta>(&meta_content)
                {
                    let mut generated_methods = std::collections::HashSet::new();
                    for func in meta.functions {
                        if !generated_methods.insert(func.name.clone())
                            || should_skip_bridge_function(&func)
                        {
                            continue;
                        }
                        let f_name = if func.flame_name.is_empty() {
                            &func.name
                        } else {
                            &func.flame_name
                        };
                        let sym = format!("flame_{}_{}", name, f_name);
                        main_rs.push_str(&format!("    base_runner.native_methods.insert(\"{sym}\".to_string(), bridge_{name}::{f_name} as fn(*const CValue, usize) -> CValue);\n", sym=sym, name=name, f_name=f_name));
                    }
                    for struct_meta in meta.structs {
                        let s_name = if struct_meta.flame_name.is_empty() {
                            &struct_meta.name
                        } else {
                            &struct_meta.flame_name
                        };
                        let _s_rust_name = &struct_meta.name;
                        for func in struct_meta.methods {
                            let f_name = if func.flame_name.is_empty() {
                                &func.name
                            } else {
                                &func.flame_name
                            };
                            let combined_name = format!("{}_{}", s_name, f_name);
                            if !generated_methods.insert(combined_name.clone())
                                || should_skip_bridge_function(&func)
                            {
                                continue;
                            }
                            let sym = format!("flame_{}_{}_{}", name, s_name, f_name);
                            main_rs.push_str(&format!("    base_runner.native_methods.insert(\"{sym}\".to_string(), bridge_{name}::{s_name}_{f_name} as fn(*const CValue, usize) -> CValue);\n", sym=sym, name=name, s_name=s_name, f_name=f_name));
                            if name.to_lowercase() == s_name.to_lowercase() {
                                let alias_sym = format!("flame_{}_{}", name, f_name);
                                main_rs.push_str(&format!("    base_runner.native_methods.insert(\"{alias_sym}\".to_string(), bridge_{name}::{s_name}_{f_name} as fn(*const CValue, usize) -> CValue);\n", alias_sym=alias_sym, name=name, s_name=s_name, f_name=f_name));
                            }
                        }
                    }
                }
            }
        }
    }

    main_rs.push_str("    flamelang::runner::set_global_native_methods(base_runner.native_methods.clone());\n");
    main_rs.push_str("    flamelang::runner::set_global_granted_permissions(base_runner.granted_permissions.clone());\n");
    main_rs.push_str("    flamelang::runner::set_global_vfs(base_runner.vfs.clone());\n");

    // We don't have execute_source right now, so we need to run file
    main_rs.push_str("    // Since execute_source does not exist, we just run_file from main.rs if we had it, but here we can just parse and run\n");
    main_rs
        .push_str("    // Read the package's source at runtime from current working directory\n");

    if is_test_mode {
        main_rs.push_str("    let mut files_to_test = vec![\n");
        if let Some(files) = files_to_test {
            for f in files {
                main_rs.push_str(&format!(
                    "        PathBuf::from(\"{}\"),\n",
                    f.to_string_lossy().replace("\\", "/")
                ));
            }
        }
        main_rs.push_str("    ];\n");

        main_rs.push_str("    let mut total_passed = 0;\n");
        main_rs.push_str("    let mut total_failed = 0;\n");
        main_rs.push_str("    let mut total_ignored = 0;\n");
        main_rs.push_str("    let mut total_measured = 0;\n");
        main_rs.push_str("    let mut total_filtered = 0;\n");

        main_rs.push_str("    for file_path in files_to_test {\n");
        main_rs.push_str(
            "        println!(\"\\nrunning tests in \\x1b[1m{}\\x1b[0m:\", file_path.display());\n",
        );
        if use_vfs {
            main_rs.push_str("        let src = base_runner.vfs.as_ref().and_then(|vfs| vfs.get(&file_path.to_string_lossy().replace(\"\\\\\", \"/\"))).cloned().unwrap_or_else(|| std::fs::read_to_string(&file_path).unwrap_or_default());\n");
        } else {
            main_rs.push_str("        let src = std::fs::read_to_string(&file_path).unwrap_or_default();\n");
        }
        main_rs.push_str("        let mut lexer = flamelang::lexer::Lexer::new(&src);\n");
        main_rs.push_str("        let mut tokens = Vec::new();\n");
        main_rs.push_str("        loop {\n");
        main_rs.push_str("            let tok = lexer.next_token();\n");
        main_rs
            .push_str("            let is_eof = tok.kind == flamelang::lexer::TokenKind::EOF;\n");
        main_rs.push_str("            tokens.push(tok);\n");
        main_rs.push_str("            if is_eof { break; }\n");
        main_rs.push_str("        }\n");
        main_rs.push_str("        let mut parser = flamelang::parser::Parser::new(tokens, file_path.to_string_lossy().to_string());\n");
        main_rs.push_str("        match parser.parse() {\n");
        main_rs.push_str("            Ok(mut stmts) => {\n");
        main_rs.push_str("                if !stmts.iter().any(|s| flamelang::parser::is_test_statement(s)) { continue; }\n");
        main_rs.push_str("                flamelang::parser::filter_platform_stmts(&mut stmts, Some(std::env::consts::OS));\n");
        main_rs.push_str("                let mut runner = Runner::new(file_path.clone());\n");
        // copy native methods over
        main_rs.push_str(
            "                runner.native_methods = base_runner.native_methods.clone();\n",
        );
        main_rs.push_str("                runner.granted_permissions = base_runner.granted_permissions.clone();\n");
        main_rs.push_str("                runner.vfs = base_runner.vfs.clone();\n");
        main_rs.push_str("                runner.test_mode = true;\n");
        main_rs.push_str("                if let Err(e) = runner.run(&stmts) {\n");
        main_rs.push_str("                    eprintln!(\"\\x1b[1;31mError during test setup:\\x1b[0m {}\", e);\n");
        main_rs.push_str("                }\n");
        main_rs.push_str("                let stats = flamelang::test_engine::execute_test_suite(&mut runner, &stmts, \"Flame Test Suite\");\n");
        main_rs.push_str("                total_passed += stats.passed;\n");
        main_rs.push_str("                total_failed += stats.failed;\n");
        main_rs.push_str("                total_ignored += stats.ignored;\n");
        main_rs.push_str("                total_measured += stats.measured;\n");
        main_rs.push_str("                total_filtered += stats.filtered;\n");
        main_rs.push_str("            }\n");
        main_rs.push_str("            Err(diag) => {\n");
        main_rs.push_str("                eprintln!(\"Parse error: {}\", diag.message);\n");
        main_rs.push_str("            }\n");
        main_rs.push_str("        }\n");
        main_rs.push_str("    }\n");

        main_rs.push_str("    let result_str = if total_failed == 0 { \"\\x1b[1;32mok.\\x1b[0m\" } else { \"\\x1b[1;31mFAILED.\\x1b[0m\" };\n");
        main_rs.push_str("    println!(\"\\n\\x1b[1;32mtest result:\\x1b[0m {} {} passed; {} failed; {} ignored; {} measured; {} filtered out\", result_str, total_passed, total_failed, total_ignored, total_measured, total_filtered);\n");
        main_rs.push_str("    if total_failed > 0 {\n");
        main_rs.push_str("        std::process::exit(1);\n");
        main_rs.push_str("    }\n");
    } else {
        main_rs.push_str("    let mut runner = base_runner;\n");
        main_rs.push_str("    let entry_file = std::env::var(\"FLAME_ENTRY_FILE\").ok().or_else(|| std::env::args().nth(1).filter(|a| a.ends_with(\".fm\"))).unwrap_or_else(|| \"src/main.fm\".to_string());\n");
        if use_vfs {
            main_rs.push_str("    let src = runner.vfs.as_ref().and_then(|vfs| vfs.get(&entry_file)).cloned().unwrap_or_else(|| std::fs::read_to_string(&entry_file).unwrap_or_default());\n");
        } else {
            main_rs.push_str("    let src = std::fs::read_to_string(&entry_file).unwrap_or_default();\n");
        }
        main_rs.push_str("    let mut lexer = flamelang::lexer::Lexer::new(&src);\n");
        main_rs.push_str("    let mut tokens = Vec::new();\n");
        main_rs.push_str("    loop {\n");
        main_rs.push_str("        let tok = lexer.next_token();\n");
        main_rs.push_str("        let is_eof = tok.kind == flamelang::lexer::TokenKind::EOF;\n");
        main_rs.push_str("        tokens.push(tok);\n");
        main_rs.push_str("        if is_eof { break; }\n");
        main_rs.push_str("    }\n");
        main_rs.push_str(
            "    let mut parser = flamelang::parser::Parser::new(tokens, entry_file.clone());\n",
        );
        main_rs.push_str("    match parser.parse() {\n");
        main_rs.push_str("        Ok(mut stmts) => {\n");
        main_rs.push_str("            flamelang::parser::filter_platform_stmts(&mut stmts, Some(std::env::consts::OS));\n");
        main_rs.push_str("            let clean_stmts: Vec<_> = stmts.into_iter().filter(|stmt| !flamelang::parser::is_test_statement(stmt)).collect();\n");
        main_rs.push_str("            let (tc_res, _) = flamelang::typechecker::TypeChecker::new(entry_file.clone()).check_program(&clean_stmts);\n");
        main_rs.push_str("            if let Err(diags) = tc_res {\n");
        main_rs.push_str("                for d in diags {\n");
        main_rs.push_str("                    d.print(&src);\n");
        main_rs.push_str("                }\n");
        main_rs.push_str("                std::process::exit(1);\n");
        main_rs.push_str("            }\n");
        main_rs.push_str("            let result = runner.run(&clean_stmts);\n");
        main_rs.push_str("            vm::wait_for_all_threads();\n");
        main_rs.push_str("            if let Err(e) = result {\n");
        main_rs
            .push_str("                eprintln!(\"\\x1b[1;31mRuntime error:\\x1b[0m {}\", e);\n");
        main_rs.push_str("                std::process::exit(1);\n");
        main_rs.push_str("            }\n");
        main_rs.push_str("        }\n");
        main_rs.push_str("        Err(diag) => {\n");
        main_rs.push_str("            eprintln!(\"Parse error: {}\", diag.message);\n");
        main_rs.push_str("        }\n");
        main_rs.push_str("    }\n");
    }
    main_rs.push_str("    }).unwrap();\n");
    main_rs.push_str("    handle.join().unwrap();\n");
    main_rs.push_str("}\n");

    fs::write(build_cache.join("src/main.rs"), main_rs).unwrap();

    let root_lockfile = Path::new("flame.lock");
    let cache_lockfile = build_cache.join("Cargo.lock");
    if root_lockfile.exists() {
        let _ = fs::copy(root_lockfile, &cache_lockfile);
    }

    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if profile == "release" {
        cmd.arg("--release");
    }
    cmd.current_dir(&build_cache);

    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();
    use std::io::Write;
    print!("\x1b[1;36m     Linking\x1b[0m flame static object files...  ");
    let _ = std::io::stdout().flush();
    let spinner_thread = std::thread::spawn(move || {
        let chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let colors = ["\x1b[37m", "\x1b[33m", "\x1b[38;2;255;0;104m"]; // white, yellow, flame
        let mut i = 0;
        let mut color_idx = 0;
        use std::io::Write;
        while !done_clone.load(Ordering::SeqCst) {
            if i > 0 && i % chars.len() == 0 {
                color_idx = (color_idx + 1) % colors.len();
            }
            let color = colors[color_idx];
            print!("\x08{}{}\x1b[0m", color, chars[i % chars.len()]);
            let _ = std::io::stdout().flush();
            i += 1;
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        print!("\x08 \n");
        let _ = std::io::stdout().flush();
    });

    let output = cmd.output().expect("Failed to execute cargo build");
    done.store(true, Ordering::SeqCst);
    let _ = spinner_thread.join();

    let status = output.status;

    if status.success() {
        if cache_lockfile.exists() {
            let _ = fs::copy(&cache_lockfile, root_lockfile);
        }
    } else {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(status.code().unwrap_or(1));
    }

    let target_dir = Path::new("target").join(profile);
    fs::create_dir_all(&target_dir).unwrap();
    let exe_name = if is_test_mode {
        format!("{}_test{}", pkg_name, std::env::consts::EXE_SUFFIX)
    } else {
        format!("{}{}", pkg_name, std::env::consts::EXE_SUFFIX)
    };
    let target_exe = target_dir.join(&exe_name);

    if status.success() {
        let cache_target_dir = build_cache
            .join("target")
            .join(if profile == "release" {
                "release"
            } else {
                "debug"
            });

        let mut compiled_exe = None;
        let candidates = [
            format!("{}{}", pkg_name, std::env::consts::EXE_SUFFIX),
            format!("{}_aot{}", pkg_name, std::env::consts::EXE_SUFFIX),
            if is_test_mode {
                format!("{}_test{}", pkg_name, std::env::consts::EXE_SUFFIX)
            } else {
                format!("{}{}", pkg_name, std::env::consts::EXE_SUFFIX)
            },
        ];

        for c in &candidates {
            let candidate_path = cache_target_dir.join(c);
            if candidate_path.exists() {
                compiled_exe = Some(candidate_path);
                break;
            }
        }

        let compiled_exe = match compiled_exe {
            Some(p) => p,
            None => {
                eprintln!(
                    "\x1b[1;31m     Error\x1b[0m could not find compiled binary in {}",
                    cache_target_dir.display()
                );
                std::process::exit(1);
            }
        };

        if let Err(e) = fs::copy(&compiled_exe, &target_exe) {
            eprintln!(
                "\x1b[1;31m     Error\x1b[0m failed to copy executable: {}",
                e
            );
            std::process::exit(1);
        }
        println!("\x1b[1;32m     Finished\x1b[0m building executable!");
    } else {
        eprintln!("\x1b[1;31m     Error\x1b[0m failed to build executable.");
        std::process::exit(1);
    }
}

fn extract_local_dependency_path(version: &str) -> Option<PathBuf> {
    for needle in ["path = \"", "path=\""] {
        if let Some(idx) = version.find(needle) {
            let rest = &version[idx + needle.len()..];
            if let Some(end_idx) = rest.find('"') {
                let path = &rest[..end_idx];
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    None
}

fn should_skip_bridge_function(func: &crate::package_manager::FlameFunctionMeta) -> bool {
    func.return_type.contains("Iter")
        || func.return_type.contains("Iterator")
        || func.return_type.contains("NonNil")
        || func.return_type.contains("Timestamp")
        || func.name.contains("_iter")
        || func.name.starts_with("from_")
        || func.name.contains("unchecked")
        || func.name == "now"
        || func.name == "try_parse_ascii"
        || func.params.iter().any(|p| {
            p.type_name.contains("Bytes")
                || p.type_name.contains("Variant")
                || p.type_name.contains("Version")
                || p.type_name.contains("Iter")
                || p.type_name.contains("StandardUniform")
                || p.type_name.contains("ClockSequence")
                || p.type_name.contains("Context")
                || p.type_name.contains("impl ")
                || p.type_name.contains("[u8]")
                || p.type_name == "Uuid"
        })
}

fn generate_param_extraction(
    p: &crate::package_manager::FlameParamMeta,
    c_idx: usize,
    func_name: &str,
) -> (String, String) {
    let p_type = p.type_name.to_lowercase();
    let var_name = format!("arg{}", c_idx);
    let mut code = String::new();
    code.push_str(&format!("        if c_args.len() <= {} {{\n", c_idx));
    code.push_str(&format!(
        "            eprintln!(\"\\x1b[1;31mRuntime error:\\x1b[0m argument mismatch in rust function '{}': expected at least {} arguments, got {{}}\", c_args.len());\n",
        func_name, c_idx + 1
    ));
    code.push_str("            std::process::exit(1);\n");
    code.push_str("        }\n");

    if p.is_callback
        || p_type.contains("callback")
        || p_type.contains("handler")
        || p.name.to_lowercase().contains("handler")
        || p.name.to_lowercase().contains("callback")
        || p.type_name.len() == 1
        || p_type.contains("fn(")
    {
        code.push_str(&format!(
            "        let fn_id{} = c_args[{}].int_val as u64;\n",
            c_idx, c_idx
        ));
        if func_name == "post" || func_name == "put" || func_name == "patch" {
            code.push_str(&format!(
                "        let {} = move |body: String| async move {{\n",
                var_name
            ));
            code.push_str(&format!("            let cb = flamelang::vm::FlameCallback {{ function_id: fn_id{}, module_id: 0 }};\n", c_idx));
            code.push_str(
                "            let arg_cv = flamelang::runner::CValue::from_string(&body);\n",
            );
            code.push_str("            let res = flamelang::vm::enqueue_callback(cb, vec![arg_cv]).unwrap_or_else(|_| flamelang::vm::CValue::null());\n");
            code.push_str(
                "            let res_val = flamelang::vm::Value::unpack(res, \"\", \"\");\n",
            );
            code.push_str("            match res_val {\n");
            code.push_str("                flamelang::vm::Value::String(s) => s,\n");
            code.push_str("                flamelang::vm::Value::Formula(_) | flamelang::vm::Value::Object(_) | flamelang::vm::Value::StructInstance { .. } | flamelang::vm::Value::Tuple(_) => {\n");
            code.push_str("                    flamelang::native_std::json::value_to_json(&res_val).to_string()\n");
            code.push_str("                }\n");
            code.push_str("                _ => res_val.to_string(),\n");
            code.push_str("            }\n");
            code.push_str("        };\n");
        } else {
            code.push_str(&format!(
                "        let {} = move || async move {{\n",
                var_name
            ));
            code.push_str(&format!("            let cb = flamelang::vm::FlameCallback {{ function_id: fn_id{}, module_id: 0 }};\n", c_idx));
            code.push_str("            let res = flamelang::vm::enqueue_callback(cb, vec![]).unwrap_or_else(|_| flamelang::vm::CValue::null());\n");
            code.push_str(
                "            let res_val = flamelang::vm::Value::unpack(res, \"\", \"\");\n",
            );
            code.push_str("            match res_val {\n");
            code.push_str("                flamelang::vm::Value::String(s) => s,\n");
            code.push_str("                flamelang::vm::Value::Formula(_) | flamelang::vm::Value::Object(_) | flamelang::vm::Value::StructInstance { .. } | flamelang::vm::Value::Tuple(_) => {\n");
            code.push_str("                    flamelang::native_std::json::value_to_json(&res_val).to_string()\n");
            code.push_str("                }\n");
            code.push_str("                _ => res_val.to_string(),\n");
            code.push_str("            }\n");
            code.push_str("        };\n");
        }
    } else if p_type.contains("range") {
        code.push_str(&format!(
            "        let {} = (c_args[{}].int_val as u32)..(c_args[{}].int_val2 as u32);\n",
            var_name, c_idx, c_idx
        ));
    } else if p_type.contains("&str") || (p_type.contains("str") && !p_type.contains("string")) {
        code.push_str(&format!(
            "        let {}_cstr = unsafe {{ std::ffi::CStr::from_ptr(c_args[{}].string_ptr) }};\n",
            var_name, c_idx
        ));
        if p_type.contains("'static") {
            code.push_str(&format!("        let {}: &'static str = Box::leak({}_cstr.to_string_lossy().into_owned().into_boxed_str());\n", var_name, var_name));
        } else {
            code.push_str(&format!(
                "        let {} = {}_cstr.to_str().unwrap_or_default();\n",
                var_name, var_name
            ));
        }
    } else if p_type == "string" {
        code.push_str(&format!(
            "        let {}_cstr = unsafe {{ std::ffi::CStr::from_ptr(c_args[{}].string_ptr) }};\n",
            var_name, c_idx
        ));
        code.push_str(&format!(
            "        let {} = {}_cstr.to_string_lossy().into_owned();\n",
            var_name, var_name
        ));
    } else if p_type == "pathbuf" {
        code.push_str(&format!(
            "        let {}_cstr = unsafe {{ std::ffi::CStr::from_ptr(c_args[{}].string_ptr) }};\n",
            var_name, c_idx
        ));
        code.push_str(&format!(
            "        let {} = std::path::PathBuf::from({}_cstr.to_string_lossy().into_owned());\n",
            var_name, var_name
        ));
    } else if p_type == "&path" {
        code.push_str(&format!(
            "        let {}_cstr = unsafe {{ std::ffi::CStr::from_ptr(c_args[{}].string_ptr) }};\n",
            var_name, c_idx
        ));
        code.push_str(&format!(
            "        let {}_path = std::path::Path::new({}_cstr.to_str().unwrap_or_default());\n",
            var_name, var_name
        ));
        code.push_str(&format!("        let {} = {}_path;\n", var_name, var_name));
    } else if p_type.contains("&[u8]") || p_type.contains("& [u8]") {
        code.push_str(&format!(
            "        let {}_cstr = unsafe {{ std::ffi::CStr::from_ptr(c_args[{}].string_ptr) }};\n",
            var_name, c_idx
        ));
        code.push_str(&format!(
            "        let {} = {}_cstr.to_bytes();\n",
            var_name, var_name
        ));
    } else if p_type == "bool" {
        code.push_str(&format!(
            "        let {} = c_args[{}].bool_val;\n",
            var_name, c_idx
        ));
    } else if p_type == "char" {
        code.push_str(&format!(
            "        let {} = std::char::from_u32(c_args[{}].int_val as u32).unwrap_or(' ');\n",
            var_name, c_idx
        ));
    } else if [
        "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128", "usize", "isize",
    ]
    .contains(&p_type.as_str())
    {
        code.push_str(&format!(
            "        let {} = c_args[{}].int_val as {};\n",
            var_name, c_idx, p.type_name
        ));
    } else if ["f32", "f64"].contains(&p_type.as_str()) {
        code.push_str(&format!(
            "        let {} = c_args[{}].float_val as {};\n",
            var_name, c_idx, p.type_name
        ));
    } else if p_type.starts_with("option<") {
        code.push_str(&format!("        let {} = if c_args[{}].tag == flamelang::runner::CValueTag::Null {{ None }} else {{ Some(c_args[{}].int_val) }};\n", var_name, c_idx, c_idx));
    } else if p_type.starts_with("vec<") || p_type == "[u8]" {
        let inner_type = p_type
            .split('<')
            .nth(1)
            .unwrap_or("u8")
            .trim_end_matches('>');
        code.push_str(&format!(
            "        let {} = if c_args[{}].tag == flamelang::runner::CValueTag::Array {{\n",
            var_name, c_idx
        ));
        code.push_str(&format!(
            "            if c_args[{}].obj_ptr.is_null() || c_args[{}].int_val == 0 {{\n",
            c_idx, c_idx
        ));
        code.push_str("                Vec::new()\n");
        code.push_str("            } else {\n");
        code.push_str(&format!("                let cvals = unsafe {{ Vec::from_raw_parts(c_args[{}].obj_ptr as *mut CValue, c_args[{}].int_val as usize, c_args[{}].int_val as usize) }};\n", c_idx, c_idx, c_idx));
        code.push_str("                let mut out = Vec::with_capacity(cvals.len());\n");
        code.push_str("                for cv in cvals.iter() {\n");
        if inner_type == "u8" {
            code.push_str("                    out.push(cv.int_val as u8);\n");
        } else if inner_type == "i64" || inner_type == "int" {
            code.push_str("                    out.push(cv.int_val);\n");
        } else {
            code.push_str("                    // Default fallback\n");
        }
        code.push_str("                }\n");
        code.push_str("                std::mem::forget(cvals);\n"); // prevent dropping the raw parts! Wait, from_raw_parts means it takes ownership! If we drop it, the pointer is freed. But wait, `cvals` is a copy? No, we shouldn't drop the original CValues yet!
        code.push_str("                out\n");
        code.push_str("            }\n");
        code.push_str("        } else {\n");
        code.push_str("            Vec::new()\n");
        code.push_str("        };\n");
    } else {
        if p.type_name.starts_with('&')
            || p_type.contains("object")
            || p.type_name
                .chars()
                .next()
                .map_or(false, |c| c.is_uppercase())
        {
            let clean_type = p
                .type_name
                .trim_start_matches('&')
                .trim_start_matches("mut ")
                .trim();
            code.push_str(&format!(
                "        let {} = unsafe {{ &mut *(c_args[{}].obj_ptr as *mut {}) }};\n",
                var_name, c_idx, clean_type
            ));
        } else {
            code.push_str(&format!(
                "        let {} = c_args[{}].int_val;\n",
                var_name, c_idx
            ));
        }
    }

    (code, var_name)
}

fn generate_return_conversion(return_type: &str, s_name: &str) -> String {
    generate_return_conversion_var(return_type, s_name, "res")
}

fn generate_return_conversion_var(return_type: &str, s_name: &str, var_name: &str) -> String {
    let rt = return_type.to_lowercase();
    let mut code = String::new();
    if rt == "()" {
        code.push_str("        let mut cv = CValue::null();\n        cv\n");
    } else if rt == "string"
        || rt == "&str"
        || rt.contains("uuid")
        || (rt == "self" && s_name == "Uuid")
    {
        code.push_str(&format!(
            "        let c_str = std::ffi::CString::new({}.to_string()).unwrap_or_default();\n",
            var_name
        ));
        code.push_str("        let mut cv = CValue::null();\n");
        code.push_str("        cv.tag = flamelang::runner::CValueTag::String;\n");
        code.push_str("        cv.string_ptr = c_str.into_raw();\n");
        code.push_str("        cv\n");
    } else if [
        "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128", "usize", "isize",
    ]
    .contains(&rt.as_str())
    {
        code.push_str("        let mut cv = CValue::null();\n");
        code.push_str("        cv.tag = flamelang::runner::CValueTag::Int;\n");
        code.push_str(&format!("        cv.int_val = {} as i64;\n", var_name));
        code.push_str("        cv\n");
    } else if rt == "bool" {
        code.push_str("        let mut cv = CValue::null();\n");
        code.push_str("        cv.tag = flamelang::runner::CValueTag::Bool;\n");
        code.push_str(&format!("        cv.bool_val = {};\n", var_name));
        code.push_str("        cv\n");
    } else if ["f32", "f64"].contains(&rt.as_str()) {
        code.push_str("        let mut cv = CValue::null();\n");
        code.push_str("        cv.tag = flamelang::runner::CValueTag::Float;\n");
        code.push_str(&format!("        cv.float_val = {} as f64;\n", var_name));
        code.push_str("        cv\n");
    } else if rt == "char" {
        code.push_str("        let mut cv = CValue::null();\n");
        code.push_str("        cv.tag = flamelang::runner::CValueTag::Int;\n");
        code.push_str(&format!(
            "        cv.int_val = {} as u32 as i64;\n",
            var_name
        ));
        code.push_str("        cv\n");
    } else if rt == "pathbuf" || rt == "&path" {
        code.push_str(&format!("        let c_str = std::ffi::CString::new({}.to_str().unwrap_or_default()).unwrap_or_default();\n", var_name));
        code.push_str("        let mut cv = CValue::null();\n");
        code.push_str("        cv.tag = flamelang::runner::CValueTag::String;\n");
        code.push_str("        cv.string_ptr = c_str.into_raw();\n");
        code.push_str("        cv\n");
    } else if rt.starts_with("option<") {
        let inner = return_type
            .split('<')
            .nth(1)
            .unwrap_or("")
            .trim_end_matches('>');
        let inner_code = generate_return_conversion_var(inner, s_name, "val");
        code.push_str(&format!("        match {} {{\n", var_name));
        code.push_str("            Some(val) => {\n");
        code.push_str(&format!(
            "                let inner_cv = {{\n{}}};\n",
            inner_code
        ));
        code.push_str("                let inner_val = Box::new(inner_cv);\n");
        code.push_str("                let c_str = std::ffi::CString::new(\"Option::Some\").unwrap_or_default();\n");
        code.push_str("                let mut cv = CValue::null();\n");
        code.push_str("                cv.tag = flamelang::runner::CValueTag::EnumVariant;\n");
        code.push_str("                cv.string_ptr = c_str.into_raw();\n");
        code.push_str(
            "                cv.obj_ptr = Box::into_raw(inner_val) as *mut std::ffi::c_void;\n",
        );
        code.push_str("                cv\n");
        code.push_str("            }\n");
        code.push_str("            None => {\n");
        code.push_str("                let c_str = std::ffi::CString::new(\"Option::None\").unwrap_or_default();\n");
        code.push_str("                let mut cv = CValue::null();\n");
        code.push_str("                cv.tag = flamelang::runner::CValueTag::EnumVariant;\n");
        code.push_str("                cv.string_ptr = c_str.into_raw();\n");
        code.push_str("                cv\n");
        code.push_str("            }\n");
        code.push_str("        }\n");
    } else if rt.starts_with("result<")
        || rt.contains("::result::")
        || rt.starts_with("std::io::result")
    {
        let inner = return_type
            .split('<')
            .nth(1)
            .unwrap_or("")
            .split(',')
            .next()
            .unwrap_or("")
            .trim();
        let inner_code = generate_return_conversion_var(inner, s_name, "val");
        code.push_str(&format!("        match {} {{\n", var_name));
        code.push_str("            Ok(val) => {\n");
        code.push_str(&format!(
            "                let inner_cv = {{\n{}}};\n",
            inner_code
        ));
        code.push_str("                let inner_val = Box::new(inner_cv);\n");
        code.push_str("                let c_str = std::ffi::CString::new(\"Result::Ok\").unwrap_or_default();\n");
        code.push_str("                let mut cv = CValue::null();\n");
        code.push_str("                cv.tag = flamelang::runner::CValueTag::EnumVariant;\n");
        code.push_str("                cv.string_ptr = c_str.into_raw();\n");
        code.push_str(
            "                cv.obj_ptr = Box::into_raw(inner_val) as *mut std::ffi::c_void;\n",
        );
        code.push_str("                cv\n");
        code.push_str("            }\n");
        code.push_str("            Err(err) => {\n");
        code.push_str("                let err_str = format!(\"{}\", err);\n");
        code.push_str("                let c_str = std::ffi::CString::new(\"Result::Err\").unwrap_or_default();\n");
        code.push_str("                let err_c_str = std::ffi::CString::new(err_str).unwrap_or_default();\n");
        code.push_str("                let mut inner_cv = CValue::null();\n");
        code.push_str("                inner_cv.tag = flamelang::runner::CValueTag::String;\n");
        code.push_str("                inner_cv.string_ptr = err_c_str.into_raw();\n");
        code.push_str("                let mut cv = CValue::null();\n");
        code.push_str("                cv.tag = flamelang::runner::CValueTag::EnumVariant;\n");
        code.push_str("                cv.string_ptr = c_str.into_raw();\n");
        code.push_str("                cv.obj_ptr = Box::into_raw(Box::new(inner_cv)) as *mut std::ffi::c_void;\n");
        code.push_str("                cv\n");
        code.push_str("            }\n");
        code.push_str("        }\n");
    } else {
        code.push_str(&format!("        let boxed = Box::new({});\n", var_name));
        code.push_str("        let ptr = Box::into_raw(boxed) as *mut std::ffi::c_void;\n");
        code.push_str("        let mut cv = CValue::null();\n");
        code.push_str("        cv.tag = flamelang::runner::CValueTag::NativeObject;\n");
        code.push_str("        cv.obj_ptr = ptr;\n");
        code.push_str("        cv\n");
    }
    code
}
