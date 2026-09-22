use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use crate::parser::{Expr, JsxChild, Stmt};

pub struct WebBuildResult {
    pub dist_dir: PathBuf,
    pub port: u16,
    pub title: String,
    pub routes: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct WebPage {
    pub path: String,
    pub title: String,
    pub layout: Option<String>,
    pub func_name: String,
    pub func_body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct WebComponent {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub is_layout: bool,
}

pub struct WebCompiler {
    pub pkg_name: String,
    pub app_title: String,
    pub port: u16,
    pub pages: Vec<WebPage>,
    pub components: HashMap<String, WebComponent>,
    pub state_vars: HashSet<String>,
    pub global_stmts: Vec<Stmt>,
    pub css_blocks: Vec<String>,
    pub default_layout: Option<String>,
    var_counter: usize,
}

impl WebCompiler {
    pub fn new(pkg_name: String) -> Self {
        Self {
            app_title: pkg_name.clone(),
            pkg_name,
            port: 3000,
            pages: Vec::new(),
            components: HashMap::new(),
            state_vars: HashSet::new(),
            global_stmts: Vec::new(),
            css_blocks: Vec::new(),
            default_layout: None,
            var_counter: 0,
        }
    }

    pub fn process_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.process_stmt(stmt);
        }
    }

    fn process_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::ExportDecl(inner, _) => {
                self.process_stmt(inner);
            }
            Stmt::LetDecl { name, annotations, .. } | Stmt::ConstDecl { name, annotations, .. } => {
                let is_state = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("state"));
                if is_state {
                    self.state_vars.insert(name.clone());
                }
                self.global_stmts.push(stmt.clone());
            }
            Stmt::FuncDecl { name, body, annotations, params, .. } => {
                let is_web = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("web"));
                let is_page = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("page"));
                let is_component = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("component"));
                let is_layout = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("layout"));
                let is_style = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("style"));

                if is_web {
                    for ann in annotations {
                        if ann.name.eq_ignore_ascii_case("web") {
                            for arg in &ann.args {
                                if let Some((k, v)) = arg.split_once(':') {
                                    let key = k.trim();
                                    let val = v.trim().trim_matches('"').trim_matches('\'');
                                    if key == "port" {
                                        if let Ok(p) = val.parse::<u16>() {
                                            self.port = p;
                                        }
                                    } else if key == "title" {
                                        self.app_title = val.to_string();
                                    }
                                } else if let Ok(p) = arg.trim().parse::<u16>() {
                                    self.port = p;
                                }
                            }
                        }
                    }
                    if let Some(body_stmts) = body {
                        for inner in body_stmts {
                            self.process_stmt(inner);
                        }
                    }
                } else if is_style {
                    // Extract CSS from annotation arguments or function body
                    for ann in annotations {
                        if ann.name.eq_ignore_ascii_case("style") {
                            for arg in &ann.args {
                                let trimmed = arg.trim().trim_matches('"').trim_matches('\'');
                                if trimmed.contains('{') || trimmed.contains(';') || trimmed.contains(':') {
                                    self.css_blocks.push(trimmed.to_string());
                                }
                            }
                        }
                    }
                    if let Some(body_stmts) = body {
                        for inner in body_stmts {
                            match inner {
                                Stmt::ExprStmt(Expr::Literal(crate::parser::LiteralValue::String(s), _)) => {
                                    self.css_blocks.push(s.clone());
                                }
                                Stmt::ReturnStmt(Some(Expr::Literal(crate::parser::LiteralValue::String(s), _)), _) => {
                                    self.css_blocks.push(s.clone());
                                }
                                _ => {}
                            }
                        }
                    }
                } else if is_page {
                    let mut path = "/".to_string();
                    let mut title = self.app_title.clone();
                    let mut layout = None;

                    for ann in annotations {
                        if ann.name.eq_ignore_ascii_case("page") {
                            for arg in &ann.args {
                                if let Some((k, v)) = arg.split_once(':') {
                                    let key = k.trim();
                                    let val = v.trim().trim_matches('"').trim_matches('\'');
                                    match key {
                                        "path" => path = val.to_string(),
                                        "title" => title = val.to_string(),
                                        "layout" => layout = Some(val.to_string()),
                                        _ => {}
                                    }
                                } else {
                                    let clean = arg.trim().trim_matches('"').trim_matches('\'');
                                    if clean.starts_with('/') {
                                        path = clean.to_string();
                                    } else if !clean.is_empty() {
                                        title = clean.to_string();
                                    }
                                }
                            }
                        } else if ann.name.eq_ignore_ascii_case("style") {
                            for arg in &ann.args {
                                let trimmed = arg.trim().trim_matches('"').trim_matches('\'');
                                if !trimmed.is_empty() {
                                    self.css_blocks.push(trimmed.to_string());
                                }
                            }
                        }
                    }

                    let body_stmts = body.clone().unwrap_or_default();
                    for inner in &body_stmts {
                        match inner {
                            Stmt::LetDecl { name, annotations, .. } | Stmt::ConstDecl { name, annotations, .. } => {
                                if annotations.iter().any(|a| a.name.eq_ignore_ascii_case("state")) {
                                    self.state_vars.insert(name.clone());
                                }
                            }
                            Stmt::FuncDecl { annotations, .. } => {
                                if annotations.iter().any(|a| a.name.eq_ignore_ascii_case("style")) {
                                    self.process_stmt(inner);
                                }
                            }
                            _ => {}
                        }
                    }
                    if layout.is_none() {
                        layout = self.default_layout.clone();
                    }
                    self.pages.push(WebPage {
                        path,
                        title,
                        layout,
                        func_name: name.clone(),
                        func_body: body_stmts.clone(),
                    });

                    // Also make page usable as a tag-like component
                    let comp_name = if let Some(first) = name.chars().next() {
                        let mut s = first.to_uppercase().to_string();
                        s.push_str(&name[first.len_utf8()..]);
                        s
                    } else {
                        name.clone()
                    };
                    self.components.insert(
                        comp_name.clone(),
                        WebComponent {
                            name: comp_name.clone(),
                            params: Vec::new(),
                            body: body_stmts.clone(),
                            is_layout: false,
                        },
                    );
                    if comp_name != *name {
                        self.components.insert(
                            name.clone(),
                            WebComponent {
                                name: name.clone(),
                                params: Vec::new(),
                                body: body_stmts,
                                is_layout: false,
                            },
                        );
                    }
                } else if is_component || is_layout {
                    if is_layout && self.default_layout.is_none() {
                        self.default_layout = Some(name.clone());
                    }
                    let param_names = params.iter().map(|p| p.name.clone()).collect();
                    let body_stmts = body.clone().unwrap_or_default();
                    for inner in &body_stmts {
                        match inner {
                            Stmt::LetDecl { name, annotations, .. } | Stmt::ConstDecl { name, annotations, .. } => {
                                if annotations.iter().any(|a| a.name.eq_ignore_ascii_case("state")) {
                                    self.state_vars.insert(name.clone());
                                }
                            }
                            Stmt::FuncDecl { annotations, .. } => {
                                if annotations.iter().any(|a| a.name.eq_ignore_ascii_case("style")) {
                                    self.process_stmt(inner);
                                }
                            }
                            _ => {}
                        }
                    }
                    self.components.insert(
                        name.clone(),
                        WebComponent {
                            name: name.clone(),
                            params: param_names,
                            body: body_stmts,
                            is_layout,
                        },
                    );
                } else {
                    self.global_stmts.push(stmt.clone());
                }
            }
            _ => {
                self.global_stmts.push(stmt.clone());
            }
        }
    }

    pub fn compile_to_js(&mut self) -> String {
        let mut js = String::new();

        js.push_str("// Flame Fine-Grained Reactive Web Runtime (Auto-Generated)\n");
        js.push_str("\"use strict\";\n\n");

        // Runtime reactivity primitives
        js.push_str(
            r#"const _signals = new Map();
const _subscribers = new Map();

function _createSignal(name, initialValue) {
  _signals.set(name, initialValue);
  _subscribers.set(name, new Set());
}

function _getSignal(name) {
  return _signals.get(name);
}

function _setSignal(name, nextValue) {
  if (typeof nextValue === 'function') {
    nextValue = nextValue(_signals.get(name));
  }
  _signals.set(name, nextValue);
  const subs = _subscribers.get(name);
  if (subs) {
    for (const sub of subs) {
      sub(nextValue);
    }
  }
}

function _subscribe(name, callback) {
  const subs = _subscribers.get(name);
  if (subs) {
    subs.add(callback);
  }
}

function navigate(url) {
  window.history.pushState({}, "", url);
  _renderActiveRoute();
}
window.navigate = navigate;
window.addEventListener("popstate", () => _renderActiveRoute());

document.addEventListener("click", (e) => {
  const link = e.target.closest("a");
  if (link) {
    const href = link.getAttribute("href");
    if (href && href.startsWith("/") && !link.getAttribute("target") && !link.hasAttribute("download")) {
      e.preventDefault();
      navigate(href);
    }
  }
});

const http = {
  get: async (url) => {
    const res = await fetch(url);
    const textData = await res.text();
    let jsonData = null;
    try {
      jsonData = JSON.parse(textData);
    } catch (_) {}
    const resultData = jsonData !== null ? jsonData : textData;
    return {
      status: res.status,
      ok: res.ok,
      data: resultData,
      text: () => textData,
      json: () => jsonData,
      then: (resolve, reject) => Promise.resolve(resultData).then(resolve, reject),
    };
  },
  post: async (url, body) => {
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const textData = await res.text();
    let jsonData = null;
    try {
      jsonData = JSON.parse(textData);
    } catch (_) {}
    const resultData = jsonData !== null ? jsonData : textData;
    return {
      status: res.status,
      ok: res.ok,
      data: resultData,
      text: () => textData,
      json: () => jsonData,
      then: (resolve, reject) => Promise.resolve(resultData).then(resolve, reject),
    };
  }
};
window.http = http;

const web = {
  document: typeof document !== 'undefined' ? document : null,
  window: typeof window !== 'undefined' ? window : null,
  navigate: navigate,
  http: http,
  fetch: (url, opts) => fetch(url, opts),
};
window.web = web;

"#,
        );

        let global_stmts = self.global_stmts.clone();

        // State declarations
        let state_vars: Vec<String> = self.state_vars.iter().cloned().collect();
        let pages = self.pages.clone();
        let components: Vec<_> = self.components.values().cloned().collect();
        for state_var in &state_vars {
            let mut val_expr = None;
            'found_init: for stmt in &global_stmts {
                if let Stmt::LetDecl { name, value, .. } | Stmt::ConstDecl { name, value, .. } = stmt {
                    if name == state_var {
                        val_expr = Some(value.clone());
                        break 'found_init;
                    }
                }
            }
            if val_expr.is_none() {
                'found_page: for page in &pages {
                    for stmt in &page.func_body {
                        if let Stmt::LetDecl { name, value, .. } | Stmt::ConstDecl { name, value, .. } = stmt {
                            if name == state_var {
                                val_expr = Some(value.clone());
                                break 'found_page;
                            }
                        }
                    }
                }
            }
            if val_expr.is_none() {
                'found_comp: for comp in &components {
                    for stmt in &comp.body {
                        if let Stmt::LetDecl { name, value, .. } | Stmt::ConstDecl { name, value, .. } = stmt {
                            if name == state_var {
                                val_expr = Some(value.clone());
                                break 'found_comp;
                            }
                        }
                    }
                }
            }
            let init_val_js = if let Some(ve) = &val_expr {
                self.expr_to_js(ve)
            } else {
                "0".to_string()
            };
            js.push_str(&format!("_createSignal(\"{}\", {});\n", state_var, init_val_js));
        }

        // Global functions & non-state let bindings
        for stmt in &global_stmts {
            match stmt {
                Stmt::LetDecl { name, value, annotations, .. }
                | Stmt::ConstDecl { name, value, annotations, .. } => {
                    let is_state = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("state")) || self.state_vars.contains(name);
                    if !is_state {
                        let val_js = self.expr_to_js(value);
                        js.push_str(&format!("let {} = {};\n", name, val_js));
                    }
                }
                Stmt::FuncDecl { name, params, body, annotations, .. } => {
                    let is_web = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("web"));
                    let is_page = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("page"));
                    let is_comp = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("component"));
                    let is_style = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("style"));
                    if !is_web && !is_page && !is_comp && !is_style {
                        let is_async = Self::body_has_await(body.as_deref().unwrap_or(&[]));
                        let async_prefix = if is_async { "async " } else { "" };
                        let param_str = params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ");
                        js.push_str(&format!("\n{}function {}({}) {{\n", async_prefix, name, param_str));
                        if let Some(body_stmts) = body {
                            for b in body_stmts {
                                self.compile_stmt(b, &mut js, 1);
                            }
                        }
                        js.push_str("}\n");
                    }
                }
                Stmt::ExprStmt(expr) => {
                    let expr_js = self.expr_to_js(expr);
                    js.push_str(&format!("{};\n", expr_js));
                }
                _ => {}
            }
        }

        // Reusable components
        let components = self.components.clone();
        for (comp_name, comp) in &components {
            let is_async = Self::body_has_await(&comp.body);
            let async_prefix = if is_async { "async " } else { "" };
            let param_str = comp.params.join(", ");
            js.push_str(&format!("\n{}function {}({}) {{\n", async_prefix, comp_name, param_str));
            let root_var = format!("_root_{}", comp_name.to_lowercase());
            let mut returned_elem = false;

            for stmt in &comp.body {
                match stmt {
                    Stmt::ExprStmt(expr) => {
                        if matches!(expr, Expr::JsxElement { .. }) {
                            let el = self.compile_jsx(expr, None, &mut js, 1);
                            js.push_str(&format!("  return {};\n", el));
                            returned_elem = true;
                        } else {
                            self.compile_stmt(stmt, &mut js, 1);
                        }
                    }
                    Stmt::ReturnStmt(Some(expr), _) => {
                        if matches!(expr, Expr::JsxElement { .. }) {
                            let el = self.compile_jsx(expr, None, &mut js, 1);
                            js.push_str(&format!("  return {};\n", el));
                            returned_elem = true;
                        } else {
                            self.compile_stmt(stmt, &mut js, 1);
                        }
                    }
                    _ => {
                        self.compile_stmt(stmt, &mut js, 1);
                    }
                }
            }

            if !returned_elem {
                js.push_str(&format!("  const {} = document.createElement(\"div\");\n", root_var));
                js.push_str(&format!("  return {};\n", root_var));
            }
            js.push_str("}\n");
        }

        // Page render functions
        let pages = self.pages.clone();
        for page in &pages {
            let is_async = Self::body_has_await(&page.func_body);
            let async_prefix = if is_async { "async " } else { "" };
            let func_name = format!("_render_page_{}", page.func_name);
            js.push_str(&format!("\n{}function {}() {{\n", async_prefix, func_name));
            if !page.title.is_empty() {
                js.push_str(&format!("  document.title = \"{}\";\n", page.title.replace('"', "\\\"")));
            }
            let mut returned = false;
            for stmt in &page.func_body {
                match stmt {
                    Stmt::ExprStmt(expr) | Stmt::ReturnStmt(Some(expr), _) => {
                        if matches!(expr, Expr::JsxElement { .. }) {
                            let el = self.compile_jsx(expr, None, &mut js, 1);
                            if let Some(layout_name) = &page.layout {
                                js.push_str(&format!("  return {}({});\n", layout_name, el));
                            } else {
                                js.push_str(&format!("  return {};\n", el));
                            }
                            returned = true;
                        } else {
                            self.compile_stmt(stmt, &mut js, 1);
                        }
                    }
                    _ => {
                        self.compile_stmt(stmt, &mut js, 1);
                    }
                }
            }
            if !returned {
                js.push_str("  return document.createElement(\"div\");\n");
            }
            js.push_str("}\n");
        }

        // Router & App Mount
        js.push_str("\n// Client-Side Router\n");
        js.push_str("const _routes = [\n");
        for page in &self.pages {
            let render_fn = format!("_render_page_{}", page.func_name);
            js.push_str(&format!("  {{ path: \"{}\", render: {} }},\n", page.path, render_fn));
        }
        js.push_str("];\n\n");

        js.push_str(
            r#"function _renderActiveRoute() {
  const currentPath = window.location.pathname || "/";
  const appRoot = document.getElementById("app");
  if (!appRoot) return;

  let match = _routes.find(r => r.path === currentPath);
  if (!match) {
    match = _routes.find(r => r.path === "/");
  }
  if (!match && _routes.length > 0) {
    match = _routes[0];
  }

  appRoot.innerHTML = "";
  if (match) {
    const el = match.render();
    if (el) {
      appRoot.appendChild(el);
    }
  }
}

// Initial mount on DOM load
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", _renderActiveRoute);
} else {
  _renderActiveRoute();
}
"#,
        );

        js
    }

    pub fn body_has_await(stmts: &[Stmt]) -> bool {
        stmts.iter().any(Self::stmt_has_await)
    }

    fn stmt_has_await(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::ExprStmt(e) => Self::expr_has_await(e),
            Stmt::LetDecl { value, .. } | Stmt::ConstDecl { value, .. } => Self::expr_has_await(value),
            Stmt::ReturnStmt(opt, _) => opt.as_ref().map_or(false, Self::expr_has_await),
            Stmt::IfStmt { cond, then_branch, else_branch, .. } => {
                Self::expr_has_await(cond)
                    || then_branch.iter().any(Self::stmt_has_await)
                    || else_branch.as_ref().map_or(false, |b| b.iter().any(Self::stmt_has_await))
            }
            Stmt::WhileStmt { cond, body, .. } => {
                Self::expr_has_await(cond) || body.iter().any(Self::stmt_has_await)
            }
            Stmt::ForStmt { iterable, body, .. } => {
                Self::expr_has_await(iterable) || body.iter().any(Self::stmt_has_await)
            }
            Stmt::LoopStmt { body, .. } => body.iter().any(Self::stmt_has_await),
            Stmt::ExportDecl(inner, _) => Self::stmt_has_await(inner),
            _ => false,
        }
    }

    fn expr_has_await(expr: &Expr) -> bool {
        match expr {
            Expr::Await(..) => true,
            Expr::Binary(l, _, r, _) => Self::expr_has_await(l) || Self::expr_has_await(r),
            Expr::Unary(_, e, _) => Self::expr_has_await(e),
            Expr::Call(callee, args, _) => {
                Self::expr_has_await(callee) || args.iter().any(|(_, e)| Self::expr_has_await(e))
            }
            Expr::Dot(obj, _, _) | Expr::SafeDot(obj, _, _) => Self::expr_has_await(obj),
            Expr::Index(obj, idx, _) => {
                Self::expr_has_await(obj) || Self::expr_has_await(idx)
            }
            Expr::VectorLiteral(elements, _) | Expr::Tuple(elements, _) => {
                elements.iter().any(Self::expr_has_await)
            }
            Expr::Block(stmts, _) => stmts.iter().any(Self::stmt_has_await),
            Expr::Closure { body, .. } => body.iter().any(Self::stmt_has_await),
            _ => false,
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        match stmt {
            Stmt::ExportDecl(inner, _) => {
                self.compile_stmt(inner, out, indent);
            }
            Stmt::LetDecl { name, value, annotations, .. } | Stmt::ConstDecl { name, value, annotations, .. } => {
                let is_state = annotations.iter().any(|a| a.name.eq_ignore_ascii_case("state")) || self.state_vars.contains(name);
                if !is_state {
                    let val_js = self.expr_to_js(value);
                    out.push_str(&format!("{pad}let {} = {};\n", name, val_js));
                }
            }
            Stmt::ExprStmt(expr) => {
                let expr_js = self.expr_to_js(expr);
                out.push_str(&format!("{pad}{};\n", expr_js));
            }
            Stmt::ReturnStmt(expr_opt, _) => {
                if let Some(expr) = expr_opt {
                    let val_js = self.expr_to_js(expr);
                    out.push_str(&format!("{pad}return {};\n", val_js));
                } else {
                    out.push_str(&format!("{pad}return;\n"));
                }
            }
            Stmt::IfStmt { cond, then_branch, else_branch, .. } => {
                let cond_js = self.expr_to_js(cond);
                out.push_str(&format!("{pad}if ({}) {{\n", cond_js));
                for s in then_branch {
                    self.compile_stmt(s, out, indent + 1);
                }
                if let Some(else_stmts) = else_branch {
                    out.push_str(&format!("{pad}}} else {{\n"));
                    for s in else_stmts {
                        self.compile_stmt(s, out, indent + 1);
                    }
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::WhileStmt { cond, body, .. } => {
                let cond_js = self.expr_to_js(cond);
                out.push_str(&format!("{pad}while ({}) {{\n", cond_js));
                for s in body {
                    self.compile_stmt(s, out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::ForStmt { var_name, iterable, body, .. } => {
                let iter_js = self.expr_to_js(iterable);
                out.push_str(&format!("{pad}for (const {} of {}) {{\n", var_name, iter_js));
                for s in body {
                    self.compile_stmt(s, out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            _ => {}
        }
    }

    fn compile_jsx(
        &mut self,
        expr: &Expr,
        parent_var: Option<&str>,
        out: &mut String,
        indent: usize,
    ) -> String {
        let pad = "  ".repeat(indent);
        match expr {
            Expr::JsxElement { tag, attributes, children, .. } => {
                // If tag starts with uppercase, treat as custom component call
                let is_custom_component = tag.chars().next().map_or(false, |c| c.is_uppercase());
                if is_custom_component {
                    let el_var = format!("_comp{}", self.var_counter);
                    self.var_counter += 1;

                    let mut prop_entries = Vec::new();
                    for attr in attributes {
                        let val_js = if let Some(val_expr) = &attr.value {
                            self.expr_to_js(val_expr)
                        } else {
                            "true".to_string()
                        };
                        prop_entries.push(format!("\"{}\": {}", attr.name, val_js));
                    }
                    let props_obj = format!("{{{}}}", prop_entries.join(", "));
                    out.push_str(&format!(
                        "{pad}const {} = {}({});\n",
                        el_var, tag, props_obj
                    ));
                    if let Some(pv) = parent_var {
                        out.push_str(&format!("{pad}{}.appendChild({});\n", pv, el_var));
                    }
                    return el_var;
                }

                let el_var = format!("_el{}", self.var_counter);
                self.var_counter += 1;

                out.push_str(&format!(
                    "{pad}const {} = document.createElement(\"{}\");\n",
                    el_var, tag
                ));

                for attr in attributes {
                    let attr_name = &attr.name;
                    if attr_name.starts_with("on") {
                        let evt_name = attr_name.trim_start_matches("on").to_ascii_lowercase();
                        if let Some(val_expr) = &attr.value {
                            match val_expr {
                                Expr::Block(stmts, _) => {
                                    let is_async = Self::body_has_await(stmts);
                                    let async_prefix = if is_async { "async " } else { "" };
                                    let mut body_str = String::new();
                                    for s in stmts {
                                        self.compile_stmt(s, &mut body_str, indent + 1);
                                    }
                                    out.push_str(&format!(
                                        "{pad}{}.addEventListener(\"{}\", {}(event) => {{\n{}{pad}}});\n",
                                        el_var, evt_name, async_prefix, body_str
                                    ));
                                }
                                Expr::Closure { params, body, .. } => {
                                    let is_async = Self::body_has_await(body);
                                    let async_prefix = if is_async { "async " } else { "" };
                                    let param_str = if params.is_empty() {
                                        "event".to_string()
                                    } else {
                                        params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ")
                                    };
                                    let mut body_str = String::new();
                                    for s in body {
                                        self.compile_stmt(s, &mut body_str, indent + 1);
                                    }
                                    out.push_str(&format!(
                                        "{pad}{}.addEventListener(\"{}\", {}({}) => {{\n{}{pad}}});\n",
                                        el_var, evt_name, async_prefix, param_str, body_str
                                    ));
                                }
                                Expr::Identifier(fn_name, _) if !self.state_vars.contains(fn_name) => {
                                    out.push_str(&format!(
                                        "{pad}{}.addEventListener(\"{}\", (event) => {{ {}(event); }});\n",
                                        el_var, evt_name, fn_name
                                    ));
                                }
                                _ => {
                                    let handler_js = self.expr_to_js(val_expr);
                                    out.push_str(&format!(
                                        "{pad}{}.addEventListener(\"{}\", (event) => {{ ({}); }});\n",
                                        el_var, evt_name, handler_js
                                    ));
                                }
                            }
                        }
                    } else if let Some(val_expr) = &attr.value {
                        let refs = self.find_referenced_states(val_expr);
                        if !refs.is_empty() {
                            let fn_var = format!("_update_attr_{}", self.var_counter);
                            self.var_counter += 1;
                            let val_js = self.expr_to_js(val_expr);
                            if attr.name == "class" {
                                out.push_str(&format!(
                                    "{pad}const {} = () => {{ {}.className = {}; }};\n",
                                    fn_var, el_var, val_js
                                ));
                            } else if attr.name == "style" {
                                out.push_str(&format!(
                                    "{pad}const {} = () => {{ {}.style.cssText = {}; }};\n",
                                    fn_var, el_var, val_js
                                ));
                            } else {
                                out.push_str(&format!(
                                    "{pad}const {} = () => {{ {}.setAttribute(\"{}\", {}); }};\n",
                                    fn_var, el_var, attr.name, val_js
                                ));
                            }
                            for s in &refs {
                                out.push_str(&format!("{pad}_subscribe(\"{}\", {});\n", s, fn_var));
                            }
                            out.push_str(&format!("{pad}{}();\n", fn_var));
                        } else {
                            let val_js = self.expr_to_js(val_expr);
                            if attr.name == "class" {
                                out.push_str(&format!("{pad}{}.className = {};\n", el_var, val_js));
                            } else if attr.name == "style" {
                                out.push_str(&format!("{pad}{}.style.cssText = {};\n", el_var, val_js));
                            } else {
                                out.push_str(&format!(
                                    "{pad}{}.setAttribute(\"{}\", {});\n",
                                    el_var, attr.name, val_js
                                ));
                            }
                        }
                    } else {
                        out.push_str(&format!(
                            "{pad}{}.setAttribute(\"{}\", \"\");\n",
                            el_var, attr.name
                        ));
                    }
                }

                for child in children {
                    match child {
                        JsxChild::Element(child_expr) => {
                            self.compile_jsx(child_expr, Some(&el_var), out, indent);
                        }
                        JsxChild::Text(txt, _) => {
                            let t_var = format!("_t{}", self.var_counter);
                            self.var_counter += 1;
                            let clean_txt = txt.replace('"', "\\\"").replace('\n', " ");
                            out.push_str(&format!(
                                "{pad}const {} = document.createTextNode(\"{}\");\n",
                                t_var, clean_txt
                            ));
                            out.push_str(&format!("{pad}{}.appendChild({});\n", el_var, t_var));
                        }
                        JsxChild::Expr(child_expr) => {
                            let t_var = format!("_t{}", self.var_counter);
                            self.var_counter += 1;
                            let refs = self.find_referenced_states(child_expr);
                            let expr_js = self.expr_to_js(child_expr);

                            if !refs.is_empty() {
                                let update_fn = format!("_update_{}", t_var);
                                out.push_str(&format!(
                                    "{pad}const {} = document.createTextNode(\"\");\n",
                                    t_var
                                ));
                                out.push_str(&format!(
                                    "{pad}const {} = () => {{ {}.textContent = String({}); }};\n",
                                    update_fn, t_var, expr_js
                                ));
                                for s in &refs {
                                    out.push_str(&format!(
                                        "{pad}_subscribe(\"{}\", {});\n",
                                        s, update_fn
                                    ));
                                }
                                out.push_str(&format!("{pad}{}();\n", update_fn));
                                out.push_str(&format!("{pad}{}.appendChild({});\n", el_var, t_var));
                            } else {
                                let val_temp = format!("_val{}", self.var_counter);
                                self.var_counter += 1;
                                out.push_str(&format!("{pad}const {} = {};\n", val_temp, expr_js));
                                out.push_str(&format!(
                                    "{pad}if (typeof {val} === 'object' && {val} instanceof Node) {{\n{pad}  {el}.appendChild({val});\n{pad}}} else if (Array.isArray({val})) {{\n{pad}  for (const _item of {val}) {{\n{pad}    if (typeof _item === 'object' && _item instanceof Node) {{\n{pad}      {el}.appendChild(_item);\n{pad}    }} else {{\n{pad}      {el}.appendChild(document.createTextNode(String(_item)));\n{pad}    }}\n{pad}  }}\n{pad}}} else {{\n{pad}  {el}.appendChild(document.createTextNode(String({val})));\n{pad}}}\n",
                                    val = val_temp,
                                    el = el_var,
                                ));
                            }
                        }
                    }
                }

                if let Some(p) = parent_var {
                    out.push_str(&format!("{pad}{}.appendChild({});\n", p, el_var));
                }
                el_var
            }
            _ => {
                let val_js = self.expr_to_js(expr);
                let t_var = format!("_t{}", self.var_counter);
                self.var_counter += 1;
                out.push_str(&format!(
                    "{pad}const {} = document.createTextNode(String({}));\n",
                    t_var, val_js
                ));
                if let Some(p) = parent_var {
                    out.push_str(&format!("{pad}{}.appendChild({});\n", p, t_var));
                }
                t_var
            }
        }
    }

    fn find_referenced_states(&self, expr: &Expr) -> HashSet<String> {
        let mut set = HashSet::new();
        self.collect_states(expr, &mut set);
        set
    }

    fn collect_states(&self, expr: &Expr, set: &mut HashSet<String>) {
        match expr {
            Expr::Identifier(id, _) => {
                if self.state_vars.contains(id) {
                    set.insert(id.clone());
                }
            }
            Expr::Binary(left, _, right, _) => {
                self.collect_states(left, set);
                self.collect_states(right, set);
            }
            Expr::Unary(_, inner, _) => {
                self.collect_states(inner, set);
            }
            Expr::Call(callee, args, _) => {
                self.collect_states(callee, set);
                for (_, arg) in args {
                    self.collect_states(arg, set);
                }
            }
            Expr::Dot(inner, _, _) | Expr::SafeDot(inner, _, _) => {
                self.collect_states(inner, set);
            }
            Expr::Index(inner, idx, _) => {
                self.collect_states(inner, set);
                self.collect_states(idx, set);
            }
            _ => {}
        }
    }

    pub fn expr_to_js(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit, _) => match lit {
                crate::parser::LiteralValue::Int(i) => i.to_string(),
                crate::parser::LiteralValue::Float(f) => f.to_string(),
                crate::parser::LiteralValue::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
                crate::parser::LiteralValue::Bool(b) => b.to_string(),
                crate::parser::LiteralValue::Nil => "null".to_string(),
            },
            Expr::Identifier(id, _) => {
                if self.state_vars.contains(id) {
                    format!("_getSignal(\"{}\")", id)
                } else {
                    id.clone()
                }
            }
            Expr::Unary(op, inner, _) => {
                let in_js = self.expr_to_js(inner);
                match op {
                    crate::parser::UnaryOp::Not => format!("!({})", in_js),
                    crate::parser::UnaryOp::Neg => format!("-({})", in_js),
                    crate::parser::UnaryOp::PreInc => {
                        if let Expr::Identifier(id, _) = &**inner {
                            if self.state_vars.contains(id) {
                                return format!("_setSignal(\"{}\", s => s + 1)", id);
                            }
                        }
                        format!("++{}", in_js)
                    }
                    crate::parser::UnaryOp::PreDec => {
                        if let Expr::Identifier(id, _) = &**inner {
                            if self.state_vars.contains(id) {
                                return format!("_setSignal(\"{}\", s => s - 1)", id);
                            }
                        }
                        format!("--{}", in_js)
                    }
                    crate::parser::UnaryOp::PostInc => {
                        if let Expr::Identifier(id, _) = &**inner {
                            if self.state_vars.contains(id) {
                                return format!("_setSignal(\"{}\", s => s + 1)", id);
                            }
                        }
                        format!("{}++", in_js)
                    }
                    crate::parser::UnaryOp::PostDec => {
                        if let Expr::Identifier(id, _) = &**inner {
                            if self.state_vars.contains(id) {
                                return format!("_setSignal(\"{}\", s => s - 1)", id);
                            }
                        }
                        format!("{}--", in_js)
                    }
                    crate::parser::UnaryOp::NonNullAssert => in_js,
                }
            }
            Expr::Binary(left, op, right, _) => {
                if let Expr::Identifier(id, _) = &**left {
                    if self.state_vars.contains(id) {
                        let r_js = self.expr_to_js(right);
                        match op {
                            crate::parser::BinaryOp::Assign => return format!("_setSignal(\"{}\", {})", id, r_js),
                            crate::parser::BinaryOp::PlusAssign => return format!("_setSignal(\"{}\", s => s + ({}))", id, r_js),
                            crate::parser::BinaryOp::MinusAssign => return format!("_setSignal(\"{}\", s => s - ({}))", id, r_js),
                            crate::parser::BinaryOp::MulAssign => return format!("_setSignal(\"{}\", s => s * ({}))", id, r_js),
                            crate::parser::BinaryOp::DivAssign => return format!("_setSignal(\"{}\", s => s / ({}))", id, r_js),
                            crate::parser::BinaryOp::ModAssign => return format!("_setSignal(\"{}\", s => s % ({}))", id, r_js),
                            _ => {}
                        }
                    }
                }

                let l_js = self.expr_to_js(left);
                let r_js = self.expr_to_js(right);
                let op_str = match op {
                    crate::parser::BinaryOp::Add => "+",
                    crate::parser::BinaryOp::Sub => "-",
                    crate::parser::BinaryOp::Mul => "*",
                    crate::parser::BinaryOp::Div => "/",
                    crate::parser::BinaryOp::Mod => "%",
                    crate::parser::BinaryOp::Assign => "=",
                    crate::parser::BinaryOp::PlusAssign => "+=",
                    crate::parser::BinaryOp::MinusAssign => "-=",
                    crate::parser::BinaryOp::MulAssign => "*=",
                    crate::parser::BinaryOp::DivAssign => "/=",
                    crate::parser::BinaryOp::ModAssign => "%=",
                    crate::parser::BinaryOp::BitAndAssign => "&=",
                    crate::parser::BinaryOp::BitOrAssign => "|=",
                    crate::parser::BinaryOp::BitXorAssign => "^=",
                    crate::parser::BinaryOp::ShlAssign => "<<=",
                    crate::parser::BinaryOp::ShrAssign => ">>=",
                    crate::parser::BinaryOp::Eq => "===",
                    crate::parser::BinaryOp::Ne => "!==",
                    crate::parser::BinaryOp::Lt => "<",
                    crate::parser::BinaryOp::Le => "<=",
                    crate::parser::BinaryOp::Gt => ">",
                    crate::parser::BinaryOp::Ge => ">=",
                    crate::parser::BinaryOp::And => "&&",
                    crate::parser::BinaryOp::Or => "||",
                    crate::parser::BinaryOp::BitAnd => "&",
                    crate::parser::BinaryOp::BitOr => "|",
                    crate::parser::BinaryOp::BitXor => "^",
                    crate::parser::BinaryOp::Shl => "<<",
                    crate::parser::BinaryOp::Shr => ">>",
                    crate::parser::BinaryOp::NilCoalesce => "??",
                    crate::parser::BinaryOp::Range => "..",
                };
                format!("({} {} {})", l_js, op_str, r_js)
            }
            Expr::Call(callee, args, _) => {
                let callee_js = self.expr_to_js(callee);
                let args_js = args
                    .iter()
                    .map(|(_, a)| self.expr_to_js(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", callee_js, args_js)
            }
            Expr::Dot(inner, member, _) => {
                let in_js = self.expr_to_js(inner);
                format!("{}.{}", in_js, member)
            }
            Expr::SafeDot(inner, member, _) => {
                let in_js = self.expr_to_js(inner);
                format!("{}?.{}", in_js, member)
            }
            Expr::Index(inner, idx, _) => {
                let in_js = self.expr_to_js(inner);
                let idx_js = self.expr_to_js(idx);
                format!("{}[{}]", in_js, idx_js)
            }
            Expr::VectorLiteral(items, _) => {
                let items_js = items
                    .iter()
                    .map(|i| self.expr_to_js(i))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", items_js)
            }
            Expr::Await(inner, _) => {
                let in_js = self.expr_to_js(inner);
                format!("await {}", in_js)
            }
            Expr::Closure { params, body, .. } => {
                let is_async = Self::body_has_await(body);
                let async_prefix = if is_async { "async " } else { "" };
                let param_str = params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ");
                let mut body_str = String::new();
                for s in body {
                    self.compile_stmt(s, &mut body_str, 2);
                }
                format!("{}({}) => {{\n{}}}", async_prefix, param_str, body_str)
            }
            Expr::Block(stmts, _) => {
                let is_async = Self::body_has_await(stmts);
                let async_prefix = if is_async { "async " } else { "" };
                let mut body_str = String::new();
                for s in stmts {
                    self.compile_stmt(s, &mut body_str, 1);
                }
                format!("({}() => {{\n{}}})()", async_prefix, body_str)
            }
            _ => "null".to_string(),
        }
    }
}

fn collect_fm_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && name != "dist" && name != "target" && name != "node_modules" {
                    collect_fm_files(&p, files);
                }
            } else if p.is_file() && p.extension().map_or(false, |ext| ext == "fm" || ext == "flame") {
                files.push(p);
            }
        }
    }
}

pub fn build_web_project(project_path: &Path) -> Result<WebBuildResult, String> {
    let (project_root, single_file) = if project_path.is_file() {
        let parent = project_path.parent().unwrap_or(Path::new("."));
        let root = if parent.ends_with("src") {
            parent.parent().unwrap_or(parent)
        } else {
            parent
        };
        (root.to_path_buf(), Some(project_path.to_path_buf()))
    } else {
        (project_path.to_path_buf(), None)
    };

    let dist_dir = project_root.join("dist");
    if !dist_dir.exists() {
        fs::create_dir_all(&dist_dir).map_err(|e| e.to_string())?;
    }

    let manifest_path = project_root.join("flame.toml");
    let mut pkg_name = "Flame Web App".to_string();
    if manifest_path.exists() {
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            for line in content.lines() {
                let trim = line.trim();
                if trim.starts_with("name") {
                    if let Some((_, v)) = trim.split_once('=') {
                        pkg_name = v.trim().trim_matches('"').to_string();
                    }
                }
            }
        }
    }

    let src_dir = project_root.join("src");
    let mut raw_files = Vec::new();
    if src_dir.exists() {
        collect_fm_files(&src_dir, &mut raw_files);
    } else {
        collect_fm_files(&project_root, &mut raw_files);
    }

    let mut ordered_files = Vec::new();
    let mut seen = HashSet::new();

    if let Some(ref sf) = single_file {
        if sf.exists() {
            seen.insert(sf.clone());
            ordered_files.push(sf.clone());
        }
    }

    let main_candidate = src_dir.join("main.fm");
    if main_candidate.exists() && seen.insert(main_candidate.clone()) {
        ordered_files.push(main_candidate);
    }

    raw_files.sort();
    for f in raw_files {
        if seen.insert(f.clone()) {
            ordered_files.push(f);
        }
    }

    let mut all_stmts = Vec::new();
    for p in &ordered_files {
        if let Ok(content) = fs::read_to_string(p) {
            let mut lexer = crate::lexer::Lexer::new(&content);
            let mut tokens = Vec::new();
            loop {
                let tok = lexer.next_token();
                let is_eof = tok.kind == crate::lexer::TokenKind::EOF;
                tokens.push(tok);
                if is_eof {
                    break;
                }
            }
            let mut parser = crate::parser::Parser::new(tokens, p.to_string_lossy().to_string());
            if let Ok(stmts) = parser.parse() {
                all_stmts.extend(stmts);
            }
        }
    }

    let mut compiler = WebCompiler::new(pkg_name.clone());
    compiler.process_stmts(&all_stmts);
    let port = compiler.port;
    let app_title = compiler.app_title.clone();
    let routes: Vec<(String, String)> = compiler
        .pages
        .iter()
        .map(|p| (p.path.clone(), p.title.clone()))
        .collect();

    let js_code = compiler.compile_to_js();

    // Write dist/app.js
    let app_js_path = dist_dir.join("app.js");
    fs::write(&app_js_path, js_code).map_err(|e| e.to_string())?;

    // Write dist/index.html
    let html_content = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{}</title>
  <link rel="stylesheet" href="app.css">
</head>
<body>
  <div id="app"></div>
  <script type="module" src="app.js"></script>
</body>
</html>
"#,
        app_title
    );
    let index_html_path = dist_dir.join("index.html");
    fs::write(&index_html_path, html_content).map_err(|e| e.to_string())?;

    // app.css: Preserve anything the user manually added or removed.
    // On rebuild don't clean it, and if user added styles via @Style in Flame code, keep that as it is in css.
    let app_css_path = dist_dir.join("app.css");
    let mut css_content = if app_css_path.exists() {
        fs::read_to_string(&app_css_path).unwrap_or_default()
    } else {
        String::new()
    };

    for custom_css in &compiler.css_blocks {
        let trimmed = custom_css.trim();
        if !trimmed.is_empty() && !css_content.contains(trimmed) {
            if !css_content.is_empty() && !css_content.ends_with('\n') {
                css_content.push('\n');
            }
            css_content.push_str(trimmed);
            css_content.push('\n');
        }
    }

    if !app_css_path.exists() || !compiler.css_blocks.is_empty() {
        let _ = fs::write(&app_css_path, &css_content);
    }

    Ok(WebBuildResult {
        dist_dir,
        port,
        title: app_title,
        routes,
    })
}
