use super::types::Type;
use crate::diagnostics::Diagnostic;
use crate::lexer::Span;
use crate::parser::*;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Projection {
    Field(String),
    Index(String),
    Deref,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub root: String,
    pub projections: Vec<Projection>,
}

impl Place {
    pub fn var(name: String) -> Self {
        Self {
            root: name,
            projections: Vec::new(),
        }
    }

    pub fn root_var(&self) -> &str {
        &self.root
    }

    pub fn overlaps(&self, other: &Place) -> bool {
        if self.root != other.root {
            return false;
        }
        let min_len = self.projections.len().min(other.projections.len());
        for i in 0..min_len {
            match (&self.projections[i], &other.projections[i]) {
                (Projection::Field(f1), Projection::Field(f2)) => {
                    if f1 != f2 {
                        return false;
                    }
                }
                (Projection::Index(i1), Projection::Index(i2)) => {
                    if i1 != i2 {
                        if let (Ok(n1), Ok(n2)) = (i1.parse::<i64>(), i2.parse::<i64>()) {
                            if n1 != n2 {
                                return false;
                            }
                        }
                    }
                }
                (Projection::Deref, Projection::Deref) => {}
                _ => {}
            }
        }
        true
    }

    pub fn to_display_string(&self) -> String {
        let mut s = self.root.clone();
        for proj in &self.projections {
            match proj {
                Projection::Field(f) => {
                    s.push('.');
                    s.push_str(f);
                }
                Projection::Index(idx) => {
                    s.push('[');
                    s.push_str(idx);
                    s.push(']');
                }
                Projection::Deref => {
                    s = format!("*{}", s);
                }
            }
        }
        s
    }
}

pub fn expr_to_place(expr: &Expr) -> Option<Place> {
    match expr {
        Expr::Identifier(name, _) => Some(Place::var(name.clone())),
        Expr::Dot(inner, member, _) | Expr::SafeDot(inner, member, _) => {
            let mut place = expr_to_place(inner)?;
            place.projections.push(Projection::Field(member.clone()));
            Some(place)
        }
        Expr::Index(inner, idx_expr, _) => {
            let mut place = expr_to_place(inner)?;
            let idx_str = match &**idx_expr {
                Expr::Literal(LiteralValue::Int(n), _) => n.to_string(),
                Expr::Identifier(name, _) => name.clone(),
                _ => format!("{:?}", idx_expr),
            };
            place.projections.push(Projection::Index(idx_str));
            Some(place)
        }
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct Loan {
    pub place: Place,
    pub is_mut: bool,
    pub loan_span: Span,
    pub ref_var: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LocalVar {
    pub name: String,
    pub is_mut: bool,
    pub is_param: bool,
    pub is_ref: bool,
    pub decl_span: Span,
    pub ty: Type,
    pub moved_at: Option<Span>,
    pub conditionally_moved: Option<Span>,
    pub points_to_local: Option<String>,
}

pub fn is_copy_type(ty: &Type) -> bool {
    match ty {
        Type::Int | Type::Float | Type::Bool | Type::Byte | Type::Nil => true,
        Type::Reference { mutable: false, .. } => true,
        _ => false,
    }
}

pub fn parse_basic_type_name(name: &str) -> Type {
    match name.trim() {
        "Int" | "i64" | "i32" | "usize" | "isize" => Type::Int,
        "Float" | "f64" | "f32" => Type::Float,
        "String" | "str" => Type::String,
        "Bool" | "bool" => Type::Bool,
        "Byte" | "u8" => Type::Byte,
        "Nil" | "void" | "()" => Type::Nil,
        s if s.starts_with("&mut ") => Type::Reference {
            inner: Box::new(parse_basic_type_name(&s[5..])),
            mutable: true,
        },
        s if s.starts_with('&') => Type::Reference {
            inner: Box::new(parse_basic_type_name(&s[1..])),
            mutable: false,
        },
        s => Type::Named(s.to_string()),
    }
}

pub fn infer_simple_type(
    expr: &Expr,
    locals: &HashMap<String, LocalVar>,
    known_types: &HashMap<String, Type>,
) -> Type {
    match expr {
        Expr::Literal(LiteralValue::Int(_), _) => Type::Int,
        Expr::Literal(LiteralValue::Float(_), _) => Type::Float,
        Expr::Literal(LiteralValue::String(_), _) => Type::String,
        Expr::Literal(LiteralValue::Bool(_), _) => Type::Bool,
        Expr::Literal(LiteralValue::Nil, _) => Type::Nil,
        Expr::Identifier(name, _) => {
            if let Some(t) = known_types.get(name) {
                t.clone()
            } else if let Some(local) = locals.get(name) {
                local.ty.clone()
            } else {
                Type::Unknown
            }
        }
        Expr::Borrow(inner, is_mut, _) => {
            let inner_ty = infer_simple_type(inner, locals, known_types);
            Type::Reference {
                inner: Box::new(inner_ty),
                mutable: *is_mut,
            }
        }
        Expr::StructInit(callee, _, _) => {
            if let Expr::Identifier(name, _) = &**callee {
                Type::Struct(name.clone())
            } else {
                Type::Unknown
            }
        }
        Expr::Binary(left, op, _, _) => match op {
            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::And
            | BinaryOp::Or => Type::Bool,
            _ => infer_simple_type(left, locals, known_types),
        },
        _ => Type::Unknown,
    }
}

pub struct BorrowChecker<'a> {
    pub filepath: &'a str,
    pub diagnostics: Vec<Diagnostic>,
    pub locals: HashMap<String, LocalVar>,
    pub loans: Vec<Loan>,
    pub current_func_return_is_ref: bool,
    pub known_types: HashMap<String, Type>,
}

impl<'a> BorrowChecker<'a> {
    pub fn new(filepath: &'a str) -> Self {
        Self {
            filepath,
            diagnostics: Vec::new(),
            locals: HashMap::new(),
            loans: Vec::new(),
            current_func_return_is_ref: false,
            known_types: HashMap::new(),
        }
    }

    pub fn with_known_types(mut self, known_types: HashMap<String, Type>) -> Self {
        self.known_types = known_types;
        self
    }

    pub fn check_function(
        &mut self,
        _name: &str,
        params: &[Param],
        return_type: &Option<String>,
        stmts: &[Stmt],
    ) {
        let ret_is_ref = return_type
            .as_ref()
            .map(|r| r.trim().starts_with('&'))
            .unwrap_or(false);
        self.current_func_return_is_ref = ret_is_ref;

        for p in params {
            let p_ty = if let Some(t) = self.known_types.get(&p.name) {
                t.clone()
            } else {
                parse_basic_type_name(&p.type_name)
            };
            self.locals.insert(
                p.name.clone(),
                LocalVar {
                    name: p.name.clone(),
                    is_mut: p.is_mut,
                    is_param: true,
                    is_ref: p.is_ref || p.type_name.trim().starts_with('&'),
                    decl_span: Span::dummy(),
                    ty: p_ty,
                    moved_at: None,
                    conditionally_moved: None,
                    points_to_local: None,
                },
            );
        }

        self.check_block(stmts);
    }

    pub fn check_block(&mut self, stmts: &[Stmt]) {
        for (stmt_idx, stmt) in stmts.iter().enumerate() {
            let remaining_stmts = &stmts[stmt_idx + 1..];
            self.check_stmt(stmt, remaining_stmts, stmt_idx + 1 == stmts.len());
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt, remaining_stmts: &[Stmt], is_last: bool) {
        match stmt {
            Stmt::LetDecl {
                name,
                is_mut,
                type_ann,
                value,
                span: _,
                name_span,
                ..
            }
            | Stmt::ConstDecl {
                name,
                is_mut,
                type_ann,
                value,
                span: _,
                name_span,
                ..
            } => {
                // 1. Check all reads in value for use-after-move
                self.check_reads_in_expr(value);

                // 2. Resolve variable type
                let ty = if let Some(type_name) = type_ann {
                    parse_basic_type_name(type_name)
                } else if let Some(t) = self.known_types.get(name) {
                    t.clone()
                } else {
                    infer_simple_type(value, &self.locals, &self.known_types)
                };

                // 3. Check if value is a move of a local variable
                if let Expr::Identifier(src_name, src_span) = value {
                    if let Some(src_local) = self.locals.get(src_name).cloned() {
                        if !src_local.is_param && !src_local.is_ref && !is_copy_type(&src_local.ty)
                        {
                            let place = Place::var(src_name.clone());
                            if let Some((loan, use_span)) =
                                self.find_active_loan(&place, remaining_stmts)
                            {
                                self.diagnostics.push(
                                    Diagnostic::new_error(
                                        format!(
                                            "[E0505] cannot move out of '{}' because it is borrowed",
                                            src_name
                                        ),
                                        self.filepath.to_string(),
                                        src_span.clone(),
                                        Some("cannot move out of borrowed variable".to_string()),
                                        None,
                                    )
                                    .with_note(format!(
                                        "borrow of '{}' occurs at line {}; borrow later used at line {}",
                                        loan.place.to_display_string(), loan.loan_span.line, use_span.line
                                    )),
                                );
                            } else {
                                self.locals.get_mut(src_name).unwrap().moved_at =
                                    Some(src_span.clone());
                            }
                        }
                    }
                }

                // 4. Check if value is a borrow expression
                if let Expr::Borrow(inner, is_borrow_mut, borrow_span) = value {
                    if let Some(place) = expr_to_place(inner) {
                        let root = place.root_var().to_string();
                        self.check_borrow(
                            &place,
                            *is_borrow_mut,
                            borrow_span,
                            remaining_stmts,
                            Some(name.clone()),
                        );
                        let points_to = if self.locals.contains_key(&root) {
                            Some(root.clone())
                        } else {
                            None
                        };
                        let inner_ty = if let Some(loc) = self.locals.get(&root) {
                            loc.ty.clone()
                        } else {
                            Type::Unknown
                        };
                        self.locals.insert(
                            name.clone(),
                            LocalVar {
                                name: name.clone(),
                                is_mut: *is_mut,
                                is_param: false,
                                is_ref: true,
                                decl_span: name_span.clone(),
                                ty: Type::Reference {
                                    inner: Box::new(inner_ty),
                                    mutable: *is_borrow_mut,
                                },
                                moved_at: None,
                                conditionally_moved: None,
                                points_to_local: points_to,
                            },
                        );
                        return;
                    }
                }

                // Normal local variable definition
                self.locals.insert(
                    name.clone(),
                    LocalVar {
                        name: name.clone(),
                        is_mut: *is_mut,
                        is_param: false,
                        is_ref: false,
                        decl_span: name_span.clone(),
                        ty,
                        moved_at: None,
                        conditionally_moved: None,
                        points_to_local: None,
                    },
                );
            }

            Stmt::ReturnStmt(val, _) => {
                if let Some(expr) = val {
                    self.check_reads_in_expr(expr);
                    self.check_return_ref(expr);
                }
            }

            Stmt::ExprStmt(expr) => {
                self.check_reads_in_expr(expr);
                self.check_assignments_in_expr(expr, remaining_stmts);
                if self.current_func_return_is_ref && is_last {
                    self.check_return_ref(expr);
                }
            }

            Stmt::IfStmt {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                // 1. Check reads in condition against current state
                self.check_reads_in_expr(cond);

                // 2. Snapshot state before entering then_branch
                let pre_if_locals = self.locals.clone();
                let pre_if_loans = self.loans.clone();

                // 3. Analyze then_branch starting from snapshot
                self.check_block(then_branch);
                let then_exit_locals = self.locals.clone();
                let then_exit_loans = self.loans.clone();

                // 4. Analyze else_branch starting from PRE-IF snapshot
                let (else_exit_locals, else_exit_loans) = if let Some(eb) = else_branch {
                    self.locals = pre_if_locals.clone();
                    self.loans = pre_if_loans.clone();
                    self.check_block(eb);
                    (self.locals.clone(), self.loans.clone())
                } else {
                    (pre_if_locals.clone(), pre_if_loans.clone())
                };

                // 5. Compute join of then_exit and else_exit
                let mut joined_locals = pre_if_locals;
                for (name, pre_local) in &mut joined_locals {
                    let then_loc = then_exit_locals.get(name);
                    let else_loc = else_exit_locals.get(name);

                    let then_moved = then_loc.and_then(|l| l.moved_at.clone());
                    let else_moved = else_loc.and_then(|l| l.moved_at.clone());

                    let then_cond = then_loc.and_then(|l| l.conditionally_moved.clone());
                    let else_cond = else_loc.and_then(|l| l.conditionally_moved.clone());

                    if then_moved.is_some() && else_moved.is_some() {
                        pre_local.moved_at = then_moved;
                        pre_local.conditionally_moved = None;
                    } else if then_moved.is_some()
                        || else_moved.is_some()
                        || then_cond.is_some()
                        || else_cond.is_some()
                    {
                        let span = then_moved.or(else_moved).or(then_cond).or(else_cond);
                        pre_local.conditionally_moved = span;
                    }
                }

                let mut joined_loans = pre_if_loans;
                for loan in then_exit_loans
                    .into_iter()
                    .chain(else_exit_loans.into_iter())
                {
                    if !joined_loans
                        .iter()
                        .any(|l| l.place == loan.place && l.loan_span == loan.loan_span)
                    {
                        joined_loans.push(loan);
                    }
                }

                self.locals = joined_locals;
                self.loans = joined_loans;
            }

            Stmt::WhileStmt { cond, body, .. } => {
                self.check_reads_in_expr(cond);

                let pre_loop_locals = self.locals.clone();
                self.check_block(body);

                // Any non-Copy variable declared before the loop moved inside the loop
                // causes a use-after-move on iteration 2
                for (name, pre_local) in &pre_loop_locals {
                    if !pre_local.is_param && !pre_local.is_ref && !is_copy_type(&pre_local.ty) {
                        if let Some(post_local) = self.locals.get(name) {
                            if let Some(m_span) = &post_local.moved_at {
                                if pre_local.moved_at.is_none() {
                                    self.diagnostics.push(
                                        Diagnostic::new_error(
                                            format!("cannot move out of '{}' in a loop", name),
                                            self.filepath.to_string(),
                                            m_span.clone(),
                                            Some("cannot move out of variable in a loop".to_string()),
                                            Some("try borrowing with '&' or cloning with '.clone()' instead".to_string()),
                                        )
                                        .with_note(format!("'{}' is moved here, which would cause a use-after-move on subsequent iterations", name)),
                                    );
                                }
                            }
                        }
                    }
                }

                // After loop, mark any variable moved in loop body as conditionally moved
                for (name, pre_local) in &mut self.locals {
                    if pre_local.moved_at.is_some()
                        && pre_loop_locals
                            .get(name)
                            .map_or(false, |l| l.moved_at.is_none())
                    {
                        let span = pre_local.moved_at.take();
                        pre_local.conditionally_moved = span;
                    }
                }
            }

            Stmt::ForStmt { iterable, body, .. } => {
                self.check_reads_in_expr(iterable);

                let pre_loop_locals = self.locals.clone();
                self.check_block(body);

                for (name, pre_local) in &pre_loop_locals {
                    if !pre_local.is_param && !pre_local.is_ref && !is_copy_type(&pre_local.ty) {
                        if let Some(post_local) = self.locals.get(name) {
                            if let Some(m_span) = &post_local.moved_at {
                                if pre_local.moved_at.is_none() {
                                    self.diagnostics.push(
                                        Diagnostic::new_error(
                                            format!("cannot move out of '{}' in a loop", name),
                                            self.filepath.to_string(),
                                            m_span.clone(),
                                            Some("cannot move out of variable in a loop".to_string()),
                                            Some("try borrowing with '&' or cloning with '.clone()' instead".to_string()),
                                        )
                                        .with_note(format!("'{}' is moved here, which would cause a use-after-move on subsequent iterations", name)),
                                    );
                                }
                            }
                        }
                    }
                }
            }

            Stmt::MatchStmt { target, arms, .. } => {
                self.check_reads_in_expr(target);
                let pre_match_locals = self.locals.clone();

                let mut arm_exit_locals = Vec::new();
                for arm in arms {
                    self.locals = pre_match_locals.clone();
                    self.check_reads_in_expr(&arm.body);
                    arm_exit_locals.push(self.locals.clone());
                }

                if !arms.is_empty() {
                    let mut joined_locals = pre_match_locals;
                    for (name, pre_local) in &mut joined_locals {
                        let all_moved = arm_exit_locals
                            .iter()
                            .all(|l| l.get(name).map_or(false, |v| v.moved_at.is_some()));
                        let any_moved = arm_exit_locals.iter().any(|l| {
                            l.get(name).map_or(false, |v| {
                                v.moved_at.is_some() || v.conditionally_moved.is_some()
                            })
                        });

                        if all_moved {
                            pre_local.moved_at = arm_exit_locals[0]
                                .get(name)
                                .and_then(|v| v.moved_at.clone());
                        } else if any_moved {
                            let span = arm_exit_locals.iter().find_map(|l| {
                                l.get(name).and_then(|v| {
                                    v.moved_at.clone().or_else(|| v.conditionally_moved.clone())
                                })
                            });
                            pre_local.conditionally_moved = span;
                        }
                    }
                    self.locals = joined_locals;
                }
            }

            _ => {}
        }
    }

    fn check_borrow(
        &mut self,
        place: &Place,
        is_borrow_mut: bool,
        borrow_span: &Span,
        remaining_stmts: &[Stmt],
        ref_var: Option<String>,
    ) {
        let root = place.root_var();
        if let Some(local) = self.locals.get(root) {
            if let Some(m_span) = &local.moved_at {
                self.diagnostics.push(
                    Diagnostic::new_error(
                        format!("use of moved value: '{}'", root),
                        self.filepath.to_string(),
                        borrow_span.clone(),
                        Some("value used here after move".to_string()),
                        None,
                    )
                    .with_note(format!("value moved here at line {}", m_span.line)),
                );
                return;
            }
            if let Some(c_span) = &local.conditionally_moved {
                self.diagnostics.push(
                    Diagnostic::new_error(
                        format!("use of possibly-moved value: '{}'", root),
                        self.filepath.to_string(),
                        borrow_span.clone(),
                        Some("value may have been moved in a previous branch".to_string()),
                        None,
                    )
                    .with_note(format!(
                        "value conditionally moved here at line {}",
                        c_span.line
                    )),
                );
                return;
            }
            if is_borrow_mut && !local.is_mut && !local.is_ref {
                self.diagnostics.push(Diagnostic::new_error(
                    format!(
                        "cannot borrow immutable local variable '{}' as mutable",
                        root
                    ),
                    self.filepath.to_string(),
                    borrow_span.clone(),
                    Some("cannot borrow as mutable".to_string()),
                    Some(format!(
                        "try declaring 'let mut {}' to make it mutable",
                        root
                    )),
                ));
            }
        }

        let place_display = place.to_display_string();

        for existing in &self.loans {
            if existing.place.overlaps(place) {
                if let Some(ref_name) = &existing.ref_var {
                    if let Some(use_span) = find_use_in_stmts(remaining_stmts, ref_name) {
                        let existing_display = existing.place.to_display_string();
                        if existing.is_mut && is_borrow_mut {
                            self.diagnostics.push(
                                Diagnostic::new_error(
                                    format!(
                                        "cannot borrow '{}' as mutable more than once at a time",
                                        place_display
                                    ),
                                    self.filepath.to_string(),
                                    borrow_span.clone(),
                                    Some("second mutable borrow occurs here".to_string()),
                                    None,
                                )
                                .with_note(format!(
                                    "first mutable borrow occurs here at line {}; first borrow later used at line {}",
                                    existing.loan_span.line, use_span.line
                                )),
                            );
                        } else if !existing.is_mut && is_borrow_mut {
                            self.diagnostics.push(
                                Diagnostic::new_error(
                                    format!(
                                        "cannot borrow '{}' as mutable because it is also borrowed as immutable",
                                        place_display
                                    ),
                                    self.filepath.to_string(),
                                    borrow_span.clone(),
                                    Some("mutable borrow occurs here".to_string()),
                                    None,
                                )
                                .with_note(format!(
                                    "immutable borrow of '{}' occurs here at line {}; immutable borrow later used at line {}",
                                    existing_display, existing.loan_span.line, use_span.line
                                )),
                            );
                        } else if existing.is_mut && !is_borrow_mut {
                            self.diagnostics.push(
                                Diagnostic::new_error(
                                    format!(
                                        "cannot borrow '{}' as immutable because it is also borrowed as mutable",
                                        place_display
                                    ),
                                    self.filepath.to_string(),
                                    borrow_span.clone(),
                                    Some("immutable borrow occurs here".to_string()),
                                    None,
                                )
                                .with_note(format!(
                                    "mutable borrow of '{}' occurs here at line {}; mutable borrow later used at line {}",
                                    existing_display, existing.loan_span.line, use_span.line
                                )),
                            );
                        }
                    }
                }
            }
        }

        self.loans.push(Loan {
            place: place.clone(),
            is_mut: is_borrow_mut,
            loan_span: borrow_span.clone(),
            ref_var,
        });
    }

    fn check_return_ref(&mut self, expr: &Expr) {
        match expr {
            Expr::Borrow(inner, _, borrow_span) => {
                if let Some(place) = expr_to_place(inner) {
                    let root = place.root_var();
                    if let Some(local) = self.locals.get(root) {
                        if !local.is_param {
                            self.diagnostics.push(
                                Diagnostic::new_error(
                                    format!("cannot return reference to local variable '{}'", root),
                                    self.filepath.to_string(),
                                    borrow_span.clone(),
                                    Some(
                                        "returns a reference to data owned by the current function"
                                            .to_string(),
                                    ),
                                    None,
                                )
                                .with_note(format!(
                                    "local variable '{}' declared here at line {}",
                                    root, local.decl_span.line
                                )),
                            );
                        }
                    }
                }
            }
            Expr::Identifier(name, span) => {
                if let Some(local) = self.locals.get(name) {
                    if let Some(target) = &local.points_to_local {
                        if let Some(target_local) = self.locals.get(target) {
                            if !target_local.is_param {
                                self.diagnostics.push(
                                    Diagnostic::new_error(
                                        format!(
                                            "cannot return reference to local variable '{}'",
                                            target
                                        ),
                                        self.filepath.to_string(),
                                        span.clone(),
                                        Some(
                                            "returns a reference to data owned by the current function"
                                                .to_string(),
                                        ),
                                        None,
                                    )
                                    .with_note(format!(
                                        "local variable '{}' declared here at line {}",
                                        target, target_local.decl_span.line
                                    )),
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn find_active_loan<'b>(
        &'b self,
        place: &Place,
        remaining_stmts: &[Stmt],
    ) -> Option<(&'b Loan, Span)> {
        for loan in &self.loans {
            if loan.place.overlaps(place) {
                if let Some(ref_name) = &loan.ref_var {
                    if let Some(use_span) = find_use_in_stmts(remaining_stmts, ref_name) {
                        return Some((loan, use_span));
                    }
                }
            }
        }
        None
    }

    fn check_reads_in_expr(&mut self, expr: &Expr) {
        let mut reads = Vec::new();
        collect_expr_reads(expr, &mut reads);
        for (name, span) in reads {
            if let Some(local) = self.locals.get(&name) {
                if let Some(m_span) = &local.moved_at {
                    self.diagnostics.push(
                        Diagnostic::new_error(
                            format!("use of moved value: '{}'", name),
                            self.filepath.to_string(),
                            span.clone(),
                            Some("value used here after move".to_string()),
                            None,
                        )
                        .with_note(format!("value moved here at line {}", m_span.line)),
                    );
                } else if let Some(c_span) = &local.conditionally_moved {
                    self.diagnostics.push(
                        Diagnostic::new_error(
                            format!("use of possibly-moved value: '{}'", name),
                            self.filepath.to_string(),
                            span.clone(),
                            Some("value may have been moved in a previous branch".to_string()),
                            None,
                        )
                        .with_note(format!(
                            "value conditionally moved here at line {}",
                            c_span.line
                        )),
                    );
                }
            }
        }
    }

    fn check_assignments_in_expr(&mut self, expr: &Expr, remaining_stmts: &[Stmt]) {
        match expr {
            Expr::Binary(left, op, _, span) if op.is_assignment_or_compound() => {
                if let Some(place) = expr_to_place(left) {
                    if let Some((loan, use_span)) = self.find_active_loan(&place, remaining_stmts) {
                        self.diagnostics.push(
                            Diagnostic::new_error(
                                format!(
                                    "cannot assign to '{}' because it is borrowed",
                                    place.to_display_string()
                                ),
                                self.filepath.to_string(),
                                span.clone(),
                                Some("assignment to borrowed variable occurs here".to_string()),
                                None,
                            )
                            .with_note(format!(
                                "borrow of '{}' occurs at line {}; borrow later used at line {}",
                                loan.place.to_display_string(),
                                loan.loan_span.line,
                                use_span.line
                            )),
                        );
                    } else if place.projections.is_empty() {
                        // Re-initialize a moved variable on direct assignment
                        if let Some(local) = self.locals.get_mut(place.root_var()) {
                            local.moved_at = None;
                            local.conditionally_moved = None;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

trait BinaryOpExt {
    fn is_assignment_or_compound(&self) -> bool;
}

impl BinaryOpExt for BinaryOp {
    fn is_assignment_or_compound(&self) -> bool {
        matches!(
            self,
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
        )
    }
}

fn collect_expr_reads(expr: &Expr, reads: &mut Vec<(String, Span)>) {
    match expr {
        Expr::Identifier(name, span) => {
            reads.push((name.clone(), span.clone()));
        }
        Expr::Borrow(inner, _, _) => {
            collect_expr_reads(inner, reads);
        }
        Expr::Unary(_, inner, _) => {
            collect_expr_reads(inner, reads);
        }
        Expr::Binary(left, op, right, _) => {
            if op.is_assignment_or_compound() {
                if *op != BinaryOp::Assign {
                    collect_expr_reads(left, reads);
                }
                collect_expr_reads(right, reads);
            } else {
                collect_expr_reads(left, reads);
                collect_expr_reads(right, reads);
            }
        }
        Expr::Call(callee, args, _) => {
            collect_expr_reads(callee, reads);
            for (_, arg) in args {
                collect_expr_reads(arg, reads);
            }
        }
        Expr::Dot(inner, _, _) | Expr::SafeDot(inner, _, _) => {
            collect_expr_reads(inner, reads);
        }
        Expr::Index(inner, idx, _) => {
            collect_expr_reads(inner, reads);
            collect_expr_reads(idx, reads);
        }
        Expr::Tuple(items, _) | Expr::VectorLiteral(items, _) => {
            for item in items {
                collect_expr_reads(item, reads);
            }
        }
        Expr::Formula(fields, _) => {
            for (_, val, _, _) in fields {
                collect_expr_reads(val, reads);
            }
        }
        Expr::InterpolatedString(segs, _) => {
            for seg in segs {
                if let InterpolatedSegment::Expr(e) = seg {
                    collect_expr_reads(e, reads);
                }
            }
        }
        Expr::Block(stmts, _) => {
            for s in stmts {
                collect_stmt_reads(s, reads);
            }
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_reads(cond, reads);
            collect_expr_reads(then_branch, reads);
            if let Some(eb) = else_branch {
                collect_expr_reads(eb, reads);
            }
        }
        _ => {}
    }
}

fn collect_stmt_reads(stmt: &Stmt, reads: &mut Vec<(String, Span)>) {
    match stmt {
        Stmt::LetDecl { value, .. } | Stmt::ConstDecl { value, .. } => {
            collect_expr_reads(value, reads);
        }
        Stmt::ExprStmt(expr) => {
            collect_expr_reads(expr, reads);
        }
        Stmt::ReturnStmt(Some(expr), _) => {
            collect_expr_reads(expr, reads);
        }
        _ => {}
    }
}

pub fn find_use_in_stmts(stmts: &[Stmt], var_name: &str) -> Option<Span> {
    for s in stmts {
        if let Some(span) = find_use_in_stmt(s, var_name) {
            return Some(span);
        }
    }
    None
}

pub fn find_use_in_stmt(stmt: &Stmt, var_name: &str) -> Option<Span> {
    match stmt {
        Stmt::LetDecl { value, .. } | Stmt::ConstDecl { value, .. } => {
            find_use_in_expr(value, var_name)
        }
        Stmt::ExprStmt(expr) => find_use_in_expr(expr, var_name),
        Stmt::ReturnStmt(val, _) => val.as_ref().and_then(|e| find_use_in_expr(e, var_name)),
        Stmt::IfStmt {
            cond,
            then_branch,
            else_branch,
            ..
        } => find_use_in_expr(cond, var_name)
            .or_else(|| find_use_in_stmts(then_branch, var_name))
            .or_else(|| {
                else_branch
                    .as_ref()
                    .and_then(|eb| find_use_in_stmts(eb, var_name))
            }),
        Stmt::WhileStmt { cond, body, .. } => {
            find_use_in_expr(cond, var_name).or_else(|| find_use_in_stmts(body, var_name))
        }
        Stmt::ForStmt { iterable, body, .. } => {
            find_use_in_expr(iterable, var_name).or_else(|| find_use_in_stmts(body, var_name))
        }
        Stmt::MatchStmt { target, arms, .. } => find_use_in_expr(target, var_name).or_else(|| {
            arms.iter()
                .find_map(|arm| find_use_in_expr(&arm.body, var_name))
        }),
        _ => None,
    }
}

pub fn find_use_in_expr(expr: &Expr, var_name: &str) -> Option<Span> {
    match expr {
        Expr::Identifier(name, span) if name == var_name => Some(span.clone()),
        Expr::Borrow(inner, _, _) => find_use_in_expr(inner, var_name),
        Expr::Unary(_, inner, _) => find_use_in_expr(inner, var_name),
        Expr::Binary(left, _, right, _) => {
            find_use_in_expr(left, var_name).or_else(|| find_use_in_expr(right, var_name))
        }
        Expr::Call(callee, args, _) => find_use_in_expr(callee, var_name).or_else(|| {
            args.iter()
                .find_map(|(_, arg)| find_use_in_expr(arg, var_name))
        }),
        Expr::Dot(inner, _, _) | Expr::SafeDot(inner, _, _) => find_use_in_expr(inner, var_name),
        Expr::Index(inner, idx, _) => {
            find_use_in_expr(inner, var_name).or_else(|| find_use_in_expr(idx, var_name))
        }
        Expr::Tuple(items, _) | Expr::VectorLiteral(items, _) => items
            .iter()
            .find_map(|item| find_use_in_expr(item, var_name)),
        Expr::Formula(fields, _) => fields
            .iter()
            .find_map(|(_, val, _, _)| find_use_in_expr(val, var_name)),
        Expr::InterpolatedString(segs, _) => segs.iter().find_map(|seg| {
            if let InterpolatedSegment::Expr(e) = seg {
                find_use_in_expr(e, var_name)
            } else {
                None
            }
        }),
        Expr::Block(stmts, _) => find_use_in_stmts(stmts, var_name),
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => find_use_in_expr(cond, var_name)
            .or_else(|| find_use_in_expr(then_branch, var_name))
            .or_else(|| {
                else_branch
                    .as_ref()
                    .and_then(|eb| find_use_in_expr(eb, var_name))
            }),
        _ => None,
    }
}
