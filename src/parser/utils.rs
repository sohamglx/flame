use super::ast::{InterpolatedSegment, Stmt};

pub fn strip_common_indentation(s: &str) -> String {
    let mut min_indent = usize::MAX;
    let lines: Vec<&str> = s.lines().collect();

    // Find minimum indentation ignoring the first line (if it's empty) and purely empty lines
    for (i, line) in lines.iter().enumerate() {
        if i == 0 && line.trim().is_empty() {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        if indent < min_indent {
            min_indent = indent;
        }
    }

    if min_indent == usize::MAX {
        min_indent = 0;
    }

    let mut result = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i == 0 && line.trim().is_empty() {
            continue; // Skip the first line completely if it's empty
        }
        if i > 0 || (i == 0 && !line.trim().is_empty()) {
            if !result.is_empty() {
                result.push('\n');
            }
            if line.trim().is_empty() {
                // Keep empty lines as empty
            } else if line.len() >= min_indent {
                result.push_str(&line[min_indent..]);
            } else {
                result.push_str(line);
            }
        }
    }

    result
}

pub fn strip_common_indentation_segments(segments: &mut Vec<InterpolatedSegment>) {
    let mut min_indent = usize::MAX;

    // Find min indent
    for seg in segments.iter() {
        if let InterpolatedSegment::Text(text) = seg {
            let lines: Vec<&str> = text.lines().collect();
            // For interpolated strings, we must be careful:
            // A segment could be `\n    hello `. The `\n` means it's a new line.
            for (i, line) in lines.iter().enumerate() {
                if i == 0 && !text.starts_with('\n') {
                    continue; // This line continues from an expression, so it's not a fresh line
                }
                if line.trim().is_empty() {
                    continue;
                }
                let indent = line.chars().take_while(|c| c.is_whitespace()).count();
                if indent < min_indent {
                    min_indent = indent;
                }
            }
        }
    }

    if min_indent == usize::MAX {
        min_indent = 0;
    }

    let mut is_first_line = true;
    let mut skipped_first_line = false;

    // Apply min indent
    for seg in segments.iter_mut() {
        if let InterpolatedSegment::Text(text) = seg {
            let mut result = String::new();
            let lines: Vec<&str> = text.split('\n').collect(); // use split instead of lines to preserve trailing empty strings

            for (i, line) in lines.iter().enumerate() {
                if i == 0 && is_first_line && line.trim().is_empty() {
                    // skip first empty line
                    is_first_line = false;
                    skipped_first_line = true;
                    continue;
                }
                is_first_line = false;

                if i > 0 {
                    // Only omit the newline if we skipped the very first line of the string AND this is the first line we're adding
                    if skipped_first_line && result.is_empty() {
                        skipped_first_line = false;
                    } else {
                        result.push('\n');
                    }
                }

                if i == 0 && !text.starts_with('\n') {
                    // This is continuing from an expression, don't strip indentation
                    result.push_str(line);
                } else {
                    if line.trim().is_empty() {
                        // just empty
                    } else if line.len() >= min_indent {
                        result.push_str(&line[min_indent..]);
                    } else {
                        result.push_str(line);
                    }
                }
            }
            *text = result;
        }
    }
}

pub fn is_test_annotation(name: &str) -> bool {
    matches!(
        name,
        "Test"
            | "Setup"
            | "Cleanup"
            | "BeforeAll"
            | "AfterAll"
            | "Benchmark"
            | "Ignore"
            | "Only"
            | "Parameterized"
            | "ExpectPanic"
    )
}

pub fn is_test_statement(stmt: &Stmt) -> bool {
    let check_stmt = if let Stmt::ExportDecl(inner, _) = stmt {
        inner.as_ref()
    } else {
        stmt
    };
    match check_stmt {
        Stmt::FuncDecl { annotations, .. } => {
            annotations.iter().any(|a| is_test_annotation(&a.name))
        }
        _ => false,
    }
}

pub fn filter_platform_stmts(stmts: &mut Vec<Stmt>, current_target: Option<&str>) {
    stmts.retain(|stmt| {
        let annotations = match stmt {
            Stmt::FuncDecl { annotations, .. } => annotations,
            Stmt::StructDecl { annotations, .. } => annotations,
            Stmt::EnumDecl { annotations, .. } => annotations,
            Stmt::LetDecl { annotations, .. } => annotations,
            Stmt::ConstDecl { annotations, .. } => annotations,
            _ => return true,
        };

        for ann in annotations {
            if ann.name == "Platform" && !ann.args.is_empty() {
                let required_platform = ann.args[0].trim_matches('"');
                if let Some(target) = current_target {
                    if !target.contains(required_platform) {
                        return false;
                    }
                } else {
                    if !required_platform.is_empty()
                        && !std::env::consts::OS.contains(&required_platform.to_lowercase())
                    {
                        return false;
                    }
                }
            }
        }
        true
    });
}
