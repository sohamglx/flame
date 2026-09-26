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

use std::cell::RefCell;
use std::sync::OnceLock;

thread_local! {
    static CURRENT_ANNOTATION_CONTEXT: RefCell<Option<Value>> = const { RefCell::new(None) };
}

pub fn set_current_annotation_context(ctx: Option<Value>) -> Option<Value> {
    CURRENT_ANNOTATION_CONTEXT.with(|c| c.replace(ctx))
}

pub fn get_current_annotation_context() -> Option<Value> {
    CURRENT_ANNOTATION_CONTEXT.with(|c| c.borrow().clone())
}

pub struct ScopedAnnotationContext {
    prev: Option<Value>,
}

impl ScopedAnnotationContext {
    pub fn new(ctx: Value) -> Self {
        let prev = set_current_annotation_context(Some(ctx));
        Self { prev }
    }
}

impl Drop for ScopedAnnotationContext {
    fn drop(&mut self) {
        set_current_annotation_context(self.prev.take());
    }
}

pub fn build_target_metadata(
    target_name: String,
    kind: &str,
    params: &[crate::parser::Param],
    return_type_opt: Option<&str>,
    annotations: &[crate::parser::Annotation],
    target_func_opt: Option<Value>,
    env: Arc<Mutex<Env>>,
) -> Value {
    let mut target_map = HashMap::new();
    target_map.insert("name".to_string(), Value::String(target_name.clone()));
    target_map.insert("kind".to_string(), Value::String(kind.to_string()));

    let mut param_vals = Vec::new();
    for p in params {
        let mut p_map = HashMap::new();
        p_map.insert("name".to_string(), Value::String(p.name.clone()));
        p_map.insert("type".to_string(), Value::String(p.type_name.clone()));
        p_map.insert(
            "has_default".to_string(),
            Value::Bool(p.default_val.is_some()),
        );
        p_map.insert("is_ref".to_string(), Value::Bool(p.is_ref));
        p_map.insert("is_mut".to_string(), Value::Bool(p.is_mut));
        param_vals.push(Value::Formula(p_map));
    }
    target_map.insert("parameters".to_string(), Value::Tuple(param_vals));

    let ret_str = return_type_opt.unwrap_or("Nil").to_string();
    target_map.insert("return_type".to_string(), Value::String(ret_str));

    let mut anno_vals = Vec::new();
    for a in annotations {
        let mut a_map = HashMap::new();
        a_map.insert("name".to_string(), Value::String(a.name.clone()));
        let args_vals = a.args.iter().map(|s| Value::String(s.clone())).collect();
        a_map.insert("args".to_string(), Value::Tuple(args_vals));
        anno_vals.push(Value::Formula(a_map));
    }
    target_map.insert("annotations".to_string(), Value::Tuple(anno_vals));

    if let Some(target_val) = target_func_opt {
        let mut clean_target = target_val.clone();
        if let Value::Function { ref mut annotations, .. } = clean_target {
            annotations.retain(|a| {
                matches!(
                    a.name.as_str(),
                    "Test"
                        | "Setup"
                        | "Cleanup"
                        | "BeforeAll"
                        | "AfterAll"
                        | "Ignore"
                        | "Only"
                        | "Parameterized"
                        | "Benchmark"
                        | "Cli"
                        | "Command"
                        | "ExpectPanic"
                        | "Requires"
                        | "Permission"
                        | "Docs"
                        | "Platform"
                        | "Application"
                        | "Embedded"
                )
            });
        }

        let t_val = clean_target.clone();
        target_map.insert(
            "ref".to_string(),
            Value::NativeClosure(crate::vm::NativeClosureType(Arc::new(move |mut args| {
                if !args.is_empty() && matches!(args[0], Value::Formula(_) | Value::Object(_)) {
                    args.remove(0);
                }
                if args.is_empty() {
                    Ok(t_val.clone())
                } else {
                    crate::vm::invoke_callback_val(&t_val, args)
                }
            }))),
        );

        let t_name = target_name;
        let env_clone = env;
        let orig_target = clean_target;
        target_map.insert(
            "transform".to_string(),
            Value::NativeClosure(crate::vm::NativeClosureType(Arc::new(move |mut args| {
                if !args.is_empty() && matches!(args[0], Value::Formula(_) | Value::Object(_)) {
                    args.remove(0);
                }
                if args.is_empty() {
                    return Err("transform expects a transformer function argument".to_string());
                }
                let transformer = args[0].clone();
                let orig_target = orig_target.clone();
                let t_name_str = t_name.clone();
                let env_inner = env_clone.clone();

                let param_count = match &transformer {
                    Value::Function { params, .. } => params.len(),
                    _ => 0,
                };

                let wrapped_func = Value::NativeClosure(crate::vm::NativeClosureType(Arc::new(
                    move |call_args| {
                        if param_count == 1 && !call_args.is_empty() {
                            let factory_res = crate::vm::invoke_callback_val(
                                &transformer,
                                vec![orig_target.clone()],
                            )?;
                            if matches!(
                                factory_res,
                                Value::Function { .. }
                                    | Value::NativeClosure(_)
                                    | Value::NativeCallback(_)
                            ) {
                                return crate::vm::invoke_callback_val(&factory_res, call_args);
                            }
                        }

                        let mut full_args = Vec::with_capacity(call_args.len() + 1);
                        full_args.push(orig_target.clone());
                        full_args.extend(call_args);
                        let res = crate::vm::invoke_callback_val(&transformer, full_args)?;
                        if matches!(
                            res,
                            Value::Function { .. }
                                | Value::NativeClosure(_)
                                | Value::NativeCallback(_)
                        ) && param_count <= 1
                        {
                            crate::vm::invoke_callback_val(&res, vec![])
                        } else {
                            Ok(res)
                        }
                    },
                )));

                env_inner
                    .lock()
                    .unwrap()
                    .define(t_name_str, wrapped_func.clone(), false);
                Ok(wrapped_func)
            }))),
        );
    } else {
        target_map.insert(
            "ref".to_string(),
            Value::NativeClosure(crate::vm::NativeClosureType(Arc::new(|_| Ok(Value::Nil)))),
        );
        target_map.insert(
            "transform".to_string(),
            Value::NativeClosure(crate::vm::NativeClosureType(Arc::new(|_| {
                Err("cannot transform target without a callable body".to_string())
            }))),
        );
    }

    Value::Formula(target_map)
}

pub fn build_annotation_context(
    target_name: String,
    kind: &str,
    params: &[crate::parser::Param],
    return_type_opt: Option<&str>,
    annotations: &[crate::parser::Annotation],
    target_func_opt: Option<Value>,
    env: Arc<Mutex<Env>>,
    filepath: &Path,
) -> Value {
    let mut ctx_map = HashMap::new();

    let target = build_target_metadata(
        target_name,
        kind,
        params,
        return_type_opt,
        annotations,
        target_func_opt,
        env,
    );
    ctx_map.insert("target".to_string(), target);

    let mut compiler_map = HashMap::new();
    compiler_map.insert("name".to_string(), Value::String("flame".to_string()));
    compiler_map.insert(
        "version".to_string(),
        Value::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    ctx_map.insert("compiler".to_string(), Value::Formula(compiler_map));

    let mut module_map = HashMap::new();
    let mod_name = filepath
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main")
        .to_string();
    let fp_str = filepath.to_string_lossy().to_string();
    module_map.insert("name".to_string(), Value::String(mod_name));
    module_map.insert("filepath".to_string(), Value::String(fp_str));
    ctx_map.insert("module".to_string(), Value::Formula(module_map));

    let mut build_map = HashMap::new();
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    build_map.insert(
        "target".to_string(),
        Value::String(format!("{}-unknown-{}-gnu", arch, os)),
    );
    build_map.insert(
        "mode".to_string(),
        Value::String(
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_string(),
        ),
    );
    build_map.insert("platform".to_string(), Value::String(os.to_string()));
    build_map.insert("arch".to_string(), Value::String(arch.to_string()));
    let features = vec![
        Value::String("std".to_string()),
        Value::String("net".to_string()),
        Value::String("web".to_string()),
        Value::String("annotation".to_string()),
    ];
    build_map.insert("features".to_string(), Value::Tuple(features));
    ctx_map.insert("build".to_string(), Value::Formula(build_map));

    Value::Formula(ctx_map)
}

static GLOBAL_NATIVE_METHODS: OnceLock<Mutex<HashMap<String, fn(*const CValue, usize) -> CValue>>> =
    OnceLock::new();
static GLOBAL_GRANTED_PERMISSIONS: OnceLock<Mutex<std::collections::HashSet<String>>> =
    OnceLock::new();
static GLOBAL_VFS: OnceLock<Mutex<Option<HashMap<String, String>>>> = OnceLock::new();
static GLOBAL_MODULES: OnceLock<Mutex<HashMap<String, Arc<Mutex<Env>>>>> = OnceLock::new();

pub fn set_global_modules(modules: HashMap<String, Arc<Mutex<Env>>>) {
    let mutex = GLOBAL_MODULES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut lock = mutex.lock().unwrap();
    for (k, v) in modules {
        lock.insert(k, v);
    }
}

pub fn get_global_modules() -> HashMap<String, Arc<Mutex<Env>>> {
    GLOBAL_MODULES
        .get()
        .map(|m| m.lock().unwrap().clone())
        .unwrap_or_default()
}

pub fn set_global_native_methods(methods: HashMap<String, fn(*const CValue, usize) -> CValue>) {
    let mutex = GLOBAL_NATIVE_METHODS.get_or_init(|| Mutex::new(HashMap::new()));
    *mutex.lock().unwrap() = methods;
}

pub fn get_global_native_methods() -> HashMap<String, fn(*const CValue, usize) -> CValue> {
    GLOBAL_NATIVE_METHODS
        .get()
        .map(|m| m.lock().unwrap().clone())
        .unwrap_or_default()
}

pub fn set_global_granted_permissions(perms: std::collections::HashSet<String>) {
    let mutex =
        GLOBAL_GRANTED_PERMISSIONS.get_or_init(|| Mutex::new(std::collections::HashSet::new()));
    *mutex.lock().unwrap() = perms;
}

pub fn get_global_granted_permissions() -> std::collections::HashSet<String> {
    GLOBAL_GRANTED_PERMISSIONS
        .get()
        .map(|m| m.lock().unwrap().clone())
        .unwrap_or_default()
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
            modules: get_global_modules(),
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
        if !self.modules.is_empty() {
            set_global_modules(self.modules.clone());
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
                if let Value::Function {
                    ref annotations, ..
                } = main_val
                {
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
                                eprintln!(
                                    "\x1b[1;31merror:\x1b[0m Failed to build web project: {}",
                                    e
                                );
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
