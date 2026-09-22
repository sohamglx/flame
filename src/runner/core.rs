#![allow(dead_code, unused_imports, unused_variables)]
use crate::lexer::{Lexer, Span};
use crate::parser::{
    BinaryOp, Expr, InterpolatedSegment, LiteralValue, Param, Parser, Stmt, UnaryOp,
};
use crate::vm::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Runner {
    pub env: Arc<Mutex<Env>>,
    pub filepath: PathBuf,
    pub modules: HashMap<String, Arc<Mutex<Env>>>,
    pub current_span: Option<Span>,
    pub native_methods: HashMap<String, fn(*const CValue, usize) -> CValue>,
    pub test_mode: bool,
    pub interactive: bool,
    pub granted_permissions: std::collections::HashSet<String>,
    pub vfs: Option<HashMap<String, String>>,
}



use std::sync::OnceLock;

static GLOBAL_NATIVE_METHODS: OnceLock<Mutex<HashMap<String, fn(*const CValue, usize) -> CValue>>> = OnceLock::new();
static GLOBAL_GRANTED_PERMISSIONS: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
static GLOBAL_VFS: OnceLock<Mutex<Option<HashMap<String, String>>>> = OnceLock::new();

pub fn set_global_native_methods(methods: HashMap<String, fn(*const CValue, usize) -> CValue>) {
    let mutex = GLOBAL_NATIVE_METHODS.get_or_init(|| Mutex::new(HashMap::new()));
    *mutex.lock().unwrap() = methods;
}

pub fn get_global_native_methods() -> HashMap<String, fn(*const CValue, usize) -> CValue> {
    GLOBAL_NATIVE_METHODS.get().map(|m| m.lock().unwrap().clone()).unwrap_or_default()
}

pub fn set_global_granted_permissions(perms: std::collections::HashSet<String>) {
    let mutex = GLOBAL_GRANTED_PERMISSIONS.get_or_init(|| Mutex::new(std::collections::HashSet::new()));
    *mutex.lock().unwrap() = perms;
}

pub fn get_global_granted_permissions() -> std::collections::HashSet<String> {
    GLOBAL_GRANTED_PERMISSIONS.get().map(|m| m.lock().unwrap().clone()).unwrap_or_default()
}

pub fn set_global_vfs(vfs: Option<HashMap<String, String>>) {
    let mutex = GLOBAL_VFS.get_or_init(|| Mutex::new(None));
    *mutex.lock().unwrap() = vfs;
}

pub fn get_global_vfs() -> Option<HashMap<String, String>> {
    GLOBAL_VFS.get().and_then(|m| m.lock().unwrap().clone())
}

impl Runner {
    pub fn new(filepath: PathBuf) -> Self {
        let runner = Self {
            env: Arc::new(Mutex::new(Env::new())),
            filepath,
            modules: HashMap::new(),
            current_span: None,
            native_methods: get_global_native_methods(),
            test_mode: false,
            interactive: true,
            granted_permissions: get_global_granted_permissions(),
            vfs: get_global_vfs(),
        };
        crate::stdlib::register_global_builtins(runner.env.clone());
        runner
    }

    pub fn run(&mut self, stmts: &[Stmt]) -> Result<Value, String> {
        if !self.native_methods.is_empty() {
            set_global_native_methods(self.native_methods.clone());
        }
        if !self.granted_permissions.is_empty() {
            set_global_granted_permissions(self.granted_permissions.clone());
        }
        if self.vfs.is_some() {
            set_global_vfs(self.vfs.clone());
        }
        let mut app_entry = None;
        let mut app_count = 0;
        for stmt in stmts {
            if let Stmt::FuncDecl {
                name, annotations, ..
            } = stmt
            {
                if annotations.iter().any(|a| a.name == "Application") {
                    app_entry = Some(name.clone());
                    app_count += 1;
                }
            }
        }

        if app_count > 1 {
            return Err("Only one @Application entry point is allowed.".to_string());
        }

        let mut last_val = Value::Nil;
        for stmt in stmts {
            let should_execute = if app_entry.is_some() {
                matches!(
                    stmt,
                    Stmt::FuncDecl { .. }
                        | Stmt::StructDecl { .. }
                        | Stmt::EnumDecl { .. }
                        | Stmt::TraitDecl { .. }
                        | Stmt::ImplDecl { .. }
                        | Stmt::LetDecl { .. }
                        | Stmt::ConstDecl { .. }
                        | Stmt::ImportDecl { .. }
                        | Stmt::PluginDecl { .. }
                )
            } else {
                true
            };

            if should_execute {
                match self.execute_statement(stmt, self.env.clone()) {
                    Ok(val) => {
                        last_val = val;
                    }
                    Err(e) => {
                        if let Some(ref span) = self.current_span {
                            return Err(format!(
                                "{} at {}:{}:{}",
                                e,
                                self.filepath.to_string_lossy(),
                                span.line,
                                span.col
                            ));
                        }
                        return Err(e);
                    }
                }
            }
        }

        if let Some(app_name) = app_entry {
            let app_func = self.env.lock().unwrap().get(&app_name);
            if let Some(app_val @ Value::Function { .. }) = app_func {
                let res = self.invoke_callback_value(&app_val, Vec::new())?;
                last_val = res;
            }
            let main_func = self.env.lock().unwrap().get("main");
            if let Some(main_val) = main_func {
                if let Value::Function { ref annotations, .. } = main_val {
                    let is_web = annotations.iter().any(|a| a.name == "Web");
                    let explicitly_called = stmts.iter().any(|s| match s {
                        Stmt::ExprStmt(Expr::Call(callee, ..)) => match &**callee {
                            Expr::Identifier(id, _) => id == "main",
                            _ => false,
                        },
                        Stmt::ExprStmt(Expr::Await(inner, _)) => {
                            if let Expr::Call(callee, ..) = &**inner {
                                match &**callee {
                                    Expr::Identifier(id, _) => id == "main",
                                    _ => false,
                                }
                            } else {
                                false
                            }
                        }
                        _ => false,
                    });
                    if !explicitly_called {
                        let res = self.invoke_callback_value(&main_val, Vec::new())?;
                        last_val = res;
                    }
                    if is_web {
                        let project_dir = std::path::Path::new(".");
                        match crate::web::build_web_project(project_dir) {
                            Ok(build_res) => {
                                let config = crate::web::WebServerConfig {
                                    dist_dir: build_res.dist_dir,
                                    port: build_res.port,
                                    routes: build_res.routes,
                                    hot_reload: false,
                                };
                                let _ = crate::web::serve_dist(config);
                            }
                            Err(e) => {
                                eprintln!("\x1b[1;31merror:\x1b[0m Failed to build web project: {}", e);
                            }
                        }
                    }
                }
            }
        }
        if crate::vm::is_event_loop_active() {
            println!(
                "\x1b[1;32m    Running\x1b[0m multi-threaded runtime daemon active (press Ctrl+C to exit)"
            );
            while crate::vm::is_event_loop_active() {
                self.process_callback_queue();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        Ok(last_val)
    }


    pub fn clone_for_thread(&self, env: Arc<Mutex<Env>>) -> Self {
        Self {
            env,
            filepath: self.filepath.clone(),
            modules: self.modules.clone(),
            current_span: None,
            native_methods: self.native_methods.clone(),
            test_mode: self.test_mode,
            interactive: self.interactive,
            granted_permissions: self.granted_permissions.clone(),
            vfs: self.vfs.clone(),
        }
    }


}
