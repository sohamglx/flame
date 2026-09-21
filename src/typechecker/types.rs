use crate::lexer::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Nil,
    Byte,
    Union(Vec<Type>),
    Nullable(Box<Type>),
    Tuple(Vec<Type>),
    Vector(Box<Type>),
    Formula(HashMap<String, Type>, HashMap<String, String>),
    Function(Vec<Type>, Box<Type>),
    Struct(String),
    Enum(String),
    EnumVariant {
        enum_name: String,
        variant_name: String,
        tuple_items: Vec<Type>,
        struct_fields: HashMap<String, Type>,
    },
    Named(String),
    Quantity(HashMap<String, i32>),
    Unit(HashMap<String, i32>),
    Unknown,
    Reference {
        inner: Box<Type>,
        mutable: bool,
    },
}

#[derive(Debug, Clone)]
pub struct VarInfo {
    pub ty: Type,
    pub is_mut: bool,
    pub hover_doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub ty: Type,
    pub is_ref: bool,
    pub is_mut: bool,
    pub has_default: bool,
}

#[derive(Debug, Clone)]
pub struct FunctionSig {
    pub params: Vec<ParamInfo>,
    pub return_type: Type,
    pub hover_doc: Option<String>,
    pub is_static: bool,
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub fields: Vec<(String, Type)>,
    pub hover_doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VariantInfo {
    pub tuple_items: Vec<Type>,
    pub struct_fields: Vec<(String, Type)>,
    pub hover_doc: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub variants: HashMap<String, VariantInfo>,
    pub hover_doc: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandInfo {
    pub name: String,
    pub about: Option<String>,
    pub func_name: String,
    pub params: Vec<ParamInfo>,
    pub hover_doc: String,
    pub span: Span,
}

