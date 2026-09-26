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
    pub(crate) fn eval_explicit_conversion(
        &mut self,
        val: &Value,
        member: &str,
        args: &[(Option<String>, Expr)],
        env: Arc<Mutex<Env>>,
    ) -> Option<Result<Value, String>> {
        match member {
            "toString" => {
                if let Value::Object(map) | Value::Formula(map) | Value::StructInstance { fields: map, .. } = val {
                    if let Some(to_string_val) = map.get("toString") {
                        let mut evaled_args = Vec::new();
                        for (_, arg_expr) in args {
                            if let Ok(a) = self.eval_expr(arg_expr, env.clone()) {
                                evaled_args.push(a);
                            }
                        }
                        return Some(self.invoke_callback_value(to_string_val, evaled_args));
                    }
                }
                if let Value::StructInstance { name, .. } = val {
                    if self.modules.get(&format!("impl_{}", name)).map_or(false, |m| m.lock().unwrap().variables.contains_key("toString")) {
                        return None;
                    }
                }
                let mut prec = None;
                if !args.is_empty() {
                    if let Ok(Value::Int(p)) = self.eval_expr(&args[0].1, env.clone()) {
                        prec = Some(p as usize);
                    }
                }
                match val {
                    Value::Float(f) => {
                        if let Some(p) = prec {
                            Some(Ok(Value::String(format!("{:.*}", p, f))))
                        } else {
                            Some(Ok(Value::String(f.to_string())))
                        }
                    }
                    Value::String(s) => Some(Ok(Value::String(s.clone()))),
                    Value::Bytes(b) => Some(Ok(Value::String(String::from_utf8_lossy(b).into_owned()))),
                    Value::Byte(b) => Some(Ok(Value::String(format!("{}", *b as char)))),
                    _ => Some(Ok(Value::String(val.to_string()))),
                }
            }
            "toInt" | "tryInt" => {
                let is_try = member.starts_with("try");
                let mut radix = 10;
                if !args.is_empty() {
                    if let Ok(Value::Int(r)) = self.eval_expr(&args[0].1, env.clone()) {
                        if r >= 2 && r <= 36 {
                            radix = r as u32;
                        }
                    }
                }
                match val {
                    Value::Int(i) => Some(Ok(Value::Int(*i))),
                    Value::Byte(b) => Some(Ok(Value::Int(*b as i64))),
                    Value::Float(f) => Some(Ok(Value::Int(*f as i64))),
                    Value::Bool(b) => Some(Ok(Value::Int(if *b { 1 } else { 0 }))),
                    Value::Quantity(v, _) => Some(Ok(Value::Int(*v as i64))),
                    Value::String(s) => match i64::from_str_radix(s.trim(), radix) {
                        Ok(res) => Some(Ok(Value::Int(res))),
                        Err(_) => {
                            if is_try {
                                Some(Ok(Value::Nil))
                            } else {
                                Some(Err(format!(
                                    "cannot convert string '{}' to Int with radix {}",
                                    s, radix
                                )))
                            }
                        }
                    },
                    _ => {
                        if is_try {
                            Some(Ok(Value::Nil))
                        } else {
                            Some(Err(format!("cannot convert {:?} to Int", val)))
                        }
                    }
                }
            }
            "toFloat" | "toDouble" | "tryFloat" | "tryDouble" => {
                let is_try = member.starts_with("try");
                match val {
                    Value::Float(f) => Some(Ok(Value::Float(*f))),
                    Value::Int(i) => Some(Ok(Value::Float(*i as f64))),
                    Value::Bool(b) => Some(Ok(Value::Float(if *b { 1.0 } else { 0.0 }))),
                    Value::Quantity(v, _) => Some(Ok(Value::Float(*v))),
                    Value::String(s) => match s.trim().parse::<f64>() {
                        Ok(res) => Some(Ok(Value::Float(res))),
                        Err(_) => {
                            if is_try {
                                Some(Ok(Value::Nil))
                            } else {
                                Some(Err(format!("cannot convert string '{}' to Float", s)))
                            }
                        }
                    },
                    _ => {
                        if is_try {
                            Some(Ok(Value::Nil))
                        } else {
                            Some(Err(format!("cannot convert {:?} to Float", val)))
                        }
                    }
                }
            }
            "toBool" | "tryBool" => {
                let is_try = member.starts_with("try");
                match val {
                    Value::Bool(b) => Some(Ok(Value::Bool(*b))),
                    Value::Int(i) => Some(Ok(Value::Bool(*i != 0))),
                    Value::Float(f) => Some(Ok(Value::Bool(*f != 0.0))),
                    Value::Quantity(v, _) => Some(Ok(Value::Bool(*v != 0.0))),
                    Value::String(s) => {
                        let lower = s.trim().to_lowercase();
                        if lower == "true" || lower == "1" {
                            Some(Ok(Value::Bool(true)))
                        } else if lower == "false" || lower == "0" {
                            Some(Ok(Value::Bool(false)))
                        } else if is_try {
                            Some(Ok(Value::Nil))
                        } else {
                            Some(Err(format!("cannot convert string '{}' to Bool", s)))
                        }
                    }
                    _ => {
                        if is_try {
                            Some(Ok(Value::Nil))
                        } else {
                            Some(Err(format!("cannot convert {:?} to Bool", val)))
                        }
                    }
                }
            }
            "toChar" => match val {
                Value::Int(i) => {
                    if let Some(c) = char::from_u32(*i as u32) {
                        Some(Ok(Value::String(c.to_string())))
                    } else {
                        Some(Err(format!("invalid character integer {}", i)))
                    }
                }
                Value::String(s) => {
                    if let Some(c) = s.chars().next() {
                        Some(Ok(Value::String(c.to_string())))
                    } else {
                        Some(Err("empty string has no character".to_string()))
                    }
                }
                _ => Some(Err(format!("cannot convert {:?} to Char", val))),
            },
            "toByte" => match val {
                Value::String(s) => Some(Ok(Value::Bytes(s.clone().into_bytes()))),
                Value::Int(i) => {
                    if *i < 0 || *i > 255 {
                        Some(Err(format!(
                            "toByte: value {} is out of bounds for Byte (0..255)",
                            i
                        )))
                    } else {
                        Some(Ok(Value::Byte(*i as u8)))
                    }
                }
                Value::Bytes(b) => Some(Ok(Value::Bytes(b.clone()))),
                _ => Some(Err(format!("cannot convert {:?} to Byte", val))),
            },
            _ => None,
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr, env: Arc<Mutex<Env>>) -> Result<Value, String> {
        self.current_span = Some(expr.span());
        match expr {
            Expr::Literal(lit, _) => match lit {
                LiteralValue::Int(i) => Ok(Value::Int(*i)),
                LiteralValue::Float(f) => Ok(Value::Float(*f)),
                LiteralValue::String(s) => Ok(Value::String(s.clone())),
                LiteralValue::Bool(b) => Ok(Value::Bool(*b)),
                LiteralValue::Nil => Ok(Value::Nil),
            },

            Expr::Identifier(name, _) => {
                let val = {
                    let e = env.lock().unwrap();
                    e.get(name)
                };
                if let Some(val) = val {
                    if let Value::Moved(moved_name) = val {
                        return Err(format!(
                            "use of moved value '{}'. Value was moved. Use '&{}' to borrow or '{}.clone()' to copy.",
                            moved_name, moved_name, moved_name
                        ));
                    }
                    let mut current = val;

                    loop {
                        match current {
                            Value::RefPath(path, _) => {
                                current = self.read_target(env.clone(), path)?;
                            }
                            other => return Ok(other),
                        }
                    }
                } else {
                    let mut found = None;
                    for (mod_name, mod_env) in &self.modules {
                        if mod_name == name || mod_name.ends_with(&format!(".{}", name)) {
                            let mut map = mod_env.lock().unwrap().to_formula_map();
                            map.insert("__module__".to_string(), Value::Bool(true));
                            found = Some(Value::Formula(map));
                            break;
                        }
                    }
                    if let Some(val) = found {
                        Ok(val)
                    } else {
                        Err(format!("undefined variable '{}'", name))
                    }
                }
            }
            Expr::Closure {
                params,
                body,
                annotations,
                ..
            } => Ok(Value::Function {
                params: params.clone(),
                body: body.clone(),
                env: env.clone(),
                annotations: annotations.clone(),
            }),
            Expr::Borrow(inner, is_mut, _) => {
                // Construct a reference path when possible; fall back to Ref(value)
                match &**inner {
                    Expr::Identifier(name, _) => {
                        let val = {
                            let e = env.lock().unwrap();
                            e.get(name)
                        };
                        if let Some(val) = val {
                            if let Value::Moved(moved_name) = val {
                                return Err(format!(
                                    "use of moved value '{}'. Value was moved. Use '&{}' to borrow or '{}.clone()' to copy.",
                                    moved_name, moved_name, moved_name
                                ));
                            }
                            if let Value::RefPath(path, _) = val {
                                return Ok(Value::RefPath(path, *is_mut));
                            }
                            return Ok(Value::RefPath(
                                RefPath::Var(name.clone(), env.clone()),
                                *is_mut,
                            ));
                        }
                    }
                    Expr::Dot(inner_id, member, _) => {
                        if let Expr::Identifier(owner, _) = &**inner_id {
                            return Ok(Value::RefPath(
                                RefPath::Field {
                                    owner: owner.clone(),
                                    member: member.clone(),
                                    env: env.clone(),
                                },
                                *is_mut,
                            ));
                        }
                    }
                    _ => {}
                }
                let val = self.eval_expr(inner, env.clone())?;
                Ok(Value::Ref(Box::new(val)))
            }
            Expr::Unary(op, inner, _) => match op {
                UnaryOp::Neg => {
                    let val = self.eval_expr(inner, env.clone())?;
                    match val {
                        Value::Int(i) => Ok(Value::Int(-i)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err("cannot apply unary '-' to non-numeric value".to_string()),
                    }
                }
                UnaryOp::Not => {
                    let val = self.eval_expr(inner, env.clone())?;
                    Ok(Value::Bool(!val.is_truthy()))
                }
                UnaryOp::NonNullAssert => {
                    let val = self.eval_expr(inner, env.clone())?;
                    if matches!(val, Value::Nil) {
                        return Err(
                            "cannot unwrap nil value: non-null assertion '!' failed".to_string()
                        );
                    }
                    Ok(val)
                }
                UnaryOp::PreInc | UnaryOp::PreDec | UnaryOp::PostInc | UnaryOp::PostDec => {
                    let delta: i64 = match op {
                        UnaryOp::PreInc | UnaryOp::PostInc => 1,
                        UnaryOp::PreDec | UnaryOp::PostDec => -1,
                        _ => 0,
                    };
                    let is_post = matches!(op, UnaryOp::PostInc | UnaryOp::PostDec);

                    if let Expr::Identifier(var_name, _) = &**inner {
                        let current = {
                            let e = env.lock().unwrap();
                            e.get(var_name)
                        };
                        let l_val = match &current {
                            Some(Value::RefPath(path, _)) => {
                                self.read_target(env.clone(), path.clone())?
                            }
                            Some(val) => val.clone(),
                            None => return Err(format!("undefined variable '{}'", var_name)),
                        };

                        let (new_val, return_val) = match l_val {
                            Value::Int(i) => {
                                let new_i = i + delta;
                                (
                                    Value::Int(new_i),
                                    if is_post {
                                        Value::Int(i)
                                    } else {
                                        Value::Int(new_i)
                                    },
                                )
                            }
                            Value::Float(f) => {
                                let new_f = f + (delta as f64);
                                (
                                    Value::Float(new_f),
                                    if is_post {
                                        Value::Float(f)
                                    } else {
                                        Value::Float(new_f)
                                    },
                                )
                            }
                            _ => {
                                return Err(format!(
                                    "cannot increment/decrement non-numeric variable '{}'",
                                    var_name
                                ));
                            }
                        };

                        if let Some(Value::RefPath(path, mutable)) = current {
                            if !mutable {
                                return Err(format!(
                                    "cannot mutate through immutable reference '{}'",
                                    var_name
                                ));
                            }
                            self.write_back(env.clone(), path, new_val)?;
                        } else {
                            env.lock().unwrap().assign(var_name.clone(), new_val)?;
                        }
                        Ok(return_val)
                    } else if let Expr::Dot(inner_owner, member, _) = &**inner {
                        if let Expr::Identifier(owner, _) = &**inner_owner {
                            let l_val = self.eval_expr(inner, env.clone())?;
                            let (new_val, return_val) = match l_val {
                                Value::Int(i) => {
                                    let new_i = i + delta;
                                    (
                                        Value::Int(new_i),
                                        if is_post {
                                            Value::Int(i)
                                        } else {
                                            Value::Int(new_i)
                                        },
                                    )
                                }
                                Value::Float(f) => {
                                    let new_f = f + (delta as f64);
                                    (
                                        Value::Float(new_f),
                                        if is_post {
                                            Value::Float(f)
                                        } else {
                                            Value::Float(new_f)
                                        },
                                    )
                                }
                                _ => {
                                    return Err(
                                        "cannot increment/decrement non-numeric field".to_string()
                                    );
                                }
                            };
                            self.write_back(
                                env.clone(),
                                RefPath::Field {
                                    owner: owner.clone(),
                                    member: member.clone(),
                                    env: env.clone(),
                                },
                                new_val,
                            )?;
                            Ok(return_val)
                        } else {
                            Err("increment/decrement operand must be a variable or field"
                                .to_string())
                        }
                    } else {
                        Err("increment/decrement operand must be a variable or field".to_string())
                    }
                }
            },

            Expr::SafeDot(inner, member, _) => {
                let left = self.eval_expr(inner, env.clone())?;
                if matches!(left, Value::Nil) {
                    return Ok(Value::Nil);
                }
                match left {
                    Value::StructConstructor { name, .. } => {
                        if let Some(impl_env) = self.modules.get(&format!("impl_{}", name)) {
                            if let Some(method) = impl_env.lock().unwrap().get(member) {
                                return Ok(method);
                            }
                        }
                        Ok(Value::Nil)
                    }
                    Value::StructInstance { name, fields } => {
                        if let Some(val) = fields.get(member) {
                            return Ok(val.clone());
                        }
                        if let Some(impl_env) = self.modules.get(&format!("impl_{}", name)) {
                            if let Some(method) = impl_env.lock().unwrap().get(member) {
                                return Ok(method);
                            }
                        }
                        Ok(Value::Nil)
                    }
                    Value::Formula(map) | Value::Object(map) => {
                        if let Some(val) = map.get(member) {
                            Ok(val.clone())
                        } else {
                            Ok(Value::Nil)
                        }
                    }
                    Value::EnumValue(_, _, data) => match data {
                        EnumData::Struct(map) => {
                            if let Some(val) = map.get(member) {
                                Ok(val.clone())
                            } else {
                                Ok(Value::Nil)
                            }
                        }
                        _ => Ok(Value::Nil),
                    },
                    _ => Ok(Value::Nil),
                }
            }
            Expr::Binary(left, op, right, _) => {
                if matches!(
                    op,
                    BinaryOp::Assign
                        | BinaryOp::PlusAssign
                        | BinaryOp::MinusAssign
                        | BinaryOp::MulAssign
                        | BinaryOp::DivAssign
                        | BinaryOp::ModAssign
                        | BinaryOp::BitAndAssign
                        | BinaryOp::BitOrAssign
                        | BinaryOp::BitXorAssign
                        | BinaryOp::ShlAssign
                        | BinaryOp::ShrAssign
                ) {
                    let compute_compound = |op: &BinaryOp,
                                            l_val: Value,
                                            r_val: &Value|
                     -> Result<Value, String> {
                        match (op, l_val, r_val) {
                            (BinaryOp::PlusAssign, Value::Int(a), Value::Int(b)) => a
                                .checked_add(*b)
                                .map(Value::Int)
                                .ok_or_else(|| "integer overflow in +=".to_string()),
                            (BinaryOp::PlusAssign, Value::Float(a), Value::Float(b)) => {
                                Ok(Value::Float(a + b))
                            }
                            (BinaryOp::PlusAssign, Value::String(a), Value::String(b)) => {
                                Ok(Value::String(format!("{}{}", a, b)))
                            }
                            (BinaryOp::MinusAssign, Value::Int(a), Value::Int(b)) => a
                                .checked_sub(*b)
                                .map(Value::Int)
                                .ok_or_else(|| "integer overflow in -=".to_string()),
                            (BinaryOp::MinusAssign, Value::Float(a), Value::Float(b)) => {
                                Ok(Value::Float(a - b))
                            }
                            (BinaryOp::MulAssign, Value::Int(a), Value::Int(b)) => a
                                .checked_mul(*b)
                                .map(Value::Int)
                                .ok_or_else(|| "integer overflow in *=".to_string()),
                            (BinaryOp::MulAssign, Value::Float(a), Value::Float(b)) => {
                                Ok(Value::Float(a * b))
                            }
                            (BinaryOp::DivAssign, Value::Int(a), Value::Int(b)) => a
                                .checked_div(*b)
                                .map(Value::Int)
                                .ok_or_else(|| "division by zero or overflow in /=".to_string()),
                            (BinaryOp::DivAssign, Value::Float(a), Value::Float(b)) => {
                                Ok(Value::Float(a / b))
                            }
                            (BinaryOp::ModAssign, Value::Int(a), Value::Int(b)) => a
                                .checked_rem(*b)
                                .map(Value::Int)
                                .ok_or_else(|| "division by zero or overflow in %=".to_string()),
                            (BinaryOp::BitAndAssign, Value::Int(a), Value::Int(b)) => {
                                Ok(Value::Int(a & b))
                            }
                            (BinaryOp::BitOrAssign, Value::Int(a), Value::Int(b)) => {
                                Ok(Value::Int(a | b))
                            }
                            (BinaryOp::BitXorAssign, Value::Int(a), Value::Int(b)) => {
                                Ok(Value::Int(a ^ b))
                            }
                            (BinaryOp::ShlAssign, Value::Int(a), Value::Int(b)) => {
                                Ok(Value::Int(a << b))
                            }
                            (BinaryOp::ShrAssign, Value::Int(a), Value::Int(b)) => {
                                Ok(Value::Int(a >> b))
                            }
                            _ => Err("invalid operands for compound assignment".to_string()),
                        }
                    };

                    // Support assignment to identifiers, references, and simple dot paths
                    if let Expr::Identifier(var_name, _) = &**left {
                        let mut r_val = self.eval_expr(right, env.clone())?;

                        let current = {
                            let e = env.lock().unwrap();
                            e.get(var_name)
                        };

                        if *op != BinaryOp::Assign {
                            let l_val = match &current {
                                Some(Value::RefPath(path, _)) => {
                                    self.read_target(env.clone(), path.clone())?
                                }
                                Some(val) => val.clone(),
                                None => return Err(format!("undefined variable '{}'", var_name)),
                            };
                            r_val = compute_compound(op, l_val, &r_val)?;
                        }

                        if let Some(Value::RefPath(path, mutable)) = current {
                            if !mutable {
                                return Err(format!(
                                    "cannot assign through immutable reference '{}'",
                                    var_name
                                ));
                            }

                            self.write_back(env.clone(), path, r_val.clone())?;

                            if let Expr::Identifier(src_name, _) = &**right {
                                env.lock().unwrap().move_var(src_name);
                            }

                            return Ok(r_val);
                        }
                        env.lock()
                            .unwrap()
                            .assign(var_name.clone(), r_val.clone())?;
                        if let Expr::Identifier(src_name, _) = &**right {
                            env.lock().unwrap().move_var(src_name);
                        }
                        return Ok(r_val);
                    } else if let Expr::Dot(inner, member, _) = &**left {
                        if let Expr::Identifier(owner, _) = &**inner {
                            let mut r_val = self.eval_expr(right, env.clone())?;

                            if *op != BinaryOp::Assign {
                                let l_val = self.eval_expr(left, env.clone())?;
                                r_val = compute_compound(op, l_val, &r_val)?;
                            }

                            self.write_back(
                                env.clone(),
                                RefPath::Field {
                                    owner: owner.clone(),
                                    member: member.clone(),
                                    env: env.clone(),
                                },
                                r_val.clone(),
                            )?;
                            if let Expr::Identifier(src_name, _) = &**right {
                                env.lock().unwrap().move_var(src_name);
                            }
                            return Ok(r_val);
                        } else if let Expr::Index(inner, idx, _) = &**left {
                            if let Expr::Identifier(owner, _) = &**inner {
                                let idx_val = self.eval_expr(idx, env.clone())?;
                                let idx_int = if let Value::Int(i) = idx_val {
                                    i as usize
                                } else {
                                    return Err("Index must be an integer".to_string());
                                };
                                let mut r_val = self.eval_expr(right, env.clone())?;

                                if *op != BinaryOp::Assign {
                                    let l_val = self.eval_expr(left, env.clone())?;
                                    r_val = compute_compound(op, l_val, &r_val)?;
                                }

                                self.write_back(
                                    env.clone(),
                                    crate::vm::RefPath::Index {
                                        owner: owner.clone(),
                                        index: idx_int,
                                        env: env.clone(),
                                    },
                                    r_val.clone(),
                                )?;
                                if let Expr::Identifier(src_name, _) = &**right {
                                    env.lock().unwrap().move_var(src_name);
                                }
                                return Ok(r_val);
                            } else {
                                return Err("left-hand side assignment must be a variable array"
                                    .to_string());
                            }
                        } else {
                            return Err(
                                "left-hand side assignment must be a variable or variable field"
                                    .to_string(),
                            );
                        }
                    }
                }

                if *op == BinaryOp::NilCoalesce {
                    let l = self.eval_expr(left, env.clone())?;
                    if !matches!(l, Value::Nil) {
                        return Ok(l);
                    }
                    return self.eval_expr(right, env.clone());
                }
                if *op == BinaryOp::And {
                    let l = self.eval_expr(left, env.clone())?;
                    if !l.is_truthy() {
                        return Ok(Value::Bool(false));
                    }
                    let r = self.eval_expr(right, env.clone())?;
                    return Ok(Value::Bool(r.is_truthy()));
                }
                if *op == BinaryOp::Or {
                    let l = self.eval_expr(left, env.clone())?;
                    if l.is_truthy() {
                        return Ok(Value::Bool(true));
                    }
                    let r = self.eval_expr(right, env.clone())?;
                    return Ok(Value::Bool(r.is_truthy()));
                }

                let mut l = self.eval_expr(left, env.clone())?;
                let mut r = self.eval_expr(right, env.clone())?;
                if let Value::NativeCallback(cb) = &l {
                    if let Ok(res) = cb(vec![]) {
                        l = res;
                    }
                }
                if let Value::NativeCallback(cb) = &r {
                    if let Ok(res) = cb(vec![]) {
                        r = res;
                    }
                }
                match (&l, &r) {
                    (Value::Int(a), Value::Int(b)) => match op {
                        BinaryOp::Add => a
                            .checked_add(*b)
                            .map(Value::Int)
                            .ok_or_else(|| "integer overflow in +".to_string()),
                        BinaryOp::Sub => a
                            .checked_sub(*b)
                            .map(Value::Int)
                            .ok_or_else(|| "integer overflow in -".to_string()),
                        BinaryOp::Mul => a
                            .checked_mul(*b)
                            .map(Value::Int)
                            .ok_or_else(|| "integer overflow in *".to_string()),
                        BinaryOp::Div => a
                            .checked_div(*b)
                            .map(Value::Int)
                            .ok_or_else(|| "division by zero or overflow in /".to_string()),
                        BinaryOp::Mod => a
                            .checked_rem(*b)
                            .map(Value::Int)
                            .ok_or_else(|| "division by zero or overflow in %".to_string()),
                        BinaryOp::BitAnd => Ok(Value::Int(a & b)),
                        BinaryOp::BitOr => Ok(Value::Int(a | b)),
                        BinaryOp::BitXor => Ok(Value::Int(a ^ b)),
                        BinaryOp::Shl => Ok(Value::Int(a << b)),
                        BinaryOp::Shr => Ok(Value::Int(a >> b)),
                        BinaryOp::Eq => Ok(Value::Bool(a == b)),
                        BinaryOp::Ne => Ok(Value::Bool(a != b)),
                        BinaryOp::Gt => Ok(Value::Bool(a > b)),
                        BinaryOp::Ge => Ok(Value::Bool(a >= b)),
                        BinaryOp::Lt => Ok(Value::Bool(a < b)),
                        BinaryOp::Le => Ok(Value::Bool(a <= b)),
                        BinaryOp::Range => Ok(Value::Range(*a, *b)),
                        _ => Ok(Value::Nil),
                    },
                    (Value::Float(a), Value::Float(b)) => match op {
                        BinaryOp::Add => Ok(Value::Float(a + b)),
                        BinaryOp::Sub => Ok(Value::Float(a - b)),
                        BinaryOp::Mul => Ok(Value::Float(a * b)),
                        BinaryOp::Div => Ok(Value::Float(if *b != 0.0 { a / b } else { 0.0 })),
                        BinaryOp::BitXor => Ok(Value::Float(a.powf(*b))),
                        BinaryOp::Eq => Ok(Value::Bool(a == b)),
                        BinaryOp::Ne => Ok(Value::Bool(a != b)),
                        BinaryOp::Gt => Ok(Value::Bool(a > b)),
                        BinaryOp::Ge => Ok(Value::Bool(a >= b)),
                        BinaryOp::Lt => Ok(Value::Bool(a < b)),
                        BinaryOp::Le => Ok(Value::Bool(a <= b)),
                        _ => Ok(Value::Nil),
                    },
                    (Value::Float(a), Value::Int(b)) => match op {
                        BinaryOp::Add => Ok(Value::Float(a + (*b as f64))),
                        BinaryOp::Sub => Ok(Value::Float(a - (*b as f64))),
                        BinaryOp::Mul => Ok(Value::Float(a * (*b as f64))),
                        BinaryOp::Div => {
                            Ok(Value::Float(if *b != 0 { a / (*b as f64) } else { 0.0 }))
                        }
                        BinaryOp::BitXor => Ok(Value::Float(a.powi(*b as i32))),
                        BinaryOp::Eq => Ok(Value::Bool(*a == *b as f64)),
                        BinaryOp::Ne => Ok(Value::Bool(*a != *b as f64)),
                        BinaryOp::Gt => Ok(Value::Bool(*a > *b as f64)),
                        BinaryOp::Ge => Ok(Value::Bool(*a >= *b as f64)),
                        BinaryOp::Lt => Ok(Value::Bool(*a < *b as f64)),
                        BinaryOp::Le => Ok(Value::Bool(*a <= *b as f64)),
                        _ => Ok(Value::Nil),
                    },
                    (Value::Int(a), Value::Float(b)) => match op {
                        BinaryOp::Add => Ok(Value::Float((*a as f64) + b)),
                        BinaryOp::Sub => Ok(Value::Float((*a as f64) - b)),
                        BinaryOp::Mul => Ok(Value::Float((*a as f64) * b)),
                        BinaryOp::Div => {
                            Ok(Value::Float(if *b != 0.0 { (*a as f64) / b } else { 0.0 }))
                        }
                        BinaryOp::BitXor => Ok(Value::Float((*a as f64).powf(*b))),
                        BinaryOp::Eq => Ok(Value::Bool(*a as f64 == *b)),
                        BinaryOp::Ne => Ok(Value::Bool(*a as f64 != *b)),
                        BinaryOp::Gt => Ok(Value::Bool(*a as f64 > *b)),
                        BinaryOp::Ge => Ok(Value::Bool(*a as f64 >= *b)),
                        BinaryOp::Lt => Ok(Value::Bool((*a as f64) < *b)),
                        BinaryOp::Le => Ok(Value::Bool((*a as f64) <= *b)),
                        _ => Ok(Value::Nil),
                    },
                    (Value::String(a), Value::String(b)) => match op {
                        BinaryOp::Add => Ok(Value::String(format!("{}{}", a, b))),
                        BinaryOp::Eq => Ok(Value::Bool(a == b)),
                        BinaryOp::Ne => Ok(Value::Bool(a != b)),
                        _ => Ok(Value::Nil),
                    },
                    (Value::String(a), other) if *op == BinaryOp::Add => {
                        Ok(Value::String(format!("{}{}", a, other.to_string())))
                    }
                    (other, Value::String(b)) if *op == BinaryOp::Add => {
                        Ok(Value::String(format!("{}{}", other.to_string(), b)))
                    }
                    (Value::Bool(a), Value::Bool(b)) => match op {
                        BinaryOp::Eq => Ok(Value::Bool(a == b)),
                        BinaryOp::Ne => Ok(Value::Bool(a != b)),
                        _ => Ok(Value::Nil),
                    },
                    (Value::Nil, Value::Nil) => match op {
                        BinaryOp::Eq => Ok(Value::Bool(true)),
                        BinaryOp::Ne => Ok(Value::Bool(false)),
                        _ => Ok(Value::Nil),
                    },
                    (Value::Nil, _) | (_, Value::Nil) => match op {
                        BinaryOp::Eq => Ok(Value::Bool(false)),
                        BinaryOp::Ne => Ok(Value::Bool(true)),
                        _ => Ok(Value::Nil),
                    },
                    (Value::Unit(a), Value::Unit(b)) => match op {
                        BinaryOp::Mul => {
                            let mut res = a.clone();
                            for (k, v) in b {
                                *res.entry(k.clone()).or_insert(0) += v;
                                if res[k] == 0 {
                                    res.remove(k);
                                }
                            }
                            Ok(Value::Unit(res))
                        }
                        BinaryOp::Div => {
                            let mut res = a.clone();
                            for (k, v) in b {
                                *res.entry(k.clone()).or_insert(0) -= v;
                                if res[k] == 0 {
                                    res.remove(k);
                                }
                            }
                            Ok(Value::Unit(res))
                        }
                        BinaryOp::Eq => Ok(Value::Bool(a == b)),
                        BinaryOp::Ne => Ok(Value::Bool(a != b)),
                        _ => Err(format!("cannot apply operator {:?} to unit and unit", op)),
                    },
                    (Value::Quantity(av, au), Value::Quantity(bv, bu)) => match op {
                        BinaryOp::Add => {
                            if au == bu {
                                Ok(Value::Quantity(av + bv, au.clone()))
                            } else {
                                Err("cannot add quantities with different units".to_string())
                            }
                        }
                        BinaryOp::Sub => {
                            if au == bu {
                                Ok(Value::Quantity(av - bv, au.clone()))
                            } else {
                                Err("cannot subtract quantities with different units".to_string())
                            }
                        }
                        BinaryOp::Mul => {
                            let mut res = au.clone();
                            for (k, v) in bu {
                                *res.entry(k.clone()).or_insert(0) += v;
                                if res[k] == 0 {
                                    res.remove(k);
                                }
                            }
                            if res.is_empty() {
                                Ok(Value::Float(av * bv))
                            } else {
                                Ok(Value::Quantity(av * bv, res))
                            }
                        }
                        BinaryOp::Div => {
                            let mut res = au.clone();
                            for (k, v) in bu {
                                *res.entry(k.clone()).or_insert(0) -= v;
                                if res[k] == 0 {
                                    res.remove(k);
                                }
                            }
                            if res.is_empty() {
                                Ok(Value::Float(av / bv))
                            } else {
                                Ok(Value::Quantity(av / bv, res))
                            }
                        }
                        BinaryOp::BitXor => {
                            if bu.is_empty() {
                                let pow = bv.round() as i32;
                                if (*bv - pow as f64).abs() < 1e-9 {
                                    let mut res = HashMap::new();
                                    for (k, v) in au {
                                        let new_pow = v * pow;
                                        if new_pow != 0 {
                                            res.insert(k.clone(), new_pow);
                                        }
                                    }
                                    let new_v = av.powi(pow);
                                    if res.is_empty() {
                                        Ok(Value::Float(new_v))
                                    } else {
                                        Ok(Value::Quantity(new_v, res))
                                    }
                                } else {
                                    Err("fractional exponents on quantities with units are not supported".to_string())
                                }
                            } else {
                                Err("exponent must be a dimensionless number".to_string())
                            }
                        }
                        BinaryOp::Eq => Ok(Value::Bool(av == bv && au == bu)),
                        BinaryOp::Ne => Ok(Value::Bool(av != bv || au != bu)),
                        BinaryOp::Lt => {
                            if au == bu {
                                Ok(Value::Bool(av < bv))
                            } else {
                                Err("cannot compare quantities with different units".to_string())
                            }
                        }
                        BinaryOp::Le => {
                            if au == bu {
                                Ok(Value::Bool(av <= bv))
                            } else {
                                Err("cannot compare quantities with different units".to_string())
                            }
                        }
                        BinaryOp::Gt => {
                            if au == bu {
                                Ok(Value::Bool(av > bv))
                            } else {
                                Err("cannot compare quantities with different units".to_string())
                            }
                        }
                        BinaryOp::Ge => {
                            if au == bu {
                                Ok(Value::Bool(av >= bv))
                            } else {
                                Err("cannot compare quantities with different units".to_string())
                            }
                        }
                        _ => Err(format!(
                            "cannot apply operator {:?} to quantity and quantity",
                            op
                        )),
                    },
                    (Value::Quantity(av, au), Value::Unit(bu)) => match op {
                        BinaryOp::Mul => {
                            let mut res = au.clone();
                            for (k, v) in bu {
                                *res.entry(k.clone()).or_insert(0) += v;
                                if res[k] == 0 {
                                    res.remove(k);
                                }
                            }
                            if res.is_empty() {
                                Ok(Value::Float(*av))
                            } else {
                                Ok(Value::Quantity(*av, res))
                            }
                        }
                        BinaryOp::Div => {
                            let mut res = au.clone();
                            for (k, v) in bu {
                                *res.entry(k.clone()).or_insert(0) -= v;
                                if res[k] == 0 {
                                    res.remove(k);
                                }
                            }
                            if res.is_empty() {
                                Ok(Value::Float(*av))
                            } else {
                                Ok(Value::Quantity(*av, res))
                            }
                        }
                        BinaryOp::Eq => Ok(Value::Bool(*av == 1.0 && au == bu)),
                        BinaryOp::Ne => Ok(Value::Bool(*av != 1.0 || au != bu)),
                        _ => Err(format!(
                            "cannot apply operator {:?} to quantity and unit",
                            op
                        )),
                    },
                    (Value::Unit(au), Value::Quantity(bv, bu)) => match op {
                        BinaryOp::Mul => {
                            let mut res = au.clone();
                            for (k, v) in bu {
                                *res.entry(k.clone()).or_insert(0) += v;
                                if res[k] == 0 {
                                    res.remove(k);
                                }
                            }
                            if res.is_empty() {
                                Ok(Value::Float(*bv))
                            } else {
                                Ok(Value::Quantity(*bv, res))
                            }
                        }
                        BinaryOp::Div => {
                            let mut res = au.clone();
                            for (k, v) in bu {
                                *res.entry(k.clone()).or_insert(0) -= v;
                                if res[k] == 0 {
                                    res.remove(k);
                                }
                            }
                            let div_val = if *bv != 0.0 { 1.0 / bv } else { 0.0 };
                            if res.is_empty() {
                                Ok(Value::Float(div_val))
                            } else {
                                Ok(Value::Quantity(div_val, res))
                            }
                        }
                        BinaryOp::Eq => Ok(Value::Bool(*bv == 1.0 && au == bu)),
                        BinaryOp::Ne => Ok(Value::Bool(*bv != 1.0 || au != bu)),
                        _ => Err(format!(
                            "cannot apply operator {:?} to unit and quantity",
                            op
                        )),
                    },
                    (Value::Int(a), Value::Unit(b)) => match op {
                        BinaryOp::Mul => Ok(Value::Quantity(*a as f64, b.clone())),
                        BinaryOp::Div => {
                            let mut res = HashMap::new();
                            for (k, v) in b {
                                res.insert(k.clone(), -v);
                            }
                            Ok(Value::Quantity(*a as f64, res))
                        }
                        _ => Err(format!("cannot apply operator {:?} to int and unit", op)),
                    },
                    (Value::Float(a), Value::Unit(b)) => match op {
                        BinaryOp::Mul => Ok(Value::Quantity(*a, b.clone())),
                        BinaryOp::Div => {
                            let mut res = HashMap::new();
                            for (k, v) in b {
                                res.insert(k.clone(), -v);
                            }
                            Ok(Value::Quantity(*a, res))
                        }
                        _ => Err(format!("cannot apply operator {:?} to float and unit", op)),
                    },
                    (Value::Unit(a), Value::Int(b)) => match op {
                        BinaryOp::Mul => Ok(Value::Quantity(*b as f64, a.clone())),
                        BinaryOp::Div => {
                            let div_val = if *b != 0 { 1.0 / (*b as f64) } else { 0.0 };
                            Ok(Value::Quantity(div_val, a.clone()))
                        }
                        BinaryOp::BitXor => {
                            if *b == 0 {
                                Ok(Value::Float(1.0))
                            } else {
                                let mut res = HashMap::new();
                                for (k, v) in a {
                                    let new_pow = v * (*b as i32);
                                    if new_pow != 0 {
                                        res.insert(k.clone(), new_pow);
                                    }
                                }
                                if res.is_empty() {
                                    Ok(Value::Float(1.0))
                                } else {
                                    Ok(Value::Unit(res))
                                }
                            }
                        }
                        _ => Err(format!("cannot apply operator {:?} to unit and int", op)),
                    },
                    (Value::Unit(a), Value::Float(b)) => match op {
                        BinaryOp::Mul => Ok(Value::Quantity(*b, a.clone())),
                        BinaryOp::Div => {
                            let div_val = if *b != 0.0 { 1.0 / b } else { 0.0 };
                            Ok(Value::Quantity(div_val, a.clone()))
                        }
                        BinaryOp::BitXor => {
                            if b.fract() == 0.0 {
                                let pow = *b as i32;
                                if pow == 0 {
                                    Ok(Value::Float(1.0))
                                } else {
                                    let mut res = HashMap::new();
                                    for (k, v) in a {
                                        let new_pow = v * pow;
                                        if new_pow != 0 {
                                            res.insert(k.clone(), new_pow);
                                        }
                                    }
                                    if res.is_empty() {
                                        Ok(Value::Float(1.0))
                                    } else {
                                        Ok(Value::Unit(res))
                                    }
                                }
                            } else {
                                Err("fractional exponents on units are not supported".to_string())
                            }
                        }
                        _ => Err(format!("cannot apply operator {:?} to unit and float", op)),
                    },
                    (Value::Quantity(v, u), Value::Int(b)) => match op {
                        BinaryOp::Mul => Ok(Value::Quantity(v * (*b as f64), u.clone())),
                        BinaryOp::Div => Ok(Value::Quantity(v / (*b as f64), u.clone())),
                        BinaryOp::BitXor => {
                            let pow = *b as i32;
                            if pow == 0 {
                                Ok(Value::Float(1.0))
                            } else {
                                let mut res = HashMap::new();
                                for (k, p) in u {
                                    let new_pow = p * pow;
                                    if new_pow != 0 {
                                        res.insert(k.clone(), new_pow);
                                    }
                                }
                                let new_v = v.powi(pow);
                                if res.is_empty() {
                                    Ok(Value::Float(new_v))
                                } else {
                                    Ok(Value::Quantity(new_v, res))
                                }
                            }
                        }
                        _ => Err(format!(
                            "cannot apply operator {:?} to quantity and int",
                            op
                        )),
                    },
                    (Value::Quantity(v, u), Value::Float(b)) => match op {
                        BinaryOp::Mul => Ok(Value::Quantity(v * b, u.clone())),
                        BinaryOp::Div => Ok(Value::Quantity(v / b, u.clone())),
                        BinaryOp::BitXor => {
                            if b.fract() == 0.0 {
                                let pow = *b as i32;
                                if pow == 0 {
                                    Ok(Value::Float(1.0))
                                } else {
                                    let mut res = HashMap::new();
                                    for (k, p) in u {
                                        let new_pow = p * pow;
                                        if new_pow != 0 {
                                            res.insert(k.clone(), new_pow);
                                        }
                                    }
                                    let new_v = v.powi(pow);
                                    if res.is_empty() {
                                        Ok(Value::Float(new_v))
                                    } else {
                                        Ok(Value::Quantity(new_v, res))
                                    }
                                }
                            } else if u.is_empty() {
                                Ok(Value::Float(v.powf(*b)))
                            } else {
                                Err("fractional exponents on quantities with units are not supported".to_string())
                            }
                        }
                        _ => Err(format!(
                            "cannot apply operator {:?} to quantity and float",
                            op
                        )),
                    },
                    (Value::Int(b), Value::Quantity(v, u)) => match op {
                        BinaryOp::Mul => Ok(Value::Quantity(v * (*b as f64), u.clone())),
                        BinaryOp::Div => {
                            let mut res = HashMap::new();
                            for (k, p) in u {
                                res.insert(k.clone(), -p);
                            }
                            let div_val = if *v != 0.0 { (*b as f64) / v } else { 0.0 };
                            Ok(Value::Quantity(div_val, res))
                        }
                        _ => Err(format!(
                            "cannot apply operator {:?} to int and quantity",
                            op
                        )),
                    },
                    (Value::Float(b), Value::Quantity(v, u)) => match op {
                        BinaryOp::Mul => Ok(Value::Quantity(v * b, u.clone())),
                        BinaryOp::Div => {
                            let mut res = HashMap::new();
                            for (k, p) in u {
                                res.insert(k.clone(), -p);
                            }
                            let div_val = if *v != 0.0 { b / v } else { 0.0 };
                            Ok(Value::Quantity(div_val, res))
                        }
                        _ => Err(format!(
                            "cannot apply operator {:?} to float and quantity",
                            op
                        )),
                    },
                    (l_val, r_val) => match op {
                        BinaryOp::Eq => Ok(Value::Bool(l_val.to_string() == r_val.to_string())),
                        BinaryOp::Ne => Ok(Value::Bool(l_val.to_string() != r_val.to_string())),
                        _ => Ok(Value::Nil),
                    },
                }
            }
            Expr::Cast(inner, target_type_str, _) => {
                let val = self.eval_expr(inner, env.clone())?;
                if target_type_str.ends_with('?') && matches!(val, Value::Nil) {
                    return Ok(Value::Nil);
                }
                let clean_type = target_type_str.trim_end_matches('?').trim();
                match clean_type {
                    "String" => Ok(Value::String(val.to_string())),
                    "Int" => match val {
                        Value::Int(i) => Ok(Value::Int(i)),
                        Value::Float(f) => Ok(Value::Int(f as i64)),
                        Value::Byte(b) => Ok(Value::Int(b as i64)),
                        Value::Bool(b) => Ok(Value::Int(if b { 1 } else { 0 })),
                        Value::String(ref s) => s.trim().parse::<i64>().map(Value::Int).map_err(|e| e.to_string()),
                        _ => Err(format!("cannot cast {:?} to Int", val)),
                    },
                    "Float" => match val {
                        Value::Float(f) => Ok(Value::Float(f)),
                        Value::Int(i) => Ok(Value::Float(i as f64)),
                        Value::Byte(b) => Ok(Value::Float(b as f64)),
                        Value::String(ref s) => s.trim().parse::<f64>().map(Value::Float).map_err(|e| e.to_string()),
                        _ => Err(format!("cannot cast {:?} to Float", val)),
                    },
                    "Bool" => match val {
                        Value::Bool(b) => Ok(Value::Bool(b)),
                        Value::Int(i) => Ok(Value::Bool(i != 0)),
                        Value::Nil => Ok(Value::Bool(false)),
                        _ => Ok(Value::Bool(true)),
                    },
                    "Byte" => match val {
                        Value::Byte(b) => Ok(Value::Byte(b)),
                        Value::Int(i) => Ok(Value::Byte(i as u8)),
                        _ => Err(format!("cannot cast {:?} to Byte", val)),
                    },
                    _ => Ok(val),
                }
            }
            Expr::StructInit(inner, fields, _) => {
                let inner_val = self.eval_expr(inner, env.clone())?;
                match inner_val {
                    Value::StructConstructor { name, .. } => {
                        let mut map = HashMap::new();
                        for (field_name, field_expr) in fields {
                            let val = self.eval_expr(field_expr, env.clone())?;
                            map.insert(field_name.clone(), val);
                        }
                        Ok(Value::StructInstance {
                            name: name.clone(),
                            fields: map,
                        })
                    }
                    Value::VariantConstructor(enum_name, var) => {
                        if let crate::parser::EnumVariant::Struct(n, _) = var {
                            let mut map = HashMap::new();
                            for (field_name, field_expr) in fields {
                                let val = self.eval_expr(field_expr, env.clone())?;
                                map.insert(field_name.clone(), val);
                            }
                            return Ok(Value::EnumValue(enum_name, n, EnumData::Struct(map)));
                        }
                        Err(format!("variant is not a struct variant"))
                    }
                    _ => Err(format!("cannot initialize struct on non-constructor value")),
                }
            }
            Expr::Index(inner, idx, _) => {
                let inner_val = self.eval_expr(inner, env.clone())?;
                let idx_val = self.eval_expr(idx, env.clone())?;
                let idx_int = if let Value::Int(i) = idx_val {
                    i as usize
                } else {
                    return Err("Index must be an integer".to_string());
                };

                match inner_val {
                    Value::Tuple(elems) => {
                        if idx_int < elems.len() {
                            Ok(elems[idx_int].clone())
                        } else {
                            Err(format!("Index out of bounds: {}", idx_int))
                        }
                    }
                    Value::String(s) => {
                        if idx_int < s.len() {
                            Ok(Value::String(s.chars().nth(idx_int).unwrap().to_string()))
                        } else {
                            Err(format!("Index out of bounds: {}", idx_int))
                        }
                    }
                    Value::Bytes(b) => {
                        if idx_int < b.len() {
                            Ok(Value::Byte(b[idx_int]))
                        } else {
                            Err(format!("Index out of bounds: {}", idx_int))
                        }
                    }
                    _ => Err("Cannot index into this value".to_string()),
                }
            }
            Expr::Dot(inner, member, _) => {
                if member == "clone" {
                    if let Expr::Identifier(name, _) = &**inner {
                        if let Some(val) = env.lock().unwrap().get(name) {
                            if let Value::Moved(moved_name) = val {
                                return Err(format!(
                                    "use of moved value '{}'. Value was moved. Use '&{}' to borrow or '{}.clone()' to copy.",
                                    moved_name, moved_name, moved_name
                                ));
                            }
                            return Ok(val);
                        }
                    }
                }
                let mut left = self.eval_expr(inner, env.clone())?;
                if let Value::RefPath(path, _) = &left {
                    left = self.read_target(env.clone(), path.clone())?;
                }
                match left {
                    Value::StructConstructor { name, .. } => {
                        if let Some(impl_env) = self.modules.get(&format!("impl_{}", name)) {
                            if let Some(method) = impl_env.lock().unwrap().get(member) {
                                return Ok(method);
                            }
                        }
                        Err(format!(
                            "associated function or member '{}' not found for struct '{}'",
                            member, name
                        ))
                    }
                    Value::StructInstance { name, fields } => {
                        if let Some(val) = fields.get(member) {
                            return Ok(val.clone());
                        }
                        if let Some(impl_env) = self.modules.get(&format!("impl_{}", name)) {
                            if let Some(method) = impl_env.lock().unwrap().get(member) {
                                return Ok(method);
                            }
                        }
                        Err(format!(
                            "member or method '{}' not found on struct '{}'",
                            member, name
                        ))
                    }
                    Value::Formula(map) | Value::Object(map) => {
                        if let Some(val) = map.get(member) {
                            Ok(val.clone())
                        } else {
                            Err(format!(
                                "Property '{}' does not exist on this object or .fmi plugin package.",
                                member
                            ))
                        }
                    }
                    Value::EnumMeta(enum_name, variants) => {
                        for var in &variants {
                            match var {
                                crate::parser::EnumVariant::Unit(n) => {
                                    if n == member {
                                        return Ok(Value::EnumValue(
                                            enum_name.clone(),
                                            n.clone(),
                                            EnumData::Unit,
                                        ));
                                    }
                                }
                                crate::parser::EnumVariant::Tuple(n, _)
                                | crate::parser::EnumVariant::Struct(n, _) => {
                                    if n == member {
                                        return Ok(Value::VariantConstructor(
                                            enum_name.clone(),
                                            var.clone(),
                                        ));
                                    }
                                }
                            }
                        }
                        Err(format!(
                            "variant '{}' not found in enum '{}'",
                            member, enum_name
                        ))
                    }
                    Value::EnumValue(enum_name, variant_name, data) => match data {
                        EnumData::Struct(map) => {
                            if let Some(val) = map.get(member) {
                                Ok(val.clone())
                            } else {
                                Err(format!(
                                    "field '{}' not found in variant '{}.{}'",
                                    member, enum_name, variant_name
                                ))
                            }
                        }

                        EnumData::Tuple(values) => {
                            if values.len() == 1
                                && let Value::Formula(map) = &values[0]
                                && let Some(val) = map.get(member)
                            {
                                return Ok(val.clone());
                            }
                            Err(format!(
                                "field '{}' not found in variant '{}.{}'",
                                member, enum_name, variant_name
                            ))
                        }

                        EnumData::Unit => Err(format!(
                            "variant '{}.{}' has no fields",
                            enum_name, variant_name
                        )),
                    },
                    Value::Quantity(val, _) => {
                        if member == "value" {
                            Ok(Value::Float(val))
                        } else {
                            Err(format!("cannot access member '{}' on Quantity", member))
                        }
                    }
                    Value::Unit(_) => {
                        if member == "value" {
                            Ok(Value::Float(1.0))
                        } else {
                            Err(format!("cannot access member '{}' on Unit", member))
                        }
                    }
                    _ => {
                        return Err(format!(
                            "cannot access member '{}' on non-namespace value in '{}'",
                            member,
                            if let Expr::Identifier(name, _) = &**inner {
                                name
                            } else {
                                "unknown"
                            }
                        ));
                    }
                }
            }
            Expr::Call(callee, args, _) => {
                if let Expr::Identifier(name, _) = &**callee {
                    if name == "print" || name == "eprint" || name == "println" {
                        let mut parts = Vec::new();
                        for (_, arg) in args {
                            let val = self.eval_expr(arg, env.clone())?;
                            let val_str = match val {
                                Value::String(ref s) => s.clone(),
                                _ => crate::native_std::fmt::stringify_value(&val),
                            };
                            parts.push(val_str);
                        }
                        if name == "print" || name == "println" {
                            if name == "println" {
                                println!("{}", parts.join(" "));
                            } else {
                                print!("{}", parts.join(" "));
                                use std::io::Write;
                                let _ = std::io::stdout().flush();
                            }
                        } else {
                            eprintln!("\x1b[1;31m{}\x1b[0m", parts.join(" "));
                        }
                        return Ok(Value::Nil);
                    }
                    if name == "panic" {
                        let mut parts = Vec::new();
                        for (_, arg) in args {
                            let val = self.eval_expr(arg, env.clone())?;
                            let val_str = match val {
                                Value::String(ref s) => s.clone(),
                                _ => crate::native_std::fmt::stringify_value(&val),
                            };
                            parts.push(val_str);
                        }
                        let msg = if parts.is_empty() {
                            "explicit panic".to_string()
                        } else {
                            parts.join(" ")
                        };
                        return Err(msg);
                    }
                    if name == "input" {
                        use std::io::{self, Write};
                        if !args.is_empty() {
                            let arg_v = self.eval_expr(&args[0].1, env.clone())?;
                            print!("{}", arg_v.to_string());
                        }
                        let _ = io::stdout().flush();
                        let mut buffer = String::new();
                        let _ = io::stdin().read_line(&mut buffer);
                        return Ok(Value::String(buffer.trim_end().to_string()));
                    }
                    if name == "assert" || name == "assert_true" {
                        if args.is_empty() {
                            return Err(
                                "assert requires at least 1 argument (condition)".to_string()
                            );
                        }
                        let cond = self.eval_expr(&args[0].1, env.clone())?;
                        let msg = if args.len() > 1 {
                            self.eval_expr(&args[1].1, env.clone())?.to_string()
                        } else {
                            "assertion failed: expression evaluated to false".to_string()
                        };
                        if let Value::Bool(true) = cond {
                            return Ok(Value::Nil);
                        }
                        return Err(msg);
                    }
                    if name == "assert_false" {
                        if args.is_empty() {
                            return Err(
                                "assert_false requires at least 1 argument (condition)".to_string()
                            );
                        }
                        let cond = self.eval_expr(&args[0].1, env.clone())?;
                        let msg = if args.len() > 1 {
                            self.eval_expr(&args[1].1, env.clone())?.to_string()
                        } else {
                            "assertion failed: expression evaluated to true".to_string()
                        };
                        if let Value::Bool(false) = cond {
                            return Ok(Value::Nil);
                        }
                        return Err(msg);
                    }
                    if name == "assertEq" {
                        if args.len() < 2 {
                            return Err(
                                "assert_eq requires at least 2 arguments (actual, expected)"
                                    .to_string(),
                            );
                        }
                        let act = self.eval_expr(&args[0].1, env.clone())?;
                        let exp = self.eval_expr(&args[1].1, env.clone())?;
                        if act.is_equal(&exp) {
                            return Ok(Value::Nil);
                        }
                        let msg = if args.len() > 2 {
                            format!(": {}", self.eval_expr(&args[2].1, env.clone())?.to_string())
                        } else {
                            "".to_string()
                        };
                        return Err(format!(
                            "Assertion failed: expected {}, got {}{}",
                            exp.to_string(),
                            act.to_string(),
                            msg
                        ));
                    }
                    if name == "assertNe" {
                        if args.len() < 2 {
                            return Err(
                                "assert_ne requires at least 2 arguments (actual, unexpected)"
                                    .to_string(),
                            );
                        }
                        let act = self.eval_expr(&args[0].1, env.clone())?;
                        let exp = self.eval_expr(&args[1].1, env.clone())?;
                        if !act.is_equal(&exp) {
                            return Ok(Value::Nil);
                        }
                        let msg = if args.len() > 2 {
                            format!(": {}", self.eval_expr(&args[2].1, env.clone())?.to_string())
                        } else {
                            "".to_string()
                        };
                        return Err(format!(
                            "Assertion failed: expected values to differ, but both were {}{}",
                            act.to_string(),
                            msg
                        ));
                    }
                    if name == "mock_data" {
                        let schema = if !args.is_empty() {
                            let val = self.eval_expr(&args[0].1, env.clone())?;
                            match val {
                                Value::String(s) => s.to_lowercase(),
                                _ => val.to_string().to_lowercase(),
                            }
                        } else {
                            "default".to_string()
                        };
                        let mut map = HashMap::new();
                        if schema.contains("user") {
                            map.insert("id".to_string(), Value::Int(1001));
                            map.insert("name".to_string(), Value::String("Alex Flame".to_string()));
                            map.insert(
                                "email".to_string(),
                                Value::String("alex@flamelang.org".to_string()),
                            );
                            map.insert("role".to_string(), Value::String("admin".to_string()));
                            map.insert("active".to_string(), Value::Bool(true));
                        } else if schema.contains("post") || schema.contains("article") {
                            map.insert("id".to_string(), Value::Int(505));
                            map.insert(
                                "title".to_string(),
                                Value::String("Flame Performance Guidance".to_string()),
                            );
                            map.insert(
                                "content".to_string(),
                                Value::String("High throughput server processing...".to_string()),
                            );
                            map.insert("views".to_string(), Value::Int(42));
                            map.insert("published".to_string(), Value::Bool(true));
                        } else if schema.contains("product") || schema.contains("item") {
                            map.insert("id".to_string(), Value::Int(9900));
                            map.insert(
                                "name".to_string(),
                                Value::String("Flame Engine v0.1.5".to_string()),
                            );
                            map.insert("price".to_string(), Value::Float(199.99));
                            map.insert("in_stock".to_string(), Value::Bool(true));
                        } else {
                            map.insert("mock_type".to_string(), Value::String(schema));
                            map.insert("status".to_string(), Value::String("OK".to_string()));
                            map.insert("code".to_string(), Value::Int(200));
                        }
                        return Ok(Value::Formula(map));
                    }
                    if name == "mock_api" {
                        let url = if !args.is_empty() {
                            self.eval_expr(&args[0].1, env.clone())?.to_string()
                        } else {
                            "*".to_string()
                        };
                        let body = if args.len() > 1 {
                            self.eval_expr(&args[1].1, env.clone())?.to_string()
                        } else {
                            "{\"status\": \"ok\"}".to_string()
                        };
                        let status = if args.len() > 2 {
                            if let Value::Int(i) = self.eval_expr(&args[2].1, env.clone())? {
                                i
                            } else {
                                200
                            }
                        } else {
                            200
                        };
                        let mut map = HashMap::new();
                        map.insert("url".to_string(), Value::String(url));
                        map.insert("body".to_string(), Value::String(body));
                        map.insert("status".to_string(), Value::Int(status));
                        map.insert("ok".to_string(), Value::Bool(status >= 200 && status < 300));
                        return Ok(Value::Formula(map));
                    }
                    if name == "mock_function" {
                        if args.len() < 2 {
                            return Err("mock_function requires 2 arguments (function_name: String, return_value: Any)".to_string());
                        }
                        let fn_name = self.eval_expr(&args[0].1, env.clone())?.to_string();
                        let ret_val = self.eval_expr(&args[1].1, env.clone())?;
                        env.lock().unwrap().define(fn_name, ret_val, false);
                        return Ok(Value::Nil);
                    }
                }

                // Safe Method Call Interception
                if let Expr::SafeDot(inner_expr, member, span) = &**callee {
                    let inner_val = self.eval_expr(inner_expr, env.clone())?;
                    if matches!(inner_val, Value::Nil) {
                        return Ok(Value::Nil);
                    }
                    let synthetic_callee =
                        Expr::Dot(inner_expr.clone(), member.clone(), span.clone());
                    return self.eval_expr(
                        &Expr::Call(Box::new(synthetic_callee), args.clone(), span.clone()),
                        env.clone(),
                    );
                }

                // Method / Namespace Member Call Interception
                if let Expr::Dot(inner_expr, member, _) = &**callee {
                    if member == "clone" {
                        if let Expr::Identifier(name, _) = &**inner_expr {
                            if let Some(val) = env.lock().unwrap().get(name) {
                                if let Value::Moved(moved_name) = val {
                                    return Err(format!(
                                        "use of moved value '{}'. Value was moved. Use '&{}' to borrow or '{}.clone()' to copy.",
                                        moved_name, moved_name, moved_name
                                    ));
                                }
                                return Ok(val);
                            }
                        }
                        return self.eval_expr(inner_expr, env.clone());
                    }
                    let mut inner_val = self.eval_expr(inner_expr, env.clone())?;
                    if let Value::RefPath(path, _) = &inner_val {
                        inner_val = self.read_target(env.clone(), path.clone())?;
                    }
                    let receiver_val = inner_val.clone();
                    if let Some(res) =
                        self.eval_explicit_conversion(&receiver_val, member, args, env.clone())
                    {
                        return res;
                    }
                    if let Value::StructConstructor { name, .. }
                    | Value::StructInstance { name, .. } = &inner_val
                    {
                        let func_val_opt = self
                            .modules
                            .get(&format!("impl_{}", name))
                            .and_then(|impl_env| impl_env.lock().unwrap().get(member));
                        if let Some(func_val) = func_val_opt {
                            match func_val {
                                Value::Function {
                                    params,
                                    body,
                                    env: closure_env,
                                    ..
                                } => {
                                    let child_env =
                                        Arc::new(Mutex::new(Env::new_child(closure_env.clone())));

                                    let is_self_method = params.first().map_or(false, |p| {
                                        p.name == "self"
                                            || p.name == "mut self"
                                            || p.name == "self_obj"
                                    });
                                    let mut arg_offset = 0;

                                    if is_self_method {
                                        let first_param = &params[0];
                                        let mut self_val = inner_val.clone();
                                        if first_param.is_mut || first_param.name.contains("mut") {
                                            if let Expr::Identifier(var_name, _) = &**inner_expr {
                                                self_val = Value::RefPath(
                                                    crate::vm::RefPath::Var(
                                                        var_name.clone(),
                                                        env.clone(),
                                                    ),
                                                    true,
                                                );
                                            } else if let Expr::Dot(owner_expr, field_name, _) =
                                                &**inner_expr
                                            {
                                                if let Expr::Identifier(owner_name, _) =
                                                    &**owner_expr
                                                {
                                                    self_val = Value::RefPath(
                                                        crate::vm::RefPath::Field {
                                                            owner: owner_name.clone(),
                                                            member: field_name.clone(),
                                                            env: env.clone(),
                                                        },
                                                        true,
                                                    );
                                                }
                                            }
                                        }
                                        self.bind_param(child_env.clone(), first_param, self_val);
                                        arg_offset = 1;
                                    }

                                    for (i, p) in params.iter().enumerate() {
                                        if i == 0 && is_self_method {
                                            continue;
                                        }
                                        let src_idx = i - arg_offset;
                                        if src_idx < args.len() {
                                            let arg_val =
                                                self.eval_expr(&args[src_idx].1, env.clone())?;
                                            if let Expr::Identifier(src_name, _) = &args[src_idx].1
                                            {
                                                env.lock().unwrap().move_var(src_name);
                                            }
                                            self.bind_param(child_env.clone(), p, arg_val);
                                        } else if let Some(def_expr) = &p.default_val {
                                            if let Ok(val) =
                                                self.eval_expr(def_expr, child_env.clone())
                                            {
                                                self.bind_param(child_env.clone(), p, val);
                                            }
                                        }
                                    }
                                    let mut last_val = Value::Nil;
                                    for stmt in &body {
                                        let res =
                                            self.execute_statement(stmt, child_env.clone())?;
                                        if let Value::Return(ret_val) = res {
                                            return Ok(*ret_val);
                                        }
                                        last_val = res;
                                    }
                                    return Ok(last_val);
                                }
                                Value::NativeCallback(cb) => {
                                    let mut evaled_args = Vec::new();
                                    for (_, arg_expr) in args {
                                        let arg_v = self.eval_expr(arg_expr, env.clone())?;
                                        evaled_args.push(arg_v);
                                    }
                                    return cb(evaled_args);
                                }
                                Value::NativeClosure(crate::vm::NativeClosureType(cb)) => {
                                    let mut evaled_args = Vec::new();
                                    for (_, arg_expr) in args {
                                        let arg_v = self.eval_expr(arg_expr, env.clone())?;
                                        evaled_args.push(arg_v);
                                    }
                                    return cb(evaled_args);
                                }
                                _ => {}
                            }
                        }
                    }
                    if let Some(conv_res) =
                        self.eval_explicit_conversion(&inner_val, member, args, env.clone())
                    {
                        return conv_res;
                    }
                    match inner_val {
                        Value::EnumMeta(enum_name, variants) => {
                            for var in &variants {
                                if let crate::parser::EnumVariant::Tuple(n, _types) = var {
                                    if n == member {
                                        let mut tuple_vals = Vec::new();
                                        for (_, arg_expr) in args {
                                            tuple_vals.push(self.eval_expr(arg_expr, env.clone())?);
                                        }
                                        return Ok(Value::EnumValue(
                                            enum_name.clone(),
                                            n.clone(),
                                            EnumData::Tuple(tuple_vals),
                                        ));
                                    }
                                }
                            }
                            return Err(format!(
                                "tuple variant '{}' not found in enum '{}'",
                                member, enum_name
                            ));
                        }
                        Value::String(ref s) => match member.as_str() {
                            "len" => return Ok(Value::Int(s.len() as i64)),
                            "toUpperCase" => return Ok(Value::String(s.to_uppercase())),
                            "toLowerCase" => return Ok(Value::String(s.to_lowercase())),
                            "trim" => return Ok(Value::String(s.trim().to_string())),
                            "isEmpty" => return Ok(Value::Bool(s.is_empty())),
                            "contains" => {
                                if !args.is_empty() {
                                    let sub = self.eval_expr(&args[0].1, env.clone())?.to_string();
                                    return Ok(Value::Bool(s.contains(&sub)));
                                }
                                return Ok(Value::Bool(false));
                            }
                            "startsWith" => {
                                if !args.is_empty() {
                                    let sub = self.eval_expr(&args[0].1, env.clone())?.to_string();
                                    return Ok(Value::Bool(s.starts_with(&sub)));
                                }
                                return Ok(Value::Bool(false));
                            }
                            "endsWith" => {
                                if !args.is_empty() {
                                    let sub = self.eval_expr(&args[0].1, env.clone())?.to_string();
                                    return Ok(Value::Bool(s.ends_with(&sub)));
                                }
                                return Ok(Value::Bool(false));
                            }
                            "replace" => {
                                if args.len() >= 2 {
                                    let from_s =
                                        self.eval_expr(&args[0].1, env.clone())?.to_string();
                                    let to_s = self.eval_expr(&args[1].1, env.clone())?.to_string();
                                    return Ok(Value::String(s.replace(&from_s, &to_s)));
                                }
                                return Ok(Value::String(s.clone()));
                            }
                            "assertEq" => {
                                if args.len() < 1 {
                                    return Err("assert_eq requires at least 2 arguments (actual, expected)".to_string());
                                }
                                let exp = self.eval_expr(&args[0].1, env.clone())?;
                                if s == &exp.to_string() {
                                    return Ok(Value::Nil);
                                }
                                let msg = if args.len() > 1 {
                                    format!(
                                        ": {}",
                                        self.eval_expr(&args[1].1, env.clone())?.to_string()
                                    )
                                } else {
                                    "".to_string()
                                };
                                return Err(format!(
                                    "Assertion failed: expected {}, got {}{}",
                                    exp.to_string(),
                                    s,
                                    msg
                                ));
                            }
                            "assertNe" => {
                                if args.len() < 1 {
                                    return Err("assert_ne requires at least 2 arguments (actual, unexpected)".to_string());
                                }
                                let exp = self.eval_expr(&args[0].1, env.clone())?;
                                if s != &exp.to_string() {
                                    return Ok(Value::Nil);
                                }
                                let msg = if args.len() > 1 {
                                    format!(
                                        ": {}",
                                        self.eval_expr(&args[1].1, env.clone())?.to_string()
                                    )
                                } else {
                                    "".to_string()
                                };
                                return Err(format!(
                                    "Assertion failed: expected values to differ, but both were {}{}",
                                    s, msg
                                ));
                            }
                            "push_str" | "push" => {
                                if !args.is_empty() {
                                    let val = self.eval_expr(&args[0].1, env.clone())?;
                                    let add_s = match val {
                                        Value::String(sub_s) => sub_s,
                                        other => other.to_string(),
                                    };
                                    if let Expr::Identifier(var_name, _) = &**inner_expr {
                                        let mut new_s = s.clone();
                                        new_s.push_str(&add_s);
                                        env.lock()
                                            .unwrap()
                                            .assign(var_name.clone(), Value::String(new_s))?;
                                    }
                                }
                                return Ok(Value::Nil);
                            }
                            _ => {}
                        },
                        Value::Tuple(ref vec) => match member.as_str() {
                            "len" => return Ok(Value::Int(vec.len() as i64)),
                            "isEmpty" => return Ok(Value::Bool(vec.is_empty())),
                            "push" => {
                                if !args.is_empty() {
                                    let val = self.eval_expr(&args[0].1, env.clone())?;
                                    if let Expr::Identifier(var_name, _) = &**inner_expr {
                                        let mut new_vec = vec.clone();
                                        new_vec.push(val);
                                        env.lock()
                                            .unwrap()
                                            .assign(var_name.clone(), Value::Tuple(new_vec))?;
                                    }
                                }
                                return Ok(Value::Nil);
                            }
                            "pop" => {
                                if let Expr::Identifier(var_name, _) = &**inner_expr {
                                    let mut new_vec = vec.clone();
                                    let popped = new_vec.pop().unwrap_or(Value::Nil);
                                    env.lock()
                                        .unwrap()
                                        .assign(var_name.clone(), Value::Tuple(new_vec))?;
                                    return Ok(popped);
                                }
                                return Ok(Value::Nil);
                            }
                            "filter" => {
                                if !args.is_empty() {
                                    let cb_val = self.eval_expr(&args[0].1, env.clone())?;
                                    if let Value::Function {
                                        params,
                                        body,
                                        env: closure_env,
                                        ..
                                    } = cb_val
                                    {
                                        let mut res = Vec::new();
                                        for item in vec {
                                            let child_env = Arc::new(Mutex::new(Env::new_child(
                                                closure_env.clone(),
                                            )));
                                            if !params.is_empty() {
                                                self.bind_param(
                                                    child_env.clone(),
                                                    &params[0],
                                                    item.clone(),
                                                );
                                            }
                                            let mut matched = false;
                                            for stmt in &body {
                                                let stmt_res = self
                                                    .execute_statement(stmt, child_env.clone())?;
                                                if let Value::Return(ret_val) = stmt_res {
                                                    if let Value::Bool(b) = *ret_val {
                                                        matched = b;
                                                    }
                                                    break;
                                                }
                                            }
                                            if matched {
                                                res.push(item.clone());
                                            }
                                        }
                                        return Ok(Value::Tuple(res));
                                    }
                                }
                                return Ok(Value::Tuple(vec.clone()));
                            }
                            "map" => {
                                if !args.is_empty() {
                                    let cb_val = self.eval_expr(&args[0].1, env.clone())?;
                                    if let Value::Function {
                                        params,
                                        body,
                                        env: closure_env,
                                        ..
                                    } = cb_val
                                    {
                                        let mut res = Vec::new();
                                        for item in vec {
                                            let child_env = Arc::new(Mutex::new(Env::new_child(
                                                closure_env.clone(),
                                            )));
                                            if !params.is_empty() {
                                                self.bind_param(
                                                    child_env.clone(),
                                                    &params[0],
                                                    item.clone(),
                                                );
                                            }
                                            let mut map_res = Value::Nil;
                                            for stmt in &body {
                                                let stmt_res = self
                                                    .execute_statement(stmt, child_env.clone())?;
                                                if let Value::Return(ret_val) = stmt_res {
                                                    map_res = *ret_val;
                                                    break;
                                                }
                                                map_res = stmt_res;
                                            }
                                            res.push(map_res);
                                        }
                                        return Ok(Value::Tuple(res));
                                    }
                                }
                                return Ok(Value::Tuple(vec.clone()));
                            }
                            _ => {}
                        },
                        Value::Bytes(ref bytes) => match member.as_str() {
                            "len" => return Ok(Value::Int(bytes.len() as i64)),
                            "isEmpty" | "is_empty" => return Ok(Value::Bool(bytes.is_empty())),
                            "toString" | "to_string" => return Ok(Value::String(String::from_utf8_lossy(bytes).into_owned())),
                            "toHex" | "to_hex" => return Ok(Value::String(bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>())),
                            "get" => {
                                if !args.is_empty() {
                                    let idx_val = self.eval_expr(&args[0].1, env.clone())?;
                                    if let Value::Int(idx) = idx_val {
                                        if idx >= 0 && (idx as usize) < bytes.len() {
                                            return Ok(Value::Byte(bytes[idx as usize]));
                                        }
                                    }
                                }
                                return Ok(Value::Nil);
                            }
                            _ => {}
                        },
                        Value::Byte(ref b) => match member.as_str() {
                            "toInt" | "to_int" => return Ok(Value::Int(*b as i64)),
                            "toString" | "to_string" => return Ok(Value::String(format!("{}", *b as char))),
                            "toHex" | "to_hex" => return Ok(Value::String(format!("{:02x}", b))),
                            _ => {}
                        },
                        Value::ThreadHandler(id) => {
                            if member == "join" {
                                let mut registry = get_threads().lock().unwrap();
                                if let Some(handle) = registry.remove(&id) {
                                    let result = handle.join().unwrap();
                                    return Ok(result);
                                }
                                return Ok(Value::Nil);
                            }
                        }
                        Value::Sender(id) => {
                            if member == "send" {
                                if !args.is_empty() {
                                    let val = self.eval_expr(&args[0].1, env.clone())?;
                                    let registry = get_channels().lock().unwrap();
                                    if let Some(tx) = registry.get(&id) {
                                        let _ = tx.send(val);
                                    }
                                }
                                return Ok(Value::Nil);
                            } else if member == "clone" {
                                let registry = get_channels().lock().unwrap();
                                if let Some(tx) = registry.get(&id) {
                                    let tx_cloned = tx.clone();
                                    drop(registry);
                                    let mut counter = get_channel_counter().lock().unwrap();
                                    *counter += 1;
                                    let new_id = *counter;
                                    get_channels().lock().unwrap().insert(new_id, tx_cloned);
                                    return Ok(Value::Sender(new_id));
                                }
                                return Ok(Value::Sender(id));
                            }
                        }
                        Value::Receiver(id) => {
                            if member == "recv" {
                                let rx_opt = {
                                    let registry = get_receivers().lock().unwrap();
                                    registry.get(&id).cloned()
                                };
                                if let Some(rx) = rx_opt {
                                    let val = rx.lock().unwrap().recv().unwrap_or(Value::Nil);
                                    return Ok(val);
                                }
                                return Ok(Value::Nil);
                            } else if member == "tryRecv" || member == "try_recv" {
                                let rx_opt = {
                                    let registry = get_receivers().lock().unwrap();
                                    registry.get(&id).cloned()
                                };
                                if let Some(rx) = rx_opt {
                                    match rx.lock().unwrap().try_recv() {
                                        Ok(val) => return Ok(val),
                                        Err(_) => return Ok(Value::Nil),
                                    }
                                }
                                return Ok(Value::Nil);
                            } else if member == "recvTimeout" || member == "recv_timeout" {
                                let ms = if !args.is_empty() {
                                    match self.eval_expr(&args[0].1, env.clone())? {
                                        Value::Int(i) => i.max(0) as u64,
                                        Value::Float(f) => f.max(0.0) as u64,
                                        _ => 0,
                                    }
                                } else {
                                    0
                                };
                                let rx_opt = {
                                    let registry = get_receivers().lock().unwrap();
                                    registry.get(&id).cloned()
                                };
                                if let Some(rx) = rx_opt {
                                    match rx.lock().unwrap().recv_timeout(std::time::Duration::from_millis(ms)) {
                                        Ok(val) => return Ok(val),
                                        Err(_) => return Ok(Value::Nil),
                                    }
                                }
                                return Ok(Value::Nil);
                            }
                        }
                        Value::CommandBuilder {
                            program,
                            args: mut builder_args,
                        } => {
                            if member == "args" {
                                if !args.is_empty() {
                                    let list_val = self.eval_expr(&args[0].1, env.clone())?;
                                    if let Value::Tuple(items) = list_val {
                                        for it in items {
                                            builder_args.push(it.to_string());
                                        }
                                    }
                                }
                                return Ok(Value::CommandBuilder {
                                    program,
                                    args: builder_args,
                                });
                            } else if member == "spawn" {
                                let mut command = std::process::Command::new(&program);
                                command.args(&builder_args);
                                let child = command
                                    .spawn()
                                    .map_err(|e| format!("failed to spawn '{}': {}", program, e))?;
                                let mut counter = get_child_process_counter().lock().unwrap();
                                *counter += 1;
                                let child_id = *counter;
                                get_child_processes()
                                    .lock()
                                    .unwrap()
                                    .insert(child_id, child);
                                return Ok(Value::ChildProcess(child_id));
                            }
                        }
                        Value::ChildProcess(pid) => {
                            if member == "wait_with_output" {
                                let child = get_child_processes().lock().unwrap().remove(&pid);
                                if let Some(child) = child {
                                    let output = child.wait_with_output().map_err(|e| {
                                        format!("failed to wait for child process: {}", e)
                                    })?;
                                    let mut output_map = HashMap::new();
                                    output_map.insert(
                                        "stdout".to_string(),
                                        Value::String(
                                            String::from_utf8_lossy(&output.stdout).to_string(),
                                        ),
                                    );
                                    output_map.insert(
                                        "stderr".to_string(),
                                        Value::String(
                                            String::from_utf8_lossy(&output.stderr).to_string(),
                                        ),
                                    );

                                    let mut status_map = HashMap::new();
                                    status_map.insert(
                                        "code".to_string(),
                                        Value::Int(output.status.code().unwrap_or(-1) as i64),
                                    );
                                    output_map
                                        .insert("status".to_string(), Value::Formula(status_map));

                                    return Ok(Value::Formula(output_map));
                                }
                                return Err(format!("child process {} is not active", pid));
                            }
                        }
                        Value::NativeObject {
                            ref crate_name,
                            ref type_name,
                            ptr: _,
                        } => {
                            let mut c_args = Vec::new();
                            c_args.push(inner_val.pack());
                            for (_, arg_expr) in args {
                                let mut arg_v = self.eval_expr(arg_expr, env.clone())?;
                                if let Value::RefPath(path, _) = &arg_v {
                                    arg_v = self.read_target(env.clone(), path.clone())?;
                                }
                                c_args.push(arg_v.pack());
                            }

                            let sym_str1 = format!("flame_{}_{}_{}", crate_name, type_name, member);
                            let sym_str2 = format!("flame_{}_{}", crate_name, member);

                            let func = self
                                .native_methods
                                .get(&sym_str1)
                                .or_else(|| self.native_methods.get(&sym_str2))
                                .or_else(|| {
                                    let prefix = format!("flame_{}_", crate_name);
                                    let suffix = format!("_{}", member);
                                    self.native_methods
                                        .iter()
                                        .find(|(k, _)| {
                                            k.starts_with(&prefix) && k.ends_with(&suffix)
                                        })
                                        .map(|(_, f)| f)
                                });

                            if let Some(func) = func {
                                let res = func(c_args.as_ptr(), c_args.len());
                                for c_val in c_args {
                                    if c_val.tag == crate::runner::CValueTag::String
                                        && !c_val.string_ptr.is_null()
                                    {
                                        unsafe {
                                            let _ = std::ffi::CString::from_raw(c_val.string_ptr);
                                        }
                                    }
                                }
                                let mut ret_type = type_name.clone();
                                if let Some(mod_env) = self.modules.get(crate_name) {
                                    if let Some(Value::String(rt)) = mod_env
                                        .lock()
                                        .unwrap()
                                        .get(&format!("__{}_{}_return_type__", type_name, member))
                                    {
                                        ret_type = rt.clone();
                                    }
                                }
                                return Ok(Value::unpack(res, crate_name, &ret_type));
                            }

                            return Err(format!(
                                "NativeObject method '{}.{}' not found in static registry (tried {} and {})",
                                type_name, member, sym_str1, sym_str2
                            ));
                        }
                        Value::RustServer { port } => {
                            if member == "bind" {
                                if std::env::var("FLAME_VERBOSE").is_ok()
                                    || std::env::var("FLAME_DEV").is_ok()
                                {
                                    println!("Bound server to http://127.0.0.1:{}", port);
                                }
                                return Ok(Value::RustServer { port });
                            } else if member == "listen" || member == "start" {
                                let mut listen_port = port;
                                if let Some((_, arg_expr)) = args.first() {
                                    if let Value::Int(p) = self.eval_expr(arg_expr, env.clone())? {
                                        listen_port = p as u16;
                                    }
                                }
                                let addr = format!("127.0.0.1:{}", listen_port);
                                match std::net::TcpListener::bind(&addr) {
                                    Ok(listener) => {
                                        println!("Server listening on http://{}", addr);
                                        for stream in listener.incoming() {
                                            if let Ok(mut stream) = stream {
                                                use std::io::{Read, Write};
                                                let mut buf = [0u8; 1024];
                                                let _ = stream.read(&mut buf);
                                                let json_body = r#"{"status": "ok", "message": "Hello from Flame HTTP Router Server"}"#;
                                                let response = format!(
                                                    "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                                    json_body.len(),
                                                    json_body
                                                );
                                                let _ = stream.write_all(response.as_bytes());
                                                let _ = stream.flush();
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        return Err(format!(
                                            "Failed to bind server to {}: {}",
                                            addr, e
                                        ));
                                    }
                                }
                                return Ok(Value::Nil);
                            } else if member == "get" {
                                return Ok(Value::RustServer { port });
                            } else if member == "accept" {
                                if std::env::var("FLAME_VERBOSE").is_ok()
                                    || std::env::var("FLAME_DEV").is_ok()
                                {
                                    println!("Accepted connection on http://127.0.0.1:{}", port);
                                }
                                return Ok(Value::String("accepted".to_string()));
                            } else if member == "stop" || member == "close" {
                                if std::env::var("FLAME_VERBOSE").is_ok()
                                    || std::env::var("FLAME_DEV").is_ok()
                                {
                                    println!("Server stopped on port {}.", port);
                                }
                                return Ok(Value::Nil);
                            }
                        }
                        Value::Formula(ref map) => {
                            if !map.contains_key("__module__") && !map.contains_key("__crate__") {
                                match member.as_str() {
                                    "len" => return Ok(Value::Int(map.len() as i64)),
                                    "insert" => {
                                        if args.len() >= 2 {
                                            let k_val = self.eval_expr(&args[0].1, env.clone())?;
                                            let v_val = self.eval_expr(&args[1].1, env.clone())?;
                                            if let Expr::Identifier(var_name, _) = &**inner_expr {
                                                let mut new_map = map.clone();
                                                new_map.insert(k_val.to_string(), v_val);
                                                env.lock().unwrap().assign(
                                                    var_name.clone(),
                                                    Value::Formula(new_map),
                                                )?;
                                            }
                                        }
                                        return Ok(Value::Nil);
                                    }
                                    "get" if !map.contains_key("get") => {
                                        if !args.is_empty() {
                                            let k_val = self.eval_expr(&args[0].1, env.clone())?;
                                            if let Some(v) = map.get(&k_val.to_string()) {
                                                return Ok(v.clone());
                                            }
                                        }
                                        return Ok(Value::Nil);
                                    }
                                    "remove" => {
                                        if !args.is_empty() {
                                            let k_val = self.eval_expr(&args[0].1, env.clone())?;
                                            if let Expr::Identifier(var_name, _) = &**inner_expr {
                                                let mut new_map = map.clone();
                                                let removed = new_map
                                                    .remove(&k_val.to_string())
                                                    .unwrap_or(Value::Nil);
                                                env.lock().unwrap().assign(
                                                    var_name.clone(),
                                                    Value::Formula(new_map),
                                                )?;
                                                return Ok(removed);
                                            }
                                        }
                                        return Ok(Value::Nil);
                                    }
                                    _ => {}
                                }
                            }
                            let crate_binding =
                                if let Some(Value::String(c_name)) = map.get("__crate__") {
                                    c_name.clone()
                                } else if let Expr::Identifier(namespace, _) = &**inner_expr {
                                    namespace
                                        .strip_prefix("native.")
                                        .unwrap_or(namespace.as_str())
                                        .to_string()
                                } else {
                                    String::new()
                                };

                            let namespace = if let Expr::Identifier(ns, _) = &**inner_expr {
                                ns.as_str()
                            } else {
                                ""
                            };
                            if (namespace == "thread"
                                || namespace == "std.thread"
                                || map.contains_key("sleep"))
                                && member == "spawn"
                            {
                                if args.is_empty() {
                                    return Err(
                                        "thread.spawn expects 1 argument (function or callback)"
                                            .to_string(),
                                    );
                                }
                                let fn_val = self.eval_expr(&args[0].1, env.clone())?;
                                let snapshot_env =
                                    Arc::new(Mutex::new(env.lock().unwrap().snapshot()));
                                let mut runner = self.clone_for_thread(snapshot_env.clone());

                                let mut counter = get_thread_counter().lock().unwrap();
                                *counter += 1;
                                let id = *counter;

                                let handle = thread::spawn(move || match fn_val {
                                    Value::Function { body, .. } => {
                                        let mut last_val = Value::Nil;
                                        for stmt in &body {
                                            match runner
                                                .execute_statement(stmt, snapshot_env.clone())
                                            {
                                                Ok(v) => last_val = v,
                                                Err(e) => {
                                                    eprintln!("Thread error: {}", e);
                                                    return Value::Nil;
                                                }
                                            }
                                        }
                                        last_val
                                    }
                                    Value::NativeCallback(cb) => cb(vec![]).unwrap_or(Value::Nil),
                                    Value::NativeClosure(crate::vm::NativeClosureType(cb)) => {
                                        cb(vec![]).unwrap_or(Value::Nil)
                                    }
                                    _ => Value::Nil,
                                });

                                get_threads().lock().unwrap().insert(id, handle);
                                return Ok(Value::ThreadHandler(id));
                            } else if (namespace == "thread_bridge" && member == "create_channel")
                                || (namespace == "thread" && member == "channel")
                            {
                                let mut counter = get_channel_counter().lock().unwrap();
                                *counter += 1;
                                let chan_id = *counter;

                                let (tx, rx) = std::sync::mpsc::channel();
                                get_channels().lock().unwrap().insert(chan_id, tx);
                                get_receivers()
                                    .lock()
                                    .unwrap()
                                    .insert(chan_id, Arc::new(Mutex::new(rx)));

                                return Ok(Value::Tuple(vec![
                                    Value::Sender(chan_id),
                                    Value::Receiver(chan_id),
                                ]));
                            } else if (namespace == "thread" || namespace == "std.thread" || map.contains_key("sleep"))
                                && (member == "yield" || member == "yield_now")
                            {
                                std::thread::yield_now();
                                return Ok(Value::Nil);
                            } else if namespace == "process_bridge" && member == "cmd" {
                                let mut prog = String::new();
                                if !args.is_empty() {
                                    prog = self.eval_expr(&args[0].1, env.clone())?.to_string();
                                }
                                return Ok(Value::CommandBuilder {
                                    program: prog,
                                    args: vec![],
                                });
                            } else if namespace == "fs_bridge" && member == "read_file" {
                                let mut path_str = String::new();
                                if !args.is_empty() {
                                    path_str = self.eval_expr(&args[0].1, env.clone())?.to_string();
                                }
                                let abs_path = self.resolve_path(path_str.trim_matches('"'));
                                if let Ok(c) = fs::read_to_string(abs_path) {
                                    return Ok(Value::String(c));
                                } else {
                                    return Err("File not found".to_string());
                                }
                            } else if namespace == "fs_bridge" && member == "write_file" {
                                let mut path_str = String::new();
                                let mut content_str = String::new();
                                if args.len() >= 2 {
                                    path_str = self
                                        .eval_expr(&args[0].1, env.clone())?
                                        .to_string()
                                        .trim_matches('"')
                                        .to_string();
                                    content_str = self
                                        .eval_expr(&args[1].1, env.clone())?
                                        .to_string()
                                        .trim_matches('"')
                                        .to_string();
                                }
                                let abs_path = self.resolve_path(&path_str);
                                if fs::write(abs_path, content_str).is_ok() {
                                    return Ok(Value::Nil);
                                } else {
                                    return Err("Failed to write file".to_string());
                                }
                            } else if namespace == "fs_bridge" && member == "create_dir" {
                                let path_str = self
                                    .eval_expr(&args[0].1, env.clone())?
                                    .to_string()
                                    .trim_matches('"')
                                    .to_string();
                                let abs_path = self.resolve_path(&path_str);
                                if fs::create_dir_all(abs_path).is_ok() {
                                    return Ok(Value::Nil);
                                } else {
                                    return Err("Failed to create directory".to_string());
                                }
                            } else if namespace == "fs_bridge" && member == "remove_file" {
                                let path_str = self
                                    .eval_expr(&args[0].1, env.clone())?
                                    .to_string()
                                    .trim_matches('"')
                                    .to_string();
                                let abs_path = self.resolve_path(&path_str);
                                if fs::remove_file(abs_path).is_ok() {
                                    return Ok(Value::Nil);
                                } else {
                                    return Err("Failed to remove file".to_string());
                                }
                            } else if namespace == "fs_bridge" && member == "create_file" {
                                let path_str = self
                                    .eval_expr(&args[0].1, env.clone())?
                                    .to_string()
                                    .trim_matches('"')
                                    .to_string();
                                let content_str = self
                                    .eval_expr(&args[1].1, env.clone())?
                                    .to_string()
                                    .trim_matches('"')
                                    .to_string();
                                let abs_path = self.resolve_path(&path_str);
                                if fs::File::create(&abs_path).is_ok() {
                                    if fs::write(&abs_path, &content_str).is_ok() {
                                        return Ok(Value::Nil);
                                    } else {
                                        return Err("Failed to write file".to_string());
                                    }
                                } else {
                                    return Err("Failed to create file".to_string());
                                }
                            } else if namespace == "fs_bridge" && member == "remove_dir" {
                                let path_str = self
                                    .eval_expr(&args[0].1, env.clone())?
                                    .to_string()
                                    .trim_matches('"')
                                    .to_string();
                                let abs_path = self.resolve_path(&path_str);
                                if fs::remove_dir_all(abs_path).is_ok() {
                                    return Ok(Value::Nil);
                                } else {
                                    return Err("Failed to remove directory".to_string());
                                }
                            } else if namespace == "fs_bridge" && member == "exists" {
                                let path_str = self
                                    .eval_expr(&args[0].1, env.clone())?
                                    .to_string()
                                    .trim_matches('"')
                                    .to_string();
                                let abs_path = self.resolve_path(&path_str);
                                return Ok(Value::Bool(abs_path.exists()));
                            } else {
                                // Universal Native Crate Interop Resolution
                                let raw_ns_str = if !crate_binding.is_empty() {
                                    crate_binding.clone()
                                } else {
                                    namespace
                                        .strip_prefix("native.")
                                        .unwrap_or(namespace)
                                        .to_string()
                                };
                                let raw_ns = raw_ns_str.as_str();
                                let mut c_args = Vec::new();
                                for (_, arg_expr) in args {
                                    let mut arg_v = self.eval_expr(arg_expr, env.clone())?;
                                    if let Value::RefPath(path, _) = &arg_v {
                                        arg_v = self.read_target(env.clone(), path.clone())?;
                                    }
                                    c_args.push(arg_v.pack());
                                }

                                let parent_type = map
                                    .get("__type__")
                                    .or_else(|| map.get("__member__"))
                                    .and_then(|v| {
                                        if let Value::String(s) = v {
                                            Some(s.clone())
                                        } else {
                                            None
                                        }
                                    });

                                let mut symbol_candidates =
                                    vec![format!("flame_{}_{}", raw_ns, member)];
                                if let Some(pt) = &parent_type {
                                    symbol_candidates
                                        .push(format!("flame_{}_{}_{}", raw_ns, pt, member));
                                }

                                for sym_str in symbol_candidates {
                                    if let Some(func) = self.native_methods.get(&sym_str) {
                                        let res = func(c_args.as_ptr(), c_args.len());

                                        for c_val in c_args {
                                            if c_val.tag == crate::runner::CValueTag::String
                                                && !c_val.string_ptr.is_null()
                                            {
                                                unsafe {
                                                    let _ = std::ffi::CString::from_raw(
                                                        c_val.string_ptr,
                                                    );
                                                }
                                            }
                                        }

                                        let mut t_name =
                                            parent_type.unwrap_or_else(|| member.clone());
                                        if let Some(Value::String(rt)) =
                                            map.get(&format!("__{}_return_type__", member))
                                        {
                                            t_name = rt.clone();
                                        }
                                        return Ok(Value::unpack(res, raw_ns, &t_name));
                                    }
                                }
                            }
                            if let Some(val) = map.get(member) {
                                if let Value::Function { params, body, .. } = val {
                                    if body.is_empty() {
                                        let mut evaled_args = Vec::new();
                                        for (_, arg_expr) in args {
                                            let arg_v = self.eval_expr(arg_expr, env.clone())?;
                                            if let Expr::Identifier(src_name, _) = arg_expr {
                                                env.lock().unwrap().move_var(src_name);
                                            }
                                            evaled_args.push(
                                                arg_v.to_string().trim_matches('"').to_string(),
                                            );
                                        }
                                        let args_str = evaled_args.join(", ");
                                        let res_str = if args_str.is_empty() {
                                            format!("{}()", member)
                                        } else {
                                            format!("{}({})", member, args_str)
                                        };
                                        return Ok(Value::String(res_str));
                                    }
                                    let child_env =
                                        Arc::new(Mutex::new(Env::new_child(env.clone())));
                                    let is_self_method = params.first().map_or(false, |p| {
                                        p.name == "self"
                                            || p.name == "&self"
                                            || p.name == "&mut self"
                                            || p.type_name == "Self"
                                            || p.type_name == "&Self"
                                            || p.type_name == "&mut Self"
                                    });
                                    let mut self_val = inner_val.clone();
                                    if let Expr::Identifier(var_name, _) = &**inner_expr {
                                        self_val = Value::RefPath(
                                            crate::vm::RefPath::Var(var_name.clone(), env.clone()),
                                            true,
                                        );
                                    } else if let Expr::Dot(owner_expr, field_name, _) =
                                        &**inner_expr
                                    {
                                        if let Expr::Identifier(owner_name, _) = &**owner_expr {
                                            self_val = Value::RefPath(
                                                crate::vm::RefPath::Field {
                                                    owner: owner_name.clone(),
                                                    member: field_name.clone(),
                                                    env: env.clone(),
                                                },
                                                true,
                                            );
                                        }
                                    }
                                    if is_self_method {
                                        let p0 = &params[0];
                                        self.bind_param(child_env.clone(), p0, self_val.clone());
                                    } else {
                                        child_env.lock().unwrap().define(
                                            "self".to_string(),
                                            self_val,
                                            true,
                                        );
                                    }
                                    let param_offset = if is_self_method { 1 } else { 0 };
                                    for (i, p) in params.iter().enumerate().skip(param_offset) {
                                        let arg_idx = i - param_offset;
                                        if arg_idx < args.len() {
                                            let arg_val =
                                                self.eval_expr(&args[arg_idx].1, env.clone())?;
                                            if let Expr::Identifier(src_name, _) = &args[arg_idx].1
                                            {
                                                env.lock().unwrap().move_var(src_name);
                                            }
                                            self.bind_param(child_env.clone(), p, arg_val);
                                        } else if let Some(def_expr) = &p.default_val {
                                            if let Ok(val) =
                                                self.eval_expr(def_expr, child_env.clone())
                                            {
                                                self.bind_param(child_env.clone(), p, val);
                                            }
                                        }
                                    }
                                    let mut last_val = Value::Nil;
                                    for stmt in body {
                                        let res =
                                            self.execute_statement(stmt, child_env.clone())?;
                                        if let Value::Return(ret_val) = res {
                                            return Ok(*ret_val);
                                        }
                                        last_val = res;
                                    }
                                    return Ok(last_val);
                                }
                                if let Value::NativeCallback(cb) = val {
                                    let mut evaled_args = Vec::new();
                                    if !map.contains_key("__module__") {
                                        evaled_args.push(inner_val.clone());
                                    }
                                    for (_, arg_expr) in args {
                                        evaled_args.push(self.eval_expr(arg_expr, env.clone())?);
                                    }
                                    return cb(evaled_args);
                                } else if let Value::NativeClosure(crate::vm::NativeClosureType(
                                    cb,
                                )) = val
                                {
                                    let mut evaled_args = Vec::new();
                                    if !map.contains_key("__module__") {
                                        evaled_args.push(inner_val.clone());
                                    }
                                    for (_, arg_expr) in args {
                                        evaled_args.push(self.eval_expr(arg_expr, env.clone())?);
                                    }
                                    return cb(evaled_args);
                                }
                                return Ok(val.clone());
                            }
                        }
                        _ => {}
                    }

                    // Support calling common builtins as methods on values, e.g. `val.assert_eq(expected)`
                    if member == "assertEq" {
                        if args.len() < 1 {
                            return Err(
                                "assert_eq requires at least 2 arguments (actual, expected)"
                                    .to_string(),
                            );
                        }
                        let act = receiver_val.clone();
                        let exp = self.eval_expr(&args[0].1, env.clone())?;
                        if act.to_string() == exp.to_string() {
                            return Ok(Value::Nil);
                        }
                        let msg = if args.len() > 1 {
                            format!(": {}", self.eval_expr(&args[1].1, env.clone())?.to_string())
                        } else {
                            "".to_string()
                        };
                        return Err(format!(
                            "Assertion failed: expected {}, got {}{}",
                            exp.to_string(),
                            act.to_string(),
                            msg
                        ));
                    } else if member == "assertNe" {
                        if args.len() < 1 {
                            return Err(
                                "assert_ne requires at least 2 arguments (actual, unexpected)"
                                    .to_string(),
                            );
                        }
                        let act = receiver_val.clone();
                        let exp = self.eval_expr(&args[0].1, env.clone())?;
                        if act.to_string() != exp.to_string() {
                            return Ok(Value::Nil);
                        }
                        let msg = if args.len() > 1 {
                            format!(": {}", self.eval_expr(&args[1].1, env.clone())?.to_string())
                        } else {
                            "".to_string()
                        };
                        return Err(format!(
                            "Assertion failed: expected values to differ, but both were {}{}",
                            act.to_string(),
                            msg
                        ));
                    } else if member == "assert" || member == "assert_true" {
                        if args.len() < 1 {
                            return Err(
                                "assert requires at least 1 argument (condition)".to_string()
                            );
                        }
                        let cond = self.eval_expr(&args[0].1, env.clone())?;
                        let msg = if args.len() > 1 {
                            self.eval_expr(&args[1].1, env.clone())?.to_string()
                        } else {
                            "assertion failed: expression evaluated to false".to_string()
                        };
                        if let Value::Bool(true) = cond {
                            return Ok(Value::Nil);
                        }
                        return Err(msg);
                    } else if member == "type" {
                        let type_name = if let Value::StructInstance { name, .. } = &receiver_val {
                            name.clone()
                        } else {
                            receiver_val.type_name().to_string()
                        };
                        return Ok(Value::String(type_name));
                    } else if member == "toString" {
                        if let Value::Bytes(b) = &receiver_val {
                            return Ok(Value::String(String::from_utf8_lossy(b).into_owned()));
                        }
                        if let Value::Byte(b) = &receiver_val {
                            return Ok(Value::String(format!("{}", *b as char)));
                        }
                        if let Value::Object(map) | Value::Formula(map) = &receiver_val {
                            if map.contains_key("toString") {
                                let func_val = self.eval_expr(callee, env.clone())?;
                                match func_val {
                                    Value::NativeClosure(crate::vm::NativeClosureType(cb)) => {
                                        let mut evaled_args = Vec::new();
                                        for (_, arg_expr) in args {
                                            evaled_args
                                                .push(self.eval_expr(arg_expr, env.clone())?);
                                        }
                                        return cb(evaled_args);
                                    }
                                    _ => return Ok(Value::String(receiver_val.to_string())),
                                }
                            }
                        }
                        return Ok(Value::String(receiver_val.to_string()));
                    } else if member == "toHex" {
                        if let Value::Bytes(b) = &receiver_val {
                            return Ok(Value::String(b.iter().map(|byte| format!("{:02x}", byte)).collect::<String>()));
                        }
                        if let Value::Byte(b) = &receiver_val {
                            return Ok(Value::String(format!("{:02x}", b)));
                        }
                        return Err(format!("toHex is not supported on {}", receiver_val.type_name()));
                    } else if member == "toInt" {
                        if let Value::Byte(b) = &receiver_val {
                            return Ok(Value::Int(*b as i64));
                        }
                        if let Ok(i) = receiver_val.as_int() {
                            return Ok(Value::Int(i));
                        } else if let Value::String(s) = &receiver_val {
                            if let Ok(i) = s.parse::<i64>() {
                                return Ok(Value::Int(i));
                            }
                        } else if let Value::Float(f) = receiver_val {
                            return Ok(Value::Int(f as i64));
                        }
                        return Err(format!(
                            "Cannot convert {} to Int",
                            receiver_val.type_name()
                        ));
                    } else if member == "tryInt" {
                        if let Ok(i) = receiver_val.as_int() {
                            return Ok(Value::Int(i));
                        } else if let Value::String(s) = &receiver_val {
                            if let Ok(i) = s.parse::<i64>() {
                                return Ok(Value::Int(i));
                            }
                        } else if let Value::Float(f) = receiver_val {
                            return Ok(Value::Int(f as i64));
                        }
                        return Ok(Value::Nil);
                    } else if member == "toFloat" {
                        if let Value::Int(i) = receiver_val {
                            return Ok(Value::Float(i as f64));
                        } else if let Value::Float(f) = receiver_val {
                            return Ok(Value::Float(f));
                        } else if let Value::String(s) = &receiver_val {
                            if let Ok(f) = s.parse::<f64>() {
                                return Ok(Value::Float(f));
                            }
                        }
                        return Err(format!(
                            "Cannot convert {} to Float",
                            receiver_val.type_name()
                        ));
                    } else if member == "tryFloat" {
                        if let Value::Int(i) = receiver_val {
                            return Ok(Value::Float(i as f64));
                        } else if let Value::Float(f) = receiver_val {
                            return Ok(Value::Float(f));
                        } else if let Value::String(s) = &receiver_val {
                            if let Ok(f) = s.parse::<f64>() {
                                return Ok(Value::Float(f));
                            }
                        }
                        return Ok(Value::Nil);
                    } else if member == "toBool" {
                        return Ok(Value::Bool(receiver_val.is_truthy()));
                    } else if member == "tryBool" {
                        return Ok(Value::Bool(receiver_val.is_truthy()));
                    } else if member == "toByte" {
                        if let Value::Int(i) = receiver_val {
                            if i < 0 || i > 255 {
                                return Err(format!(
                                    "toByte: value {} is out of bounds for Byte (0..255)",
                                    i
                                ));
                            }
                            return Ok(Value::Byte(i as u8));
                        }
                        if let Value::Byte(b) = receiver_val {
                            return Ok(Value::Byte(b));
                        }
                        let bytes_vec = receiver_val.to_string().into_bytes();
                        return Ok(Value::Bytes(bytes_vec));
                    } else if member == "toUtf8" {
                        if let Value::Bytes(b) = receiver_val {
                            return String::from_utf8(b)
                                .map(Value::String)
                                .map_err(|_| "toUtf8: invalid UTF-8 data".to_string());
                        }
                        return Err(format!(
                            "toUtf8 is only supported on Bytes, found {}",
                            receiver_val.type_name()
                        ));
                    } else if member == "tryUtf8" {
                        if let Value::Bytes(b) = receiver_val {
                            return match String::from_utf8(b) {
                                Ok(s) => Ok(Value::String(s)),
                                Err(_) => Ok(Value::Nil),
                            };
                        }
                        return Err(format!(
                            "tryUtf8 is only supported on Bytes, found {}",
                            receiver_val.type_name()
                        ));
                    } else if member == "toHex" {
                        if let Value::Bytes(b) = receiver_val {
                            let hex = b
                                .iter()
                                .map(|byte| format!("{:02x}", byte))
                                .collect::<String>();
                            return Ok(Value::String(hex));
                        }
                        return Err(format!(
                            "toHex is only supported on Bytes, found {}",
                            receiver_val.type_name()
                        ));
                    }
                    #[cfg(feature = "base64")]
                    if member == "toBase64" {
                        if let Value::Bytes(b) = receiver_val {
                            use base64::{Engine as _, engine::general_purpose};
                            return Ok(Value::String(general_purpose::STANDARD.encode(&b)));
                        }
                        return Err(format!(
                            "toBase64 is only supported on Bytes, found {}",
                            receiver_val.type_name()
                        ));
                    }

                    if member == "concat" {
                        if args.is_empty() {
                            return Err("concat expects 1 argument".to_string());
                        }
                        let other = self.eval_expr(&args[0].1, env.clone())?;
                        if let Value::Bytes(b1) = receiver_val {
                            if let Value::Bytes(b2) = other {
                                let mut res = b1.clone();
                                res.extend(b2);
                                return Ok(Value::Bytes(res));
                            }
                            return Err(format!(
                                "concat on Bytes expects Bytes argument, found {}",
                                other.type_name()
                            ));
                        } else if let Value::String(s1) = receiver_val {
                            let s2 = other.to_string();
                            return Ok(Value::String(format!("{}{}", s1, s2)));
                        } else if let Value::Tuple(t1) = receiver_val {
                            if let Value::Tuple(t2) = other {
                                let mut res = t1.clone();
                                res.extend(t2);
                                return Ok(Value::Tuple(res));
                            }
                            return Err(format!(
                                "concat on Tuple expects Tuple argument, found {}",
                                other.type_name()
                            ));
                        }
                        return Err(format!(
                            "concat is not supported on {}",
                            receiver_val.type_name()
                        ));
                    } else if member == "index" {
                        if args.len() < 1 {
                            return Err("index requires at least 1 argument (key)".to_string());
                        }
                        let key_val = self.eval_expr(&args[0].1, env.clone())?;
                        match receiver_val {
                            Value::Tuple(items) => {
                                if let Value::Int(i) = key_val {
                                    if i >= 0 && (i as usize) < items.len() {
                                        return Ok(items[i as usize].clone());
                                    }
                                }
                                return Ok(Value::Nil);
                            }
                            Value::Bytes(items) => {
                                if let Value::Int(i) = key_val {
                                    if i >= 0 && (i as usize) < items.len() {
                                        return Ok(Value::Byte(items[i as usize]));
                                    }
                                }
                                return Ok(Value::Nil);
                            }
                            Value::Object(map)
                            | Value::Formula(map)
                            | Value::StructInstance { fields: map, .. } => {
                                let key_str = key_val.to_string();
                                if let Some(v) = map.get(&key_str) {
                                    return Ok(v.clone());
                                }
                                return Ok(Value::Nil);
                            }
                            _ => {
                                return Err(format!(
                                    "cannot index on {}",
                                    receiver_val.type_name()
                                ));
                            }
                        }
                    } else if member == "abs" {
                        if let Value::Int(i) = receiver_val {
                            return Ok(Value::Int(i.abs()));
                        }
                        if let Value::Float(f) = receiver_val {
                            return Ok(Value::Float(f.abs()));
                        }
                        return Err("abs requires a number".to_string());
                    } else if member == "floor" {
                        if let Value::Int(i) = receiver_val {
                            return Ok(Value::Int(i));
                        }
                        if let Value::Float(f) = receiver_val {
                            return Ok(Value::Float(f.floor()));
                        }
                        return Err("floor requires a number".to_string());
                    } else if member == "ceil" {
                        if let Value::Int(i) = receiver_val {
                            return Ok(Value::Int(i));
                        }
                        if let Value::Float(f) = receiver_val {
                            return Ok(Value::Float(f.ceil()));
                        }
                        return Err("ceil requires a number".to_string());
                    } else if member == "round" {
                        if let Value::Int(i) = receiver_val {
                            return Ok(Value::Int(i));
                        }
                        if let Value::Float(f) = receiver_val {
                            return Ok(Value::Float(f.round()));
                        }
                        return Err("round requires a number".to_string());
                    } else if member == "sqrt" {
                        if let Value::Int(i) = receiver_val {
                            return Ok(Value::Float((i as f64).sqrt()));
                        }
                        if let Value::Float(f) = receiver_val {
                            return Ok(Value::Float(f.sqrt()));
                        }
                        return Err("sqrt requires a number".to_string());
                    } else if member == "pow" {
                        if args.len() < 1 {
                            return Err("pow requires 1 argument (exponent)".to_string());
                        }
                        let exp_val = self.eval_expr(&args[0].1, env.clone())?;
                        let exp_f = if let Value::Float(f) = exp_val {
                            f
                        } else if let Ok(i) = exp_val.as_int() {
                            i as f64
                        } else {
                            0.0
                        };
                        if let Value::Int(i) = receiver_val {
                            return Ok(Value::Float((i as f64).powf(exp_f)));
                        }
                        if let Value::Float(f) = receiver_val {
                            return Ok(Value::Float(f.powf(exp_f)));
                        }
                        return Err("pow requires a number".to_string());
                    } else if member == "min" {
                        if args.len() < 1 {
                            return Err("min requires 1 argument".to_string());
                        }
                        let other = self.eval_expr(&args[0].1, env.clone())?;
                        if let (Value::Int(a), Value::Int(b)) = (&receiver_val, &other) {
                            return Ok(Value::Int(*a.min(b)));
                        }
                        let a_f = if let Value::Float(f) = receiver_val {
                            f
                        } else {
                            receiver_val.as_int().unwrap_or(0) as f64
                        };
                        let b_f = if let Value::Float(f) = other {
                            f
                        } else {
                            other.as_int().unwrap_or(0) as f64
                        };
                        return Ok(Value::Float(a_f.min(b_f)));
                    } else if member == "max" {
                        if args.len() < 1 {
                            return Err("max requires 1 argument".to_string());
                        }
                        let other = self.eval_expr(&args[0].1, env.clone())?;
                        if let (Value::Int(a), Value::Int(b)) = (&receiver_val, &other) {
                            return Ok(Value::Int(*a.max(b)));
                        }
                        let a_f = if let Value::Float(f) = receiver_val {
                            f
                        } else {
                            receiver_val.as_int().unwrap_or(0) as f64
                        };
                        let b_f = if let Value::Float(f) = other {
                            f
                        } else {
                            other.as_int().unwrap_or(0) as f64
                        };
                        return Ok(Value::Float(a_f.max(b_f)));
                    } else if member == "clamp" {
                        if args.len() < 2 {
                            return Err("clamp requires 2 arguments (min, max)".to_string());
                        }
                        let min_val = self.eval_expr(&args[0].1, env.clone())?;
                        let max_val = self.eval_expr(&args[1].1, env.clone())?;
                        if let (Value::Int(v), Value::Int(min), Value::Int(max)) =
                            (&receiver_val, &min_val, &max_val)
                        {
                            return Ok(Value::Int(*v.max(min).min(max)));
                        }
                        let v_f = if let Value::Float(f) = receiver_val {
                            f
                        } else {
                            receiver_val.as_int().unwrap_or(0) as f64
                        };
                        let min_f = if let Value::Float(f) = min_val {
                            f
                        } else {
                            min_val.as_int().unwrap_or(0) as f64
                        };
                        let max_f = if let Value::Float(f) = max_val {
                            f
                        } else {
                            max_val.as_int().unwrap_or(0) as f64
                        };
                        return Ok(Value::Float(v_f.max(min_f).min(max_f)));
                    }
                    #[cfg(feature = "utils")]
                    {
                        if member == "year" {
                            if let Value::Int(timestamp) = receiver_val {
                                if let Some(dt) = chrono::DateTime::from_timestamp_millis(timestamp)
                                {
                                    return Ok(Value::Int(chrono::Datelike::year(&dt) as i64));
                                }
                            }
                            return Err("year requires a valid timestamp".to_string());
                        } else if member == "month" {
                            if let Value::Int(timestamp) = receiver_val {
                                if let Some(dt) = chrono::DateTime::from_timestamp_millis(timestamp)
                                {
                                    return Ok(Value::Int(chrono::Datelike::month(&dt) as i64));
                                }
                            }
                            return Err("month requires a valid timestamp".to_string());
                        } else if member == "day" {
                            if let Value::Int(timestamp) = receiver_val {
                                if let Some(dt) = chrono::DateTime::from_timestamp_millis(timestamp)
                                {
                                    return Ok(Value::Int(chrono::Datelike::day(&dt) as i64));
                                }
                            }
                            return Err("day requires a valid timestamp".to_string());
                        } else if member == "addDays" {
                            if args.len() < 1 {
                                return Err("addDays requires 1 argument".to_string());
                            }
                            let other = self.eval_expr(&args[0].1, env.clone())?;
                            if let (Value::Int(timestamp), Value::Int(days)) =
                                (&receiver_val, &other)
                            {
                                if let Some(dt) =
                                    chrono::DateTime::from_timestamp_millis(*timestamp)
                                {
                                    if let Some(new_dt) =
                                        dt.checked_add_signed(chrono::Duration::days(*days))
                                    {
                                        return Ok(Value::Int(new_dt.timestamp_millis()));
                                    }
                                }
                            }
                            return Err(
                                "addDays requires a valid timestamp and an integer".to_string()
                            );
                        } else if member == "addHours" {
                            if args.len() < 1 {
                                return Err("addHours requires 1 argument".to_string());
                            }
                            let other = self.eval_expr(&args[0].1, env.clone())?;
                            if let (Value::Int(timestamp), Value::Int(hours)) =
                                (&receiver_val, &other)
                            {
                                if let Some(dt) =
                                    chrono::DateTime::from_timestamp_millis(*timestamp)
                                {
                                    if let Some(new_dt) =
                                        dt.checked_add_signed(chrono::Duration::hours(*hours))
                                    {
                                        return Ok(Value::Int(new_dt.timestamp_millis()));
                                    }
                                }
                            }
                            return Err(
                                "addHours requires a valid timestamp and an integer".to_string()
                            );
                        }
                    }
                    if member == "toJson" {
                        // Very simple JSON serializer
                        fn to_json(val: &Value) -> String {
                            match val {
                                Value::String(s) => {
                                    format!("\"{}\"", s.replace("\"", "\\\"").replace("\n", "\\n"))
                                }
                                Value::Int(i) => i.to_string(),
                                Value::Float(f) => f.to_string(),
                                Value::Bool(b) => {
                                    if *b {
                                        "true".to_string()
                                    } else {
                                        "false".to_string()
                                    }
                                }
                                Value::Nil => "null".to_string(),
                                Value::Tuple(arr) => {
                                    let items: Vec<String> =
                                        arr.iter().map(|v| to_json(v)).collect();
                                    format!("[{}]", items.join(", "))
                                }
                                Value::Bytes(arr) => {
                                    let items: Vec<String> =
                                        arr.iter().map(|v| v.to_string()).collect();
                                    format!("[{}]", items.join(", "))
                                }
                                Value::StructInstance { fields, .. }
                                | Value::Object(fields)
                                | Value::Formula(fields) => {
                                    let items: Vec<String> = fields
                                        .iter()
                                        .map(|(k, v)| format!("\"{}\": {}", k, to_json(v)))
                                        .collect();
                                    format!("{{{}}}", items.join(", "))
                                }
                                _ => "\"<unserializable>\"".to_string(),
                            }
                        }
                        return Ok(Value::String(to_json(&receiver_val)));
                    } else if member == "fromJson" {
                        if args.len() < 1 {
                            return Err("fromJson requires 1 argument (json string)".to_string());
                        }
                        let json_val = self.eval_expr(&args[0].1, env.clone())?;
                        let json_str = json_val.to_string();

                        fn from_json(v: &serde_json::Value) -> Value {
                            match v {
                                serde_json::Value::Null => Value::Nil,
                                serde_json::Value::Bool(b) => Value::Bool(*b),
                                serde_json::Value::Number(n) => {
                                    if let Some(i) = n.as_i64() {
                                        Value::Int(i)
                                    } else if let Some(f) = n.as_f64() {
                                        Value::Float(f)
                                    } else {
                                        Value::Nil
                                    }
                                }
                                serde_json::Value::String(s) => Value::String(s.clone()),
                                serde_json::Value::Array(arr) => {
                                    let items: Vec<Value> =
                                        arr.iter().map(|x| from_json(x)).collect();
                                    Value::Tuple(items)
                                }
                                serde_json::Value::Object(map) => {
                                    let mut m = std::collections::HashMap::new();
                                    for (k, v) in map {
                                        m.insert(k.clone(), from_json(v));
                                    }
                                    Value::Object(m) // Return Object, which can be coerced or used dynamically
                                }
                            }
                        }

                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            let obj = from_json(&parsed);
                            if let Value::StructConstructor { name, .. } = &receiver_val {
                                if let Value::Object(fields) = obj {
                                    return Ok(Value::StructInstance {
                                        name: name.clone(),
                                        fields,
                                    });
                                }
                            }
                            return Ok(obj);
                        }
                        return Err("Invalid JSON string".to_string());
                    } else if member == "fromBytes" || member == "fromByte" {
                        if args.len() < 1 {
                            return Err(format!("{} requires 1 argument (byte vector)", member));
                        }
                        let bytes_val = self.eval_expr(&args[0].1, env.clone())?;
                        let raw_bytes_opt = match bytes_val {
                            Value::Bytes(b) => Some(b),
                            Value::Tuple(items) => {
                                let mut b = Vec::new();
                                for it in items {
                                    if let Value::Int(i) = it {
                                        b.push(i as u8);
                                    } else if let Value::Byte(by) = it {
                                        b.push(by);
                                    }
                                }
                                Some(b)
                            }
                            _ => None,
                        };
                        if let Some(bytes) = raw_bytes_opt {
                            if let Ok(json_str) = String::from_utf8(bytes.clone()) {
                                fn from_json(v: &serde_json::Value) -> Value {
                                    match v {
                                        serde_json::Value::Null => Value::Nil,
                                        serde_json::Value::Bool(b) => Value::Bool(*b),
                                        serde_json::Value::Number(n) => {
                                            if let Some(i) = n.as_i64() {
                                                Value::Int(i)
                                            } else if let Some(f) = n.as_f64() {
                                                Value::Float(f)
                                            } else {
                                                Value::Nil
                                            }
                                        }
                                        serde_json::Value::String(s) => Value::String(s.clone()),
                                        serde_json::Value::Array(arr) => {
                                            let items: Vec<Value> =
                                                arr.iter().map(|x| from_json(x)).collect();
                                            Value::Tuple(items)
                                        }
                                        serde_json::Value::Object(map) => {
                                            let mut m = std::collections::HashMap::new();
                                            for (k, v) in map {
                                                m.insert(k.clone(), from_json(v));
                                            }
                                            Value::Object(m)
                                        }
                                    }
                                }
                                if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(&json_str)
                                {
                                    let obj = from_json(&parsed);
                                    if let Value::StructConstructor { name, .. } = &receiver_val {
                                        if let Value::Object(fields) = obj {
                                            return Ok(Value::StructInstance {
                                                name: name.clone(),
                                                fields,
                                            });
                                        }
                                    }
                                    return Ok(obj);
                                }
                            }
                        }
                        return Err("Invalid JSON bytes".to_string());
                    }

                    let mut custom_method = None;
                    if let Value::Formula(ref map) | Value::Object(ref map) = receiver_val {
                        if let Some(m) = map.get(member) {
                            custom_method = Some(m.clone());
                        }
                    }
                    if custom_method.is_none() {
                        if let Some(m) = env.lock().unwrap().get(member) {
                            custom_method = Some(m);
                        }
                    }

                    if let Some(func_val) = custom_method {
                        match func_val {
                            Value::Function {
                                params,
                                body,
                                env: closure_env,
                                ..
                            } => {
                                let mut evaled_args = Vec::new();
                                let is_self_method = params.first().map_or(false, |p| {
                                    p.name == "self"
                                        || p.name == "&self"
                                        || p.name == "&mut self"
                                        || p.type_name == "Self"
                                        || p.type_name == "&Self"
                                        || p.type_name == "&mut Self"
                                });
                                if is_self_method {
                                    let mut self_val = receiver_val.clone();
                                    if let Expr::Identifier(var_name, _) = &**inner_expr {
                                        self_val = Value::RefPath(
                                            crate::vm::RefPath::Var(var_name.clone(), env.clone()),
                                            true,
                                        );
                                    } else if let Expr::Dot(owner_expr, field_name, _) =
                                        &**inner_expr
                                    {
                                        if let Expr::Identifier(owner_name, _) = &**owner_expr {
                                            self_val = Value::RefPath(
                                                crate::vm::RefPath::Field {
                                                    owner: owner_name.clone(),
                                                    member: field_name.clone(),
                                                    env: env.clone(),
                                                },
                                                true,
                                            );
                                        }
                                    }
                                    evaled_args.push(self_val);
                                }
                                for (_, arg_expr) in args {
                                    let arg_v = self.eval_expr(arg_expr, env.clone())?;
                                    if let Expr::Identifier(src_name, _) = arg_expr {
                                        env.lock().unwrap().move_var(src_name);
                                    }
                                    evaled_args.push(arg_v);
                                }

                                let child_env =
                                    Arc::new(Mutex::new(Env::new_child(closure_env.clone())));
                                for (i, p) in params.iter().enumerate() {
                                    if i < evaled_args.len() {
                                        self.bind_param(
                                            child_env.clone(),
                                            p,
                                            evaled_args[i].clone(),
                                        );
                                    } else if let Some(def_expr) = &p.default_val {
                                        if let Ok(val) = self.eval_expr(def_expr, child_env.clone())
                                        {
                                            self.bind_param(child_env.clone(), p, val);
                                        }
                                    }
                                }
                                let mut last_val = Value::Nil;
                                for stmt in &body {
                                    let res = self.execute_statement(stmt, child_env.clone())?;
                                    if let Value::Return(ret_val) = res {
                                        return Ok(*ret_val);
                                    }
                                    last_val = res;
                                }
                                return Ok(last_val);
                            }
                            Value::NativeCallback(cb) => {
                                let mut evaled_args = Vec::new();
                                let mut is_module = false;
                                if let Value::Object(map) | Value::Formula(map) = &receiver_val {
                                    if map.contains_key("__module__") {
                                        is_module = true;
                                    }
                                }
                                if !is_module {
                                    evaled_args.push(receiver_val.clone());
                                }
                                for (_, arg_expr) in args {
                                    evaled_args.push(self.eval_expr(arg_expr, env.clone())?);
                                }
                                return cb(evaled_args);
                            }
                            Value::NativeClosure(crate::vm::NativeClosureType(cb)) => {
                                let mut evaled_args = Vec::new();
                                let mut is_module = false;
                                if let Value::Object(map) | Value::Formula(map) = &receiver_val {
                                    if map.contains_key("__module__") {
                                        is_module = true;
                                    }
                                }
                                if !is_module {
                                    evaled_args.push(receiver_val.clone());
                                }
                                for (_, arg_expr) in args {
                                    evaled_args.push(self.eval_expr(arg_expr, env.clone())?);
                                }
                                return cb(evaled_args);
                            }
                            _ => {}
                        }
                    }
                }

                // Resolve function or structure constructors
                let func_val = self.eval_expr(callee, env.clone())?;
                match func_val {
                    Value::StructConstructor { name, fields } => {
                        let mut instance_map = HashMap::new();
                        for (i, (f_name, _)) in fields.iter().enumerate() {
                            let mut val = Value::Nil;
                            for (arg_name, arg_expr) in args {
                                if let Some(an) = arg_name {
                                    if an == f_name {
                                        val = self.eval_expr(arg_expr, env.clone())?;
                                        break;
                                    }
                                }
                            }
                            if matches!(val, Value::Nil) && i < args.len() && args[i].0.is_none() {
                                val = self.eval_expr(&args[i].1, env.clone())?;
                            }
                            instance_map.insert(f_name.clone(), val);
                        }
                        if let Some(impl_env) = self.modules.get(&format!("impl_{}", name)) {
                            for (m_name, m_val) in &impl_env.lock().unwrap().variables {
                                instance_map.insert(m_name.clone(), m_val.value.clone());
                            }
                        }
                        Ok(Value::Formula(instance_map))
                    }
                    Value::VariantConstructor(enum_name, var) => {
                        if let crate::parser::EnumVariant::Tuple(n, _) = var {
                            let mut tuple_vals = Vec::new();
                            for (_, arg_expr) in args {
                                tuple_vals.push(self.eval_expr(arg_expr, env.clone())?);
                            }
                            return Ok(Value::EnumValue(enum_name, n, EnumData::Tuple(tuple_vals)));
                        }
                        Err(format!("variant '{:?}' is not a tuple variant", var))
                    }
                    Value::Function {
                        params,
                        body,
                        env: closure_env,
                        annotations,
                    } => {
                        let child_env = Arc::new(Mutex::new(Env::new_child(closure_env.clone())));
                        for anno in &annotations {
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
                                                    crate::vm::EnumData::Tuple(vec![
                                                        Value::String(format!(
                                                            "PermissionDenied: {}",
                                                            perm_name
                                                        )),
                                                    ]),
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
                                                        crate::vm::EnumData::Tuple(vec![
                                                            Value::String(format!(
                                                                "PermissionDenied: {}",
                                                                perm_name
                                                            )),
                                                        ]),
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
                                    if let Some(mut current) =
                                        closure_env.lock().unwrap().get(parts[0])
                                    {
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
                                        let mut parser = crate::parser::Parser::new(
                                            tokens,
                                            "anno_arg".to_string(),
                                        );
                                        if let Ok(expr) = parser.parse_expr() {
                                            if let Ok(val) = self.eval_expr(&expr, env.clone()) {
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
                                        &params,
                                        None,
                                        &annotations,
                                        None,
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
                                                "Annotation '{}' evaluation failed: {}",
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
                                if i < args.len() {
                                    let arg_val = self.eval_expr(&args[i].1, env.clone())?;
                                    if let Expr::Identifier(src_name, _) = &args[i].1 {
                                        env.lock().unwrap().move_var(src_name);
                                    }
                                    self.bind_param(child_env.clone(), p, arg_val);
                                } else if let Some(def_expr) = &p.default_val {
                                    if let Ok(val) = self.eval_expr(def_expr, child_env.clone()) {
                                        self.bind_param(child_env.clone(), p, val);
                                    }
                                }
                            }
                        }
                        let mut last_val = Value::Nil;
                        for stmt in &body {
                            let res = self.execute_statement(stmt, child_env.clone())?;
                            if let Value::Return(ret_val) = res {
                                return Ok(*ret_val);
                            }
                            last_val = res;
                        }
                        Ok(last_val)
                    }
                    Value::NativeCallback(cb) => {
                        let mut evaled_args = Vec::new();
                        for (_, arg_expr) in args {
                            evaled_args.push(self.eval_expr(arg_expr, env.clone())?);
                        }
                        return cb(evaled_args);
                    }
                    Value::NativeClosure(crate::vm::NativeClosureType(cb)) => {
                        let mut evaled_args = Vec::new();
                        for (_, arg_expr) in args {
                            evaled_args.push(self.eval_expr(arg_expr, env.clone())?);
                        }
                        return cb(evaled_args);
                    }
                    _ => {
                        let lexeme = match &**callee {
                            Expr::Identifier(id, _) => id.as_str(),
                            _ => "unknown",
                        };
                        if lexeme == "print" {
                            for (_, arg_expr) in args {
                                let arg_v = self.eval_expr(arg_expr, env.clone())?;
                                print!("{} ", arg_v.to_string());
                            }
                            println!();
                            return Ok(Value::Nil);
                        }
                        if lexeme == "RustServer" {
                            let mut port = 8080;
                            for (arg_name, arg_expr) in args {
                                if let Some(n) = arg_name {
                                    if n == "port" {
                                        if let Value::Int(p) =
                                            self.eval_expr(arg_expr, env.clone())?
                                        {
                                            port = p as u16;
                                        }
                                    }
                                }
                            }
                            return Ok(Value::RustServer { port });
                        }
                        Ok(Value::Nil)
                    }
                }
            }
            Expr::Formula(mappings, _) => {
                let mut map = HashMap::new();
                for (k, v, _, _) in mappings {
                    let val = self.eval_expr(v, env.clone())?;
                    map.insert(k.clone(), val);
                }
                Ok(Value::Formula(map))
            }
            Expr::Object(mappings, _) => {
                let mut map = HashMap::new();
                for (k, v, _annotations) in mappings {
                    let val = self.eval_expr(v, env.clone())?;
                    map.insert(k.clone(), val);
                }
                Ok(Value::Object(map))
            }
            Expr::InterpolatedString(segments, _) => {
                let mut result = String::new();
                for seg in segments {
                    match seg {
                        InterpolatedSegment::Text(text) => result.push_str(text),
                        InterpolatedSegment::Expr(ex) => {
                            let val = self.eval_expr(ex, env.clone())?;
                            result.push_str(&val.to_string());
                        }
                    }
                }
                Ok(Value::String(result))
            }
            Expr::ThreadSpawn(expr_block, _) => {
                let expr_clone = expr_block.clone();
                let thread_env = Arc::new(Mutex::new(env.lock().unwrap().snapshot()));
                let mut runner = self.clone_for_thread(thread_env.clone());

                let mut counter = get_thread_counter().lock().unwrap();
                *counter += 1;
                let id = *counter;

                let handle = thread::spawn(move || {
                    let mut res = runner
                        .eval_expr(&expr_clone, thread_env)
                        .unwrap_or(Value::Nil);
                    if let Value::Return(ret_val) = res {
                        res = *ret_val;
                    }
                    res
                });

                get_threads().lock().unwrap().insert(id, handle);

                Ok(Value::ThreadHandler(id))
            }
            Expr::Tuple(exprs, _) => {
                let mut vals = Vec::new();
                for e in exprs {
                    vals.push(self.eval_expr(e, env.clone())?);
                }
                Ok(Value::Tuple(vals))
            }
            Expr::Await(inner, _) => {
                let inner_val = self.eval_expr(inner, env)?;
                if let Value::ThreadHandler(id) = inner_val {
                    let handle_opt = get_threads().lock().unwrap().remove(&id);
                    if let Some(handle) = handle_opt {
                        match handle.join() {
                            Ok(val) => Ok(val),
                            Err(_) => Err("Thread panicked".to_string()),
                        }
                    } else {
                        Err(format!("Invalid thread handle {}", id))
                    }
                } else {
                    Ok(inner_val)
                }
            }
            Expr::Block(statements, _) => {
                let child_env = Arc::new(Mutex::new(Env::new_child(env)));
                let mut last_val = Value::Nil;
                for stmt in statements {
                    let res = self.execute_statement(stmt, child_env.clone())?;
                    if matches!(res, Value::Return(_)) || matches!(res, Value::Break) {
                        return Ok(res);
                    }
                    last_val = res;
                }
                Ok(last_val)
            }
            Expr::VectorLiteral(exprs, _) => {
                let mut vals = Vec::new();
                for e in exprs {
                    vals.push(self.eval_expr(e, env.clone())?);
                }
                Ok(Value::Tuple(vals))
            }
            Expr::JsxElement {
                tag,
                attributes,
                children,
                ..
            } => {
                let mut attrs_map = HashMap::new();
                for attr in attributes {
                    let val = if let Some(v_expr) = &attr.value {
                        self.eval_expr(v_expr, env.clone())?
                    } else {
                        Value::Bool(true)
                    };
                    attrs_map.insert(attr.name.clone(), val);
                }

                fn eval_child(
                    runner: &mut Runner,
                    child: &crate::parser::JsxChild,
                    env: Arc<Mutex<Env>>,
                    out: &mut Vec<Value>,
                ) -> Result<(), String> {
                    match child {
                        crate::parser::JsxChild::Text(t, _) => {
                            out.push(Value::String(t.clone()));
                        }
                        crate::parser::JsxChild::Expr(e) => {
                            out.push(runner.eval_expr(e, env)?);
                        }
                        crate::parser::JsxChild::Element(e) => {
                            out.push(runner.eval_expr(e, env)?);
                        }
                        crate::parser::JsxChild::For {
                            var_name,
                            iterable,
                            body,
                            ..
                        } => {
                            let iter_val = runner.eval_expr(iterable, env.clone())?;
                            let items = match iter_val {
                                Value::Tuple(items) => items,
                                _ => Vec::new(),
                            };
                            for item in items {
                                let child_env = Arc::new(Mutex::new(Env::new_child(env.clone())));
                                child_env
                                    .lock()
                                    .unwrap()
                                    .define(var_name.clone(), item, false);
                                for b in body {
                                    eval_child(runner, b, child_env.clone(), out)?;
                                }
                            }
                        }
                    }
                    Ok(())
                }

                let mut children_vals = Vec::new();
                for child in children {
                    eval_child(self, child, env.clone(), &mut children_vals)?;
                }

                let mut node_obj = HashMap::new();
                node_obj.insert("tag".to_string(), Value::String(tag.clone()));
                node_obj.insert("attributes".to_string(), Value::Object(attrs_map));
                node_obj.insert("children".to_string(), Value::Tuple(children_vals));
                node_obj.insert("__type__".to_string(), Value::String("HtmlNode".to_string()));

                Ok(Value::Object(node_obj))
            }
        }
    }


}
