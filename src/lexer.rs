#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Let,
    Const,
    Fn,
    Struct,
    Enum,
    Trait,
    Impl,
    Export,
    Import,
    Package,
    In,
    Match,
    If,
    Else,
    For,
    While,
    Loop,
    Break,
    Continue,
    Defer,
    Async,
    Await,
    Thread,
    Yield,
    Return,
    Mut,
    As,
    SelfLower,
    SelfUpper,
    Type,
    Where,
    Formula,
    Annotation,
    True,
    False,
    Nil,
    Comment,
    Newline,

    // Literals
    Identifier,
    IntLiteral,
    FloatLiteral,
    StringLiteral,
    MultilineStringLiteral,

    // Interpolation Tokens
    InterpolatedStringStart,   // $"
    MultilineInterpolatedStringStart, // $"""
    InterpolatedStringContent, // raw characters inside
    InterpolationStart,        // %{
    InterpolationEnd,          // }
    StringEnd,                 // "

    // Symbols & Operators
    Arrow,            // ->
    FatArrow,         // =>
    Equal,            // =
    EqualEqual,       // ==
    Plus,             // +
    PlusPlus,         // ++
    PlusEqual,        // +=
    Minus,            // -
    MinusMinus,       // --
    MinusEqual,       // -=
    Star,             // *
    StarEqual,        // *=
    Slash,            // /
    SlashEqual,       // /=
    Percent,          // %
    PercentEqual,     // %=
    Ampersand,        // &
    AmpersandEqual,   // &=
    Pipe,             // |
    PipeEqual,        // |=
    Caret,            // ^
    CaretEqual,       // ^=
    Ampersand2,       // &&
    Pipe2,            // ||
    Dot,              // .
    Comma,            // ,
    Colon,            // :
    At,               // @
    Dollar,           // $
    Question,         // ?
    QuestionDot,      // ?.
    QuestionColon,    // ?:
    Exclamation,      // !
    ExclamationEqual, // !=
    OpenParen,        // (
    CloseParen,       // )
    OpenBracket,      // [
    CloseBracket,     // ]
    OpenBrace,        // {
    CloseBrace,       // }
    DoubleDot,        // ..
    DoubleDotEqual,   // ..=
    Lt,               // <
    Le,               // <=
    LtLt,             // <<
    ShlEqual,         // <<=
    Gt,               // >
    Ge,               // >=
    GtGt,             // >>
    ShrEqual,         // >>=

    // End of File
    EOF,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<char>,
    char_indices: Vec<usize>,
    index: usize,
    line: usize,
    col: usize,
    // Stack to keep track of string interpolation state
    // (inside_expression: bool, is_multiline: bool)
    pub interpolation_stack: Vec<(bool, bool)>,
    pub keep_comments: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let char_indices = source.char_indices().map(|(i, _)| i).collect();
        Self {
            source,
            chars: source.chars().collect(),
            char_indices,
            index: 0,
            line: 1,
            col: 1,
            interpolation_stack: Vec::new(),
            keep_comments: false,
        }
    }

    fn byte_pos(&self, char_idx: usize) -> usize {
        if char_idx < self.char_indices.len() {
            self.char_indices[char_idx]
        } else {
            self.source.len()
        }
    }

    fn peek(&self) -> Option<char> {
        if self.index < self.chars.len() {
            Some(self.chars[self.index])
        } else {
            None
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.index + 1 < self.chars.len() {
            Some(self.chars[self.index + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.index < self.chars.len() {
            let ch = self.chars[self.index];
            self.index += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += ch.len_utf16();
            }
            Some(ch)
        } else {
            None
        }
    }

    pub fn next_token(&mut self) -> Token {
        let start_pos_before_skip = self.index;
        let start_line_before_skip = self.line;
        let start_col_before_skip = self.col;

        let in_string_text = matches!(self.interpolation_stack.last(), Some(&(false, _)));
        
        if !in_string_text {
            if let Some(kind) = self.skip_whitespace_and_comments() {
            return Token {
                kind,
                lexeme: self.source
                    [self.byte_pos(start_pos_before_skip)..self.byte_pos(self.index)]
                    .to_string(),
                span: Span {
                    start: self.byte_pos(start_pos_before_skip),
                    end: self.byte_pos(self.index),
                    line: start_line_before_skip,
                    col: start_col_before_skip,
                },
            };
            }
        }

        let start_pos = self.index;
        let start_line = self.line;
        let start_col = self.col;

        let ch = match self.advance() {
            Some(c) => c,
            None => {
                return Token {
                    kind: TokenKind::EOF,
                    lexeme: String::new(),
                    span: Span {
                        start: self.byte_pos(start_pos),
                        end: self.byte_pos(start_pos),
                        line: start_line,
                        col: start_col,
                    },
                };
            }
        };

        // If we are currently scanning inside an interpolated string, we have to look for %{ or "
        if let Some(&(false, is_multi)) = self.interpolation_stack.last() {
            // We are scanning the text part of a string
            if is_multi {
                if ch == '"' && self.peek() == Some('"') && self.peek_next() == Some('"') {
                    self.advance(); // consume 2nd "
                    self.advance(); // consume 3rd "
                    self.interpolation_stack.pop();
                    return Token {
                        kind: TokenKind::StringEnd,
                        lexeme: "\"\"\"".to_string(),
                        span: Span {
                            start: self.byte_pos(start_pos),
                            end: self.byte_pos(self.index),
                            line: start_line,
                            col: start_col,
                        },
                    };
                }
            } else {
                if ch == '"' {
                    self.interpolation_stack.pop();
                    return Token {
                        kind: TokenKind::StringEnd,
                        lexeme: "\"".to_string(),
                        span: Span {
                            start: self.byte_pos(start_pos),
                            end: self.byte_pos(self.index),
                            line: start_line,
                            col: start_col,
                        },
                    };
                }
            }
            
            if ch == '{' {
                // Toggle the top of the stack to true (we are inside an expression now)
                self.interpolation_stack.last_mut().unwrap().0 = true;
                return Token {
                    kind: TokenKind::InterpolationStart,
                    lexeme: "{".to_string(),
                    span: Span {
                        start: self.byte_pos(start_pos),
                        end: self.byte_pos(self.index),
                        line: start_line,
                        col: start_col,
                    },
                };
            } else {
                // Read text until " or {
                let mut content = String::new();
                let mut current = ch;
                
                loop {
                    if current == '\\' {
                        if let Some(escaped) = self.peek() {
                            self.advance(); // consume the escaped char
                            match escaped {
                                'n' => content.push('\n'),
                                'r' => content.push('\r'),
                                't' => content.push('\t'),
                                '\\' => content.push('\\'),
                                '"' => content.push('"'),
                                '{' => content.push('{'),
                                '}' => content.push('}'),
                                _ => {
                                    content.push('\\');
                                    content.push(escaped);
                                }
                            }
                        } else {
                            content.push('\\');
                        }
                    } else {
                        content.push(current);
                    }
                    
                    if let Some(next) = self.peek() {
                        if next == '{' {
                            break;
                        }
                        if is_multi {
                            if next == '"' && self.peek_next() == Some('"') && self.index + 2 < self.chars.len() && self.chars[self.index + 2] == '"' {
                                break;
                            }
                        } else {
                            if next == '"' {
                                break;
                            }
                        }
                        current = self.advance().unwrap();
                    } else {
                        break;
                    }
                }
                
                return Token {
                    kind: TokenKind::InterpolatedStringContent,
                    lexeme: content,
                    span: Span {
                        start: self.byte_pos(start_pos),
                        end: self.byte_pos(self.index),
                        line: start_line,
                        col: start_col,
                    },
                };
            }
        }

        // Standard character handling
        let kind = match ch {
            '(' => TokenKind::OpenParen,
            ')' => {
                // If we are in an interpolation expression block and hit a CloseBrace or CloseParen, we must handle it.
                TokenKind::CloseParen
            }
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            '{' => TokenKind::OpenBrace,
            '}' => {
                // Check if we are inside an interpolation expression block.
                // If the top of the stack is true, this '}' finishes the interpolation expression!
                if let Some(&(true, _)) = self.interpolation_stack.last() {
                    self.interpolation_stack.last_mut().unwrap().0 = false; // toggle back to scanning string text
                    TokenKind::InterpolationEnd
                } else {
                    TokenKind::CloseBrace
                }
            }
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '@' => TokenKind::At,
            '?' => {
                if self.peek() == Some('.') {
                    self.advance();
                    TokenKind::QuestionDot
                } else if self.peek() == Some(':') {
                    self.advance();
                    TokenKind::QuestionColon
                } else {
                    TokenKind::Question
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::ExclamationEqual
                } else {
                    TokenKind::Exclamation
                }
            }
            '+' => {
                if self.peek() == Some('+') {
                    self.advance();
                    TokenKind::PlusPlus
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PlusEqual
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else if self.peek() == Some('-') {
                    self.advance();
                    TokenKind::MinusMinus
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::MinusEqual
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::StarEqual
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                if self.keep_comments && self.peek() == Some('/') {
                    self.advance();
                    while let Some(ch) = self.peek() {
                        if ch == '\n' {
                            break;
                        }
                        self.advance();
                    }
                    TokenKind::Comment
                } else if self.keep_comments && self.peek() == Some('*') {
                    self.advance();
                    while let Some(ch) = self.peek() {
                        if ch == '*' && self.peek_next() == Some('/') {
                            self.advance();
                            self.advance();
                            break;
                        }
                        self.advance();
                    }
                    TokenKind::Comment
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::SlashEqual
                } else {
                    TokenKind::Slash
                }
            }
            '%' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PercentEqual
                } else {
                    TokenKind::Percent
                }
            }
            '^' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::CaretEqual
                } else {
                    TokenKind::Caret
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::DoubleDotEqual
                    } else {
                        TokenKind::DoubleDot
                    }
                } else {
                    TokenKind::Dot
                }
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::EqualEqual
                } else if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Equal
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    TokenKind::Ampersand2
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::AmpersandEqual
                } else {
                    TokenKind::Ampersand
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    TokenKind::Pipe2
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PipeEqual
                } else {
                    TokenKind::Pipe
                }
            }
            '$' => {
                if self.peek() == Some('"') {
                    self.advance(); // consume '"'
                    
                    let mut is_multi = false;
                    if self.peek() == Some('"') && self.peek_next() == Some('"') {
                        self.advance(); // consume 2nd "
                        self.advance(); // consume 3rd "
                        is_multi = true;
                    }
                    let kind = if is_multi {
                        TokenKind::MultilineInterpolatedStringStart
                    } else {
                        TokenKind::InterpolatedStringStart
                    };
                    
                    self.interpolation_stack.push((false, is_multi)); // start scanning string text
                    kind
                } else {
                    TokenKind::Dollar
                }
            }
            '<' => {
                if self.peek() == Some('<') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::ShlEqual
                    } else {
                        TokenKind::LtLt
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                if self.peek() == Some('>') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::ShrEqual
                    } else {
                        TokenKind::GtGt
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            '"' => {
                let mut content = String::new();
                let mut is_multi = false;
                
                if self.peek() == Some('"') && self.peek_next() == Some('"') {
                    self.advance();
                    self.advance();
                    is_multi = true;
                }

                while let Some(next) = self.peek() {
                    if next == '\\' {
                        self.advance();
                        if let Some(escaped) = self.peek() {
                            match escaped {
                                'n' => content.push('\n'),
                                'r' => content.push('\r'),
                                't' => content.push('\t'),
                                '\\' => content.push('\\'),
                                '"' => content.push('"'),
                                _ => {
                                    content.push('\\');
                                    content.push(escaped);
                                }
                            }
                            self.advance();
                        }
                        continue;
                    }
                    if is_multi {
                        if next == '"' && self.peek_next() == Some('"') && self.index + 2 < self.chars.len() && self.chars[self.index + 2] == '"' {
                            self.advance();
                            self.advance();
                            self.advance();
                            break;
                        }
                    } else {
                        if next == '"' {
                            self.advance();
                            break;
                        }
                    }

                    content.push(self.advance().unwrap());
                }

                let kind = if is_multi {
                    TokenKind::MultilineStringLiteral
                } else {
                    TokenKind::StringLiteral
                };

                return Token {
                    kind,
                    lexeme: content,
                    span: Span {
                        start: self.byte_pos(start_pos),
                        end: self.byte_pos(self.index),
                        line: start_line,
                        col: start_col,
                    },
                };
            }
            '\'' => {
                let has_closing_on_line = {
                    let mut i = self.index;
                    let mut found = false;
                    while i < self.chars.len() {
                        let c = self.chars[i];
                        if c == '\n' {
                            break;
                        }
                        if c == '\\' {
                            i += 2;
                            continue;
                        }
                        if c == '\'' {
                            found = true;
                            break;
                        }
                        i += 1;
                    }
                    found
                };

                if has_closing_on_line {
                    let mut content = String::new();
                    while let Some(next) = self.peek() {
                        if next == '\\' {
                            self.advance();
                            if let Some(escaped) = self.peek() {
                                match escaped {
                                    'n' => content.push('\n'),
                                    'r' => content.push('\r'),
                                    't' => content.push('\t'),
                                    '\\' => content.push('\\'),
                                    '\'' => content.push('\''),
                                    _ => {
                                        content.push('\\');
                                        content.push(escaped);
                                    }
                                }
                                self.advance();
                            }
                            continue;
                        }
                        if next == '\'' {
                            self.advance();
                            break;
                        }
                        content.push(self.advance().unwrap());
                    }

                    return Token {
                        kind: TokenKind::StringLiteral,
                        lexeme: content,
                        span: Span {
                            start: self.byte_pos(start_pos),
                            end: self.byte_pos(self.index),
                            line: start_line,
                            col: start_col,
                        },
                    };
                } else {
                    return Token {
                        kind: TokenKind::Identifier,
                        lexeme: "'".to_string(),
                        span: Span {
                            start: self.byte_pos(start_pos),
                            end: self.byte_pos(self.index),
                            line: start_line,
                            col: start_col,
                        },
                    };
                }
            }
            _ => {
                if ch.is_ascii_digit() {
                    self.scan_number(ch)
                } else if ch.is_alphabetic() || ch == '_' {
                    self.scan_identifier(ch)
                } else {
                    // Unknown character, yield Identifier representing error or fallback
                    TokenKind::Identifier
                }
            }
        };

        let lexeme = self.source[self.byte_pos(start_pos)..self.byte_pos(self.index)].to_string();
        let mut resolved_kind = kind;
        if resolved_kind == TokenKind::Identifier {
            resolved_kind = Self::check_keyword(&lexeme);
        }

        Token {
            kind: resolved_kind,
            lexeme,
            span: Span {
                start: self.byte_pos(start_pos),
                end: self.byte_pos(self.index),
                line: start_line,
                col: start_col,
            },
        }
    }

    fn scan_number(&mut self, first_char: char) -> TokenKind {
        if first_char == '0' && (self.peek() == Some('x') || self.peek() == Some('X')) {
            self.advance(); // consume 'x' or 'X'
            while let Some(next) = self.peek() {
                if next.is_ascii_hexdigit() || next == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            return TokenKind::IntLiteral;
        }
        let mut has_dot = false;
        while let Some(next) = self.peek() {
            if next.is_ascii_digit() || next == '_' {
                self.advance();
            } else if next == '.' && !has_dot {
                // Make sure it is not double dots `..` or `..=` or safe navigation `?.`
                if let Some(after) = self.peek_next() {
                    if after == '.' || after.is_ascii_alphabetic() || after == '_' {
                        break;
                    }
                }
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }
        if has_dot {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntLiteral
        }
    }

    fn scan_identifier(&mut self, _first_char: char) -> TokenKind {
        while let Some(next) = self.peek() {
            if next.is_alphanumeric() || next == '_' {
                self.advance();
            } else {
                break;
            }
        }
        TokenKind::Identifier
    }

    pub fn check_keyword(lexeme: &str) -> TokenKind {
        match lexeme {
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "fn" => TokenKind::Fn,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "export" => TokenKind::Export,
            "import" => TokenKind::Import,
            "package" => TokenKind::Package,
            "in" => TokenKind::In,
            "match" => TokenKind::Match,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "loop" => TokenKind::Loop,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "defer" => TokenKind::Defer,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "thread" => TokenKind::Thread,
            "yield" => TokenKind::Yield,
            "return" => TokenKind::Return,
            "mut" => TokenKind::Mut,
            "as" => TokenKind::As,
            "self" => TokenKind::SelfLower,
            "Self" => TokenKind::SelfUpper,
            "type" => TokenKind::Type,
            "where" => TokenKind::Where,
            "formula" => TokenKind::Formula,
            "annotation" => TokenKind::Annotation,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            "and" => TokenKind::Ampersand2,
            "or" => TokenKind::Pipe2,
            "not" => TokenKind::Exclamation,
            _ => TokenKind::Identifier,
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Option<TokenKind> {
        loop {
            match self.peek() {
                Some('\u{feff}') | Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('\n') | Some(';') => {
                    self.advance();
                    if self.keep_comments {
                        return Some(TokenKind::Newline);
                    }
                }
                Some('/') => {
                    if self.keep_comments {
                        break None;
                    }
                    if self.peek_next() == Some('/') {
                        // Line comment
                        self.advance();
                        self.advance();
                        while let Some(ch) = self.peek() {
                            if ch == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    } else if self.peek_next() == Some('*') {
                        // Block comment
                        self.advance();
                        self.advance();
                        while let Some(ch) = self.peek() {
                            if ch == '*' && self.peek_next() == Some('/') {
                                self.advance();
                                self.advance();
                                break;
                            }
                            self.advance();
                        }
                    } else {
                        break None;
                    }
                }
                _ => break None,
            }
        }
    }
}
