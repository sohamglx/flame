use std::collections::{HashMap, HashSet};
use crate::diagnostics::Diagnostic;
use crate::lexer::Span;
use crate::parser::*;
use super::checker::*;
use super::types::*;

impl TypeChecker {
    pub(crate) fn infer_expr_type(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(lit, _) => match lit {
                LiteralValue::Int(_) => Type::Int,
                LiteralValue::Float(_) => Type::Float,
                LiteralValue::String(_) => Type::String,
                LiteralValue::Bool(_) => Type::Bool,
                LiteralValue::Nil => Type::Nil,
            },
            Expr::Identifier(name, span) => {
                let inferred = if let Some(var) = self.lookup_var(name).cloned() {
                    if let Some(doc) = &var.hover_doc {
                        self.insert_hover_info(span.clone(), doc.clone());
                    } else if !matches!(var.ty, Type::Unknown) {
                        let doc = format!("```flame\nlet {}: {}\n```", name, self.format_type(&var.ty));
                        self.insert_hover_info(span.clone(), doc);
                    }
                    var.ty
                } else if let Some(struct_info) = self.structs.get(name).cloned() {
                    if let Some(doc) = struct_info.hover_doc {
                        self.insert_hover_info(span.clone(), doc);
                    }
                    Type::Named(name.clone())
                } else if let Some(enum_info) = self.enums.get(name).cloned() {
                    if let Some(doc) = enum_info.hover_doc {
                        self.insert_hover_info(span.clone(), doc);
                    }
                    Type::Enum(name.clone())
                } else if let Some(func) = self.functions.get(name) {
                    let params_str: Vec<String> = func
                        .params
                        .iter()
                        .map(|p| {
                            let mut mods = String::new();
                            if p.is_ref {
                                mods.push('&');
                            }
                            if p.is_mut {
                                mods.push_str("mut ");
                            }
                            let type_str = match &p.ty {
                                Type::Named(n) => n.clone(),
                                Type::Int => "Int".to_string(),
                                Type::Float => "Float".to_string(),
                                Type::String => "String".to_string(),
                                Type::Bool => "Bool".to_string(),
                                t => format!("{:?}", t),
                            };
                            format!(
                                "{}{}: {}{}",
                                if p.is_mut && !p.is_ref { "mut " } else { "" },
                                p.name,
                                mods,
                                type_str
                            )
                        })
                        .collect();
                    let ret_str = match &func.return_type {
                        Type::Named(n) => n.clone(),
                        Type::Int => "Int".to_string(),
                        Type::Float => "Float".to_string(),
                        Type::String => "String".to_string(),
                        Type::Bool => "Bool".to_string(),
                        Type::Nil => "Nil".to_string(),
                        t => format!("{:?}", t),
                    };
                    let mut hover_str = format!(
                        "```flame\nfn {}({}){}\n```",
                        name,
                        params_str.join(", "),
                        if ret_str.is_empty() || ret_str == "Nil" {
                            "".to_string()
                        } else {
                            format!(" -> {}", ret_str)
                        }
                    );
                    if let Some(doc) = &func.hover_doc {
                        hover_str.push_str(&format!("\n\n{}", doc));
                    }
                    self.insert_hover_info(span.clone(), hover_str);
                    Type::Named(format!("fn({}) -> {}", params_str.join(", "), ret_str))
                } else if self.plugins.contains(name) {
                    self.insert_hover_info(
                        span.clone(),
                        format!("```flame\nplugin {}\n```\n**Native Plugin**", name),
                    );
                    Type::Named(format!("plugin:{}", name))
                } else if self.modules.contains(name) {
                    if let Some(doc) = self.module_docs.get(name) {
                        self.insert_hover_info(span.clone(), doc.clone());
                    }
                    Type::Named(format!("module:{}", name))
                } else {
                    let mut found_variant = None;
                    for (enum_name, enum_info) in &self.enums {
                        if let Some(variant) = enum_info.variants.get(name) {
                            found_variant = Some((enum_name.clone(), variant.clone()));
                            break;
                        }
                    }
                    if let Some((enum_name, variant)) = found_variant {
                        if let Some(doc) = &variant.hover_doc {
                            self.insert_hover_info(span.clone(), doc.clone());
                        }
                        if variant.struct_fields.is_empty() && variant.tuple_items.is_empty() {
                            Type::EnumVariant {
                                enum_name,
                                variant_name: name.clone(),
                                tuple_items: vec![],
                                struct_fields: HashMap::new(),
                            }
                        } else {
                            // It's a constructor for a tuple or struct variant
                            let params_str: Vec<String> = variant
                                .tuple_items
                                .iter()
                                .map(|t| format!("{:?}", t))
                                .collect();
                            Type::Named(format!("fn({}) -> EnumVariant", params_str.join(", ")))
                        }
                    } else {
                        self.error(
                            format!("undefined identifier '{}'", name),
                            span.clone(),
                            Some("This name is not declared in the current scope".to_string()),
                            None,
                        );
                        Type::Unknown
                    }
                };

                let hover_str = if let Some(var) = self.lookup_var(name) {
                    if let Some(doc) = &var.hover_doc {
                        doc.clone()
                    } else if let (Type::Function(_, _), Some(func_sig)) =
                        (&var.ty, self.functions.get(name))
                    {
                        let mut params_str = Vec::new();
                        for p in &func_sig.params {
                            let ty_str = self.format_type(&p.ty);
                            params_str.push(format!("{}: {}", p.name, ty_str));
                        }
                        if let Some(doc) = &func_sig.hover_doc {
                            doc.clone()
                        } else {
                            let ret_ty_str = self.format_type(&func_sig.return_type);
                            format!(
                                "```flame\nfn {}({}) -> {}\n```",
                                name,
                                params_str.join(", "),
                                ret_ty_str
                            )
                        }
                    } else {
                        let type_str = self.format_type(&inferred);
                        let sig_str = if var.is_mut {
                            format!("```flame\nlet mut {}: {}\n```", name, type_str)
                        } else {
                            format!("```flame\nlet {}: {}\n```", name, type_str)
                        };
                        sig_str
                    }
                } else if self.plugins.contains(name) {
                    "plugin".to_string()
                } else if self.modules.contains(name) {
                    if let Some(doc) = self.module_docs.get(name) {
                        self.insert_hover_info(span.clone(), doc.clone());
                    }
                    format!("module:{}", name)
                } else {
                    let mut variant_doc = None;
                    for (enum_name, enum_info) in &self.enums {
                        if let Some(variant) = enum_info.variants.get(name) {
                            if let Some(doc) = &variant.hover_doc {
                                variant_doc = Some(format!(
                                    "```flame\n{}::{}\n```\n{}",
                                    enum_name, name, doc
                                ));
                            } else {
                                let params_str: Vec<String> = variant
                                    .tuple_items
                                    .iter()
                                    .map(|t| self.format_type(t))
                                    .collect();
                                variant_doc = Some(format!(
                                    "```flame\n{}({}) -> {}\n```",
                                    name,
                                    params_str.join(", "),
                                    enum_name
                                ));
                            }
                            break;
                        }
                    }
                    if let Some(doc) = variant_doc {
                        doc
                    } else if let Some(struct_info) = self.structs.get(name) {
                        if let Some(doc) = &struct_info.hover_doc {
                            format!("```flame\nstruct {}\n```\n\n{}", name, doc)
                        } else {
                            format!("```flame\nstruct {}\n```", name)
                        }
                    } else if let Some(enum_info) = self.enums.get(name) {
                        if let Some(doc) = &enum_info.hover_doc {
                            format!("```flame\nenum {}\n```\n\n{}", name, doc)
                        } else {
                            format!("```flame\nenum {}\n```", name)
                        }
                    } else if let Some(func_sig) = self.functions.get(name) {
                        let mut params_str = Vec::new();
                        for p in &func_sig.params {
                            let is_already_ref = matches!(p.ty, Type::Reference { .. });
                            let mut mods = String::new();
                            if p.is_ref && !is_already_ref {
                                mods.push('&');
                            }
                            if p.is_mut && !is_already_ref {
                                mods.push_str("mut ");
                            }
                            params_str.push(format!(
                                "{}{}: {}{}",
                                if p.is_mut && !p.is_ref { "mut " } else { "" },
                                p.name,
                                mods,
                                self.format_type(&p.ty)
                            ));
                        }
                        let ret_str = if func_sig.return_type == Type::Nil {
                            "".to_string()
                        } else {
                            format!(" -> {}", self.format_type(&func_sig.return_type))
                        };
                        let mut s = format!(
                            "```flame\nfn {}({}){}\n```",
                            name,
                            params_str.join(", "),
                            ret_str
                        );
                        if let Some(doc) = &func_sig.hover_doc {
                            s = format!("{}\n\n{}", s, doc);
                        }
                        s
                    } else if let Type::Named(s) = &inferred {
                        s.clone()
                    } else {
                        self.format_type(&inferred)
                    }
                };

                self.insert_hover_info(span.clone(), hover_str);
                inferred
            }
            Expr::Tuple(items, _) => Type::Tuple(
                items
                    .iter()
                    .map(|item| self.infer_expr_type(item))
                    .collect(),
            ),
            Expr::VectorLiteral(items, _) => {
                let mut unique_types = HashSet::new();
                let mut inferred = Vec::new();
                for item in items {
                    let item_ty = self.infer_expr_type(item);
                    unique_types.insert(self.type_key(&item_ty));
                    inferred.push(item_ty);
                }
                if unique_types.len() > 1 {
                    self.error(
                        "vector literal contains incompatible element types".to_string(),
                        expr.span(),
                        Some("All elements in a vector should have the same type".to_string()),
                        None,
                    );
                }
                Type::Vector(Box::new(inferred.first().cloned().unwrap_or(Type::Unknown)))
            }
            Expr::Formula(pairs, _) => {
                let mut map = HashMap::new();
                let mut docs = HashMap::new();
                for (k, v, span, annotations) in pairs {
                    let ty = self.infer_expr_type(v);
                    let signature = match v {
                        Expr::Closure {
                            params,
                            return_type,
                            ..
                        } => {
                            let params_str = params
                                .iter()
                                .map(|p| format!("{}: {}", p.name, p.type_name))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let ret_str = if let Some(ret) = return_type {
                                format!(" -> {}", ret)
                            } else {
                                "".to_string()
                            };
                            format!("```flame\nfn {}({}){}\n```", k, params_str, ret_str)
                        }
                        _ => format!("```flame\n{}: {}\n```", k, self.format_type(&ty)),
                    };
                    let hover_doc = self.process_annotations(annotations);
                    let full_doc = if let Some(doc) = hover_doc {
                        format!("{}\n\n{}", signature, doc)
                    } else {
                        signature
                    };
                    self.insert_hover_info(span.clone(), full_doc.clone());
                    docs.insert(k.clone(), full_doc);
                    map.insert(k.clone(), ty);
                }
                Type::Formula(map, docs)
            }
            Expr::Object(pairs, _) => {
                let mut map = HashMap::new();
                for (k, v, annotations) in pairs {
                    let ty = self.infer_expr_type(v);
                    let _ = self.process_annotations(annotations);
                    map.insert(k.clone(), ty);
                }
                Type::Formula(map, HashMap::new()) // We treat Object and Formula as structurally equivalent in types for now, or we can use a new Type::Object. Let's use Type::Formula since it's a dynamic map
            }
            Expr::InterpolatedString(segments, span) => {
                for segment in segments {
                    if let crate::parser::InterpolatedSegment::Expr(inner) = segment {
                        self.infer_expr_type(inner);
                    }
                }
                let _ = span;
                Type::String
            }
            Expr::Borrow(inner, is_mut, _) => Type::Reference {
                inner: Box::new(self.infer_expr_type(inner)),
                mutable: *is_mut,
            },
            Expr::Await(inner, _) => self.infer_expr_type(inner),
            Expr::ThreadSpawn(_, _) => Type::Named("ThreadHandler".to_string()),
            Expr::Block(stmts, _) => {
                self.push_scope();
                for stmt in stmts {
                    self.check_stmt(stmt);
                }
                self.pop_scope();
                Type::Nil
            }
            Expr::Unary(op, inner, span) => match op {
                UnaryOp::Neg => {
                    let ty = self.infer_expr_type(inner);
                    if self.is_numeric(&ty) {
                        ty
                    } else {
                        self.error(
                            "cannot apply unary '-' to non-numeric type".to_string(),
                            span.clone(),
                            None,
                            None,
                        );
                        Type::Unknown
                    }
                }
                UnaryOp::Not => {
                    let _ = self.infer_expr_type(inner);
                    Type::Bool
                }
                UnaryOp::NonNullAssert => {
                    let ty = self.infer_expr_type(inner);
                    ty
                }
                UnaryOp::PreInc | UnaryOp::PreDec | UnaryOp::PostInc | UnaryOp::PostDec => {
                    let ty = self.infer_expr_type(inner);
                    if !self.is_numeric(&ty) {
                        self.error(
                            "cannot increment/decrement non-numeric type".to_string(),
                            span.clone(),
                            None,
                            None,
                        );
                    }
                    ty
                }
            },
            Expr::SafeDot(inner, member, span) => self.infer_dot_type(inner, member, span),
            Expr::Binary(left, op, right, span) => self.infer_binary_type(left, op, right, span),
            Expr::Dot(inner, member, span) => self.infer_dot_type(inner, member, span),
            Expr::StructInit(inner, fields, span) => {
                self.infer_struct_init_type(inner, fields, span)
            }
            Expr::Index(inner, idx, span) => self.infer_index_type(inner, idx, span),
            Expr::Cast(inner, target_type_str, _span) => {
                let _inner_ty = self.infer_expr_type(inner);
                self.parse_type_name(target_type_str)
            }
            Expr::Closure {
                params,
                return_type,
                body,
                annotations,
                span,
                ..
            } => {
                let expected_func = match &self.expected_closure_type {
                    Some(Type::Function(e_params, e_ret)) => Some((e_params.clone(), (**e_ret).clone())),
                    Some(Type::Tuple(e_params)) => Some((e_params.clone(), Type::Nil)),
                    _ => None,
                };

                let ret_ty = return_type
                    .as_ref()
                    .map(|ret| self.parse_type_name(ret))
                    .unwrap_or_else(|| {
                        if let Some((_, e_ret)) = &expected_func {
                            e_ret.clone()
                        } else {
                            Type::Unknown
                        }
                    });

                self.push_scope();
                let mut param_types = Vec::new();
                for (idx, param) in params.iter().enumerate() {
                    let mut p_ty = if param.type_name.trim().is_empty() {
                        Type::Unknown
                    } else {
                        self.parse_type_name(&param.type_name)
                    };
                    if matches!(p_ty, Type::Unknown) {
                        if let Some((e_params, _)) = &expected_func {
                            if let Some(expected_p) = e_params.get(idx) {
                                p_ty = expected_p.clone();
                            }
                        }
                    }
                    param_types.push(p_ty.clone());
                    let p_hover_doc = format!("```flame\n(parameter) {}: {}\n```", param.name, self.format_type(&p_ty));
                    self.define_var(
                        param.name.clone(),
                        VarInfo {
                            ty: p_ty.clone(),
                            is_mut: param.is_mut,
                            hover_doc: Some(p_hover_doc),
                        },
                    );
                }

                let params_str = params
                    .iter()
                    .zip(param_types.iter())
                    .map(|(p, ty)| {
                        if !p.type_name.trim().is_empty() {
                            format!("{}: {}", p.name, p.type_name)
                        } else if !matches!(ty, Type::Unknown) {
                            format!("{}: {}", p.name, self.format_type(ty))
                        } else {
                            p.name.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let ret_str = if ret_ty != Type::Nil && ret_ty != Type::Unknown {
                    format!(" -> {}", self.format_type(&ret_ty))
                } else if let Some(ret) = &return_type {
                    format!(" -> {}", ret)
                } else {
                    "".to_string()
                };
                let mut hover_str = format!("```flame\n({}){}\n```", params_str, ret_str);

                let hover_doc = self.process_annotations(annotations);
                if let Some(doc) = hover_doc {
                    hover_str = format!("{}\n\n{}", hover_str, doc);
                }
                self.insert_hover_info(span.clone(), hover_str);

                let prev_return = self.current_return_type.clone();
                self.current_return_type = Some(ret_ty.clone());
                let prev_expected = self.expected_closure_type.take();

                for stmt in body {
                    self.check_stmt(stmt);
                }
                self.pop_scope();
                self.expected_closure_type = prev_expected;
                self.current_return_type = prev_return;
                Type::Function(param_types, Box::new(ret_ty))
            }
            Expr::Call(callee, args, span) => self.infer_call_type(callee, args, span),
            Expr::JsxElement {
                tag,
                attributes,
                children,
                span,
            } => {
                for attr in attributes {
                    if let Some(val) = &attr.value {
                        self.infer_expr_type(val);
                    }
                }
                fn check_child(checker: &mut TypeChecker, child: &crate::parser::JsxChild) {
                    match child {
                        crate::parser::JsxChild::Expr(e) => {
                            checker.infer_expr_type(e);
                        }
                        crate::parser::JsxChild::Element(e) => {
                            checker.infer_expr_type(e);
                        }
                        crate::parser::JsxChild::Text(_, _) => {}
                        crate::parser::JsxChild::For {
                            var_name,
                            iterable,
                            body,
                            ..
                        } => {
                            let item_ty = match checker.infer_expr_type(iterable) {
                                Type::Tuple(items) => items.first().cloned().unwrap_or(Type::Unknown),
                                Type::Vector(item) => (*item).clone(),
                                _ => Type::Unknown,
                            };
                            checker.push_scope();
                            checker.define_var(
                                var_name.clone(),
                                VarInfo {
                                    ty: item_ty,
                                    is_mut: false,
                                    hover_doc: None,
                                },
                            );
                            for b in body {
                                check_child(checker, b);
                            }
                            checker.pop_scope();
                        }
                    }
                }

                for child in children {
                    check_child(self, child);
                }
                self.insert_hover_info(
                    span.clone(),
                    format!("JSX Element: <{}>\nRepresents an HTML DOM element in std.web.", tag),
                );
                Type::Named("HtmlNode".to_string())
            }
        }
    }

    pub(crate) fn infer_binary_type(&mut self, left: &Expr, op: &BinaryOp, right: &Expr, span: &Span) -> Type {
        if matches!(
            op,
            BinaryOp::Assign
                | BinaryOp::PlusAssign
                | BinaryOp::MinusAssign
                | BinaryOp::MulAssign
                | BinaryOp::DivAssign
                | BinaryOp::ModAssign
                | BinaryOp::BitAndAssign
                | BinaryOp::BitOrAssign
                | BinaryOp::BitXorAssign
                | BinaryOp::ShlAssign
                | BinaryOp::ShrAssign
        ) {
            if let Expr::Identifier(name, left_span) = left {
                let prev_expected = self.expected_closure_type.take();
                if let Some(var) = self.lookup_var(name) {
                    self.expected_closure_type = Some(var.ty.clone());
                }
                let rhs_ty = self.infer_expr_type(right);
                self.expected_closure_type = prev_expected;
                if let Some(var) = self.lookup_var(name).cloned() {
                    if !var.is_mut {
                        self.error(
                            format!("cannot assign to immutable variable '{}'", name),
                            left_span.clone(),
                            Some(
                                "Declare it with 'let mut' if reassignment is intended".to_string(),
                            ),
                            None,
                        );
                    }
                    if *op != BinaryOp::Assign {
                        if !self.is_numeric(&var.ty) || !self.is_numeric(&rhs_ty) {
                            if matches!(op, BinaryOp::PlusAssign)
                                && matches!(var.ty, Type::String)
                                && matches!(rhs_ty, Type::String)
                            {
                                // String concatenation
                            } else if matches!(var.ty, Type::Unknown)
                                || matches!(rhs_ty, Type::Unknown)
                            {
                                // Ignore mismatch if type is Unknown
                            } else {
                                self.error_binary_mismatch(op, &var.ty, &rhs_ty, span);
                            }
                        }
                    }
                    self.expect_assignable(&var.ty, &rhs_ty, span, "assignment");
                } else {
                    self.error(
                        format!("cannot assign to undefined variable '{}'", name),
                        left_span.clone(),
                        None,
                        None,
                    );
                }
                return rhs_ty;
            } else if let Expr::Dot(inner, member, _) = left {
                // con.name = expr: resolve the field type and ensure RHS matches
                let lhs_ty = self.infer_dot_type(inner, member, span);
                let rhs_ty = self.infer_expr_type(right);

                if *op != BinaryOp::Assign {
                    if !self.is_numeric(&lhs_ty) || !self.is_numeric(&rhs_ty) {
                        if matches!(op, BinaryOp::PlusAssign)
                            && matches!(lhs_ty, Type::String)
                            && matches!(rhs_ty, Type::String)
                        {
                            // String concatenation
                        } else if matches!(lhs_ty, Type::Unknown) || matches!(rhs_ty, Type::Unknown)
                        {
                            // Ignore mismatch if type is Unknown
                        } else {
                            self.error_binary_mismatch(op, &lhs_ty, &rhs_ty, span);
                        }
                    }
                }

                self.expect_assignable(&lhs_ty, &rhs_ty, span, "field assignment");
                return lhs_ty;
            } else if let Expr::Index(inner, idx, _) = left {
                let lhs_ty = self.infer_index_type(inner, idx, span);
                let rhs_ty = self.infer_expr_type(right);
                if *op != BinaryOp::Assign {
                    if !self.is_numeric(&lhs_ty) || !self.is_numeric(&rhs_ty) {
                        if matches!(lhs_ty, Type::Unknown) || matches!(rhs_ty, Type::Unknown) {
                            // ignore
                        } else {
                            self.error_binary_mismatch(op, &lhs_ty, &rhs_ty, span);
                        }
                    }
                }
                self.expect_assignable(&lhs_ty, &rhs_ty, span, "index assignment");
                return rhs_ty;
            }

            self.error(
                "left-hand side of assignment must be an identifier or field".to_string(),
                span.clone(),
                None,
                None,
            );
            return Type::Unknown;
        }

        let mut left_ty = self.infer_expr_type(left);
        let mut right_ty = self.infer_expr_type(right);

        if let Type::Function(params, ret) = &left_ty {
            if params.is_empty() {
                left_ty = *ret.clone();
            }
        }
        if let Type::Function(params, ret) = &right_ty {
            if params.is_empty() {
                right_ty = *ret.clone();
            }
        }
        match op {
            BinaryOp::Add => {
                if self.is_numeric(&left_ty) && self.is_numeric(&right_ty) {
                    if matches!(left_ty, Type::Float) || matches!(right_ty, Type::Float) {
                        Type::Float
                    } else {
                        Type::Int
                    }
                } else if (matches!(left_ty, Type::String) || matches!(left_ty, Type::Union(ref u) if u.iter().any(|t| matches!(t, Type::String))))
                    && (matches!(right_ty, Type::String) || matches!(right_ty, Type::Union(ref u) if u.iter().any(|t| matches!(t, Type::String))))
                {
                    Type::String
                } else if let (Type::Quantity(m1), Type::Quantity(m2)) = (&left_ty, &right_ty) {
                    if m1 == m2 {
                        Type::Quantity(m1.clone())
                    } else {
                        self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                        Type::Unknown
                    }
                } else if let (Type::Unit(m1), Type::Unit(m2)) = (&left_ty, &right_ty) {
                    if m1 == m2 {
                        Type::Unit(m1.clone())
                    } else {
                        self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                        Type::Unknown
                    }
                } else if let (Type::Quantity(m1), Type::Unit(m2)) = (&left_ty, &right_ty) {
                    if m1 == m2 {
                        Type::Quantity(m1.clone())
                    } else {
                        self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                        Type::Unknown
                    }
                } else if let (Type::Unit(m1), Type::Quantity(m2)) = (&left_ty, &right_ty) {
                    if m1 == m2 {
                        Type::Quantity(m1.clone())
                    } else {
                        self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                        Type::Unknown
                    }
                } else if matches!(left_ty, Type::Named(ref n) if n == "Quantity" || n == "Unit")
                    && matches!(right_ty, Type::Named(ref m) if m == "Quantity" || m == "Unit")
                {
                    Type::Named("Quantity".to_string())
                } else if matches!(left_ty, Type::Unknown) || matches!(right_ty, Type::Unknown) {
                    if matches!(left_ty, Type::String) || matches!(right_ty, Type::String) {
                        Type::String
                    } else {
                        Type::Unknown
                    }
                } else {
                    self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                    Type::Unknown
                }
            }
            BinaryOp::Sub => {
                if self.is_numeric(&left_ty) && self.is_numeric(&right_ty) {
                    if matches!(left_ty, Type::Float) || matches!(right_ty, Type::Float) {
                        Type::Float
                    } else {
                        Type::Int
                    }
                } else if let (Type::Quantity(m1), Type::Quantity(m2)) = (&left_ty, &right_ty) {
                    if m1 == m2 {
                        Type::Quantity(m1.clone())
                    } else {
                        self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                        Type::Unknown
                    }
                } else if let (Type::Unit(m1), Type::Unit(m2)) = (&left_ty, &right_ty) {
                    if m1 == m2 {
                        Type::Unit(m1.clone())
                    } else {
                        self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                        Type::Unknown
                    }
                } else if let (Type::Quantity(m1), Type::Unit(m2)) = (&left_ty, &right_ty) {
                    if m1 == m2 {
                        Type::Quantity(m1.clone())
                    } else {
                        self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                        Type::Unknown
                    }
                } else if let (Type::Unit(m1), Type::Quantity(m2)) = (&left_ty, &right_ty) {
                    if m1 == m2 {
                        Type::Quantity(m1.clone())
                    } else {
                        self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                        Type::Unknown
                    }
                } else if matches!(left_ty, Type::Named(ref n) if n == "Quantity" || n == "Unit")
                    && matches!(right_ty, Type::Named(ref m) if m == "Quantity" || m == "Unit")
                {
                    Type::Named("Quantity".to_string())
                } else if matches!(left_ty, Type::Unknown) || matches!(right_ty, Type::Unknown) {
                    Type::Unknown
                } else {
                    self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                    Type::Unknown
                }
            }
            BinaryOp::Mul | BinaryOp::Div => {
                let is_q_or_u = |t: &Type| {
                    if let Type::Named(n) = t {
                        n == "Quantity" || n == "Unit"
                    } else {
                        matches!(t, Type::Quantity(_) | Type::Unit(_))
                    }
                };
                let get_dims = |t: &Type| -> HashMap<String, i32> {
                    match t {
                        Type::Quantity(map) | Type::Unit(map) => map.clone(),
                        _ => HashMap::new(),
                    }
                };
                let compute_dims =
                    |m1: &HashMap<String, i32>, m2: &HashMap<String, i32>, is_div: bool| {
                        let mut res = m1.clone();
                        for (k, v) in m2 {
                            let current = res.entry(k.clone()).or_insert(0);
                            if is_div {
                                *current -= v;
                            } else {
                                *current += v;
                            }
                            if *current == 0 {
                                res.remove(k);
                            }
                        }
                        res
                    };
                let invert_dims = |m: &HashMap<String, i32>| {
                    let mut res = HashMap::new();
                    for (k, v) in m {
                        res.insert(k.clone(), -v);
                    }
                    res
                };

                if self.is_numeric(&left_ty) && self.is_numeric(&right_ty) {
                    if matches!(left_ty, Type::Float) || matches!(right_ty, Type::Float) {
                        Type::Float
                    } else {
                        Type::Int
                    }
                } else if self.is_numeric(&left_ty) && is_q_or_u(&right_ty) {
                    let dims = get_dims(&right_ty);
                    if matches!(*op, BinaryOp::Div) {
                        Type::Quantity(invert_dims(&dims))
                    } else {
                        Type::Quantity(dims)
                    }
                } else if is_q_or_u(&left_ty) && self.is_numeric(&right_ty) {
                    Type::Quantity(get_dims(&left_ty))
                } else if is_q_or_u(&left_ty) && is_q_or_u(&right_ty) {
                    let dims = compute_dims(
                        &get_dims(&left_ty),
                        &get_dims(&right_ty),
                        matches!(*op, BinaryOp::Div),
                    );
                    Type::Quantity(dims)
                } else if matches!(left_ty, Type::Unknown) || matches!(right_ty, Type::Unknown) {
                    Type::Unknown
                } else {
                    self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                    Type::Unknown
                }
            }
            BinaryOp::Mod => {
                if self.is_numeric(&left_ty) && self.is_numeric(&right_ty) {
                    if matches!(left_ty, Type::Float) || matches!(right_ty, Type::Float) {
                        Type::Float
                    } else {
                        Type::Int
                    }
                } else if matches!(left_ty, Type::Unknown) || matches!(right_ty, Type::Unknown) {
                    Type::Unknown
                } else {
                    self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                    Type::Unknown
                }
            }
            BinaryOp::Eq | BinaryOp::Ne => {
                let is_nil_or_nullable = |t: &Type| matches!(t, Type::Nil | Type::Nullable(_));
                if !matches!(left_ty, Type::Nil)
                    && !matches!(right_ty, Type::Nil)
                    && !matches!(left_ty, Type::Unknown)
                    && !matches!(right_ty, Type::Unknown)
                    && !(is_nil_or_nullable(&left_ty) && is_nil_or_nullable(&right_ty))
                    && !self.is_compatible(&left_ty, &right_ty)
                    && !self.is_compatible(&right_ty, &left_ty)
                {
                    self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                }
                Type::Bool
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                if self.is_numeric(&left_ty) && self.is_numeric(&right_ty) {
                    Type::Bool
                } else if matches!(left_ty, Type::Unknown) || matches!(right_ty, Type::Unknown) {
                    Type::Bool
                } else {
                    self.error_binary_mismatch(op, &left_ty, &right_ty, span);
                    Type::Bool
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                self.expect_assignable(&Type::Bool, &left_ty, span, "logical expression");
                self.expect_assignable(&Type::Bool, &right_ty, span, "logical expression");
                Type::Bool
            }
            BinaryOp::NilCoalesce => {
                if matches!(left_ty, Type::Nil) {
                    right_ty
                } else {
                    left_ty
                }
            }
            BinaryOp::BitXor => {
                if let Type::Quantity(dims) | Type::Unit(dims) = &left_ty {
                    Type::Quantity(dims.clone())
                } else if self.is_numeric(&left_ty) && self.is_numeric(&right_ty) {
                    if matches!(left_ty, Type::Float) || matches!(right_ty, Type::Float) {
                        Type::Float
                    } else {
                        Type::Int
                    }
                } else {
                    Type::Int
                }
            }
            BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::Shl
            | BinaryOp::Shr => Type::Int,
            BinaryOp::Range => {
                self.expect_assignable(&Type::Int, &left_ty, span, "range start");
                self.expect_assignable(&Type::Int, &right_ty, span, "range end");
                Type::Named("Range".to_string())
            }
            BinaryOp::Assign
            | BinaryOp::PlusAssign
            | BinaryOp::MinusAssign
            | BinaryOp::MulAssign
            | BinaryOp::DivAssign
            | BinaryOp::ModAssign
            | BinaryOp::BitAndAssign
            | BinaryOp::BitOrAssign
            | BinaryOp::BitXorAssign
            | BinaryOp::ShlAssign
            | BinaryOp::ShrAssign => Type::Unknown,
        }
    }

    pub(crate) fn infer_index_type(&mut self, inner: &Expr, idx: &Expr, span: &Span) -> Type {
        let inner_ty = self.infer_expr_type(inner);
        let idx_ty = self.infer_expr_type(idx);
        if !matches!(idx_ty, Type::Int | Type::Unknown) {
            self.error(
                format!(
                    "expected integer index, found {}",
                    self.format_type(&idx_ty)
                ),
                idx.span(),
                None,
                None,
            );
        }
        match inner_ty {
            Type::Vector(elem) => *elem,
            Type::Tuple(elems) => {
                if let Expr::Literal(crate::parser::LiteralValue::Int(i), _) = idx {
                    if *i >= 0 && (*i as usize) < elems.len() {
                        return elems[*i as usize].clone();
                    } else {
                        self.error(
                            format!(
                                "tuple index out of bounds: {} for tuple of length {}",
                                i,
                                elems.len()
                            ),
                            span.clone(),
                            None,
                            None,
                        );
                    }
                }
                Type::Unknown
            }
            Type::String => Type::String,
            Type::Byte => Type::Byte,
            Type::Unknown => Type::Unknown,
            Type::Reference {
                inner: ref_inner, ..
            } => match *ref_inner {
                Type::Vector(elem) => *elem,
                _ => Type::Unknown,
            },
            _ => {
                self.error(
                    format!("cannot index into type {}", self.format_type(&inner_ty)),
                    span.clone(),
                    None,
                    None,
                );
                Type::Unknown
            }
        }
    }

    pub(crate) fn infer_dot_type(&mut self, inner: &Expr, member: &str, span: &Span) -> Type {
        // Intercept Flame-defined package fields first
        if let Expr::Identifier(mod_name, _) = inner {
            let prefixed_member = format!("{}.{}", mod_name, member);

            if self.structs.contains_key(&prefixed_member) {
                if let Some(info) = self.structs.get(&prefixed_member) {
                    let mut doc_str = format!("```flame\nstruct {}\n```", member);
                    if let Some(d) = &info.hover_doc {
                        doc_str = format!("{}\n\n{}", doc_str, d);
                    }
                    self.insert_hover_info(span.clone(), doc_str);
                }
                return Type::Struct(prefixed_member);
            }
            if self.enums.contains_key(&prefixed_member) {
                if let Some(info) = self.enums.get(&prefixed_member) {
                    let mut doc_str = format!("```flame\nenum {}\n```", member);
                    if let Some(d) = &info.hover_doc {
                        doc_str = format!("{}\n\n{}", doc_str, d);
                    }
                    self.insert_hover_info(span.clone(), doc_str);
                }
                return Type::Enum(prefixed_member);
            }
            if let Some(sig) = self.functions.get(&prefixed_member).cloned() {
                let params_str = sig
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, self.format_type(&p.ty)))
                    .collect::<Vec<_>>()
                    .join(", ");
                let ret_str = if sig.return_type == Type::Nil {
                    "".to_string()
                } else {
                    format!(" -> {}", self.format_type(&sig.return_type))
                };
                let fallback = format!("```flame\nfn {}({}){}\n```", member, params_str, ret_str);
                if let Some(doc) = &sig.hover_doc {
                    self.insert_hover_info(span.clone(), format!("{}\n\n{}", fallback, doc));
                } else {
                    self.insert_hover_info(span.clone(), fallback);
                }

                let mut p_tys = Vec::new();
                for p in &sig.params {
                    p_tys.push(p.ty.clone());
                }
                return Type::Function(p_tys, Box::new(sig.return_type));
            }
        }

        let mut inner_ty = self.infer_expr_type(inner);
        if let Type::Reference {
            inner: ref_inner, ..
        } = inner_ty
        {
            inner_ty = *ref_inner;
        }
        match member {
            "toString" | "toChar" | "trim" | "toUpperCase" | "toLowerCase" | "replace" | "join"
            | "push_str" | "push" | "pop" | "clear" | "remove" | "insert" | "slice"
            | "substring" => return Type::String,
            "toInt" | "tryInt" | "len" => return Type::Int,
            "toFloat" | "tryFloat" => return Type::Float,
            "toBool" | "tryBool" | "contains" | "startsWith" | "endsWith" | "isEmpty" => {
                return Type::Bool;
            }
            "toByte" | "to_byte" => {
                return Type::Byte;
            }
            "split" | "keys" | "values" => {
                return Type::Vector(Box::new(Type::Unknown));
            }
            "clone" => return inner_ty.clone(),
            "assertEq" => return Type::Nil,
            _ => {}
        }
        match inner_ty {
            Type::Named(name) if name.starts_with("plugin:") || name.starts_with("module:") => {
                let prefix = if name.starts_with("plugin:") {
                    &name["plugin:".len()..]
                } else {
                    &name["module:".len()..]
                };

                // Check for Flame-defined functions via indirect access
                let prefixed_member = format!("{}.{}", prefix, member);
                if let Some(sig) = self.functions.get(&prefixed_member).cloned() {
                    let params_str = sig
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name, self.format_type(&p.ty)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let ret_str = if sig.return_type == Type::Nil {
                        "".to_string()
                    } else {
                        format!(" -> {}", self.format_type(&sig.return_type))
                    };
                    let fallback =
                        format!("```flame\nfn {}({}){}\n```", member, params_str, ret_str);
                    if let Some(doc) = &sig.hover_doc {
                        self.insert_hover_info(span.clone(), format!("{}\n\n{}", fallback, doc));
                    } else {
                        self.insert_hover_info(span.clone(), fallback);
                    }

                    let mut p_tys = Vec::new();
                    for p in &sig.params {
                        p_tys.push(p.ty.clone());
                    }
                    return Type::Function(p_tys, Box::new(sig.return_type));
                }

                if name.starts_with("plugin:") {
                    if let Some(funcs) = self.plugin_functions.get(prefix) {
                        if let Some(sig) = funcs.get(member).cloned() {
                            let params_str = sig
                                .params
                                .iter()
                                .map(|p| format!("{}: {}", p.name, self.format_type(&p.ty)))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let ret_str = if sig.return_type == Type::Nil {
                                "".to_string()
                            } else {
                                format!(" -> {}", self.format_type(&sig.return_type))
                            };
                            let fallback =
                                format!("```flame\nfn {}({}){}\n```", member, params_str, ret_str);
                            if let Some(doc) = &sig.hover_doc {
                                self.insert_hover_info(
                                    span.clone(),
                                    format!("{}\n{}", fallback, doc),
                                );
                            } else {
                                self.insert_hover_info(span.clone(), fallback);
                            }
                            return Type::Named("Function".into());
                        }
                    }
                    if let Some(methods) = self.plugin_methods.get(prefix) {
                        if let Some(ty) = methods.get(member) {
                            return ty.clone();
                        }
                    }
                } else {
                    let ret_ty = match (prefix, member) {
                        ("fs", "read") => Type::String,
                        ("fs", "readDir") => Type::Vector(Box::new(Type::String)),
                        ("fs", "readBytes") => Type::Byte,
                        ("fs", "open") => Type::Unknown,
                        ("thread", "sleep") => Type::Nil,
                        ("thread", "yield") | ("thread", "yield_now") => Type::Nil,
                        ("thread", "id") => Type::String,
                        ("thread", "channel") => Type::Tuple(vec![Type::Unknown, Type::Unknown]),
                        ("byte", "readBytes") => Type::Byte,
                        ("byte", "readByte") => Type::Byte,
                        ("byte", "readByteAt") => Type::Byte,
                        _ => Type::Unknown,
                    };
                    if !matches!(ret_ty, Type::Unknown) {
                        return ret_ty;
                    }
                }
                Type::Unknown
            }
            Type::Enum(enum_name) => {
                if let Some(info) = self.enums.get(&enum_name).cloned() {
                    if let Some(variant) = info.variants.get(member) {
                        if let Some(doc) = &variant.hover_doc {
                            self.insert_hover_info(span.clone(), doc.clone());
                        }
                        let struct_fields = variant
                            .struct_fields
                            .iter()
                            .map(|(name, ty)| (name.clone(), ty.clone()))
                            .collect();
                        return Type::EnumVariant {
                            enum_name,
                            variant_name: member.to_string(),
                            tuple_items: variant.tuple_items.clone(),
                            struct_fields,
                        };
                    }
                }
                self.error(
                    format!("enum '{}' has no variant '{}'", enum_name, member),
                    span.clone(),
                    None,
                    None,
                );
                Type::Unknown
            }
            Type::EnumVariant {
                enum_name,
                variant_name,
                tuple_items,
                struct_fields,
            } => {
                if let Some(field_ty) = struct_fields.get(member) {
                    return field_ty.clone();
                }

                // Delegate member access to single-item tuple Formula payloads.
                if let [Type::Formula(fmap, _)] = tuple_items.as_slice() {
                    if let Some(t) = fmap.get(member) {
                        return t.clone();
                    } else {
                        return Type::Unknown;
                    }
                }

                self.error(
                    format!(
                        "variant '{}.{}' has no field '{}'",
                        enum_name, variant_name, member
                    ),
                    span.clone(),
                    None,
                    None,
                );

                Type::Unknown
            }

            Type::Struct(ref struct_name)
            | Type::Named(ref struct_name)
                if self.structs.contains_key(struct_name) || self.methods.contains_key(struct_name) =>
            {
                let found_field = self
                    .structs
                    .get(struct_name)
                    .and_then(|info| info.fields.iter().find(|(name, _)| name == member).map(|(_, ty)| ty.clone()));
                if let Some(ty) = found_field {
                    let formatted_ty = self.format_type(&ty);
                    let member_span = Span {
                        start: span.end.saturating_sub(member.len()),
                        end: span.end,
                        line: span.line,
                        col: span.col + (span.end - span.start).saturating_sub(member.len()),
                    };
                    let doc = format!("```flame\n{}.{}: {}\n```\nProperty of `{}`", struct_name, member, formatted_ty, struct_name);
                    self.insert_hover_info(member_span, doc.clone());
                    self.insert_hover_info(span.clone(), doc);
                    return ty;
                }

                let found_sig = self
                    .methods
                    .get(struct_name)
                    .and_then(|methods| methods.get(member))
                    .cloned();
                if let Some(sig) = found_sig {
                    let mut params_str = Vec::new();
                    for p in &sig.params {
                        let is_already_ref = matches!(p.ty, Type::Reference { .. });
                        let mut mods = String::new();
                        if p.is_ref && !is_already_ref {
                            mods.push('&');
                        }
                        if p.is_mut && !is_already_ref {
                            mods.push_str("mut ");
                        }
                        params_str.push(format!(
                            "{}{}: {}{}",
                            if p.is_mut && !p.is_ref { "mut " } else { "" },
                            p.name,
                            mods,
                            self.format_type(&p.ty)
                        ));
                    }
                    let ret_str = if sig.return_type == Type::Nil {
                        "".to_string()
                    } else {
                        format!(" -> {}", self.format_type(&sig.return_type))
                    };
                    let mut hover_str = format!(
                        "```flame\nfn {}({}){}\n```",
                        member,
                        params_str.join(", "),
                        ret_str
                    );
                    if let Some(doc) = &sig.hover_doc {
                        hover_str = format!("{}\n\n{}", hover_str, doc);
                    }
                    let member_span = Span {
                        start: span.end.saturating_sub(member.len()),
                        end: span.end,
                        line: span.line,
                        col: span.col + (span.end - span.start).saturating_sub(member.len()),
                    };
                    self.insert_hover_info(member_span, hover_str.clone());
                    self.insert_hover_info(span.clone(), hover_str);

                    return Type::Named("Function".into());
                }

                self.error(
                    format!(
                        "struct '{}' has no field or method '{}'",
                        struct_name, member
                    ),
                    span.clone(),
                    None,
                    None,
                );
                Type::Unknown
            }
            Type::Formula(ref fmap, ref docs) => {
                let ty = fmap.get(member).cloned().unwrap_or(Type::Unknown);
                if let Some(doc) = docs.get(member) {
                    self.insert_hover_info(span.clone(), doc.clone());
                }
                ty
            }
            Type::Quantity(_) | Type::Unit(_) => {
                if member == "value" {
                    Type::Float
                } else {
                    self.error(
                        format!(
                            "type '{}' has no field or method '{}'",
                            self.format_type(&inner_ty),
                            member
                        ),
                        span.clone(),
                        None,
                        None,
                    );
                    Type::Unknown
                }
            }
            Type::Vector(_)
            | Type::String
            | Type::Int
            | Type::Float
            | Type::Bool
            | Type::Tuple(_)
            | Type::Byte
            | Type::Union(_)
            | Type::Nullable(_) => Type::Named("Function".into()),
            Type::Unknown | Type::Named(_) => Type::Unknown,
            other => {
                self.error(
                    format!(
                        "cannot access member '{}' on value of type {}",
                        member,
                        self.format_type(&other)
                    ),
                    span.clone(),
                    None,
                    None,
                );
                Type::Unknown
            }
        }
    }

    pub(crate) fn infer_struct_init_type(
        &mut self,
        inner: &Expr,
        fields: &[(String, Expr)],
        span: &Span,
    ) -> Type {
        let base_ty = self.infer_expr_type(inner);
        match base_ty {
            Type::EnumVariant {
                enum_name,
                variant_name,
                tuple_items,
                struct_fields,
            } => {
                if !tuple_items.is_empty() {
                    self.error(
                        format!(
                            "variant '{}.{}' is a tuple variant, not a struct variant",
                            enum_name, variant_name
                        ),
                        span.clone(),
                        None,
                        None,
                    );
                    return Type::Unknown;
                }

                let mut seen = HashSet::new();
                for (field_name, field_expr) in fields {
                    seen.insert(field_name.clone());
                    let actual = self.infer_expr_type(field_expr);
                    if let Some(expected) = struct_fields.get(field_name) {
                        self.expect_assignable(
                            expected,
                            &actual,
                            &field_expr.span(),
                            "enum field initializer",
                        );
                    } else {
                        self.error(
                            format!(
                                "unknown field '{}' for variant '{}.{}'",
                                field_name, enum_name, variant_name
                            ),
                            field_expr.span(),
                            None,
                            None,
                        );
                    }
                }

                for required in struct_fields.keys() {
                    if !seen.contains(required) {
                        self.error(
                            format!(
                                "missing field '{}' for variant '{}.{}'",
                                required, enum_name, variant_name
                            ),
                            span.clone(),
                            None,
                            None,
                        );
                    }
                }

                Type::EnumVariant {
                    enum_name,
                    variant_name,
                    tuple_items: Vec::new(),
                    struct_fields,
                }
            }
            _ => {
                for (_, field_expr) in fields {
                    self.infer_expr_type(field_expr);
                }
                if let Expr::Identifier(name, _) = inner {
                    Type::Struct(name.clone())
                } else if let Type::Named(name) = base_ty {
                    Type::Struct(name)
                } else {
                    base_ty
                }
            }
        }
    }

    pub(crate) fn infer_call_type(
        &mut self,
        callee: &Expr,
        args: &[(Option<String>, Expr)],
        span: &Span,
    ) -> Type {
        let callee_ty = self.infer_expr_type(callee);
        let callee_name = match callee {
            Expr::Identifier(n, _) => Some(n.clone()),
            Expr::Dot(inner, member, _) => {
                if let Expr::Identifier(mod_name, _) = &**inner {
                    Some(format!("{}.{}", mod_name, member))
                } else {
                    None
                }
            }
            _ => None,
        };
        let func_sig = callee_name.as_ref().and_then(|n| self.functions.get(n).cloned());

        let is_anno_context_call = match callee_name.as_deref() {
            Some(
                "annotation.context"
                | "annoation.context"
                | "annotations.context"
                | "std.annotation.context"
                | "std.annotations.context",
            ) => true,
            Some("context") => {
                self.lookup_var("context").is_none() && self.functions.contains_key("context")
            }
            _ => false,
        };

        if is_anno_context_call && !self.in_annotation_decl && !self.in_expect_panic {
            self.error(
                "annotation.context() can only be called inside of a custom annotation".to_string(),
                span.clone(),
                Some("AnnotationContext provides reflection and transformation of the annotated target.".to_string()),
                Some("Move this call inside an `annotation Name(...) { ... }` declaration.".to_string()),
            );
        }

        if let Type::Function(params, ret) = &callee_ty {
            let (min_args, max_args) = if let Some(sig) = &func_sig {
                (
                    sig.params.iter().filter(|p| !p.has_default).count(),
                    sig.params.len(),
                )
            } else {
                let min_count = params.iter().take_while(|p| !matches!(p, Type::Nullable(_))).count();
                (min_count, params.len())
            };

            if args.len() < min_args || args.len() > max_args {
                self.error(
                    if min_args == max_args {
                        format!(
                            "function expects {} argument(s), got {}",
                            params.len(),
                            args.len()
                        )
                    } else {
                        format!(
                            "function expects between {} and {} arguments, got {}",
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
            for (idx, expected) in params.iter().enumerate() {
                if let Some((_, arg)) = args.get(idx) {
                    let actual = self.infer_expr_type(arg);
                    self.expect_assignable(expected, &actual, &arg.span(), "function argument");
                }
            }

            // Statically evaluate `unit.Equation` to capture exact units
            if let Expr::Dot(_, member, _) = callee {
                if member == "Equation" && params.len() == 3 {
                    let mut is_literal = true;
                    let mut vals = vec![];
                    for arg in args {
                        if let Expr::Literal(LiteralValue::Int(v), _) = &arg.1 {
                            vals.push(*v as i32);
                        } else if let Expr::Unary(UnaryOp::Neg, inner_arg, _) = &arg.1 {
                            if let Expr::Literal(LiteralValue::Int(v), _) = &**inner_arg {
                                vals.push(-(*v as i32));
                            } else {
                                is_literal = false;
                            }
                        } else {
                            is_literal = false;
                        }
                    }
                    if is_literal && vals.len() == 3 {
                        let mut map = HashMap::new();
                        if vals[0] != 0 {
                            map.insert("kg".to_string(), vals[0]);
                        }
                        if vals[1] != 0 {
                            map.insert("m".to_string(), vals[1]);
                        }
                        if vals[2] != 0 {
                            map.insert("s".to_string(), vals[2]);
                        }
                        return Type::Unit(map);
                    }
                }
            }

            return *ret.clone();
        }

        if let Expr::Identifier(name, id_span) = callee {
            if let Some(sig) = self.functions.get(name).cloned() {
                self.check_call_args(&sig.params, args, span, name);

                let mut params_str = Vec::new();
                for p in &sig.params {
                    let is_already_ref = matches!(p.ty, Type::Reference { .. });
                    let mut mods = String::new();
                    if p.is_ref && !is_already_ref {
                        mods.push('&');
                    }
                    if p.is_mut && !is_already_ref {
                        mods.push_str("mut ");
                    }
                    params_str.push(format!(
                        "{}{}: {}{}",
                        if p.is_mut && !p.is_ref { "mut " } else { "" },
                        p.name,
                        mods,
                        self.format_type(&p.ty)
                    ));
                }

                let ret_str = if sig.return_type == Type::Nil {
                    "".to_string()
                } else {
                    format!(" -> {}", self.format_type(&sig.return_type))
                };
                let mut hover_str = format!(
                    "```flame\nfn {}({}){}\n```",
                    name,
                    params_str.join(", "),
                    ret_str
                );
                if let Some(doc) = &sig.hover_doc {
                    hover_str = format!("{}\n\n{}", hover_str, doc);
                }
                self.insert_hover_info(id_span.clone(), hover_str);

                return sig.return_type;
            }

            if let Some(struct_info) = self.structs.get(name).cloned() {
                self.check_struct_constructor_args(name, &struct_info, args, span);
                let mut hover_str = format!("```flame\nstruct {}\n```", name);
                if let Some(doc) = struct_info.hover_doc {
                    hover_str = format!("{}\n\n{}", hover_str, doc);
                }
                self.insert_hover_info(id_span.clone(), hover_str);
                return Type::Struct(name.clone());
            }

            // Check if it's a globally available enum variant (like Ok, Err, Some, None)
            let mut found_variant = None;
            for (enum_name, enum_info) in &self.enums {
                if let Some(variant) = enum_info.variants.get(name) {
                    found_variant = Some((enum_name.clone(), variant.clone()));
                    break;
                }
            }

            if let Some((enum_name, variant)) = found_variant {
                if variant.struct_fields.is_empty() && !variant.tuple_items.is_empty() {
                    if variant.tuple_items.len() != args.len() {
                        self.error(
                            format!(
                                "enum constructor '{}' expects {} argument(s), got {}",
                                name,
                                variant.tuple_items.len(),
                                args.len()
                            ),
                            span.clone(),
                            None,
                            None,
                        );
                    }
                    for (idx, expected) in variant.tuple_items.iter().enumerate() {
                        if let Some((_, arg)) = args.get(idx) {
                            let actual = self.infer_expr_type(arg);
                            self.expect_assignable(
                                expected,
                                &actual,
                                &arg.span(),
                                "enum constructor argument",
                            );
                        }
                    }
                    let mut hover_str = format!("```flame\n{}::{}\n```", enum_name, name);
                    if let Some(doc) = variant.hover_doc {
                        hover_str = format!("{}\n\n{}", hover_str, doc);
                    }
                    self.insert_hover_info(id_span.clone(), hover_str);
                    return Type::EnumVariant {
                        enum_name: enum_name.clone(),
                        variant_name: name.clone(),
                        tuple_items: variant.tuple_items.clone(),
                        struct_fields: HashMap::new(),
                    };
                }
            }
        }

        if let Expr::Dot(inner, member, _) = callee {
            let mut inner_ty = self.infer_expr_type(inner);
            if let Type::Reference {
                inner: ref_inner, ..
            } = inner_ty
            {
                inner_ty = *ref_inner;
            }

            match member.as_str() {
                "type" => return Type::String,
                "toString" => return Type::String,
                "toHex" => return Type::String,
                "toInt" | "tryInt" => return Type::Int,
                "toFloat" | "toDouble" | "tryFloat" => return Type::Float,
                "toBool" | "tryBool" => return Type::Bool,
                "toChar" => return Type::String,
                "toByte" => return Type::Byte,
                "assertEq" => return Type::Nil,
                "toJson" => return Type::String,
                "fromJson" | "fromByte" | "fromBytes" => return inner_ty.clone(),
                "tryRecv" | "try_recv" => return Type::Unknown,
                "recvTimeout" | "recv_timeout" => return Type::Unknown,
                "isEmpty" | "is_empty" => return Type::Bool,
                "clone" => return inner_ty.clone(),
                _ => {}
            }

            if let Type::Formula(ref fmap, ref fdocs) = inner_ty {
                if let Some(member_ty) = fmap.get(member).cloned() {
                    let doc_opt = fdocs.get(member).cloned();
                    match member_ty {
                        Type::Function(ref param_types, ref ret_type) => {
                            let callee_span = callee.span();
                            let dot_member_span = Span {
                                start: callee_span.end.saturating_sub(member.len()),
                                end: callee_span.end,
                                line: callee_span.line,
                                col: callee_span.col + (callee_span.end - callee_span.start).saturating_sub(member.len()),
                            };
                            let params_str = param_types
                                .iter()
                                .enumerate()
                                .map(|(i, t)| format!("arg{}: {}", i, self.format_type(t)))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let ret_str = if **ret_type == Type::Nil {
                                "".to_string()
                            } else {
                                format!(" -> {}", self.format_type(ret_type))
                            };
                            let mut hover_str = format!("```flame\nfn {}({}){}\n```", member, params_str, ret_str);
                            if let Some(doc) = doc_opt {
                                hover_str = format!("{}\n\n{}", hover_str, doc);
                            }
                            self.insert_hover_info(dot_member_span, hover_str.clone());
                            self.insert_hover_info(callee_span, hover_str.clone());
                            self.insert_hover_info(span.clone(), hover_str);
                            return *ret_type.clone();
                        }
                        _ => {
                            return member_ty;
                        }
                    }
                }
            }

            if let Type::Named(name) = &inner_ty {
                if name.starts_with("plugin:") || name.starts_with("module:") {
                    let prefix = if name.starts_with("plugin:") {
                        &name["plugin:".len()..]
                    } else {
                        &name["module:".len()..]
                    };

                    let prefixed_member = format!("{}.{}", prefix, member);
                    if let Some(sig) = self.functions.get(&prefixed_member).cloned() {
                        self.check_call_args(&sig.params, args, span, member);
                        let params_str = sig
                            .params
                            .iter()
                            .map(|p| format!("{}: {}", p.name, self.format_type(&p.ty)))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let ret_str = if sig.return_type == Type::Nil {
                            "".to_string()
                        } else {
                            format!(" -> {}", self.format_type(&sig.return_type))
                        };
                        let fallback =
                            format!("```flame\nfn {}({}){}\n```", member, params_str, ret_str);
                        if let Some(doc) = &sig.hover_doc {
                            self.insert_hover_info(
                                span.clone(),
                                format!("{}\n\n{}", fallback, doc),
                            );
                        } else {
                            self.insert_hover_info(span.clone(), fallback);
                        }
                        return sig.return_type.clone();
                    }

                    if name.starts_with("plugin:") {
                        if let Some(funcs) = self.plugin_functions.get(prefix) {
                            if let Some(sig) = funcs.get(member).cloned() {
                                self.check_call_args(&sig.params, args, span, member);
                                let params_str = sig
                                    .params
                                    .iter()
                                    .map(|p| format!("{}: {}", p.name, self.format_type(&p.ty)))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let ret_str = if sig.return_type == Type::Nil {
                                    "".to_string()
                                } else {
                                    format!(" -> {}", self.format_type(&sig.return_type))
                                };
                                let fallback = format!(
                                    "```flame\nfn {}({}){}\n```",
                                    member, params_str, ret_str
                                );
                                if let Some(doc) = &sig.hover_doc {
                                    if doc.trim().starts_with("```") {
                                        self.insert_hover_info(span.clone(), doc.clone());
                                    } else {
                                        self.insert_hover_info(
                                            span.clone(),
                                            format!("{}\n\n{}", fallback, doc),
                                        );
                                    }
                                } else {
                                    self.insert_hover_info(span.clone(), fallback);
                                }
                                return sig.return_type.clone();
                            }
                        }
                        if let Some(methods) = self.plugin_methods.get(prefix) {
                            if let Some(ty) = methods.get(member) {
                                return ty.clone();
                            }
                        }
                    } else {
                        let ret_ty = match (prefix, member.as_str()) {
                            ("fs", "read") => Type::String,
                            ("fs", "readDir") => Type::Vector(Box::new(Type::String)),
                            ("fs", "readBytes") => Type::Byte,
                            ("fs", "open") => Type::Unknown,
                            ("thread", "sleep") => Type::Nil,
                            ("thread", "yield") | ("thread", "yield_now") => Type::Nil,
                            ("thread", "id") => Type::String,
                            ("thread", "channel") => {
                                Type::Tuple(vec![Type::Unknown, Type::Unknown])
                            }
                            ("byte", "readBytes") => Type::Byte,
                            ("byte", "readByte") => Type::Byte,
                            ("byte", "readByteAt") => Type::Byte,
                            _ => Type::Unknown,
                        };
                        if !matches!(ret_ty, Type::Unknown) {
                            return ret_ty;
                        }
                    }
                }
            }

            let struct_name_opt = match &inner_ty {
                Type::Struct(name) => Some(name.clone()),
                Type::Named(name) if self.methods.contains_key(name) || self.structs.contains_key(name) => Some(name.clone()),
                _ => None,
            };

            if let Some(ref struct_name) = struct_name_opt {
                let sig_opt = self
                    .methods
                    .get(struct_name)
                    .and_then(|methods| methods.get(member))
                    .cloned();

                if let Some(sig) = sig_opt {
                    let params_to_check = if sig.is_static {
                        &sig.params[..]
                    } else if !sig.params.is_empty() {
                        &sig.params[1..]
                    } else {
                        &[]
                    };
                    self.check_call_args(params_to_check, args, span, member);

                    let mut params_str = Vec::new();
                    for p in &sig.params {
                        let is_already_ref = matches!(p.ty, Type::Reference { .. });
                        let mut mods = String::new();
                        if p.is_ref && !is_already_ref {
                            mods.push('&');
                        }
                        if p.is_mut && !is_already_ref {
                            mods.push_str("mut ");
                        }
                        params_str.push(format!(
                            "{}{}: {}{}",
                            if p.is_mut && !p.is_ref { "mut " } else { "" },
                            p.name,
                            mods,
                            self.format_type(&p.ty)
                        ));
                    }
                    let ret_str = if sig.return_type == Type::Nil {
                        "".to_string()
                    } else {
                        format!(" -> {}", self.format_type(&sig.return_type))
                    };
                    let mut hover_str = format!(
                        "```flame\nfn {}({}){}\n```",
                        member,
                        params_str.join(", "),
                        ret_str
                    );
                    if let Some(doc) = &sig.hover_doc {
                        hover_str = format!("{}\n\n{}", hover_str, doc);
                    }
                    let callee_span = callee.span();
                    let dot_member_span = Span {
                        start: callee_span.end.saturating_sub(member.len()),
                        end: callee_span.end,
                        line: callee_span.line,
                        col: callee_span.col + (callee_span.end - callee_span.start).saturating_sub(member.len()),
                    };
                    self.insert_hover_info(dot_member_span, hover_str.clone());
                    self.insert_hover_info(callee_span, hover_str.clone());
                    self.insert_hover_info(span.clone(), hover_str);

                    return sig.return_type;
                }

                self.error(
                    format!("struct '{}' has no method '{}'", struct_name, member),
                    span.clone(),
                    None,
                    None,
                );
                return Type::Unknown;
            }

            if let Type::Byte = &inner_ty {
                match member.as_str() {
                    "toHex" | "toBase64" | "toUtf8" | "tryUtf8" => {
                        self.check_call_args(&[], args, span, member);
                        return match member.as_str() {
                            "toHex" | "toBase64" | "toUtf8" => Type::String,
                            "tryUtf8" => Type::Named("String?".to_string()),
                            _ => Type::Unknown,
                        };
                    }
                    "concat" => {
                        self.check_call_args(
                            &[ParamInfo {
                                name: "other".into(),
                                ty: Type::Byte,
                                is_ref: false,
                                is_mut: false,
                                has_default: false,
                            }],
                            args,
                            span,
                            member,
                        );
                        return Type::Byte;
                    }
                    "len" => {
                        self.check_call_args(&[], args, span, member);
                        return Type::Int;
                    }
                    "type" => {
                        self.check_call_args(&[], args, span, member);
                        return Type::String;
                    }
                    _ => {
                        self.error(
                            format!("Bytes has no method '{}'", member),
                            span.clone(),
                            None,
                            None,
                        );
                        return Type::Unknown;
                    }
                }
            }

            if let Type::Vector(element_ty) = &inner_ty {
                match member.as_str() {
                    "push" => {
                        self.check_call_args(
                            &[ParamInfo {
                                name: "item".into(),
                                ty: *element_ty.clone(),
                                is_ref: false,
                                is_mut: false,
                                has_default: false,
                            }],
                            args,
                            span,
                            member,
                        );
                        return Type::Nil;
                    }
                    "pop" => {
                        self.check_call_args(&[], args, span, member);
                        return *element_ty.clone();
                    }
                    "len" => {
                        self.check_call_args(&[], args, span, member);
                        return Type::Int;
                    }
                    "filter" => {
                        let cb_ty = Type::Function(vec![*element_ty.clone()], Box::new(Type::Bool));
                        self.check_call_args(
                            &[ParamInfo {
                                name: "cb".into(),
                                ty: cb_ty,
                                is_ref: false,
                                is_mut: false,
                                has_default: false,
                            }],
                            args,
                            span,
                            member,
                        );
                        return Type::Vector(element_ty.clone());
                    }
                    "map" => {
                        if let Some((_, arg)) = args.get(0) {
                            let arg_ty = self.infer_expr_type(arg);
                            if let Type::Function(_, ret) = arg_ty {
                                return Type::Vector(ret);
                            }
                        }
                        return Type::Unknown;
                    }
                    "type" | "toHex" | "toBase64" | "concat" | "assertEq" => {
                        return Type::Unknown;
                    }
                    _ => {
                        self.error(
                            format!("array has no method '{}'", member),
                            span.clone(),
                            None,
                            None,
                        );
                        return Type::Unknown;
                    }
                }
            }

            if let Type::Enum(enum_name) = inner_ty {
                if let Some(variant) = self
                    .enums
                    .get(&enum_name)
                    .and_then(|ei| ei.variants.get(member))
                    .cloned()
                {
                    if variant.struct_fields.is_empty() && !variant.tuple_items.is_empty() {
                        if variant.tuple_items.len() != args.len() {
                            self.error(
                                format!(
                                    "enum constructor '{}.{}' expects {} argument(s), got {}",
                                    enum_name,
                                    member,
                                    variant.tuple_items.len(),
                                    args.len()
                                ),
                                span.clone(),
                                None,
                                None,
                            );
                        }
                        for (idx, expected) in variant.tuple_items.iter().enumerate() {
                            if let Some((_, arg)) = args.get(idx) {
                                let actual = self.infer_expr_type(arg);
                                self.expect_assignable(
                                    expected,
                                    &actual,
                                    &arg.span(),
                                    "enum constructor argument",
                                );
                            }
                        }
                        return Type::EnumVariant {
                            enum_name,
                            variant_name: member.clone(),
                            tuple_items: variant.tuple_items.clone(),
                            struct_fields: HashMap::new(),
                        };
                    }
                }
            }
        }

        for (_, arg) in args {
            self.infer_expr_type(arg);
        }
        let _ = self.infer_expr_type(callee);
        Type::Unknown
    }


}
