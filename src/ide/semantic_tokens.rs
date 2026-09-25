use std::collections::HashSet;

#[derive(serde::Serialize)]
pub struct SemanticToken {
    pub line: usize,
    pub col: usize,
    pub length: usize,
    pub token_type: usize,
    pub token_modifiers: usize,
}

pub fn get_semantic_tokens(source: &str) -> Vec<SemanticToken> {
    get_semantic_tokens_with_types(source, None)
}

pub fn get_semantic_tokens_with_types(
    _source: &str,
    _extra_types: Option<&HashSet<String>>,
) -> Vec<SemanticToken> {
    Vec::new()
}
