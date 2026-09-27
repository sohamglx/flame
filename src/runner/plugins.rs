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
use super::core::Runner;

impl Runner {
    pub(crate) fn bind_param(&self, child_env: Arc<Mutex<Env>>, param: &Param, arg_val: Value) {
        // Store RefPath arguments directly; never re-wrap as RefPath::Var(param.name).
        let mut env = child_env.lock().unwrap();
        let name = param.name.clone();
        let is_mut = param.is_mut || name.contains("mut");
        env.define(name.clone(), arg_val.clone(), is_mut);
        if name == "&self" || name == "&mut self" || name == "mut self" {
            env.define("self".to_string(), arg_val, is_mut);
        }
    }

    pub(crate) fn load_rust_file_methods(&self, rs_code: &str, env: Arc<Mutex<Env>>) {
        let mut e = env.lock().unwrap();
        for line in rs_code.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub fn ") {
                if let Some(rest) = trimmed.strip_prefix("pub fn ") {
                    if let Some(fn_name) = rest.split('(').next() {
                        let fn_name = fn_name.trim().to_string();
                        if !fn_name.is_empty() {
                            e.define(
                                fn_name,
                                Value::Function {
                                    params: vec![],
                                    body: vec![],
                                    env: env.clone(),
                                    annotations: vec![],
                                },
                                false,
                            );
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn execute_plugin(&mut self, plugin_name: &str, env: Arc<Mutex<Env>>) -> Result<Value, String> {
        let rel_meta = format!(".flame/pkg/{}/{}.fmi", plugin_name, plugin_name);
        let meta_candidates = vec![
            PathBuf::from(&rel_meta),
            self.resolve_path(&rel_meta),
            self.filepath
                .parent()
                .unwrap_or(Path::new("."))
                .parent()
                .unwrap_or(Path::new("."))
                .join(&rel_meta),
        ];

        for meta_path in meta_candidates {
            if !meta_path.exists() {
                continue;
            }
            let meta_str = fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
            let meta = serde_json::from_str::<crate::package_manager::FlameMeta>(&meta_str)
                .map_err(|e| e.to_string())?;
            let mod_env = Arc::new(Mutex::new(Env::new()));
            mod_env.lock().unwrap().define(
                "__crate__".to_string(),
                Value::String(plugin_name.to_string()),
                false,
            );
            for fn_meta in &meta.functions {
                mod_env.lock().unwrap().define(
                    fn_meta.flame_name.clone(),
                    Value::Function {
                        params: fn_meta
                            .params
                            .iter()
                            .map(|p| Param {
                                name: p.name.clone(),
                                type_name: p.type_name.clone(),
                                default_val: None,
                                is_ref: false,
                                is_mut: false,
                            })
                            .collect(),
                        body: vec![],
                        env: mod_env.clone(),
                        annotations: vec![],
                    },
                    false,
                );
            }
            let mut map = mod_env.lock().unwrap().to_formula_map();
            map.insert("__module__".to_string(), Value::Bool(true));
            env.lock()
                .unwrap()
                .define(plugin_name.to_string(), Value::Formula(map), false);
            self.modules.insert(plugin_name.to_string(), mod_env);
            return Ok(Value::Nil);
        }

        let local_file = self
            .filepath
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(format!("{}.flame", plugin_name));
        let plugin_file = if local_file.exists() {
            local_file
        } else {
            let ws_file = self
                .filepath
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("src")
                .join(format!("{}.flame", plugin_name));
            if ws_file.exists() {
                ws_file
            } else {
                return Err(format!(
                    "Plugin '{}' not found as native metadata or local .flame file",
                    plugin_name
                ));
            }
        };

        let content = std::fs::read_to_string(&plugin_file).map_err(|e| e.to_string())?;
        let mut lexer = Lexer::new(&content);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == crate::lexer::TokenKind::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = Parser::new(tokens, plugin_file.to_string_lossy().to_string());
        let parsed_stmts = parser.parse().map_err(|e| e.message)?;

        for s in &parsed_stmts {
            self.execute_statement(s, env.clone())?;
        }
        Ok(Value::Nil)
    }

    pub(crate) fn resolve_path(&self, path_str: &str) -> PathBuf {
        let p = Path::new(path_str);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            let parent = self.filepath
                .parent()
                .unwrap_or_else(|| Path::new("."));
            let parent = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            parent.join(p)
        }
    }

    pub fn read_file_or_vfs(&self, path: &Path) -> Result<String, String> {
        let vfs_path = path.to_string_lossy().replace("\\", "/");
        if let Some(vfs) = &self.vfs {
            if let Some(content) = vfs.get(&vfs_path) {
                return Ok(content.clone());
            }
            // Fallback match by suffix for robust VFS loading
            for (k, v) in vfs.iter() {
                if vfs_path.ends_with(k) || k.ends_with(&vfs_path) {
                    return Ok(v.clone());
                }
            }
        }
        if path.exists() {
            return fs::read_to_string(path).map_err(|e| e.to_string());
        }
        Err(format!("File not found: {}", path.display()))
    }

    pub(crate) fn extract_cmd_annotation_name(args: &[String], fallback_func_name: &str) -> String {
        for (idx, arg) in args.iter().enumerate() {
            let trimmed = arg.trim();
            if trimmed.starts_with("name:")
                || trimmed.starts_with("name :")
                || trimmed.starts_with("name=")
                || trimmed.starts_with("name =")
            {
                let val = if let Some((_, v)) = trimmed.split_once(':') {
                    v
                } else if let Some((_, v)) = trimmed.split_once('=') {
                    v
                } else {
                    trimmed
                };
                return val.trim().trim_matches('"').trim_matches('\'').to_string();
            } else if !trimmed.starts_with("about")
                && !trimmed.starts_with("description")
                && idx == 0
            {
                return trimmed.trim_matches('"').trim_matches('\'').to_string();
            }
        }
        fallback_func_name.to_string()
    }

    pub(crate) fn execute_cli_dispatch(&mut self, env: Arc<Mutex<Env>>) -> Result<Value, String> {
        let raw_args = std::env::args().collect::<Vec<String>>();
        let mut script_args: Vec<String> = Vec::new();

        let mut script_file_index = None;
        for (i, arg) in raw_args.iter().enumerate() {
            if i > 0 && (arg.ends_with(".fm") || arg.ends_with(".flame")) {
                script_file_index = Some(i);
                break;
            }
        }

        if let Some(idx) = script_file_index {
            for arg in &raw_args[idx + 1..] {
                if arg == "--local" {
                    continue;
                }
                script_args.push(arg.clone());
            }
        } else {
            for arg in raw_args.into_iter().skip(1) {
                if arg == "--local" {
                    continue;
                }
                script_args.push(arg);
            }
        }

        if script_args.is_empty() || script_args[0] == "help" || script_args[0] == "--help" {
            println!("Usage: <command> [args]");
            println!("\nAvailable Commands:");
            let mut search_envs = vec![env.clone(), self.env.clone()];
            for (_, mod_env) in &self.modules {
                search_envs.push(mod_env.clone());
            }
            let mut printed_cmds = std::collections::HashSet::new();
            for e in search_envs {
                let env_lock = e.lock().unwrap();
                for (name, entry) in env_lock.variables.iter() {
                    if let Value::Function {
                        annotations,
                        params,
                        ..
                    } = &entry.value
                    {
                        if let Some(cmd_anno) = annotations.iter().find(|a| a.name == "Command") {
                            let cmd_name = Self::extract_cmd_annotation_name(&cmd_anno.args, name);
                            if printed_cmds.insert(cmd_name.clone()) {
                                let mut param_strs = Vec::new();
                                for p in params {
                                    param_strs.push(format!("--{} <{}>", p.name, p.type_name));
                                }
                                println!("  {} {}", cmd_name, param_strs.join(" "));
                            }
                        }
                    }
                }
            }
            let mut map = std::collections::HashMap::new();
            map.insert("$variant".to_string(), Value::String("help".to_string()));
            return Ok(Value::Object(map));
        }

        let subcommand = &script_args[0];
        let mut target_func = None;
        let mut target_params = Vec::new();

        {
            let mut search_envs = vec![env.clone(), self.env.clone()];
            for (_, mod_env) in &self.modules {
                search_envs.push(mod_env.clone());
            }
            'find_func: for e in search_envs {
                let env_lock = e.lock().unwrap();
                for (name, entry) in env_lock.variables.iter() {
                    if let Value::Function {
                        annotations,
                        params,
                        ..
                    } = &entry.value
                    {
                        if let Some(cmd_anno) = annotations.iter().find(|a| a.name == "Command") {
                            let cmd_name = Self::extract_cmd_annotation_name(&cmd_anno.args, name);
                            if &cmd_name == subcommand {
                                target_func = Some(entry.value.clone());
                                target_params = params.clone();
                                break 'find_func;
                            }
                        }
                    }
                }
            }
        }

        if let Some(_func) = target_func {
            let mut map = std::collections::HashMap::new();
            for param in &target_params {
                let mut found_val = None;
                let flag_name = format!("--{}", param.name);
                for (i, arg) in script_args.iter().enumerate() {
                    let mut str_val = None;
                    if arg == &flag_name {
                        if param.type_name == "Bool" || param.type_name == "bool" {
                            found_val = Some(Value::Bool(true));
                        } else if i + 1 < script_args.len() {
                            str_val = Some(script_args[i + 1].clone());
                        }
                    } else if arg.starts_with(&format!("{}=", flag_name)) {
                        let parts: Vec<&str> = arg.splitn(2, '=').collect();
                        if parts.len() == 2 {
                            if param.type_name == "Bool" || param.type_name == "bool" {
                                found_val = Some(Value::Bool(parts[1] == "true"));
                            } else {
                                str_val = Some(parts[1].to_string());
                            }
                        }
                    }

                    if let Some(s) = str_val {
                        if param.type_name == "Int" || param.type_name == "int" {
                            if let Ok(num) = s.parse::<i64>() {
                                found_val = Some(Value::Int(num));
                            } else {
                                println!(
                                    "\x1b[1;31merror:\x1b[0m invalid integer for '{}'",
                                    param.name
                                );
                            }
                        } else if param.type_name == "Float" || param.type_name == "float" {
                            if let Ok(num) = s.parse::<f64>() {
                                found_val = Some(Value::Float(num));
                            } else {
                                println!(
                                    "\x1b[1;31merror:\x1b[0m invalid float for '{}'",
                                    param.name
                                );
                            }
                        } else {
                            found_val = Some(Value::String(s));
                        }
                    }
                    if found_val.is_some() {
                        break;
                    }
                }

                if found_val.is_none() {
                    if let Some(def_expr) = &param.default_val {
                        if let Ok(val) = self.eval_expr(def_expr, self.env.clone()) {
                            found_val = Some(val);
                        }
                    }
                }

                if found_val.is_none() {
                    if param.type_name == "Bool" || param.type_name == "bool" {
                        found_val = Some(Value::Bool(false));
                    } else if param.type_name.starts_with("List")
                        || param.type_name.starts_with("Vector")
                    {
                        found_val = Some(Value::Tuple(Vec::new()));
                    } else {
                        found_val = Some(Value::String("".to_string()));
                    }
                }
                map.insert(param.name.clone(), found_val.unwrap());
            }
            map.insert("$variant".to_string(), Value::String(subcommand.clone()));
            return Ok(Value::Object(map));
        } else {
            let mut map = std::collections::HashMap::new();
            map.insert("$variant".to_string(), Value::String(subcommand.clone()));
            return Ok(Value::Object(map));
        }
    }
}
