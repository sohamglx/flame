use super::types::Type;
use crate::diagnostics::Diagnostic;
use crate::lexer::Span;
use crate::parser::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Place {
    Var(String),
    Field(String, String),
    Index(String),
}

impl Place {
    pub fn root_var(&self) -> &str {
        match self {
            Place::Var(v) => v,
            Place::Field(v, _) => v,
            Place::Index(v) => v,
        }
    }
}

pub fn expr_to_place(expr: &Expr) -> Option<Place> {
    match expr {
        Expr::Identifier(name, _) => Some(Place::Var(name.clone())),
        Expr::Dot(inner, member, _) => {
            if let Expr::Identifier(owner, _) = &**inner {
                Some(Place::Field(owner.clone(), member.clone()))
            } else {
                None
            }
        }
        Expr::Index(inner, _, _) => {
            if let Expr::Identifier(owner, _) = &**inner {
                Some(Place::Index(owner.clone()))
            } else {
                None
            }
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
    pub points_to_local: Option<String>,
}

pub fn is_copy_type(ty: &Type) -> bool {
    match ty {
        Type::Int | Type::Float | Type::Bool | Type::Byte | Type::Nil => true,
        Type::Reference { mutable: false, .. } => true,
        _ => false,
    }
}

pub struct BorrowChecker<'a> {
    pub filepath: &'a str,
    pub diagnostics: Vec<Diagnostic>,
    pub locals: HashMap<String, LocalVar>,
    pub loans: Vec<Loan>,
    pub current_func_return_is_ref: bool,
}

impl<'a> BorrowChecker<'a> {
    pub fn new(filepath: &'a str) -> Self {
        Self {
            filepath,
            diagnostics: Vec::new(),
            locals: HashMap::new(),
            loans: Vec::new(),
            current_func_return_is_ref: false,
        }
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
            self.locals.insert(
                p.name.clone(),
                LocalVar {
                    name: p.name.clone(),
                    is_mut: p.is_mut,
                    is_param: true,
                    is_ref: p.is_ref || p.type_name.trim().starts_with('&'),
                    decl_span: Span::dummy(),
                    ty: Type::Unknown,
                    moved_at: None,
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
                value,
                span: _,
                name_span,
                ..
            }
            | Stmt::ConstDecl {
                name,
                is_mut,
                value,
                span: _,
                name_span,
                ..
            } => {
                // 1. Check all reads in value for use-after-move
                self.check_reads_in_expr(value);

                // 2. Check if value is a move of a local variable
                if let Expr::Identifier(src_name, src_span) = value {
                    if let Some(src_local) = self.locals.get(src_name).cloned() {
                        if !src_local.is_param && !src_local.is_ref && !is_copy_type(&src_local.ty)
                        {
                            if let Some((loan, use_span)) =
                                self.find_active_loan(src_name, remaining_stmts)
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
                                        src_name, loan.loan_span.line, use_span.line
                                    )),
                                );
                            } else {
                                self.locals.get_mut(src_name).unwrap().moved_at =
                                    Some(src_span.clone());
                            }
                        }
                    }
                }

                // 3. Check if value is a borrow expression
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
                        self.locals.insert(
                            name.clone(),
                            LocalVar {
                                name: name.clone(),
                                is_mut: *is_mut,
                                is_param: false,
                                is_ref: true,
                                decl_span: name_span.clone(),
                                ty: Type::Reference {
                                    inner: Box::new(Type::Unknown),
                                    mutable: *is_borrow_mut,
                                },
                                moved_at: None,
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
                        ty: Type::Unknown,
                        moved_at: None,
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
                self.check_reads_in_expr(cond);
                self.check_block(then_branch);
                if let Some(eb) = else_branch {
                    self.check_block(eb);
                }
            }

            Stmt::WhileStmt { cond, body, .. } => {
                self.check_reads_in_expr(cond);
                self.check_block(body);
            }

            Stmt::ForStmt { iterable, body, .. } => {
                self.check_reads_in_expr(iterable);
                self.check_block(body);
            }

            Stmt::MatchStmt { target, arms, .. } => {
                self.check_reads_in_expr(target);
                for arm in arms {
                    self.check_reads_in_expr(&arm.body);
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

        for existing in &self.loans {
            if existing.place.root_var() == root {
                if let Some(ref_name) = &existing.ref_var {
                    if let Some(use_span) = find_use_in_stmts(remaining_stmts, ref_name) {
                        if existing.is_mut && is_borrow_mut {
                            self.diagnostics.push(
                                Diagnostic::new_error(
                                    format!(
                                        "cannot borrow '{}' as mutable more than once at a time",
                                        root
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
                                        root
                                    ),
                                    self.filepath.to_string(),
                                    borrow_span.clone(),
                                    Some("mutable borrow occurs here".to_string()),
                                    None,
                                )
                                .with_note(format!(
                                    "immutable borrow occurs here at line {}; immutable borrow later used at line {}",
                                    existing.loan_span.line, use_span.line
                                )),
                            );
                        } else if existing.is_mut && !is_borrow_mut {
                            self.diagnostics.push(
                                Diagnostic::new_error(
                                    format!(
                                        "cannot borrow '{}' as immutable because it is also borrowed as mutable",
                                        root
                                    ),
                                    self.filepath.to_string(),
                                    borrow_span.clone(),
                                    Some("immutable borrow occurs here".to_string()),
                                    None,
                                )
                                .with_note(format!(
                                    "mutable borrow occurs here at line {}; mutable borrow later used at line {}",
                                    existing.loan_span.line, use_span.line
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
                                    format!(
                                        "cannot return reference to local variable '{}'",
                                        root
                                    ),
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
        var_name: &str,
        remaining_stmts: &[Stmt],
    ) -> Option<(&'b Loan, Span)> {
        for loan in &self.loans {
            if loan.place.root_var() == var_name {
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
                }
            }
        }
    }

    fn check_assignments_in_expr(&mut self, expr: &Expr, remaining_stmts: &[Stmt]) {
        match expr {
            Expr::Binary(left, op, _, span) if op.is_assignment_or_compound() => {
                if let Expr::Identifier(name, _) = &**left {
                    if let Some((loan, use_span)) = self.find_active_loan(name, remaining_stmts) {
                        self.diagnostics.push(
                            Diagnostic::new_error(
                                format!(
                                    "cannot assign to '{}' because it is borrowed",
                                    name
                                ),
                                self.filepath.to_string(),
                                span.clone(),
                                Some("assignment to borrowed variable occurs here".to_string()),
                                None,
                            )
                            .with_note(format!(
                                "borrow of '{}' occurs at line {}; borrow later used at line {}",
                                name, loan.loan_span.line, use_span.line
                            )),
                        );
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
                // If it's a compound assign, left is read too; if plain assign, only right is read
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
