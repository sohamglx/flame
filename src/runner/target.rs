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
    pub(crate) fn read_target(&self, _env: Arc<Mutex<Env>>, path: RefPath) -> Result<Value, String> {
        match path {
            RefPath::Var(name, env) => {
                let val = {
                    let e = env.lock().unwrap();
                    e.get(&name)
                };
                match val {
                    Some(Value::Moved(moved_name)) => Err(format!(
                        "use of moved value '{}'. Value was moved. Use '&{}' to borrow or '{}.clone()' to copy.",
                        moved_name, moved_name, moved_name
                    )),
                    Some(Value::RefPath(next, _)) => self.read_target(env.clone(), next),

                    Some(v) => Ok(v),
                    None => Err(format!("undefined variable '{}'", name)),
                }
            }
            RefPath::Field { owner, member, env } => {
                let owner_val = {
                    let e = env.lock().unwrap();
                    e.get(&owner)
                };
                match owner_val {
                    Some(Value::Moved(moved_name)) => Err(format!(
                        "use of moved value '{}'. Value was moved. Use '&{}' to borrow or '{}.clone()' to copy.",
                        moved_name, moved_name, moved_name
                    )),
                    Some(Value::RefPath(next, _)) => {
                        let resolved_owner = self.read_target(env.clone(), next)?;
                        self.read_field_value(&resolved_owner, &member, &owner)
                    }
                    Some(val) => self.read_field_value(&val, &member, &owner),
                    None => Err(format!("variable '{}' not found for field read", owner)),
                }
            }
            RefPath::Index { owner, index, env } => {
                let direct_res = env.lock().unwrap().with_ref(&owner, |owner_val| {
                    match owner_val {
                        Value::Moved(moved_name) => {
                            Some(Err(format!("use of moved value '{}'.", moved_name)))
                        }
                        Value::Tuple(elems) => {
                            if index < elems.len() {
                                Some(Ok(elems[index].clone()))
                            } else {
                                Some(Err(format!("Index out of bounds: {}", index)))
                            }
                        }
                        Value::SharedTuple(elems) => {
                            if index < elems.len() {
                                Some(Ok(elems[index].clone()))
                            } else {
                                Some(Err(format!("Index out of bounds: {}", index)))
                            }
                        }
                        Value::RefPath(..) => None,
                        _ => Some(Err(format!("cannot index non-tuple/array '{}'", owner))),
                    }
                });
                if let Some(Some(res)) = direct_res {
                    return res;
                }
                let owner_val = {
                    let e = env.lock().unwrap();
                    e.get(&owner)
                };
                match owner_val {
                    Some(Value::Moved(moved_name)) => {
                        Err(format!("use of moved value '{}'.", moved_name))
                    }
                    Some(Value::RefPath(next, _)) => {
                        let resolved_owner = self.read_target(env.clone(), next)?;
                        if let Some(elems) = resolved_owner.as_tuple() {
                            if index < elems.len() {
                                Ok(elems[index].clone())
                            } else {
                                Err(format!("Index out of bounds: {}", index))
                            }
                        } else {
                            Err(format!("cannot index non-tuple/array '{}'", owner))
                        }
                    }
                    Some(Value::Tuple(elems)) => {
                        if index < elems.len() {
                            Ok(elems[index].clone())
                        } else {
                            Err(format!("Index out of bounds: {}", index))
                        }
                    }
                    Some(Value::SharedTuple(elems)) => {
                        if index < elems.len() {
                            Ok(elems[index].clone())
                        } else {
                            Err(format!("Index out of bounds: {}", index))
                        }
                    }
                    Some(_) => Err(format!("cannot index non-tuple/array '{}'", owner)),
                    None => Err(format!("variable '{}' not found for index read", owner)),
                }
            }
        }
    }

    pub(crate) fn read_field_value(
        &self,
        owner_val: &Value,
        member: &str,
        owner: &str,
    ) -> Result<Value, String> {
        match owner_val {
            Value::Formula(map) => map
                .get(member)
                .cloned()
                .ok_or_else(|| format!("member '{}' not found in '{}'", member, owner)),
            Value::StructInstance { name, fields } => {
                fields.get(member).cloned().ok_or_else(|| {
                    format!(
                        "field '{}' not found in struct '{}' ('{}')",
                        member, name, owner
                    )
                })
            }
            Value::EnumValue(enum_name, variant_name, data) => match data {
                EnumData::Struct(map) => map.get(member).cloned().ok_or_else(|| {
                    format!(
                        "field '{}' not found in variant '{}.{}'",
                        member, enum_name, variant_name
                    )
                }),
                EnumData::Tuple(values) => {
                    if values.len() == 1 {
                        if let Value::Formula(map) = &values[0] {
                            map.get(member).cloned().ok_or_else(|| {
                                format!(
                                    "field '{}' not found in variant '{}.{}'",
                                    member, enum_name, variant_name
                                )
                            })
                        } else {
                            Err(format!(
                                "field '{}' not found in variant '{}.{}'",
                                member, enum_name, variant_name
                            ))
                        }
                    } else {
                        Err(format!(
                            "field '{}' not found in variant '{}.{}'",
                            member, enum_name, variant_name
                        ))
                    }
                }
                EnumData::Unit => Err(format!(
                    "variant '{}.{}' has no fields",
                    enum_name, variant_name
                )),
            },
            _ => Err(format!(
                "cannot access member '{}' on non-namespace value in '{}'",
                member, owner
            )),
        }
    }

    pub(crate) fn write_back(
        &self,
        _env: Arc<Mutex<Env>>,
        path: RefPath,
        new_val: Value,
    ) -> Result<(), String> {
        match path {
            RefPath::Var(name, env) => {
                let current = {
                    let e = env.lock().unwrap();
                    e.get(&name)
                };

                if let Some(Value::RefPath(next, _)) = current {
                    return self.write_back(env.clone(), next, new_val);
                }

                env.lock().unwrap().assign(name, new_val)
            }
            RefPath::Index { owner, index, env } => {
                let mut owner_val = {
                    let e = env.lock().unwrap();
                    e.get(&owner)
                };

                let mut final_owner = owner.clone();
                let mut final_env = env.clone();
                while let Some(Value::RefPath(RefPath::Var(ref next_owner, ref next_env), _)) =
                    owner_val
                {
                    final_owner = next_owner.clone();
                    final_env = next_env.clone();
                    owner_val = {
                        let e = final_env.lock().unwrap();
                        e.get(&final_owner)
                    };
                }

                let Some(mut owner_val) = owner_val else {
                    return Err(format!(
                        "variable '{}' not found for index assignment",
                        final_owner
                    ));
                };

                match &mut owner_val {
                    Value::Tuple(elems) => {
                        if index < elems.len() {
                            elems[index] = new_val.clone();
                        } else {
                            return Err(format!("Index out of bounds: {}", index));
                        }
                    }
                    Value::SharedTuple(elems) => {
                        let vec = std::sync::Arc::make_mut(elems);
                        if index < vec.len() {
                            vec[index] = new_val.clone();
                        } else {
                            return Err(format!("Index out of bounds: {}", index));
                        }
                    }
                    _ => {
                        return Err(format!(
                            "cannot assign to index of non-array value in '{}'",
                            final_owner
                        ));
                    }
                }

                final_env
                    .lock()
                    .unwrap()
                    .assign(final_owner, owner_val.clone())
            }
            RefPath::Field { owner, member, env } => {
                let mut owner_val = {
                    let e = env.lock().unwrap();
                    e.get(&owner)
                };

                // Follow reference paths to get the actual value to modify
                let mut final_owner = owner.clone();
                let mut final_env = env.clone();
                while let Some(Value::RefPath(RefPath::Var(ref next_owner, ref next_env), _)) =
                    owner_val
                {
                    final_owner = next_owner.clone();
                    final_env = next_env.clone();
                    owner_val = {
                        let e = final_env.lock().unwrap();
                        e.get(&final_owner)
                    };
                }

                let Some(mut owner_val) = owner_val else {
                    return Err(format!(
                        "variable '{}' not found for field assignment",
                        final_owner
                    ));
                };
                match &mut owner_val {
                    Value::Formula(map) | Value::Object(map) => {
                        map.insert(member, new_val);
                    }
                    Value::SharedObject(map) => {
                        std::sync::Arc::make_mut(map).insert(member, new_val);
                    }
                    Value::StructInstance {
                        name: _,
                        fields: map,
                    } => {
                        map.insert(member, new_val);
                    }
                    Value::EnumValue(enum_name, variant_name, data) => match data {
                        EnumData::Struct(map) => {
                            map.insert(member, new_val);
                        }
                        EnumData::Tuple(vec) => {
                            if vec.len() == 1 {
                                if let Value::Formula(map) = &mut vec[0] {
                                    map.insert(member, new_val);
                                } else {
                                    return Err(format!(
                                        "cannot assign field '{}' on '{}.{}' tuple payload that is not a Formula",
                                        member, enum_name, variant_name
                                    ));
                                }
                            } else {
                                return Err(format!(
                                    "cannot assign field '{}' on '{}.{}' tuple variant",
                                    member, enum_name, variant_name
                                ));
                            }
                        }
                        EnumData::Unit => {
                            return Err(format!(
                                "cannot assign field '{}' on '{}.{}' unit variant",
                                member, enum_name, variant_name
                            ));
                        }
                    },
                    _ => {
                        return Err(
                            "field assignment supported only on Formula and Enum variants"
                                .to_string(),
                        );
                    }
                }
                final_env.lock().unwrap().assign(final_owner, owner_val)
            }
        }
    }


}
