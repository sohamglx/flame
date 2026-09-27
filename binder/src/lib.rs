use flamelang::runner::Runner;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use flamelang::vm::Value;

/// Extension trait providing ergonomic accessors for `Value`.
pub trait ValueExt {
    fn to_int(&self) -> Option<i64>;
    fn to_float(&self) -> Option<f64>;
    fn to_bool(&self) -> Option<bool>;
    fn to_str(&self) -> Option<&str>;
    fn as_list(&self) -> Option<&[Value]>;
    fn as_formula(&self) -> Option<&HashMap<String, Value>>;
}

impl ValueExt for Value {
    fn to_int(&self) -> Option<i64> {
        self.as_int().ok()
    }

    fn to_float(&self) -> Option<f64> {
        self.as_float().ok()
    }

    fn to_bool(&self) -> Option<bool> {
        self.as_bool().ok()
    }

    fn to_str(&self) -> Option<&str> {
        self.as_str().ok()
    }

    fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::Tuple(l) => Some(l.as_slice()),
            _ => None,
        }
    }

    fn as_formula(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Formula(m) | Value::Object(m) => Some(m),
            _ => None,
        }
    }
}

/// In-process Flame execution engine and script host.
pub struct Binder {
    pub runner: Runner,
}

impl Default for Binder {
    fn default() -> Self {
        Self::new()
    }
}

impl Binder {
    /// Creates a new, blank Binder with the Flame standard library and environment initialized.
    pub fn new() -> Self {
        Self {
            runner: Runner::new(PathBuf::from("embed.fm")),
        }
    }

    /// Loads and executes a single Flame source file (`.fm`).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path_ref = path.as_ref();
        let content = std::fs::read_to_string(path_ref)
            .map_err(|e| format!("Failed to read {}: {}", path_ref.display(), e))?;

        let mut binder = Self {
            runner: Runner::new(path_ref.to_path_buf()),
        };
        binder.load_source(&content, &path_ref.to_string_lossy())?;
        Ok(binder)
    }

    /// Loads an entire Flame application project from a directory containing `flame.toml`.
    /// This resolves dependency packages under `.flame/pkg`, compiles all source files in `src/`,
    /// and initializes the global runtime environment so host applications can embed the full project.
    pub fn load_project(project_dir: impl AsRef<Path>) -> Result<Self, String> {
        let root = project_dir.as_ref();
        let toml_path = root.join("flame.toml");
        if !toml_path.exists() {
            return Err(format!("No flame.toml found in {}", root.display()));
        }

        let main_path = root.join("src").join("main.fm");
        let runner_path = if main_path.exists() {
            main_path
        } else {
            root.join("src")
        };

        let mut binder = Self {
            runner: Runner::new(runner_path),
        };

        // 1. Load dependency packages from .flame/pkg if present
        let pkg_dir = root.join(".flame").join("pkg");
        if pkg_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&pkg_dir) {
                for entry in entries.flatten() {
                    let p = entry.path().join("src");
                    if p.is_dir() {
                        binder.load_directory(&p)?;
                    }
                }
            }
        }

        // 2. Load all project source files in src/
        let src_dir = root.join("src");
        if src_dir.exists() {
            binder.load_directory(&src_dir)?;
        }

        Ok(binder)
    }

    /// Recursively loads and executes all `.fm` files in the given directory.
    pub fn load_directory(&mut self, dir: impl AsRef<Path>) -> Result<(), String> {
        let d = dir.as_ref();
        if let Ok(entries) = std::fs::read_dir(d) {
            let mut fm_files = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "fm") {
                    fm_files.push(path);
                }
            }

            // Deterministic sort; keep main.fm last if present so dependencies run first
            fm_files.sort_by(|a, b| {
                let a_is_main = a.file_name().map_or(false, |n| n == "main.fm");
                let b_is_main = b.file_name().map_or(false, |n| n == "main.fm");
                a_is_main.cmp(&b_is_main).then_with(|| a.cmp(b))
            });

            for file in fm_files {
                let content = std::fs::read_to_string(&file)
                    .map_err(|e| format!("Failed to read {}: {}", file.display(), e))?;
                self.load_source(&content, &file.to_string_lossy())?;
            }
        }
        Ok(())
    }

    /// Parses and evaluates Flame source code in the current VM environment.
    pub fn load_source(&mut self, source: &str, file_name: &str) -> Result<Value, String> {
        let mut lexer = flamelang::lexer::Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == flamelang::lexer::TokenKind::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }

        let mut parser = flamelang::parser::Parser::new(tokens, file_name.to_string());
        let stmts = parser.parse().map_err(|e| e.message)?;
        self.runner.run(&stmts)
    }

    /// Evaluates a Flame expression or code snippet and returns the resulting `Value`.
    pub fn eval(&mut self, code: &str) -> Result<Value, String> {
        self.load_source(code, "eval.fm")
    }

    /// Invokes a named Flame function with the specified argument list.
    pub fn call(&mut self, func_name: &str, args: Vec<Value>) -> Result<Value, String> {
        let func_val = {
            let env = self.runner.env.lock().unwrap();
            env.get(func_name)
                .ok_or_else(|| format!("Function '{}' not found in Flame environment", func_name))?
        };
        self.runner.invoke_callback_value(&func_val, args)
    }

    /// Reads a global variable from the Flame environment.
    pub fn get_global(&self, name: &str) -> Option<Value> {
        let env = self.runner.env.lock().unwrap();
        env.get(name)
    }

    /// Sets or creates a global variable in the Flame environment.
    pub fn set_global(&mut self, name: &str, value: Value) {
        let mut env = self.runner.env.lock().unwrap();
        env.define(name.to_string(), value, true);
    }

    /// Registers a native Rust function / callback in the Flame environment.
    ///
    /// This allows host applications (e.g. game engines, plugin systems, CLI tools)
    /// to expose Rust functions directly into Flame scripts.
    pub fn register_fn<F>(&mut self, name: &str, f: F)
    where
        F: Fn(Vec<Value>) -> Result<Value, String> + Send + Sync + 'static,
    {
        let closure = Value::NativeClosure(flamelang::vm::NativeClosureType(Arc::new(f)));
        let mut env = self.runner.env.lock().unwrap();
        env.define(name.to_string(), closure, false);
    }

    /// Grants a security permission (e.g., `"fs"`, `"net"`, `"process"`) to the execution environment.
    pub fn grant_permission(&mut self, permission: &str) {
        self.runner
            .granted_permissions
            .insert(permission.to_string());
    }
}

/// Handle to a standalone compiled Flame runtime executable created with `fmp build`.
///
/// Use this when you want to execute scripts using a pre-compiled native runtime binary
/// configured with specific features and native dependencies.
#[derive(Debug, Clone)]
pub struct CompiledRuntime {
    pub binary_path: PathBuf,
}

impl CompiledRuntime {
    /// Creates a runtime handle from an existing binary file path.
    pub fn from_path(binary_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
    }

    /// Resolves the compiled runtime binary from a Flame project directory.
    /// Checks `target/release/<pkg>` or `target/dev/<pkg>`.
    pub fn from_project(project_dir: impl AsRef<Path>, release: bool) -> Result<Self, String> {
        let root = project_dir.as_ref();
        let toml_path = root.join("flame.toml");
        if !toml_path.exists() {
            return Err(format!("No flame.toml found in {}", root.display()));
        }

        let mut pkg_name = "app".to_string();
        if let Ok(content) = std::fs::read_to_string(&toml_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("name =") {
                    if let Some(val) = trimmed.split('=').nth(1) {
                        pkg_name = val.trim().trim_matches('"').trim_matches('\'').to_string();
                    }
                }
            }
        }

        let profile = if release { "release" } else { "dev" };
        let ext = std::env::consts::EXE_SUFFIX;
        let exe_name = format!("{}{}", pkg_name, ext);
        let binary_path = root.join("target").join(profile).join(exe_name);

        if !binary_path.exists() {
            return Err(format!(
                "Compiled runtime binary not found at '{}'. Run 'fmp build' first.",
                binary_path.display()
            ));
        }

        Ok(Self { binary_path })
    }

    /// Runs a Flame script file using this compiled runtime binary.
    pub fn run_script(
        &self,
        script_path: impl AsRef<Path>,
        args: &[String],
    ) -> Result<std::process::Output, String> {
        let script = script_path.as_ref();
        if !script.exists() {
            return Err(format!("Script file '{}' not found", script.display()));
        }

        std::process::Command::new(&self.binary_path)
            .env("FLAME_ENTRY_FILE", script.to_string_lossy().to_string())
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute compiled runtime: {}", e))
    }

    /// Runs a Flame script and returns its captured standard output string.
    pub fn run_script_stdout(
        &self,
        script_path: impl AsRef<Path>,
        args: &[String],
    ) -> Result<String, String> {
        let output = self.run_script(script_path, args)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Runtime exited with error: {}", stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Executes the runtime binary directly with custom arguments.
    pub fn execute(&self, args: &[String]) -> Result<std::process::Output, String> {
        std::process::Command::new(&self.binary_path)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute compiled runtime: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_binder_load_and_call() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file
            .write_all(
                b"
        export fn hello(name: String) -> String {
            return $\"Hello, {name}\";
        }
        ",
            )
            .unwrap();

        let path = temp_file.path().to_str().unwrap();
        let mut binder = Binder::load(path).unwrap();

        let args = vec![Value::from("World")];
        let res = binder.call("hello", args).unwrap();

        assert_eq!(res.to_str(), Some("Hello, World"));
    }

    #[test]
    fn test_binder_register_native_fn() {
        let mut binder = Binder::new();

        // Host registers a native Rust function
        binder.register_fn("add_numbers", |args| {
            let a = args.get(0).and_then(|v| v.to_int()).unwrap_or(0);
            let b = args.get(1).and_then(|v| v.to_int()).unwrap_or(0);
            Ok(Value::from(a + b))
        });

        // Flame script calls the native Rust function
        binder
            .load_source(
                r#"
            export fn calculate() -> Int {
                return add_numbers(15, 27);
            }
        "#,
                "calc.fm",
            )
            .unwrap();

        let res = binder.call("calculate", vec![]).unwrap();
        assert_eq!(res.to_int(), Some(42));
    }

    #[test]
    fn test_binder_eval_and_globals() {
        let mut binder = Binder::new();
        binder.set_global("score", Value::from(100));

        let res = binder.eval("score + 50").unwrap();
        assert_eq!(res.to_int(), Some(150));
    }

    #[test]
    fn test_binder_load_project() {
        let temp_dir = tempfile::tempdir().unwrap();
        let proj_path = temp_dir.path();

        // Create flame.toml
        std::fs::write(
            proj_path.join("flame.toml"),
            "[package]\nname = \"mod_app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        // Create src/main.fm
        let src_dir = proj_path.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            src_dir.join("main.fm"),
            "export fn get_version() -> String {\n    return \"1.0.0\";\n}\n",
        )
        .unwrap();

        let mut binder = Binder::load_project(proj_path).unwrap();
        let res = binder.call("get_version", vec![]).unwrap();
        assert_eq!(res.to_str(), Some("1.0.0"));
    }
}
