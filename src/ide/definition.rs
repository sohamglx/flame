use super::scanner::scan_document;
use crate::utils::text::extract_balanced_block;

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct JsonDefinition {
    pub file: String,
    pub line: usize,
    pub column: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
}

pub fn locate_blaze_dir() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;

    // 1. Environment variables
    if let Ok(env_path) = std::env::var("FLAME_BLAZE_DIR") {
        let p = PathBuf::from(env_path);
        if p.join("std").exists() {
            return Some(p);
        }
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(env_path) = std::env::var("BLAZE_HOME") {
        let p = PathBuf::from(env_path);
        if p.join("std").exists() {
            return Some(p);
        }
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(env_path) = std::env::var("FLAME_HOME") {
        let p = PathBuf::from(env_path).join("Blaze");
        if p.join("std").exists() {
            return Some(p);
        }
    }

    // 2. Current working directory and parent hierarchies (development / repo root)
    if let Ok(cwd) = std::env::current_dir() {
        let mut curr = Some(cwd.as_path());
        while let Some(dir) = curr {
            let blaze = dir.join("Blaze");
            if blaze.join("std").exists() {
                return Some(blaze);
            }
            let blaze_lower = dir.join("blaze");
            if blaze_lower.join("std").exists() {
                return Some(blaze_lower);
            }
            curr = dir.parent();
        }
    }

    // 3. Executable-relative
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidates = [
                exe_dir.join("Blaze"),
                exe_dir.join("blaze"),
                exe_dir.join("..").join("Blaze"),
                exe_dir.join("..").join("blaze"),
                exe_dir.join("..").join("share").join("blaze"),
                exe_dir.join("..").join("lib").join("blaze"),
            ];
            for cand in candidates {
                if cand.join("std").exists() {
                    return Some(cand);
                }
            }
        }
    }

    // 4. User and platform-standard directories
    #[cfg(target_os = "windows")]
    {
        if let Ok(prog_files) = std::env::var("ProgramFiles") {
            let p = PathBuf::from(prog_files).join("Blaze");
            if p.join("std").exists() {
                return Some(p);
            }
        }
        if let Ok(prog_files_x86) = std::env::var("ProgramFiles(x86)") {
            let p = PathBuf::from(prog_files_x86).join("Blaze");
            if p.join("std").exists() {
                return Some(p);
            }
        }
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let p = PathBuf::from(local_app_data).join("Blaze");
            if p.join("std").exists() {
                return Some(p);
            }
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            let p = PathBuf::from(user_profile).join(".blaze");
            if p.join("std").exists() {
                return Some(p);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join(".blaze");
            if p.join("std").exists() {
                return Some(p);
            }
        }
        let sys_candidates = [
            PathBuf::from("/usr/local/share/blaze"),
            PathBuf::from("/usr/share/blaze"),
            PathBuf::from("/opt/blaze"),
        ];
        for p in sys_candidates {
            if p.join("std").exists() {
                return Some(p);
            }
        }
    }

    // 5. Automatic self-healing: if no definition directory exists yet, populate default
    crate::blaze::ensure_blaze_definitions_installed()
}

pub fn find_declaration_in_stmts(
    stmts: &[crate::parser::Stmt],
    symbol: &str,
    filepath: &str,
) -> Option<JsonDefinition> {
    let clean = symbol.trim_start_matches('@');
    for stmt in stmts {
        match stmt {
            crate::parser::Stmt::FuncDecl {
                name, name_span, ..
            } if name == clean => {
                return Some(JsonDefinition {
                    file: filepath.to_string(),
                    line: name_span.line,
                    column: name_span.col,
                    end_line: Some(name_span.line),
                    end_column: Some(name_span.col + name.len()),
                });
            }
            crate::parser::Stmt::AnnotationDecl {
                name, name_span, ..
            } if name == clean => {
                return Some(JsonDefinition {
                    file: filepath.to_string(),
                    line: name_span.line,
                    column: name_span.col,
                    end_line: Some(name_span.line),
                    end_column: Some(name_span.col + name.len()),
                });
            }
            crate::parser::Stmt::StructDecl {
                name, name_span, ..
            } if name == clean => {
                return Some(JsonDefinition {
                    file: filepath.to_string(),
                    line: name_span.line,
                    column: name_span.col,
                    end_line: Some(name_span.line),
                    end_column: Some(name_span.col + name.len()),
                });
            }
            crate::parser::Stmt::EnumDecl {
                name, name_span, ..
            } if name == clean => {
                return Some(JsonDefinition {
                    file: filepath.to_string(),
                    line: name_span.line,
                    column: name_span.col,
                    end_line: Some(name_span.line),
                    end_column: Some(name_span.col + name.len()),
                });
            }
            crate::parser::Stmt::ConstDecl {
                name, name_span, ..
            } if name == clean => {
                return Some(JsonDefinition {
                    file: filepath.to_string(),
                    line: name_span.line,
                    column: name_span.col,
                    end_line: Some(name_span.line),
                    end_column: Some(name_span.col + name.len()),
                });
            }
            crate::parser::Stmt::LetDecl {
                name, name_span, ..
            } if name == clean => {
                return Some(JsonDefinition {
                    file: filepath.to_string(),
                    line: name_span.line,
                    column: name_span.col,
                    end_line: Some(name_span.line),
                    end_column: Some(name_span.col + name.len()),
                });
            }
            crate::parser::Stmt::ExportDecl(inner, _) => {
                if let Some(def) = find_declaration_in_stmts(&[(**inner).clone()], symbol, filepath)
                {
                    return Some(def);
                }
            }
            crate::parser::Stmt::ImplDecl { methods, .. } => {
                for m in methods {
                    if let crate::parser::Stmt::FuncDecl {
                        name, name_span, ..
                    } = m
                    {
                        if name == clean {
                            return Some(JsonDefinition {
                                file: filepath.to_string(),
                                line: name_span.line,
                                column: name_span.col,
                                end_line: Some(name_span.line),
                                end_column: Some(name_span.col + name.len()),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn search_body_for_local_symbol(
    body: &[crate::parser::Stmt],
    clean: &str,
    cursor_line: usize,
    filepath: &str,
) -> Option<JsonDefinition> {
    for b_stmt in body {
        match b_stmt {
            crate::parser::Stmt::LetDecl {
                name, name_span, ..
            } => {
                if name == clean && name_span.line <= cursor_line {
                    return Some(JsonDefinition {
                        file: filepath.to_string(),
                        line: name_span.line,
                        column: name_span.col,
                        end_line: Some(name_span.line),
                        end_column: Some(name_span.col + name.len()),
                    });
                }
            }
            crate::parser::Stmt::ConstDecl {
                name, name_span, ..
            } => {
                if name == clean {
                    return Some(JsonDefinition {
                        file: filepath.to_string(),
                        line: name_span.line,
                        column: name_span.col,
                        end_line: Some(name_span.line),
                        end_column: Some(name_span.col + name.len()),
                    });
                }
            }
            crate::parser::Stmt::FuncDecl {
                name, name_span, ..
            } => {
                if name == clean {
                    return Some(JsonDefinition {
                        file: filepath.to_string(),
                        line: name_span.line,
                        column: name_span.col,
                        end_line: Some(name_span.line),
                        end_column: Some(name_span.col + name.len()),
                    });
                }
            }
            _ => {}
        }
    }
    None
}

pub fn find_local_symbol(
    stmts: &[crate::parser::Stmt],
    symbol: &str,
    filepath: &str,
    cursor_line: usize,
) -> Option<JsonDefinition> {
    let clean = symbol.trim_start_matches('@');

    // 1. Check enclosing function / impl method parameters and local declarations
    for stmt in stmts {
        if let crate::parser::Stmt::FuncDecl {
            params, body, span, ..
        } = stmt
        {
            if cursor_line >= span.line {
                for p in params {
                    if p.name == clean {
                        return Some(JsonDefinition {
                            file: filepath.to_string(),
                            line: span.line,
                            column: span.col,
                            end_line: Some(span.line),
                            end_column: Some(span.col + p.name.len()),
                        });
                    }
                }
                if let Some(inner_body) = body {
                    if let Some(def) =
                        search_body_for_local_symbol(inner_body, clean, cursor_line, filepath)
                    {
                        return Some(def);
                    }
                }
            }
        } else if let crate::parser::Stmt::ImplDecl { methods, .. } = stmt {
            for m in methods {
                if let crate::parser::Stmt::FuncDecl {
                    params, body, span, ..
                } = m
                {
                    if cursor_line >= span.line {
                        for p in params {
                            if p.name == clean {
                                return Some(JsonDefinition {
                                    file: filepath.to_string(),
                                    line: span.line,
                                    column: span.col,
                                    end_line: Some(span.line),
                                    end_column: Some(span.col + p.name.len()),
                                });
                            }
                        }
                        if let Some(inner_body) = body {
                            if let Some(def) = search_body_for_local_symbol(
                                inner_body,
                                clean,
                                cursor_line,
                                filepath,
                            ) {
                                return Some(def);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Top-level declarations in current file
    find_declaration_in_stmts(stmts, symbol, filepath)
}

pub fn find_symbol_in_file(path: &std::path::Path, symbol: &str) -> Option<JsonDefinition> {
    let content = std::fs::read_to_string(path).ok()?;
    let path_str = path.to_string_lossy().to_string();

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
    let mut parser = crate::parser::Parser::new(tokens, path_str.clone());
    if let Ok(stmts) = parser.parse() {
        if let Some(def) = find_declaration_in_stmts(&stmts, symbol, &path_str) {
            return Some(def);
        }
    }

    // Fast fallback line-by-line pattern matching
    let clean = symbol.trim_start_matches('@');
    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        let is_match = trimmed.starts_with(&format!("fn {}", clean))
            || trimmed.starts_with(&format!("export fn {}", clean))
            || trimmed.starts_with(&format!("annotation {}", clean))
            || trimmed.starts_with(&format!("export annotation {}", clean))
            || trimmed.starts_with(&format!("struct {}", clean))
            || trimmed.starts_with(&format!("export struct {}", clean))
            || trimmed.starts_with(&format!("enum {}", clean))
            || trimmed.starts_with(&format!("export enum {}", clean))
            || trimmed.starts_with(&format!("const {}:", clean))
            || trimmed.starts_with(&format!("export const {}:", clean))
            || trimmed.starts_with(&format!("let {}", clean))
            || trimmed.starts_with(&format!("export let {}", clean))
            || trimmed.contains(&format!("fn {}(", clean));

        if is_match {
            if let Some(col_idx) = line.find(clean) {
                return Some(JsonDefinition {
                    file: path_str,
                    line: line_idx + 1,
                    column: col_idx + 1,
                    end_line: Some(line_idx + 1),
                    end_column: Some(col_idx + 1 + clean.len()),
                });
            }
        }
    }

    None
}

pub fn find_symbol_in_rust_file(
    path: &std::path::Path,
    struct_name: Option<&str>,
    symbol: &str,
) -> Option<JsonDefinition> {
    let content = std::fs::read_to_string(path).ok()?;
    let path_str = path.to_string_lossy().to_string();

    // 1. If struct_name is provided, search inside `impl ... StructName` block
    if let Some(target_struct) = struct_name {
        let impl_pattern = format!(
            r"impl(?:\s*<[^>]+>)?\s+(?:[a-zA-Z_]\w*::)*{}\b",
            regex::escape(target_struct)
        );
        if let Ok(re_impl) = regex::Regex::new(&impl_pattern) {
            for m in re_impl.find_iter(&content) {
                if let Some(brace_offset) = content[m.end()..].find('{') {
                    let open_brace = m.end() + brace_offset;
                    if let Some(impl_body) = extract_balanced_block(&content, open_brace) {
                        let base_offset = open_brace + 1;
                        let fn_pattern = format!(
                            r"(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+{}\b",
                            regex::escape(symbol)
                        );
                        if let Ok(re_fn) = regex::Regex::new(&fn_pattern) {
                            if let Some(fn_match) = re_fn.find(impl_body) {
                                let sub = &impl_body[fn_match.start()..fn_match.end()];
                                let sym_offset = sub.find(symbol).unwrap_or(0);
                                let abs_offset = base_offset + fn_match.start() + sym_offset;
                                let line_idx =
                                    content[..abs_offset].chars().filter(|&c| c == '\n').count()
                                        + 1;
                                let last_line_start = content[..abs_offset]
                                    .rfind('\n')
                                    .map(|i| i + 1)
                                    .unwrap_or(0);
                                let col_idx =
                                    content[last_line_start..abs_offset].chars().count() + 1;
                                return Some(JsonDefinition {
                                    file: path_str.clone(),
                                    line: line_idx,
                                    column: col_idx,
                                    end_line: Some(line_idx),
                                    end_column: Some(col_idx + symbol.len()),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Search for fn symbol (free function or method in file)
    let fn_pattern = format!(
        r"(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+{}\b",
        regex::escape(symbol)
    );
    if let Ok(re_fn) = regex::Regex::new(&fn_pattern) {
        if let Some(fn_match) = re_fn.find(&content) {
            let sub = &content[fn_match.start()..fn_match.end()];
            let sym_offset = sub.find(symbol).unwrap_or(0);
            let abs_offset = fn_match.start() + sym_offset;
            let line_idx = content[..abs_offset].chars().filter(|&c| c == '\n').count() + 1;
            let last_line_start = content[..abs_offset]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let col_idx = content[last_line_start..abs_offset].chars().count() + 1;
            return Some(JsonDefinition {
                file: path_str.clone(),
                line: line_idx,
                column: col_idx,
                end_line: Some(line_idx),
                end_column: Some(col_idx + symbol.len()),
            });
        }
    }

    // 3. Search for struct or enum: pub struct <symbol>, pub enum <symbol>
    let type_pattern = format!(
        r"(?:pub(?:\s*\([^)]*\))?\s+)?(?:struct|enum)\s+{}\b",
        regex::escape(symbol)
    );
    if let Ok(re_type) = regex::Regex::new(&type_pattern) {
        if let Some(type_match) = re_type.find(&content) {
            let sub = &content[type_match.start()..type_match.end()];
            let sym_offset = sub.find(symbol).unwrap_or(0);
            let abs_offset = type_match.start() + sym_offset;
            let line_idx = content[..abs_offset].chars().filter(|&c| c == '\n').count() + 1;
            let last_line_start = content[..abs_offset]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let col_idx = content[last_line_start..abs_offset].chars().count() + 1;
            return Some(JsonDefinition {
                file: path_str,
                line: line_idx,
                column: col_idx,
                end_line: Some(line_idx),
                end_column: Some(col_idx + symbol.len()),
            });
        }
    }

    None
}

pub fn get_plugin_func_return_type(
    manifest_dir: &std::path::Path,
    plugin_name: &str,
    method_name: &str,
) -> Option<String> {
    // 1. Check .flame/pkg/<plugin_name>/<plugin_name>.fmi
    let fmi_path = manifest_dir
        .join(".flame")
        .join("pkg")
        .join(plugin_name)
        .join(format!("{}.fmi", plugin_name));
    if let Ok(content) = std::fs::read_to_string(&fmi_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(funcs) = json.get("functions").and_then(|f| f.as_array()) {
                for f in funcs {
                    if let (Some(name), Some(ret)) = (
                        f.get("flame_name")
                            .and_then(|n| n.as_str())
                            .or_else(|| f.get("name").and_then(|n| n.as_str())),
                        f.get("return_type").and_then(|r| r.as_str()),
                    ) {
                        if name == method_name {
                            return Some(ret.to_string());
                        }
                    }
                }
            }
        }
    }

    // 2. Check plugin source Rust files
    let toml_path = manifest_dir.join("flame.toml");
    if let Ok(content) = std::fs::read_to_string(&toml_path) {
        let entries = crate::package_manager::parse_section_entries(&content, "[plugins]");
        for (name, path_str) in entries {
            if name == plugin_name {
                let clean_path = path_str.trim().trim_matches('"').trim_matches('\'');
                let resolved = if clean_path.starts_with('.')
                    || clean_path.starts_with('/')
                    || clean_path.contains('\\')
                {
                    manifest_dir.join(clean_path)
                } else {
                    manifest_dir.join(".flame").join("pkg").join(&name)
                };
                let lib_rs = resolved.join("src").join("lib.rs");
                if let Ok(rs_content) = std::fs::read_to_string(&lib_rs) {
                    let fn_re = regex::Regex::new(&format!(
                        r"(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+{}\s*[^{{]*->\s*([a-zA-Z_]\w*)",
                        regex::escape(method_name)
                    ));
                    if let Ok(re) = fn_re {
                        if let Some(cap) = re.captures(&rs_content) {
                            return Some(cap[1].to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

pub fn find_native_plugin_symbol(
    manifest_dir: &std::path::Path,
    plugin_name_or_alias: &str,
    struct_name: Option<&str>,
    symbol: &str,
) -> Option<JsonDefinition> {
    // 1. Locate plugin directory from flame.toml
    let toml_path = manifest_dir.join("flame.toml");
    let mut plugin_dir = None;
    if let Ok(content) = std::fs::read_to_string(&toml_path) {
        let entries = crate::package_manager::parse_section_entries(&content, "[plugins]");
        for (name, path_str) in entries {
            if name == plugin_name_or_alias {
                let clean_path = path_str.trim().trim_matches('"').trim_matches('\'');
                let resolved = if clean_path.starts_with('.')
                    || clean_path.starts_with('/')
                    || clean_path.contains('\\')
                {
                    manifest_dir.join(clean_path)
                } else {
                    manifest_dir.join(".flame").join("pkg").join(&name)
                };
                plugin_dir = Some(resolved);
                break;
            }
        }
    }

    if plugin_dir.is_none() {
        // Check .flame/pkg/<plugin_name_or_alias>
        let candidate = manifest_dir
            .join(".flame")
            .join("pkg")
            .join(plugin_name_or_alias);
        if candidate.exists() {
            plugin_dir = Some(candidate);
        }
    }

    if let Some(dir) = plugin_dir {
        let src_dir = dir.join("src");
        let lib_rs = src_dir.join("lib.rs");
        if lib_rs.is_file() {
            if let Some(def) = find_symbol_in_rust_file(&lib_rs, struct_name, symbol) {
                return Some(def);
            }
        }
        let main_rs = src_dir.join("main.rs");
        if main_rs.is_file() {
            if let Some(def) = find_symbol_in_rust_file(&main_rs, struct_name, symbol) {
                return Some(def);
            }
        }
        if let Ok(entries) = std::fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if p != lib_rs && p != main_rs {
                        if let Some(def) = find_symbol_in_rust_file(&p, struct_name, symbol) {
                            return Some(def);
                        }
                    }
                }
            }
        }

        // If symbol wasn't found in Rust source, check .fmi file if present
        let fmi_path = manifest_dir
            .join(".flame")
            .join("pkg")
            .join(plugin_name_or_alias)
            .join(format!("{}.fmi", plugin_name_or_alias));
        if fmi_path.is_file() {
            if let Some(def) = find_symbol_in_file(&fmi_path, symbol) {
                return Some(def);
            }
        }

        // If still looking for the plugin itself (e.g. symbol == plugin_name)
        if symbol == plugin_name_or_alias {
            if lib_rs.is_file() {
                return Some(JsonDefinition {
                    file: lib_rs.to_string_lossy().to_string(),
                    line: 1,
                    column: 1,
                    end_line: Some(1),
                    end_column: Some(1),
                });
            }
        }
    }

    None
}

pub fn infer_receiver_type(
    stmts: &[crate::parser::Stmt],
    content: &str,
    var_name: &str,
    manifest_dir: &std::path::Path,
) -> (Option<String>, Option<String>) {
    // 1. First check AST for LetDecl or ConstDecl matching var_name
    let mut ast_found = None;
    for stmt in stmts {
        match stmt {
            crate::parser::Stmt::LetDecl {
                name,
                type_ann,
                value,
                ..
            } if name == var_name => {
                ast_found = Some((type_ann.clone(), value.clone()));
                break;
            }
            crate::parser::Stmt::FuncDecl {
                body: Some(body), ..
            } => {
                for b in body {
                    if let crate::parser::Stmt::LetDecl {
                        name,
                        type_ann,
                        value,
                        ..
                    } = b
                    {
                        if name == var_name {
                            ast_found = Some((type_ann.clone(), value.clone()));
                            break;
                        }
                    }
                }
                if ast_found.is_some() {
                    break;
                }
            }
            _ => {}
        }
    }

    if let Some((type_ann, value)) = ast_found {
        if let Some(ann) = type_ann {
            let clean_ann = ann.split('.').last().unwrap_or(&ann).to_string();
            return (Some(clean_ann), None);
        }

        let mut curr_expr = &value;
        while let crate::parser::Expr::Await(inner, _) | crate::parser::Expr::Borrow(inner, _, _) =
            curr_expr
        {
            curr_expr = inner;
        }

        if let crate::parser::Expr::Call(callee, _, _) = curr_expr {
            if let crate::parser::Expr::Dot(receiver, method_name, _) = callee.as_ref() {
                if let crate::parser::Expr::Identifier(plugin_candidate, _) = receiver.as_ref() {
                    if let Some(ret_type) =
                        get_plugin_func_return_type(manifest_dir, plugin_candidate, method_name)
                    {
                        return (Some(ret_type), Some(plugin_candidate.clone()));
                    }
                }
            }
        }
    }

    // 2. Fallback regex on content
    let re = regex::Regex::new(&format!(
        r"(?:let|const)(?:\s+mut)?\s+{}\s*(?::\s*([a-zA-Z_]\w*))?\s*=\s*(?:await\s+)?([a-zA-Z_]\w*)\.([a-zA-Z_]\w*)",
        regex::escape(var_name)
    ));
    if let Ok(re) = re {
        if let Some(cap) = re.captures(content) {
            if let Some(ann) = cap.get(1) {
                return (Some(ann.as_str().to_string()), None);
            }
            let receiver = cap[2].to_string();
            let method = cap[3].to_string();
            if let Some(ret_type) = get_plugin_func_return_type(manifest_dir, &receiver, &method) {
                return (Some(ret_type), Some(receiver));
            }
        }
    }

    (None, None)
}

pub fn find_definition(
    file: &str,
    content: &str,
    line: usize,
    col: usize,
    stmts: &[crate::parser::Stmt],
    manifest_dir: &std::path::Path,
) -> Option<JsonDefinition> {
    let line_str = content.lines().nth(line.saturating_sub(1)).unwrap_or("");
    let col = col.min(line_str.len() + 1);

    // Check if cursor is on an import line
    let trimmed_line = line_str.trim();
    if trimmed_line.starts_with("import ") {
        let import_target = trimmed_line.trim_start_matches("import ").trim();
        let clean_import = import_target
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches(';');

        if clean_import.starts_with("std.") || clean_import == "std" {
            let mod_name = clean_import.trim_start_matches("std.");
            if let Some(blaze) = locate_blaze_dir() {
                let std_file = blaze.join("std").join(format!("{}.fm", mod_name));
                if std_file.is_file() {
                    return Some(JsonDefinition {
                        file: std_file.to_string_lossy().to_string(),
                        line: 1,
                        column: 1,
                        end_line: Some(1),
                        end_column: Some(1),
                    });
                }
            }
        } else if clean_import.starts_with("native.") {
            let plugin_name = clean_import.trim_start_matches("native.");
            if let Some(def) =
                find_native_plugin_symbol(manifest_dir, plugin_name, None, plugin_name)
            {
                return Some(def);
            }
        } else {
            // Workspace or package import
            let path_parts: Vec<String> = clean_import.split('.').map(String::from).collect();
            if let Some(found_file) =
                crate::stdlib::locate_import_file(std::path::Path::new(file), &path_parts)
            {
                let target_path = if found_file.is_dir() {
                    if found_file.join("src").join("main.fm").is_file() {
                        found_file.join("src").join("main.fm")
                    } else if found_file.join("main.fm").is_file() {
                        found_file.join("main.fm")
                    } else if found_file.join("src").join("lib.rs").is_file() {
                        found_file.join("src").join("lib.rs")
                    } else if found_file.join("flame.toml").is_file() {
                        found_file.join("flame.toml")
                    } else {
                        found_file
                    }
                } else {
                    found_file
                };
                if target_path.is_file() {
                    return Some(JsonDefinition {
                        file: target_path.to_string_lossy().to_string(),
                        line: 1,
                        column: 1,
                        end_line: Some(1),
                        end_column: Some(1),
                    });
                }
            }
        }
    }

    // Extract word under cursor and dot context
    let mut start = col.saturating_sub(1);
    let chars: Vec<char> = line_str.chars().collect();

    while start > 0 {
        if let Some(&ch) = chars.get(start - 1) {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '@' {
                start -= 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let mut end = col.saturating_sub(1);
    while end < chars.len() {
        if let Some(&ch) = chars.get(end) {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '@' {
                end += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let word = if start < end {
        chars[start..end].iter().collect::<String>()
    } else {
        String::new()
    };

    if word.is_empty() {
        return None;
    }

    // Check for member access prefix (e.g. math.sqrt or helper.do_work)
    let mut namespace = None;
    if start > 0 && chars.get(start - 1) == Some(&'.') {
        let mut ns_start = start - 1;
        while ns_start > 0 {
            if let Some(&ch) = chars.get(ns_start - 1) {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    ns_start -= 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if ns_start < start - 1 {
            namespace = Some(chars[ns_start..start - 1].iter().collect::<String>());
        }
    }

    let clean_word = word.trim_start_matches('@');

    // 1. If no namespace prefix, check local symbols in the same file first
    if namespace.is_none() {
        if let Some(local_def) = find_local_symbol(stmts, &word, file, line) {
            return Some(local_def);
        }
        // Check if word matches a native plugin directly (e.g. server.init)
        if let Some(def) = find_native_plugin_symbol(manifest_dir, clean_word, None, clean_word) {
            return Some(def);
        }
    }

    // 2. If namespace prefix is present (e.g. math.sin or server.init or app.get)
    if let Some(ref ns) = namespace {
        // 2a. Standard library module
        let std_names = [
            "os", "fmt", "fs", "byte", "net", "thread", "time", "process", "json", "math", "unit",
            "window", "desktop", "env", "camera", "web", "annotation"
        ];
        if std_names.contains(&ns.as_str()) {
            if let Some(blaze) = locate_blaze_dir() {
                let std_file = blaze.join("std").join(format!("{}.fm", ns));
                if std_file.is_file() {
                    if let Some(def) = find_symbol_in_file(&std_file, clean_word) {
                        return Some(def);
                    }
                    return Some(JsonDefinition {
                        file: std_file.to_string_lossy().to_string(),
                        line: 1,
                        column: 1,
                        end_line: Some(1),
                        end_column: Some(1),
                    });
                }
            }
        }

        // 2b. Direct native plugin member (e.g. server.init)
        if let Some(def) = find_native_plugin_symbol(manifest_dir, ns, None, clean_word) {
            return Some(def);
        }

        // 2c. Receiver method call on a variable (e.g. app.get or app.post)
        let (inferred_struct, inferred_plugin) =
            infer_receiver_type(stmts, content, ns, manifest_dir);
        if let Some(ref st) = inferred_struct {
            if let Some(ref pl) = inferred_plugin {
                if let Some(def) = find_native_plugin_symbol(manifest_dir, pl, Some(st), clean_word)
                {
                    return Some(def);
                }
            } else {
                let toml_path = manifest_dir.join("flame.toml");
                if let Ok(m_content) = std::fs::read_to_string(&toml_path) {
                    for (pl_name, _) in
                        crate::package_manager::parse_section_entries(&m_content, "[plugins]")
                    {
                        if let Some(def) =
                            find_native_plugin_symbol(manifest_dir, &pl_name, Some(st), clean_word)
                        {
                            return Some(def);
                        }
                    }
                }
            }
        }

        // 2d. Workspace module import
        let path_parts = vec![ns.clone()];
        if let Some(found_file) =
            crate::stdlib::locate_import_file(std::path::Path::new(file), &path_parts)
        {
            let target_path = if found_file.is_dir() {
                if found_file.join("src").join("main.fm").is_file() {
                    found_file.join("src").join("main.fm")
                } else if found_file.join("main.fm").is_file() {
                    found_file.join("main.fm")
                } else if found_file.join("src").join("lib.rs").is_file() {
                    found_file.join("src").join("lib.rs")
                } else if found_file.join("flame.toml").is_file() {
                    found_file.join("flame.toml")
                } else {
                    found_file
                }
            } else {
                found_file
            };
            if target_path.is_file() {
                if let Some(def) = find_symbol_in_file(&target_path, clean_word) {
                    return Some(def);
                }
                return Some(JsonDefinition {
                    file: target_path.to_string_lossy().to_string(),
                    line: 1,
                    column: 1,
                    end_line: Some(1),
                    end_column: Some(1),
                });
            }
        }

        // 2e. Check local struct methods matching namespace as variable
        let (scanned_vars, scanned_structs) = scan_document(content);
        if let Some(var) = scanned_vars.iter().find(|v| v.name == *ns) {
            if let Some(ref typ_name) = var.typ {
                if let Some(st) = scanned_structs.iter().find(|s| s.name == *typ_name) {
                    if st.methods.contains(&clean_word.to_string()) {
                        if let Some(def) = find_local_symbol(stmts, clean_word, file, line) {
                            return Some(def);
                        }
                    }
                }
            }
        }
    }

    // 3. Check imported modules (direct function / struct call from imported file)
    let import_re =
        regex::Regex::new(r"(?m)^import\s+([a-zA-Z_][\w]*(?:\.[a-zA-Z_][\w]*)*)").unwrap();
    for cap in import_re.captures_iter(content) {
        let mod_path = &cap[1];
        if mod_path.starts_with("std.") {
            let std_mod = mod_path.trim_start_matches("std.");
            if let Some(blaze) = locate_blaze_dir() {
                let std_file = blaze.join("std").join(format!("{}.fm", std_mod));
                if std_file.is_file() {
                    if let Some(def) = find_symbol_in_file(&std_file, clean_word) {
                        return Some(def);
                    }
                }
            }
        } else {
            let parts: Vec<String> = mod_path.split('.').map(String::from).collect();
            if let Some(found_file) =
                crate::stdlib::locate_import_file(std::path::Path::new(file), &parts)
            {
                let target_path = if found_file.is_dir() {
                    if found_file.join("src").join("main.fm").is_file() {
                        found_file.join("src").join("main.fm")
                    } else if found_file.join("main.fm").is_file() {
                        found_file.join("main.fm")
                    } else if found_file.join("src").join("lib.rs").is_file() {
                        found_file.join("src").join("lib.rs")
                    } else if found_file.join("flame.toml").is_file() {
                        found_file.join("flame.toml")
                    } else {
                        found_file
                    }
                } else {
                    found_file
                };
                if target_path.is_file() {
                    if let Some(def) = find_symbol_in_file(&target_path, clean_word) {
                        return Some(def);
                    }
                }
            }
        }
    }

    // 4. Workspace modules matching clean_word
    let candidate_sibling = std::path::Path::new(file)
        .parent()
        .map(|p| p.join(format!("{}.fm", clean_word)));
    if let Some(ref cand) = candidate_sibling {
        if cand.is_file() {
            return Some(JsonDefinition {
                file: cand.to_string_lossy().to_string(),
                line: 1,
                column: 1,
                end_line: Some(1),
                end_column: Some(1),
            });
        }
    }

    let candidate_src = manifest_dir.join("src").join(format!("{}.fm", clean_word));
    if candidate_src.is_file() {
        return Some(JsonDefinition {
            file: candidate_src.to_string_lossy().to_string(),
            line: 1,
            column: 1,
            end_line: Some(1),
            end_column: Some(1),
        });
    }

    // 5. Flame packages (.flame/pkg/<clean_word>)
    let pkg_root = manifest_dir.join(".flame").join("pkg").join(clean_word);
    let pkg_candidates = [
        pkg_root.join("src").join("main.fm"),
        pkg_root.join("main.fm"),
        pkg_root.join("src").join("lib.rs"),
        pkg_root.join(format!("{}.fmi", clean_word)),
        pkg_root.join("flame.toml"),
    ];
    for cand in &pkg_candidates {
        if cand.is_file() {
            return Some(JsonDefinition {
                file: cand.to_string_lossy().to_string(),
                line: 1,
                column: 1,
                end_line: Some(1),
                end_column: Some(1),
            });
        }
    }

    // 6. Standard library modules & symbols
    if let Some(blaze) = locate_blaze_dir() {
        let std_file = blaze.join("std").join(format!("{}.fm", clean_word));
        if std_file.is_file() {
            return Some(JsonDefinition {
                file: std_file.to_string_lossy().to_string(),
                line: 1,
                column: 1,
                end_line: Some(1),
                end_column: Some(1),
            });
        }

        // Search in Blaze/std/*.fm
        let std_dir = blaze.join("std");
        if let Ok(entries) = std::fs::read_dir(&std_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("fm") {
                    if let Some(def) = find_symbol_in_file(&p, clean_word) {
                        return Some(def);
                    }
                }
            }
        }
    }

    // 7. Compiler Built-ins and Annotations
    let builtins = [
        "println",
        "print",
        "eprint",
        "assert",
        "assertEq",
        "assertNe",
        "assertTrue",
        "assertFalse",
        "panic",
        "typeof",
        "range",
        "sleep",
        "mockData",
        "mockApi",
        "mockFunction",
        "input",
        "Result",
        "Option",
        "Error",
    ];
    if builtins.contains(&clean_word) {
        if let Some(blaze) = locate_blaze_dir() {
            let builtins_file = blaze.join("std").join("builtins.fm");
            if builtins_file.is_file() {
                if let Some(def) = find_symbol_in_file(&builtins_file, clean_word) {
                    return Some(def);
                }
                return Some(JsonDefinition {
                    file: builtins_file.to_string_lossy().to_string(),
                    line: 1,
                    column: 1,
                    end_line: Some(1),
                    end_column: Some(1),
                });
            }
        }
    }

    let annotations = [
        "Application",
        "Test",
        "Embedded",
        "Cli",
        "Command",
        "Platform",
        "Docs",
        "Requires",
        "Permission",
        "Suggestions",
        "Setup",
        "Cleanup",
        "BeforeAll",
        "AfterAll",
        "Ignore",
        "Only",
        "Parameterized",
        "Benchmark",
        "ExpectPanic",
    ];
    if annotations.contains(&clean_word) || word.starts_with('@') {
        if let Some(blaze) = locate_blaze_dir() {
            let ann_file = blaze.join("std").join("annotations.fm");
            if ann_file.is_file() {
                if let Some(def) = find_symbol_in_file(&ann_file, clean_word) {
                    return Some(def);
                }
                return Some(JsonDefinition {
                    file: ann_file.to_string_lossy().to_string(),
                    line: 1,
                    column: 1,
                    end_line: Some(1),
                    end_column: Some(1),
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locate_blaze_dir() {
        let blaze_dir = locate_blaze_dir();
        assert!(blaze_dir.is_some(), "Blaze directory must be located");
        let std_dir = blaze_dir.unwrap().join("std");
        assert!(std_dir.exists(), "Blaze/std directory must exist");
        assert!(
            std_dir.join("builtins.fm").exists(),
            "builtins.fm must exist"
        );
        assert!(
            std_dir.join("annotations.fm").exists(),
            "annotations.fm must exist"
        );
        assert!(std_dir.join("math.fm").exists(), "math.fm must exist");
    }

    #[test]
    fn test_find_local_definition() {
        let source = "fn calculate() -> Int {\n    return 42\n}\n\nfn main() {\n    let res = calculate()\n}";
        let mut lexer = crate::lexer::Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            let is_eof = t.kind == crate::lexer::TokenKind::EOF;
            tokens.push(t);
            if is_eof {
                break;
            }
        }
        let mut parser = crate::parser::Parser::new(tokens, "test.fm".to_string());
        let stmts = parser.parse().unwrap();
        let manifest_dir = std::path::Path::new(".");

        // Hover over calculate() on line 6, col 18
        let def = find_definition("test.fm", source, 6, 18, &stmts, manifest_dir);
        assert!(def.is_some(), "Definition must be found for local function");
        let d = def.unwrap();
        assert_eq!(d.file, "test.fm");
        assert_eq!(d.line, 1);
        assert_eq!(d.column, 4);
    }

    #[test]
    fn test_find_builtin_definition() {
        let source = "fn main() {\n    println(\"hello\")\n}";
        let mut lexer = crate::lexer::Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            let is_eof = t.kind == crate::lexer::TokenKind::EOF;
            tokens.push(t);
            if is_eof {
                break;
            }
        }
        let mut parser = crate::parser::Parser::new(tokens, "test.fm".to_string());
        let stmts = parser.parse().unwrap();
        let manifest_dir = std::path::Path::new(".");

        // Hover over println on line 2, col 6
        let def = find_definition("test.fm", source, 2, 6, &stmts, manifest_dir);
        assert!(def.is_some(), "Definition must be found for println");
        let d = def.unwrap();
        assert!(d.file.ends_with("builtins.fm"));
    }

    #[test]
    fn test_find_annotation_definition() {
        let source = "@Application\nfn main() {}";
        let mut lexer = crate::lexer::Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            let is_eof = t.kind == crate::lexer::TokenKind::EOF;
            tokens.push(t);
            if is_eof {
                break;
            }
        }
        let mut parser = crate::parser::Parser::new(tokens, "test.fm".to_string());
        let stmts = parser.parse().unwrap();
        let manifest_dir = std::path::Path::new(".");

        // Hover over @Application on line 1, col 5
        let def = find_definition("test.fm", source, 1, 5, &stmts, manifest_dir);
        assert!(def.is_some(), "Definition must be found for @Application");
        let d = def.unwrap();
        assert!(d.file.ends_with("annotations.fm"));
    }

    #[test]
    fn test_find_stdlib_member_definition() {
        let source = "import std.math\nfn main() {\n    let s = math.sqrt(4.0)\n}";
        let mut lexer = crate::lexer::Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let t = lexer.next_token();
            let is_eof = t.kind == crate::lexer::TokenKind::EOF;
            tokens.push(t);
            if is_eof {
                break;
            }
        }
        let mut parser = crate::parser::Parser::new(tokens, "test.fm".to_string());
        let stmts = parser.parse().unwrap();
        let manifest_dir = std::path::Path::new(".");

        // Hover over sqrt on line 3, col 20
        let def = find_definition("test.fm", source, 3, 20, &stmts, manifest_dir);
        assert!(def.is_some(), "Definition must be found for math.sqrt");
        let d = def.unwrap();
        assert!(d.file.ends_with("math.fm"));
    }
}
