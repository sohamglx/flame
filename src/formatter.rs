use crate::lexer::{Lexer, TokenKind};

pub fn format_code(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    lexer.keep_comments = true;

    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        if tok.kind == TokenKind::EOF {
            break;
        }
        tokens.push(tok);
    }

    let mut multiline_parens = vec![false; tokens.len()];
    let mut paren_stack = Vec::new();

    for (i, tok) in tokens.iter().enumerate() {
        if tok.kind == TokenKind::OpenParen {
            paren_stack.push(i);
        } else if tok.kind == TokenKind::CloseParen {
            if let Some(start_idx) = paren_stack.pop() {
                let mut has_brace = false;
                for j in (start_idx + 1)..i {
                    if tokens[j].kind == TokenKind::OpenBrace {
                        has_brace = true;
                        break;
                    }
                }
                if has_brace {
                    multiline_parens[start_idx] = true;
                    multiline_parens[i] = true;
                }
            }
        }
    }

    let mut last_import_idx = None;
    for (i, tok) in tokens.iter().enumerate() {
        if tok.kind == TokenKind::Import {
            last_import_idx = Some(i);
        }
    }
    let mut last_import_line_end_idx = None;
    if let Some(idx) = last_import_idx {
        for j in idx..tokens.len() {
            if tokens[j].kind == TokenKind::Newline {
                last_import_line_end_idx = Some(j);
                break;
            }
        }
    }

    let mut is_generic_lt_gt = vec![false; tokens.len()];
    for i in 0..tokens.len() {
        if tokens[i].kind == TokenKind::Lt {
            let mut is_ident = false;
            let mut last_idx = i;
            while last_idx > 0 {
                last_idx -= 1;
                if tokens[last_idx].kind != TokenKind::Newline {
                    if matches!(
                        tokens[last_idx].kind,
                        TokenKind::Identifier | TokenKind::Type
                    ) {
                        is_ident = true;
                    }
                    break;
                }
            }
            if is_ident {
                let mut depth = 1;
                let mut is_valid = true;
                let mut matching_gt = None;
                for j in (i + 1)..tokens.len() {
                    match tokens[j].kind {
                        TokenKind::Lt => depth += 1,
                        TokenKind::Gt => {
                            depth -= 1;
                            if depth == 0 {
                                matching_gt = Some(j);
                                break;
                            }
                        }
                        TokenKind::IntLiteral
                        | TokenKind::FloatLiteral
                        | TokenKind::StringLiteral
                        | TokenKind::MultilineStringLiteral
                        | TokenKind::Plus
                        | TokenKind::Minus
                        | TokenKind::Star
                        | TokenKind::Slash
                        | TokenKind::Percent
                        | TokenKind::Equal
                        | TokenKind::EqualEqual
                        | TokenKind::ExclamationEqual
                        | TokenKind::Le
                        | TokenKind::Ge
                        | TokenKind::PlusEqual
                        | TokenKind::MinusEqual
                        | TokenKind::StarEqual
                        | TokenKind::SlashEqual
                        | TokenKind::Let
                        | TokenKind::Return
                        | TokenKind::If
                        | TokenKind::While
                        | TokenKind::For
                        | TokenKind::Match => {
                            is_valid = false;
                            break;
                        }
                        _ => {}
                    }
                }
                if is_valid {
                    if let Some(gt_idx) = matching_gt {
                        is_generic_lt_gt[i] = true;
                        is_generic_lt_gt[gt_idx] = true;
                    }
                }
            }
        }
    }

    let mut is_jsx_lt = vec![false; tokens.len()];
    let mut is_jsx_gt = vec![false; tokens.len()];
    let mut is_jsx_slash = vec![false; tokens.len()];
    let mut is_jsx_tag_name = vec![false; tokens.len()];
    let mut is_jsx_attr_eq = vec![false; tokens.len()];
    let mut jsx_unindent_before_lt = vec![false; tokens.len()];
    let mut jsx_indent_after_gt = vec![false; tokens.len()];

    let mut inline_braces = vec![false; tokens.len()];
    let mut inside_inline_braces = vec![false; tokens.len()];
    let mut brace_idx_stack = Vec::new();
    for (idx, tok) in tokens.iter().enumerate() {
        if tok.kind == TokenKind::OpenBrace {
            brace_idx_stack.push(idx);
        } else if tok.kind == TokenKind::CloseBrace {
            if let Some(start_idx) = brace_idx_stack.pop() {
                let mut non_newline_tokens = Vec::new();
                let mut has_newline = false;
                let mut has_statement_sep = false;
                for j in (start_idx + 1)..idx {
                    if tokens[j].kind == TokenKind::Newline {
                        has_newline = true;
                    } else {
                        non_newline_tokens.push(j);
                        if matches!(
                            tokens[j].kind,
                            TokenKind::Let
                                | TokenKind::Const
                                | TokenKind::Fn
                                | TokenKind::Return
                                | TokenKind::If
                                | TokenKind::While
                                | TokenKind::For
                        ) {
                            has_statement_sep = true;
                        }
                    }
                }
                let is_simple_expr = non_newline_tokens.is_empty()
                    || (non_newline_tokens.len() == 1 && !has_statement_sep);
                let is_inline = (!has_newline && !has_statement_sep) || is_simple_expr;
                if is_inline {
                    inline_braces[start_idx] = true;
                    inline_braces[idx] = true;
                    for j in start_idx..=idx {
                        inside_inline_braces[j] = true;
                    }
                }
            }
        }
    }

    for idx in 0..tokens.len() {
        if tokens[idx].kind == TokenKind::Lt && !is_generic_lt_gt[idx] {
            if idx + 1 < tokens.len() && tokens[idx + 1].kind == TokenKind::Slash
                && idx + 2 < tokens.len() && matches!(tokens[idx + 2].kind, TokenKind::Identifier | TokenKind::Type)
            {
                is_jsx_lt[idx] = true;
                is_jsx_slash[idx + 1] = true;
                is_jsx_tag_name[idx + 2] = true;
                let mut opened_on_same_line = false;
                let mut prev = idx;
                while prev > 0 {
                    prev -= 1;
                    if tokens[prev].kind == TokenKind::Newline {
                        break;
                    }
                    if is_jsx_lt[prev] {
                        opened_on_same_line = true;
                        break;
                    }
                }
                if !opened_on_same_line {
                    jsx_unindent_before_lt[idx] = true;
                }
                if idx + 3 < tokens.len() && tokens[idx + 3].kind == TokenKind::Gt {
                    is_jsx_gt[idx + 3] = true;
                }
            } else if idx + 1 < tokens.len() && matches!(tokens[idx + 1].kind, TokenKind::Identifier | TokenKind::Type) {
                let mut prev = idx;
                let mut saw_newline = false;
                let mut prev_token_kind = None;
                while prev > 0 {
                    prev -= 1;
                    if tokens[prev].kind == TokenKind::Newline {
                        saw_newline = true;
                    } else {
                        prev_token_kind = Some(tokens[prev].kind.clone());
                        break;
                    }
                }
                let is_jsx_pattern = if idx + 2 < tokens.len() {
                    matches!(tokens[idx + 2].kind, TokenKind::Gt | TokenKind::Slash | TokenKind::Identifier | TokenKind::Type)
                } else {
                    false
                };
                let is_start = saw_newline || is_jsx_pattern || match prev_token_kind {
                    None => true,
                    Some(TokenKind::OpenBrace | TokenKind::OpenParen | TokenKind::Equal | TokenKind::Return | TokenKind::Arrow) => true,
                    Some(_) => false,
                };
                if is_start {
                    is_jsx_lt[idx] = true;
                    is_jsx_tag_name[idx + 1] = true;
                    let mut k = idx + 2;
                    let mut b_depth: usize = 0;
                    while k < tokens.len() {
                        if tokens[k].kind == TokenKind::OpenBrace {
                            b_depth += 1;
                        } else if tokens[k].kind == TokenKind::CloseBrace {
                            b_depth = b_depth.saturating_sub(1);
                        } else if b_depth == 0 {
                            if tokens[k].kind == TokenKind::Gt {
                                is_jsx_gt[k] = true;
                                let mut nxt = k + 1;
                                while nxt < tokens.len() && tokens[nxt].kind == TokenKind::Newline {
                                    nxt += 1;
                                }
                                if nxt > k + 1 {
                                    jsx_indent_after_gt[k] = true;
                                }
                                break;
                            } else if tokens[k].kind == TokenKind::Slash && k + 1 < tokens.len() && tokens[k + 1].kind == TokenKind::Gt {
                                is_jsx_slash[k] = true;
                                is_jsx_gt[k + 1] = true;
                                break;
                            } else if tokens[k].kind == TokenKind::Equal {
                                is_jsx_attr_eq[k] = true;
                            }
                        }
                        k += 1;
                    }
                }
            }
        }
    }

    let mut is_in_jsx_text = vec![false; tokens.len()];
    let mut jsx_tag_depth: usize = 0;
    let mut in_jsx_tag = false;
    let mut jsx_expr_depth: usize = 0;
    let mut jsx_for_depth: usize = 0;

    for (idx, tok) in tokens.iter().enumerate() {
        if is_jsx_lt[idx] {
            in_jsx_tag = true;
            if idx + 1 < tokens.len() && is_jsx_slash[idx + 1] {
                // closing tag: </tag>
            } else {
                // opening tag
                jsx_tag_depth += 1;
            }
        }

        let was_in_tag = in_jsx_tag;
        if in_jsx_tag {
            if is_jsx_slash[idx] && idx + 1 < tokens.len() && is_jsx_gt[idx + 1] {
                // self-closing: />
                jsx_tag_depth = jsx_tag_depth.saturating_sub(1);
            } else if is_jsx_gt[idx] {
                in_jsx_tag = false;
                if idx >= 2 && is_jsx_tag_name[idx - 1] && is_jsx_slash[idx - 2] {
                    // </tag> closing tag
                    jsx_tag_depth = jsx_tag_depth.saturating_sub(1);
                }
            }
        }

        if jsx_tag_depth > 0 {
            if tok.kind == TokenKind::For
                && idx + 2 < tokens.len()
                && tokens[idx + 1].kind == TokenKind::Identifier
                && tokens[idx + 2].kind == TokenKind::In
            {
                jsx_for_depth += 1;
            }

            if tok.kind == TokenKind::OpenBrace {
                jsx_expr_depth += 1;
            } else if tok.kind == TokenKind::CloseBrace {
                jsx_expr_depth = jsx_expr_depth.saturating_sub(1);
                if jsx_for_depth > 0 && jsx_expr_depth == 0 {
                    jsx_for_depth = jsx_for_depth.saturating_sub(1);
                }
            }
        } else {
            jsx_expr_depth = 0;
            jsx_for_depth = 0;
        }

        if jsx_tag_depth > 0 && !was_in_tag && !in_jsx_tag && jsx_expr_depth == 0 && jsx_for_depth == 0 {
            if tok.kind != TokenKind::Newline && !is_jsx_lt[idx] && !is_jsx_gt[idx] && !is_jsx_slash[idx] && !is_jsx_tag_name[idx] {
                is_in_jsx_text[idx] = true;
            }
        }
    }

    let mut out = String::new();
    let mut indent_level: usize = 0;
    let mut needs_indent = true;
    let mut last_tok: Option<crate::lexer::Token> = None;
    let mut last_tok_was_generic = false;
    let mut in_multiline_paren_count: usize = 0;

    let mut grouping_depth: usize = 0;
    let mut brace_stack: Vec<bool> = Vec::new(); // true = object, false = block

    let indent_str = |level: usize| -> String { "    ".repeat(level) };

    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];

        if tok.kind == TokenKind::Newline {
            let run_start = i;
            let mut run_end = i;
            while run_end < tokens.len() && tokens[run_end].kind == TokenKind::Newline {
                run_end += 1;
            }
            let newline_count = run_end - run_start;
            i = run_end;

            if run_end < tokens.len() && is_jsx_gt[run_end] {
                continue;
            }

            if inside_inline_braces[run_start] {
                continue;
            }

            while out.ends_with(' ') || out.ends_with('\t') {
                out.pop();
            }

            if out.is_empty() {
                needs_indent = true;
                continue;
            }

            let next_tok = tokens.get(run_end);
            let next_is_close_brace = next_tok
                .map(|t| t.kind == TokenKind::CloseBrace && !inline_braces[run_end])
                .unwrap_or(false);

            let is_after_import = if let Some(import_line_end) = last_import_line_end_idx {
                run_start <= import_line_end && import_line_end < run_end
            } else {
                false
            };

            if is_after_import {
                while out.ends_with("\n\n") {
                    out.pop();
                }
                if !out.ends_with('\n') {
                    out.push_str("\n\n");
                } else {
                    out.push('\n');
                }
            } else if next_is_close_brace {
                while out.ends_with("\n\n") {
                    out.pop();
                }
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            } else if indent_level == 0
                && matches!(last_tok.as_ref().map(|t| &t.kind), Some(TokenKind::CloseBrace))
            {
                while out.ends_with("\n\n") {
                    out.pop();
                }
                if !out.ends_with('\n') {
                    out.push_str("\n\n");
                } else {
                    out.push('\n');
                }
            } else {
                let last_ended_with_newline = out.ends_with('\n');
                if last_ended_with_newline {
                    if newline_count >= 2 && !out.ends_with("\n\n") {
                        out.push('\n');
                    }
                } else {
                    if newline_count >= 2 {
                        out.push_str("\n\n");
                    } else {
                        out.push('\n');
                    }
                }
            }

            last_tok = Some(tokens[run_end - 1].clone());
            needs_indent = true;
            continue;
        }

        if tok.kind == TokenKind::OpenParen || tok.kind == TokenKind::OpenBracket {
            grouping_depth += 1;
        } else if tok.kind == TokenKind::CloseParen || tok.kind == TokenKind::CloseBracket {
            grouping_depth = grouping_depth.saturating_sub(1);
        } else if tok.kind == TokenKind::OpenBrace {
            let mut j = i;
            let mut last_sig = TokenKind::EOF;
            while j > 0 {
                j -= 1;
                if tokens[j].kind != TokenKind::Newline {
                    last_sig = tokens[j].kind.clone();
                    break;
                }
            }
            let is_object = matches!(
                last_sig,
                TokenKind::Equal
                    | TokenKind::Comma
                    | TokenKind::Colon
                    | TokenKind::Return
                    | TokenKind::OpenParen
                    | TokenKind::OpenBracket
            );
            brace_stack.push(is_object);
            if is_object {
                grouping_depth += 1;
            }
        }

        if tok.kind == TokenKind::CloseBrace {
            if inline_braces[i] {
                // Inline brace: keep on same line
            } else {
                if let Some(is_object) = brace_stack.pop() {
                    if is_object {
                        grouping_depth = grouping_depth.saturating_sub(1);
                    }
                }
                indent_level = indent_level.saturating_sub(1);
                while out.ends_with("\n\n") {
                    out.pop();
                }
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                needs_indent = true;
            }
        } else if tok.kind == TokenKind::CloseParen && multiline_parens[i] {
            indent_level = indent_level.saturating_sub(1);
            in_multiline_paren_count = in_multiline_paren_count.saturating_sub(1);
            if !out.ends_with('\n') {
                out.push('\n');
            }
            needs_indent = true;
        }

        if jsx_unindent_before_lt[i] {
            indent_level = indent_level.saturating_sub(1);
            let prev_indent = indent_str(indent_level + 1);
            if out.ends_with(&prev_indent) {
                out.truncate(out.len() - prev_indent.len());
                out.push_str(&indent_str(indent_level));
            }
        }

        if needs_indent {
            out.push_str(&indent_str(indent_level));
            needs_indent = false;
        }

        if is_in_jsx_text[i] {
            let run_start = i;
            let mut run_end = i;
            while run_end < tokens.len() && is_in_jsx_text[run_end] && tokens[run_end].kind != TokenKind::Newline {
                run_end += 1;
            }

            if run_start > 0 {
                let prev_tok = &tokens[run_start - 1];
                let gap = &source[prev_tok.span.end..tokens[run_start].span.start];
                if gap.chars().any(|c| c == ' ' || c == '\t') && !out.ends_with(' ') && !out.ends_with('\n') {
                    out.push(' ');
                }
            }

            let raw_slice = &source[tokens[run_start].span.start..tokens[run_end - 1].span.end];
            out.push_str(raw_slice);

            last_tok = Some(tokens[run_end - 1].clone());
            last_tok_was_generic = false;
            i = run_end;
            continue;
        }

        let original_text = &source[tok.span.start..tok.span.end];
        let last_kind = last_tok
            .as_ref()
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::EOF);

        // Spacing before
        match tok.kind {
            TokenKind::OpenBrace => {
                let is_jsx_eq_before = i > 0 && is_jsx_attr_eq[i - 1];
                if !is_jsx_eq_before {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
            }
            TokenKind::Equal => {
                if !is_jsx_attr_eq[i] {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
            }
            TokenKind::EqualEqual
            | TokenKind::ExclamationEqual
            | TokenKind::PlusEqual
            | TokenKind::MinusEqual
            | TokenKind::StarEqual
            | TokenKind::SlashEqual
            | TokenKind::PercentEqual
            | TokenKind::AmpersandEqual
            | TokenKind::PipeEqual
            | TokenKind::CaretEqual
            | TokenKind::ShlEqual
            | TokenKind::ShrEqual
            | TokenKind::Star
            | TokenKind::Percent
            | TokenKind::Le
            | TokenKind::Ge
            | TokenKind::Arrow
            | TokenKind::FatArrow
            | TokenKind::Pipe
            | TokenKind::Pipe2
            | TokenKind::Ampersand2 => {
                if !out.ends_with(' ') && !out.ends_with('\n') {
                    out.push(' ');
                }
            }
            TokenKind::Slash => {
                if is_jsx_slash[i] {
                    if last_kind != TokenKind::Lt {
                        if !out.ends_with(' ') && !out.ends_with('\n') {
                            out.push(' ');
                        }
                    }
                } else {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
            }
            TokenKind::Plus | TokenKind::Minus => {
                let mut prev_idx = if i >= 1 { i - 1 } else { 0 };
                while prev_idx > 0 && tokens[prev_idx].kind == TokenKind::Newline {
                    prev_idx -= 1;
                }
                let before_op = &tokens[prev_idx].kind;
                let is_unary = matches!(
                    before_op,
                    TokenKind::Comma
                        | TokenKind::OpenParen
                        | TokenKind::OpenBracket
                        | TokenKind::OpenBrace
                        | TokenKind::Equal
                        | TokenKind::EqualEqual
                        | TokenKind::ExclamationEqual
                        | TokenKind::Plus
                        | TokenKind::Minus
                        | TokenKind::Star
                        | TokenKind::Slash
                        | TokenKind::Percent
                        | TokenKind::Arrow
                        | TokenKind::FatArrow
                        | TokenKind::Lt
                        | TokenKind::Gt
                        | TokenKind::Le
                        | TokenKind::Ge
                        | TokenKind::Return
                );
                if !is_unary || i == 0 {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
            }
            TokenKind::Lt => {
                if is_jsx_lt[i] {
                    if last_kind == TokenKind::Equal {
                        if !out.ends_with(' ') && !out.ends_with('\n') {
                            out.push(' ');
                        }
                    }
                } else if !is_generic_lt_gt[i] {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
            }
            TokenKind::Gt => {
                if !is_jsx_gt[i] && !is_generic_lt_gt[i] {
                    if !out.ends_with(' ') && !out.ends_with('\n') {
                        out.push(' ');
                    }
                }
            }
            _ => {
                let mut needs_space = false;

                if let Some(ltok) = &last_tok {
                    let last_text = &source[ltok.span.start..ltok.span.end];

                    let last_char = last_text.chars().last().unwrap_or(' ');
                    let first_char = original_text.chars().next().unwrap_or(' ');

                    // If both end and start with identifier-like characters (alphanumeric or underscore),
                    // they MUST be separated by a space to prevent lexical merging.
                    if (last_char.is_alphanumeric() || last_char == '_')
                        && (first_char.is_alphanumeric() || first_char == '_')
                    {
                        needs_space = true;
                    }
                }

                let is_last_ident_like = matches!(
                    last_kind,
                    TokenKind::Identifier
                        | TokenKind::Annotation
                        | TokenKind::StringLiteral
                        | TokenKind::MultilineStringLiteral
                        | TokenKind::IntLiteral
                        | TokenKind::FloatLiteral
                        | TokenKind::CloseParen
                        | TokenKind::CloseBracket
                        | TokenKind::CloseBrace
                        | TokenKind::True
                        | TokenKind::False
                );
                let is_current_ident_like = matches!(
                    tok.kind,
                    TokenKind::Identifier
                        | TokenKind::Let
                        | TokenKind::Const
                        | TokenKind::Fn
                        | TokenKind::Struct
                        | TokenKind::Enum
                        | TokenKind::Trait
                        | TokenKind::Impl
                        | TokenKind::Export
                        | TokenKind::Import
                        | TokenKind::Return
                        | TokenKind::Mut
                        | TokenKind::In
                        | TokenKind::As
                        | TokenKind::Match
                        | TokenKind::If
                        | TokenKind::Else
                        | TokenKind::For
                        | TokenKind::While
                        | TokenKind::Loop
                        | TokenKind::Yield
                        | TokenKind::Defer
                        | TokenKind::Async
                        | TokenKind::Await
                        | TokenKind::Thread
                        | TokenKind::Formula
                        | TokenKind::Annotation
                        | TokenKind::Type
                        | TokenKind::Where
                        | TokenKind::True
                        | TokenKind::False
                );

                let mut is_last_keyword = matches!(
                    last_kind,
                    TokenKind::Let
                        | TokenKind::Const
                        | TokenKind::Fn
                        | TokenKind::Struct
                        | TokenKind::Enum
                        | TokenKind::Trait
                        | TokenKind::Impl
                        | TokenKind::Export
                        | TokenKind::Import
                        | TokenKind::Return
                        | TokenKind::Mut
                        | TokenKind::In
                        | TokenKind::As
                        | TokenKind::Match
                        | TokenKind::If
                        | TokenKind::Else
                        | TokenKind::For
                        | TokenKind::While
                        | TokenKind::Loop
                        | TokenKind::Yield
                        | TokenKind::Defer
                        | TokenKind::Async
                        | TokenKind::Await
                        | TokenKind::Thread
                        | TokenKind::Formula
                        | TokenKind::Annotation
                        | TokenKind::Type
                        | TokenKind::Where
                        | TokenKind::Comma
                        | TokenKind::Colon
                        | TokenKind::Equal
                        | TokenKind::EqualEqual
                        | TokenKind::ExclamationEqual
                        | TokenKind::PlusEqual
                        | TokenKind::MinusEqual
                        | TokenKind::StarEqual
                        | TokenKind::SlashEqual
                        | TokenKind::PercentEqual
                        | TokenKind::AmpersandEqual
                        | TokenKind::PipeEqual
                        | TokenKind::CaretEqual
                        | TokenKind::ShlEqual
                        | TokenKind::ShrEqual
                        | TokenKind::Plus
                        | TokenKind::Minus
                        | TokenKind::Star
                        | TokenKind::Slash
                        | TokenKind::Percent
                        | TokenKind::Le
                        | TokenKind::Ge
                        | TokenKind::Arrow
                        | TokenKind::FatArrow
                        | TokenKind::Pipe
                        | TokenKind::Pipe2
                        | TokenKind::Ampersand2
                );

                if last_kind == TokenKind::Lt || last_kind == TokenKind::Gt {
                    let prev_was_jsx_lt = i > 0 && is_jsx_lt[i - 1];
                    let prev_was_jsx_gt = i > 0 && is_jsx_gt[i - 1];
                    if !last_tok_was_generic && !prev_was_jsx_lt && !prev_was_jsx_gt {
                        is_last_keyword = true;
                    }
                }

                if last_kind == TokenKind::Plus || last_kind == TokenKind::Minus {
                    // It might be unary. Let's check the token before the Plus/Minus.
                    let mut prev_idx = if i >= 2 { i - 2 } else { 0 };
                    // We might need to skip spaces/newlines, but our `tokens` array doesn't include spaces, only Newlines.
                    while prev_idx > 0 && tokens[prev_idx].kind == TokenKind::Newline {
                        prev_idx -= 1;
                    }
                    let before_op = &tokens[prev_idx].kind;
                    let is_unary = matches!(
                        before_op,
                        TokenKind::Comma
                            | TokenKind::OpenParen
                            | TokenKind::OpenBracket
                            | TokenKind::OpenBrace
                            | TokenKind::Equal
                            | TokenKind::EqualEqual
                            | TokenKind::ExclamationEqual
                            | TokenKind::Plus
                            | TokenKind::Minus
                            | TokenKind::Star
                            | TokenKind::Slash
                            | TokenKind::Percent
                            | TokenKind::Arrow
                            | TokenKind::FatArrow
                            | TokenKind::Lt
                            | TokenKind::Gt
                            | TokenKind::Le
                            | TokenKind::Ge
                            | TokenKind::Return
                    );
                    if is_unary {
                        is_last_keyword = false;
                    }
                }

                if (is_last_ident_like && is_current_ident_like) || is_last_keyword {
                    needs_space = true;
                }

                if is_jsx_tag_name[i] && (last_kind == TokenKind::Lt || last_kind == TokenKind::Slash) {
                    needs_space = false;
                }
                if is_jsx_slash[i] && last_kind == TokenKind::Lt {
                    needs_space = false;
                }
                if is_jsx_attr_eq[i] {
                    needs_space = false;
                }
                if i > 0 && is_jsx_attr_eq[i - 1] {
                    needs_space = false;
                }
                if tok.kind == TokenKind::OpenBrace && i > 0 && is_jsx_attr_eq[i - 1] {
                    needs_space = false;
                }
                if tok.kind == TokenKind::Gt && is_jsx_gt[i] {
                    needs_space = false;
                }

                if tok.kind != TokenKind::Dot && last_kind != TokenKind::Dot && !out.ends_with('.')
                {
                    if needs_space {
                        // Exception: do not add a space before OpenParen if the last token was a keyword that acts like a function (e.g. type(), yield())
                        let skip_space = tok.kind == TokenKind::OpenParen
                            && matches!(last_kind, TokenKind::Type | TokenKind::Identifier | TokenKind::Yield);
                        if !skip_space && !out.ends_with(' ') && !out.ends_with('\n') {
                            out.push(' ');
                        }
                    }
                }
            }
        }

        out.push_str(original_text);

        last_tok = Some(tok.clone());
        last_tok_was_generic = is_generic_lt_gt[i];

        // Spacing/newlines after
        match tok.kind {
            TokenKind::OpenBrace => {
                if !inline_braces[i] {
                    indent_level += 1;
                    out.push('\n');
                    needs_indent = true;
                }
            }
            TokenKind::Gt => {
                if jsx_indent_after_gt[i] {
                    indent_level += 1;
                }
            }
            TokenKind::OpenParen => {
                if multiline_parens[i] {
                    indent_level += 1;
                    in_multiline_paren_count += 1;
                    out.push('\n');
                    needs_indent = true;
                }
            }
            TokenKind::Comma => {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
                if in_multiline_paren_count > 0 {
                    out.push('\n');
                    needs_indent = true;
                }
            }
            TokenKind::Comment => {
                out.push('\n');
                needs_indent = true;
            }
            _ => {}
        }

        i += 1;
    }

    out = out.trim_end().to_string();
    out.push('\n');
    out
}
