#![allow(dead_code)]
use crate::lexer::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub filepath: String,
    pub span: Span,
    pub label: Option<String>,
    pub suggestion: Option<String>,
    pub note: Option<String>,
}

impl Diagnostic {
    pub fn new_error(
        message: String,
        filepath: String,
        span: Span,
        label: Option<String>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            message,
            filepath,
            span,
            label,
            suggestion,
            note: None,
        }
    }

    pub fn new_warning(
        message: String,
        filepath: String,
        span: Span,
        label: Option<String>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            message,
            filepath,
            span,
            label,
            suggestion,
            note: None,
        }
    }

    pub fn with_note(mut self, note: String) -> Self {
        self.note = Some(note);
        self
    }

    pub fn print(&self, source: &str) {
        let (icon, color_code, severity_name) = match self.severity {
            DiagnosticSeverity::Error => ("✖", "\x1b[1;31m", "error"),     // Bold Red
            DiagnosticSeverity::Warning => ("⚠", "\x1b[1;33m", "warning"), // Bold Yellow
            DiagnosticSeverity::Info => ("ℹ", "\x1b[1;34m", "info"),       // Bold Blue
        };
        let bold = "\x1b[1m";
        let cyan = "\x1b[1;36m";
        let reset = "\x1b[0m";

        println!(
            "{}{} {}{}: {}{}{}",
            color_code, icon, severity_name, reset, bold, self.message, reset
        );
        println!(
            "  {}-->{} {}:{}:{}",
            cyan, reset, self.filepath, self.span.line, self.span.col
        );

        // Extract the offending line and context
        let lines: Vec<&str> = source.lines().collect();
        let line_idx = self.span.line.saturating_sub(1);

        if line_idx < lines.len() {
            let line_num_str = format!(" {} |", self.span.line);
            let spacer = format!(" {} |", " ".repeat(self.span.line.to_string().len()));
            println!("{}{}{}", cyan, spacer, reset);
            println!("{}{}{} {}", cyan, line_num_str, reset, lines[line_idx]);

            // Print the underline pointer
            let col = self.span.col.saturating_sub(1);
            let len = self.span.end.saturating_sub(self.span.start).max(1);
            let underline = "^".repeat(len);
            let indent = " ".repeat(col);

            let label_str = match &self.label {
                Some(l) => format!(" {}", l),
                None => String::new(),
            };

            println!(
                "{}{}{} {}{}{}{}{}",
                cyan, spacer, reset, indent, color_code, underline, label_str, reset
            );
            println!("{}{}{}", cyan, spacer, reset);
        }

        if let Some(note) = &self.note {
            println!("  \x1b[1;36m=\x1b[0m \x1b[1mnote:\x1b[0m {}", note);
        }
        if let Some(sug) = &self.suggestion {
            println!("  \x1b[1;36m=\x1b[0m \x1b[1mhelp:\x1b[0m {}", sug);
        }
        println!();
    }
}
