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

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::String => write!(f, "String"),
            Type::Bool => write!(f, "Bool"),
            Type::Nil => write!(f, "Nil"),
            Type::Byte => write!(f, "Byte"),
            Type::Named(s) | Type::Struct(s) | Type::Enum(s) => write!(f, "{}", s),
            Type::Vector(inner) => write!(f, "[{}]", inner),
            Type::Nullable(inner) => write!(f, "{}?", inner),
            Type::Reference { inner, mutable: true } => write!(f, "&mut {}", inner),
            Type::Reference { inner, mutable: false } => write!(f, "&{}", inner),
            Type::Tuple(items) => {
                let items_str = items.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "({})", items_str)
            }
            Type::Union(variants) => {
                let s = variants.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" | ");
                write!(f, "{}", s)
            }
            Type::Function(params, ret) => {
                let p_str = params.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ");
                write!(f, "fn({}) -> {}", p_str, ret)
            }
            _ => write!(f, "{:?}", self),
        }
    }
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
    pub generic_params: Vec<crate::parser::GenericParam>,
}

#[derive(Debug, Clone)]
pub struct StructInfo {
    pub fields: Vec<(String, Type)>,
    pub hover_doc: Option<String>,
    pub generic_params: Vec<crate::parser::GenericParam>,
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
    pub generic_params: Vec<crate::parser::GenericParam>,
}

#[derive(Debug, Clone)]
pub struct TraitInfo {
    pub name: String,
    pub generic_params: Vec<crate::parser::GenericParam>,
    pub super_traits: Vec<String>,
    pub methods: HashMap<String, FunctionSig>,
    pub default_methods: HashMap<String, (FunctionSig, Vec<crate::parser::Stmt>)>,
    pub associated_types: Vec<String>,
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

