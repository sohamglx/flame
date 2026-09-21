pub use crate::utils::text::{extract_balanced_block, strip_comments_and_strings};
use regex::Regex;

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

pub fn scan_document(content: &str) -> (Vec<ScannedVar>, Vec<ScannedStruct>) {
    let stripped = strip_comments_and_strings(content);
    let content = &stripped;
    let mut vars: Vec<ScannedVar> = Vec::new();
    let mut structs = Vec::new();

    // Scan for structs: `struct Name { field: type, ... }`
    let struct_header_re = Regex::new(r#"(?:@Docs\s*\(\s*(?:"([^"]*)"|'([^']*)')\s*\)\s*)?(?:export\s+)?struct\s+([a-zA-Z_]\w*)\s*\{"#).unwrap();
    let field_re = Regex::new(r"([a-zA-Z_]\w*)\s*:\s*([a-zA-Z_]\w*)").unwrap();
    for cap in struct_header_re.captures_iter(content) {
        let doc = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().to_string());
        let name = cap[3].to_string();
        let match_obj = cap.get(0).unwrap();
        let open_brace_pos = match_obj.end() - 1;
        let mut fields = Vec::new();
        if let Some(body) = extract_balanced_block(content, open_brace_pos) {
            for field_cap in field_re.captures_iter(body) {
                fields.push((field_cap[1].to_string(), field_cap[2].to_string()));
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

    // Helper to split parameters respecting nested (), [], <>
    fn split_params_balanced(s: &str) -> Vec<String> {
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

    // Scan for impls: `impl Name { fn method(...) { ... } }`
    let impl_header_re = Regex::new(r"impl\s+([a-zA-Z_]\w*)\s*\{").unwrap();
    let fn_start_re =
        Regex::new(r#"(?:@Docs\s*\(\s*(?:"([^"]*)"|'([^']*)')\s*\)\s*)?fn\s+([a-zA-Z_]\w*)\s*\("#)
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

    // Scan for variables: `let x = StructName.new(...)`, `let x: StructName = ...`, `let x: (msg: Unknown) -> Nil = ...`, `let x = StructName { ... }`
    let var_re =
        Regex::new(r#"(?:let|const)(?:\s+mut)?\s+([a-zA-Z_]\w*)(?:\s*:\s*([a-zA-Z0-9_<>, \t()\[\]\-]+?))?(?:\s*=\s*(?:(await)\s+)?(?:[a-zA-Z_]\w*\.)*([a-zA-Z_]\w*)(?:\.new|\s*\{|\s*\()?)?"#).unwrap();
    for cap in var_re.captures_iter(content) {
        let name = cap[1].to_string();
        let mut typ = cap.get(2).map(|m| m.as_str().to_string()).or_else(|| {
            if cap.get(3).is_some() {
                Some("Promise".to_string())
            } else {
                cap.get(4).map(|m| m.as_str().to_string())
            }
        });
        if let Some(ref t) = typ {
            if t == "listen" {
                typ = Some("Server".to_string());
            } else if t == "connect" {
                typ = Some("ClientSocket".to_string());
            } else if t == "messages" {
                typ = Some("Stream".to_string());
            }
        }
        vars.push(ScannedVar {
            name,
            typ,
            doc: None,
        });
    }

    // Scan for variable assignments to known standard library function calls and literals
    let let_assign_re = Regex::new(r#"(?:let|const)(?:\s+mut)?\s+([a-zA-Z_]\w*)(?:\s*:\s*([a-zA-Z0-9_<>, \t()\[\]\-]+?))?\s*=\s*(.+)"#).unwrap();
    for cap in let_assign_re.captures_iter(content) {
        let name = cap[1].to_string();
        let explicit_ty = cap.get(2).map(|m| m.as_str().trim().to_string());
        let rhs = cap[3].trim();

        let inferred_ty = if let Some(ty) = explicit_ty {
            Some(ty)
        } else if rhs.starts_with("window.list(") {
            Some("[Window]".to_string())
        } else if rhs.starts_with("window.find(") {
            Some("Window".to_string())
        } else if rhs.starts_with("os.memory(") {
            Some("memStates".to_string())
        } else if rhs.starts_with("os.user(") {
            Some("userInfo".to_string())
        } else if rhs.starts_with("desktop.mouse") {
            Some("Mouse".to_string())
        } else if rhs.starts_with("desktop.keyboard") {
            Some("Keyboard".to_string())
        } else if rhs.starts_with("desktop.open(") {
            Some("Bool".to_string())
        } else if rhs.starts_with("fs.readFile(") || rhs.starts_with("fs.readToString(") {
            Some("String".to_string())
        } else if rhs.starts_with("fs.readBytes(") {
            Some("Bytes".to_string())
        } else if rhs.starts_with("fs.open(") {
            Some("File".to_string())
        } else if rhs.starts_with('"') || rhs.starts_with("$\"") {
            Some("String".to_string())
        } else if rhs.starts_with('[') {
            Some("[Any]".to_string())
        } else if rhs.starts_with('{') {
            Some("Object".to_string())
        } else if rhs == "true" || rhs == "false" {
            Some("Bool".to_string())
        } else if rhs.parse::<i64>().is_ok() {
            Some("Int".to_string())
        } else if rhs.parse::<f64>().is_ok() {
            Some("Float".to_string())
        } else {
            None
        };

        if let Some(ty) = inferred_ty {
            push_or_update(&mut vars, name, ty, None);
        }
    }

    // Scan for loop variables: `for w in windows {`, `for w in window.list() {`, `for i in 0..10 {`
    let for_in_re =
        Regex::new(r"for\s+([a-zA-Z_]\w*)\s+in\s+([a-zA-Z0-9_.]+(?:\([^)]*\))?)").unwrap();
    for cap in for_in_re.captures_iter(content) {
        let item_name = cap[1].to_string();
        let iter_expr = cap[2].trim();

        let item_ty = if iter_expr == "window.list()" || iter_expr.ends_with(".list()") {
            Some("Window".to_string())
        } else if iter_expr.contains("..") {
            Some("Int".to_string())
        } else if let Some(v) = vars.iter().find(|v| v.name == iter_expr) {
            if let Some(ref t) = v.typ {
                if t.starts_with('[') && t.ends_with(']') {
                    Some(t[1..t.len() - 1].trim().to_string())
                } else if t.starts_with("Vec<") && t.ends_with('>') {
                    Some(t[4..t.len() - 1].trim().to_string())
                } else if t == "list" {
                    Some("Window".to_string())
                } else {
                    Some(t.clone())
                }
            } else {
                None
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

    // Scan for parameters in functions or closures (naive): `name: Type` where Type starts with uppercase
    let param_re = Regex::new(r"\b([a-z_]\w*)\s*:\s*([A-Z]\w*)").unwrap();
    for cap in param_re.captures_iter(content) {
        vars.push(ScannedVar {
            name: cap[1].to_string(),
            typ: Some(cap[2].to_string()),
            doc: None,
        });
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

    // Scan for function and annotation decls: `fn name(a: Type, b: Type)` or `annotation name(...) -> Ret`
    let fn_decl_re = Regex::new(
        r#"(?:@Docs\s*\(\s*(?:"([^"]*)"|'([^']*)')\s*\)\s*)?(?:export\s+)?(?:async\s+)?(fn|annotation)\s+([a-zA-Z_]\w*)\s*\(([^)]*)\)(?:\s*->\s*([a-zA-Z0-9_<>, \t]+))?"#,
    )
    .unwrap();

    let mut annotation_returns = std::collections::HashMap::new();
    for cap in fn_decl_re.captures_iter(content) {
        let doc_str = cap
            .get(1)
            .or_else(|| cap.get(2))
            .map(|m| m.as_str().to_string());
        let kind_kw = &cap[3];
        let name_str = &cap[4];
        let params_str = cap[5].trim();
        let ret_str = cap.get(6).map_or("()", |m| m.as_str().trim());

        let sig = if kind_kw == "annotation" {
            annotation_returns.insert(name_str.to_string(), ret_str.to_string());
            if ret_str == "()" {
                format!("annotation @{}({})", name_str, params_str)
            } else {
                format!("annotation @{}({}) -> {}", name_str, params_str, ret_str)
            }
        } else {
            if ret_str == "()" {
                format!("fn {}({})", name_str, params_str)
            } else {
                format!("fn {}({}) -> {}", name_str, params_str, ret_str)
            }
        };

        vars.push(ScannedVar {
            name: name_str.to_string(),
            typ: Some(sig),
            doc: doc_str,
        });
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

    (vars, structs)
}
