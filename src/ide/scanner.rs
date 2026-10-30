pub use crate::utils::text::{extract_balanced_block, strip_comments_and_strings};
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct ScannedVar {
    pub name: String,
    pub typ: Option<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScannedMethod {
    pub name: String,
    pub signature: String,
    pub doc: Option<String>,
    pub return_type: Option<String>,
    pub params: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct ScannedStruct {
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub methods: Vec<String>,
    pub method_details: Vec<ScannedMethod>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScannedEnumVariant {
    pub name: String,
    pub payload: Option<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScannedEnum {
    pub name: String,
    pub variants: Vec<ScannedEnumVariant>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScannedFunction {
    pub name: String,
    pub module: String,
    pub params: Vec<(String, String)>,
    pub return_type: Option<String>,
    pub doc: Option<String>,
    pub signature: String,
}

#[derive(Debug, Clone, Default)]
pub struct BlazeStdIndex {
    pub structs: Vec<ScannedStruct>,
    pub enums: Vec<ScannedEnum>,
    pub functions: HashMap<String, ScannedFunction>,
}

static BLAZE_STD_INDEX: OnceLock<BlazeStdIndex> = OnceLock::new();

pub fn get_blaze_std_index() -> &'static BlazeStdIndex {
    BLAZE_STD_INDEX.get_or_init(|| {
        let mut index = BlazeStdIndex::default();
        for (file_name, content) in crate::blaze::EMBEDDED_BLAZE_STD {
            let mod_name = file_name.strip_suffix(".fm").unwrap_or(file_name);

            // 1. Scan enums
            let enums = scan_document_enums(content);
            for e in enums {
                if !index.enums.iter().any(|existing| existing.name == e.name) {
                    index.enums.push(e);
                }
            }

            // 2. Scan structs & impls
            let structs = scan_document_structs(content);
            for s in structs {
                if let Some(existing) = index
                    .structs
                    .iter_mut()
                    .find(|existing| existing.name == s.name)
                {
                    for f in s.fields {
                        if !existing.fields.iter().any(|(n, _)| n == &f.0) {
                            existing.fields.push(f);
                        }
                    }
                    for m in s.methods {
                        if !existing.methods.contains(&m) {
                            existing.methods.push(m);
                        }
                    }
                    for md in s.method_details {
                        if !existing.method_details.iter().any(|e| e.name == md.name) {
                            existing.method_details.push(md);
                        }
                    }
                    if existing.doc.is_none() {
                        existing.doc = s.doc;
                    }
                } else {
                    index.structs.push(s);
                }
            }

            // 3. Scan top-level functions
            let fn_decl_re = Regex::new(
                r#"(?:@Docs\s*\(\s*(?:"([^"]*)"|'([^']*)')\s*\)\s*)?(?:export\s+)?(?:async\s+)?fn\s+([a-zA-Z_]\w*)(?:<[^>]*>)?\s*\(([^)]*)\)(?:\s*->\s*([a-zA-Z0-9_<>, \t\[\]?]+))?"#,
            )
            .unwrap();
            for cap in fn_decl_re.captures_iter(content) {
                let doc = cap
                    .get(1)
                    .or_else(|| cap.get(2))
                    .map(|m| m.as_str().to_string());
                let fn_name = cap[3].to_string();
                let params_str = cap[4].trim();
                let ret_str = cap.get(5).map(|m| m.as_str().trim().to_string());
                let sig = if let Some(ref ret) = ret_str {
                    format!("fn {}({}) -> {}", fn_name, params_str, ret)
                } else {
                    format!("fn {}({})", fn_name, params_str)
                };

                let mut params = Vec::new();
                for p in split_params_balanced(params_str) {
                    let p = p.trim();
                    if let Some((pname, ptype)) = p.split_once(':') {
                        params.push((pname.trim().to_string(), ptype.trim().to_string()));
                    } else if !p.is_empty() {
                        params.push((p.to_string(), "Unknown".to_string()));
                    }
                }

                let scanned_fn = ScannedFunction {
                    name: fn_name.clone(),
                    module: mod_name.to_string(),
                    params,
                    return_type: ret_str,
                    doc,
                    signature: sig,
                };

                index
                    .functions
                    .insert(format!("{}.{}", mod_name, fn_name), scanned_fn.clone());
                index.functions.insert(
                    format!("std.{}.{}", mod_name, fn_name),
                    scanned_fn.clone(),
                );
                index.functions.entry(fn_name).or_insert(scanned_fn);
            }
        }
        index
    })
}

pub fn scan_document_enums(content: &str) -> Vec<ScannedEnum> {
    let stripped = strip_comments_and_strings(content);
    let content = &stripped;
    let mut enums = Vec::new();

    let enum_header_re = Regex::new(
        r#"(?:@Docs\s*\(\s*(?:"([^"]*)"|'([^']*)')\s*\)\s*)?(?:export\s+)?enum\s+([a-zA-Z_]\w*)(?:\s*<[^>]*>)?\s*\{"#,
    )
    .unwrap();
    let variant_re = Regex::new(
        r#"(?:@Docs\s*\(\s*(?:"([^"]*)"|'([^']*)')\s*\)\s*)?([a-zA-Z_]\w*)(?:\s*(\([^)]*\)))?"#,
    )
    .unwrap();

    for cap in enum_header_re.captures_iter(content) {
        let doc = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().to_string());
        let name = cap[3].to_string();
        let match_obj = cap.get(0).unwrap();
        let open_brace_pos = match_obj.end() - 1;
        let mut variants = Vec::new();
        if let Some(body) = extract_balanced_block(content, open_brace_pos) {
            for v_cap in variant_re.captures_iter(&body) {
                let v_doc = v_cap
                    .get(1)
                    .or_else(|| v_cap.get(2))
                    .map(|m| m.as_str().to_string());
                let v_name = v_cap[3].to_string();
                let v_payload = v_cap.get(4).map(|m| m.as_str().to_string());
                variants.push(ScannedEnumVariant {
                    name: v_name,
                    payload: v_payload,
                    doc: v_doc,
                });
            }
        }
        enums.push(ScannedEnum {
            name,
            variants,
            doc,
        });
    }

    enums
}

pub fn split_params_balanced(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' | '[' | '<' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '>' => {
                if depth > 0 {
                    depth -= 1;
                }
                current.push(c);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(c),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        parts.push(trimmed.to_string());
    }
    parts
}

pub fn scan_document_structs(content: &str) -> Vec<ScannedStruct> {
    let stripped = strip_comments_and_strings(content);
    let content = &stripped;
    let mut structs = Vec::new();

    // Scan for structs: `struct Name { field: type, ... }`
    let struct_header_re = Regex::new(
        r#"(?:@Docs\s*\(\s*(?:"([^"]*)"|'([^']*)')\s*\)\s*)?(?:export\s+)?struct\s+([a-zA-Z_]\w*)(?:\s*<[^>]*>)?\s*\{"#,
    )
    .unwrap();
    let field_re = Regex::new(r"([a-zA-Z_]\w*)\s*:\s*([a-zA-Z0-9_<>, \t\[\]\?]+)").unwrap();
    for cap in struct_header_re.captures_iter(content) {
        let mut fields = Vec::new();
        let doc = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().to_string());
        let name = cap[3].to_string();
        let match_obj = cap.get(0).unwrap();
        let open_brace_pos = match_obj.end() - 1;
        if let Some(body) = extract_balanced_block(content, open_brace_pos) {
            for field_cap in field_re.captures_iter(body) {
                let clean_type = field_cap[2]
                    .trim()
                    .trim_end_matches(',')
                    .trim_end_matches(';')
                    .trim()
                    .to_string();
                fields.push((field_cap[1].to_string(), clean_type));
            }
        }
        structs.push(ScannedStruct {
            name,
            fields,
            methods: vec![],
            method_details: vec![],
            doc,
        });
    }

    // Scan for impls: `impl Name { fn method(...) { ... } }` or `impl Circle: Drawable { ... }` or `impl<T> Box<T> { ... }`
    let impl_header_re =
        Regex::new(r#"impl(?:\s*<[^>]*>)?\s+([a-zA-Z_]\w*)(?:\s*<[^>]*>)?(?:\s*:\s*[^{]+)?\s*\{"#)
            .unwrap();
    let fn_start_re =
        Regex::new(r#"(?:@Docs\s*\(\s*(?:"([^"]*)"|'([^']*)')\s*\)\s*)?fn\s+([a-zA-Z_]\w*)(?:\s*<[^>]*>)?\s*\("#)
            .unwrap();
    for cap in impl_header_re.captures_iter(content) {
        let name = cap[1].to_string();
        let match_obj = cap.get(0).unwrap();
        let open_brace_pos = match_obj.end() - 1;
        if let Some(body) = extract_balanced_block(content, open_brace_pos) {
            let mut methods = Vec::new();
            let mut method_details = Vec::new();
            for fn_cap in fn_start_re.captures_iter(body) {
                let m_doc = fn_cap
                    .get(1)
                    .or_else(|| fn_cap.get(2))
                    .map(|m| m.as_str().to_string());
                let m_name = fn_cap[3].to_string();
                let full_match = fn_cap.get(0).unwrap();
                let paren_open_pos = full_match.end() - 1;
                // Find matching ')'
                let mut p_depth = 1;
                let mut paren_close_pos = None;
                for (offset, c) in body[paren_open_pos + 1..].char_indices() {
                    if c == '(' {
                        p_depth += 1;
                    } else if c == ')' {
                        p_depth -= 1;
                        if p_depth == 0 {
                            paren_close_pos = Some(paren_open_pos + 1 + offset);
                            break;
                        }
                    }
                }

                if let Some(close_pos) = paren_close_pos {
                    let params_str = body[paren_open_pos + 1..close_pos].trim();
                    let after_paren = &body[close_pos + 1..];
                    let ret_str = if let Some(arrow_idx) = after_paren.find("->") {
                        let after_arrow = &after_paren[arrow_idx + 2..];
                        let end_pos = after_arrow
                            .find('{')
                            .or_else(|| after_arrow.find(';'))
                            .unwrap_or_else(|| after_arrow.find('\n').unwrap_or(after_arrow.len()));
                        let r = after_arrow[..end_pos].trim();
                        if !r.is_empty() {
                            Some(r.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let ret_display = ret_str.as_deref().unwrap_or("Nil");
                    let sig = format!("fn {}({}) -> {}", m_name, params_str, ret_display);

                    let mut params = Vec::new();
                    for p in split_params_balanced(params_str) {
                        let p = p.trim();
                        if let Some((pname, ptype)) = p.split_once(':') {
                            params.push((pname.trim().to_string(), ptype.trim().to_string()));
                        } else if !p.is_empty() {
                            params.push((p.to_string(), "Unknown".to_string()));
                        }
                    }

                    methods.push(m_name.clone());
                    method_details.push(ScannedMethod {
                        name: m_name,
                        signature: sig,
                        doc: m_doc,
                        return_type: ret_str,
                        params,
                    });
                }
            }
            if let Some(s) = structs.iter_mut().find(|s| s.name == name) {
                s.methods.extend(methods);
                s.method_details.extend(method_details);
            } else {
                structs.push(ScannedStruct {
                    name,
                    fields: vec![],
                    methods,
                    method_details,
                    doc: None,
                });
            }
        }
    }

    structs
}

pub fn infer_expression_type(
    expr: &str,
    doc_functions: &HashMap<String, String>,
    doc_structs: &[ScannedStruct],
    doc_enums: &[ScannedEnum],
    blaze_std: &BlazeStdIndex,
    vars: &[ScannedVar],
) -> Option<String> {
    let expr = expr.trim().trim_end_matches(';').trim();
    if expr.is_empty() {
        return None;
    }
    let expr = if let Some(stripped) = expr.strip_prefix("await ") {
        stripped.trim()
    } else {
        expr
    };

    // 1. String literal
    if expr.starts_with('"') || expr.starts_with("$\"") || expr.starts_with('\'') {
        return Some("String".to_string());
    }

    // 2. Boolean literal
    if expr == "true" || expr == "false" {
        return Some("Bool".to_string());
    }

    // 3. Integer literal
    if expr.parse::<i64>().is_ok() {
        return Some("Int".to_string());
    }

    // 4. Float literal
    if expr.parse::<f64>().is_ok() {
        return Some("Float".to_string());
    }

    // 5. Array literal
    if expr.starts_with('[') && expr.ends_with(']') {
        return Some("[Unknown]".to_string());
    }

    // 6. Object literal
    if expr.starts_with('{') && expr.ends_with('}') {
        return Some("Object".to_string());
    }

    // 7. Struct constructor: `Name { ... }` or `Name.new(...)`
    if let Some(brace_pos) = expr.find('{') {
        let prefix = expr[..brace_pos].trim();
        let struct_name = prefix.split('.').last().unwrap_or(prefix).trim();
        if !struct_name.is_empty()
            && struct_name
                .chars()
                .next()
                .map_or(false, |c| c.is_ascii_uppercase())
            && !matches!(struct_name, "IF" | "FOR" | "MATCH" | "WHILE")
        {
            return Some(struct_name.to_string());
        }
    }

    if let Some(paren_pos) = expr.find('(') {
        let callee = expr[..paren_pos].trim();

        // 7b. `StructName.new(...)` or `StructName.default(...)`
        if let Some((s_name, method)) = callee.rsplit_once('.') {
            if (method == "new" || method == "default")
                && s_name
                    .chars()
                    .next()
                    .map_or(false, |c| c.is_ascii_uppercase())
            {
                let pure_name = s_name.split('.').last().unwrap_or(s_name).trim();
                return Some(pure_name.to_string());
            }
        }

        // 8. Function call: `callee(...)`
        // 8a. Check user-defined functions in document
        if let Some(ret) = doc_functions.get(callee) {
            if !ret.is_empty() && ret != "()" && ret != "Nil" {
                return Some(ret.clone());
            }
        }

        // 8b. Check standard library functions from Blaze std
        if let Some(f) = blaze_std.functions.get(callee) {
            if let Some(ref ret) = f.return_type {
                return Some(ret.clone());
            }
        }

        // Try stripped callee without module or std prefix
        if let Some(bare_name) = callee.rsplit('.').next() {
            if let Some(ret) = doc_functions.get(bare_name) {
                if !ret.is_empty() && ret != "()" && ret != "Nil" {
                    return Some(ret.clone());
                }
            }
            if let Some(f) = blaze_std.functions.get(bare_name) {
                if let Some(ref ret) = f.return_type {
                    return Some(ret.clone());
                }
            }
        }

        // 8c. Method call on an object / member chain: `a.b.c(...)`
        if let Some((base, method_name)) = callee.rsplit_once('.') {
            let base_type = infer_expression_type(
                base,
                doc_functions,
                doc_structs,
                doc_enums,
                blaze_std,
                vars,
            );
            if let Some(ref b_type) = base_type {
                let s_def = doc_structs
                    .iter()
                    .find(|s| s.name == *b_type)
                    .or_else(|| blaze_std.structs.iter().find(|s| s.name == *b_type));
                if let Some(s) = s_def {
                    if let Some(m) = s.method_details.iter().find(|m| m.name == method_name) {
                        if let Some(ref ret) = m.return_type {
                            return Some(ret.clone());
                        }
                    }
                }
            }
        }
    }

    // 9. Enum variant: `EnumName.Variant` or `EnumName::Variant`
    if let Some((enum_name, _variant)) = expr.split_once('.').or_else(|| expr.split_once("::")) {
        let pure_enum = enum_name.trim();
        if doc_enums.iter().any(|e| e.name == pure_enum)
            || blaze_std.enums.iter().any(|e| e.name == pure_enum)
        {
            return Some(pure_enum.to_string());
        }
    }

    // 10. Member access chain without parens: `a.b.c`
    if let Some((base, field_name)) = expr.rsplit_once('.') {
        let field_name = field_name.trim();
        let base_type = infer_expression_type(
            base,
            doc_functions,
            doc_structs,
            doc_enums,
            blaze_std,
            vars,
        );
        if let Some(ref b_type) = base_type {
            let s_def = doc_structs
                .iter()
                .find(|s| s.name == *b_type)
                .or_else(|| blaze_std.structs.iter().find(|s| s.name == *b_type));
            if let Some(s) = s_def {
                if let Some((_, f_type)) = s.fields.iter().find(|(name, _)| name == field_name) {
                    return Some(f_type.clone());
                }
            }
        }
    }

    // 11. Variable reference: `x`
    if let Some(v) = vars.iter().find(|v| v.name == expr) {
        if let Some(ref t) = v.typ {
            return Some(t.clone());
        }
    }

    None
}

pub fn scan_document(content: &str) -> (Vec<ScannedVar>, Vec<ScannedStruct>) {
    let stripped = strip_comments_and_strings(content);
    let content = &stripped;
    let mut vars: Vec<ScannedVar> = Vec::new();

    // 1. Scan structs and impls in document
    let mut structs = scan_document_structs(content);
    let doc_enums = scan_document_enums(content);

    // 2. Scan functions and annotations in document FIRST so return types are known!
    let fn_decl_re = Regex::new(
        r#"(?:@Docs\s*\(\s*(?:"([^"]*)"|'([^']*)')\s*\)\s*)?(?:export\s+)?(?:async\s+)?(fn|annotation)\s+([a-zA-Z_]\w*)(?:<[^>]*>)?\s*\(([^)]*)\)(?:\s*->\s*([a-zA-Z0-9_<>, \t\[\]?]+))?"#,
    )
    .unwrap();

    let mut doc_functions: HashMap<String, String> = HashMap::new();
    let mut annotation_returns: HashMap<String, String> = HashMap::new();
    for cap in fn_decl_re.captures_iter(content) {
        let doc_str = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().to_string());
        let kind_kw = &cap[3];
        let name_str = &cap[4];
        let params_str = cap[5].trim();
        let ret_str = cap.get(6).map_or("()", |m| m.as_str().trim());

        if kind_kw == "annotation" {
            annotation_returns.insert(name_str.to_string(), ret_str.to_string());
            let sig = if ret_str == "()" {
                format!("annotation @{}({})", name_str, params_str)
            } else {
                format!("annotation @{}({}) -> {}", name_str, params_str, ret_str)
            };
            vars.push(ScannedVar {
                name: name_str.to_string(),
                typ: Some(sig),
                doc: doc_str,
            });
        } else {
            if ret_str != "()" && ret_str != "Nil" {
                doc_functions.insert(name_str.to_string(), ret_str.to_string());
            }
            let sig = if ret_str == "()" {
                format!("fn {}({})", name_str, params_str)
            } else {
                format!("fn {}({}) -> {}", name_str, params_str, ret_str)
            };
            vars.push(ScannedVar {
                name: name_str.to_string(),
                typ: Some(sig),
                doc: doc_str,
            });
        }
    }

    // Helper to push or update variable type and doc without duplicates
    fn push_or_update(vars: &mut Vec<ScannedVar>, name: String, typ: String, doc: Option<String>) {
        if let Some(existing) = vars.iter_mut().find(|v| v.name == name) {
            existing.typ = Some(typ);
            if doc.is_some() {
                existing.doc = doc;
            }
        } else {
            vars.push(ScannedVar {
                name,
                typ: Some(typ),
                doc,
            });
        }
    }

    let blaze_std = get_blaze_std_index();

    // 3. Scan for variable declarations: `let x: Type = ...` or `let x = rhs`
    let var_decl_re = Regex::new(
        r#"(?:let|const)(?:\s+mut)?\s+([a-zA-Z_]\w*)(?:\s*:\s*([a-zA-Z0-9_<>, \t()\[\]\-?]+?))?(?:\s*=\s*([^;\n]+))?"#,
    )
    .unwrap();
    for cap in var_decl_re.captures_iter(content) {
        let name = cap[1].to_string();
        let explicit_ty = cap.get(2).map(|m| m.as_str().trim().to_string());
        let rhs = cap.get(3).map(|m| m.as_str().trim());

        let typ = if let Some(ty) = explicit_ty {
            if matches!(
                ty.as_str(),
                "if" | "else" | "match" | "loop" | "while" | "for" | "return" | "break"
                    | "continue" | "fn" | "let" | "const" | "mut"
            ) {
                None
            } else {
                Some(ty)
            }
        } else if let Some(rhs_str) = rhs {
            infer_expression_type(
                rhs_str,
                &doc_functions,
                &structs,
                &doc_enums,
                blaze_std,
                &vars,
            )
        } else {
            None
        };

        if let Some(t) = typ {
            push_or_update(&mut vars, name, t, None);
        } else if !vars.iter().any(|v| v.name == name) {
            vars.push(ScannedVar {
                name,
                typ: None,
                doc: None,
            });
        }
    }

    // 4. Scan for loop variables: `for w in windows {`, `for w in window.list() {`, `for i in 0..10 {`
    let for_in_re =
        Regex::new(r"for\s+([a-zA-Z_]\w*)\s+in\s+([a-zA-Z0-9_.]+(?:\([^)]*\))?)").unwrap();
    for cap in for_in_re.captures_iter(content) {
        let item_name = cap[1].to_string();
        let iter_expr = cap[2].trim();

        let item_ty = if iter_expr.contains("..") {
            Some("Int".to_string())
        } else if let Some(iter_ty) = infer_expression_type(
            iter_expr,
            &doc_functions,
            &structs,
            &doc_enums,
            blaze_std,
            &vars,
        ) {
            if iter_ty.starts_with('[') && iter_ty.ends_with(']') {
                Some(iter_ty[1..iter_ty.len() - 1].trim().to_string())
            } else if iter_ty.starts_with("Vec<") && iter_ty.ends_with('>') {
                Some(iter_ty[4..iter_ty.len() - 1].trim().to_string())
            } else {
                Some(iter_ty)
            }
        } else {
            None
        };

        if let Some(ty) = item_ty {
            push_or_update(
                &mut vars,
                item_name,
                ty,
                Some("Iteration loop variable".to_string()),
            );
        } else {
            push_or_update(
                &mut vars,
                item_name,
                "Unknown".to_string(),
                Some("Iteration loop variable".to_string()),
            );
        }
    }

    // Scan for tuple destructuring from channel: `let (tx, rx) = ...channel(...)`
    let channel_destructure_re = Regex::new(
        r"(?:let|const)\s*\(\s*([a-zA-Z_]\w*)\s*,\s*([a-zA-Z_]\w*)\s*\)\s*=\s*(?:[a-zA-Z_]\w*\.)*channel\s*\(",
    )
    .unwrap();
    for cap in channel_destructure_re.captures_iter(content) {
        vars.push(ScannedVar {
            name: cap[1].to_string(),
            typ: Some("Sender".to_string()),
            doc: Some("Thread message channel sender".to_string()),
        });
        vars.push(ScannedVar {
            name: cap[2].to_string(),
            typ: Some("Receiver".to_string()),
            doc: Some("Thread message channel receiver".to_string()),
        });
    }

    // Scan for cloned senders: `let tx2 = tx.clone()`
    let clone_sender_re =
        Regex::new(r"(?:let|const)\s+([a-zA-Z_]\w*)\s*=\s*([a-zA-Z_]\w*)\.clone\s*\(").unwrap();
    for cap in clone_sender_re.captures_iter(content) {
        let new_var = cap[1].to_string();
        let orig_var = &cap[2];
        if vars
            .iter()
            .any(|v| v.name == *orig_var && v.typ.as_deref() == Some("Sender"))
        {
            vars.push(ScannedVar {
                name: new_var,
                typ: Some("Sender".to_string()),
                doc: Some("Thread message channel sender (cloned)".to_string()),
            });
        }
    }

    let client_doc = "`ServerClient` connection instance representing the remote WebSocket client.\n\nProvides methods for message transmission and connection management:\n- `client.send(msg)` - Transmit text or binary payload\n- `client.sendText(text)` - Transmit UTF-8 text frame\n- `client.sendBytes(bytes)` - Transmit binary frame\n- `client.ping(data)` - Send Ping control frame\n- `client.close()` - Gracefully disconnect client\n- `client.id` - Unique connection identifier (`Int`)\n- `client.address` - Client IP/port address string (`String`)".to_string();

    let bytes_doc = "`Bytes` buffer containing raw binary WebSocket frame data.\n\nProvides methods for binary manipulation:\n- `bytes.len()` - Total byte count (`Int`)\n- `bytes.slice(start, len)` - Sub-slice of byte buffer\n- `bytes.toString()` - Decode buffer to UTF-8 String\n- `bytes.toHex()` - Hexadecimal representation string\n- `bytes.get(idx)` - Read single byte value at index".to_string();

    // Scan for WebSocket server callback parameters: onConnect((client) { ... })
    let on_connect_re =
        Regex::new(r"onConnect\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)")
            .unwrap();
    for cap in on_connect_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "ServerClient".to_string(),
            Some(client_doc.clone()),
        );
    }

    // onMessage((client, msg) { ... })
    let on_msg_re = Regex::new(r"onMessage\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*,\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)").unwrap();
    for cap in on_msg_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "ServerClient".to_string(),
            Some(client_doc.clone()),
        );
        push_or_update(
            &mut vars,
            cap[2].to_string(),
            "String".to_string(),
            Some("UTF-8 text message payload received from the WebSocket connection.".to_string()),
        );
    }

    // onBinary((client, bytes) { ... })
    let on_bin_re = Regex::new(r"onBinary\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*,\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)").unwrap();
    for cap in on_bin_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "ServerClient".to_string(),
            Some(client_doc.clone()),
        );
        push_or_update(
            &mut vars,
            cap[2].to_string(),
            "Bytes".to_string(),
            Some(bytes_doc.clone()),
        );
    }

    // onClose((client, code, reason) { ... })
    let on_close_re = Regex::new(r"onClose\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*,\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*,\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)").unwrap();
    for cap in on_close_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "ServerClient".to_string(),
            Some(client_doc.clone()),
        );
        push_or_update(&mut vars, cap[2].to_string(), "Int".to_string(), Some("WebSocket close status code according to RFC 6455 (e.g., `1000` Normal Closure, `1001` Going Away, `1006` Abnormal Closure).".to_string()));
        push_or_update(
            &mut vars,
            cap[3].to_string(),
            "String".to_string(),
            Some(
                "WebSocket close reason message explaining why the connection terminated."
                    .to_string(),
            ),
        );
    }

    // onError((client, err) { ... })
    let on_err_re = Regex::new(r"onError\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*,\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)").unwrap();
    for cap in on_err_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "ServerClient".to_string(),
            Some(client_doc.clone()),
        );
        push_or_update(
            &mut vars,
            cap[2].to_string(),
            "String".to_string(),
            Some(
                "WebSocket error diagnostic message describing transport or protocol failure."
                    .to_string(),
            ),
        );
    }

    // onPing / onPong: ((client, data) { ... })
    let on_ping_re = Regex::new(r"(?:onPing|onPong)\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*,\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)").unwrap();
    for cap in on_ping_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "ServerClient".to_string(),
            Some(client_doc.clone()),
        );
        push_or_update(
            &mut vars,
            cap[2].to_string(),
            "Bytes".to_string(),
            Some(bytes_doc.clone()),
        );
    }

    // Client socket single-param callbacks: socket.onMessage((msg) { ... }), socket.onBinary((bytes) { ... })
    let client_msg_re = Regex::new(r"(?:socket|client)\.onMessage\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)").unwrap();
    for cap in client_msg_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "String".to_string(),
            Some("UTF-8 text message payload received from the WebSocket connection.".to_string()),
        );
    }

    let client_bin_re = Regex::new(r"(?:socket|client)\.onBinary\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)").unwrap();
    for cap in client_bin_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "Bytes".to_string(),
            Some(bytes_doc.clone()),
        );
    }

    let client_close_re = Regex::new(r"(?:socket|client)\.onClose\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*,\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)").unwrap();
    for cap in client_close_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "Int".to_string(),
            Some(
                "WebSocket close status code according to RFC 6455 (e.g. 1000 for normal closure)."
                    .to_string(),
            ),
        );
        push_or_update(
            &mut vars,
            cap[2].to_string(),
            "String".to_string(),
            Some("WebSocket close reason message.".to_string()),
        );
    }

    let client_err_re = Regex::new(r"(?:socket|client)\.onError\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)").unwrap();
    for cap in client_err_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "String".to_string(),
            Some(
                "WebSocket error diagnostic message describing transport or protocol failure."
                    .to_string(),
            ),
        );
    }

    // Stream callback: stream.forEach((msg) { ... })
    let stream_for_re = Regex::new(
        r"\.(?:forEach|onEach)\s*\(\s*(?:fn\s*)?\(\s*([a-zA-Z_]\w*)(?:\s*:\s*[a-zA-Z_]\w*)?\s*\)",
    )
    .unwrap();
    for cap in stream_for_re.captures_iter(content) {
        push_or_update(
            &mut vars,
            cap[1].to_string(),
            "String".to_string(),
            Some("Stream message item payload.".to_string()),
        );
    }

    // Scan for annotations to inject implicit variables
    let annotation_re = Regex::new(r"@([a-zA-Z_]\w*)\s*(?:\(|$)").unwrap();
    for cap in annotation_re.captures_iter(content) {
        let name = cap[1].to_string();
        let var_name = name.to_lowercase();
        // Set typ to a special marker so main.rs knows it's from an annotation
        vars.push(ScannedVar {
            name: var_name.clone(),
            typ: Some(format!("annotation_plugin:{}", var_name)),
            doc: None,
        });
    }

    // Scan for variables with formula bodies to extract fields
    let formula_header_re =
        Regex::new(r"(?:let|const)(?:\s+mut)?\s+([a-zA-Z_]\w*)\s*=\s*formula\s*\{").unwrap();
    for cap in formula_header_re.captures_iter(content) {
        let name = cap[1].to_string();
        let match_obj = cap.get(0).unwrap();
        let open_brace_pos = match_obj.end() - 1;
        if let Some(body) = extract_balanced_block(content, open_brace_pos) {
            let mut fields = Vec::new();
            let formula_field_re = Regex::new(r"([a-zA-Z_]\w*)\s*:").unwrap();
            for field_cap in formula_field_re.captures_iter(body) {
                fields.push((field_cap[1].to_string(), "Unknown".to_string()));
            }
            let synthetic_type = format!("__formula_{}", name);
            structs.push(ScannedStruct {
                name: synthetic_type.clone(),
                fields,
                methods: vec!["toString".to_string()],
                method_details: vec![],
                doc: None,
            });
            // Overwrite or add to vars at the beginning so it is found first
            vars.insert(
                0,
                ScannedVar {
                    name,
                    typ: Some(synthetic_type),
                    doc: None,
                },
            );
        }
    }

    // Scan for annotation usages: `@Component` -> injects `component: ReturnType`
    let ann_usage_re = Regex::new(r"@([A-Z]\w*)").unwrap();
    for cap in ann_usage_re.captures_iter(content) {
        let ann_name = &cap[1];
        let mut c = ann_name.chars();
        let lower_name = match c.next() {
            None => String::new(),
            Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        };

        let typ = annotation_returns
            .get(ann_name)
            .cloned()
            .unwrap_or_else(|| ann_name.to_string());

        vars.push(ScannedVar {
            name: lower_name,
            typ: Some(typ),
            doc: None,
        });
    }

    // Scan for imports: `import path as alias` or `import path`
    let import_re =
        Regex::new(r"import\s+([a-zA-Z_][\w]*(?:\.[a-zA-Z_][\w]*)*)(?:\s+as\s+([a-zA-Z_]\w*))?")
            .unwrap();
    for cap in import_re.captures_iter(content) {
        let path = cap[1].to_string();
        let alias = cap
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| path.rsplit('.').next().unwrap_or(&path).to_string());
        let doc = format!("```flame\nimport {}\n```\nImported module `{}`", path, path);
        vars.push(ScannedVar {
            name: alias,
            typ: Some(format!("import:{}", path)),
            doc: Some(doc),
        });
    }

    // Scan for quoted imports: `import "path" as alias` or `import "path"`
    let quoted_import_re = Regex::new(r#"import\s+"([^"]+)"(?:\s+as\s+([a-zA-Z_]\w*))?"#).unwrap();
    for cap in quoted_import_re.captures_iter(content) {
        let path = cap[1].to_string();
        if let Some(alias_match) = cap.get(2) {
            let alias = alias_match.as_str().to_string();
            let ext = std::path::Path::new(&path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let desc = match ext {
                "js" | "mjs" | "cjs" | "ts" => "External JavaScript module",
                "css" => "CSS stylesheet resource",
                "json" => "JSON data resource",
                "fmi" => "FMI interface metadata resource",
                "toml" => "TOML configuration resource",
                "txt" | "text" => "Text resource",
                "html" | "htm" => "HTML template resource",
                _ => "File resource",
            };
            let doc = format!(
                "```flame\nimport \"{}\" as {}\n```\n{} `{}`",
                path, alias, desc, path
            );
            vars.push(ScannedVar {
                name: alias,
                typ: Some(format!("import:\"{}\"", path)),
                doc: Some(doc),
            });
        }
    }

    (vars, structs)
}
