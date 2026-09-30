use std::collections::HashSet;

#[derive(serde::Serialize, Clone, Debug)]
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
    source: &str,
    extra_types: Option<&HashSet<String>>,
) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();

    let mut declared_enums = HashSet::new();
    let mut declared_variants = HashSet::new();

    declared_variants.insert("None".to_string());
    declared_variants.insert("Some".to_string());
    declared_variants.insert("Ok".to_string());
    declared_variants.insert("Err".to_string());
    declared_enums.insert("Option".to_string());
    declared_enums.insert("Result".to_string());
    declared_enums.insert("Error".to_string());

    if let Some(types) = extra_types {
        for t in types {
            declared_enums.insert(t.clone());
        }
    }

    if let Ok(enum_decl_re) = regex::Regex::new(r"(?:export\s+)?enum\s+([a-zA-Z_]\w*)\s*\{([^}]*)\}") {
        if let Ok(variant_re) = regex::Regex::new(r"([a-zA-Z_]\w*)(?:\s*\([^)]*\))?") {
            for cap in enum_decl_re.captures_iter(source) {
                let e_name = cap[1].to_string();
                declared_enums.insert(e_name);
                if let Some(body) = cap.get(2) {
                    for v_cap in variant_re.captures_iter(body.as_str()) {
                        declared_variants.insert(v_cap[1].to_string());
                    }
                }
            }
        }
    }

    if let Ok(ident_re) = regex::Regex::new(r"\b[a-zA-Z_]\w*\b") {
        for (line_idx, line) in source.lines().enumerate() {
            let trimmed_start = line.trim_start();
            if trimmed_start.starts_with("//") {
                continue;
            }

            for mat in ident_re.find_iter(line) {
                let word = mat.as_str();
                let col = mat.start();
                let len = word.len();

                if declared_enums.contains(word) {
                    tokens.push(SemanticToken {
                        line: line_idx,
                        col,
                        length: len,
                        token_type: 5, // enum
                        token_modifiers: 0,
                    });
                } else if declared_variants.contains(word) {
                    tokens.push(SemanticToken {
                        line: line_idx,
                        col,
                        length: len,
                        token_type: 6, // enumMember
                        token_modifiers: 0,
                    });
                }
            }
        }
    }

    tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.col.cmp(&b.col)));
    tokens
}
