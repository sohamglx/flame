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
    pub fn process_callback_queue(&mut self) {
        let rx = {
            let (_, ref receiver) = *crate::vm::get_runtime_queue();
            receiver
        };
        loop {
            let req = {
                let guard = match rx.try_lock() {
                    Ok(g) => g,
                    Err(_) => break,
                };
                match guard.try_recv() {
                    Ok(r) => r,
                    Err(TryRecvError::Empty) => {
                        break;
                    }

                    Err(TryRecvError::Disconnected) => {
                        break;
                    }
                }
            };

            let callback_val = match crate::vm::get_callback_value(req.callback.function_id) {
                Some(val) => val,
                None => {
                    let _ = req.responder.send(crate::vm::CValue::null());
                    continue;
                }
            };

            let mut flame_args = Vec::new();
            for arg in req.args {
                flame_args.push(Value::unpack(arg, "", ""));
            }

            let res_val = match self.invoke_callback_value(&callback_val, flame_args) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Runtime error during callback execution: {}", e);
                    Value::Nil
                }
            };

            let _ = req.responder.send(res_val.pack());
        }
    }

    pub fn invoke_callback_value(
        &mut self,
        callback_val: &Value,
        evaled_args: Vec<Value>,
    ) -> Result<Value, String> {
        match callback_val {
            Value::Function {
                params,
                body,
                env: closure_env,
                annotations,
            } => {
                let child_env = Arc::new(Mutex::new(Env::new_child(closure_env.clone())));
                for anno in annotations {
                    if anno.name == "Requires" {
                        for arg_str in &anno.args {
                            if arg_str.starts_with('"') && arg_str.ends_with('"') {
                                let mod_name = arg_str[1..arg_str.len() - 1].to_string();
                                let parts: Vec<String> =
                                    mod_name.split('.').map(|s| s.to_string()).collect();
                                let _ = self.execute_statement(
                                    &Stmt::ImportDecl {
                                        path: parts,
                                        glob: false,
                                        alias: None,
                                        span: anno.span.clone(),
                                    },
                                    child_env.clone(),
                                );
                            }
                        }
                    } else if anno.name == "Permission" {
                        for arg_str in &anno.args {
                            if arg_str.starts_with('"') && arg_str.ends_with('"') {
                                let perm_name = arg_str[1..arg_str.len() - 1].to_string();
                                if !self.granted_permissions.contains(&perm_name) {
                                    if !self.interactive {
                                        return Ok(Value::EnumValue(
                                            "Result".to_string(),
                                            "Err".to_string(),
                                            crate::vm::EnumData::Tuple(vec![Value::String(
                                                format!("PermissionDenied: {}", perm_name),
                                            )]),
                                        ));
                                    } else {
                                        println!(
                                            "Function requires permission for: {}. Allow? [y/N]",
                                            perm_name
                                        );
                                        let mut input = String::new();
                                        if std::io::stdin().read_line(&mut input).is_ok()
                                            && input.trim().eq_ignore_ascii_case("y")
                                        {
                                            self.granted_permissions.insert(perm_name);
                                        } else {
                                            return Ok(Value::EnumValue(
                                                "Result".to_string(),
                                                "Err".to_string(),
                                                crate::vm::EnumData::Tuple(vec![Value::String(
                                                    format!("PermissionDenied: {}", perm_name),
                                                )]),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !matches!(
                        anno.name.as_str(),
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
                    ) {
                        let mut anno_func_opt = closure_env.lock().unwrap().get(&anno.name);
                        if anno_func_opt.is_none() {
                            anno_func_opt = self.env.lock().unwrap().get(&anno.name);
                        }
                        if anno_func_opt.is_none() {
                            for (_, mod_env) in &self.modules {
                                if let Some(f) = mod_env.lock().unwrap().get(&anno.name) {
                                    anno_func_opt = Some(f);
                                    break;
                                }
                            }
                        }
                        if anno_func_opt.is_none() && anno.name.contains('.') {
                            let parts: Vec<&str> = anno.name.split('.').collect();
                            if let Some(mut current) = closure_env.lock().unwrap().get(parts[0]) {
                                for part in &parts[1..] {
                                    if let Value::Object(map) = &current {
                                        if let Some(next) = map.get(*part) {
                                            current = next.clone();
                                        } else {
                                            break;
                                        }
                                    } else if let Value::Formula(map) = &current {
                                        if let Some(next) = map.get(*part) {
                                            current = next.clone();
                                        } else {
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                }
                                anno_func_opt = Some(current);
                            }
                        }

                        if anno_func_opt.is_none() {
                            let env_lock = closure_env.lock().unwrap();
                            for (_, val) in env_lock.variables.iter() {
                                if let Value::Object(map) = &val.value {
                                    if let Some(exported_anno) = map.get(&anno.name) {
                                        anno_func_opt = Some(exported_anno.clone());
                                        break;
                                    }
                                } else if let Value::Formula(map) = &val.value {
                                    if let Some(exported_anno) = map.get(&anno.name) {
                                        anno_func_opt = Some(exported_anno.clone());
                                        break;
                                    }
                                }
                            }
                        }

                        if let Some(anno_func) = anno_func_opt {
                            let mut anno_args = Vec::new();
                            for arg_str in &anno.args {
                                let mut lexer = crate::lexer::Lexer::new(arg_str);
                                let mut tokens = Vec::new();
                                loop {
                                    let tok = lexer.next_token();
                                    let is_eof = tok.kind == crate::lexer::TokenKind::EOF;
                                    tokens.push(tok);
                                    if is_eof {
                                        break;
                                    }
                                }
                                if tokens.len() >= 2
                                    && tokens[0].kind == crate::lexer::TokenKind::Identifier
                                    && (tokens[1].kind == crate::lexer::TokenKind::Colon
                                        || tokens[1].kind == crate::lexer::TokenKind::Equal)
                                {
                                    tokens.remove(0);
                                    tokens.remove(0);
                                }
                                let mut parser =
                                    crate::parser::Parser::new(tokens, "anno_arg".to_string());
                                if let Ok(expr) = parser.parse_expr() {
                                    if let Ok(val) = self.eval_expr(&expr, closure_env.clone()) {
                                        anno_args.push(val);
                                    }
                                }
                            }
                            let target_name = {
                                let env_lock = closure_env.lock().unwrap();
                                env_lock.variables.iter()
                                    .find(|(_, v)| matches!(&v.value, Value::Function { body: b, .. } if b.len() == body.len()))
                                    .map(|(k, _)| k.clone())
                                    .unwrap_or_else(|| "function".to_string())
                            };
                            let ctx_val = crate::runner::core::build_annotation_context(
                                target_name,
                                "function",
                                params,
                                None,
                                annotations,
                                Some(callback_val.clone()),
                                closure_env.clone(),
                                &self.filepath,
                            );
                            let _guard = crate::runner::core::ScopedAnnotationContext::new(ctx_val);

                            match self.invoke_callback_value(&anno_func, anno_args) {
                                Ok(anno_res) => {
                                    child_env.lock().unwrap().define(
                                        anno.name.clone(),
                                        anno_res.clone(),
                                        true,
                                    );
                                    child_env.lock().unwrap().define(
                                        anno.name.to_lowercase(),
                                        anno_res.clone(),
                                        true,
                                    );
                                    child_env.lock().unwrap().define(
                                        format!("__{}_data__", anno.name),
                                        anno_res,
                                        true,
                                    );
                                }
                                Err(e) => {
                                    return Err(format!(
                                        "Annotation '{}' failed: {}",
                                        anno.name, e
                                    ));
                                }
                            }
                        }
                    }
                }
                let is_cli = annotations.iter().any(|a| a.name == "Cli");
                if is_cli {
                    let cli_obj = self.execute_cli_dispatch(closure_env.clone())?;
                    if let Some(first_param) = params.first() {
                        self.bind_param(child_env.clone(), first_param, cli_obj);
                    }
                } else {
                    for (i, p) in params.iter().enumerate() {
                        if i < evaled_args.len() {
                            self.bind_param(child_env.clone(), p, evaled_args[i].clone());
                        } else if let Some(def_expr) = &p.default_val {
                            if let Ok(val) = self.eval_expr(def_expr, child_env.clone()) {
                                self.bind_param(child_env.clone(), p, val);
                            }
                        }
                    }
                }
                let mut last_val = Value::Nil;
                for stmt in body {
                    let res = self.execute_statement(stmt, child_env.clone())?;
                    if let Value::Return(ret_val) = res {
                        return Ok(*ret_val);
                    }
                    last_val = res;
                }
                Ok(last_val)
            }
            Value::NativeCallback(cb) => cb(evaled_args),
            Value::NativeClosure(crate::vm::NativeClosureType(cb)) => cb(evaled_args),
            _ => Ok(Value::Nil),
        }
    }


}
