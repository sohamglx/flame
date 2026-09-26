use std::collections::{HashMap, HashSet};
use crate::diagnostics::Diagnostic;
use crate::lexer::Span;
use crate::parser::*;
use super::types::*;

pub struct TypeChecker {
    pub(crate) filepath: String,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) scopes: Vec<HashMap<String, VarInfo>>,
    pub functions: HashMap<String, FunctionSig>,
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub methods: HashMap<String, HashMap<String, FunctionSig>>,
    pub commands: HashMap<String, CommandInfo>,
    pub(crate) current_return_type: Option<Type>,
    pub hover_info: HashMap<Span, String>,
    pub modules: HashSet<String>,
    pub module_docs: HashMap<String, String>,
    pub plugins: HashSet<String>,
    pub plugin_methods: HashMap<String, HashMap<String, Type>>,
    pub plugin_functions: HashMap<String, HashMap<String, FunctionSig>>,
    pub annotations: HashSet<String>,
    pub is_importing: bool,
    pub(crate) expected_closure_type: Option<Type>,
    pub defined_functions_in_file: HashMap<String, Vec<Option<String>>>,
    pub defined_types_in_file: HashMap<String, Vec<(String, Option<String>)>>,
    pub in_annotation_decl: bool,
    pub in_expect_panic: bool,
}


pub(crate) fn get_platform_annotation(annotations: &[Annotation]) -> Option<String> {
    for ann in annotations {
        if ann.name.eq_ignore_ascii_case("Platform") && !ann.args.is_empty() {
            let mut raw = ann.args[0].trim();
            if let Some(pos) = raw.find(':') {
                raw = raw[pos + 1..].trim();
            }
            let p = raw
                .trim_matches(|c| c == '\'' || c == '"' || c == '[' || c == ']')
                .trim()
                .to_lowercase();
            if !p.is_empty() {
                return Some(p);
            }
        }
    }
    None
}


impl TypeChecker {
    pub fn insert_hover_info(&mut self, span: crate::lexer::Span, info: String) {
        if !self.is_importing {
            self.hover_info.insert(span, info);
        }
    }

    pub fn new(filepath: String) -> Self {
        let mut checker = Self {
            filepath,
            diagnostics: Vec::new(),
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            methods: HashMap::new(),
            commands: HashMap::new(),
            current_return_type: None,
            hover_info: HashMap::new(),
            modules: HashSet::new(),
            module_docs: HashMap::new(),
            plugins: HashSet::new(),
            plugin_methods: HashMap::new(),
            plugin_functions: HashMap::new(),
            annotations: HashSet::new(),
            is_importing: false,
            expected_closure_type: None,
            defined_functions_in_file: HashMap::new(),
            defined_types_in_file: HashMap::new(),
            in_annotation_decl: false,
            in_expect_panic: false,
        };
        checker.register_builtins();
        checker
    }

    pub fn check_program(mut self, stmts: &[Stmt]) -> (Result<(), Vec<Diagnostic>>, Self) {
        self.collect_top_level_declarations(stmts);
        for stmt in stmts {
            self.check_stmt(stmt);
        }

        let res = if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(self.diagnostics.clone())
        };
        (res, self)
    }

    pub(crate) fn process_annotations(&mut self, annotations: &[Annotation]) -> Option<String> {
        let mut docs = Vec::new();
        for ann in annotations {
            if ann.name == "Docs" {
                self.insert_hover_info(
                    ann.name_span.clone(),
                    "```flame\n@Docs(text: String)\n```\n\nAdds documentation to declarations. This documentation will appear when hovering over the declared item.\n\n**Example:**\n```flame\n@Docs(\"Adds two numbers\")\nfn add(a: Int, b: Int) -> Int {\n    return a + b\n}\n```".to_string()
                );

                if !ann.args.is_empty() {
                    let mut raw = ann.args[0].clone();
                    if raw.starts_with('"') && raw.ends_with('"') {
                        raw = raw[1..raw.len() - 1].to_string();
                    }
                    let unquoted = raw.replace("\\n", "\n");
                    let lines: Vec<&str> = unquoted.lines().collect();
                    let mut min_indent = usize::MAX;
                    for line in &lines {
                        if line.trim().is_empty() {
                            continue;
                        }
                        let indent = line.chars().take_while(|c| *c == ' ' || *c == '\t').count();
                        if indent < min_indent {
                            min_indent = indent;
                        }
                    }
                    let mut cleaned_doc = String::new();
                    for line in &lines {
                        if line.trim().is_empty() {
                            cleaned_doc.push('\n');
                        } else {
                            let indent = if min_indent == usize::MAX {
                                0
                            } else {
                                min_indent
                            };
                            let slice_start = std::cmp::min(indent, line.len());
                            cleaned_doc.push_str(&line[slice_start..]);
                            cleaned_doc.push('\n');
                        }
                    }
                    docs.push(cleaned_doc.trim().to_string());
                }
            } else if ann.name == "Test" {
                self.insert_hover_info(
                    ann.name_span.clone(),
                    "```flame\n@Test\n```\n\nMarks a function as a unit test. It will be executed by the test runner.".to_string()
                );
                docs.push("**@Test Function**\nThis function is a unit test case.".to_string());
            } else if ann.name == "Requires" {
                self.insert_hover_info(
                    ann.name_span.clone(),
                    "```flame\n@Requires(...modules: String)\n```\n\nSpecifies module dependencies required by this function.".to_string()
                );
                docs.push(format!(
                    "**Requires Dependencies:** `{}`",
                    ann.args.join(", ")
                ));
            } else if ann.name == "Permission" {
                self.insert_hover_info(
                    ann.name_span.clone(),
                    "```flame\n@Permission(...permissions: String)\n```\n\nRequests specific runtime permissions for this function.".to_string()
                );
                docs.push(format!(
                    "**Required Permissions:** `{}`",
                    ann.args.join(", ")
                ));
            } else if ann.name == "Suggestions" {
                self.insert_hover_info(
                    ann.name_span.clone(),
                    "```flame\n@Suggestions([{name: String, kind: String}])\n```\n\nProvides custom suggestions for IDE autocompletion when typing the package name.".to_string()
                );
            } else if ann.name == "Command" {
                self.insert_hover_info(
                    ann.name_span.clone(),
                    "```flame\n@Command(name: String, about: String)\n```\n\nDeclares a CLI subcommand handled by this function.".to_string()
                );
                let mut about = None;
                for (idx, arg) in ann.args.iter().enumerate() {
                    let trimmed = arg.trim();
                    if trimmed.starts_with("about:") || trimmed.starts_with("about=") || trimmed.starts_with("about :") || trimmed.starts_with("about =") {
                        let val = if let Some((_, v)) = trimmed.split_once(':') { v } else if let Some((_, v)) = trimmed.split_once('=') { v } else { trimmed };
                        about = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
                    } else if trimmed.starts_with("description:") || trimmed.starts_with("description=") {
                        let val = if let Some((_, v)) = trimmed.split_once(':') { v } else if let Some((_, v)) = trimmed.split_once('=') { v } else { trimmed };
                        about = Some(val.trim().trim_matches('"').trim_matches('\'').to_string());
                    } else if !trimmed.starts_with("name:") && !trimmed.starts_with("name=") && idx == 1 && about.is_none() {
                        about = Some(trimmed.trim_matches('"').trim_matches('\'').to_string());
                    }
                }
                if let Some(ab) = about {
                    docs.push(ab);
                }
            } else if let Some(func) = self.functions.get(&ann.name).cloned() {
                if let Some(doc) = func.hover_doc {
                    self.insert_hover_info(ann.name_span.clone(), doc);
                }
            }
        }
        if docs.is_empty() {
            None
        } else {
            Some(docs.join("\n\n---\n\n"))
        }
    }


}
