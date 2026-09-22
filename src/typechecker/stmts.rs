use std::collections::{HashMap, HashSet};
use crate::diagnostics::Diagnostic;
use crate::lexer::Span;
use crate::parser::*;
use super::checker::*;
use super::types::*;

impl TypeChecker {
    pub(crate) fn collect_top_level_declarations(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::StructDecl {
                    name,
                    fields,
                    annotations,
                    span: _,
                    name_span,
                } => {
                    let mut hover_str = format!("```flame\nstruct {}\n```", name);
                    let hover_doc = self.process_annotations(annotations);
                    if let Some(doc) = &hover_doc {
                        hover_str = format!("{}\n\n{}", hover_str, doc);
                    }
                    self.insert_hover_info(name_span.clone(), hover_str);

                    let is_builtin_file = self.filepath.ends_with("builtins.fm");
                    let platform = get_platform_annotation(annotations);

                    if !self.is_importing {
                        let mut is_dup = false;
                        if let Some(prev_decls) = self.defined_types_in_file.get(name) {
                            for (prev_kind, prev_plat) in prev_decls {
                                let duplicate = match (prev_plat.as_deref(), platform.as_deref()) {
                                    (Some(p1), Some(p2)) => p1 == p2,
                                    _ => true,
                                };
                                if duplicate {
                                    let msg = if prev_kind == "struct" {
                                        format!("Duplicate struct definition: '{}' is already defined", name)
                                    } else {
                                        format!("Duplicate type definition: '{}' is already defined as an enum", name)
                                    };
                                    self.diagnostics.push(crate::diagnostics::Diagnostic::new_error(
                                        msg,
                                        self.filepath.clone(),
                                        name_span.clone(),
                                        None,
                                        None,
                                    ));
                                    is_dup = true;
                                    break;
                                }
                            }
                        }

                        if !is_dup && !is_builtin_file {
                            if self.structs.contains_key(name) && !self.defined_types_in_file.contains_key(name) {
                                self.diagnostics.push(crate::diagnostics::Diagnostic::new_error(
                                    format!("Duplicate struct definition: '{}' is already defined as a built-in type", name),
                                    self.filepath.clone(),
                                    name_span.clone(),
                                    None,
                                    None,
                                ));
                            } else if self.enums.contains_key(name) && !self.defined_types_in_file.contains_key(name) {
                                self.diagnostics.push(crate::diagnostics::Diagnostic::new_error(
                                    format!("Duplicate type definition: '{}' is already defined as a built-in enum", name),
                                    self.filepath.clone(),
                                    name_span.clone(),
                                    None,
                                    None,
                                ));
                            }
                        }

                        self.defined_types_in_file
                            .entry(name.clone())
                            .or_default()
                            .push(("struct".to_string(), platform.clone()));
                    }

                    let parsed_fields = fields
                        .iter()
                        .map(|(field_name, type_name)| {
                            (field_name.clone(), self.parse_type_name(type_name))
                        })
                        .collect();

                    let active_os = std::env::consts::OS.to_lowercase();
                    let matches_active_os = platform
                        .as_ref()
                        .map(|p| active_os.contains(p) || p.contains(&active_os))
                        .unwrap_or(true);
                    if matches_active_os || !self.structs.contains_key(name) {
                        self.structs
                            .insert(name.clone(), StructInfo { fields: parsed_fields, hover_doc });
                    }
                }
                Stmt::EnumDecl {
                    name,
                    variants,
                    annotations,
                    span: _,
                    name_span,
                } => {
                    let mut hover_str = format!("```flame\nenum {}\n```", name);
                    let hover_doc = self.process_annotations(annotations);
                    if let Some(doc) = &hover_doc {
                        hover_str = format!("{}\n\n{}", hover_str, doc);
                    }
                    self.insert_hover_info(name_span.clone(), hover_str);

                    let is_builtin_file = self.filepath.ends_with("builtins.fm");
                    let platform = get_platform_annotation(annotations);

                    if !self.is_importing {
                        let mut is_dup = false;
                        if let Some(prev_decls) = self.defined_types_in_file.get(name) {
                            for (prev_kind, prev_plat) in prev_decls {
                                let duplicate = match (prev_plat.as_deref(), platform.as_deref()) {
                                    (Some(p1), Some(p2)) => p1 == p2,
                                    _ => true,
                                };
                                if duplicate {
                                    let msg = if prev_kind == "enum" {
                                        format!("Duplicate enum definition: '{}' is already defined", name)
                                    } else {
                                        format!("Duplicate type definition: '{}' is already defined as a struct", name)
                                    };
                                    self.diagnostics.push(crate::diagnostics::Diagnostic::new_error(
                                        msg,
                                        self.filepath.clone(),
                                        name_span.clone(),
                                        None,
                                        None,
                                    ));
                                    is_dup = true;
                                    break;
                                }
                            }
                        }

                        if !is_dup && !is_builtin_file {
                            if self.enums.contains_key(name) && !self.defined_types_in_file.contains_key(name) {
                                self.diagnostics.push(crate::diagnostics::Diagnostic::new_error(
                                    format!("Duplicate enum definition: '{}' is already defined as a built-in type", name),
                                    self.filepath.clone(),
                                    name_span.clone(),
                                    None,
                                    None,
                                ));
                            } else if self.structs.contains_key(name) && !self.defined_types_in_file.contains_key(name) {
                                self.diagnostics.push(crate::diagnostics::Diagnostic::new_error(
                                    format!("Duplicate type definition: '{}' is already defined as a built-in struct", name),
                                    self.filepath.clone(),
                                    name_span.clone(),
                                    None,
                                    None,
                                ));
                            }
                        }

                        self.defined_types_in_file
                            .entry(name.clone())
                            .or_default()
                            .push(("enum".to_string(), platform.clone()));
                    }

                    let mut map = HashMap::new();
                    for variant in variants {
                        match variant {
                            EnumVariant::Unit(variant_name) => {
                                map.insert(
                                    variant_name.clone(),
                                    VariantInfo {
                                        tuple_items: Vec::new(),
                                        struct_fields: Vec::new(),
                                        hover_doc: None,
                                    },
                                );
                            }
                            EnumVariant::Tuple(variant_name, items) => {
                                map.insert(
                                    variant_name.clone(),
                                    VariantInfo {
                                        tuple_items: items
                                            .iter()
                                            .map(|item| self.parse_type_name(item))
                                            .collect(),
                                        struct_fields: Vec::new(),
                                        hover_doc: None,
                                    },
                                );
                            }
                            EnumVariant::Struct(variant_name, fields) => {
                                map.insert(
                                    variant_name.clone(),
                                    VariantInfo {
                                        tuple_items: Vec::new(),
                                        struct_fields: fields
                                            .iter()
                                            .map(|(field_name, type_name)| {
                                                (
                                                    field_name.clone(),
                                                    self.parse_type_name(type_name),
                                                )
                                            })
                                            .collect(),
                                        hover_doc: None,
                                    },
                                );
                            }
                        }
                    }

                    let active_os = std::env::consts::OS.to_lowercase();
                    let matches_active_os = platform
                        .as_ref()
                        .map(|p| active_os.contains(p) || p.contains(&active_os))
                        .unwrap_or(true);
                    if matches_active_os || !self.enums.contains_key(name) {
                        self.enums.insert(
                            name.clone(),
                            EnumInfo {
                                variants: map,
                                hover_doc,
                            },
                        );
                    }
                }
                Stmt::FuncDecl {
                    name,
                    params,
                    return_type,
                    annotations,
                    span,
                    name_span,
                    ..
                } => {
                    let hover_doc = self.process_annotations(annotations);

                    let mut hover_str = format!("```flame\nfn {}(", name);
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 {
                            hover_str.push_str(", ");
                        }
                        let ref_mut = match (p.is_ref, p.is_mut) {
                            (true, true) => "ref mut ",
                            (true, false) => "ref ",
                            (false, true) => "mut ",
                            _ => "",
                        };
                        hover_str.push_str(&format!("{}{}: {}", ref_mut, p.name, p.type_name));
                    }
                    hover_str.push_str(")");
                    if let Some(ret) = return_type {
                        hover_str.push_str(&format!(" -> {}", ret));
                    }
                    hover_str.push_str("\n```");

                    if let Some(doc) = &hover_doc {
                        hover_str.push_str(&format!("\n\n{}", doc));
                    }

                    self.insert_hover_info(name_span.clone(), hover_str.clone());
                    if let Some(cmd_info) =
                        self.parse_command_annotation(name, annotations, params, span)
                    {
                        self.commands.insert(cmd_info.name.clone(), cmd_info);
                    }

                    let is_builtin_file = self.filepath.ends_with("builtins.fm")
                        || self.filepath.contains("Blaze/std/")
                        || self.filepath.contains("std/");
                    let platform = get_platform_annotation(annotations);
                    if !self.is_importing {
                        let mut is_dup = false;
                        if let Some(prev_plats) = self.defined_functions_in_file.get(name) {
                            for prev_plat in prev_plats {
                                let duplicate = match (prev_plat.as_deref(), platform.as_deref()) {
                                    (Some(p1), Some(p2)) => p1 == p2,
                                    _ => true,
                                };
                                if duplicate {
                                    self.diagnostics
                                        .push(crate::diagnostics::Diagnostic::new_error(
                                            format!(
                                                "Duplicate function definition: '{}' is already defined",
                                                name
                                            ),
                                            self.filepath.clone(),
                                            span.clone(),
                                            None,
                                            None,
                                        ));
                                    is_dup = true;
                                    break;
                                }
                            }
                        }
                        if !is_dup && !is_builtin_file && self.functions.contains_key(name) && !self.defined_functions_in_file.contains_key(name) {
                            self.diagnostics
                                .push(crate::diagnostics::Diagnostic::new_error(
                                    format!(
                                        "Duplicate function definition: '{}' is already defined",
                                        name
                                    ),
                                    self.filepath.clone(),
                                    span.clone(),
                                    None,
                                    None,
                                ));
                        }
                        self.defined_functions_in_file
                            .entry(name.clone())
                            .or_default()
                            .push(platform.clone());
                    }

                    let active_os = std::env::consts::OS.to_lowercase();
                    let matches_active_os = platform
                        .as_ref()
                        .map(|p| active_os.contains(p) || p.contains(&active_os))
                        .unwrap_or(true);
                    if matches_active_os || !self.functions.contains_key(name) {
                        self.functions.insert(
                            name.clone(),
                            FunctionSig {
                                is_static: false,
                                params: params
                                    .iter()
                                    .map(|param| ParamInfo {
                                        name: param.name.clone(),
                                        ty: self.parse_type_name(&param.type_name),
                                        is_ref: param.is_ref,
                                        is_mut: param.is_mut,
                                        has_default: param.default_val.is_some() || param.type_name.ends_with('?'),
                                    })
                                    .collect(),
                                hover_doc: hover_doc,
                                return_type: return_type
                                    .as_ref()
                                    .map(|ret| self.parse_type_name(ret))
                                    .unwrap_or(Type::Nil),
                            },
                        );
                    }
                }
                Stmt::PackageDecl { .. } => {}
                Stmt::AnnotationDecl {
                    name,
                    params,
                    return_type,
                    annotations,
                    span: _,
                    name_span,
                    ..
                } => {
                    let mut hover_str = format!("```flame\nannotation @{}(", name);
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 {
                            hover_str.push_str(", ");
                        }
                        let ref_mut = match (p.is_ref, p.is_mut) {
                            (true, true) => "ref mut ",
                            (true, false) => "ref ",
                            (false, true) => "mut ",
                            _ => "",
                        };
                        hover_str.push_str(&format!("{}{}: {}", ref_mut, p.name, p.type_name));
                    }
                    if let Some(ret) = return_type {
                        hover_str.push_str(&format!(") -> {}\n```", ret));
                    } else {
                        hover_str.push_str(")\n```");
                    }

                    let hover_doc = self.process_annotations(annotations);
                    if let Some(doc) = &hover_doc {
                        hover_str.push_str(&format!("\n\n{}", doc));
                    }
                    self.insert_hover_info(name_span.clone(), hover_str.clone());

                    self.annotations.insert(name.clone());
                    self.functions.insert(
                        name.clone(),
                        FunctionSig {
                            is_static: false,
                            params: params
                                .iter()
                                .map(|param| ParamInfo {
                                    name: param.name.clone(),
                                    ty: self.parse_type_name(&param.type_name),
                                    is_ref: param.is_ref,
                                    is_mut: param.is_mut,
                                    has_default: param.default_val.is_some() || param.type_name.ends_with('?'),
                                })
                                .collect(),
                            hover_doc: hover_doc,
                            return_type: return_type
                                .as_ref()
                                .map(|ret| self.parse_type_name(ret))
                                .unwrap_or(Type::Nil),
                        },
                    );
                }
                Stmt::ImplDecl {
                    target_type,
                    trait_name,
                    methods,
                    annotations,
                    span: _,
                    name_span,
                } => {
                    let mut hover_str = if let Some(tr) = &trait_name {
                        format!("```flame\nimpl {} for {}\n```", tr, target_type)
                    } else {
                        format!("```flame\nimpl {}\n```", target_type)
                    };
                    let hover_doc = self.process_annotations(annotations);
                    if let Some(doc) = hover_doc {
                        hover_str = format!("{}\n\n{}", hover_str, doc);
                    }
                    self.insert_hover_info(name_span.clone(), hover_str);
                    for method in methods {
                        if let Stmt::FuncDecl {
                            name,
                            params,
                            return_type,
                            annotations,
                            name_span,
                            ..
                        } = method
                        {
                            let mut m_hover_str = format!("```flame\nfn {}(", name);
                            let params_info: Vec<ParamInfo> = params
                                .iter()
                                .enumerate()
                                .map(|(i, param)| {
                                    if i > 0 {
                                        m_hover_str.push_str(", ");
                                    }
                                    let ref_mut = match (param.is_ref, param.is_mut) {
                                        (true, true) => "ref mut ",
                                        (true, false) => "ref ",
                                        (false, true) => "mut ",
                                        _ => "",
                                    };
                                    m_hover_str.push_str(&format!(
                                        "{}{}: {}",
                                        ref_mut, param.name, param.type_name
                                    ));
                                    ParamInfo {
                                        name: param.name.clone(),
                                        ty: self.parse_type_name(&param.type_name),
                                        is_ref: param.is_ref,
                                        is_mut: param.is_mut,
                                        has_default: param.default_val.is_some() || param.type_name.ends_with('?'),
                                    }
                                })
                                .collect();
                            m_hover_str.push_str(")");
                            if let Some(ret) = return_type {
                                m_hover_str.push_str(&format!(" -> {}", ret));
                            }
                            m_hover_str.push_str("\n```");

                            let ret_type = return_type
                                .as_ref()
                                .map(|ret| self.parse_type_name(ret))
                                .unwrap_or(Type::Nil);

                            let is_static = !params.first().map_or(false, |p| p.name == "self");
                            let hover_doc = self.process_annotations(annotations);
                            if let Some(doc) = &hover_doc {
                                m_hover_str.push_str(&format!("\n\n{}", doc));
                            }
                            self.insert_hover_info(name_span.clone(), m_hover_str);

                            self.methods.entry(target_type.clone()).or_default().insert(
                                name.clone(),
                                FunctionSig {
                                    is_static,
                                    params: params_info,
                                    hover_doc,
                                    return_type: ret_type,
                                },
                            );
                        }
                    }
                }
                Stmt::ImportDecl { path, alias, .. } => {
                    if let Some(mod_name) = path.last() {
                        let registered_name = alias.as_ref().unwrap_or(mod_name);
                        if path.first().map_or(false, |p| p == "native" || p == "std") {
                            self.plugins.insert(registered_name.clone());
                            self.modules.insert(registered_name.clone());
                            self.plugins.insert(mod_name.clone());
                            self.modules.insert(mod_name.clone());
                            if path.first().map_or(false, |p| p == "native") {
                                let mut methods = HashMap::new();
                                let mut p = std::path::Path::new(&self.filepath);
                                let mut fmi_path = None;
                                while let Some(parent) = p.parent() {
                                    let candidate = parent
                                        .join(".flame")
                                        .join("pkg")
                                        .join(mod_name)
                                        .join(format!("{}.fmi", mod_name));
                                    if candidate.exists() {
                                        fmi_path = Some(candidate);
                                        break;
                                    }
                                    p = parent;
                                }

                                if let Some(fmi) = fmi_path {
                                    if let Ok(content) = std::fs::read_to_string(&fmi) {
                                        if let Ok(json) =
                                            serde_json::from_str::<serde_json::Value>(&content)
                                        {
                                            let mut p_funcs = HashMap::new();
                                            if let Some(funcs) =
                                                json.get("functions").and_then(|f| f.as_array())
                                            {
                                                for func in funcs {
                                                    if let (Some(name), Some(ret_str)) = (
                                                        func.get("flame_name")
                                                            .and_then(|n| n.as_str())
                                                            .or(func
                                                                .get("name")
                                                                .and_then(|n| n.as_str())),
                                                        func.get("return_type")
                                                            .and_then(|r| r.as_str()),
                                                    ) {
                                                        let ret_ty = self.parse_type_name(ret_str);
                                                        methods.insert(
                                                            name.to_string(),
                                                            ret_ty.clone(),
                                                        );

                                                        let mut params = Vec::new();
                                                        if let Some(ps) = func
                                                            .get("params")
                                                            .and_then(|p| p.as_array())
                                                        {
                                                            for p in ps {
                                                                if let (
                                                                    Some(p_name),
                                                                    Some(p_ty_str),
                                                                ) = (
                                                                    p.get("name")
                                                                        .and_then(|n| n.as_str()),
                                                                    p.get("type_name")
                                                                        .and_then(|t| t.as_str()),
                                                                ) {
                                                                    params.push(ParamInfo {
                                                                        name: p_name.to_string(),
                                                                        ty: self.parse_type_name(
                                                                            p_ty_str,
                                                                        ),
                                                                        is_ref: p
                                                                            .get("is_ref")
                                                                            .and_then(|r| {
                                                                                r.as_bool()
                                                                            })
                                                                            .unwrap_or(false),
                                                                        is_mut: p
                                                                            .get("is_mut")
                                                                            .and_then(|m| {
                                                                                m.as_bool()
                                                                            })
                                                                            .unwrap_or(false),
                                                                        has_default: p_ty_str.ends_with('?') || p.get("default_val").is_some(),
                                                                    });
                                                                }
                                                            }
                                                        }

                                                        let doc = func
                                                            .get("docs")
                                                            .and_then(|d| d.as_str())
                                                            .map(|s| s.to_string());

                                                        p_funcs.insert(
                                                            name.to_string(),
                                                            FunctionSig {
                                                                is_static: true,
                                                                params,
                                                                return_type: ret_ty,
                                                                hover_doc: doc,
                                                            },
                                                        );
                                                    }
                                                }
                                            }
                                            self.plugin_functions.insert(mod_name.clone(), p_funcs);
                                            if let Some(structs) =
                                                json.get("structs").and_then(|s| s.as_array())
                                            {
                                                for s in structs {
                                                    if let Some(struct_name) =
                                                        s.get("name").and_then(|n| n.as_str())
                                                    {
                                                        methods.insert(
                                                            struct_name.to_string(),
                                                            Type::Struct(struct_name.to_string()),
                                                        );

                                                        self.structs.insert(struct_name.to_string(), StructInfo {
                                                            fields: Vec::new(),
                                                            hover_doc: Some("**Native Plugin Struct**".to_string()),
                                                        });

                                                        if let Some(s_methods) = s
                                                            .get("methods")
                                                            .and_then(|m| m.as_array())
                                                        {
                                                            let mut struct_methods = HashMap::new();
                                                            for m in s_methods {
                                                                if let (
                                                                    Some(m_name),
                                                                    Some(ret_str),
                                                                ) = (
                                                                    m.get("flame_name")
                                                                        .and_then(|n| n.as_str())
                                                                        .or(m
                                                                            .get("name")
                                                                            .and_then(|n| {
                                                                                n.as_str()
                                                                            })),
                                                                    m.get("return_type")
                                                                        .and_then(|r| r.as_str()),
                                                                ) {
                                                                    let is_static = m
                                                                        .get("is_static")
                                                                        .and_then(|s| s.as_bool())
                                                                        .unwrap_or(false);
                                                                    let mut params = Vec::new();
                                                                    if !is_static {
                                                                        params.push(ParamInfo {
                                                                            name: "self"
                                                                                .to_string(),
                                                                            ty: Type::Struct(
                                                                                struct_name
                                                                                    .to_string(),
                                                                            ),
                                                                            is_ref: true,
                                                                            is_mut: true,
                                                                            has_default: false,
                                                                        });
                                                                    }
                                                                    if let Some(ps) = m
                                                                        .get("params")
                                                                        .and_then(|p| p.as_array())
                                                                    {
                                                                        for p in ps {
                                                                            if let (Some(p_name), Some(p_ty_str)) = (p.get("name").and_then(|n| n.as_str()), p.get("type_name").and_then(|t| t.as_str())) {
                                                                                params.push(ParamInfo {
                                                                                    name: p_name.to_string(),
                                                                                    ty: self.parse_type_name(p_ty_str),
                                                                                    is_ref: p.get("is_ref").and_then(|r| r.as_bool()).unwrap_or(false),
                                                                                    is_mut: p.get("is_mut").and_then(|m| m.as_bool()).unwrap_or(false),
                                                                                    has_default: p_ty_str.ends_with('?') || p.get("default_val").is_some(),
                                                                                });
                                                                            }
                                                                        }
                                                                    }

                                                                    let doc = m
                                                                        .get("docs")
                                                                        .and_then(|d| d.as_str())
                                                                        .map(|s| s.to_string());
                                                                    struct_methods.insert(
                                                                        m_name.to_string(),
                                                                        FunctionSig {
                                                                            is_static,
                                                                            params,
                                                                            return_type: self
                                                                                .parse_type_name(
                                                                                    ret_str,
                                                                                ),
                                                                            hover_doc: doc,
                                                                        },
                                                                    );
                                                                }
                                                            }
                                                            self.methods.insert(
                                                                struct_name.to_string(),
                                                                struct_methods,
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Fallback to parsing native/src/lib.rs if .fmi not found
                                    let p = std::path::Path::new(&self.filepath);
                                    if let Some(parent) = p.parent() {
                                        if let Some(root) = parent.parent() {
                                            let lib_rs =
                                                root.join("native").join("src").join("lib.rs");
                                            if let Ok(content) = std::fs::read_to_string(&lib_rs) {
                                                for line in content.lines() {
                                                    let line = line.trim();
                                                    if line.starts_with("pub fn ") {
                                                        let rest = &line["pub fn ".len()..];
                                                        if let Some(paren) = rest.find('(') {
                                                            let name =
                                                                rest[..paren].trim().to_string();
                                                            let mut ret_ty = Type::Unknown;
                                                            if let Some(arrow) = rest.find("->") {
                                                                let after_arrow =
                                                                    rest[arrow + 2..].trim();
                                                                let end = after_arrow
                                                                    .find('{')
                                                                    .unwrap_or(after_arrow.len());
                                                                let ret_str =
                                                                    after_arrow[..end].trim();
                                                                if ret_str == "i64"
                                                                    || ret_str == "i32"
                                                                    || ret_str == "usize"
                                                                {
                                                                    ret_ty = Type::Int;
                                                                } else if ret_str == "f64"
                                                                    || ret_str == "f32"
                                                                {
                                                                    ret_ty = Type::Float;
                                                                } else if ret_str == "bool" {
                                                                    ret_ty = Type::Bool;
                                                                } else if ret_str == "String" {
                                                                    ret_ty = Type::String;
                                                                }
                                                            }
                                                            methods.insert(name, ret_ty);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                self.plugin_methods.insert(mod_name.clone(), methods);
                            }
                        } else {
                            self.modules.insert(mod_name.clone());
                            if let Some(file_path) = crate::stdlib::locate_import_file(
                                std::path::Path::new(&self.filepath),
                                path,
                            ) {
                                if let Ok(content) = std::fs::read_to_string(&file_path) {
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
                                        file_path.to_string_lossy().to_string(),
                                    );
                                    if let Ok(parsed_stmts) = parser.parse() {
                                        let prev = self.is_importing;
                                        self.is_importing = true;
                                        self.collect_top_level_declarations(&parsed_stmts);
                                        self.is_importing = prev;
                                    }
                                }
                            }
                        }
                    }
                }
                Stmt::ExportDecl(inner, _) => {
                    self.collect_top_level_declarations(std::slice::from_ref(inner.as_ref()));
                }
                _ => {}
            }
        }
    }

    pub(crate) fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::ImportDecl { path, alias, span, .. } => {
                if let Some(last) = path.last() {
                    let bind_name = alias.as_ref().unwrap_or(last);
                    let is_native = path.first().map_or(false, |p| p == "native");
                    let kind_str = if is_native {
                        format!("plugin:{}", last)
                    } else {
                        format!("module:{}", last)
                    };

                    let path_str = path.join(".");
                    let mut is_package = false;
                    let mut package_docs = None;
                    let mut suggestions = Vec::new();
                    if !path.first().map_or(false, |p| p == "native" || p == "std") {
                        if let Some(file_path) = crate::stdlib::locate_import_file(
                            std::path::Path::new(&self.filepath),
                            path,
                        ) {
                            let p_str = file_path.to_string_lossy();
                            if p_str.contains(".flame/pkg/") || p_str.contains(".flame\\pkg\\") {
                                is_package = true;
                                if let Ok(content) = std::fs::read_to_string(&file_path) {
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
                                        file_path.to_string_lossy().to_string(),
                                    );
                                    if let Ok(stmts) = parser.parse() {
                                        for stmt in stmts {
                                            if let crate::parser::Stmt::PackageDecl {
                                                annotations,
                                                ..
                                            } = stmt
                                            {
                                                package_docs =
                                                    self.process_annotations(&annotations);
                                                for ann in &annotations {
                                                    if ann.name == "Suggestions" {
                                                        if let Some(s) = ann.args.first() {
                                                            let s_trimmed = s.trim_matches(|c| {
                                                                c == '"' || c == '[' || c == ']'
                                                            });
                                                            let parts: Vec<&str> = s_trimmed
                                                                .split(',')
                                                                .map(|p| p.trim().trim_matches('"'))
                                                                .collect();
                                                            let name = parts[0].to_string();
                                                            let kind = if parts.len() > 1 {
                                                                parts[1].to_string()
                                                            } else {
                                                                "object".to_string()
                                                            };
                                                            suggestions.push((name, kind));
                                                        }
                                                    }
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let hover_str = if is_native {
                        format!("```flame\nimport {}\n```\n**Native Plugin**", path_str)
                    } else if path.first().map_or(false, |p| p == "std") {
                        format!(
                            "```flame\nimport {}\n```\n**Standard Library Module**",
                            path_str
                        )
                    } else if is_package {
                        let h = if let Some(ref docs) = package_docs {
                            format!("```flame\nimport package {}\n```\n\n{}", path_str, docs)
                        } else {
                            format!("```flame\nimport package {}\n```", path_str)
                        };
                        h
                    } else {
                        format!("```flame\nimport {}\n```\n**Local Module**", path_str)
                    };

                    let ty = if path.first().map_or(false, |p| p == "std") {
                        self.get_std_module_type(last)
                    } else if let Some((s, _)) = suggestions
                        .iter()
                        .find(|(_, k)| k == "object")
                        .or_else(|| suggestions.first())
                    {
                        Type::Named(s.clone())
                    } else {
                        Type::Named(kind_str)
                    };

                    let mut final_hover = hover_str.clone();
                    let mut final_ty = ty.clone();

                    if let Some(existing) = self.lookup_var(&last).cloned() {
                        let is_existing_native =
                            matches!(existing.ty, Type::Named(ref n) if n.starts_with("plugin:"));
                        let is_new_native = is_native;

                        if is_existing_native && !is_new_native {
                            final_ty = existing.ty.clone();
                            if is_package && package_docs.is_some() {
                                final_hover = format!(
                                    "```flame\nimport package {}\n```\n\n{}",
                                    path_str,
                                    package_docs.clone().unwrap()
                                );
                            } else if let Some(existing_doc) = existing.hover_doc {
                                final_hover = existing_doc;
                            }
                        } else if !is_existing_native && is_new_native {
                            if let Some(existing_doc) = existing.hover_doc {
                                final_hover = existing_doc;
                            }
                        }
                    }

                    self.insert_hover_info(span.clone(), final_hover.clone());
                    if let Some(first_part) = path.first() {
                        self.module_docs
                            .insert(first_part.to_string(), final_hover.clone());
                    }

                    self.define_var(
                        bind_name.clone(),
                        VarInfo {
                            ty: final_ty,
                            is_mut: false,
                            hover_doc: Some(final_hover),
                        },
                    );
                    let mut sources_to_parse = Vec::new();
                    if path.first().map_or(false, |p| p == "std") {
                        for part in path.iter().rev() {
                            let candidate = format!("{}.fm", part);
                            if let Some((_, src)) = crate::blaze::EMBEDDED_BLAZE_STD.iter().find(|(name, _)| *name == candidate) {
                                sources_to_parse.push((format!("<std::{}>", candidate), (*src).to_string()));
                                break;
                            }
                        }
                    } else if !path.first().map_or(false, |p| p == "native") {
                        if let Some(file_path) = crate::stdlib::locate_import_file(
                            std::path::Path::new(&self.filepath),
                            path,
                        ) {
                            let mut paths_to_read = Vec::new();
                            if file_path.is_dir() {
                                if let Ok(entries) = std::fs::read_dir(&file_path) {
                                    for entry in entries.flatten() {
                                        let p = entry.path();
                                        if p.is_file()
                                            && p.extension().and_then(|s| s.to_str()) == Some("fm")
                                        {
                                            paths_to_read.push(p);
                                        }
                                    }
                                }
                            } else {
                                paths_to_read.push(file_path);
                            }

                            for path_to_read in paths_to_read {
                                if let Ok(content) = std::fs::read_to_string(&path_to_read) {
                                    sources_to_parse.push((path_to_read.to_string_lossy().to_string(), content));
                                }
                            }
                        }
                    }

                    for (source_name, content) in sources_to_parse {
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
                            source_name,
                        );
                        if let Ok(parsed_stmts) = parser.parse() {
                            let prev = self.is_importing;
                            self.is_importing = true;
                            for s in &parsed_stmts {
                                let inner_stmt = if let Stmt::ExportDecl(inner, _) = s {
                                    inner.as_ref()
                                } else {
                                    s
                                };

                                match inner_stmt {
                                    Stmt::ImplDecl {
                                        target_type,
                                        methods,
                                        ..
                                    } => {
                                        let prefixed_target = format!("{}.{}", last, target_type);
                                        if !self.structs.contains_key(&prefixed_target) {
                                            self.structs.insert(
                                                prefixed_target.clone(),
                                                StructInfo {
                                                    fields: Vec::new(),
                                                    hover_doc: None,
                                                },
                                            );
                                        }
                                        if !self.structs.contains_key(target_type) {
                                            self.structs.insert(
                                                target_type.clone(),
                                                StructInfo {
                                                    fields: Vec::new(),
                                                    hover_doc: None,
                                                },
                                            );
                                        }
                                        for m in methods {
                                            if let Stmt::FuncDecl {
                                                name,
                                                params,
                                                return_type,
                                                annotations,
                                                ..
                                            } = m
                                            {
                                                let is_static = !params
                                                    .first()
                                                    .map_or(false, |p| p.name == "self");
                                                let p_info = params
                                                    .iter()
                                                    .map(|p| ParamInfo {
                                                        name: p.name.clone(),
                                                        ty: self.parse_type_name(&p.type_name),
                                                        is_ref: p.is_ref,
                                                        is_mut: p.is_mut,
                                                        has_default: p.default_val.is_some() || p.type_name.ends_with('?'),
                                                    })
                                                    .collect();
                                                let r_type = return_type
                                                    .as_ref()
                                                    .map(|t| self.parse_type_name(t))
                                                    .unwrap_or(Type::Nil);
                                                let m_hover_doc = self.process_annotations(annotations);
                                                let sig = FunctionSig {
                                                    params: p_info,
                                                    return_type: r_type,
                                                    is_static,
                                                    hover_doc: m_hover_doc,
                                                };
                                                self.methods
                                                    .entry(prefixed_target.clone())
                                                    .or_default()
                                                    .insert(name.clone(), sig.clone());
                                                self.methods
                                                    .entry(target_type.clone())
                                                    .or_default()
                                                    .insert(name.clone(), sig);
                                            }
                                        }
                                    }
                                    Stmt::StructDecl {
                                        name,
                                        fields,
                                        annotations,
                                        ..
                                    } => {
                                        let hover_doc = self.process_annotations(annotations);
                                        let mut struct_fields = Vec::new();
                                        for (f_name, f_type) in fields {
                                            struct_fields.push((
                                                f_name.clone(),
                                                self.parse_type_name(f_type),
                                            ));
                                        }
                                        let s_info = StructInfo {
                                            fields: struct_fields,
                                            hover_doc,
                                        };
                                        self.structs.insert(name.clone(), s_info.clone());
                                        self.structs.insert(format!("{}.{}", last, name), s_info);
                                    }
                                    Stmt::FuncDecl {
                                        name,
                                        params,
                                        return_type,
                                        annotations,
                                        ..
                                    } => {
                                        let hover_doc = self.process_annotations(annotations);
                                        let p_info = params
                                            .iter()
                                            .map(|p| ParamInfo {
                                                name: p.name.clone(),
                                                ty: self.parse_type_name(&p.type_name),
                                                is_ref: p.is_ref,
                                                is_mut: p.is_mut,
                                                has_default: p.default_val.is_some() || p.type_name.ends_with('?'),
                                            })
                                            .collect();
                                        let r_type = return_type
                                            .as_ref()
                                            .map(|t| self.parse_type_name(t))
                                            .unwrap_or(Type::Nil);
                                        let sig = FunctionSig {
                                            params: p_info,
                                            return_type: r_type,
                                            is_static: false,
                                            hover_doc,
                                        };
                                        self.functions.insert(name.clone(), sig.clone());
                                        self.functions.insert(format!("{}.{}", last, name), sig);
                                    }
                                    Stmt::LetDecl {
                                        name, annotations, ..
                                    }
                                    | Stmt::ConstDecl {
                                        name, annotations, ..
                                    } => {
                                        let hover_doc = self.process_annotations(annotations);
                                        self.define_var(
                                            format!("{}.{}", last, name),
                                            VarInfo {
                                                ty: Type::Unknown,
                                                is_mut: false,
                                                hover_doc: hover_doc.clone(),
                                            },
                                        );
                                        self.define_var(
                                            name.clone(),
                                            VarInfo {
                                                ty: Type::Unknown,
                                                is_mut: false,
                                                hover_doc,
                                            },
                                        );
                                    }
                                    Stmt::EnumDecl {
                                        name,
                                        variants,
                                        annotations,
                                        ..
                                    } => {
                                        let hover_doc = self.process_annotations(annotations);
                                        let mut enum_variants = HashMap::new();
                                        for var in variants {
                                            match var {
                                                crate::parser::EnumVariant::Unit(n) => {
                                                    enum_variants.insert(n.clone(), VariantInfo {
                                                        tuple_items: vec![],
                                                        struct_fields: Vec::new(),
                                                        hover_doc: None,
                                                    });
                                                }
                                                crate::parser::EnumVariant::Tuple(n, items) => {
                                                    enum_variants.insert(n.clone(), VariantInfo {
                                                        tuple_items: items.iter().map(|item| self.parse_type_name(item)).collect(),
                                                        struct_fields: Vec::new(),
                                                        hover_doc: None,
                                                    });
                                                }
                                                crate::parser::EnumVariant::Struct(n, fields) => {
                                                    let mut struct_fields = Vec::new();
                                                    for (f_name, f_type) in fields {
                                                        struct_fields.push((f_name.clone(), self.parse_type_name(f_type)));
                                                    }
                                                    enum_variants.insert(n.clone(), VariantInfo {
                                                        tuple_items: vec![],
                                                        struct_fields,
                                                        hover_doc: None,
                                                    });
                                                }
                                            }
                                        }
                                        let e_info = EnumInfo {
                                            variants: enum_variants,
                                            hover_doc,
                                        };
                                        self.enums.insert(name.clone(), e_info.clone());
                                        self.enums.insert(format!("{}.{}", last, name), e_info);
                                    }
                                    _ => {}
                                }
                            }
                            self.is_importing = prev;
                        }
                    }
                }
            }
            Stmt::ExportDecl(inner, _) => self.check_stmt(inner),
            Stmt::LetDecl {
                name,
                is_mut,
                type_ann,
                value,
                annotations,
                span,
                name_span,
                ..
            }
            | Stmt::ConstDecl {
                name,
                is_mut,
                type_ann,
                value,
                annotations,
                span,
                name_span,
                ..
            } => {
                let declared_ty = type_ann
                    .as_ref()
                    .map(|type_name| self.parse_type_name(type_name));

                let prev_expected = self.expected_closure_type.take();
                if let Some(ref d_ty) = declared_ty {
                    self.expected_closure_type = Some(d_ty.clone());
                }
                let value_ty = self.infer_expr_type(value);
                self.expected_closure_type = prev_expected;

                if let Some(expected) = &declared_ty {
                    self.expect_assignable(expected, &value_ty, span, "variable initializer");
                }

                if !name.starts_with('{') && !name.starts_with('(') {
                    if let Some(scope) = self.scopes.last() {
                        if scope.contains_key(name) {
                            self.error(
                                format!("cannot redeclare variable '{}' in the same scope", name),
                                name_span.clone(),
                                Some(format!("'{}' already declared in this scope", name)),
                                Some(format!("reassign to '{}' without 'let' or rename the variable", name)),
                            );
                        }
                    }
                    let stored_ty = match (&declared_ty, &value_ty) {
                        (Some(Type::Formula(_, _)), Type::Formula(map, _)) => {
                            Type::Formula(map.clone(), HashMap::new())
                        }
                        (Some(Type::Enum(expected)), Type::EnumVariant { enum_name, .. })
                            if expected == enum_name =>
                        {
                            value_ty.clone()
                        }
                        (Some(expected), _) => expected.clone(),
                        (None, Type::EnumVariant { enum_name, .. }) => {
                            Type::Enum(enum_name.clone())
                        }
                        (None, ty) => ty.clone(),
                    };

                    let type_str = match &stored_ty {
                        Type::Named(n) => n.clone(),
                        Type::Int => "Int".to_string(),
                        Type::Float => "Float".to_string(),
                        Type::String => "String".to_string(),
                        Type::Bool => "Bool".to_string(),
                        Type::Nil => "Nil".to_string(),
                        t => format!("{:?}", t),
                    };
                    let decl_kw = if matches!(stmt, Stmt::ConstDecl { .. }) {
                        "const"
                    } else if *is_mut {
                        "let mut"
                    } else {
                        "let"
                    };
                    let mut hover_str =
                        format!("```flame\n{} {}: {}\n```", decl_kw, name, type_str);

                    let hover_doc = self.process_annotations(annotations);

                    if let Some(doc) = hover_doc {
                        hover_str = format!("{}\n\n{}", hover_str, doc);
                    }

                    self.insert_hover_info(name_span.clone(), hover_str.clone());

                    self.define_var(
                        name.clone(),
                        VarInfo {
                            ty: stored_ty,
                            is_mut: *is_mut,
                            hover_doc: Some(hover_str.clone()),
                        },
                    );
                } else if name.starts_with('(') && name.ends_with(')') {
                    // It's a tuple destructuring assignment, e.g. "(tx, rx)" or "(b: 1, c: 2)"
                    let inner_names = name[1..name.len() - 1]
                        .split(',')
                        .map(|s| s.trim())
                        .collect::<Vec<_>>();

                    if let Type::Tuple(types) = &value_ty {
                        for (i, inner_name) in inner_names.iter().enumerate() {
                            if inner_name.is_empty() || *inner_name == "_" {
                                continue;
                            }
                            // Handle "(b: 1)" style destructuring
                            let actual_name = inner_name.split(':').next().unwrap().trim();
                            if let Some(scope) = self.scopes.last() {
                                if scope.contains_key(actual_name) {
                                    self.error(
                                        format!("cannot redeclare variable '{}' in the same scope", actual_name),
                                        name_span.clone(),
                                        Some(format!("'{}' already declared in this scope", actual_name)),
                                        None,
                                    );
                                }
                            }
                            let v_ty = types.get(i).cloned().unwrap_or(Type::Unknown);
                            self.define_var(
                                actual_name.to_string(),
                                VarInfo {
                                    ty: v_ty,
                                    is_mut: *is_mut,
                                    hover_doc: None,
                                },
                            );
                        }
                    } else {
                        // Fallback: If it's an unknown type or not a tuple
                        for inner_name in inner_names {
                            if inner_name.is_empty() || inner_name == "_" {
                                continue;
                            }
                            let actual_name = inner_name.split(':').next().unwrap().trim();
                            self.define_var(
                                actual_name.to_string(),
                                VarInfo {
                                    ty: value_ty.clone(),
                                    is_mut: *is_mut,
                                    hover_doc: None,
                                },
                            );
                        }
                    }
                } else if name.starts_with('{') && name.ends_with('}') {
                    // It's an object destructuring assignment, e.g. "{status, data}"
                    let inner_names = name
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect::<Vec<_>>();

                    if let Type::Formula(map, _) = &value_ty {
                        for v_name in &inner_names {
                            if v_name != "_" {
                                if let Some(scope) = self.scopes.last() {
                                    if scope.contains_key(v_name) {
                                        self.error(
                                            format!("cannot redeclare variable '{}' in the same scope", v_name),
                                            name_span.clone(),
                                            Some(format!("'{}' already declared in this scope", v_name)),
                                            None,
                                        );
                                    }
                                }
                                let v_ty = map.get(v_name).cloned().unwrap_or(Type::Unknown);
                                self.define_var(
                                    v_name.clone(),
                                    VarInfo {
                                        ty: v_ty,
                                        is_mut: *is_mut,
                                        hover_doc: None,
                                    },
                                );
                            }
                        }
                    } else if let Type::Struct(struct_name) = &value_ty {
                        for v_name in &inner_names {
                            if v_name != "_" {
                                if let Some(scope) = self.scopes.last() {
                                    if scope.contains_key(v_name) {
                                        self.error(
                                            format!("cannot redeclare variable '{}' in the same scope", v_name),
                                            name_span.clone(),
                                            Some(format!("'{}' already declared in this scope", v_name)),
                                            None,
                                        );
                                    }
                                }
                                let mut field_ty = Type::Unknown;
                                if let Some(info) = self.structs.get(struct_name) {
                                    if let Some((_, ty)) =
                                        info.fields.iter().find(|(n, _)| n == v_name)
                                    {
                                        field_ty = ty.clone();
                                    }
                                }
                                self.define_var(
                                    v_name.clone(),
                                    VarInfo {
                                        ty: field_ty,
                                        is_mut: *is_mut,
                                        hover_doc: None,
                                    },
                                );
                            }
                        }
                    } else {
                        // Fallback
                        for v_name in inner_names {
                            if v_name != "_" {
                                self.define_var(
                                    v_name.clone(),
                                    VarInfo {
                                        ty: Type::Unknown,
                                        is_mut: *is_mut,
                                        hover_doc: None,
                                    },
                                );
                            }
                        }
                    }
                }
            }
            Stmt::FuncDecl {
                name,
                params,
                return_type,
                body,
                annotations,
                span: _span,
                name_span,
                ..
            } => {
                let func_type = Type::Function(
                    params
                        .iter()
                        .map(|param| self.parse_type_name(&param.type_name))
                        .collect(),
                    Box::new(
                        return_type
                            .as_ref()
                            .map(|ret| self.parse_type_name(ret))
                            .unwrap_or(Type::Nil),
                    ),
                );
                let params_str = params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.type_name))
                    .collect::<Vec<_>>()
                    .join(", ");
                let ret_str = if let Some(ret) = &return_type {
                    format!(" -> {}", ret)
                } else {
                    "".to_string()
                };
                let mut hover_str =
                    format!("```flame\nfn {}({}){}\n```", name, params_str, ret_str);
                let hover_doc = self.process_annotations(annotations);

                if let Some(doc) = hover_doc {
                    hover_str = format!("{}\n\n{}", hover_str, doc);
                }

                self.insert_hover_info(name_span.clone(), hover_str.clone());

                self.define_var(
                    name.clone(),
                    VarInfo {
                        ty: func_type,
                        is_mut: false,
                        hover_doc: Some(hover_str.clone()),
                    },
                );

                let prev_return = self.current_return_type.clone();
                self.current_return_type = Some(
                    return_type
                        .as_ref()
                        .map(|ret| self.parse_type_name(ret))
                        .unwrap_or(Type::Nil),
                );

                self.push_scope();
                for anno in annotations {
                    if anno.name == "Requires" {
                        for arg in &anno.args {
                            if arg.starts_with('"') && arg.ends_with('"') {
                                let mod_name = arg[1..arg.len() - 1].to_string();
                                let parts: Vec<String> =
                                    mod_name.split('.').map(|s| s.to_string()).collect();
                                self.check_stmt(&Stmt::ImportDecl {
                                    path: parts,
                                    glob: false,
                                    alias: None,
                                    span: anno.span.clone(),
                                });
                            }
                        }
                    }

                    let mut ret_ty = if let Some(sig) = self.functions.get(&anno.name) {
                        sig.return_type.clone()
                    } else if let Some(funcs) = self.plugin_functions.get(&anno.name.to_lowercase())
                    {
                        if let Some(sig) = funcs.get("init") {
                            sig.return_type.clone()
                        } else {
                            Type::Unknown
                        }
                    } else {
                        Type::Unknown
                    };
                    if let Type::Named(name) = &ret_ty {
                        if self.structs.contains_key(name) {
                            ret_ty = Type::Struct(name.clone());
                        }
                    }
                    self.define_var(
                        anno.name.clone(),
                        VarInfo {
                            ty: ret_ty.clone(),
                            is_mut: true,
                            hover_doc: None,
                        },
                    );
                    self.define_var(
                        anno.name.to_lowercase(),
                        VarInfo {
                            ty: ret_ty,
                            is_mut: true,
                            hover_doc: None,
                        },
                    );
                }
                for Param {
                    name,
                    type_name,
                    is_mut,
                    ..
                } in params
                {
                    self.define_var(
                        name.clone(),
                        VarInfo {
                            ty: self.parse_type_name(type_name),
                            is_mut: *is_mut,
                            hover_doc: None,
                        },
                    );
                }
                if let Some(body_stmts) = body {
                    for stmt in body_stmts {
                        self.check_stmt(stmt);
                    }
                }
                self.pop_scope();
                self.current_return_type = prev_return;
            }
            Stmt::AnnotationDecl {
                name,
                params,
                return_type,
                body,
                annotations,
                span: _span,
                name_span,
            } => {
                if let Some(first_char) = name.chars().next() {
                    if !first_char.is_uppercase() {
                        self.diagnostics.push(Diagnostic::new_error(
                            format!("Annotation names should start with an uppercase letter, found '{}'", name),
                            self.filepath.clone(),
                            name_span.clone(),
                            None,
                            None
                        ));
                    }
                }
                let func_type = Type::Function(
                    params
                        .iter()
                        .map(|param| self.parse_type_name(&param.type_name))
                        .collect(),
                    Box::new(
                        return_type
                            .as_ref()
                            .map(|ret| self.parse_type_name(ret))
                            .unwrap_or(Type::Nil),
                    ),
                );

                let params_str = params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.type_name))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut hover_str = if let Some(ret) = &return_type {
                    format!(
                        "```flame\nannotation @{}({}) -> {}\n```",
                        name, params_str, ret
                    )
                } else {
                    format!("```flame\nannotation @{}({})\n```", name, params_str)
                };
                let hover_doc = self.process_annotations(annotations);

                if let Some(doc) = hover_doc {
                    hover_str = format!("{}\n\n{}", hover_str, doc);
                }

                self.insert_hover_info(name_span.clone(), hover_str);

                self.define_var(
                    name.clone(),
                    VarInfo {
                        ty: func_type,
                        is_mut: false,
                        hover_doc: None,
                    },
                );

                let prev_return = self.current_return_type.clone();
                self.current_return_type = Some(
                    return_type
                        .as_ref()
                        .map(|ret| self.parse_type_name(ret))
                        .unwrap_or(Type::Nil),
                );

                self.push_scope();
                for Param {
                    name,
                    type_name,
                    is_mut,
                    ..
                } in params
                {
                    self.define_var(
                        name.clone(),
                        VarInfo {
                            ty: self.parse_type_name(type_name),
                            is_mut: *is_mut,
                            hover_doc: None,
                        },
                    );
                }
                for stmt in body {
                    self.check_stmt(stmt);
                }
                self.pop_scope();
                self.current_return_type = prev_return;
            }
            Stmt::IfStmt {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_ty = self.infer_expr_type(cond);
                self.expect_assignable(&Type::Bool, &cond_ty, &cond.span(), "if condition");

                self.push_scope();
                for stmt in then_branch {
                    self.check_stmt(stmt);
                }
                self.pop_scope();

                if let Some(else_branch) = else_branch {
                    self.push_scope();
                    for stmt in else_branch {
                        self.check_stmt(stmt);
                    }
                    self.pop_scope();
                }
            }
            Stmt::WhileStmt { cond, body, .. } => {
                let cond_ty = self.infer_expr_type(cond);
                self.expect_assignable(&Type::Bool, &cond_ty, &cond.span(), "while condition");
                self.push_scope();
                for stmt in body {
                    self.check_stmt(stmt);
                }
                self.pop_scope();
            }
            Stmt::ForStmt {
                var_name,
                iterable,
                body,
                ..
            } => {
                let item_ty = match self.infer_expr_type(iterable) {
                    Type::Tuple(items) => items.first().cloned().unwrap_or(Type::Unknown),
                    Type::Vector(item) => (*item).clone(),
                    _ => Type::Unknown,
                };
                self.push_scope();
                self.define_var(
                    var_name.clone(),
                    VarInfo {
                        ty: item_ty,
                        is_mut: false,
                        hover_doc: None,
                    },
                );
                for stmt in body {
                    self.check_stmt(stmt);
                }
                self.pop_scope();
            }
            Stmt::LoopStmt { body, .. } => {
                self.push_scope();
                for stmt in body {
                    self.check_stmt(stmt);
                }
                self.pop_scope();
            }
            Stmt::ReturnStmt(value, span) => {
                let actual = value
                    .as_ref()
                    .map(|expr| self.infer_expr_type(expr))
                    .unwrap_or(Type::Nil);
                if let Some(expected) = self.current_return_type.clone() {
                    self.expect_assignable(&expected, &actual, span, "return value");
                }
            }
            Stmt::ExprStmt(expr) => {
                self.infer_expr_type(expr);
            }
            Stmt::DeferStmt(inner, _) => self.check_stmt(inner),
            Stmt::MatchStmt {
                target,
                arms,
                span: _,
            } => {
                let target_ty = self.infer_expr_type(target);
                let is_cli_match = match &target_ty {
                    Type::Named(name) => name == "Cli",
                    Type::Struct(name) => name == "Cli",
                    _ => match target {
                        Expr::Identifier(id, _) => id == "cli",
                        _ => false,
                    },
                };

                if is_cli_match {
                    for arm in arms {
                        let is_wildcard = arm.patterns.iter().any(|p| p == "_");
                        let is_help = arm.patterns.iter().any(|p| p == "help");
                        let cmd_match = arm
                            .patterns
                            .iter()
                            .find_map(|p| self.commands.get(p).map(|c| (p, c.clone())));

                        if is_wildcard {
                            let wildcard_doc = "```flame\n_ => ...\n```\n**Wildcard Match Arm**\nMatches any unrecognized CLI command.".to_string();
                            self.insert_hover_info(arm.pattern_span.clone(), wildcard_doc);
                            self.push_scope();
                            self.infer_expr_type(&arm.body);
                            self.pop_scope();
                        } else if is_help {
                            let help_doc = if let Some(cmd) = self.commands.get("help") {
                                cmd.hover_doc.clone()
                            } else {
                                "```flame\n@Command(name: \"help\", about: \"Print help message\")\n```\n**CLI Subcommand**: `help`\n\nPrint help message".to_string()
                            };
                            self.insert_hover_info(arm.pattern_span.clone(), help_doc);
                            self.push_scope();
                            self.infer_expr_type(&arm.body);
                            self.pop_scope();
                        } else if let Some((_pat, cmd)) = cmd_match {
                            self.insert_hover_info(arm.pattern_span.clone(), cmd.hover_doc.clone());
                            self.push_scope();
                            for field in &arm.destructure {
                                if let Some(param) = cmd.params.iter().find(|p| &p.name == field) {
                                    self.define_var(
                                        field.clone(),
                                        VarInfo {
                                            ty: param.ty.clone(),
                                            is_mut: false,
                                            hover_doc: None,
                                        },
                                    );
                                } else {
                                    self.define_var(
                                        field.clone(),
                                        VarInfo {
                                            ty: Type::Unknown,
                                            is_mut: false,
                                            hover_doc: None,
                                        },
                                    );
                                }
                            }
                            self.infer_expr_type(&arm.body);
                            self.pop_scope();
                        } else {
                            // Command was not defined with @Command -> emit error!
                            let pat = arm.patterns.first().unwrap_or(&String::new()).clone();
                            self.error(
                                format!("unknown command '{}': no matching function annotated with @Command(name: \"{}\") was found", pat, pat),
                                arm.pattern_span.clone(),
                                Some(format!("unknown command '{}'", pat)),
                                Some(format!("define a function annotated with '@Command(name: \"{}\")' to handle this command", pat)),
                            );
                            self.push_scope();
                            for field in &arm.destructure {
                                self.define_var(
                                    field.clone(),
                                    VarInfo {
                                        ty: Type::Unknown,
                                        is_mut: false,
                                        hover_doc: None,
                                    },
                                );
                            }
                            self.infer_expr_type(&arm.body);
                            self.pop_scope();
                        }
                    }
                } else {
                    for arm in arms {
                        self.push_scope();
                        for field in &arm.destructure {
                            self.define_var(
                                field.clone(),
                                VarInfo {
                                    ty: Type::Unknown,
                                    is_mut: false,
                                    hover_doc: None,
                                },
                            );
                        }
                        self.infer_expr_type(&arm.body);
                        self.pop_scope();
                    }
                }
            }
            Stmt::StructDecl { .. }
            | Stmt::EnumDecl { .. }
            | Stmt::TraitDecl { .. }
            | Stmt::ImplDecl { .. }
            | Stmt::Break(_)
            | Stmt::Continue(_)
            | Stmt::PackageDecl { .. }
            | Stmt::PluginDecl { .. } => {}
        }
    }


}
