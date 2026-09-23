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
    source: &str,
    extra_types: Option<&HashSet<String>>,
) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut lexer = crate::lexer::Lexer::new(source);
    let mut raw_tokens = Vec::new();

    loop {
        let t = lexer.next_token();
        if t.kind == crate::lexer::TokenKind::EOF {
            break;
        }
        raw_tokens.push(t);
    }

    let len = raw_tokens.len();

    // Pass 1: Collect locally declared structs, enums, and functions
    let mut local_types = HashSet::new();
    let mut local_funcs = HashSet::new();

    // Standard library and built-in type names
    for std_t in &[
        "Element", "NodeList", "Storage", "Location", "History", "Document", "Window",
        "Option", "Result", "Vector", "Map", "Array", "String", "Int", "Num", "Float", "Bool", "Byte",
    ] {
        local_types.insert(std_t.to_string());
    }

    for k in 0..len {
        if (raw_tokens[k].kind == crate::lexer::TokenKind::Struct
            || raw_tokens[k].kind == crate::lexer::TokenKind::Enum)
            && k + 1 < len
            && raw_tokens[k + 1].kind == crate::lexer::TokenKind::Identifier
        {
            local_types.insert(raw_tokens[k + 1].lexeme.clone());
        } else if raw_tokens[k].kind == crate::lexer::TokenKind::Fn
            && k + 1 < len
            && raw_tokens[k + 1].kind == crate::lexer::TokenKind::Identifier
        {
            local_funcs.insert(raw_tokens[k + 1].lexeme.clone());
        }
    }

    let mut i = 0;
    let mut jsx_tag_depth: usize = 0;
    let mut in_jsx_tag = false;
    let mut jsx_expr_depth: usize = 0;

    while i < len {
        let t = &raw_tokens[i];
        let mut token_type = None;
        let modifiers = 0;

        // Check JSX tag entry - avoid false positives on comparison operators like `a < b`
        if t.kind == crate::lexer::TokenKind::Lt {
            if i + 1 < len && raw_tokens[i + 1].kind == crate::lexer::TokenKind::Slash {
                in_jsx_tag = true;
            } else if i + 1 < len
                && (raw_tokens[i + 1].kind == crate::lexer::TokenKind::Identifier
                    || raw_tokens[i + 1].kind == crate::lexer::TokenKind::Type)
            {
                let is_jsx_start = if i == 0 {
                    true
                } else {
                    matches!(
                        raw_tokens[i - 1].kind,
                        crate::lexer::TokenKind::Return
                            | crate::lexer::TokenKind::Yield
                            | crate::lexer::TokenKind::Await
                            | crate::lexer::TokenKind::Equal
                            | crate::lexer::TokenKind::OpenParen
                            | crate::lexer::TokenKind::OpenBracket
                            | crate::lexer::TokenKind::OpenBrace
                            | crate::lexer::TokenKind::Comma
                            | crate::lexer::TokenKind::Colon
                            | crate::lexer::TokenKind::Arrow
                            | crate::lexer::TokenKind::FatArrow
                            | crate::lexer::TokenKind::Question
                            | crate::lexer::TokenKind::Ampersand2
                            | crate::lexer::TokenKind::Pipe2
                            | crate::lexer::TokenKind::Newline
                            | crate::lexer::TokenKind::Gt
                    )
                };

                if is_jsx_start {
                    in_jsx_tag = true;
                    jsx_tag_depth += 1;
                }
            }
        }

        if in_jsx_tag {
            if t.kind == crate::lexer::TokenKind::Slash
                && i + 1 < len
                && raw_tokens[i + 1].kind == crate::lexer::TokenKind::Gt
            {
                // Self closing: />
                jsx_tag_depth = jsx_tag_depth.saturating_sub(1);
            } else if t.kind == crate::lexer::TokenKind::Gt {
                in_jsx_tag = false;
                if i >= 2
                    && raw_tokens[i - 2].kind == crate::lexer::TokenKind::Slash
                    && i >= 3
                    && raw_tokens[i - 3].kind == crate::lexer::TokenKind::Lt
                {
                    // </tag> closing tag
                    jsx_tag_depth = jsx_tag_depth.saturating_sub(1);
                }
            }
        }

        // Track dynamic expressions { ... } inside JSX
        if jsx_tag_depth > 0 {
            if t.kind == crate::lexer::TokenKind::OpenBrace {
                jsx_expr_depth += 1;
            } else if t.kind == crate::lexer::TokenKind::CloseBrace {
                jsx_expr_depth = jsx_expr_depth.saturating_sub(1);
            }
        } else {
            jsx_expr_depth = 0;
        }

        let is_in_jsx_text = jsx_tag_depth > 0 && !in_jsx_tag && jsx_expr_depth == 0;

        if !is_in_jsx_text {
            match &t.kind {
                crate::lexer::TokenKind::Comment => {
                    token_type = Some(3); // comment
                }
                crate::lexer::TokenKind::StringLiteral
                | crate::lexer::TokenKind::InterpolatedStringContent
                | crate::lexer::TokenKind::StringEnd => {
                    token_type = Some(4); // string
                }
                crate::lexer::TokenKind::At
                | crate::lexer::TokenKind::Annotation => {
                    token_type = Some(0); // keyword (same as tags)
                }
                crate::lexer::TokenKind::Fn
                | crate::lexer::TokenKind::Let
                | crate::lexer::TokenKind::Const
                | crate::lexer::TokenKind::Struct
                | crate::lexer::TokenKind::Enum
                | crate::lexer::TokenKind::Trait
                | crate::lexer::TokenKind::Impl
                | crate::lexer::TokenKind::Export
                | crate::lexer::TokenKind::Import
                | crate::lexer::TokenKind::Mut
                | crate::lexer::TokenKind::As
                | crate::lexer::TokenKind::Type
                | crate::lexer::TokenKind::Where
                | crate::lexer::TokenKind::Formula
                | crate::lexer::TokenKind::If
                | crate::lexer::TokenKind::Else
                | crate::lexer::TokenKind::Match
                | crate::lexer::TokenKind::For
                | crate::lexer::TokenKind::In
                | crate::lexer::TokenKind::While
                | crate::lexer::TokenKind::Loop
                | crate::lexer::TokenKind::Break
                | crate::lexer::TokenKind::Continue
                | crate::lexer::TokenKind::Defer
                | crate::lexer::TokenKind::Return
                | crate::lexer::TokenKind::Yield
                | crate::lexer::TokenKind::Await
                | crate::lexer::TokenKind::Async
                | crate::lexer::TokenKind::Thread
                | crate::lexer::TokenKind::Ampersand2
                | crate::lexer::TokenKind::Pipe2
                | crate::lexer::TokenKind::Exclamation
                | crate::lexer::TokenKind::True
                | crate::lexer::TokenKind::False
                | crate::lexer::TokenKind::Nil => {
                    token_type = Some(0); // keyword (pink, same as tags)
                }
                crate::lexer::TokenKind::Identifier => {
                    if t.lexeme == "self" || t.lexeme == "Self" {
                        token_type = Some(0); // keyword
                    } else if i > 0 && raw_tokens[i - 1].kind == crate::lexer::TokenKind::At {
                        token_type = Some(0); // annotation identifier colored like tags (keyword)
                    } else if i > 0 && raw_tokens[i - 1].kind == crate::lexer::TokenKind::Lt {
                        // JSX opening tag name colored like keyword (pink)
                        token_type = Some(0); // JSX tag name
                    } else if i > 1
                        && raw_tokens[i - 2].kind == crate::lexer::TokenKind::Lt
                        && raw_tokens[i - 1].kind == crate::lexer::TokenKind::Slash
                    {
                        // JSX closing tag name colored like keyword (pink)
                        token_type = Some(0); // JSX tag name
                    } else if i > 0 && raw_tokens[i - 1].kind == crate::lexer::TokenKind::Fn {
                        token_type = Some(1); // function declaration name (blue)
                    } else if i > 0
                        && matches!(
                            raw_tokens[i - 1].kind,
                            crate::lexer::TokenKind::Struct | crate::lexer::TokenKind::Enum
                        )
                    {
                        token_type = Some(1); // struct / enum declaration name (blue)
                    } else if local_types.contains(&t.lexeme)
                        || extra_types.map_or(false, |et| et.contains(&t.lexeme))
                    {
                        token_type = Some(1); // declared workspace struct / enum (blue)
                    } else if i + 1 < len
                        && raw_tokens[i + 1].kind == crate::lexer::TokenKind::OpenParen
                    {
                        token_type = Some(1); // function call (blue)
                    } else if i + 2 < len
                        && raw_tokens[i + 1].kind == crate::lexer::TokenKind::Exclamation
                        && raw_tokens[i + 2].kind == crate::lexer::TokenKind::OpenParen
                    {
                        token_type = Some(1); // macro call like println!(...) (blue)
                    } else if matches!(
                        t.lexeme.as_str(),
                        "println" | "print" | "eprintln" | "assert" | "panic" | "todo"
                    ) {
                        token_type = Some(1); // built-in function (blue)
                    } else if local_funcs.contains(&t.lexeme) {
                        token_type = Some(1); // local function reference (blue)
                    } else if t.lexeme.starts_with("on") && t.lexeme.len() > 2 {
                        token_type = Some(1); // function color (blue) for event handlers like onClick
                    }
                }
                _ => {}
            }
        }

        if let Some(ty) = token_type {
            tokens.push(SemanticToken {
                line: t.span.line.saturating_sub(1),
                col: t.span.col.saturating_sub(1),
                length: t.lexeme.encode_utf16().count(),
                token_type: ty,
                token_modifiers: modifiers,
            });
        }

        i += 1;
    }

    tokens
}
