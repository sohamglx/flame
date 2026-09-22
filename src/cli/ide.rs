use crate::diagnostics::Diagnostic;
use crate::lexer::Lexer;
use crate::parser::{Parser, Stmt};
use crate::typechecker::TypeChecker;
use crate::utils::{clean_table_borders, find_manifest_root, parse_manifest_section};
use crate::{ide, lexer, package_manager};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct JsonDiagnostic {
    severity: String,
    message: String,
    file: String,
    line: usize,
    column: usize,
}

#[derive(Serialize)]
pub struct JsonCompletion {
    pub label: String,
    pub kind: String,
    pub detail: String,
    pub documentation: Option<String>,
    #[serde(rename = "sortText", skip_serializing_if = "Option::is_none")]
    pub sort_text: Option<String>,
}

#[derive(Serialize)]
pub struct JsonHover {
    pub label: String,
    pub documentation: Option<String>,
}

#[derive(Serialize)]
pub struct JsonSignatureHelp {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: u32,
}

#[derive(Serialize)]
pub struct JsonCheckOutput {
    file: String,
    diagnostics: Vec<JsonDiagnostic>,
    std_modules: Vec<String>,
    native_modules: Vec<String>,
    plugins: Vec<package_manager::PluginSpec>,
    completions: Vec<JsonCompletion>,
    hover: Option<JsonHover>,
    signature_help: Option<JsonSignatureHelp>,
    pub tokens: Vec<crate::ide::SemanticToken>,
    pub definition: Option<crate::ide::JsonDefinition>,
}

pub fn run_definition_command(args: &[String]) {
    if args.len() < 3 {
        println!("\x1b[1;31merror:\x1b[0m please specify a Flame file to inspect");
        println!("usage: flame definition <file> [--line N] [--col N] [--json]");
        return;
    }

    let file = &args[2];
    let json_mode = args.iter().any(|arg| arg == "--json");
    let line = args
        .iter()
        .position(|arg| arg == "--line")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);
    let col = args
        .iter()
        .position(|arg| arg == "--col")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1);

    let stdin_content = if args.iter().any(|arg| arg == "--stdin") {
        use std::io::Read;
        let mut buf = String::new();
        let _ = std::io::stdin().read_to_string(&mut buf);
        Some(buf)
    } else {
        None
    };

    let output = analyze_file_for_json(file, Some(line), Some(col), stdin_content);

    if json_mode {
        #[derive(Serialize)]
        struct DefinitionResponse {
            definition: Option<crate::ide::JsonDefinition>,
        }
        let resp = DefinitionResponse {
            definition: output.definition,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&resp).unwrap_or_else(|_| "{}".to_string())
        );
    } else if let Some(ref def) = output.definition {
        println!("{}:{}:{}", def.file, def.line, def.column);
    } else {
        println!("No definition found");
    }
}

fn collect_check_files(path: &Path, files: &mut Vec<std::path::PathBuf>) {
    if path.is_file() {
        if path.extension().map_or(false, |ext| ext == "fm" || ext == "flame") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            let mut entries_vec: Vec<_> = entries.flatten().collect();
            entries_vec.sort_by_key(|e| e.path());
            for entry in entries_vec {
                let p = entry.path();
                if p.is_dir() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') && name != "dist" && name != "target" && name != "node_modules" {
                        collect_check_files(&p, files);
                    }
                } else if p.is_file() && p.extension().map_or(false, |ext| ext == "fm" || ext == "flame") {
                    files.push(p);
                }
            }
        }
    }
}

pub fn run_check_command(args: &[String]) {
    let json_mode = args.iter().any(|arg| arg == "--json");
    let line = args
        .iter()
        .position(|arg| arg == "--line")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|value| value.parse::<usize>().ok());
    let col = args
        .iter()
        .position(|arg| arg == "--col")
        .and_then(|idx| args.get(idx + 1))
        .and_then(|value| value.parse::<usize>().ok());
    let stdin_content = if args.iter().any(|arg| arg == "--stdin") {
        use std::io::Read;
        let mut buf = String::new();
        let _ = std::io::stdin().read_to_string(&mut buf);
        Some(buf)
    } else {
        None
    };

    let mut target_args: Vec<String> = Vec::new();
    let mut skip_next = false;
    for arg in args.iter().skip(2) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--line" || arg == "--col" {
            skip_next = true;
            continue;
        }
        if arg == "--json" || arg == "--stdin" {
            continue;
        }
        target_args.push(arg.clone());
    }

    let mut files_to_check: Vec<std::path::PathBuf> = Vec::new();
    if target_args.is_empty() {
        if Path::new("src").is_dir() {
            collect_check_files(Path::new("src"), &mut files_to_check);
        } else if Path::new("src/main.fm").exists() {
            files_to_check.push(std::path::PathBuf::from("src/main.fm"));
        } else {
            collect_check_files(Path::new("."), &mut files_to_check);
        }

        if files_to_check.is_empty() && stdin_content.is_none() {
            println!("\x1b[1;31merror:\x1b[0m please specify a Flame file or directory to check");
            println!("usage: flame check [<file-or-dir>...] [--json] [--line N --col N]");
            return;
        }
    } else {
        for target in &target_args {
            let p = Path::new(target);
            if p.is_dir() {
                collect_check_files(p, &mut files_to_check);
            } else {
                files_to_check.push(p.to_path_buf());
            }
        }
    }

    // Single file mode with IDE cursor query (line/col/stdin)
    if (line.is_some() || col.is_some() || stdin_content.is_some()) && files_to_check.len() <= 1 {
        let file = files_to_check.first().map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown.fm".to_string());
        let output = std::panic::catch_unwind(|| analyze_file_for_json(&file, line, col, stdin_content))
            .unwrap_or_else(|_| JsonCheckOutput {
                file: file.clone(),
                diagnostics: vec![],
                std_modules: vec![],
                native_modules: vec![],
                plugins: vec![],
                completions: vec![],
                hover: None,
                signature_help: None,
                tokens: vec![],
                definition: None,
            });

        if json_mode {
            println!(
                "{}",
                serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string())
            );
        } else if output.diagnostics.is_empty() {
            println!("\x1b[1;32mcheck:\x1b[0m no diagnostics in {}", file);
        } else {
            let content = std::fs::read_to_string(&file).unwrap_or_default();
            let file_lines: Vec<&str> = content.lines().collect();
            for diagnostic in &output.diagnostics {
                let color = match diagnostic.severity.as_str() {
                    "warning" => "\x1b[1;33m",
                    "info" => "\x1b[1;34m",
                    _ => "\x1b[1;31m",
                };
                println!(
                    "{}{} :\x1b[0m \x1b[1m{}\x1b[0m",
                    color, diagnostic.severity, diagnostic.message
                );
                println!(
                    "  \x1b[1;36m-->\x1b[0m {}:{}:{}",
                    diagnostic.file, diagnostic.line, diagnostic.column
                );

                let line_idx = diagnostic.line.saturating_sub(1);
                if line_idx < file_lines.len() {
                    let line_str = diagnostic.line.to_string();
                    let spacer = " ".repeat(line_str.len());
                    println!(" \x1b[1;36m{} |\x1b[0m", spacer);
                    println!(" \x1b[1;36m{} |\x1b[0m {}", line_str, file_lines[line_idx]);
                    let col = diagnostic.column.saturating_sub(1);
                    let pointer = " ".repeat(col) + "^";
                    println!(" \x1b[1;36m{} |\x1b[0m {}{}\x1b[0m", spacer, color, pointer);
                }
            }
        }
        return;
    }

    // Multi-file or project check mode
    let mut all_outputs: Vec<JsonCheckOutput> = Vec::new();
    let mut total_errors = 0;
    let mut total_warnings = 0;

    for file_path in &files_to_check {
        let file_str = file_path.to_string_lossy().to_string();
        let output = std::panic::catch_unwind(|| analyze_file_for_json(&file_str, None, None, None))
            .unwrap_or_else(|_| JsonCheckOutput {
                file: file_str.clone(),
                diagnostics: vec![],
                std_modules: vec![],
                native_modules: vec![],
                plugins: vec![],
                completions: vec![],
                hover: None,
                signature_help: None,
                tokens: vec![],
                definition: None,
            });

        for d in &output.diagnostics {
            if d.severity == "warning" {
                total_warnings += 1;
            } else if d.severity == "error" {
                total_errors += 1;
            }
        }

        all_outputs.push(output);
    }

    if json_mode {
        if files_to_check.len() == 1 {
            println!(
                "{}",
                serde_json::to_string_pretty(&all_outputs[0]).unwrap_or_else(|_| "{}".to_string())
            );
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&all_outputs).unwrap_or_else(|_| "[]".to_string())
            );
        }
    } else {
        let mut had_any_diags = false;
        for output in &all_outputs {
            if !output.diagnostics.is_empty() {
                had_any_diags = true;
                let content = std::fs::read_to_string(&output.file).unwrap_or_default();
                let file_lines: Vec<&str> = content.lines().collect();

                for diagnostic in &output.diagnostics {
                    let color = match diagnostic.severity.as_str() {
                        "warning" => "\x1b[1;33m",
                        "info" => "\x1b[1;34m",
                        _ => "\x1b[1;31m",
                    };
                    println!(
                        "{}{} :\x1b[0m \x1b[1m{}\x1b[0m",
                        color, diagnostic.severity, diagnostic.message
                    );
                    println!(
                        "  \x1b[1;36m-->\x1b[0m {}:{}:{}",
                        diagnostic.file, diagnostic.line, diagnostic.column
                    );

                    let line_idx = diagnostic.line.saturating_sub(1);
                    if line_idx < file_lines.len() {
                        let line_str = diagnostic.line.to_string();
                        let spacer = " ".repeat(line_str.len());
                        println!(" \x1b[1;36m{} |\x1b[0m", spacer);
                        println!(" \x1b[1;36m{} |\x1b[0m {}", line_str, file_lines[line_idx]);
                        let col = diagnostic.column.saturating_sub(1);
                        let pointer = " ".repeat(col) + "^";
                        println!(" \x1b[1;36m{} |\x1b[0m {}{}\x1b[0m", spacer, color, pointer);
                    }
                }
            }
        }

        if !had_any_diags {
            if files_to_check.len() == 1 {
                println!("\x1b[1;32mcheck:\x1b[0m no diagnostics in {}", files_to_check[0].display());
            } else {
                println!(
                    "\x1b[1;32mcheck:\x1b[0m no diagnostics across {} files",
                    files_to_check.len()
                );
            }
        } else {
            println!(
                "\x1b[1;31mcheck:\x1b[0m {} error(s), {} warning(s) found across {} files",
                total_errors, total_warnings, files_to_check.len()
            );
        }
    }
}

pub fn list_plugins_command(args: &[String]) {
    let plugins = package_manager::list_plugins();
    if args.iter().any(|arg| arg == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&plugins).unwrap_or_else(|_| "[]".to_string())
        );
        return;
    }

    for plugin in plugins {
        println!("{}\t{}", plugin.name, plugin.source);
    }
}

pub fn analyze_file_for_json(
    file: &str,
    line: Option<usize>,
    col: Option<usize>,
    stdin_content: Option<String>,
) -> JsonCheckOutput {
    let content = stdin_content.unwrap_or_else(|| fs::read_to_string(file).unwrap_or_default());
    let manifest_dir = find_manifest_root(Path::new(file)).unwrap_or_else(|| {
        Path::new(file)
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    });
    let manifest_content = fs::read_to_string(manifest_dir.join("flame.toml")).unwrap_or_default();

    let mut diagnostics = Vec::new();
    let mut lexer = Lexer::new(&content);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        let is_eof = tok.kind == lexer::TokenKind::EOF;
        tokens.push(tok);
        if is_eof {
            break;
        }
    }

    let mut parser = Parser::new(tokens, file.to_string());
    let mut tc_opt = None;
    let mut parsed_stmts = Vec::new();
    match parser.parse() {
        Ok(mut stmts) => {
            parsed_stmts = stmts.clone();
            let mut target = None;
            for line in manifest_content.lines() {
                let t = line.trim();
                if t.starts_with("target =") {
                    if let Some(val) = t.split('=').nth(1) {
                        target = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
                    }
                }
            }
            crate::parser::filter_platform_stmts(&mut stmts, target.as_deref());
            let (res, tc) = TypeChecker::new(file.to_string()).check_program(&stmts);
            tc_opt = Some(tc);
            if let Err(diags) = res {
                for d in diags {
                    let sev = match d.severity {
                        crate::diagnostics::DiagnosticSeverity::Error => "error".to_string(),
                        crate::diagnostics::DiagnosticSeverity::Warning => "warning".to_string(),
                        crate::diagnostics::DiagnosticSeverity::Info => "info".to_string(),
                    };
                    diagnostics.push(JsonDiagnostic {
                        severity: sev,
                        message: d.message,
                        file: d.filepath,
                        line: d.span.line,
                        column: d.span.col,
                    });
                }
            }
        }
        Err(diag) => {
            let sev = match diag.severity {
                crate::diagnostics::DiagnosticSeverity::Error => "error".to_string(),
                crate::diagnostics::DiagnosticSeverity::Warning => "warning".to_string(),
                crate::diagnostics::DiagnosticSeverity::Info => "info".to_string(),
            };
            diagnostics.push(JsonDiagnostic {
                severity: sev,
                message: diag.message,
                file: diag.filepath,
                line: diag.span.line,
                column: diag.span.col,
            });
        }
    }

    let std_modules = list_std_modules(&manifest_dir);
    let mut native_modules = parse_manifest_section(&manifest_content, "[native-dependencies]")
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    for (name, _) in parse_manifest_section(&manifest_content, "[plugins]") {
        if !native_modules.contains(&name) {
            native_modules.push(name);
        }
    }
    for (name, _) in parse_manifest_section(&manifest_content, "[dependencies]") {
        if !native_modules.contains(&name) && name != "std" {
            native_modules.push(name);
        }
    }
    let plugins = parse_manifest_section(&manifest_content, "[plugins]")
        .into_iter()
        .map(|(name, source)| package_manager::PluginSpec {
            name,
            version: source
                .rsplit_once('@')
                .map(|(_, version)| version.to_string()),
            is_local: source == "*" || source.starts_with('.') || source.starts_with('/'),
            source,
        })
        .collect::<Vec<_>>();

    let _lines = content.lines().collect::<Vec<_>>();
    let current_line = if let Some(l) = line {
        content.lines().nth(l.saturating_sub(1)).unwrap_or("")
    } else {
        ""
    };
    let cursor_col = col.unwrap_or(current_line.len() + 1);

    let mut completions = Vec::new();
    if current_line.trim_end().ends_with("import") {
        completions.push(JsonCompletion {
            sort_text: None,
            label: "native".to_string(),
            kind: "module".to_string(),
            detail: "native dependencies".to_string(),
            documentation: None,
        });
        completions.push(JsonCompletion {
            sort_text: None,
            label: "std".to_string(),
            kind: "module".to_string(),
            detail: "standard library".to_string(),
            documentation: None,
        });
    } else if current_line.contains("import native.") {
        for module in &native_modules {
            completions.push(JsonCompletion {
                sort_text: None,
                label: module.clone(),
                kind: "plugin".to_string(),
                detail: "native plugin".to_string(),
                documentation: None,
            });
        }
    } else if current_line.contains("import std.") {
        for module in &std_modules {
            completions.push(JsonCompletion {
                sort_text: None,
                label: module.clone(),
                kind: "module".to_string(),
                detail: "standard library".to_string(),
                documentation: None,
            });
        }
    } else if current_line.trim().starts_with("@Requires(")
        && current_line[..cursor_col as usize].matches('"').count() % 2 == 1
    {
        for module in &std_modules {
            completions.push(JsonCompletion {
                sort_text: None,
                label: format!("std.{}", module),
                kind: "module".to_string(),
                detail: "standard library".to_string(),
                documentation: None,
            });
        }
        for module in &native_modules {
            completions.push(JsonCompletion {
                sort_text: None,
                label: module.clone(),
                kind: "plugin".to_string(),
                detail: "native plugin".to_string(),
                documentation: None,
            });
        }
    } else if current_line.contains("@p") || current_line.contains("@plugin") {
        for plugin in &plugins {
            completions.push(JsonCompletion {
                sort_text: None,
                label: "plugin".to_string(),
                kind: "plugin".to_string(),
                detail: plugin.source.clone(),
                documentation: None,
            });
        }
    }

    let word_under_cursor_raw = extract_word_at_cursor(current_line, cursor_col);
    let word_under_cursor = word_under_cursor_raw.trim_start_matches('@').to_string();

    // Scan for variables and structs
    let (mut scanned_vars, mut scanned_structs) = ide::scan_document(&content);

    let mut cursor_byte_idx = 0;
    if let Some(l) = line {
        let mut curr_line = 1;
        let mut curr_col = 1;
        let c_col = col.unwrap_or(1);
        for (i, c) in content.char_indices() {
            if curr_line == l && curr_col == c_col {
                cursor_byte_idx = i;
                break;
            }
            if c == '\n' {
                curr_line += 1;
                curr_col = 1;
            } else {
                curr_col += 1;
            }
        }
    }

    let impl_re = regex::Regex::new(r"impl\s+([a-zA-Z_]\w*)").unwrap();
    let mut current_impl = None;
    for cap in impl_re.captures_iter(&content) {
        if let Some(m) = cap.get(0) {
            if m.start() <= cursor_byte_idx {
                current_impl = Some(cap[1].to_string());
            }
        }
    }
    if let Some(ref impl_name) = current_impl {
        scanned_vars.push(crate::ide::ScannedVar {
            name: "self".to_string(),
            typ: Some(impl_name.clone()),
            doc: None,
        });
    }

    let imported_module_decls = load_imported_module_declarations(&manifest_dir, file);
    let mut all_decls = imported_module_decls.clone();
    all_decls.extend(parsed_stmts.clone());
    for stmt in &all_decls {
        if let Some((name, params, return_type, is_annotation, annotations)) = match stmt {
            crate::parser::Stmt::FuncDecl {
                name,
                params,
                return_type,
                annotations,
                ..
            } => Some((name, params, return_type.as_deref(), false, annotations)),
            crate::parser::Stmt::PackageDecl {
                name, annotations, ..
            } => {
                let mut doc_str = String::new();
                for ann in annotations {
                    if ann.name == "Docs" {
                        if let Some(s) = ann.args.get(0) {
                            doc_str = s.trim_matches('"').to_string();
                        }
                    }
                }
                if !doc_str.is_empty() {
                    if let Some(var) = scanned_vars.iter_mut().find(|v| {
                        v.name == *name
                            && v.typ.as_deref()
                                == Some(&format!("```flame\nimport package {}\n```", name))
                    }) {
                        var.doc = Some(format!(
                            "```flame\nimport package {}\n```\n{}",
                            name, doc_str
                        ));
                    }
                }
                None
            }
            crate::parser::Stmt::AnnotationDecl {
                name,
                params,
                return_type,
                annotations,
                ..
            } => Some((name, params, return_type.as_deref(), true, annotations)),
            crate::parser::Stmt::ExportDecl(inner, _) => match &**inner {
                crate::parser::Stmt::FuncDecl {
                    name,
                    params,
                    return_type,
                    annotations,
                    ..
                } => Some((name, params, return_type.as_deref(), false, annotations)),
                crate::parser::Stmt::AnnotationDecl {
                    name,
                    params,
                    return_type,
                    annotations,
                    ..
                } => Some((name, params, return_type.as_deref(), true, annotations)),
                crate::parser::Stmt::StructDecl {
                    name,
                    fields,
                    annotations,
                    ..
                } => {
                    let mut struct_doc = None;
                    for ann in annotations {
                        if ann.name == "Docs" {
                            if let Some(s) = ann.args.get(0) {
                                struct_doc = Some(s.trim_matches('"').to_string());
                            }
                        }
                    }
                    let mut struct_methods = Vec::new();
                    let mut method_details = Vec::new();
                    for other_stmt in &all_decls {
                        let impl_cand = match other_stmt {
                            crate::parser::Stmt::ImplDecl {
                                target_type,
                                methods,
                                ..
                            } => Some((target_type, methods)),
                            crate::parser::Stmt::ExportDecl(inner, _) => match &**inner {
                                crate::parser::Stmt::ImplDecl {
                                    target_type,
                                    methods,
                                    ..
                                } => Some((target_type, methods)),
                                _ => None,
                            },
                            _ => None,
                        };
                        if let Some((target_type, methods)) = impl_cand {
                            if target_type == name {
                                for m in methods {
                                    if let crate::parser::Stmt::FuncDecl {
                                        name: m_name,
                                        params,
                                        return_type,
                                        annotations,
                                        ..
                                    } = m
                                    {
                                        let mut m_doc = None;
                                        for ann in annotations {
                                            if ann.name == "Docs" {
                                                if let Some(s) = ann.args.get(0) {
                                                    m_doc = Some(s.trim_matches('"').to_string());
                                                }
                                            }
                                        }
                                        let p_str = params
                                            .iter()
                                            .map(|p| format!("{}: {}", p.name, p.type_name))
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        let sig = format!(
                                            "fn {}({}) -> {}",
                                            m_name,
                                            p_str,
                                            return_type.as_deref().unwrap_or("Nil")
                                        );
                                        if !struct_methods.contains(m_name) {
                                            struct_methods.push(m_name.clone());
                                        }
                                        if !method_details.iter().any(
                                            |existing: &ide::ScannedMethod| {
                                                existing.name == *m_name
                                            },
                                        ) {
                                            method_details.push(ide::ScannedMethod {
                                                name: m_name.clone(),
                                                signature: sig,
                                                doc: m_doc,
                                                return_type: return_type.clone(),
                                                params: params
                                                    .iter()
                                                    .map(|p| (p.name.clone(), p.type_name.clone()))
                                                    .collect(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(existing) = scanned_structs.iter_mut().find(|s| s.name == *name) {
                        existing.fields = fields.clone();
                        for m in struct_methods {
                            if !existing.methods.contains(&m) {
                                existing.methods.push(m);
                            }
                        }
                        for md in method_details {
                            if !existing.method_details.iter().any(|e| e.name == md.name) {
                                existing.method_details.push(md);
                            }
                        }
                        if existing.doc.is_none() {
                            existing.doc = struct_doc;
                        }
                    } else {
                        scanned_structs.push(ide::ScannedStruct {
                            name: name.clone(),
                            fields: fields.clone(),
                            methods: struct_methods,
                            method_details,
                            doc: struct_doc,
                        });
                    }
                    None
                }
                _ => None,
            },
            crate::parser::Stmt::StructDecl {
                name,
                fields,
                annotations,
                ..
            } => {
                let mut struct_doc = None;
                for ann in annotations {
                    if ann.name == "Docs" {
                        if let Some(s) = ann.args.get(0) {
                            struct_doc = Some(s.trim_matches('"').to_string());
                        }
                    }
                }
                let mut struct_methods = Vec::new();
                let mut method_details = Vec::new();
                for other_stmt in &all_decls {
                    let impl_cand = match other_stmt {
                        crate::parser::Stmt::ImplDecl {
                            target_type,
                            methods,
                            ..
                        } => Some((target_type, methods)),
                        crate::parser::Stmt::ExportDecl(inner, _) => match &**inner {
                            crate::parser::Stmt::ImplDecl {
                                target_type,
                                methods,
                                ..
                            } => Some((target_type, methods)),
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some((target_type, methods)) = impl_cand {
                        if target_type == name {
                            for m in methods {
                                if let crate::parser::Stmt::FuncDecl {
                                    name: m_name,
                                    params,
                                    return_type,
                                    annotations,
                                    ..
                                } = m
                                {
                                    let mut m_doc = None;
                                    for ann in annotations {
                                        if ann.name == "Docs" {
                                            if let Some(s) = ann.args.get(0) {
                                                m_doc = Some(s.trim_matches('"').to_string());
                                            }
                                        }
                                    }
                                    let p_str = params
                                        .iter()
                                        .map(|p| format!("{}: {}", p.name, p.type_name))
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    let sig = format!(
                                        "fn {}({}) -> {}",
                                        m_name,
                                        p_str,
                                        return_type.as_deref().unwrap_or("Nil")
                                    );
                                    if !struct_methods.contains(m_name) {
                                        struct_methods.push(m_name.clone());
                                    }
                                    if !method_details.iter().any(
                                        |existing: &ide::ScannedMethod| existing.name == *m_name,
                                    ) {
                                        method_details.push(ide::ScannedMethod {
                                            name: m_name.clone(),
                                            signature: sig,
                                            doc: m_doc,
                                            return_type: return_type.clone(),
                                            params: params
                                                .iter()
                                                .map(|p| (p.name.clone(), p.type_name.clone()))
                                                .collect(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(existing) = scanned_structs.iter_mut().find(|s| s.name == *name) {
                    existing.fields = fields.clone();
                    for m in struct_methods {
                        if !existing.methods.contains(&m) {
                            existing.methods.push(m);
                        }
                    }
                    for md in method_details {
                        if !existing.method_details.iter().any(|e| e.name == md.name) {
                            existing.method_details.push(md);
                        }
                    }
                    if existing.doc.is_none() {
                        existing.doc = struct_doc;
                    }
                } else {
                    scanned_structs.push(ide::ScannedStruct {
                        name: name.clone(),
                        fields: fields.clone(),
                        methods: struct_methods,
                        method_details,
                        doc: struct_doc,
                    });
                }
                None
            }
            _ => None,
        } {
            let mut doc_str = None;
            for ann in annotations {
                if ann.name == "Docs" {
                    if let Some(s) = ann.args.get(0) {
                        doc_str = Some(s.trim_matches('"').to_string());
                    }
                }
            }

            let params_str = params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.type_name))
                .collect::<Vec<_>>()
                .join(", ");
            let sig = if is_annotation {
                if let Some(ret) = return_type {
                    format!("annotation @{}({}) -> {}", name, params_str, ret)
                } else {
                    format!("annotation @{}({})", name, params_str)
                }
            } else {
                format!(
                    "fn {}({}) -> {}",
                    name,
                    params_str,
                    return_type.unwrap_or("Nil")
                )
            };

            let is_local_file_decl = parsed_stmts.iter().any(|s| match s {
                crate::parser::Stmt::FuncDecl { name: n, .. } => n == name,
                crate::parser::Stmt::ExportDecl(inner, _) => match &**inner {
                    crate::parser::Stmt::FuncDecl { name: n, .. } => n == name,
                    _ => false,
                },
                _ => false,
            });

            // Only register local user functions or annotations in scanned_vars and bare completions.
            // Functions from imported modules (e.g. std.window, std.desktop) must only be accessed via their module namespace!
            if is_annotation || is_local_file_decl {
                let doc_text = doc_str.clone().unwrap_or(sig.clone());
                if let Some(existing) = scanned_vars.iter_mut().find(|v| v.name == *name) {
                    existing.typ = Some(sig.clone());
                    if doc_str.is_some() {
                        existing.doc = doc_str.clone();
                    }
                } else {
                    scanned_vars.push(ide::ScannedVar {
                        name: name.clone(),
                        typ: Some(sig.clone()),
                        doc: doc_str.clone(),
                    });
                }
                let (actual_label, sort_text) = if is_annotation {
                    (format!("@{}", name), Some("1_".to_string()))
                } else {
                    (name.clone(), Some("1_".to_string()))
                };
                completions.push(JsonCompletion {
                    sort_text,
                    label: actual_label,
                    kind: if is_annotation {
                        "annotation".to_string()
                    } else {
                        "function".to_string()
                    },
                    detail: if is_annotation {
                        "annotation".to_string()
                    } else {
                        "function".to_string()
                    },
                    documentation: Some(doc_text),
                });
            }
        }
    }

    // Enrich scanned_vars with return types from native module function calls
    for mod_name in &native_modules {
        if let Some(meta) = load_meta_from_project(&manifest_dir, mod_name) {
            // Resolve annotation plugins
            if let Some(init_func) = meta.functions.iter().find(|f| f.flame_name == "init") {
                for var in &mut scanned_vars {
                    if var.typ.as_deref() == Some(&format!("annotation_plugin:{}", mod_name)) {
                        var.typ = Some(init_func.return_type.clone());
                    }
                }
            }

            for func in &meta.functions {
                let pattern1 = format!("= {}.{}(", mod_name, func.flame_name);
                let pattern2 = format!("= await {}.{}(", mod_name, func.flame_name);
                for line in content.lines() {
                    if line.contains(&pattern1) || line.contains(&pattern2) {
                        if let Some(eq_idx) = line.find('=') {
                            let left = &line[..eq_idx].trim();
                            if let Some(var_name) = left.split_whitespace().last() {
                                if let Some(var) =
                                    scanned_vars.iter_mut().find(|v| v.name == var_name)
                                {
                                    var.typ = Some(func.return_type.clone());
                                } else {
                                    scanned_vars.push(ide::ScannedVar {
                                        name: var_name.to_string(),
                                        typ: Some(func.return_type.clone()),
                                        doc: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            for struct_meta in &meta.structs {
                for func in &struct_meta.methods {
                    let pattern1 =
                        format!("= {}.{}.{}(", mod_name, struct_meta.name, func.flame_name);
                    let pattern2 = format!(
                        "= await {}.{}.{}(",
                        mod_name, struct_meta.name, func.flame_name
                    );
                    for line in content.lines() {
                        if line.contains(&pattern1) || line.contains(&pattern2) {
                            if let Some(eq_idx) = line.find('=') {
                                let left = &line[..eq_idx].trim();
                                if let Some(var_name) = left.split_whitespace().last() {
                                    if let Some(var) =
                                        scanned_vars.iter_mut().find(|v| v.name == var_name)
                                    {
                                        var.typ = Some(func.return_type.clone());
                                    } else {
                                        scanned_vars.push(ide::ScannedVar {
                                            name: var_name.to_string(),
                                            typ: Some(func.return_type.clone()),
                                            doc: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(tc) = &tc_opt {
        for var in &mut scanned_vars {
            if let Some(vinfo) = tc.lookup_var(&var.name) {
                let inferred_name = match &vinfo.ty {
                    crate::typechecker::Type::Named(n) => Some(n.clone()),
                    crate::typechecker::Type::Struct(n) => Some(n.clone()),
                    crate::typechecker::Type::Enum(n) => Some(n.clone()),
                    _ => None,
                };
                if let Some(name) = inferred_name {
                    if name != "Unknown" {
                        var.typ = Some(name);
                    }
                }
            }
        }
    }

    let (namespace, member_prefix) = extract_member_context(current_line, cursor_col);
    // eprintln!("DEBUG_CONTEXT: namespace={:?}, prefix={:?}, line='{}', col={}", namespace, member_prefix, current_line, cursor_col);

    let mut exact_ast_hover = None;
    if let Some(tc) = &tc_opt {
        if let Some(l) = line {
            let mut cursor_byte_idx = 0;
            let mut curr_line = 1;
            let mut curr_col = 1;
            for (i, c) in content.char_indices() {
                if curr_line == l && curr_col == cursor_col {
                    cursor_byte_idx = i;
                    break;
                }
                if c == '\n' {
                    curr_line += 1;
                    curr_col = 1;
                } else {
                    curr_col += 1;
                }
            }
            if cursor_byte_idx == 0 && curr_line == l && cursor_col >= curr_col {
                cursor_byte_idx = content.len();
            }

            let mut best_span: Option<&crate::lexer::Span> = None;
            let mut best_ty_str = None;

            for (span, ty_str) in &tc.hover_info {
                if cursor_byte_idx >= span.start && cursor_byte_idx <= span.end {
                    if let Some(best) = best_span {
                        if (span.end - span.start) < (best.end - best.start) {
                            best_span = Some(span);
                            best_ty_str = Some(ty_str);
                        }
                    } else {
                        best_span = Some(span);
                        best_ty_str = Some(ty_str);
                    }
                }
            }

            if let Some(ty_str) = best_ty_str {
                if ty_str != "Unknown" {
                    if ty_str.starts_with("```") || ty_str.starts_with('@') || ty_str.contains('\n')
                    {
                        exact_ast_hover = Some(JsonHover {
                            label: word_under_cursor.clone(),
                            documentation: Some(ty_str.clone()),
                        });
                    } else if ty_str.starts_with("fn(") {
                        if let Some(var) = scanned_vars.iter().find(|v| v.name == word_under_cursor)
                        {
                            if let Some(t) = &var.typ {
                                if t.starts_with("fn ") {
                                    let doc = if let Some(d) = &var.doc {
                                        format!("```flame\n{}\n```\n\n{}", t, d)
                                    } else {
                                        format!("```flame\n{}\n```", t)
                                    };
                                    exact_ast_hover = Some(JsonHover {
                                        label: word_under_cursor.clone(),
                                        documentation: Some(doc),
                                    });
                                }
                            }
                        }
                        if exact_ast_hover.is_none() && !word_under_cursor.is_empty() {
                            let sig = format!("fn {}{}", word_under_cursor, &ty_str[2..]);
                            let doc = if let Some(var) =
                                scanned_vars.iter().find(|v| v.name == word_under_cursor)
                            {
                                if let Some(d) = &var.doc {
                                    format!("```flame\n{}\n```\n\n{}", sig, d)
                                } else {
                                    format!("```flame\n{}\n```", sig)
                                }
                            } else {
                                format!("```flame\n{}\n```", sig)
                            };
                            exact_ast_hover = Some(JsonHover {
                                label: word_under_cursor.clone(),
                                documentation: Some(doc),
                            });
                        }
                    } else if !word_under_cursor.is_empty() {
                        let sig = format!("{}: {}", word_under_cursor, ty_str);
                        let struct_doc = scanned_structs
                            .iter()
                            .find(|s| s.name == *ty_str)
                            .and_then(|s| s.doc.clone());
                        let doc = if let Some(var) =
                            scanned_vars.iter().find(|v| v.name == word_under_cursor)
                        {
                            if let Some(d) = &var.doc {
                                format!("```flame\nlet {}\n```\n\n{}", sig, d)
                            } else if let Some(sdoc) = &struct_doc {
                                format!("```flame\nlet {}\n```\n\n{}", sig, sdoc)
                            } else {
                                format!("```flame\nlet {}\n```", sig)
                            }
                        } else if let Some(sdoc) = &struct_doc {
                            format!("```flame\nlet {}\n```\n\n{}", sig, sdoc)
                        } else {
                            format!("```flame\nlet {}\n```", sig)
                        };
                        exact_ast_hover = Some(JsonHover {
                            label: word_under_cursor.clone(),
                            documentation: Some(doc),
                        });
                    }
                }
            }
        }
    }

    let mut scanned_var_hover = None;
    let mut hover_found = None;
    if !word_under_cursor.is_empty() {
        if let Some(var) = scanned_vars.iter().find(|v| v.name == word_under_cursor) {
            if let Some(t) = &var.typ {
                if t != "Unknown" {
                    let (code_block, doc_body) = if t.starts_with("import:") {
                        let imported = &t["import:".len()..];
                        (
                            format!("import {} as {}", imported, word_under_cursor),
                            var.doc
                                .clone()
                                .unwrap_or_else(|| format!("Imported module `{}`", imported)),
                        )
                    } else if t.starts_with("fn ") || t.starts_with("annotation ") {
                        (t.clone(), var.doc.clone().unwrap_or_default())
                    } else if let Some(mod_name) = native_modules.iter().find(|m| {
                        load_meta_from_project(&manifest_dir, m)
                            .map_or(false, |meta| meta.structs.iter().any(|s| s.name == *t))
                    }) {
                        (
                            format!("{}: {}", word_under_cursor, t),
                            format!("Struct type from native module '{}'", mod_name),
                        )
                    } else {
                        let struct_doc = scanned_structs
                            .iter()
                            .find(|s| s.name == *t)
                            .and_then(|s| s.doc.clone());
                        (
                            format!("{}: {}", word_under_cursor, t),
                            var.doc.clone().or(struct_doc).unwrap_or_default(),
                        )
                    };
                    let documentation = if doc_body.is_empty() {
                        format!("```flame\nlet {}\n```", code_block)
                    } else if doc_body.starts_with("```") {
                        doc_body
                    } else if t.starts_with("fn ") || t.starts_with("annotation ") {
                        format!("```flame\n{}\n```\n\n{}", code_block, doc_body)
                    } else {
                        format!("```flame\nlet {}\n```\n\n{}", code_block, doc_body)
                    };
                    scanned_var_hover = Some(JsonHover {
                        label: word_under_cursor.clone(),
                        documentation: Some(documentation),
                    });
                }
            }
        }
    }

    if let Some(namespace) = namespace {
        let mut resolved_as_var = false;

        let mut alias_map: HashMap<String, String> = HashMap::new();
        let import_as_re = Regex::new(
            r"import\s+([a-zA-Z_][\w]*(?:\.[a-zA-Z_][\w]*)*)(?:\s+as\s+([a-zA-Z_]\w*))?",
        )
        .unwrap();
        for cap in import_as_re.captures_iter(&content) {
            let full_path = cap[1].to_string();
            let alias = cap
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| {
                    full_path
                        .rsplit('.')
                        .next()
                        .unwrap_or(&full_path)
                        .to_string()
                });
            alias_map.insert(alias, full_path);
        }

        let target_path = alias_map.get(&namespace).cloned();
        let effective_mod = target_path
            .as_ref()
            .map(|t| t.rsplit('.').next().unwrap_or(t.as_str()).to_string());

        let mut lookup_namespaces = vec![namespace.clone()];
        if let Some(ref em) = effective_mod {
            if !lookup_namespaces.contains(em) {
                lookup_namespaces.push(em.clone());
            }
        }
        if let Some(ref tp) = target_path {
            if !lookup_namespaces.contains(tp) {
                lookup_namespaces.push(tp.clone());
            }
        }

        if let Some(tc) = &tc_opt {
            if let Some(var_info) = tc.lookup_var(&namespace) {
                if let crate::typechecker::Type::Formula(members, docs) = &var_info.ty {
                    for (member_name, _) in members {
                        if member_prefix
                            .as_deref()
                            .map_or(true, |p| member_name.starts_with(p))
                        {
                            let doc = docs.get(member_name).cloned();
                            completions.push(JsonCompletion {
                                sort_text: None,
                                label: member_name.clone(),
                                kind: "function".to_string(),
                                detail: format!("{}.{}", namespace, member_name),
                                documentation: doc,
                            });
                        }
                    }
                    if !word_under_cursor.is_empty() && members.contains_key(&word_under_cursor) {
                        let doc = docs.get(&word_under_cursor).cloned();
                        hover_found = Some(JsonHover {
                            label: format!("{namespace}.{word_under_cursor}()"),
                            documentation: doc.or_else(|| {
                                Some(format!("Function {namespace}.{word_under_cursor}"))
                            }),
                        });
                    }
                    resolved_as_var = true;
                }
            }
        }

        // First check if it's a known variable! If so, it might be a struct or native type (e.g. FlameServer)
        if !resolved_as_var {
            if let Some(var) = scanned_vars.iter().find(|v| v.name == namespace) {
                if let Some(typ) = &var.typ {
                    if typ != "Unknown" && !typ.starts_with("import:") {
                        let mut provided = false;
                        for s in &scanned_structs {
                            if s.name == *typ {
                                for (field_name, field_type) in &s.fields {
                                    if member_prefix
                                        .as_deref()
                                        .map_or(true, |p| field_name.starts_with(p))
                                    {
                                        completions.push(JsonCompletion {
                                            sort_text: Some("0_".to_string()),
                                            label: field_name.clone(),
                                            kind: "property".to_string(),
                                            detail: format!("{}: {}", field_name, field_type),
                                            documentation: Some(format!(
                                                "```flame\n{}.{}: {}\n```\nField of `{}`",
                                                typ, field_name, field_type, typ
                                            )),
                                        });
                                    }
                                }
                                for m in &s.method_details {
                                    if member_prefix
                                        .as_deref()
                                        .map_or(true, |p| m.name.starts_with(p))
                                    {
                                        let doc = if let Some(d) = &m.doc {
                                            Some(format!("```flame\n{}\n```\n\n{}", m.signature, d))
                                        } else {
                                            Some(format!("```flame\n{}\n```", m.signature))
                                        };
                                        completions.push(JsonCompletion {
                                            sort_text: Some("1_".to_string()),
                                            label: m.name.clone(),
                                            kind: "method".to_string(),
                                            detail: m.signature.clone(),
                                            documentation: doc,
                                        });
                                    }
                                }
                                if s.method_details.is_empty() {
                                    for func_name in &s.methods {
                                        if member_prefix
                                            .as_deref()
                                            .map_or(true, |p| func_name.starts_with(p))
                                        {
                                            completions.push(JsonCompletion {
                                                sort_text: Some("1_".to_string()),
                                                label: func_name.clone(),
                                                kind: "method".to_string(),
                                                detail: format!("struct {}", typ),
                                                documentation: None,
                                            });
                                        }
                                    }
                                }
                                provided = true;
                            }
                        }
                        if !provided {
                            if typ.starts_with('[') || typ.starts_with("Vec<") {
                                let array_methods = [
                                    ("len", "Returns the number of elements in the array"),
                                    ("isEmpty", "Returns true if the array contains no elements"),
                                    ("push", "Appends an element to the end of the array"),
                                    ("pop", "Removes and returns the last element of the array"),
                                    (
                                        "map",
                                        "Transforms each element of the array using the given function",
                                    ),
                                    (
                                        "filter",
                                        "Returns a new array with elements that satisfy the predicate",
                                    ),
                                    ("concat", "Concatenates this array with another array"),
                                    ("reverse", "Reverses the order of elements in the array"),
                                    ("slice", "Returns a sub-slice of elements (start, len)"),
                                    ("join", "Joins string elements with the given separator"),
                                    ("get", "Returns the element at the specified index"),
                                ];
                                for (m_name, m_doc) in array_methods {
                                    if member_prefix
                                        .as_deref()
                                        .map_or(true, |p| m_name.starts_with(p))
                                    {
                                        completions.push(JsonCompletion {
                                            sort_text: Some("1_".to_string()),
                                            label: m_name.to_string(),
                                            kind: "method".to_string(),
                                            detail: format!("Array method"),
                                            documentation: Some(m_doc.to_string()),
                                        });
                                    }
                                }
                                provided = true;
                            } else if typ == "String" {
                                let string_methods = [
                                    ("len", "Returns the length of the string in bytes"),
                                    ("isEmpty", "Returns true if the string is empty"),
                                    ("pushStr", "Appends another string slice to this string"),
                                    ("toUpperCase", "Returns an uppercase copy of the string"),
                                    ("toLowerCase", "Returns a lowercase copy of the string"),
                                    (
                                        "trim",
                                        "Returns a copy with leading and trailing whitespace removed",
                                    ),
                                    ("split", "Splits the string by the specified delimiter"),
                                    (
                                        "contains",
                                        "Returns true if the string contains the specified substring",
                                    ),
                                    (
                                        "startsWith",
                                        "Returns true if the string starts with the specified prefix",
                                    ),
                                    (
                                        "endsWith",
                                        "Returns true if the string ends with the specified suffix",
                                    ),
                                    (
                                        "replace",
                                        "Replaces occurrences of pattern with replacement",
                                    ),
                                    ("repeat", "Repeats the string n times"),
                                    ("lines", "Returns an array of lines in the string"),
                                    ("toByte", "Converts the string into a Byte or byte array"),
                                    ("toInt", "Parses the string into an integer"),
                                    (
                                        "tryInt",
                                        "Attempts to parse into an integer, returning nil on failure",
                                    ),
                                    ("toFloat", "Parses the string into a float"),
                                    (
                                        "tryFloat",
                                        "Attempts to parse into a float, returning nil on failure",
                                    ),
                                    ("toBool", "Parses the string into a boolean"),
                                    (
                                        "tryBool",
                                        "Attempts to parse into a boolean, returning nil on failure",
                                    ),
                                ];
                                for (m_name, m_doc) in string_methods {
                                    if member_prefix
                                        .as_deref()
                                        .map_or(true, |p| m_name.starts_with(p))
                                    {
                                        completions.push(JsonCompletion {
                                            sort_text: Some("1_".to_string()),
                                            label: m_name.to_string(),
                                            kind: "method".to_string(),
                                            detail: format!("String method"),
                                            documentation: Some(m_doc.to_string()),
                                        });
                                    }
                                }
                                provided = true;
                            } else if typ == "Bytes" {
                                let bytes_methods = [
                                    ("len", "Returns the total number of bytes"),
                                    ("isEmpty", "Returns true if byte buffer is empty"),
                                    (
                                        "slice",
                                        "Returns a sub-slice of the byte buffer (start, len)",
                                    ),
                                    ("toHex", "Encodes byte buffer into hexadecimal string"),
                                    ("toBase64", "Encodes byte buffer into Base64 string"),
                                    ("toString", "Decodes byte buffer into UTF-8 string"),
                                    ("toUtf8", "Decodes byte buffer into UTF-8 string"),
                                    ("get", "Reads a single byte value at the index"),
                                ];
                                for (m_name, m_doc) in bytes_methods {
                                    if member_prefix
                                        .as_deref()
                                        .map_or(true, |p| m_name.starts_with(p))
                                    {
                                        completions.push(JsonCompletion {
                                            sort_text: Some("1_".to_string()),
                                            label: m_name.to_string(),
                                            kind: "method".to_string(),
                                            detail: format!("Bytes method"),
                                            documentation: Some(m_doc.to_string()),
                                        });
                                    }
                                }
                                provided = true;
                            } else if typ == "Sender" {
                                let sender_methods = [
                                    ("send", "Sends a message value through the channel"),
                                    ("close", "Closes the channel sender"),
                                ];
                                for (m_name, m_doc) in sender_methods {
                                    if member_prefix
                                        .as_deref()
                                        .map_or(true, |p| m_name.starts_with(p))
                                    {
                                        completions.push(JsonCompletion {
                                            sort_text: Some("1_".to_string()),
                                            label: m_name.to_string(),
                                            kind: "method".to_string(),
                                            detail: format!("Sender method"),
                                            documentation: Some(m_doc.to_string()),
                                        });
                                    }
                                }
                                provided = true;
                            } else if typ == "Receiver" {
                                let receiver_methods = [
                                    (
                                        "recv",
                                        "Blocks until a message is received from the channel",
                                    ),
                                    (
                                        "tryRecv",
                                        "Non-blocking attempt to receive a message from the channel",
                                    ),
                                ];
                                for (m_name, m_doc) in receiver_methods {
                                    if member_prefix
                                        .as_deref()
                                        .map_or(true, |p| m_name.starts_with(p))
                                    {
                                        completions.push(JsonCompletion {
                                            sort_text: Some("1_".to_string()),
                                            label: m_name.to_string(),
                                            kind: "method".to_string(),
                                            detail: format!("Receiver method"),
                                            documentation: Some(m_doc.to_string()),
                                        });
                                    }
                                }
                                provided = true;
                            }
                        }
                        if !provided {
                            if let Some(tc) = &tc_opt {
                                if let Some((_, s)) = tc
                                    .structs
                                    .iter()
                                    .find(|(k, _)| *k == typ || k.ends_with(&format!(".{}", typ)))
                                {
                                    for (f_name, f_ty) in &s.fields {
                                        if member_prefix
                                            .as_deref()
                                            .map_or(true, |p| f_name.starts_with(p))
                                        {
                                            completions.push(JsonCompletion {
                                                sort_text: Some("0_".to_string()),
                                                label: f_name.clone(),
                                                kind: "property".to_string(),
                                                detail: format!("{}: {:?}", f_name, f_ty),
                                                documentation: Some(format!(
                                                    "```flame\n{}.{}: {:?}\n```\nField of `{}`",
                                                    typ, f_name, f_ty, typ
                                                )),
                                            });
                                        }
                                    }
                                    if let Some(methods) = tc.methods.get(typ).or_else(|| {
                                        tc.methods
                                            .iter()
                                            .find(|(k, _)| {
                                                *k == typ || k.ends_with(&format!(".{}", typ))
                                            })
                                            .map(|(_, v)| v)
                                    }) {
                                        for (m_name, sig) in methods {
                                            if member_prefix
                                                .as_deref()
                                                .map_or(true, |p| m_name.starts_with(p))
                                            {
                                                let params_str = sig
                                                    .params
                                                    .iter()
                                                    .map(|p| format!("{}: {:?}", p.name, p.ty))
                                                    .collect::<Vec<_>>()
                                                    .join(", ");
                                                let return_str = if sig.return_type
                                                    == crate::typechecker::Type::Nil
                                                {
                                                    "".to_string()
                                                } else {
                                                    format!(" -> {:?}", sig.return_type)
                                                };
                                                let fallback = format!(
                                                    "fn {}({}){}",
                                                    m_name, params_str, return_str
                                                );
                                                completions.push(JsonCompletion {
                                                    sort_text: Some("1_".to_string()),
                                                    label: m_name.clone(),
                                                    kind: "method".to_string(),
                                                    detail: fallback.clone(),
                                                    documentation: sig.hover_doc.clone().or(Some(
                                                        format!("```flame\n{}\n```", fallback),
                                                    )),
                                                });
                                            }
                                        }
                                    }
                                    provided = true;
                                }
                            }
                        }
                        if !provided {
                            for mod_name in &native_modules {
                                if let Some(meta) = load_meta_from_project(&manifest_dir, mod_name)
                                {
                                    if let Some(struct_meta) =
                                        meta.structs.iter().find(|s| s.name == *typ)
                                    {
                                        for func in &struct_meta.methods {
                                            if member_prefix
                                                .as_deref()
                                                .map_or(true, |p| func.flame_name.starts_with(p))
                                            {
                                                completions.push(JsonCompletion {
                                                    sort_text: None,
                                                    label: func.flame_name.clone(),
                                                    kind: "method".to_string(),
                                                    detail: format!("native {}.{}", mod_name, typ),
                                                    documentation: func.docs.clone(),
                                                });
                                            }
                                        }
                                        provided = true;
                                        break;
                                    }
                                }
                            }
                        }
                        if provided {
                            // Append universal methods
                            for (u_name, u_doc) in [
                                ("type", "Returns the type name of the value"),
                                (
                                    "toString",
                                    "Converts the value to its string representation",
                                ),
                                ("toJson", "Serializes the value to a JSON string"),
                            ] {
                                if member_prefix
                                    .as_deref()
                                    .map_or(true, |p| u_name.starts_with(p))
                                {
                                    completions.push(JsonCompletion {
                                        sort_text: Some("3_".to_string()),
                                        label: u_name.to_string(),
                                        kind: "method".to_string(),
                                        detail: format!("universal method"),
                                        documentation: Some(u_doc.to_string()),
                                    });
                                }
                            }
                            resolved_as_var = true;

                            if !word_under_cursor.is_empty() {
                                if let Some(tc) = &tc_opt {
                                    if let Some(methods) = tc.methods.get(typ) {
                                        if let Some((_, sig)) = methods
                                            .iter()
                                            .find(|(name, _)| *name == &word_under_cursor)
                                        {
                                            let params_str = sig
                                                .params
                                                .iter()
                                                .map(|p| format!("{}: {:?}", p.name, p.ty))
                                                .collect::<Vec<_>>()
                                                .join(", ");
                                            let return_str = if sig.return_type
                                                == crate::typechecker::Type::Nil
                                            {
                                                "".to_string()
                                            } else {
                                                format!(" -> {:?}", sig.return_type)
                                            };
                                            let fallback = format!(
                                                "```flame\nfn {}({}){}\n```",
                                                word_under_cursor, params_str, return_str
                                            );
                                            let doc = if let Some(d) = &sig.hover_doc {
                                                format!("{}\n\n{}", fallback, d)
                                            } else {
                                                fallback
                                            };
                                            hover_found = Some(JsonHover {
                                                label: format!("{}::{}()", typ, word_under_cursor),
                                                documentation: Some(doc),
                                            });
                                        }
                                    }
                                }

                                if hover_found.is_none() {
                                    for s in &scanned_structs {
                                        if s.name == *typ {
                                            if let Some(m) = s
                                                .method_details
                                                .iter()
                                                .find(|m| m.name == word_under_cursor)
                                            {
                                                let doc = if let Some(d) = &m.doc {
                                                    format!(
                                                        "```flame\n{}\n```\n\n{}",
                                                        m.signature, d
                                                    )
                                                } else {
                                                    format!("```flame\n{}\n```", m.signature)
                                                };
                                                hover_found = Some(JsonHover {
                                                    label: format!("{}::{}()", typ, m.name),
                                                    documentation: Some(doc),
                                                });
                                                break;
                                            }
                                            if let Some((field_name, field_type)) = s
                                                .fields
                                                .iter()
                                                .find(|(f, _)| f == &word_under_cursor)
                                            {
                                                hover_found = Some(JsonHover {
                                                    label: format!(
                                                        "{}.{}: {}",
                                                        typ, field_name, field_type
                                                    ),
                                                    documentation: Some(format!(
                                                        "```flame\n{}.{}: {}\n```\nField of `{}`",
                                                        typ, field_name, field_type, typ
                                                    )),
                                                });
                                                break;
                                            }
                                            if let Some(func_name) =
                                                s.methods.iter().find(|&f| f == &word_under_cursor)
                                            {
                                                let sig = format!("fn {}(...)", func_name);
                                                hover_found = Some(JsonHover {
                                                    label: format!("{}::{}()", typ, func_name),
                                                    documentation: Some(format!(
                                                        "```flame\n{}\n```",
                                                        sig
                                                    )),
                                                });
                                                break;
                                            }
                                        }
                                    }
                                }
                                if hover_found.is_none() {
                                    for mod_name in &native_modules {
                                        if let Some(meta) =
                                            load_meta_from_project(&manifest_dir, mod_name)
                                        {
                                            if let Some(struct_meta) =
                                                meta.structs.iter().find(|s| s.name == *typ)
                                            {
                                                if let Some(function) = struct_meta
                                                    .methods
                                                    .iter()
                                                    .find(|f| f.flame_name == word_under_cursor)
                                                {
                                                    let params_str = function
                                                        .params
                                                        .iter()
                                                        .map(|p| {
                                                            format!("{}: {}", p.name, p.type_name)
                                                        })
                                                        .collect::<Vec<_>>()
                                                        .join(", ");
                                                    let sig = format!(
                                                        "fn {}({}) -> {}",
                                                        function.flame_name,
                                                        params_str,
                                                        function.return_type
                                                    );
                                                    hover_found = Some(JsonHover {
                                                        label: format!(
                                                            "{}.{}",
                                                            typ, function.flame_name
                                                        ),
                                                        documentation: Some(format!(
                                                            "```flame\n{}\n```\n{}",
                                                            sig,
                                                            function
                                                                .docs
                                                                .clone()
                                                                .unwrap_or_default()
                                                        )),
                                                    });
                                                    break;
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

        if !resolved_as_var {
            let meta_match = lookup_namespaces.iter().find_map(|ns| {
                load_meta_from_project(&manifest_dir, ns).map(|m| (ns.to_string(), m))
            });
            if let Some((meta_ns, meta)) = meta_match {
                for function in &meta.functions {
                    if member_prefix
                        .as_deref()
                        .map(|prefix| function.flame_name.starts_with(prefix))
                        .unwrap_or(true)
                    {
                        completions.push(JsonCompletion {
                            sort_text: None,
                            label: function.flame_name.clone(),
                            kind: "function".to_string(),
                            detail: format!("native.{}", namespace),
                            documentation: function.docs.clone().or_else(|| {
                                load_local_rust_doc(&manifest_dir, &meta_ns, &function.flame_name)
                            }),
                        });
                    }
                }
                // Add struct names themselves as completions
                for struct_meta in &meta.structs {
                    if member_prefix
                        .as_deref()
                        .map_or(true, |prefix| struct_meta.name.starts_with(prefix))
                    {
                        completions.push(JsonCompletion {
                            sort_text: Some("2_".to_string()),
                            label: struct_meta.name.clone(),
                            kind: "class".to_string(),
                            detail: format!("struct (from {})", namespace),
                            documentation: struct_meta.docs.clone(),
                        });
                    }

                    if struct_meta.name.to_lowercase() == meta_ns.to_lowercase()
                        || struct_meta.name.to_lowercase() == namespace.to_lowercase()
                    {
                        for function in &struct_meta.methods {
                            if member_prefix
                                .as_deref()
                                .map(|prefix| function.flame_name.starts_with(prefix))
                                .unwrap_or(true)
                            {
                                completions.push(JsonCompletion {
                                    sort_text: None,
                                    label: function.flame_name.clone(),
                                    kind: "function".to_string(),
                                    detail: format!("native.{}", namespace),
                                    documentation: function.docs.clone().or_else(|| {
                                        load_local_rust_doc(
                                            &manifest_dir,
                                            &meta_ns,
                                            &function.flame_name,
                                        )
                                    }),
                                });
                            }
                        }
                    }
                }
                if !word_under_cursor.is_empty() {
                    hover_found = meta
                        .functions
                        .iter()
                        .find(|function| function.flame_name == word_under_cursor)
                        .map(|function| {
                            let params_str = function
                                .params
                                .iter()
                                .map(|p| format!("{}: {}", p.name, p.type_name))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let sig = format!(
                                "fn {}({}) -> {}",
                                function.flame_name, params_str, function.return_type
                            );
                            let doc = function.docs.clone().unwrap_or_else(|| {
                                load_local_rust_doc(&manifest_dir, &meta_ns, &function.flame_name)
                                    .unwrap_or_default()
                            });
                            JsonHover {
                                label: format!("{}.{}", namespace, function.flame_name),
                                documentation: Some(format!(
                                    "```flame\n{}\n```\n{}\n\n**Return Type / Structure**: `{}`",
                                    sig, doc, function.return_type
                                )),
                            }
                        });

                    if hover_found.is_none() {
                        for struct_meta in &meta.structs {
                            if struct_meta.name.to_lowercase() == meta_ns.to_lowercase()
                                || struct_meta.name.to_lowercase() == namespace.to_lowercase()
                            {
                                if let Some(function) = struct_meta
                                    .methods
                                    .iter()
                                    .find(|f| f.flame_name == word_under_cursor)
                                {
                                    let params_str = function
                                        .params
                                        .iter()
                                        .map(|p| format!("{}: {}", p.name, p.type_name))
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    let sig = format!(
                                        "fn {}({}) -> {}",
                                        function.flame_name, params_str, function.return_type
                                    );
                                    let doc = function.docs.clone().unwrap_or_else(|| {
                                        load_local_rust_doc(
                                            &manifest_dir,
                                            &meta_ns,
                                            &function.flame_name,
                                        )
                                        .unwrap_or_default()
                                    });
                                    hover_found = Some(JsonHover {
                                        label: format!("{}.{}", namespace, function.flame_name),
                                        documentation: Some(format!(
                                            "```flame\n{}\n```\n{}\n\n**Return Type / Structure**: `{}`",
                                            sig, doc, function.return_type
                                        )),
                                    });
                                    break;
                                }
                            }
                        }
                    }

                    if hover_found.is_none() {
                        if let Some(struct_meta) =
                            meta.structs.iter().find(|s| s.name == word_under_cursor)
                        {
                            let doc = struct_meta.docs.clone().unwrap_or_default();
                            hover_found = Some(JsonHover {
                                label: format!("{}::{}", namespace, struct_meta.name),
                                documentation: Some(format!(
                                    "```flame\nstruct {}\n```\n{}",
                                    struct_meta.name, doc
                                )),
                            });
                        }
                    }
                }
            } else if let Some((_def_ns, def)) = lookup_namespaces
                .iter()
                .find_map(|ns| ide::get_native_module_def(ns).map(|d| (ns.to_string(), d)))
            {
                for func in &def.functions {
                    if member_prefix
                        .as_deref()
                        .map_or(true, |p| func.name.starts_with(p))
                    {
                        completions.push(JsonCompletion {
                            sort_text: None,
                            label: func.name.clone(),
                            kind: "function".to_string(),
                            detail: format!("{}", func.return_type),
                            documentation: Some(format!(
                                "```flame\nfn {}({}) -> {}\n```\n{}",
                                func.name,
                                func.params
                                    .iter()
                                    .map(|(n, t)| format!("{}: {}", n, t))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                func.return_type,
                                func.description
                            )),
                        });
                    }
                    if !word_under_cursor.is_empty() && func.name == word_under_cursor {
                        hover_found = Some(JsonHover {
                            label: format!("{}::{}()", namespace, func.name),
                            documentation: Some(format!(
                                "```flame\nfn {}({}) -> {}\n```\n{}",
                                func.name,
                                func.params
                                    .iter()
                                    .map(|(n, t)| format!("{}: {}", n, t))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                func.return_type,
                                func.description
                            )),
                        });
                    }
                }
                for typ in &def.types {
                    if member_prefix
                        .as_deref()
                        .map_or(true, |p| typ.name.starts_with(p))
                    {
                        completions.push(JsonCompletion {
                            sort_text: None,
                            label: typ.name.clone(),
                            kind: "class".to_string(),
                            detail: "type".to_string(),
                            documentation: Some(format!(
                                "```flame\ntype {}\n```\n{}",
                                typ.name, typ.description
                            )),
                        });
                    }
                }

                // For hover:
                if !word_under_cursor.is_empty() {
                    if let Some(func) = def.functions.iter().find(|f| f.name == word_under_cursor) {
                        hover_found = Some(JsonHover {
                            label: format!("{}.{}()", namespace, func.name),
                            documentation: Some(format!(
                                "```flame\nfn {}({}) -> {}\n```\n{}",
                                func.name,
                                func.params
                                    .iter()
                                    .map(|(n, t)| format!("{}: {}", n, t))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                func.return_type,
                                func.description
                            )),
                        });
                    } else if let Some(typ) = def.types.iter().find(|t| t.name == word_under_cursor)
                    {
                        hover_found = Some(JsonHover {
                            label: format!("{}.{}", namespace, typ.name),
                            documentation: Some(format!(
                                "```flame\ntype {}\n```\n{}",
                                typ.name, typ.description
                            )),
                        });
                    }
                }
            } else if let Some(tc) = &tc_opt {
                let found_enum = lookup_namespaces
                    .iter()
                    .find_map(|ns| tc.enums.get(ns).map(|e| (ns.to_string(), e)));
                let found_methods = lookup_namespaces.iter().find_map(|ns| {
                    tc.methods
                        .iter()
                        .find(|(k, _)| *k == ns || k.ends_with(&format!(".{}", ns)))
                        .map(|(_, v)| v)
                });
                if found_enum.is_some() || found_methods.is_some() {
                    if let Some((_, enum_info)) = found_enum {
                        if !word_under_cursor.is_empty() {
                            if let Some((variant_name, variant_info)) = enum_info
                                .variants
                                .iter()
                                .find(|(n, _)| *n == &word_under_cursor)
                            {
                                hover_found = Some(JsonHover {
                                    label: format!("{}::{}", namespace, variant_name),
                                    documentation: variant_info.hover_doc.clone().or_else(|| {
                                        Some(format!(
                                            "```flame\n{}::{} variant\n```",
                                            namespace, variant_name
                                        ))
                                    }),
                                });
                            }
                        }
                    }
                    if let Some(methods) = found_methods {
                        for (method_name, sig) in methods {
                            if sig.is_static {
                                if member_prefix
                                    .as_deref()
                                    .map_or(true, |p| method_name.starts_with(p))
                                {
                                    completions.push(JsonCompletion {
                                        sort_text: None,
                                        label: method_name.clone(),
                                        kind: "function".to_string(),
                                        detail: format!("{} method", namespace),
                                        documentation: sig.hover_doc.clone(),
                                    });
                                }
                            }
                            if !word_under_cursor.is_empty() && method_name == &word_under_cursor {
                                let params_str = sig
                                    .params
                                    .iter()
                                    .map(|p| format!("{}: {:?}", p.name, p.ty))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let return_str = if sig.return_type == crate::typechecker::Type::Nil
                                {
                                    "".to_string()
                                } else {
                                    format!(" -> {:?}", sig.return_type)
                                };
                                let fallback_doc = format!(
                                    "```flame\nfn {}({}){}\n```",
                                    method_name, params_str, return_str
                                );
                                let final_doc = if let Some(doc) = &sig.hover_doc {
                                    format!("{}\n\n{}", fallback_doc, doc)
                                } else {
                                    fallback_doc
                                };
                                hover_found = Some(JsonHover {
                                    label: format!("{}::{}()", namespace, method_name),
                                    documentation: Some(final_doc),
                                });
                            }
                        }
                    }
                } else if let Some((std_ns, std_methods)) = lookup_namespaces
                    .iter()
                    .find_map(|ns| ide::get_std_module_methods(ns).map(|m| (ns.to_string(), m)))
                {
                    for method in &std_methods {
                        if member_prefix
                            .as_deref()
                            .map_or(true, |prefix| method.starts_with(prefix))
                        {
                            let doc = crate::blaze::get_std_function_doc(&std_ns, method)
                                .or_else(|| {
                                    effective_mod.as_ref().and_then(|em| {
                                        crate::blaze::get_std_function_doc(em, method)
                                    })
                                })
                                .or_else(|| crate::blaze::get_std_function_doc(&namespace, method));
                            completions.push(JsonCompletion {
                                sort_text: None,
                                label: method.clone(),
                                kind: "function".to_string(),
                                detail: format!("std.{}", namespace),
                                documentation: doc.map(|d| d.to_string()),
                            });
                        }
                    }

                    if !word_under_cursor.is_empty() && std_methods.contains(&word_under_cursor) {
                        let doc = crate::blaze::get_std_function_doc(&std_ns, &word_under_cursor)
                            .or_else(|| {
                                effective_mod.as_ref().and_then(|em| {
                                    crate::blaze::get_std_function_doc(em, &word_under_cursor)
                                })
                            })
                            .or_else(|| {
                                crate::blaze::get_std_function_doc(&namespace, &word_under_cursor)
                            });
                        if let Some(doc) = doc {
                            hover_found = Some(JsonHover {
                                label: format!("{namespace}.{word_under_cursor}()"),
                                documentation: Some(doc.to_string()),
                            });
                        } else {
                            hover_found = Some(JsonHover {
                                label: format!("{namespace}.{word_under_cursor}()"),
                                documentation: Some(format!(
                                    "Standard library function: {namespace}.{word_under_cursor}"
                                )),
                            });
                        }
                    }
                }
            } else if let Some((std_ns, std_methods)) = lookup_namespaces
                .iter()
                .find_map(|ns| ide::get_std_module_methods(ns).map(|m| (ns.to_string(), m)))
            {
                for method in &std_methods {
                    if member_prefix
                        .as_deref()
                        .map_or(true, |prefix| method.starts_with(prefix))
                    {
                        let doc = crate::blaze::get_std_function_doc(&std_ns, method)
                            .or_else(|| {
                                effective_mod
                                    .as_ref()
                                    .and_then(|em| crate::blaze::get_std_function_doc(em, method))
                            })
                            .or_else(|| crate::blaze::get_std_function_doc(&namespace, method));
                        completions.push(JsonCompletion {
                            sort_text: None,
                            label: method.clone(),
                            kind: "function".to_string(),
                            detail: format!("std.{}", namespace),
                            documentation: doc.map(|d| d.to_string()),
                        });
                    }
                }

                if !word_under_cursor.is_empty() && std_methods.contains(&word_under_cursor) {
                    let doc = crate::blaze::get_std_function_doc(&std_ns, &word_under_cursor)
                        .or_else(|| {
                            effective_mod.as_ref().and_then(|em| {
                                crate::blaze::get_std_function_doc(em, &word_under_cursor)
                            })
                        })
                        .or_else(|| {
                            crate::blaze::get_std_function_doc(&namespace, &word_under_cursor)
                        });
                    if let Some(doc) = doc {
                        hover_found = Some(JsonHover {
                            label: format!("{namespace}.{word_under_cursor}()"),
                            documentation: Some(doc.to_string()),
                        });
                    } else {
                        hover_found = Some(JsonHover {
                            label: format!("{namespace}.{word_under_cursor}()"),
                            documentation: Some(format!(
                                "Standard library function: {namespace}.{word_under_cursor}"
                            )),
                        });
                    }
                }
            } else if let Some((_loc_ns, local_stmts)) = lookup_namespaces.iter().find_map(|ns| {
                load_local_module_declarations(&manifest_dir, file, ns).map(|s| (ns.to_string(), s))
            }) {
                let mut provided_completions = false;

                for stmt in &local_stmts {
                    let func_info = match stmt {
                        crate::parser::Stmt::FuncDecl {
                            name,
                            params,
                            return_type,
                            annotations,
                            ..
                        } => Some((name, params, return_type, false, annotations)),
                        crate::parser::Stmt::AnnotationDecl {
                            name,
                            params,
                            return_type,
                            annotations,
                            ..
                        } => Some((name, params, return_type, true, annotations)),
                        crate::parser::Stmt::ExportDecl(inner, _) => match &**inner {
                            crate::parser::Stmt::FuncDecl {
                                name,
                                params,
                                return_type,
                                annotations,
                                ..
                            } => Some((name, params, return_type, false, annotations)),
                            crate::parser::Stmt::AnnotationDecl {
                                name,
                                params,
                                return_type,
                                annotations,
                                ..
                            } => Some((name, params, return_type, true, annotations)),
                            crate::parser::Stmt::StructDecl { name, .. } => {
                                completions.push(JsonCompletion {
                                    sort_text: None,
                                    label: name.clone(),
                                    kind: "class".to_string(),
                                    detail: "struct".to_string(),
                                    documentation: None,
                                });
                                provided_completions = true;
                                None
                            }
                            _ => None,
                        },
                        crate::parser::Stmt::StructDecl { name, .. } => {
                            completions.push(JsonCompletion {
                                sort_text: None,
                                label: name.clone(),
                                kind: "class".to_string(),
                                detail: "struct".to_string(),
                                documentation: None,
                            });
                            provided_completions = true;
                            None
                        }
                        _ => None,
                    };

                    if let Some((name, params, return_type, is_annotation, annotations)) = func_info
                    {
                        let param_strs = params
                            .iter()
                            .map(|p| {
                                format!(
                                    "{}{}: {}{}",
                                    if p.is_mut { "mut " } else { "" },
                                    p.name,
                                    if p.is_ref { "&" } else { "" },
                                    p.type_name
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        let sig = if is_annotation {
                            if let Some(ret) = return_type.as_deref() {
                                format!("annotation @{}({}) -> {}", name, param_strs, ret)
                            } else {
                                format!("annotation @{}({})", name, param_strs)
                            }
                        } else {
                            format!(
                                "fn {}({}) -> {}",
                                name,
                                param_strs,
                                return_type.as_deref().unwrap_or("Nil")
                            )
                        };

                        let actual_label = if is_annotation {
                            format!("@{}", name)
                        } else {
                            name.clone()
                        };

                        let mut doc_str = String::new();
                        for ann in annotations.iter() {
                            if ann.name == "Docs" {
                                if let Some(s) = ann.args.get(0) {
                                    let unquoted = s.trim_matches('"').replace("\\n", "\n");
                                    let lines: Vec<&str> = unquoted.lines().collect();
                                    let mut min_indent = usize::MAX;
                                    for line in &lines {
                                        if line.trim().is_empty() {
                                            continue;
                                        }
                                        let indent = line
                                            .chars()
                                            .take_while(|c| *c == ' ' || *c == '\t')
                                            .count();
                                        if indent < min_indent {
                                            min_indent = indent;
                                        }
                                    }
                                    let mut cleaned_doc = String::new();
                                    for line in &lines {
                                        if line.trim().is_empty() {
                                            cleaned_doc.push('\n');
                                        } else {
                                            let indent = if min_indent == usize::MAX {
                                                0
                                            } else {
                                                min_indent
                                            };
                                            let slice_start = std::cmp::min(indent, line.len());
                                            cleaned_doc.push_str(&line[slice_start..]);
                                            cleaned_doc.push('\n');
                                        }
                                    }
                                    let trimmed = cleaned_doc.trim();
                                    if !trimmed.is_empty() {
                                        doc_str = format!("\n\n{}", trimmed);
                                    }
                                }
                            }
                        }

                        if member_prefix
                            .as_deref()
                            .map_or(true, |prefix| name.starts_with(prefix))
                        {
                            if is_annotation && !word_under_cursor_raw.starts_with('@') {
                                // Only suggest annotations when user types @
                            } else {
                                completions.push(JsonCompletion {
                                    sort_text: Some("1_".to_string()),
                                    label: actual_label,
                                    kind: if is_annotation {
                                        "annotation".to_string()
                                    } else {
                                        "function".to_string()
                                    },
                                    detail: format!("module {}", namespace),
                                    documentation: Some(format!(
                                        "```flame\n{}\n```{}",
                                        sig, doc_str
                                    )),
                                });
                                provided_completions = true;
                            }
                        }

                        if !word_under_cursor.is_empty() && name == &word_under_cursor {
                            hover_found = Some(JsonHover {
                                label: format!("{}.{}()", namespace, name),
                                documentation: Some(format!("```flame\n{}\n```{}", sig, doc_str)),
                            });
                        }
                    }
                }

                if !provided_completions {
                    // If the local file had no such functions matching the prefix, we just do nothing here.
                }
            } else {
                let mut var_type = None;
                let mut is_instance = false;
                let mut pkg_suggestions = Vec::new();
                if let Some(first_part) = namespace.split('.').next() {
                    var_type = scanned_vars
                        .iter()
                        .find(|v| v.name == first_part)
                        .and_then(|v| {
                            is_instance = true;
                            v.typ.clone()
                        });

                    for part in namespace.split('.').skip(1) {
                        if let Some(vt) = var_type {
                            var_type = None;
                            if let Some(struct_def) = scanned_structs.iter().find(|s| s.name == vt)
                            {
                                if let Some(field) = struct_def.fields.iter().find(|f| f.0 == part)
                                {
                                    var_type = Some(field.1.clone());
                                }
                            }
                        }
                    }
                }

                // If not found as a variable, maybe it's a struct name directly?
                if var_type.is_none() {
                    is_instance = false;
                    if scanned_structs.iter().any(|s| s.name == namespace) {
                        var_type = Some(namespace.to_string());
                    } else if let Some(dot_idx) = namespace.find('.') {
                        let mod_name = &namespace[..dot_idx];
                        let struct_name = &namespace[dot_idx + 1..];
                        if native_modules.contains(&mod_name.to_string()) {
                            if let Some(meta) = load_meta_from_project(&manifest_dir, mod_name) {
                                if meta.structs.iter().any(|s| s.name == struct_name) {
                                    var_type = Some(struct_name.to_string());
                                }
                            }
                        }
                    } else {
                        let path_parts = vec![namespace.clone()];
                        if let Some(candidate) =
                            crate::stdlib::locate_import_file(Path::new(file), &path_parts).or_else(
                                || {
                                    let direct = manifest_dir.join(format!("{}.fm", namespace));
                                    if direct.exists() {
                                        return Some(direct);
                                    }
                                    let pkg_main = manifest_dir
                                        .join(".flame")
                                        .join("pkg")
                                        .join(&namespace)
                                        .join("src")
                                        .join("main.fm");
                                    if pkg_main.exists() {
                                        return Some(pkg_main);
                                    }
                                    None
                                },
                            )
                        {
                            eprintln!("DEBUG_CANDIDATE: {:?}", candidate);
                            match std::fs::read_to_string(&candidate) {
                                Ok(content) => {
                                    eprintln!("DEBUG_CANDIDATE_READ_OK, len={}", content.len());
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
                                    let mut parser = crate::parser::Parser::new(
                                        tokens,
                                        candidate.to_string_lossy().to_string(),
                                    );
                                    if let Ok(stmts) = parser.parse() {
                                        eprintln!("DEBUG_STMTS_LEN: {}", stmts.len());
                                        for stmt in stmts {
                                            if let crate::parser::Stmt::PackageDecl {
                                                annotations,
                                                ..
                                            } = stmt
                                            {
                                                for ann in annotations {
                                                    eprintln!(
                                                        "DEBUG_ANN: name={}, args={:?}",
                                                        ann.name, ann.args
                                                    );
                                                    if ann.name == "Suggestions"
                                                        && !ann.args.is_empty()
                                                    {
                                                        let s_args = ann.args.join(" ");
                                                        let re = regex::Regex::new(r"\{\s*name\s*:\s*([^,]+),\s*kind\s*:\s*([^,}]+)(?:,\s*doc\s*:\s*([^}]+))?\}").unwrap();
                                                        for cap in re.captures_iter(&s_args) {
                                                            let struct_name = cap[1]
                                                                .trim()
                                                                .trim_matches(|c| {
                                                                    c == '"' || c == '\''
                                                                })
                                                                .to_string();
                                                            let kind = cap[2]
                                                                .trim()
                                                                .trim_matches(|c| {
                                                                    c == '"' || c == '\''
                                                                })
                                                                .to_string();
                                                            let doc = cap.get(3).map(|m| {
                                                                m.as_str()
                                                                    .trim()
                                                                    .trim_matches(|c| {
                                                                        c == '"' || c == '\''
                                                                    })
                                                                    .to_string()
                                                            });
                                                            pkg_suggestions.push((
                                                                struct_name,
                                                                kind,
                                                                doc,
                                                            ));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("DEBUG_CANDIDATE_READ_ERR: {}", e);
                                }
                            }
                        }
                    }
                }
                // eprintln!("DEBUG_COMPLETIONS_PRE: namespace={}, var_type={:?}", namespace, var_type);

                let mut provided_completions = false;

                if let Some(tc) = &tc_opt {
                    if let Some(enum_info) = tc.enums.get(&namespace) {
                        for (variant_name, _) in &enum_info.variants {
                            if member_prefix
                                .as_deref()
                                .map_or(true, |p| variant_name.starts_with(p))
                            {
                                completions.push(JsonCompletion {
                                    sort_text: Some("1_".to_string()),
                                    label: variant_name.clone(),
                                    kind: "enumMember".to_string(),
                                    detail: format!("{} enum variant", namespace),
                                    documentation: None,
                                });
                                provided_completions = true;
                            }
                        }
                    } else if var_type.is_none() {
                        for (enum_name, enum_info) in &tc.enums {
                            if enum_name.starts_with(&format!("{}.", namespace)) {
                                let short_name =
                                    enum_name.strip_prefix(&format!("{}.", namespace)).unwrap();
                                if !short_name.contains('.')
                                    && member_prefix
                                        .as_deref()
                                        .map_or(true, |p| short_name.starts_with(p))
                                {
                                    completions.push(JsonCompletion {
                                        sort_text: Some("1_".to_string()),
                                        label: short_name.to_string(),
                                        kind: "enum".to_string(),
                                        detail: format!("{} enum", namespace),
                                        documentation: enum_info.hover_doc.clone(),
                                    });
                                    provided_completions = true;
                                }
                            }
                        }
                        for (struct_name, s_info) in &tc.structs {
                            if struct_name.starts_with(&format!("{}.", namespace)) {
                                let short_name = struct_name
                                    .strip_prefix(&format!("{}.", namespace))
                                    .unwrap();
                                if !short_name.contains('.')
                                    && member_prefix
                                        .as_deref()
                                        .map_or(true, |p| short_name.starts_with(p))
                                {
                                    completions.push(JsonCompletion {
                                        sort_text: Some("1_".to_string()),
                                        label: short_name.to_string(),
                                        kind: "struct".to_string(),
                                        detail: format!("{} struct", namespace),
                                        documentation: s_info.hover_doc.clone(),
                                    });
                                    provided_completions = true;
                                }
                            }
                        }
                    }
                }

                // If it's a standard module directly (e.g. `json.`, `tcp.`, `http.`)
                let is_std_module = std_modules.contains(&namespace)
                    || matches!(
                        namespace.as_str(),
                        "json"
                            | "tcp"
                            | "udp"
                            | "http"
                            | "ws"
                            | "mqtt"
                            | "dns"
                            | "url"
                            | "interface"
                    );
                if var_type.is_none() {
                    if is_std_module {
                        if let Some(methods) = ide::get_std_module_methods(&namespace) {
                            for method in methods {
                                if member_prefix
                                    .as_deref()
                                    .map_or(true, |p| method.starts_with(p))
                                {
                                    let doc =
                                        crate::blaze::get_std_function_doc(&namespace, &method);
                                    completions.push(JsonCompletion {
                                        sort_text: Some("4_".to_string()),
                                        label: method.clone(),
                                        kind: "function".to_string(),
                                        detail: format!("std.{} function", namespace),
                                        documentation: doc.map(|d| d.to_string()),
                                    });
                                    provided_completions = true;
                                }
                                if !word_under_cursor.is_empty() && method == word_under_cursor {
                                    let doc =
                                        crate::blaze::get_std_function_doc(&namespace, &method);
                                    hover_found = Some(JsonHover {
                                        label: format!("std.{}::{}()", namespace, method),
                                        documentation: doc.map(|d| d.to_string()), // std docs dynamically parsed from Blaze/std
                                    });
                                }
                            }
                        }
                    } else if !pkg_suggestions.is_empty() {
                        for (s_name, s_kind, s_doc) in &pkg_suggestions {
                            if member_prefix
                                .as_deref()
                                .map_or(true, |p| s_name.starts_with(p))
                            {
                                completions.push(JsonCompletion {
                                    sort_text: Some("1_".to_string()),
                                    label: s_name.clone(),
                                    kind: s_kind.clone(),
                                    detail: format!("{} {}", namespace, s_kind),
                                    documentation: s_doc.clone(),
                                });
                                provided_completions = true;
                            }
                        }
                    }
                }

                // If it's a native module directly (e.g. `flamer.`)
                if !provided_completions
                    && var_type.is_none()
                    && native_modules.contains(&namespace)
                {
                    if let Some(meta) = load_meta_from_project(&manifest_dir, &namespace) {
                        for function in &meta.functions {
                            if member_prefix
                                .as_deref()
                                .map(|p| function.flame_name.starts_with(p))
                                .unwrap_or(true)
                            {
                                let params_str = function
                                    .params
                                    .iter()
                                    .map(|p| format!("{}: {}", p.name, p.type_name))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let sig = format!(
                                    "fn {}({}) -> {}",
                                    function.flame_name, params_str, function.return_type
                                );

                                completions.push(JsonCompletion {
                                    sort_text: None,
                                    label: function.flame_name.clone(),
                                    kind: "function".to_string(),
                                    detail: format!("{} (from {})", function.flame_name, namespace),
                                    documentation: Some(
                                        function.docs.clone().unwrap_or(sig.clone()),
                                    ),
                                });
                                provided_completions = true;
                            }

                            if !word_under_cursor.is_empty()
                                && function.flame_name == word_under_cursor
                            {
                                let params_str = function
                                    .params
                                    .iter()
                                    .map(|p| format!("{}: {}", p.name, p.type_name))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let sig = format!(
                                    "fn {}({}) -> {}",
                                    function.flame_name, params_str, function.return_type
                                );
                                let doc = function.docs.clone().unwrap_or_default();
                                let formatted_doc = if doc.trim().is_empty() {
                                    format!(
                                        "```flame\n{}\n```\n\n**Return Type**: `{}`",
                                        sig, function.return_type
                                    )
                                } else {
                                    format!(
                                        "```flame\n{}\n```\n{}\n\n**Return Type**: `{}`",
                                        sig,
                                        doc.trim(),
                                        function.return_type
                                    )
                                };

                                hover_found = Some(JsonHover {
                                    label: format!("{}::{}()", namespace, function.flame_name),
                                    documentation: Some(formatted_doc),
                                });
                            }
                        }

                        for struct_meta in &meta.structs {
                            // eprintln!("DEBUG_COMPLETIONS: Checking struct: {}", struct_meta.name);
                            if member_prefix
                                .as_deref()
                                .map(|p| struct_meta.name.starts_with(p))
                                .unwrap_or(true)
                            {
                                // eprintln!("DEBUG_COMPLETIONS: Adding struct: {}", struct_meta.name);
                                completions.push(JsonCompletion {
                                    sort_text: Some("2_".to_string()),
                                    label: struct_meta.name.clone(),
                                    kind: "class".to_string(),
                                    detail: format!("struct (from {})", namespace),
                                    documentation: struct_meta.docs.clone(),
                                });
                                provided_completions = true;
                            }

                            if !word_under_cursor.is_empty()
                                && struct_meta.name == word_under_cursor
                            {
                                let doc = struct_meta.docs.clone().unwrap_or_default();
                                let formatted_doc = if doc.trim().is_empty() {
                                    format!("```flame\nstruct {}\n```", struct_meta.name)
                                } else {
                                    format!(
                                        "```flame\nstruct {}\n```\n{}",
                                        struct_meta.name,
                                        doc.trim()
                                    )
                                };
                                hover_found = Some(JsonHover {
                                    label: format!("{}::{}", namespace, struct_meta.name),
                                    documentation: Some(formatted_doc),
                                });
                            }
                        }
                    }
                }

                if let Some(t) = &var_type {
                    // eprintln!("DEBUG_COMPLETIONS: namespace={}, var_type={:?}, t={}, is_instance={}", namespace, var_type, t, is_instance);
                    if let Some(struct_def) = scanned_structs.iter().find(|s| s.name == *t) {
                        for field in &struct_def.fields {
                            if member_prefix
                                .as_deref()
                                .map_or(true, |prefix| field.0.starts_with(prefix))
                            {
                                completions.push(JsonCompletion {
                                    sort_text: Some("1_".to_string()),
                                    label: field.0.clone(),
                                    kind: "property".to_string(),
                                    detail: format!("{} field", t),
                                    documentation: None,
                                });
                                provided_completions = true;
                            }
                        }
                        for method in &struct_def.methods {
                            if member_prefix
                                .as_deref()
                                .map_or(true, |prefix| method.starts_with(prefix))
                            {
                                completions.push(JsonCompletion {
                                    sort_text: Some("1_".to_string()),
                                    label: method.clone(),
                                    kind: "method".to_string(),
                                    detail: format!("{} method", t),
                                    documentation: None,
                                });
                                provided_completions = true;
                            }
                        }
                    }

                    if let Some(tc) = &tc_opt {
                        if let Some(s_info) = tc.structs.get(t) {
                            for (field_name, _) in &s_info.fields {
                                if member_prefix
                                    .as_deref()
                                    .map_or(true, |prefix| field_name.starts_with(prefix))
                                {
                                    completions.push(JsonCompletion {
                                        sort_text: Some("1_".to_string()),
                                        label: field_name.clone(),
                                        kind: "property".to_string(),
                                        detail: format!("{} field", t),
                                        documentation: s_info.hover_doc.clone(),
                                    });
                                    provided_completions = true;
                                }
                            }
                        }
                        if let Some(methods) = tc.methods.get(t) {
                            for (method_name, m_sig) in methods {
                                if member_prefix
                                    .as_deref()
                                    .map_or(true, |prefix| method_name.starts_with(prefix))
                                {
                                    completions.push(JsonCompletion {
                                        sort_text: Some("1_".to_string()),
                                        label: method_name.clone(),
                                        kind: "method".to_string(),
                                        detail: format!("{} method", t),
                                        documentation: m_sig.hover_doc.clone(),
                                    });
                                    provided_completions = true;
                                }
                            }
                        }
                        if let Some(enum_info) = tc.enums.get(t) {
                            for (variant_name, _) in &enum_info.variants {
                                if member_prefix
                                    .as_deref()
                                    .map_or(true, |prefix| variant_name.starts_with(prefix))
                                {
                                    completions.push(JsonCompletion {
                                        sort_text: Some("1_".to_string()),
                                        label: variant_name.clone(),
                                        kind: "enumMember".to_string(),
                                        detail: format!("{} variant", t),
                                        documentation: None,
                                    });
                                    provided_completions = true;
                                }
                            }
                        }
                    }

                    // Also check native types like ThreadHandler or FlameServer across all modules
                    if !provided_completions {
                        let modules_to_check = {
                            let mut mods = native_modules.clone();
                            if !mods.contains(&t) {
                                mods.push(t.clone());
                            }
                            mods
                        };
                        for mod_name in modules_to_check {
                            if let Some(meta) = load_meta_from_project(&manifest_dir, &mod_name) {
                                // eprintln!("DEBUG_COMPLETIONS: loaded meta for {}", mod_name);
                                for struct_meta in &meta.structs {
                                    if struct_meta.name == *t
                                        || struct_meta.name.to_lowercase() == t.to_lowercase()
                                    {
                                        // eprintln!("DEBUG_COMPLETIONS: matched struct {}", struct_meta.name);
                                        for function in &struct_meta.methods {
                                            if is_instance && function.is_static {
                                                continue;
                                            }
                                            if !is_instance && !function.is_static {
                                                continue;
                                            }

                                            if member_prefix
                                                .as_deref()
                                                .map(|p| function.flame_name.starts_with(p))
                                                .unwrap_or(true)
                                            {
                                                completions.push(JsonCompletion {
                                                    sort_text: Some("1_".to_string()),
                                                    label: function.flame_name.clone(),
                                                    kind: "method".to_string(),
                                                    detail: format!(
                                                        "{}::{} (from {})",
                                                        struct_meta.name,
                                                        function.flame_name,
                                                        mod_name
                                                    ),
                                                    documentation: function.docs.clone().or_else(
                                                        || {
                                                            load_local_rust_doc(
                                                                &manifest_dir,
                                                                &mod_name,
                                                                &function.flame_name,
                                                            )
                                                        },
                                                    ),
                                                });
                                                provided_completions = true;
                                            }
                                            if !word_under_cursor.is_empty()
                                                && function.flame_name == word_under_cursor
                                            {
                                                let params_str = function
                                                    .params
                                                    .iter()
                                                    .map(|p| format!("{}: {}", p.name, p.type_name))
                                                    .collect::<Vec<_>>()
                                                    .join(", ");
                                                let sig = format!(
                                                    "fn {}({}) -> {}",
                                                    function.flame_name,
                                                    params_str,
                                                    function.return_type
                                                );
                                                let doc =
                                                    function.docs.clone().unwrap_or_else(|| {
                                                        load_local_rust_doc(
                                                            &manifest_dir,
                                                            &mod_name,
                                                            &function.flame_name,
                                                        )
                                                        .unwrap_or_default()
                                                    });
                                                let final_doc = if function.docs.is_some() {
                                                    doc
                                                } else {
                                                    if doc.trim().is_empty() {
                                                        format!(
                                                            "```flame\n{}\n```\n\n**Return Type / Structure**: `{}`",
                                                            sig, function.return_type
                                                        )
                                                    } else {
                                                        format!(
                                                            "```flame\n{}\n```\n{}\n\n**Return Type / Structure**: `{}`",
                                                            sig,
                                                            doc.trim(),
                                                            function.return_type
                                                        )
                                                    }
                                                };
                                                hover_found = Some(JsonHover {
                                                    label: format!(
                                                        "{}::{}()",
                                                        struct_meta.name, function.flame_name
                                                    ),
                                                    documentation: Some(final_doc),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !provided_completions {
                        if t == "Sender" {
                            let sender_methods = [
                                (
                                    "send",
                                    "fn send(value: Any) -> Nil",
                                    "```flame\nfn send(value: Any) -> Nil\n```\nSends a message value through the channel to the connected Receiver.\n\n**Example**:\n```flame\ntx.send(\"hello\")\n```",
                                ),
                                (
                                    "clone",
                                    "fn clone() -> Sender",
                                    "```flame\nfn clone() -> Sender\n```\nClones the channel sender handle so multiple threads can send messages to the same receiver.\n\n**Example**:\n```flame\nlet tx2 = tx.clone()\n```",
                                ),
                            ];
                            for (name, detail, doc) in sender_methods {
                                if member_prefix
                                    .as_deref()
                                    .map_or(true, |p| name.starts_with(p))
                                {
                                    completions.push(JsonCompletion {
                                        sort_text: Some("0_".to_string()),
                                        label: name.to_string(),
                                        kind: "method".to_string(),
                                        detail: detail.to_string(),
                                        documentation: Some(doc.to_string()),
                                    });
                                    provided_completions = true;
                                }
                            }
                        } else if t == "Receiver" {
                            let receiver_methods = [
                                (
                                    "recv",
                                    "fn recv() -> Any",
                                    "```flame\nfn recv() -> Any\n```\nBlocks the current thread until a message is received from the channel.\n\n**Example**:\n```flame\nlet msg = rx.recv()\n```",
                                ),
                                (
                                    "tryRecv",
                                    "fn tryRecv() -> Any | Nil",
                                    "```flame\nfn tryRecv() -> Any | Nil\n```\nAttempts to receive a message without blocking. Returns nil immediately if the channel is currently empty.\n\n**Example**:\n```flame\nlet msg = rx.tryRecv()\nif msg != nil {\n    println($\"Received: {msg}\")\n}\n```",
                                ),
                                (
                                    "isEmpty",
                                    "fn isEmpty() -> Bool",
                                    "```flame\nfn isEmpty() -> Bool\n```\nReturns true if there are no pending messages in the channel receiver.\n\n**Example**:\n```flame\nif !rx.isEmpty() {\n    let msg = rx.recv()\n}\n```",
                                ),
                            ];
                            for (name, detail, doc) in receiver_methods {
                                if member_prefix
                                    .as_deref()
                                    .map_or(true, |p| name.starts_with(p))
                                {
                                    completions.push(JsonCompletion {
                                        sort_text: Some("0_".to_string()),
                                        label: name.to_string(),
                                        kind: "method".to_string(),
                                        detail: detail.to_string(),
                                        documentation: Some(doc.to_string()),
                                    });
                                    provided_completions = true;
                                }
                            }
                        }
                    }

                    if !provided_completions {
                        let native_module_lookup = match t.as_str() {
                            "ThreadHandler" => Some("thread"),
                            "ProcessHandler" => Some("process"),
                            "File" => Some("fs"),
                            "TcpStream" | "TcpListener" | "UdpSocket" => Some("net"),
                            _ => None,
                        };

                        if let Some(mod_name) = native_module_lookup {
                            if let Some(std_methods) = ide::get_std_module_methods(mod_name) {
                                for method in &std_methods {
                                    if member_prefix
                                        .as_deref()
                                        .map_or(true, |prefix| method.starts_with(prefix))
                                    {
                                        completions.push(JsonCompletion {
                                            sort_text: None,
                                            label: method.clone(),
                                            kind: "method".to_string(),
                                            detail: format!("{} method", t),
                                            documentation: None,
                                        });
                                        provided_completions = true;
                                    }
                                }
                            }
                        }
                    }
                }

                if !provided_completions
                    && !list_std_modules(&manifest_dir).contains(&namespace)
                    && !native_modules.contains(&namespace)
                    && !alias_map.contains_key(&namespace)
                {
                    // Fallback for primitive and collection methods
                    let builtin_methods = vec![
                        ("type", "Returns the type of the value as a string"),
                        ("toString", "Converts the value to a string representation"),
                        ("toJson", "Serializes a struct or object into a JSON string"),
                        (
                            "len",
                            "Returns the length in bytes (String) or elements (Array)",
                        ),
                        ("isEmpty", "Returns true if empty"),
                    ];

                    for (method, doc) in &builtin_methods {
                        if member_prefix
                            .as_deref()
                            .map(|prefix| method.starts_with(prefix))
                            .unwrap_or(true)
                        {
                            completions.push(JsonCompletion {
                                sort_text: Some("3_".to_string()),
                                label: method.to_string(),
                                kind: "method".to_string(),
                                detail: "built-in method".to_string(),
                                documentation: Some(doc.to_string()),
                            });
                        }
                    }

                    if !word_under_cursor.is_empty() {
                        hover_found = builtin_methods
                            .into_iter()
                            .find(|(m, _)| *m == word_under_cursor)
                            .map(|(m, doc)| JsonHover {
                                label: format!("{m}()"),
                                documentation: Some(format!("```flame\nfn {m}(...)\n```\n{}", doc)),
                            });
                    }
                }
            }
        } // closes if !resolved_as_var
    } else {
        // Keyword completions for bare words
        completions.extend(ide::get_keyword_completions(
            current_line,
            &word_under_cursor_raw,
            &word_under_cursor,
            tc_opt.as_ref(),
        ));

        if !word_under_cursor.is_empty() {
            if let Some(doc) = crate::blaze::get_std_module_doc(&word_under_cursor) {
                hover_found = Some(JsonHover {
                    label: word_under_cursor.clone(),
                    documentation: Some(format!(
                        "```flame\nmodule {}\n```\n{}",
                        word_under_cursor, doc
                    )),
                });
            } else if let Some(v) = scanned_vars.iter().find(|v| v.name == word_under_cursor) {
                if let Some(typ) = &v.typ {
                    if let Some(meta) = load_meta_from_project(&manifest_dir, typ) {
                        let struct_names = meta
                            .structs
                            .iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let desc = format!(
                            "```flame\nlet {}: native.{}\n```\n**Native Plugin Instance** (`{}`)\n\n**Structures in plugin**: `{}`",
                            v.name, typ, typ, struct_names
                        );
                        hover_found = Some(JsonHover {
                            label: format!("{}: native.{}", v.name, typ),
                            documentation: Some(desc),
                        });
                    } else {
                        let is_func_or_anno =
                            typ.starts_with("fn ") || typ.starts_with("annotation ");

                        if is_func_or_anno {
                            let doc = v
                                .doc
                                .clone()
                                .unwrap_or_else(|| format!("```flame\n{}\n```", typ));
                            hover_found = Some(JsonHover {
                                label: format!("{}", typ),
                                documentation: Some(doc),
                            });
                        } else {
                            let doc = v.doc.clone().unwrap_or_else(|| {
                                format!("```flame\nlet {}: {}\n```", v.name, typ)
                            });
                            hover_found = Some(JsonHover {
                                label: format!("{}: {}", v.name, typ),
                                documentation: Some(doc),
                            });
                        }
                    }
                } else {
                    hover_found = Some(JsonHover {
                        label: format!("{}: Unknown", v.name),
                        documentation: Some(format!("```flame\nlet {}: Unknown\n```", v.name)),
                    });
                }
            }
        }

        // Provide variables as completion for bare words
        for v in &scanned_vars {
            let typ_str = v.typ.clone().unwrap_or_else(|| "unknown".to_string());
            let is_annotation = typ_str.starts_with("annotation ");

            // If user typed `@`, ONLY show annotations!
            if word_under_cursor_raw.starts_with('@') && !is_annotation {
                continue;
            }

            if v.name.starts_with(&word_under_cursor) || word_under_cursor.is_empty() {
                let mut kind = "variable".to_string();
                let mut label = v.name.clone();
                let mut sort_text = Some("0_".to_string());

                if typ_str.starts_with("fn ") {
                    kind = "function".to_string();
                    sort_text = Some("1_".to_string());
                } else if is_annotation {
                    kind = "annotation".to_string();
                    sort_text = Some("0_".to_string()); // Workspace annotations first
                    if !label.starts_with('@') {
                        label = format!("@{}", label);
                    }
                }

                completions.push(JsonCompletion {
                    sort_text,
                    label,
                    kind,
                    detail: typ_str,
                    documentation: None,
                });
            }
        }

        // Provide structs as completion for bare words
        if !word_under_cursor_raw.starts_with('@') {
            for s in &scanned_structs {
                if s.name.starts_with(&word_under_cursor) || word_under_cursor.is_empty() {
                    completions.push(JsonCompletion {
                        sort_text: None,
                        label: s.name.clone(),
                        kind: "class".to_string(),
                        detail: "struct".to_string(),
                        documentation: None,
                    });
                }
            }
        }

        if hover_found.is_none()
            && exact_ast_hover.is_none()
            && scanned_var_hover.is_none()
            && !word_under_cursor.is_empty()
        {
            if let Some(tc) = &tc_opt {
                if let Some(s) = tc
                    .structs
                    .iter()
                    .find(|(k, _)| {
                        k == &&word_under_cursor || k.ends_with(&format!(".{}", word_under_cursor))
                    })
                    .map(|(_, v)| v)
                {
                    let doc = s.hover_doc.clone().unwrap_or_default();
                    let final_doc = format!("```flame\nstruct {}\n```\n{}", word_under_cursor, doc);
                    hover_found = Some(JsonHover {
                        label: word_under_cursor.clone(),
                        documentation: Some(final_doc),
                    });
                } else if let Some(e) = tc
                    .enums
                    .iter()
                    .find(|(k, _)| {
                        k == &&word_under_cursor || k.ends_with(&format!(".{}", word_under_cursor))
                    })
                    .map(|(_, v)| v)
                {
                    let doc = e.hover_doc.clone().unwrap_or_default();
                    let final_doc = format!("```flame\nenum {}\n```\n{}", word_under_cursor, doc);
                    hover_found = Some(JsonHover {
                        label: word_under_cursor.clone(),
                        documentation: Some(final_doc),
                    });
                } else if let Some(f) = tc
                    .functions
                    .iter()
                    .find(|(k, _)| {
                        k == &&word_under_cursor || k.ends_with(&format!(".{}", word_under_cursor))
                    })
                    .map(|(_, v)| v)
                {
                    let params_str = f
                        .params
                        .iter()
                        .map(|p| format!("{}: {:?}", p.name, p.ty))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let return_str = if f.return_type == crate::typechecker::Type::Nil {
                        "".to_string()
                    } else {
                        format!(" -> {:?}", f.return_type)
                    };
                    let fallback_doc = format!(
                        "```flame\nfn {}({}){}\n```",
                        word_under_cursor, params_str, return_str
                    );
                    let final_doc = if let Some(doc) = &f.hover_doc {
                        format!("{}\n{}", fallback_doc, doc)
                    } else {
                        fallback_doc
                    };
                    hover_found = Some(JsonHover {
                        label: format!("{}()", word_under_cursor),
                        documentation: Some(final_doc),
                    });
                } else if let Some(impl_name) = &current_impl {
                    if let Some(methods) = tc.methods.get(impl_name) {
                        if let Some((_, f)) = methods.iter().find(|(k, _)| k == &&word_under_cursor)
                        {
                            let params_str = f
                                .params
                                .iter()
                                .map(|p| format!("{}: {:?}", p.name, p.ty))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let return_str = if f.return_type == crate::typechecker::Type::Nil {
                                "".to_string()
                            } else {
                                format!(" -> {:?}", f.return_type)
                            };
                            let fallback_doc = format!(
                                "```flame\nfn {}({}){}\n```",
                                word_under_cursor, params_str, return_str
                            );
                            let final_doc = if let Some(doc) = &f.hover_doc {
                                format!("{}\n{}", fallback_doc, doc)
                            } else {
                                fallback_doc
                            };
                            hover_found = Some(JsonHover {
                                label: format!("{}::{}()", impl_name, word_under_cursor),
                                documentation: Some(final_doc),
                            });
                        }
                    }
                }
            }

            if hover_found.is_none() {
                // Check if the bare word is a function/annotation from any native module
                for mod_name in &native_modules {
                    if let Some(meta) = load_meta_from_project(&manifest_dir, mod_name) {
                        if let Some(function) = meta
                            .functions
                            .iter()
                            .find(|f| f.flame_name == word_under_cursor)
                        {
                            let params_str = function
                                .params
                                .iter()
                                .map(|p| format!("{}: {}", p.name, p.type_name))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let sig = format!(
                                "fn {}({}) -> {}",
                                function.flame_name, params_str, function.return_type
                            );
                            let doc = function.docs.clone().unwrap_or_else(|| {
                                load_local_rust_doc(&manifest_dir, mod_name, &function.flame_name)
                                    .unwrap_or_default()
                            });
                            let final_doc = if doc.trim().is_empty() {
                                format!(
                                    "```flame\n{}\n```\n\n**Return Type**: `{}`",
                                    sig, function.return_type
                                )
                            } else {
                                format!(
                                    "```flame\n{}\n```\n{}\n\n**Return Type**: `{}`",
                                    sig, doc, function.return_type
                                )
                            };
                            hover_found = Some(JsonHover {
                                label: format!("{}::{}()", mod_name, function.flame_name),
                                documentation: Some(final_doc),
                            });
                            break;
                        }
                    }
                }
            }
        } // closes if !resolved_as_var
    };

    // Prioritize rich documentation (keywords, built-ins, standard library, decorators, native docs).

    let mut signature_help = None;
    if let Some(prefix) = current_line.get(..cursor_col.saturating_sub(1)) {
        let mut open_parens = 0;
        let mut chars = prefix.chars().rev().enumerate();
        let mut found_call = false;
        let mut commas = 0;
        let mut call_start_idx = 0;

        while let Some((i, c)) = chars.next() {
            if c == ')' {
                open_parens += 1;
            } else if c == ',' && open_parens == 0 {
                commas += 1;
            } else if c == '(' {
                if open_parens > 0 {
                    open_parens -= 1;
                } else {
                    found_call = true;
                    call_start_idx = prefix.len() - 1 - i;
                    break;
                }
            }
        }

        if found_call {
            let func_name_str = prefix[..call_start_idx].trim_end();
            let name_end = func_name_str.len();
            let mut name_start = name_end;
            for (i, c) in func_name_str.chars().rev().enumerate() {
                if !c.is_alphanumeric() && c != '_' && c != '.' && c != '@' {
                    name_start = name_end - i;
                    break;
                }
                if i == name_end - 1 {
                    name_start = 0;
                }
            }
            if name_start < name_end {
                let func_name = &func_name_str[name_start..name_end];
                let clean_name = func_name
                    .trim_start_matches('@')
                    .split('.')
                    .last()
                    .unwrap_or(func_name);

                if let Some(tc) = &tc_opt {
                    let mut found_sig = tc.functions.get(clean_name);

                    if found_sig.is_none() {
                        for (_, methods) in &tc.methods {
                            if let Some(sig) = methods.get(clean_name) {
                                found_sig = Some(sig);
                                break;
                            }
                        }
                    }

                    if found_sig.is_none() {
                        for (_, funcs) in &tc.plugin_functions {
                            if let Some(sig) = funcs.get(clean_name) {
                                found_sig = Some(sig);
                                break;
                            }
                        }
                    }

                    if let Some(func) = found_sig {
                        let params_str = func
                            .params
                            .iter()
                            .map(|p| format!("{}: {}", p.name, tc.format_type(&p.ty)))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let ret_str = format!(" -> {}", tc.format_type(&func.return_type));

                        let label = if func_name.starts_with('@') {
                            format!("@{}({})", clean_name, params_str)
                        } else {
                            format!("{}({}){}", clean_name, params_str, ret_str)
                        };
                        let parameters = func
                            .params
                            .iter()
                            .map(|p| format!("{}: {}", p.name, tc.format_type(&p.ty)))
                            .collect();

                        signature_help = Some(JsonSignatureHelp {
                            label,
                            parameters,
                            active_parameter: commas,
                        });
                    }
                }
            }
        }
    }

    let mut hover = exact_ast_hover
        .or(hover_found)
        .or(scanned_var_hover)
        .or_else(|| ide::get_keyword_hover(&word_under_cursor));

    if let (Some(l), Some(c)) = (line, col) {
        fn is_pos_in_jsx_text(stmts: &[crate::parser::ast::Stmt], target_line: usize, target_col: usize) -> bool {
            fn check_expr(expr: &crate::parser::ast::Expr, target_line: usize, target_col: usize) -> bool {
                match expr {
                    crate::parser::ast::Expr::JsxElement { children, .. } => {
                        for child in children {
                            match child {
                                crate::parser::ast::JsxChild::Text(_, span) => {
                                    if span.line == target_line {
                                        let len = span.end.saturating_sub(span.start);
                                        if target_col >= span.col && target_col <= span.col + len + 1 {
                                            return true;
                                        }
                                    }
                                }
                                crate::parser::ast::JsxChild::Element(e) => {
                                    if check_expr(e, target_line, target_col) {
                                        return true;
                                    }
                                }
                                crate::parser::ast::JsxChild::Expr(e) => {
                                    if check_expr(e, target_line, target_col) {
                                        return true;
                                    }
                                }
                            }
                        }
                        false
                    }
                    crate::parser::ast::Expr::Block(stmts, _) => {
                        for s in stmts {
                            if check_stmt(s, target_line, target_col) {
                                return true;
                            }
                        }
                        false
                    }
                    crate::parser::ast::Expr::Closure { body, .. } => {
                        for s in body {
                            if check_stmt(s, target_line, target_col) {
                                return true;
                            }
                        }
                        false
                    }
                    _ => false,
                }
            }

            fn check_stmt(stmt: &crate::parser::ast::Stmt, target_line: usize, target_col: usize) -> bool {
                match stmt {
                    crate::parser::ast::Stmt::FuncDecl { body: Some(stmts), .. } => {
                        for s in stmts {
                            if check_stmt(s, target_line, target_col) {
                                return true;
                            }
                        }
                        false
                    }
                    crate::parser::ast::Stmt::ExprStmt(e) => check_expr(e, target_line, target_col),
                    _ => false,
                }
            }

            for stmt in stmts {
                if check_stmt(stmt, target_line, target_col) {
                    return true;
                }
            }
            false
        }

        if is_pos_in_jsx_text(&parsed_stmts, l, c) {
            hover = None;
        }
    }

    if let Some(h) = &mut hover {
        if let Some(d) = &mut h.documentation {
            *d = clean_table_borders(d);
        }
    }

    let tokens = ide::get_semantic_tokens(&content);

    let mut unique_completions = Vec::new();
    let mut seen_labels = std::collections::HashSet::new();
    for mut c in completions {
        if let Some(doc) = &mut c.documentation {
            *doc = clean_table_borders(doc);
        }
        if seen_labels.insert(c.label.clone()) {
            unique_completions.push(c);
        }
    }

    let definition = if let (Some(l), Some(c)) = (line, col) {
        ide::find_definition(file, &content, l, c, &parsed_stmts, &manifest_dir)
    } else {
        None
    };

    JsonCheckOutput {
        file: file.to_string(),
        diagnostics,
        std_modules,
        native_modules,
        plugins,
        completions: unique_completions,
        hover,
        signature_help,
        tokens,
        definition,
    }
}

fn list_std_modules(_manifest_dir: &Path) -> Vec<String> {
    vec![
        "thread".to_string(),
        "process".to_string(),
        "fs".to_string(),
        "byte".to_string(),
        "net".to_string(),
        "json".to_string(),
        "math".to_string(),
        "time".to_string(),
        "fmt".to_string(),
        "os".to_string(),
        "window".to_string(),
        "desktop".to_string(),
        "env".to_string(),
        "camera".to_string(),
        "unit".to_string(),
    ]
}

fn extract_member_context(line: &str, col: usize) -> (Option<String>, Option<String>) {
    let end = col.min(line.len());
    let upto = line.chars().take(end).collect::<String>();

    // If the character just after the extracted part is a dot (meaning the cursor is exactly ON the dot),
    // we should include it to properly trigger member completions.
    let upto = if end < line.len() && line[end..].starts_with('.') {
        line.chars().take(end + 1).collect::<String>()
    } else {
        upto
    };

    if let Some(dot_index) = upto.rfind('.') {
        let after_dot = &upto[dot_index + 1..];
        if !after_dot
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return (None, None);
        }
        let left = upto[..dot_index].trim();
        let right = after_dot.to_string();
        return (
            left.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                .filter(|s| !s.is_empty())
                .last()
                .map(|value| value.to_string()),
            Some(right),
        );
    }
    (None, None)
}

fn extract_word_at_cursor(line: &str, col: usize) -> String {
    if line.is_empty() {
        return String::new();
    }
    let col = col.min(line.len() + 1);
    let mut start = col.saturating_sub(1);
    let chars: Vec<char> = line.chars().collect();

    // Scan backwards
    while start > 0 {
        if let Some(&ch) = chars.get(start - 1) {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '@' || ch == '"' || ch == '-' {
                start -= 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Scan forwards
    let mut end = col.saturating_sub(1);
    while end < chars.len() {
        if let Some(&ch) = chars.get(end) {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '@' || ch == '"' || ch == '-' {
                end += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if start < end {
        chars[start..end].iter().collect()
    } else {
        String::new()
    }
}

fn load_local_module_declarations(
    manifest_dir: &Path,
    current_file: &str,
    namespace: &str,
) -> Option<Vec<crate::parser::Stmt>> {
    let path_parts = namespace
        .split('.')
        .map(|part| part.to_string())
        .collect::<Vec<_>>();
    let candidate = crate::stdlib::locate_import_file(Path::new(current_file), &path_parts)
        .or_else(|| {
            let direct = manifest_dir.join(format!("{}.fm", namespace));
            if direct.exists() {
                return Some(direct);
            }

            let pkg_main = manifest_dir
                .join(".flame")
                .join("pkg")
                .join(namespace)
                .join("src")
                .join("main.fm");
            if pkg_main.exists() {
                return Some(pkg_main);
            }

            None
        })?;

    let mut paths_to_read = Vec::new();
    if candidate.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&candidate) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("fm") {
                    paths_to_read.push(p);
                }
            }
        }
    } else {
        paths_to_read.push(candidate);
    }

    let mut exported_stmts = Vec::new();
    for path in paths_to_read {
        if let Ok(content) = fs::read_to_string(&path) {
            let mut lexer = Lexer::new(&content);
            let mut tokens = Vec::new();
            loop {
                let tok = lexer.next_token();
                let is_eof = tok.kind == crate::lexer::TokenKind::EOF;
                tokens.push(tok);
                if is_eof {
                    break;
                }
            }
            let mut parser = Parser::new(tokens, path.to_string_lossy().to_string());
            if let Ok(stmts) = parser.parse() {
                for stmt in stmts {
                    if let crate::parser::Stmt::ExportDecl(inner, _) = stmt {
                        exported_stmts.push(*inner);
                    }
                }
            }
        }
    }

    if exported_stmts.is_empty() {
        None
    } else {
        Some(exported_stmts)
    }
}

fn load_imported_module_declarations(
    _manifest_dir: &Path,
    current_file: &str,
) -> Vec<crate::parser::Stmt> {
    let mut results = Vec::new();
    let content = fs::read_to_string(current_file).unwrap_or_default();
    let import_re = Regex::new(r"(?m)^import\s+([a-zA-Z_][\w]*(?:\.[a-zA-Z_][\w]*)*)").unwrap();

    for cap in import_re.captures_iter(&content) {
        let module_path = cap[1].to_string();
        if module_path == "native" || module_path.starts_with("native.") {
            continue;
        }

        let path_parts: Vec<String> = module_path.split('.').map(|s| s.to_string()).collect();

        let std_source = if module_path == "std" || module_path.starts_with("std.") {
            let mut found = None;
            for part in path_parts.iter().rev() {
                let candidate = format!("{}.fm", part);
                if let Some((_, src)) = crate::blaze::EMBEDDED_BLAZE_STD
                    .iter()
                    .find(|(name, _)| *name == candidate)
                {
                    found = Some((*src).to_string());
                    break;
                }
            }
            found
        } else {
            None
        };

        if let Some(src) = std_source {
            let mut lexer = Lexer::new(&src);
            let mut tokens = Vec::new();
            loop {
                let tok = lexer.next_token();
                let is_eof = tok.kind == crate::lexer::TokenKind::EOF;
                tokens.push(tok);
                if is_eof {
                    break;
                }
            }
            let mut parser = Parser::new(tokens, format!("<std::{}>", module_path));
            if let Ok(parsed_stmts) = parser.parse() {
                for stmt in parsed_stmts {
                    if let crate::parser::Stmt::ExportDecl(inner, _) = &stmt {
                        results.push((**inner).clone());
                    } else if let crate::parser::Stmt::PackageDecl { .. } = &stmt {
                        results.push(stmt.clone());
                    } else if let crate::parser::Stmt::ImplDecl { .. } = &stmt {
                        results.push(stmt.clone());
                    } else if let crate::parser::Stmt::StructDecl { .. } = &stmt {
                        results.push(stmt.clone());
                    } else if let crate::parser::Stmt::EnumDecl { .. } = &stmt {
                        results.push(stmt.clone());
                    }
                }
            }
            continue;
        }

        let file_path = crate::stdlib::locate_import_file(Path::new(current_file), &path_parts)
            .or_else(|| {
                let pkg_main = _manifest_dir
                    .join(".flame")
                    .join("pkg")
                    .join(&module_path)
                    .join("src")
                    .join("main.fm");
                if pkg_main.exists() {
                    return Some(pkg_main);
                }
                None
            });

        if let Some(file_path) = file_path {
            let mut paths_to_read = Vec::new();
            if file_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&file_path) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("fm") {
                            paths_to_read.push(p);
                        }
                    }
                }
            } else {
                paths_to_read.push(file_path);
            }

            for path in paths_to_read {
                if let Ok(module_content) = fs::read_to_string(&path) {
                    let mut lexer = Lexer::new(&module_content);
                    let mut tokens = Vec::new();
                    loop {
                        let tok = lexer.next_token();
                        let is_eof = tok.kind == crate::lexer::TokenKind::EOF;
                        tokens.push(tok);
                        if is_eof {
                            break;
                        }
                    }
                    let mut parser = Parser::new(tokens, path.to_string_lossy().to_string());
                    if let Ok(parsed_stmts) = parser.parse() {
                        for stmt in parsed_stmts {
                            if let crate::parser::Stmt::ExportDecl(inner, _) = &stmt {
                                results.push((**inner).clone());
                            } else if let crate::parser::Stmt::PackageDecl { .. } = &stmt {
                                results.push(stmt.clone());
                            } else if let crate::parser::Stmt::ImplDecl { .. } = &stmt {
                                results.push(stmt.clone());
                            } else if let crate::parser::Stmt::StructDecl { .. } = &stmt {
                                results.push(stmt.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    results
}

fn load_meta_from_project(
    manifest_dir: &Path,
    module_name: &str,
) -> Option<package_manager::FlameMeta> {
    let mut meta_path = manifest_dir
        .join(".flame")
        .join("pkg")
        .join(module_name)
        .join(format!("{}.fmi", module_name));
    if !meta_path.exists() {
        meta_path = manifest_dir
            .join(".flame")
            .join("pkg")
            .join("native")
            .join(format!("{}.fmi", module_name));
    }
    let meta_str = fs::read_to_string(meta_path).ok()?;
    serde_json::from_str::<package_manager::FlameMeta>(&meta_str).ok()
}

fn load_local_rust_doc(manifest_dir: &Path, module_name: &str, member: &str) -> Option<String> {
    let candidate_paths = [
        manifest_dir.join(module_name).join("src").join("lib.rs"),
        manifest_dir
            .join("native")
            .join(module_name)
            .join("src")
            .join("lib.rs"),
    ];

    for candidate in candidate_paths {
        let source = match fs::read_to_string(&candidate) {
            Ok(source) => source,
            Err(_) => continue,
        };
        let lines = source.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if line.contains(&format!("fn {}", member)) {
                let mut docs = Vec::new();
                for doc_index in (0..index).rev() {
                    let trimmed = lines[doc_index].trim();
                    if trimmed.starts_with("///") {
                        docs.push(trimmed.trim_start_matches("///").trim().to_string());
                    } else if trimmed.is_empty() {
                        if docs.is_empty() {
                            continue;
                        }
                        break;
                    } else {
                        break;
                    }
                }
                docs.reverse();
                if !docs.is_empty() {
                    return Some(docs.join("\n"));
                }
            }
        }
    }

    None
}
