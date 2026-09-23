#![allow(dead_code)]
use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Assign,
    PlusAssign,
    MinusAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    NilCoalesce,
    Range,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    PreInc,
    PreDec,
    PostInc,
    PostDec,
    NonNullAssert,
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_name: String,
    pub default_val: Option<Expr>,
    pub is_ref: bool,
    pub is_mut: bool,
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub name: String,
    pub args: Vec<String>,
    pub span: Span,
    pub name_span: Span,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub patterns: Vec<String>,
    pub pattern_span: Span,
    pub destructure: Vec<String>,
    pub is_tuple_destructure: bool,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(LiteralValue, Span),
    Identifier(String, Span),
    Unary(UnaryOp, Box<Expr>, Span),
    Binary(Box<Expr>, BinaryOp, Box<Expr>, Span),
    Call(Box<Expr>, Vec<(Option<String>, Expr)>, Span),
    Dot(Box<Expr>, String, Span),
    SafeDot(Box<Expr>, String, Span),
    Formula(Vec<(String, Expr, Span, Vec<Annotation>)>, Span),
    Object(Vec<(String, Expr, Vec<Annotation>)>, Span),
    ThreadSpawn(Box<Expr>, Span),
    Closure {
        params: Vec<Param>,
        return_type: Option<String>,
        body: Vec<Stmt>,
        annotations: Vec<Annotation>,
        span: Span,
    },
    Await(Box<Expr>, Span),
    Tuple(Vec<Expr>, Span),
    VectorLiteral(Vec<Expr>, Span),
    InterpolatedString(Vec<InterpolatedSegment>, Span),
    Block(Vec<Stmt>, Span),
    Borrow(Box<Expr>, bool, Span),
    StructInit(Box<Expr>, Vec<(String, Expr)>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    Cast(Box<Expr>, String, Span),
    JsxElement {
        tag: String,
        attributes: Vec<JsxAttribute>,
        children: Vec<JsxChild>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct JsxAttribute {
    pub name: String,
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum JsxChild {
    Text(String, Span),
    Expr(Expr),
    Element(Box<Expr>),
    For {
        var_name: String,
        iterable: Expr,
        body: Vec<JsxChild>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum InterpolatedSegment {
    Text(String),
    Expr(Expr),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(_, s) => s.clone(),
            Expr::Identifier(_, s) => s.clone(),
            Expr::Unary(_, _, s) => s.clone(),
            Expr::Binary(_, _, _, s) => s.clone(),
            Expr::Call(_, _, s) => s.clone(),
            Expr::Dot(_, _, s) => s.clone(),
            Expr::SafeDot(_, _, s) => s.clone(),
            Expr::Formula(_, span) => span.clone(),
            Expr::Object(_, span) => span.clone(),
            Expr::ThreadSpawn(_, span) => span.clone(),
            Expr::Closure { span, .. } => span.clone(),
            Expr::Await(_, s) => s.clone(),
            Expr::Tuple(_, s) => s.clone(),
            Expr::VectorLiteral(_, s) => s.clone(),
            Expr::InterpolatedString(_, s) => s.clone(),
            Expr::Block(_, s) => s.clone(),
            Expr::Borrow(_, _, s) => s.clone(),
            Expr::StructInit(_, _, s) => s.clone(),
            Expr::Index(_, _, s) => s.clone(),
            Expr::Cast(_, _, s) => s.clone(),
            Expr::JsxElement { span, .. } => span.clone(),
        }
    }
}


#[derive(Debug, Clone)]
pub enum EnumVariant {
    Unit(String),
    Tuple(String, Vec<String>),
    Struct(String, Vec<(String, String)>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    LetDecl {
        name: String,
        is_mut: bool,
        type_ann: Option<String>,
        value: Expr,
        annotations: Vec<Annotation>,
        span: Span,
        name_span: Span,
    },
    ConstDecl {
        name: String,
        is_mut: bool,
        type_ann: Option<String>,
        value: Expr,
        annotations: Vec<Annotation>,
        span: Span,
        name_span: Span,
    },
    FuncDecl {
        name: String,
        params: Vec<Param>,
        return_type: Option<String>,
        body: Option<Vec<Stmt>>,
        annotations: Vec<Annotation>,
        span: Span,
        name_span: Span,
    },
    AnnotationDecl {
        name: String,
        params: Vec<Param>,
        return_type: Option<String>,
        body: Vec<Stmt>,
        annotations: Vec<Annotation>,
        span: Span,
        name_span: Span,
    },
    StructDecl {
        name: String,
        fields: Vec<(String, String)>,
        annotations: Vec<Annotation>,
        span: Span,
        name_span: Span,
    },
    EnumDecl {
        name: String,
        variants: Vec<EnumVariant>,
        annotations: Vec<Annotation>,
        span: Span,
        name_span: Span,
    },
    TraitDecl {
        name: String,
        signatures: Vec<String>,
        span: Span,
    },
    ImplDecl {
        target_type: String,
        trait_name: Option<String>,
        methods: Vec<Stmt>,
        annotations: Vec<Annotation>,
        span: Span,
        name_span: Span,
    },
    ImportDecl {
        path: Vec<String>,
        glob: bool,
        alias: Option<String>,
        span: Span,
    },
    ExportDecl(Box<Stmt>, Span),
    ExprStmt(Expr),
    IfStmt {
        cond: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
        span: Span,
    },
    ForStmt {
        var_name: String,
        iterable: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    WhileStmt {
        cond: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    LoopStmt {
        body: Vec<Stmt>,
        span: Span,
    },
    MatchStmt {
        target: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },
    ReturnStmt(Option<Expr>, Span),
    DeferStmt(Box<Stmt>, Span),
    Break(Span),
    Continue(Span),
    PluginDecl {
        name: String,
        span: Span,
    },
    PackageDecl {
        name: String,
        annotations: Vec<Annotation>,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::LetDecl { span, .. } => span.clone(),
            Stmt::ConstDecl { span, .. } => span.clone(),
            Stmt::FuncDecl { span, .. } => span.clone(),
            Stmt::StructDecl { span, .. } => span.clone(),
            Stmt::EnumDecl { span, .. } => span.clone(),
            Stmt::TraitDecl { span, .. } => span.clone(),
            Stmt::ImplDecl { span, .. } => span.clone(),
            Stmt::ImportDecl { span, .. } => span.clone(),
            Stmt::ExportDecl(_, span) => span.clone(),
            Stmt::ExprStmt(e) => e.span(),
            Stmt::IfStmt { span, .. } => span.clone(),
            Stmt::ForStmt { span, .. } => span.clone(),
            Stmt::WhileStmt { span, .. } => span.clone(),
            Stmt::LoopStmt { span, .. } => span.clone(),
            Stmt::MatchStmt { span, .. } => span.clone(),
            Stmt::ReturnStmt(_, span) => span.clone(),
            Stmt::DeferStmt(_, span) => span.clone(),
            Stmt::Break(s) => s.clone(),
            Stmt::Continue(s) => s.clone(),
            Stmt::PluginDecl { span, .. } => span.clone(),
            Stmt::AnnotationDecl { span, .. } => span.clone(),
            Stmt::PackageDecl { span, .. } => span.clone(),
        }
    }
}
