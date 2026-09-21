use std::collections::{HashMap, HashSet};
use crate::diagnostics::Diagnostic;
use crate::lexer::Span;
use crate::parser::*;
use super::checker::*;
use super::types::*;

impl TypeChecker {
    pub(crate) fn check_call_args(
        &mut self,
        params: &[ParamInfo],
        args: &[(Option<String>, Expr)],
        span: &Span,
        name: &str,
    ) {
        let min_args = params.iter().filter(|p| !p.has_default).count();
        let max_args = params.len();
        if name != "print" && name != "eprint" && (args.len() < min_args || args.len() > max_args) {
            self.error(
                if min_args == max_args {
                    format!(
                        "function '{}' expects {} argument(s), got {}",
                        name,
                        params.len(),
                        args.len()
                    )
                } else {
                    format!(
                        "function '{}' expects between {} and {} arguments, got {}",
                        name,
                        min_args,
                        max_args,
                        args.len()
                    )
                },
                span.clone(),
                None,
                None,
            );
        }

        for (idx, (_, arg)) in args.iter().enumerate() {
            let prev_expected = self.expected_closure_type.take();
            if let Some(param) = params.get(idx) {
                let ty = match &param.ty {
                    Type::Reference { inner, .. } => (**inner).clone(),
                    other => other.clone(),
                };
                self.expected_closure_type = Some(ty);
            }
            let actual = self.infer_expr_type(arg);
            self.expected_closure_type = prev_expected;
            if name == "print" || name == "eprint" {
                continue;
            }
            if let Some(param) = params.get(idx) {
                if param.is_ref {
                    if matches!(actual, Type::String) && matches!(param.ty, Type::String) {
                        self.expect_assignable(
                            &param.ty,
                            &actual,
                            &arg.span(),
                            "function argument (string by value to ref)",
                        );
                    } else {
                        if param.is_mut {
                            if let Type::Reference { mutable, .. } = &actual {
                                if !mutable {
                                    self.error(
                                        format!(
                                            "parameter '{}' requires '&mut' but argument is not mutable",
                                            param.name
                                        ),
                                        arg.span(),
                                        None,
                                        None,
                                    );
                                }
                            }
                        }
                        self.expect_assignable(
                            &param.ty,
                            &actual,
                            &arg.span(),
                            "function argument (by reference)",
                        );
                    }
                } else {
                    self.expect_assignable(&param.ty, &actual, &arg.span(), "function argument");
                }
            }
        }
    }

    pub(crate) fn check_struct_constructor_args(
        &mut self,
        name: &str,
        struct_info: &StructInfo,
        args: &[(Option<String>, Expr)],
        span: &Span,
    ) {
        let named = args.iter().any(|(arg_name, _)| arg_name.is_some());
        if named {
            for (field_name, field_ty) in &struct_info.fields {
                if let Some((_, expr)) = args.iter().find(|(arg_name, _)| {
                    arg_name.as_ref().map(|n| n == field_name).unwrap_or(false)
                }) {
                    let actual = self.infer_expr_type(expr);
                    self.expect_assignable(
                        field_ty,
                        &actual,
                        &expr.span(),
                        "struct field initializer",
                    );
                } else {
                    self.error(
                        format!("missing field '{}' for struct '{}'", field_name, name),
                        span.clone(),
                        None,
                        None,
                    );
                }
            }
            for (arg_name, expr) in args {
                if let Some(arg_name) = arg_name {
                    if !struct_info
                        .fields
                        .iter()
                        .any(|(field_name, _)| field_name == arg_name)
                    {
                        self.error(
                            format!("unknown field '{}' for struct '{}'", arg_name, name),
                            expr.span(),
                            None,
                            None,
                        );
                    }
                }
            }
        } else {
            if args.len() != struct_info.fields.len() {
                self.error(
                    format!(
                        "struct '{}' expects {} argument(s), got {}",
                        name,
                        struct_info.fields.len(),
                        args.len()
                    ),
                    span.clone(),
                    None,
                    None,
                );
            }
            for (idx, (_, expr)) in args.iter().enumerate() {
                if let Some((_, expected)) = struct_info.fields.get(idx) {
                    let actual = self.infer_expr_type(expr);
                    self.expect_assignable(
                        expected,
                        &actual,
                        &expr.span(),
                        "struct constructor argument",
                    );
                }
            }
        }
    }

    pub(crate) fn expect_assignable(&mut self, expected: &Type, actual: &Type, span: &Span, context: &str) {
        if !self.is_compatible(expected, actual) {
            self.error(
                format!(
                    "type mismatch in {}: expected {}, found {}",
                    context,
                    self.format_type(expected),
                    self.format_type(actual)
                ),
                span.clone(),
                None,
                None,
            );
        }
    }

    pub(crate) fn is_compatible(&self, expected: &Type, actual: &Type) -> bool {
        if let Type::Union(expected_types) = expected {
            if let Type::Union(actual_types) = actual {
                return actual_types.iter().all(|act| expected_types.iter().any(|exp| self.is_compatible(exp, act)));
            }
            return expected_types.iter().any(|t| self.is_compatible(t, actual));
        }
        if let Type::Union(actual_types) = actual {
            return actual_types.iter().all(|act| self.is_compatible(expected, act));
        }

        if let Type::Nullable(inner) = expected {
            if matches!(actual, Type::Nil) {
                return true;
            }
            if let Type::Nullable(actual_inner) = actual {
                return self.is_compatible(inner, actual_inner);
            }
            return self.is_compatible(inner, actual);
        }
        if let Type::Nullable(actual_inner) = actual {
            if let Type::Nullable(expected_inner) = expected {
                return self.is_compatible(expected_inner, actual_inner);
            }
            return false;
        }

        if matches!(expected, Type::Unknown) || matches!(actual, Type::Unknown) {
            return true;
        }
        if let Type::Named(name) = expected {
            if name.len() == 1 && name.chars().next().unwrap().is_uppercase() {
                return true;
            }
        }
        if expected == actual {
            return true;
        }
        // Allow Type::String to be assigned to &'staticstr
        if matches!(actual, Type::String) {
            if let Type::Reference { inner, .. } = expected {
                if let Type::Named(name) = &**inner {
                    if name == "'staticstr" || name == "'static str" {
                        return true;
                    }
                }
            } else if let Type::Named(name) = expected {
                if name == "&'staticstr" || name == "&'static str" {
                    return true;
                }
            }
        }

        // Fallback for cases where structural equality fails but formatted strings match.
        if self.format_type(expected) == self.format_type(actual) {
            return true;
        }

        match (expected, actual) {
            (Type::Float, Type::Int) => true,
            (Type::Byte, Type::Int) => true,
            (Type::Int, Type::Byte) => true,
            (Type::Enum(expected_name), Type::EnumVariant { enum_name, .. }) => {
                expected_name == enum_name
            }
            (Type::EnumVariant { enum_name: e1, .. }, Type::EnumVariant { enum_name: e2, .. }) => {
                e1 == e2
            }
            (Type::EnumVariant { enum_name, .. }, Type::Enum(actual_name)) => {
                enum_name == actual_name
            }
            (Type::Enum(e1), Type::Enum(e2)) => e1 == e2,
            (Type::Tuple(expected_items), Type::Function(a_params, _))
                if expected_items.is_empty() && a_params.is_empty() =>
            {
                true
            }
            (Type::Named(expected_name), Type::Struct(actual_name))
            | (Type::Named(expected_name), Type::Enum(actual_name)) => {
                expected_name == actual_name
                    || expected_name.starts_with(&format!("{}<", actual_name))
            }
            (Type::Struct(expected_name), Type::Named(actual_name))
            | (Type::Enum(expected_name), Type::Named(actual_name)) => {
                expected_name == actual_name
                    || actual_name.starts_with(&format!("{}<", expected_name))
            }
            (Type::Named(expected_name), Type::EnumVariant { enum_name, .. }) => {
                expected_name == enum_name || expected_name.starts_with(&format!("{}<", enum_name))
            }
            (Type::Byte, Type::Byte) => true,
            (Type::Vector(expected_item), Type::Vector(actual_item)) => {
                self.is_compatible(expected_item, actual_item)
            }
            (Type::Tuple(expected_items), Type::Tuple(actual_items)) => {
                expected_items.len() == actual_items.len()
                    && expected_items
                        .iter()
                        .zip(actual_items.iter())
                        .all(|(expected, actual)| self.is_compatible(expected, actual))
            }
            (
                Type::Reference {
                    inner: expected,
                    mutable: em,
                },
                Type::Reference {
                    inner: actual,
                    mutable: am,
                },
            ) => em == am && self.is_compatible(expected, actual),
            (Type::Reference { inner, .. }, actual) => self.is_compatible(inner, actual),
            (expected, Type::Reference { inner, .. }) => self.is_compatible(expected, inner),
            (Type::Named(expected_name), Type::Named(actual_name)) => {
                expected_name == actual_name
                    || expected_name.split('<').next() == actual_name.split('<').next()
            }
            (Type::Named(name), Type::Byte) if name == "Bytes" => true,
            (Type::Byte, Type::Named(name)) if name == "Bytes" => true,
            (Type::Formula(_, _), Type::Formula(_, _)) => true,
            (Type::Function(_, _), _) => true,
            (_, Type::Function(_, _)) => true,
            _ => false,
        }
    }

    pub(crate) fn is_numeric(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Float | Type::Byte => true,
            Type::Union(types) => types.iter().any(|t| self.is_numeric(t)),
            _ => false,
        }
    }

    pub(crate) fn parse_type_name(&self, type_name: &str) -> Type {
        let trimmed = type_name.trim();

        // If it's a named parameter in a closure type like `msg: Unknown` or `client: ServerClient`, strip parameter name at depth 0
        let trimmed = {
            let mut depth = 0;
            let mut colon_pos = None;
            for (idx, c) in trimmed.char_indices() {
                if c == '(' || c == '[' || c == '<' {
                    depth += 1;
                } else if c == ')' || c == ']' || c == '>' {
                    depth -= 1;
                } else if c == ':' && depth == 0 {
                    colon_pos = Some(idx);
                    break;
                }
            }
            if let Some(pos) = colon_pos {
                let before = trimmed[..pos].trim();
                if !before.is_empty() && before.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    trimmed[pos + 1..].trim()
                } else {
                    trimmed
                }
            } else {
                trimmed
            }
        };

        // Check for top-level union type: A | B | C
        let mut union_parts = Vec::new();
        let mut u_curr = String::new();
        let mut u_depth = 0;
        for c in trimmed.chars() {
            if c == '(' || c == '[' || c == '<' {
                u_depth += 1;
                u_curr.push(c);
            } else if c == ')' || c == ']' || c == '>' {
                u_depth -= 1;
                u_curr.push(c);
            } else if c == '|' && u_depth == 0 {
                union_parts.push(u_curr.trim().to_string());
                u_curr.clear();
            } else {
                u_curr.push(c);
            }
        }
        if !u_curr.trim().is_empty() {
            union_parts.push(u_curr.trim().to_string());
        }
        if union_parts.len() > 1 {
            let types: Vec<Type> = union_parts.into_iter().map(|p| self.parse_type_name(&p)).collect();
            return Type::Union(types);
        }

        if trimmed == "Object" || trimmed == "Formula" {
            return Type::Formula(HashMap::new(), HashMap::new());
        }
        if trimmed == "&str"
            || trimmed == "&'static str"
            || trimmed == "&'staticstr"
            || trimmed == "str"
            || trimmed == "'static str"
            || trimmed == "'staticstr"
        {
            return Type::String;
        }
        if let Some(rest) = trimmed.strip_prefix("&mut ") {
            return Type::Reference {
                inner: Box::new(self.parse_type_name(rest)),
                mutable: true,
            };
        }
        if let Some(rest) = trimmed.strip_prefix('&') {
            return Type::Reference {
                inner: Box::new(self.parse_type_name(rest)),
                mutable: false,
            };
        }
        if let Some(rest) = trimmed.strip_suffix('?') {
            return Type::Nullable(Box::new(self.parse_type_name(rest)));
        }
        match trimmed {
            "Unknown" | "unknown" | "Any" | "any" => Type::Unknown,
            "Int" | "I32" | "I64" | "U32" | "U64" | "i32" | "i64" | "u32" | "u64" => Type::Int,
            "Float" | "F32" | "F64" | "f32" | "f64" => Type::Float,
            "String" | "string" | "str" | "'static str" => Type::String,
            "Bool" | "bool" => Type::Bool,
            "Nil" | "nil" => Type::Nil,
            "Byte" | "u8" | "U8" => Type::Byte,
            "Bytes" => Type::Named("Bytes".to_string()),
            "Formula" | "Object" => Type::Formula(HashMap::new(), HashMap::new()),
            _ if trimmed.len() == 1 && trimmed.chars().next().unwrap().is_uppercase() => {
                Type::Named(trimmed.to_string())
            }
            _ if trimmed.contains("->") => {
                if let Some((left, right)) = trimmed.split_once("->") {
                    let left_type = self.parse_type_name(left.trim());
                    let right_type = self.parse_type_name(right.trim());
                    let params = match left_type {
                        Type::Tuple(items) => items,
                        Type::Nil => Vec::new(),
                        Type::Function(params, _) => params,
                        other => vec![other], // e.g. Int -> String
                    };
                    return Type::Function(params, Box::new(right_type));
                }
                Type::Named(trimmed.to_string())
            }
            _ if trimmed.starts_with('[') && trimmed.ends_with(']') => {
                let inner = &trimmed[1..trimmed.len() - 1];
                Type::Vector(Box::new(self.parse_type_name(inner)))
            }
            _ if trimmed.starts_with("Vec<") => {
                Type::Unknown
            }
            _ if trimmed.starts_with('(') && trimmed.ends_with(')') => {
                let inner = &trimmed[1..trimmed.len() - 1];
                if inner.trim().is_empty() {
                    Type::Tuple(Vec::new())
                } else {
                    let mut parts = Vec::new();
                    let mut current = String::new();
                    let mut depth = 0;
                    for c in inner.chars() {
                        if c == '(' || c == '[' || c == '<' {
                            depth += 1;
                            current.push(c);
                        } else if c == ')' || c == ']' || c == '>' {
                            depth -= 1;
                            current.push(c);
                        } else if c == ',' && depth == 0 {
                            parts.push(current.trim().to_string());
                            current.clear();
                        } else {
                            current.push(c);
                        }
                    }
                    if !current.trim().is_empty() {
                        parts.push(current.trim().to_string());
                    }
                    if inner.contains(':') {
                        let param_types: Vec<Type> = parts
                            .into_iter()
                            .map(|part| self.parse_type_name(&part))
                            .collect();
                        Type::Function(param_types, Box::new(Type::Nil))
                    } else {
                        Type::Tuple(
                            parts
                                .into_iter()
                                .map(|part| self.parse_type_name(&part))
                                .collect(),
                        )
                    }
                }
            }
            _ if self.structs.contains_key(trimmed) => Type::Struct(trimmed.to_string()),
            _ if self.enums.contains_key(trimmed) => Type::Enum(trimmed.to_string()),
            _ => Type::Named(trimmed.to_string()),
        }
    }

    pub fn format_type(&self, ty: &Type) -> String {
        match ty {
            Type::Int => "Int".to_string(),
            Type::Float => "Float".to_string(),
            Type::String => "String".to_string(),
            Type::Bool => "Bool".to_string(),
            Type::Nil => "Nil".to_string(),
            Type::Byte => "Byte".to_string(),
            Type::Union(types) => types
                .iter()
                .map(|item| self.format_type(item))
                .collect::<Vec<_>>()
                .join(" | "),
            Type::Nullable(item) => format!("{}?", self.format_type(item)),
            Type::Tuple(items) => format!(
                "({})",
                items
                    .iter()
                    .map(|item| self.format_type(item))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::Vector(item) => format!("[{}]", self.format_type(item)),
            Type::Quantity(map) => {
                let mut terms = Vec::new();
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for k in keys {
                    let v = map[k];
                    if v == 1 {
                        terms.push(k.clone());
                    } else {
                        terms.push(format!("{}^{}", k, v));
                    }
                }
                if terms.is_empty() {
                    "Quantity".to_string()
                } else {
                    format!("Quantity <{}>", terms.join(" * "))
                }
            }
            Type::Unit(map) => {
                let mut terms = Vec::new();
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for k in keys {
                    let v = map[k];
                    if v == 1 {
                        terms.push(k.clone());
                    } else {
                        terms.push(format!("{}^{}", k, v));
                    }
                }
                if terms.is_empty() {
                    "Unit".to_string()
                } else {
                    format!("Unit <{}>", terms.join(" * "))
                }
            }
            Type::Formula(_, _) => "Formula".to_string(),
            Type::Function(params, ret) => {
                let params_str = params
                    .iter()
                    .map(|p| self.format_type(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                if matches!(**ret, Type::Nil) {
                    format!("({})", params_str)
                } else {
                    format!("({}) -> {}", params_str, self.format_type(ret))
                }
            }
            Type::Struct(name) | Type::Enum(name) | Type::Named(name) => name.clone(),
            Type::EnumVariant {
                enum_name,
                variant_name,
                ..
            } => format!("{}.{}", enum_name, variant_name),
            Type::Unknown => "Unknown".to_string(),
            Type::Reference { inner, mutable } => {
                if *mutable {
                    format!("&mut {}", self.format_type(inner))
                } else {
                    format!("&{}", self.format_type(inner))
                }
            }
        }
    }

    pub(crate) fn error_binary_mismatch(&mut self, op: &BinaryOp, left: &Type, right: &Type, span: &Span) {
        self.error(
            format!(
                "operator {:?} cannot be applied to {} and {}",
                op,
                self.format_type(left),
                self.format_type(right)
            ),
            span.clone(),
            None,
            None,
        );
    }

    pub(crate) fn type_key(&self, ty: &Type) -> String {
        self.format_type(ty)
    }

    pub(crate) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(crate) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(crate) fn define_var(&mut self, name: String, info: VarInfo) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, info);
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<&VarInfo> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    pub(crate) fn error(
        &mut self,
        message: String,
        span: Span,
        label: Option<String>,
        suggestion: Option<String>,
    ) {
        self.diagnostics.push(Diagnostic::new_error(
            message,
            self.filepath.clone(),
            span,
            label,
            suggestion,
        ));
    }

    pub(crate) fn parse_command_annotation(
        &self,
        func_name: &str,
        annotations: &[crate::parser::Annotation],
        params: &[crate::parser::Param],
        func_span: &Span,
    ) -> Option<CommandInfo> {
        let cmd_anno = annotations.iter().find(|a| a.name == "Command")?;
        let mut cmd_name = None;
        let mut about = None;

        for (idx, arg) in cmd_anno.args.iter().enumerate() {
            let trimmed = arg.trim();
            if trimmed.starts_with("name:")
                || trimmed.starts_with("name :")
                || trimmed.starts_with("name=")
                || trimmed.starts_with("name =")
            {
                let val = if let Some((_, v)) = trimmed.split_once(':') {
                    v
                } else if let Some((_, v)) = trimmed.split_once('=') {
                    v
                } else {
                    trimmed
                };
                cmd_name = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            } else if trimmed.starts_with("about:")
                || trimmed.starts_with("about :")
                || trimmed.starts_with("about=")
                || trimmed.starts_with("about =")
            {
                let val = if let Some((_, v)) = trimmed.split_once(':') {
                    v
                } else if let Some((_, v)) = trimmed.split_once('=') {
                    v
                } else {
                    trimmed
                };
                about = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            } else if trimmed.starts_with("description:")
                || trimmed.starts_with("description :")
                || trimmed.starts_with("description=")
                || trimmed.starts_with("description =")
            {
                let val = if let Some((_, v)) = trimmed.split_once(':') {
                    v
                } else if let Some((_, v)) = trimmed.split_once('=') {
                    v
                } else {
                    trimmed
                };
                about = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
            } else {
                let unquoted = trimmed.trim_matches('"').trim_matches('\'').to_string();
                if idx == 0 && cmd_name.is_none() {
                    cmd_name = Some(unquoted);
                } else if idx == 1 && about.is_none() {
                    about = Some(unquoted);
                }
            }
        }

        let final_name = cmd_name.unwrap_or_else(|| func_name.to_string());
        let anno_sig = match &about {
            Some(ab) => format!("@Command(name: \"{}\", about: \"{}\")", final_name, ab),
            None => format!("@Command(name: \"{}\")", final_name),
        };

        let mut doc = format!(
            "```flame\n{}\n```\n**CLI Subcommand**: `{}`",
            anno_sig, final_name
        );
        if let Some(ab) = &about {
            doc.push_str(&format!("\n\n{}", ab));
        }

        if !params.is_empty() {
            doc.push_str("\n\n**Arguments & Flags:**");
            for p in params {
                let default_str = if let Some(def) = &p.default_val {
                    format!(" = {}", format_expr_simple(def))
                } else {
                    String::new()
                };
                doc.push_str(&format!(
                    "\n- `--{}`: `{}`{}",
                    p.name, p.type_name, default_str
                ));
            }
        }

        Some(CommandInfo {
            name: final_name,
            about,
            func_name: func_name.to_string(),
            params: params
                .iter()
                .map(|p| ParamInfo {
                    name: p.name.clone(),
                    ty: self.parse_type_name(&p.type_name),
                    is_ref: p.is_ref,
                    is_mut: p.is_mut,
                    has_default: p.default_val.is_some() || p.type_name.ends_with('?'),
                })
                .collect(),
            hover_doc: doc,
            span: func_span.clone(),
        })
    }
}

pub(crate) fn format_expr_simple(expr: &Expr) -> String {
    match expr {
        Expr::Literal(LiteralValue::Int(i), _) => i.to_string(),
        Expr::Literal(LiteralValue::Float(f), _) => f.to_string(),
        Expr::Literal(LiteralValue::String(s), _) => format!("\"{}\"", s),
        Expr::Literal(LiteralValue::Bool(b), _) => b.to_string(),
        Expr::Literal(LiteralValue::Nil, _) => "nil".to_string(),
        Expr::Identifier(id, _) => id.clone(),
        Expr::VectorLiteral(items, _) => {
            let inner = items
                .iter()
                .map(format_expr_simple)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{}]", inner)
        }
        _ => "...".to_string(),
    }
}
