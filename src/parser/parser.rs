#![allow(dead_code)]
use crate::diagnostics::Diagnostic;
use crate::lexer::{Span, Token, TokenKind};
use super::ast::*;
use super::utils::*;

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
    filepath: String,
    thread_aliases: std::collections::HashSet<String>,
    web_function_stack: Vec<bool>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, filepath: String) -> Self {
        let mut thread_aliases = std::collections::HashSet::new();
        thread_aliases.insert("thread".to_string());
        Self {
            tokens,
            index: 0,
            filepath,
            thread_aliases,
            web_function_stack: Vec::new(),
        }
    }

    pub fn is_field_name_token(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Identifier
                | TokenKind::Type
                | TokenKind::Yield
                | TokenKind::Formula
                | TokenKind::Async
                | TokenKind::Await
                | TokenKind::Thread
                | TokenKind::Match
                | TokenKind::As
        )
    }

    fn consume_field_name(&mut self, err_msg: &str) -> Result<Token, Diagnostic> {
        if Self::is_field_name_token(&self.peek().kind) {
            Ok(self.advance())
        } else {
            self.consume(TokenKind::Identifier, err_msg)
        }
    }

    pub fn is_web_annotation(name: &str) -> bool {
        let clean = name.rsplit('.').next().unwrap_or(name);
        let lower = clean.to_ascii_lowercase();
        matches!(
            lower.as_str(),
            "component"
                | "page"
                | "web"
                | "layout"
                | "client"
                | "route"
                | "island"
                | "view"
                | "server"
                | "head"
                | "body"
                | "html"
                | "style"
        )
    }

    fn is_inside_web_annotated_function(&self) -> bool {
        self.web_function_stack.last().copied().unwrap_or(false)
    }

    fn is_jsx_tag_lookahead(&self) -> bool {
        if !self.check(TokenKind::Lt) {
            return false;
        }
        if self.index + 1 >= self.tokens.len() {
            return false;
        }
        if self.tokens[self.index + 1].kind == TokenKind::Slash {
            return true;
        }
        if self.tokens[self.index + 1].kind != TokenKind::Identifier {
            return false;
        }
        if self.index + 2 < self.tokens.len() {
            match self.tokens[self.index + 2].kind {
                TokenKind::Gt | TokenKind::Slash => return true,
                TokenKind::Identifier => return true,
                _ => {}
            }
        }
        false
    }

    fn peek(&self) -> Token {
        if self.index < self.tokens.len() {
            self.tokens[self.index].clone()
        } else {
            self.tokens[self.tokens.len() - 1].clone()
        }
    }

    fn advance(&mut self) -> Token {
        let current = self.peek();
        if self.index < self.tokens.len() {
            self.index += 1;
        }
        current
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    fn check_next(&self, kind: TokenKind) -> bool {
        if self.index + 1 < self.tokens.len() {
            self.tokens[self.index + 1].kind == kind
        } else {
            false
        }
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, kind: TokenKind, msg: &str) -> Result<Token, Diagnostic> {
        let token = self.peek();
        if token.kind == kind {
            Ok(self.advance())
        } else {
            let primary_msg = if !msg.is_empty() {
                msg.to_string()
            } else {
                format!("expected {:?}, found '{}'", kind, token.lexeme)
            };
            Err(Diagnostic::new_error(
                primary_msg,
                self.filepath.clone(),
                token.span.clone(),
                Some(msg.to_string()),
                Some(format!(
                    "Insert '{}' here to fix syntax error",
                    token.lexeme
                )),
            ))
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        let mut statements = Vec::new();
        while !self.check(TokenKind::EOF) {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Stmt, Diagnostic> {
        // Check for standalone @plugin directive
        if self.check(TokenKind::At) && self.check_next(TokenKind::Identifier) {
            let next_tok = &self.tokens[self.index + 1];
            if next_tok.lexeme == "plugin" {
                let at_tok = self.consume(TokenKind::At, "")?;
                self.consume(TokenKind::Identifier, "")?; // consume "plugin"
                let name_tok = if self.check(TokenKind::StringLiteral) {
                    self.consume(TokenKind::StringLiteral, "")?
                } else {
                    self.consume(TokenKind::Identifier, "expected plugin name or path")?
                };
                let span = Span {
                    start: at_tok.span.start,
                    end: name_tok.span.end,
                    line: at_tok.span.line,
                    col: at_tok.span.col,
                };
                return Ok(Stmt::PluginDecl {
                    name: name_tok.lexeme.trim_matches('"').to_string(),
                    span,
                });
            }
        }

        // Handle annotations
        let mut annotations = Vec::new();
        while self.check(TokenKind::At) {
            annotations.push(self.parse_annotation()?);
        }

        let token = self.peek();
        match token.kind {
            TokenKind::Async => {
                self.advance(); // consume "async"
                self.parse_func_decl(annotations)
            }
            TokenKind::Package => self.parse_package_decl(annotations),
            TokenKind::Import => self.parse_import_statement(),
            TokenKind::Export => self.parse_export_statement(annotations),
            TokenKind::Let => self.parse_var_decl(TokenKind::Let, annotations),
            TokenKind::Const => self.parse_var_decl(TokenKind::Const, annotations),
            TokenKind::Fn => self.parse_func_decl(annotations),
            TokenKind::Annotation => self.parse_annotation_decl(annotations),
            TokenKind::Struct => self.parse_struct_decl(annotations),
            TokenKind::Enum => self.parse_enum_decl(annotations),
            TokenKind::Impl => self.parse_impl_decl(annotations),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::Loop => self.parse_loop_statement(),
            TokenKind::Match => self.parse_match_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Defer => self.parse_defer_statement(),
            TokenKind::Break => {
                let tok = self.advance();
                Ok(Stmt::Break(tok.span.clone()))
            }
            TokenKind::Continue => {
                let tok = self.advance();
                Ok(Stmt::Continue(tok.span.clone()))
            }
            _ => {
                // Expression statement
                let expr = self.parse_expr()?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn parse_annotation(&mut self) -> Result<Annotation, Diagnostic> {
        let start_tok = self.consume(TokenKind::At, "expected '@' decorator prefix")?;
        let id = self.consume(TokenKind::Identifier, "expected annotation name")?;
        let mut name = id.lexeme.clone();
        let mut end_span = id.span.end;
        while self.match_token(TokenKind::Dot) {
            let next_id = self.consume(TokenKind::Identifier, "expected property name after '.'")?;
            name.push('.');
            name.push_str(&next_id.lexeme);
            end_span = next_id.span.end;
        }
        let mut args = Vec::new();

        if self.match_token(TokenKind::OpenParen) {
            while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
                let mut arg_str = String::new();
                let mut depth = 0;
                while !self.check(TokenKind::EOF) {
                    if depth == 0
                        && (self.check(TokenKind::Comma) || self.check(TokenKind::CloseParen))
                    {
                        break;
                    }
                    let tok = self.peek();
                    match tok.kind {
                        TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::OpenBrace => {
                            depth += 1
                        }
                        TokenKind::CloseParen | TokenKind::CloseBracket | TokenKind::CloseBrace => {
                            if depth > 0 {
                                depth -= 1;
                            } else {
                                break;
                            }
                        }
                        _ => {}
                    }
                    if !arg_str.is_empty()
                        && tok.kind != TokenKind::Comma
                        && !arg_str.ends_with(' ')
                        && !arg_str.ends_with('(')
                        && !arg_str.ends_with('[')
                        && !arg_str.ends_with(':')
                    {
                        arg_str.push(' ');
                    }
                    if tok.kind == TokenKind::StringLiteral {
                        arg_str.push('"');
                        arg_str.push_str(&tok.lexeme);
                        arg_str.push('"');
                    } else {
                        arg_str.push_str(&tok.lexeme);
                    }
                    self.advance();
                }
                let trimmed = arg_str.trim().to_string();
                if !trimmed.is_empty() {
                    args.push(trimmed);
                }
                self.match_token(TokenKind::Comma);
            }
            let close_tok = self.consume(
                TokenKind::CloseParen,
                "expected ')' to close annotation arguments",
            )?;
            end_span = close_tok.span.end;
        }

        Ok(Annotation { 
            name, 
            args,
            span: Span {
                start: start_tok.span.start,
                end: end_span,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
            name_span: id.span.clone(),
        })
    }

    fn parse_import_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Import, "expected 'import' keyword")?;
        let mut path = Vec::new();
        let mut glob = false;

        let first = self.advance(); // consume any token (allows keywords like thread/process)
        path.push(first.lexeme.clone());

        while self.match_token(TokenKind::Dot) {
            if self.match_token(TokenKind::Star) {
                glob = true;
                break;
            }
            let next = self.advance(); // consume any token
            path.push(next.lexeme.clone());
        }

        let mut alias = None;
        if self.match_token(TokenKind::As) {
            let alias_tok = self.consume(TokenKind::Identifier, "expected identifier after 'as'")?;
            alias = Some(alias_tok.lexeme.clone());
        }

        if path.iter().any(|p| p == "thread") {
            if let Some(ref a) = alias {
                self.thread_aliases.insert(a.clone());
            } else if let Some(last) = path.last() {
                self.thread_aliases.insert(last.clone());
            }
        }

        let end_span = self.peek().span.clone();
        Ok(Stmt::ImportDecl {
            path,
            glob,
            alias,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        })
    }

    fn parse_package_decl(&mut self, annotations: Vec<Annotation>) -> Result<Stmt, Diagnostic> {
        let start_tok = self.advance().clone(); // consume "package"
        let name_tok = if self.check(TokenKind::Identifier) {
            self.advance().clone()
        } else if matches!(
            self.peek().kind,
            TokenKind::Thread
                | TokenKind::Type
                | TokenKind::Where
                | TokenKind::Formula
                | TokenKind::Async
                | TokenKind::Await
                | TokenKind::Mut
        ) {
            self.advance().clone()
        } else {
            let tok = self.peek();
            return Err(Diagnostic::new_error(
                format!("expected Identifier, found '{}'", tok.lexeme),
                self.filepath.clone(),
                tok.span.clone(),
                Some("Expected package name.".to_string()),
                None,
            ));
        };
        let end_span = name_tok.span.clone();
        Ok(Stmt::PackageDecl {
            name: name_tok.lexeme.clone(),
            annotations,
            span: Span {
                start: start_tok.span.start,
                end: end_span.end,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        })
    }

    fn parse_export_statement(&mut self, outer_annotations: Vec<Annotation>) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Export, "expected 'export' keyword")?;
        let mut inner = self.parse_statement()?;
        if !outer_annotations.is_empty() {
            match &mut inner {
                Stmt::FuncDecl { annotations, .. }
                | Stmt::LetDecl { annotations, .. }
                | Stmt::ConstDecl { annotations, .. }
                | Stmt::StructDecl { annotations, .. }
                | Stmt::EnumDecl { annotations, .. }
                | Stmt::AnnotationDecl { annotations, .. }
                | Stmt::PackageDecl { annotations, .. }
                | Stmt::ImplDecl { annotations, .. } => {
                    let mut combined = outer_annotations;
                    combined.append(annotations);
                    *annotations = combined;
                }
                _ => {}
            }
        }
        let end_span = inner.span();
        Ok(Stmt::ExportDecl(
            Box::new(inner),
            Span {
                start: start_tok.span.start,
                end: end_span.end,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        ))
    }

    fn parse_single_type(&mut self) -> Result<String, Diagnostic> {
        let mut t = String::new();
        if self.match_token(TokenKind::Ampersand) {
            t.push('&');
            if self.match_token(TokenKind::Mut) {
                t.push_str("mut ");
            }
        }
        if self.match_token(TokenKind::OpenParen) {
            t.push('(');
            let mut sub_types = Vec::new();
            while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
                // Support closure parameter types with optional names: `(msg: Unknown)` or `(client: ServerClient, bytes: Bytes)`
                if self.check(TokenKind::Identifier) && self.check_next(TokenKind::Colon) {
                    let param_name = self.advance().lexeme;
                    self.consume(TokenKind::Colon, "expected ':' after parameter name in function type")?;
                    let param_type = self.parse_type()?;
                    sub_types.push(format!("{}: {}", param_name, param_type));
                } else {
                    sub_types.push(self.parse_type()?);
                }
                self.match_token(TokenKind::Comma);
            }
            self.consume(TokenKind::CloseParen, "expected ')' to close type parameter list")?;
            t.push_str(&sub_types.join(", "));
            t.push(')');
        } else if self.match_token(TokenKind::OpenBracket) {
            t.push('[');
            let inner = self.parse_type()?;
            self.consume(TokenKind::CloseBracket, "expected ']' to close list type")?;
            t.push_str(&inner);
            t.push(']');
        } else {
            let tok = self.advance();
            if tok.lexeme == "Vec" && self.check(TokenKind::Lt) {
                return Err(Diagnostic::new_error(
                    "Vec<T> syntax is removed; use '[T]' for vector array types instead".to_string(),
                    self.filepath.clone(),
                    tok.span.clone(),
                    None,
                    Some("replace with '[T]'".to_string()),
                ));
            }
            t.push_str(&tok.lexeme);
            while self.match_token(TokenKind::Dot) {
                t.push('.');
                let next_tok = self.consume(TokenKind::Identifier, "expected identifier after '.' in type")?;
                t.push_str(&next_tok.lexeme);
            }
        }

        if self.match_token(TokenKind::Lt) {
            t.push('<');
            let mut sub_types = Vec::new();
            while !self.check(TokenKind::Gt) && !self.check(TokenKind::EOF) {
                sub_types.push(self.parse_type()?);
                self.match_token(TokenKind::Comma);
            }
            self.consume(
                TokenKind::Gt,
                "expected '>' to close generic type arguments",
            )?;
            t.push_str(&sub_types.join(", "));
            t.push('>');
        }

        if self.match_token(TokenKind::Question) {
            t.push('?');
        }

        if self.match_token(TokenKind::Arrow) {
            t.push_str(" -> ");
            t.push_str(&self.parse_type()?);
        }
        Ok(t)
    }

    fn parse_type(&mut self) -> Result<String, Diagnostic> {
        let mut t = self.parse_single_type()?;
        while self.match_token(TokenKind::Pipe) {
            t.push_str(" | ");
            t.push_str(&self.parse_single_type()?);
        }
        Ok(t)
    }

    fn parse_var_decl(
        &mut self,
        kind: TokenKind,
        annotations: Vec<Annotation>,
    ) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(kind.clone(), "expected variable keyword")?;

        let mut is_mut = false;
        let mut name_span = start_tok.span.clone();
        let (name, final_name_span) = if self.match_token(TokenKind::OpenParen) {
            let mut items = Vec::new();
            while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
                let id = self.consume(
                    TokenKind::Identifier,
                    "expected variable identifier in destructuring",
                )?;
                let mut item = id.lexeme.clone();
                if self.match_token(TokenKind::Colon) {
                    let index_tok = self.consume(
                        TokenKind::IntLiteral,
                        "expected integer index after ':' in tuple destructuring",
                    )?;
                    item = format!("{}:{}", item, index_tok.lexeme);
                }
                items.push(item);
                if self.match_token(TokenKind::Comma) {
                    if self.check(TokenKind::CloseParen) {
                        return Err(Diagnostic::new_error(
                            "trailing comma in destructuring without identifier".to_string(),
                            self.filepath.clone(),
                            id.span.clone(),
                            Some("expected variable name after ','".to_string()),
                            Some("Remove the comma or add another variable name".to_string()),
                        ));
                    }
                }
            }
            let close_tok = self.consume(TokenKind::CloseParen, "expected ')' to close destructuring")?;
            name_span.end = close_tok.span.end;
            (format!("({})", items.join(", ")), name_span)
        } else if self.match_token(TokenKind::OpenBrace) {
            let mut items = Vec::new();
            while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
                let id = self.consume(
                    TokenKind::Identifier,
                    "expected variable identifier in object destructuring",
                )?;
                items.push(id.lexeme.clone());
                if self.match_token(TokenKind::Comma) {
                    if self.check(TokenKind::CloseBrace) {
                        return Err(Diagnostic::new_error(
                            "trailing comma in object destructuring without identifier".to_string(),
                            self.filepath.clone(),
                            id.span.clone(),
                            Some("expected variable name after ','".to_string()),
                            Some("Remove the comma or add another variable name".to_string()),
                        ));
                    }
                }
            }
            let close_tok = self.consume(TokenKind::CloseBrace, "expected '}' to close object destructuring")?;
            name_span.end = close_tok.span.end;
            (format!("{{{}}}", items.join(", ")), name_span)
        } else {
            if self.match_token(TokenKind::Mut) {
                is_mut = true;
            }
            let name_tok = self.consume(TokenKind::Identifier, "expected variable name")?;
            (name_tok.lexeme.clone(), name_tok.span.clone())
        };

        let mut type_ann = None;
        if self.match_token(TokenKind::Colon) {
            let t = self.parse_type()?;
            type_ann = Some(t);
        }

        self.consume(TokenKind::Equal, "expected '=' before value assignment")?;
        let value = self.parse_expr()?;
        let end_span = value.span();

        let decl_span = Span {
            start: start_tok.span.start,
            end: end_span.end,
            line: start_tok.span.line,
            col: start_tok.span.col,
        };

        match kind {
            TokenKind::Let => Ok(Stmt::LetDecl {
                name,
                is_mut,
                type_ann,
                value,
                annotations,
                span: decl_span,
                name_span: final_name_span,
            }),
            TokenKind::Const => Ok(Stmt::ConstDecl {
                name,
                is_mut: false,
                type_ann,
                value,
                annotations,
                span: decl_span,
                name_span: final_name_span,
            }),
            _ => unreachable!(),
        }
    }

    fn parse_func_decl(&mut self, annotations: Vec<Annotation>) -> Result<Stmt, Diagnostic> {
        let is_web = annotations.iter().any(|a| Self::is_web_annotation(&a.name));
        self.web_function_stack.push(is_web);
        let res = self.parse_func_decl_inner(annotations);
        self.web_function_stack.pop();
        res
    }

    fn parse_func_decl_inner(&mut self, annotations: Vec<Annotation>) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Fn, "expected 'fn' function definition")?;
        let name_tok = if matches!(
            self.peek().kind,
            TokenKind::Identifier
                | TokenKind::Yield
                | TokenKind::Type
                | TokenKind::Formula
                | TokenKind::Async
                | TokenKind::Await
                | TokenKind::Thread
                | TokenKind::Match
                | TokenKind::As
        ) {
            self.advance()
        } else {
            self.consume(TokenKind::Identifier, "expected function name")?
        };
        let name = name_tok.lexeme.clone();

        if self.match_token(TokenKind::Lt) {
            while !self.check(TokenKind::Gt) && !self.check(TokenKind::EOF) {
                self.advance();
            }
            self.consume(TokenKind::Gt, "expected '>' after generic type parameters")?;
        }

        self.consume(TokenKind::OpenParen, "expected '(' for parameters list")?;
        let mut params = Vec::new();
        while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
            let mut is_ref = false;
            let mut is_mut = false;

            if self.match_token(TokenKind::Ampersand) {
                is_ref = true;

                if self.match_token(TokenKind::Mut) {
                    is_mut = true;
                }
            }

            let p_name_tok = if self.check(TokenKind::SelfLower) {
                self.advance()
            } else {
                self.consume(TokenKind::Identifier, "expected parameter name")?
            };

            let p_type;
            if self.match_token(TokenKind::Colon) {
                p_type = self.parse_type()?;
            } else if p_name_tok.kind != TokenKind::SelfLower {
                let is_web_context = self.is_inside_web_annotated_function()
                    || annotations.iter().any(|a| Self::is_web_annotation(&a.name));
                if is_web_context {
                    p_type = "Any".to_string();
                } else {
                    return Err(Diagnostic::new_error(
                        "expected ':' after parameter name".to_string(),
                        self.filepath.clone(),
                        p_name_tok.span.clone(),
                        Some("Add a type annotation for this parameter".to_string()),
                        Some("Use ': Type' after the parameter name".to_string()),
                    ));
                }
            } else {
                p_type = "Self".to_string();
            }

            let mut default_val = None;
            if self.match_token(TokenKind::Equal) {
                default_val = Some(self.parse_expr()?);
            }

            params.push(Param {
                name: p_name_tok.lexeme.clone(),
                type_name: p_type,
                default_val,
                is_ref,
                is_mut,
            });

            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        self.consume(
            TokenKind::CloseParen,
            "expected ')' to close parameters list",
        )?;

        let mut return_type = None;
        if self.match_token(TokenKind::Arrow) {
            let ret = self.parse_type()?;
            return_type = Some(ret);
        }

        let body = self.parse_block()?;
        let end_span = self.peek().span.clone();

        Ok(Stmt::FuncDecl {
            name,
            params,
            return_type,
            body: Some(body),
            annotations,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
            name_span: name_tok.span,
        })
    }

    fn parse_annotation_decl(&mut self, annotations: Vec<Annotation>) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Annotation, "expected 'annotation' keyword")?;
        let name_tok = self.consume(TokenKind::Identifier, "expected annotation function name")?;
        let name = name_tok.lexeme.clone();

        if self.match_token(TokenKind::Lt) {
            while !self.check(TokenKind::Gt) && !self.check(TokenKind::EOF) {
                self.advance();
            }
            self.consume(TokenKind::Gt, "expected '>' after generic type parameters")?;
        }

        let mut params = Vec::new();
        if self.match_token(TokenKind::OpenParen) {
            while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
                let mut is_ref = false;
                let mut is_mut = false;

                if self.match_token(TokenKind::Ampersand) {
                    is_ref = true;
                    if self.match_token(TokenKind::Mut) {
                        is_mut = true;
                    }
                }

                let p_name_tok = self.consume(TokenKind::Identifier, "expected parameter name")?;
                let p_type;
                if self.match_token(TokenKind::Colon) {
                    p_type = self.parse_type()?;
                } else {
                    return Err(Diagnostic::new_error(
                        "expected ':' after parameter name".to_string(),
                        self.filepath.clone(),
                        p_name_tok.span.clone(),
                        Some("Add a type annotation for this parameter".to_string()),
                        Some("Use ': Type' after the parameter name".to_string()),
                    ));
                }

                let mut default_val = None;
                if self.match_token(TokenKind::Equal) {
                    default_val = Some(self.parse_expr()?);
                }

                params.push(Param {
                    name: p_name_tok.lexeme.clone(),
                    type_name: p_type,
                    default_val,
                    is_ref,
                    is_mut,
                });

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            self.consume(
                TokenKind::CloseParen,
                "expected ')' to close parameters list",
            )?;
        }

        let mut return_type = None;
        if self.match_token(TokenKind::Arrow) {
            let ret = self.parse_type()?;
            return_type = Some(ret);
        }

        let body = self.parse_block()?;
        let end_span = self.peek().span.clone();

        Ok(Stmt::AnnotationDecl {
            name,
            params,
            return_type,
            body,
            annotations,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
            name_span: name_tok.span,
        })
    }

    fn parse_struct_decl(&mut self, annotations: Vec<Annotation>) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Struct, "expected 'struct'")?;
        let name_tok = self.consume(TokenKind::Identifier, "expected struct name")?;
        let name = name_tok.lexeme.clone();

        self.consume(TokenKind::OpenBrace, "expected '{'")?;
        let mut fields = Vec::new();
        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
            let field_name = self.consume_field_name("expected field name")?;
            self.consume(TokenKind::Colon, "expected ':'")?;
            let field_type = self.parse_type()?;
            fields.push((field_name.lexeme.clone(), field_type));
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::CloseBrace, "expected '}'")?;
        let end_span = self.peek().span.clone();

        Ok(Stmt::StructDecl {
            name,
            fields,
            annotations,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
            name_span: name_tok.span,
        })
    }

    fn parse_enum_decl(&mut self, annotations: Vec<Annotation>) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Enum, "expected 'enum'")?;
        let name_tok = self.consume(TokenKind::Identifier, "expected enum name")?;
        let name = name_tok.lexeme.clone();

        self.consume(TokenKind::OpenBrace, "expected '{'")?;
        let mut variants = Vec::new();
        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
            let var_tok = self.consume(TokenKind::Identifier, "expected variant name")?;
            let var_name = var_tok.lexeme.clone();

            if self.match_token(TokenKind::OpenParen) {
                let mut tuple_types = Vec::new();
                while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
                    tuple_types.push(self.parse_type()?);
                    self.match_token(TokenKind::Comma);
                }
                self.consume(TokenKind::CloseParen, "expected ')'")?;
                variants.push(EnumVariant::Tuple(var_name, tuple_types));
            } else if self.match_token(TokenKind::OpenBrace) {
                let mut struct_fields = Vec::new();
                while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
                    let field_tok = self.consume(
                        TokenKind::Identifier,
                        "expected field name in struct variant",
                    )?;
                    self.consume(TokenKind::Colon, "expected ':' after field name")?;
                    let field_type = self.parse_type()?;
                    struct_fields.push((field_tok.lexeme.clone(), field_type));
                    self.match_token(TokenKind::Comma);
                }
                self.consume(TokenKind::CloseBrace, "expected '}'")?;
                variants.push(EnumVariant::Struct(var_name, struct_fields));
            } else {
                variants.push(EnumVariant::Unit(var_name));
            }

            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::CloseBrace, "expected '}'")?;
        let end_span = self.peek().span.clone();

        Ok(Stmt::EnumDecl {
            name,
            variants,
            annotations,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
            name_span: name_tok.span,
        })
    }

    fn parse_impl_decl(&mut self, annotations: Vec<Annotation>) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Impl, "expected 'impl'")?;
        let name_tok =
            self.consume(TokenKind::Identifier, "expected implementation target type")?;
        let mut trait_name = None;
        let mut target_type = name_tok.lexeme.clone();
        let mut target_span = name_tok.span.clone();

        if self.match_token(TokenKind::For) {
            trait_name = Some(target_type);
            let target_tok = self.consume(TokenKind::Identifier, "expected target struct name")?;
            target_type = target_tok.lexeme.clone();
            target_span = target_tok.span.clone();
        }

        self.consume(TokenKind::OpenBrace, "expected '{'")?;
        let mut methods = Vec::new();
        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
            methods.push(self.parse_statement()?);
        }
        self.consume(TokenKind::CloseBrace, "expected '}'")?;
        let end_span = self.peek().span.clone();

        Ok(Stmt::ImplDecl {
            trait_name,
            target_type,
            methods,
            annotations,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
            name_span: target_span,
        })
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::If, "expected 'if'")?;
        let cond = self.parse_expr()?;
        let then_branch = self.parse_block()?;
        let mut else_branch = None;

        if self.match_token(TokenKind::Else) {
            if self.check(TokenKind::If) {
                else_branch = Some(vec![self.parse_if_statement()?]);
            } else {
                else_branch = Some(self.parse_block()?);
            }
        }

        let end_span = self.peek().span.clone();
        Ok(Stmt::IfStmt {
            cond,
            then_branch,
            else_branch,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        })
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::For, "expected 'for'")?;
        let var_tok = self.consume(TokenKind::Identifier, "expected loop variable identifier")?;
        let var_name = var_tok.lexeme.clone();
        self.consume(TokenKind::In, "expected 'in' loop generator")?;
        let iterable = self.parse_expr()?;
        let body = self.parse_block()?;
        let end_span = self.peek().span.clone();

        Ok(Stmt::ForStmt {
            var_name,
            iterable,
            body,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        })
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::While, "expected 'while'")?;
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        let end_span = self.peek().span.clone();

        Ok(Stmt::WhileStmt {
            cond,
            body,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        })
    }

    fn parse_loop_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Loop, "expected 'loop'")?;
        let body = self.parse_block()?;
        let end_span = self.peek().span.clone();

        Ok(Stmt::LoopStmt {
            body,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        })
    }

    fn parse_match_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Match, "expected 'match'")?;
        let target = self.parse_expr()?;
        self.consume(TokenKind::OpenBrace, "expected '{'")?;
        let mut arms = Vec::new();
        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
            let mut patterns = Vec::new();
            let mut pattern_span = self.peek().span.clone();

            loop {
                let pat_tok = self.peek();
                let mut pat = pat_tok.lexeme.clone();
                self.advance();

                // Handle enum paths like `Result.Ok`
                while self.match_token(TokenKind::Dot) {
                    let next_tok = self.consume(TokenKind::Identifier, "expected identifier in pattern path")?;
                    pat = format!("{}.{}", pat, next_tok.lexeme);
                    pattern_span.end = next_tok.span.end;
                }
                
                if pat_tok.kind == TokenKind::Identifier && pat == "_" {
                    pat = "_".to_string();
                }
                
                patterns.push(pat);
                
                if self.match_token(TokenKind::Pipe) || self.match_token(TokenKind::Pipe2) {
                    continue;
                }
                
                let peek_tok = self.peek();
                if peek_tok.kind == TokenKind::Identifier && (peek_tok.lexeme == "or" || peek_tok.lexeme == "and") {
                    self.advance();
                    continue;
                }
                break;
            }

            let mut destructure = Vec::new();
            let mut is_tuple_destructure = false;
            if self.match_token(TokenKind::OpenBrace) {
                while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
                    let field = self.consume_field_name("expected identifier in pattern destructuring")?;
                    destructure.push(field.lexeme.clone());
                    self.match_token(TokenKind::Comma);
                }
                self.consume(TokenKind::CloseBrace, "expected '}' closing pattern destructuring")?;
            } else if self.match_token(TokenKind::OpenParen) {
                is_tuple_destructure = true;
                while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
                    let field = self.consume_field_name("expected identifier in pattern destructuring")?;
                    destructure.push(field.lexeme.clone());
                    self.match_token(TokenKind::Comma);
                }
                self.consume(TokenKind::CloseParen, "expected ')' closing pattern destructuring")?;
            }
            
            let mut guard = None;
            if self.match_token(TokenKind::Ampersand2) || self.match_token(TokenKind::If) {
                guard = Some(self.parse_expr()?);
            }
            
            if !self.match_token(TokenKind::FatArrow) && !self.match_token(TokenKind::Arrow) {
                return Err(self.consume(TokenKind::FatArrow, "expected '=>' pattern arm arrow").unwrap_err());
            }

            let body = if self.check(TokenKind::OpenBrace) {
                let start_span = self.peek().span.clone();
                let stmts = self.parse_block()?;
                let end_span = self.tokens[self.index - 1].span.clone();
                Expr::Block(stmts, Span {
                    start: start_span.start,
                    end: end_span.end,
                    line: start_span.line,
                    col: start_span.col,
                })
            } else {
                self.parse_expr()?
            };

            arms.push(MatchArm {
                patterns,
                pattern_span,
                destructure,
                is_tuple_destructure,
                guard,
                body,
            });
            self.match_token(TokenKind::Comma);
        }
        self.consume(TokenKind::CloseBrace, "expected '}'")?;
        let end_span = self.peek().span.clone();

        Ok(Stmt::MatchStmt {
            target,
            arms,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        })
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Return, "expected 'return'")?;
        let mut val = None;
        if !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
            val = Some(self.parse_expr()?);
        }
        let end_span = self.peek().span.clone();

        Ok(Stmt::ReturnStmt(
            val,
            Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        ))
    }

    fn parse_defer_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start_tok = self.consume(TokenKind::Defer, "expected 'defer'")?;
        let inner = self.parse_statement()?;
        let end_span = inner.span();
        Ok(Stmt::DeferStmt(
            Box::new(inner),
            Span {
                start: start_tok.span.start,
                end: end_span.end,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        ))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.consume(TokenKind::OpenBrace, "expected '{' for code block")?;
        let mut statements = Vec::new();
        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
            statements.push(self.parse_statement()?);
        }
        self.consume(TokenKind::CloseBrace, "expected '}' to close code block")?;
        Ok(statements)
    }

    pub fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_nil_coalesce()?;
        let op = if self.match_token(TokenKind::Equal) {
            Some(BinaryOp::Assign)
        } else if self.match_token(TokenKind::PlusEqual) {
            Some(BinaryOp::PlusAssign)
        } else if self.match_token(TokenKind::MinusEqual) {
            Some(BinaryOp::MinusAssign)
        } else if self.match_token(TokenKind::StarEqual) {
            Some(BinaryOp::MulAssign)
        } else if self.match_token(TokenKind::SlashEqual) {
            Some(BinaryOp::DivAssign)
        } else if self.match_token(TokenKind::PercentEqual) {
            Some(BinaryOp::ModAssign)
        } else if self.match_token(TokenKind::AmpersandEqual) {
            Some(BinaryOp::BitAndAssign)
        } else if self.match_token(TokenKind::PipeEqual) {
            Some(BinaryOp::BitOrAssign)
        } else if self.match_token(TokenKind::CaretEqual) {
            Some(BinaryOp::BitXorAssign)
        } else if self.match_token(TokenKind::ShlEqual) {
            Some(BinaryOp::ShlAssign)
        } else if self.match_token(TokenKind::ShrEqual) {
            Some(BinaryOp::ShrAssign)
        } else {
            None
        };

        if let Some(binary_op) = op {
            let value = self.parse_assignment()?;
            let span = Span {
                start: expr.span().start,
                end: value.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), binary_op, Box::new(value), span);
        }
        Ok(expr)
    }

    fn parse_nil_coalesce(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_or()?;
        while self.match_token(TokenKind::QuestionColon) {
            let right = self.parse_or()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), BinaryOp::NilCoalesce, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_and()?;
        while self.match_token(TokenKind::Pipe2) {
            let right = self.parse_and()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), BinaryOp::Or, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_bitor()?;
        while self.match_token(TokenKind::Ampersand2) {
            let right = self.parse_bitor()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), BinaryOp::And, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_bitor(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_bitxor()?;
        while self.match_token(TokenKind::Pipe) {
            let right = self.parse_bitxor()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), BinaryOp::BitOr, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_bitxor(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_bitand()
    }

    fn parse_bitand(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_equality()?;
        while self.match_token(TokenKind::Ampersand) {
            let right = self.parse_equality()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), BinaryOp::BitAnd, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = if self.match_token(TokenKind::EqualEqual) {
                Some(BinaryOp::Eq)
            } else if self.match_token(TokenKind::ExclamationEqual) {
                Some(BinaryOp::Ne)
            } else {
                None
            };
            if let Some(binary_op) = op {
                let right = self.parse_comparison()?;
                let span = Span {
                    start: expr.span().start,
                    end: right.span().end,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::Binary(Box::new(expr), binary_op, Box::new(right), span);
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_shift()?;
        if matches!(expr, Expr::JsxElement { .. }) {
            return Ok(expr);
        }
        while self.check(TokenKind::Lt)
            || self.check(TokenKind::Le)
            || self.check(TokenKind::Gt)
            || self.check(TokenKind::Ge)
        {
            if self.check(TokenKind::Lt) && self.is_inside_web_annotated_function() {
                if self.peek().span.line > expr.span().line || self.is_jsx_tag_lookahead() {
                    break;
                }
            }
            let tok = self.advance();
            let op = match tok.kind {
                TokenKind::Le => BinaryOp::Le,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::Ge => BinaryOp::Ge,
                _ => BinaryOp::Lt,
            };
            let right = self.parse_shift()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), op, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_shift(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_range()?;
        while self.check(TokenKind::LtLt) || self.check(TokenKind::GtGt) {
            let tok = self.advance();
            let op = match tok.kind {
                TokenKind::LtLt => BinaryOp::Shl,
                _ => BinaryOp::Shr,
            };
            let right = self.parse_range()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), op, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_range(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_term()?;
        if self.match_token(TokenKind::DoubleDot) || self.match_token(TokenKind::DoubleDotEqual) {
            let right = self.parse_term()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), BinaryOp::Range, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_factor()?;
        while self.check(TokenKind::Plus) || self.check(TokenKind::Minus) {
            let tok = self.advance();
            let op = match tok.kind {
                TokenKind::Plus => BinaryOp::Add,
                _ => BinaryOp::Sub,
            };
            let right = self.parse_factor()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), op, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_power()?;
        while self.check(TokenKind::Star)
            || self.check(TokenKind::Slash)
            || self.check(TokenKind::Percent)
        {
            let tok = self.advance();
            let op = match tok.kind {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => BinaryOp::Mod,
            };
            let right = self.parse_power()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            expr = Expr::Binary(Box::new(expr), op, Box::new(right), span);
        }
        Ok(expr)
    }

    fn parse_power(&mut self) -> Result<Expr, Diagnostic> {
        let expr = self.parse_unary()?;
        if self.match_token(TokenKind::Caret) {
            let right = self.parse_power()?;
            let span = Span {
                start: expr.span().start,
                end: right.span().end,
                line: expr.span().line,
                col: expr.span().col,
            };
            Ok(Expr::Binary(Box::new(expr), BinaryOp::BitXor, Box::new(right), span))
        } else {
            Ok(expr)
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.check(TokenKind::Ampersand) {
            let tok = self.advance();
            let mut is_mut = false;
            if self.match_token(TokenKind::Mut) {
                is_mut = true;
            }

            let expr = self.parse_unary()?;
            let span = Span {
                start: tok.span.start,
                end: expr.span().end,
                line: tok.span.line,
                col: tok.span.col,
            };
            return Ok(Expr::Borrow(Box::new(expr), is_mut, span));
        }
        if self.check(TokenKind::Mut) {
            let tok = self.advance();
            let expr = self.parse_unary()?;
            let span = Span {
                start: tok.span.start,
                end: expr.span().end,
                line: tok.span.line,
                col: tok.span.col,
            };
            return Ok(Expr::Borrow(Box::new(expr), true, span));
        }
        if self.check(TokenKind::PlusPlus) {
            let tok = self.advance();
            let expr = self.parse_unary()?;
            let span = Span {
                start: tok.span.start,
                end: expr.span().end,
                line: tok.span.line,
                col: tok.span.col,
            };
            return Ok(Expr::Unary(UnaryOp::PreInc, Box::new(expr), span));
        }
        if self.check(TokenKind::MinusMinus) {
            let tok = self.advance();
            let expr = self.parse_unary()?;
            let span = Span {
                start: tok.span.start,
                end: expr.span().end,
                line: tok.span.line,
                col: tok.span.col,
            };
            return Ok(Expr::Unary(UnaryOp::PreDec, Box::new(expr), span));
        }
        if self.check(TokenKind::Minus) {
            let tok = self.advance();
            let expr = self.parse_unary()?;
            let span = Span {
                start: tok.span.start,
                end: expr.span().end,
                line: tok.span.line,
                col: tok.span.col,
            };
            return Ok(Expr::Unary(UnaryOp::Neg, Box::new(expr), span));
        }
        if self.check(TokenKind::Await) {
            let tok = self.advance();
            let expr = self.parse_unary()?;
            let span = Span {
                start: tok.span.start,
                end: expr.span().end,
                line: tok.span.line,
                col: tok.span.col,
            };
            return Ok(Expr::Await(Box::new(expr), span));
        }
        if self.check(TokenKind::Exclamation) {
            let tok = self.advance();
            let expr = self.parse_unary()?;
            let span = Span {
                start: tok.span.start,
                end: expr.span().end,
                line: tok.span.line,
                col: tok.span.col,
            };
            return Ok(Expr::Unary(UnaryOp::Not, Box::new(expr), span));
        }
        self.parse_primary()
    }

    fn parse_jsx_element(&mut self) -> Result<Expr, Diagnostic> {
        let lt_tok = self.consume(TokenKind::Lt, "expected '<'")?;
        if !self.is_inside_web_annotated_function() {
            return Err(Diagnostic::new_error(
                "JSX syntax is only permitted within functions annotated with a web annotation (e.g. @Component, @Page, @Web)".to_string(),
                self.filepath.clone(),
                lt_tok.span.clone(),
                Some("Annotate the enclosing function with @Component, @Page, or @Web to use JSX".to_string()),
                Some("Add @Component or @Page before 'fn'".to_string()),
            ));
        }
        let tag_tok = self.consume(TokenKind::Identifier, "expected tag name after '<'")?;
        let tag = tag_tok.lexeme.clone();

        let mut attributes = Vec::new();
        while !self.check(TokenKind::Gt)
            && !self.check(TokenKind::Slash)
            && !self.check(TokenKind::EOF)
        {
            let is_attr_start = matches!(
                self.peek().kind,
                TokenKind::Identifier
                    | TokenKind::Type
                    | TokenKind::For
                    | TokenKind::In
                    | TokenKind::As
                    | TokenKind::Match
                    | TokenKind::If
                    | TokenKind::Else
                    | TokenKind::While
                    | TokenKind::Loop
                    | TokenKind::Return
                    | TokenKind::Let
                    | TokenKind::Const
                    | TokenKind::Fn
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::Nil
                    | TokenKind::Import
                    | TokenKind::Export
                    | TokenKind::Struct
                    | TokenKind::Enum
                    | TokenKind::Trait
                    | TokenKind::Impl
                    | TokenKind::Mut
                    | TokenKind::Async
                    | TokenKind::Await
                    | TokenKind::Yield
            );
            if is_attr_start {
                let attr_tok = self.advance();
                let mut attr_name = attr_tok.lexeme.clone();
                let attr_start = attr_tok.span.start;
                let mut attr_end = attr_tok.span.end;

                while self.match_token(TokenKind::Minus) {
                    let is_sub = matches!(
                        self.peek().kind,
                        TokenKind::Identifier
                            | TokenKind::Type
                            | TokenKind::For
                            | TokenKind::In
                            | TokenKind::As
                            | TokenKind::Match
                            | TokenKind::If
                            | TokenKind::Else
                            | TokenKind::While
                            | TokenKind::Loop
                            | TokenKind::Return
                            | TokenKind::Let
                            | TokenKind::Const
                            | TokenKind::Fn
                            | TokenKind::True
                            | TokenKind::False
                            | TokenKind::Nil
                            | TokenKind::Import
                            | TokenKind::Export
                            | TokenKind::Struct
                            | TokenKind::Enum
                            | TokenKind::Trait
                            | TokenKind::Impl
                            | TokenKind::Mut
                            | TokenKind::Async
                            | TokenKind::Await
                            | TokenKind::Yield
                    );
                    if is_sub {
                        let sub_tok = self.advance();
                        attr_name.push('-');
                        attr_name.push_str(&sub_tok.lexeme);
                        attr_end = sub_tok.span.end;
                    }
                }

                let value = if self.match_token(TokenKind::Equal) || self.check(TokenKind::OpenBrace) {
                    if self.match_token(TokenKind::OpenBrace) {
                        let mut stmts = Vec::new();
                        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
                            stmts.push(self.parse_statement()?);
                        }
                        let close_tok = self.consume(
                            TokenKind::CloseBrace,
                            "expected '}' after JSX attribute expression",
                        )?;
                        attr_end = close_tok.span.end;
                        if stmts.len() == 1 {
                            if let Stmt::ExprStmt(e) = stmts.remove(0) {
                                Some(e)
                            } else {
                                Some(Expr::Block(stmts, Span {
                                    start: attr_start,
                                    end: close_tok.span.end,
                                    line: attr_tok.span.line,
                                    col: attr_tok.span.col,
                                }))
                            }
                        } else {
                            Some(Expr::Block(stmts, Span {
                                start: attr_start,
                                end: close_tok.span.end,
                                line: attr_tok.span.line,
                                col: attr_tok.span.col,
                            }))
                        }
                    } else if self.check(TokenKind::StringLiteral)
                        || self.check(TokenKind::MultilineStringLiteral)
                    {
                        let str_tok = self.advance();
                        attr_end = str_tok.span.end;
                        Some(Expr::Literal(
                            LiteralValue::String(str_tok.lexeme),
                            str_tok.span.clone(),
                        ))
                    } else {
                        let val_expr = self.parse_primary()?;
                        attr_end = val_expr.span().end;
                        Some(val_expr)
                    }
                } else {
                    None
                };

                attributes.push(JsxAttribute {
                    name: attr_name,
                    value,
                    span: Span {
                        start: attr_start,
                        end: attr_end,
                        line: attr_tok.span.line,
                        col: attr_tok.span.col,
                    },
                });
            } else {
                break;
            }
        }

        if self.match_token(TokenKind::Slash) {
            let gt_tok = self.consume(TokenKind::Gt, "expected '>' after '/' in self-closing JSX tag")?;
            return Ok(Expr::JsxElement {
                tag,
                attributes,
                children: Vec::new(),
                span: Span {
                    start: lt_tok.span.start,
                    end: gt_tok.span.end,
                    line: lt_tok.span.line,
                    col: lt_tok.span.col,
                },
            });
        }

        let _gt_tok = self.consume(TokenKind::Gt, "expected '>' to close JSX tag header")?;
        let mut children = Vec::new();
        let mut text_buf = String::new();
        let mut text_start = 0;
        let mut text_line = 0;
        let mut text_col = 0;
        let mut prev_token_end = 0;

        let flush_text = |children: &mut Vec<JsxChild>,
                          text_buf: &mut String,
                          text_start: usize,
                          prev_token_end: usize,
                          text_line: usize,
                          text_col: usize| {
            let trimmed = text_buf.trim();
            if !trimmed.is_empty() {
                children.push(JsxChild::Text(
                    trimmed.to_string(),
                    Span {
                        start: text_start,
                        end: prev_token_end,
                        line: text_line,
                        col: text_col,
                    },
                ));
            }
            text_buf.clear();
        };

        while !self.check(TokenKind::EOF) {
            if self.check(TokenKind::Lt) && self.check_next(TokenKind::Slash) {
                break;
            }

            if self.check(TokenKind::Lt) && self.check_next(TokenKind::Identifier) {
                flush_text(
                    &mut children,
                    &mut text_buf,
                    text_start,
                    prev_token_end,
                    text_line,
                    text_col,
                );
                let child_elem = self.parse_jsx_element()?;
                children.push(JsxChild::Element(Box::new(child_elem)));
                prev_token_end = 0;
                continue;
            }

            if self.match_token(TokenKind::OpenBrace) {
                flush_text(
                    &mut children,
                    &mut text_buf,
                    text_start,
                    prev_token_end,
                    text_line,
                    text_col,
                );
                while self.check(TokenKind::Newline) {
                    self.advance();
                }
                if self.check(TokenKind::For) {
                    let for_child = self.parse_jsx_for_block()?;
                    children.push(for_child);
                    while self.check(TokenKind::Newline) {
                        self.advance();
                    }
                    self.consume(
                        TokenKind::CloseBrace,
                        "expected '}' after JSX child expression",
                    )?;
                    prev_token_end = 0;
                    continue;
                }
                let expr = self.parse_expr()?;
                while self.check(TokenKind::Newline) {
                    self.advance();
                }
                self.consume(
                    TokenKind::CloseBrace,
                    "expected '}' after JSX child expression",
                )?;
                children.push(JsxChild::Expr(expr));
                prev_token_end = 0;
                continue;
            }

            if self.check(TokenKind::For)
                && self.index + 2 < self.tokens.len()
                && self.tokens[self.index + 1].kind == TokenKind::Identifier
                && self.tokens[self.index + 2].kind == TokenKind::In
            {
                flush_text(
                    &mut children,
                    &mut text_buf,
                    text_start,
                    prev_token_end,
                    text_line,
                    text_col,
                );
                let for_child = self.parse_jsx_for_block()?;
                children.push(for_child);
                prev_token_end = 0;
                continue;
            }

            let tok = self.advance();
            if text_buf.is_empty() {
                text_start = tok.span.start;
                text_line = tok.span.line;
                text_col = tok.span.col;
            } else if prev_token_end > 0 && tok.span.start > prev_token_end {
                text_buf.push(' ');
            }
            text_buf.push_str(&tok.lexeme);
            prev_token_end = tok.span.end;
        }

        flush_text(
            &mut children,
            &mut text_buf,
            text_start,
            prev_token_end,
            text_line,
            text_col,
        );

        let close_err = format!("expected '</{}>' to close JSX tag", tag);
        self.consume(TokenKind::Lt, &close_err)?;
        self.consume(TokenKind::Slash, "expected '/' in closing JSX tag")?;
        let close_tag_tok = self.consume(TokenKind::Identifier, "expected closing tag name")?;
        let end_gt = self.consume(TokenKind::Gt, "expected '>' after closing tag name")?;

        if close_tag_tok.lexeme != tag {
            return Err(Diagnostic::new_error(
                format!(
                    "mismatched closing JSX tag: expected '</{}>', found '</{}>'",
                    tag, close_tag_tok.lexeme
                ),
                self.filepath.clone(),
                close_tag_tok.span.clone(),
                None,
                None,
            ));
        }

        Ok(Expr::JsxElement {
            tag,
            attributes,
            children,
            span: Span {
                start: lt_tok.span.start,
                end: end_gt.span.end,
                line: lt_tok.span.line,
                col: lt_tok.span.col,
            },
        })
    }

    fn parse_jsx_for_block(&mut self) -> Result<JsxChild, Diagnostic> {
        let for_tok = self.consume(TokenKind::For, "expected 'for'")?;
        let var_tok = self.consume(TokenKind::Identifier, "expected variable name after 'for'")?;
        let var_name = var_tok.lexeme.clone();
        self.consume(TokenKind::In, "expected 'in' after variable name in 'for' loop")?;
        let iterable = self.parse_expr()?;
        while self.check(TokenKind::Newline) {
            self.advance();
        }
        self.consume(TokenKind::OpenBrace, "expected '{' after for loop iterable")?;

        let mut body = Vec::new();
        let mut text_buf = String::new();
        let mut text_start = 0;
        let mut text_line = 0;
        let mut text_col = 0;
        let mut prev_token_end = 0;

        let flush_text = |children: &mut Vec<JsxChild>,
                          text_buf: &mut String,
                          text_start: usize,
                          prev_token_end: usize,
                          text_line: usize,
                          text_col: usize| {
            let trimmed = text_buf.trim();
            if !trimmed.is_empty() {
                children.push(JsxChild::Text(
                    trimmed.to_string(),
                    Span {
                        start: text_start,
                        end: prev_token_end,
                        line: text_line,
                        col: text_col,
                    },
                ));
            }
            text_buf.clear();
        };

        while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
            if self.check(TokenKind::Newline) && text_buf.trim().is_empty() {
                text_buf.clear();
                self.advance();
                continue;
            }

            if self.check(TokenKind::Lt) && self.check_next(TokenKind::Identifier) {
                flush_text(
                    &mut body,
                    &mut text_buf,
                    text_start,
                    prev_token_end,
                    text_line,
                    text_col,
                );
                let child_elem = self.parse_jsx_element()?;
                body.push(JsxChild::Element(Box::new(child_elem)));
                prev_token_end = 0;
                continue;
            }

            if self.match_token(TokenKind::OpenBrace) {
                flush_text(
                    &mut body,
                    &mut text_buf,
                    text_start,
                    prev_token_end,
                    text_line,
                    text_col,
                );
                while self.check(TokenKind::Newline) {
                    self.advance();
                }
                if self.check(TokenKind::For) {
                    let child = self.parse_jsx_for_block()?;
                    body.push(child);
                    while self.check(TokenKind::Newline) {
                        self.advance();
                    }
                    self.consume(TokenKind::CloseBrace, "expected '}' after JSX expression")?;
                    prev_token_end = 0;
                    continue;
                }
                let expr = self.parse_expr()?;
                while self.check(TokenKind::Newline) {
                    self.advance();
                }
                self.consume(TokenKind::CloseBrace, "expected '}' after JSX child expression")?;
                body.push(JsxChild::Expr(expr));
                prev_token_end = 0;
                continue;
            }

            if self.check(TokenKind::For)
                && self.index + 2 < self.tokens.len()
                && self.tokens[self.index + 1].kind == TokenKind::Identifier
                && self.tokens[self.index + 2].kind == TokenKind::In
            {
                flush_text(
                    &mut body,
                    &mut text_buf,
                    text_start,
                    prev_token_end,
                    text_line,
                    text_col,
                );
                let child = self.parse_jsx_for_block()?;
                body.push(child);
                prev_token_end = 0;
                continue;
            }

            let tok = self.advance();
            if text_buf.is_empty() {
                text_start = tok.span.start;
                text_line = tok.span.line;
                text_col = tok.span.col;
            } else if prev_token_end > 0 && tok.span.start > prev_token_end {
                text_buf.push(' ');
            }
            text_buf.push_str(&tok.lexeme);
            prev_token_end = tok.span.end;
        }

        flush_text(
            &mut body,
            &mut text_buf,
            text_start,
            prev_token_end,
            text_line,
            text_col,
        );

        while self.check(TokenKind::Newline) {
            self.advance();
        }
        let close_brace = self.consume(TokenKind::CloseBrace, "expected '}' to close for loop body")?;
        let span = Span {
            start: for_tok.span.start,
            end: close_brace.span.end,
            line: for_tok.span.line,
            col: for_tok.span.col,
        };

        Ok(JsxChild::For {
            var_name,
            iterable,
            body,
            span,
        })
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.peek();
        match token.kind {
            TokenKind::Lt if self.check_next(TokenKind::Identifier) => {
                let expr = self.parse_jsx_element()?;
                self.parse_accessors(expr)
            }
            TokenKind::At => {
                let mut annotations = Vec::new();
                while self.check(TokenKind::At) {
                    annotations.push(self.parse_annotation()?);
                }
                if self.check(TokenKind::OpenParen) && self.is_closure_lookahead() {
                    return self.parse_closure(annotations);
                }
                return Err(Diagnostic::new_error(
                    "annotations in expressions are only supported on closures".to_string(),
                    self.filepath.clone(),
                    token.span.clone(),
                    None,
                    None,
                ));
            }
            TokenKind::IntLiteral => {
                let tok = self.advance();
                let clean = tok.lexeme.replace('_', "");
                let val = if clean.starts_with("0x") || clean.starts_with("0X") {
                    i64::from_str_radix(&clean[2..], 16).unwrap_or(0)
                } else {
                    clean.parse::<i64>().unwrap_or(0)
                };
                let expr = Expr::Literal(LiteralValue::Int(val), tok.span.clone());
                self.parse_accessors(expr)
            }
            TokenKind::FloatLiteral => {
                let tok = self.advance();
                let clean = tok.lexeme.replace('_', "");
                let val = clean.parse::<f64>().unwrap_or(0.0);
                let expr = Expr::Literal(LiteralValue::Float(val), tok.span.clone());
                self.parse_accessors(expr)
            }
            TokenKind::StringLiteral | TokenKind::MultilineStringLiteral => {
                let is_multi = token.kind == TokenKind::MultilineStringLiteral;
                let tok = self.advance();
                let mut content = tok.lexeme.clone();
                if is_multi {
                    content = strip_common_indentation(&content);
                }
                let expr =
                    Expr::Literal(LiteralValue::String(content), tok.span.clone());
                self.parse_accessors(expr)
            }
            TokenKind::True => {
                let tok = self.advance();
                let expr = Expr::Literal(LiteralValue::Bool(true), tok.span.clone());
                self.parse_accessors(expr)
            }
            TokenKind::False => {
                let tok = self.advance();
                let expr = Expr::Literal(LiteralValue::Bool(false), tok.span.clone());
                self.parse_accessors(expr)
            }
            TokenKind::Nil => {
                let tok = self.advance();
                let expr = Expr::Literal(LiteralValue::Nil, tok.span.clone());
                self.parse_accessors(expr)
            }
            TokenKind::Identifier | TokenKind::SelfLower | TokenKind::Annotation => {
                let peek_tok = self.peek();
                if self.thread_aliases.contains(&peek_tok.lexeme) && self.check_next(TokenKind::OpenBrace) {
                    let start_tok = self.advance();
                    let start_brace = self.peek().span.clone();
                    let block_stmts = self.parse_block()?;
                    let end_brace = self.peek().span.clone();
                    let block_expr = Expr::Block(
                        block_stmts,
                        Span {
                            start: start_brace.start,
                            end: end_brace.start,
                            line: start_brace.line,
                            col: start_brace.col,
                        },
                    );
                    let span = Span {
                        start: start_tok.span.start,
                        end: block_expr.span().end,
                        line: start_tok.span.line,
                        col: start_tok.span.col,
                    };
                    return Ok(Expr::ThreadSpawn(Box::new(block_expr), span));
                }
                let tok = self.advance();
                let expr = Expr::Identifier(tok.lexeme.clone(), tok.span.clone());
                self.parse_accessors(expr)
            }
            TokenKind::Thread => {
                if self.check_next(TokenKind::Dot) {
                    let tok = self.advance();
                    let expr = Expr::Identifier(tok.lexeme.clone(), tok.span.clone());
                    self.parse_accessors(expr)
                } else {
                    let start_tok = self.advance();
                    let block_expr = if self.check(TokenKind::OpenBrace) {
                        let start_brace = self.peek().span.clone();
                        let block_stmts = self.parse_block()?;
                        let end_brace = self.peek().span.clone();
                        Expr::Block(
                            block_stmts,
                            Span {
                                start: start_brace.start,
                                end: end_brace.start,
                                line: start_brace.line,
                                col: start_brace.col,
                            },
                        )
                    } else {
                        self.parse_primary()?
                    };
                    let span = Span {
                        start: start_tok.span.start,
                        end: block_expr.span().end,
                        line: start_tok.span.line,
                        col: start_tok.span.col,
                    };
                    Ok(Expr::ThreadSpawn(Box::new(block_expr), span))
                }
            }
            TokenKind::Formula => {
                let start_tok = self.advance();
                self.consume(TokenKind::OpenBrace, "expected '{' for formula structure")?;
                let mut mappings = Vec::new();
                while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
                    let mut annotations = Vec::new();
                    while self.check(TokenKind::At) {
                        annotations.push(self.parse_annotation()?);
                    }
                    let key_tok =
                        self.consume(TokenKind::Identifier, "expected formula field key")?;
                    self.consume(TokenKind::Colon, "expected ':' separator")?;
                    let val = self.parse_expr()?;
                    mappings.push((key_tok.lexeme.clone(), val, key_tok.span.clone(), annotations));
                    self.match_token(TokenKind::Comma);
                }
                let end_tok = self.consume(
                    TokenKind::CloseBrace,
                    "expected '}' to close formula structure",
                )?;
                Ok(Expr::Formula(
                    mappings,
                    Span {
                        start: start_tok.span.start,
                        end: end_tok.span.end,
                        line: start_tok.span.line,
                        col: start_tok.span.col,
                    },
                ))
            }
            TokenKind::InterpolatedStringStart | TokenKind::MultilineInterpolatedStringStart => {
                let is_multi = token.kind == TokenKind::MultilineInterpolatedStringStart;
                let start_tok = self.advance();
                let mut segments = Vec::new();
                while !self.check(TokenKind::StringEnd) && !self.check(TokenKind::EOF) {
                    if self.check(TokenKind::InterpolatedStringContent) {
                        let tok = self.advance();
                        segments.push(InterpolatedSegment::Text(tok.lexeme.clone()));
                    } else if self.check(TokenKind::InterpolationStart) {
                        self.advance(); // consume '%{'
                        let expr = self.parse_expr()?;
                        segments.push(InterpolatedSegment::Expr(expr));
                        self.consume(
                            TokenKind::InterpolationEnd,
                            "expected '}' to close string interpolation",
                        )?;
                    } else {
                        break;
                    }
                }
                let end_tok =
                    self.consume(TokenKind::StringEnd, "expected ending quote for string")?;
                    
                if is_multi {
                    strip_common_indentation_segments(&mut segments);
                }
                
                Ok(Expr::InterpolatedString(
                    segments,
                    Span {
                        start: start_tok.span.start,
                        end: end_tok.span.end,
                        line: start_tok.span.line,
                        col: start_tok.span.col,
                    },
                ))
            }
            TokenKind::OpenBracket => {
                let start_tok = self.advance();
                let mut elements = Vec::new();
                while !self.check(TokenKind::CloseBracket) && !self.check(TokenKind::EOF) {
                    elements.push(self.parse_expr()?);
                    self.match_token(TokenKind::Comma);
                }
                let end_tok =
                    self.consume(TokenKind::CloseBracket, "expected ']' to close list")?;
                Ok(Expr::VectorLiteral(
                    elements,
                    Span {
                        start: start_tok.span.start,
                        end: end_tok.span.end,
                        line: start_tok.span.line,
                        col: start_tok.span.col,
                    },
                ))
            }
            TokenKind::OpenParen => {
                if self.is_closure_lookahead() {
                    return self.parse_closure(vec![]);
                }
                let start_tok = self.advance();
                let mut expressions = Vec::new();
                while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
                    expressions.push(self.parse_expr()?);
                    self.match_token(TokenKind::Comma);
                }
                let end_tok = self.consume(TokenKind::CloseParen, "expected ')' to close group")?;
                if expressions.len() == 1 {
                    Ok(expressions[0].clone())
                } else {
                    Ok(Expr::Tuple(
                        expressions,
                        Span {
                            start: start_tok.span.start,
                            end: end_tok.span.end,
                            line: start_tok.span.line,
                            col: start_tok.span.col,
                        },
                    ))
                }
            }
            TokenKind::Await => {
                let start_tok = self.advance();
                let expr = self.parse_primary()?;
                let span = Span {
                    start: start_tok.span.start,
                    end: expr.span().end,
                    line: start_tok.span.line,
                    col: start_tok.span.col,
                };
                Ok(Expr::Await(Box::new(expr), span))
            }
            TokenKind::OpenBrace => {
                let start_tok = self.advance();
                let mut mappings = Vec::new();
                while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
                    let mut annotations = Vec::new();
                    while self.check(TokenKind::At) {
                        annotations.push(self.parse_annotation()?);
                    }
                    let key_tok = self.consume(
                        TokenKind::Identifier,
                        "expected object field key",
                    )?;
                    self.consume(
                        TokenKind::Colon,
                        "expected ':' after object field key",
                    )?;
                    let value = self.parse_expr()?;
                    mappings.push((key_tok.lexeme.clone(), value, annotations));
                    self.match_token(TokenKind::Comma);
                }
                let end_tok = self.consume(
                    TokenKind::CloseBrace,
                    "expected '}' to close object",
                )?;
                Ok(Expr::Object(
                    mappings,
                    Span {
                        start: start_tok.span.start,
                        end: end_tok.span.end,
                        line: start_tok.span.line,
                        col: start_tok.span.col,
                    },
                ))
            }
            _ => Err(Diagnostic::new_error(
                format!("expected expression, found '{}'", token.lexeme),
                self.filepath.clone(),
                token.span.clone(),
                Some("Failed to parse expression".to_string()),
                Some("Check your expression formatting here".to_string()),
            )),
        }
    }

    fn is_generic_call_lookahead(&self) -> bool {
        let mut idx = self.index;
        while idx < self.tokens.len() {
            let tok = &self.tokens[idx];
            match tok.kind {
                TokenKind::Gt => {
                    if idx + 1 < self.tokens.len() {
                        return self.tokens[idx + 1].kind == TokenKind::OpenParen;
                    }
                    return false;
                }
                TokenKind::Identifier
                | TokenKind::Comma
                | TokenKind::Lt
                | TokenKind::Star
                | TokenKind::SelfUpper
                | TokenKind::Dollar => {
                    idx += 1;
                }
                _ => break,
            }
        }
        false
    }

    fn is_struct_init_lookahead(&self, target: &Expr) -> bool {
        if !self.check(TokenKind::OpenBrace) {
            return false;
        }
        if self.index + 1 < self.tokens.len() {
            let next = &self.tokens[self.index + 1];
            if next.kind == TokenKind::CloseBrace {
                return match target {
                    Expr::Identifier(name, _) => {
                        name.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
                    }
                    Expr::Dot(_, member, _) => {
                        member.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
                    }
                    _ => false,
                };
            }
            if Self::is_field_name_token(&next.kind) {
                if self.index + 2 < self.tokens.len() {
                    let next2 = &self.tokens[self.index + 2];
                    if next2.kind == TokenKind::Colon {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn is_closure_lookahead(&self) -> bool {
        let mut idx = self.index;
        // Assume current token is OpenParen
        if idx >= self.tokens.len() || self.tokens[idx].kind != TokenKind::OpenParen {
            return false;
        }
        if idx + 1 < self.tokens.len() {
            let next = &self.tokens[idx + 1].kind;
            if *next == TokenKind::CloseParen {
                let after = idx + 2;
                if after < self.tokens.len() {
                    return matches!(self.tokens[after].kind, TokenKind::OpenBrace | TokenKind::Arrow);
                }
                return false;
            }
            if matches!(
                next,
                TokenKind::IntLiteral
                    | TokenKind::FloatLiteral
                    | TokenKind::StringLiteral
                    | TokenKind::MultilineStringLiteral
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::Nil
                    | TokenKind::Exclamation
                    | TokenKind::PlusPlus
                    | TokenKind::MinusMinus
                    | TokenKind::Minus
                    | TokenKind::Plus
                    | TokenKind::Star
                    | TokenKind::Slash
            ) {
                return false;
            }
        }
        let mut parens = 0;
        let mut in_default = false;
        while idx < self.tokens.len() {
            match self.tokens[idx].kind {
                TokenKind::OpenParen => parens += 1,
                TokenKind::CloseParen => {
                    parens -= 1;
                    if parens == 0 {
                        // Look at next token
                        let next_idx = idx + 1;
                        if next_idx < self.tokens.len() {
                            let next_kind = &self.tokens[next_idx].kind;
                            return *next_kind == TokenKind::OpenBrace
                                || *next_kind == TokenKind::Arrow;
                        }
                        return false;
                    }
                }
                TokenKind::Equal if parens == 1 => in_default = true,
                TokenKind::Comma if parens == 1 => in_default = false,
                TokenKind::Pipe2
                | TokenKind::Ampersand2
                | TokenKind::EqualEqual
                | TokenKind::ExclamationEqual
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::QuestionColon
                | TokenKind::Pipe
                | TokenKind::Caret
                | TokenKind::LtLt
                | TokenKind::GtGt
                | TokenKind::Le
                | TokenKind::Ge
                | TokenKind::Exclamation
                | TokenKind::PlusPlus
                | TokenKind::MinusMinus
                    if parens == 1 && !in_default =>
                {
                    return false;
                }
                TokenKind::EOF => return false,
                _ => {}
            }
            idx += 1;
        }
        false
    }

    fn parse_closure(&mut self, annotations: Vec<Annotation>) -> Result<Expr, Diagnostic> {
        let start_tok =
            self.consume(TokenKind::OpenParen, "expected '(' for closure parameters")?;
        let mut params = Vec::new();
        while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
            let mut is_ref = false;
            let mut is_mut = false;

            if self.match_token(TokenKind::Ampersand) {
                is_ref = true;
                if self.match_token(TokenKind::Mut) {
                    is_mut = true;
                }
            }

            let p_name_tok = self.consume(TokenKind::Identifier, "expected parameter name")?;
            let name = p_name_tok.lexeme.clone();

            let mut p_type = "Unknown".to_string();
            if self.match_token(TokenKind::Colon) {
                p_type = self.parse_type()?;
            }

            let mut default_val = None;
            if self.match_token(TokenKind::Equal) {
                default_val = Some(self.parse_expr()?);
            }

            params.push(Param {
                name,
                type_name: p_type,
                default_val,
                is_ref,
                is_mut,
            });

            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }
        self.consume(
            TokenKind::CloseParen,
            "expected ')' to close parameters list",
        )?;

        let mut return_type = None;
        if self.match_token(TokenKind::Arrow) {
            return_type = Some(self.parse_type()?);
        }

        let body = self.parse_block()?;
        let end_span = self.peek().span.clone();

        Ok(Expr::Closure {
            params,
            return_type,
            body,
            annotations,
            span: Span {
                start: start_tok.span.start,
                end: end_span.start,
                line: start_tok.span.line,
                col: start_tok.span.col,
            },
        })
    }

    fn parse_accessors(&mut self, mut expr: Expr) -> Result<Expr, Diagnostic> {
        loop {
            if self.match_token(TokenKind::Dot) {
                let name = if matches!(
                    self.peek().kind,
                    TokenKind::Identifier
                        | TokenKind::Yield
                        | TokenKind::Type
                        | TokenKind::Formula
                        | TokenKind::Async
                        | TokenKind::Await
                        | TokenKind::Thread
                        | TokenKind::Match
                        | TokenKind::As
                ) {
                    self.advance()
                } else {
                    self.consume(TokenKind::Identifier, "expected member identifier after '.'")?
                };
                let span = Span {
                    start: expr.span().start,
                    end: name.span.end,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::Dot(Box::new(expr), name.lexeme.clone(), span);
            } else if self.match_token(TokenKind::QuestionDot) {
                let name = if matches!(
                    self.peek().kind,
                    TokenKind::Identifier
                        | TokenKind::Yield
                        | TokenKind::Type
                        | TokenKind::Formula
                        | TokenKind::Async
                        | TokenKind::Await
                        | TokenKind::Thread
                        | TokenKind::Match
                        | TokenKind::As
                ) {
                    self.advance()
                } else {
                    self.consume(TokenKind::Identifier, "expected member identifier after '?.'")?
                };
                let span = Span {
                    start: expr.span().start,
                    end: name.span.end,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::SafeDot(Box::new(expr), name.lexeme.clone(), span);
            } else if self.match_token(TokenKind::PlusPlus) {
                let span = Span {
                    start: expr.span().start,
                    end: self.tokens[self.index - 1].span.end,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::Unary(UnaryOp::PostInc, Box::new(expr), span);
            } else if self.match_token(TokenKind::MinusMinus) {
                let span = Span {
                    start: expr.span().start,
                    end: self.tokens[self.index - 1].span.end,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::Unary(UnaryOp::PostDec, Box::new(expr), span);
            } else if self.match_token(TokenKind::Exclamation) {
                let span = Span {
                    start: expr.span().start,
                    end: self.tokens[self.index - 1].span.end,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::Unary(UnaryOp::NonNullAssert, Box::new(expr), span);
            } else if self.check(TokenKind::Lt) && self.is_generic_call_lookahead() {
                self.advance(); // consume '<'
                while !self.check(TokenKind::Gt) && !self.check(TokenKind::EOF) {
                    self.advance();
                }
                self.consume(
                    TokenKind::Gt,
                    "expected '>' to close generic type arguments",
                )?;
            } else if self.match_token(TokenKind::OpenParen) {
                let mut args = Vec::new();
                while !self.check(TokenKind::CloseParen) && !self.check(TokenKind::EOF) {
                    let mut arg_name = None;
                    if self.check(TokenKind::Identifier) {
                        let id = self.peek().lexeme.clone();
                        if self.index + 1 < self.tokens.len()
                            && self.tokens[self.index + 1].kind == TokenKind::Colon
                        {
                            self.advance(); // consume identifier
                            self.advance(); // consume ':'
                            arg_name = Some(id);
                        }
                    }
                    let val = self.parse_expr()?;
                    args.push((arg_name, val));
                    self.match_token(TokenKind::Comma);
                }
                let end_tok = self.consume(
                    TokenKind::CloseParen,
                    "expected ')' to close argument calls",
                )?;
                let span = Span {
                    start: expr.span().start,
                    end: end_tok.span.end,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::Call(Box::new(expr), args, span);
            } else if self.is_struct_init_lookahead(&expr) {
                self.advance(); // consume '{'
                let mut fields = Vec::new();
                while !self.check(TokenKind::CloseBrace) && !self.check(TokenKind::EOF) {
                    let field_tok =
                        self.consume_field_name("expected struct field name")?;
                    self.consume(TokenKind::Colon, "expected ':' after field name")?;
                    let val = self.parse_expr()?;
                    fields.push((field_tok.lexeme.clone(), val));
                    self.match_token(TokenKind::Comma);
                }
                let end_tok =
                    self.consume(TokenKind::CloseBrace, "expected '}' to close struct init")?;
                let span = Span {
                    start: expr.span().start,
                    end: end_tok.span.end,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::StructInit(Box::new(expr), fields, span);
            } else if self.match_token(TokenKind::OpenBracket) {
                let index_expr = self.parse_expr()?;
                let end_tok = self.consume(
                    TokenKind::CloseBracket,
                    "expected ']' to close index expression",
                )?;
                let span = Span {
                    start: expr.span().start,
                    end: end_tok.span.end,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::Index(Box::new(expr), Box::new(index_expr), span);
            } else if self.match_token(TokenKind::As) {
                let type_str = self.parse_type()?;
                let end_pos = self.tokens[self.index - 1].span.end;
                let span = Span {
                    start: expr.span().start,
                    end: end_pos,
                    line: expr.span().line,
                    col: expr.span().col,
                };
                expr = Expr::Cast(Box::new(expr), type_str, span);
            } else {
                break;
            }
        }
        Ok(expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_code(code: &str) -> Vec<Stmt> {
        let mut lexer = Lexer::new(code);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == TokenKind::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = Parser::new(tokens, "test.fm".to_string());
        parser.parse().unwrap()
    }

    #[test]
    fn test_parse_jsx() {
        let code = r#"
        @Component
        fn render() {
            <main>
                <h1>Counter</h1>
                <button onclick={count += 1}>
                    Count: {count}
                </button>
            </main>
        }
        "#;
        let stmts = parse_code(code);
        assert_eq!(stmts.len(), 1);
        if let Stmt::FuncDecl { body: Some(body), .. } = &stmts[0] {
            if let Stmt::ExprStmt(Expr::JsxElement { tag, children, .. }) = &body[0] {
                assert_eq!(tag, "main");
                assert_eq!(children.len(), 2);
            } else {
                panic!("Expected JsxElement");
            }
        } else {
            panic!("Expected FuncDecl");
        }
    }

    #[test]
    fn test_parse_jsx_block() {
        let code = r#"
        @Web
        fn main() {
            @Page("/")
            fn index() {
                @State
                let mut count = 0

                <main>
                    <h1>Counter</h1>
                    <button onClick={
                        count += 1;
                    }>
                        Count: {count}
                    </button>
                </main>
            }
        }
        "#;
        let stmts = parse_code(code);
        assert!(!stmts.is_empty());
    }

    #[test]
    fn test_parse_jsx_without_web_annotation_fails() {
        let code = r#"
        fn render() {
            <main><h1>Invalid</h1></main>
        }
        "#;
        let mut lexer = Lexer::new(code);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == TokenKind::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = Parser::new(tokens, "test.fm".to_string());
        let res = parser.parse();
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.message.contains("JSX syntax is only permitted within functions annotated with a web annotation"));
    }
}

