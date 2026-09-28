use crate::vm::{Value, NativeModuleDef, NativeFunctionDef, NativeTypeDef};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Instant as StdInstant};
use chrono::TimeZone;

pub fn def() -> NativeModuleDef {
    NativeModuleDef {
        name: "std.time".to_string(),
        description: "Time and Date utilities including Instants, Timestamps, and Durations.".to_string(),
        features: vec![],
        functions: vec![
            NativeFunctionDef {
                name: "now".to_string(),
                description: "Returns the current UTC time as a Timestamp object.".to_string(),
                params: vec![],
                return_type: "Timestamp".to_string(),
            },
            NativeFunctionDef {
                name: "parse".to_string(),
                description: "Parses an ISO-8601 or RFC-3339 string into a Timestamp.".to_string(),
                params: vec![("date_string".to_string(), "String".to_string())],
                return_type: "Timestamp".to_string(),
            },
            NativeFunctionDef {
                name: "fromMillis".to_string(),
                description: "Creates a Timestamp from Unix epoch milliseconds.".to_string(),
                params: vec![("milliseconds".to_string(), "Int".to_string())],
                return_type: "Timestamp".to_string(),
            },
            NativeFunctionDef {
                name: "fromSeconds".to_string(),
                description: "Creates a Timestamp from Unix epoch seconds.".to_string(),
                params: vec![("seconds".to_string(), "Int".to_string())],
                return_type: "Timestamp".to_string(),
            },
            NativeFunctionDef {
                name: "instant".to_string(),
                description: "Returns a monotonic point in time suitable for measuring elapsed durations.".to_string(),
                params: vec![],
                return_type: "Instant".to_string(),
            },
            NativeFunctionDef {
                name: "perfCounter".to_string(),
                description: "Returns the current performance counter in fractional seconds (high-resolution monotonic clock) suitable for benchmarking.".to_string(),
                params: vec![],
                return_type: "Float".to_string(),
            },
            NativeFunctionDef {
                name: "counter".to_string(),
                description: "Alias for perfCounter(). Returns current monotonic counter in fractional seconds.".to_string(),
                params: vec![],
                return_type: "Float".to_string(),
            },
        ],
        types: vec![
            NativeTypeDef {
                name: "Timestamp".to_string(),
                description: "An absolute point in time.".to_string(),
                fields: vec![("millis".to_string(), "Int".to_string())],
                methods: vec![
                    NativeFunctionDef {
                        name: "toMillis".to_string(),
                        description: "Returns the UNIX epoch milliseconds.".to_string(),
                        params: vec![],
                        return_type: "Int".to_string(),
                    },
                    NativeFunctionDef {
                        name: "toSeconds".to_string(),
                        description: "Returns the UNIX epoch seconds.".to_string(),
                        params: vec![],
                        return_type: "Int".to_string(),
                    },
                    NativeFunctionDef {
                        name: "toString".to_string(),
                        description: "Returns the human readable UTC date time string.".to_string(),
                        params: vec![],
                        return_type: "String".to_string(),
                    },
                ],
            },
            NativeTypeDef {
                name: "Duration".to_string(),
                description: "A span of time.".to_string(),
                fields: vec![
                    ("millis".to_string(), "Int".to_string()),
                    ("nanos".to_string(), "Int".to_string()),
                    ("seconds".to_string(), "Float".to_string()),
                ],
                methods: vec![
                    NativeFunctionDef {
                        name: "toMillis".to_string(),
                        description: "Returns the total milliseconds of this duration as a Float.".to_string(),
                        params: vec![],
                        return_type: "Float".to_string(),
                    },
                    NativeFunctionDef {
                        name: "toMilliseconds".to_string(),
                        description: "Alias for toMillis(). Returns the total milliseconds of this duration as a Float.".to_string(),
                        params: vec![],
                        return_type: "Float".to_string(),
                    },
                    NativeFunctionDef {
                        name: "toSeconds".to_string(),
                        description: "Returns the total seconds of this duration as a Float.".to_string(),
                        params: vec![],
                        return_type: "Float".to_string(),
                    },
                    NativeFunctionDef {
                        name: "toString".to_string(),
                        description: "Returns a human readable duration string.".to_string(),
                        params: vec![],
                        return_type: "String".to_string(),
                    },
                ],
            },
            NativeTypeDef {
                name: "Instant".to_string(),
                description: "A monotonic clock instant.".to_string(),
                fields: vec![],
                methods: vec![
                    NativeFunctionDef {
                        name: "elapsed".to_string(),
                        description: "Returns the duration elapsed since this instant was created.".to_string(),
                        params: vec![],
                        return_type: "Duration".to_string(),
                    },
                ],
            },
            NativeTypeDef {
                name: "Second".to_string(),
                description: "Type alias for a Second value.".to_string(),
                fields: vec![],
                methods: vec![
                    NativeFunctionDef {
                        name: "toMilliseconds".to_string(),
                        description: "Convert seconds to milliseconds.".to_string(),
                        params: vec![],
                        return_type: "Int".to_string(),
                    }
                ],
            },
            NativeTypeDef {
                name: "Millisecond".to_string(),
                description: "Type alias for a Millisecond value.".to_string(),
                fields: vec![],
                methods: vec![
                    NativeFunctionDef {
                        name: "toSeconds".to_string(),
                        description: "Convert milliseconds to seconds.".to_string(),
                        params: vec![],
                        return_type: "Int".to_string(),
                    }
                ],
            },
        ],
    }
}

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    m.insert(
        "Timestamp".to_string(),
        Value::NativeCallback(|args| {
            if args.len() != 1 {
                return Err("Timestamp expects 1 argument (millis)".to_string());
            }
            if let Value::Int(m) = args[0] {
                let mut fields = HashMap::new();
                fields.insert("millis".to_string(), Value::Int(m));
                
                fields.insert("toMillis".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m))))));
                fields.insert("toSeconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m / 1000))))));
                fields.insert("toString".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                    if let chrono::LocalResult::Single(dt) = chrono::Utc.timestamp_millis_opt(m) {
                        Ok(Value::String(dt.to_rfc3339()))
                    } else {
                        Ok(Value::String(format!("{}ms", m)))
                    }
                }))));
                
                Ok(Value::Object(fields))
            } else {
                Err("Timestamp expects Int".to_string())
            }
        })
    );

    m.insert(
        "Duration".to_string(),
        Value::NativeCallback(|args| {
            if args.len() != 1 {
                return Err("Duration expects 1 argument (millis)".to_string());
            }
            if let Value::Int(m) = args[0] {
                let mut fields = HashMap::new();
                fields.insert("millis".to_string(), Value::Int(m));
                fields.insert("toMilliseconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m))))));
                fields.insert("toSeconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m / 1000))))));
                Ok(Value::Object(fields))
            } else {
                Err("Duration expects Int".to_string())
            }
        })
    );
    
    m.insert(
        "Second".to_string(),
        Value::NativeCallback(|args| {
            if args.len() != 1 { return Err("Second expects 1 argument".to_string()); }
            if let Value::Int(v) = args[0] {
                let mut fields = HashMap::new();
                fields.insert("value".to_string(), Value::Int(v));
                fields.insert("toMilliseconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(v * 1000))))));
                Ok(Value::Object(fields))
            } else {
                Err("Second expects Int".to_string())
            }
        })
    );
    
    m.insert(
        "Millisecond".to_string(),
        Value::NativeCallback(|args| {
            if args.len() != 1 { return Err("Millisecond expects 1 argument".to_string()); }
            if let Value::Int(v) = args[0] {
                let mut fields = HashMap::new();
                fields.insert("value".to_string(), Value::Int(v));
                fields.insert("toSeconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(v / 1000))))));
                Ok(Value::Object(fields))
            } else {
                Err("Millisecond expects Int".to_string())
            }
        })
    );

    m.insert(
        "now".to_string(),
        Value::NativeCallback(|_args| {
            match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(n) => {
                    let m = n.as_millis() as i64;
                    let nanos = n.as_nanos() as i64;
                    let mut fields = HashMap::new();
                    fields.insert("millis".to_string(), Value::Int(m));
                    fields.insert("nanos".to_string(), Value::Int(nanos));
                    fields.insert("toMillis".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m))))));
                    fields.insert("toSeconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Float((nanos as f64) / 1_000_000_000.0))))));
                    fields.insert("toString".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                        if let chrono::LocalResult::Single(dt) = chrono::Utc.timestamp_millis_opt(m) {
                            Ok(Value::String(dt.to_rfc3339()))
                        } else {
                            Ok(Value::String(format!("{}ms", m)))
                        }
                    }))));
                    Ok(Value::Object(fields))
                },
                Err(_) => Err("SystemTime before UNIX EPOCH!".to_string()),
            }
        }),
    );

    m.insert(
        "fromMillis".to_string(),
        Value::NativeCallback(|args| {
            if args.len() != 1 { return Err("fromMillis expects 1 argument (Int)".to_string()); }
            if let Value::Int(m) = args[0] {
                let mut fields = HashMap::new();
                fields.insert("millis".to_string(), Value::Int(m));
                fields.insert("toMillis".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m))))));
                fields.insert("toSeconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m / 1000))))));
                fields.insert("toString".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                    if let chrono::LocalResult::Single(dt) = chrono::Utc.timestamp_millis_opt(m) {
                        Ok(Value::String(dt.to_rfc3339()))
                    } else {
                        Ok(Value::String(format!("{}ms", m)))
                    }
                }))));
                Ok(Value::Object(fields))
            } else {
                Err("fromMillis expects Int".to_string())
            }
        })
    );
    
    m.insert(
        "fromSeconds".to_string(),
        Value::NativeCallback(|args| {
            if args.len() != 1 { return Err("fromSeconds expects 1 argument (Int)".to_string()); }
            if let Value::Int(s) = args[0] {
                let m = s * 1000;
                let mut fields = HashMap::new();
                fields.insert("millis".to_string(), Value::Int(m));
                fields.insert("toMillis".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m))))));
                fields.insert("toSeconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(s))))));
                fields.insert("toString".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                    if let chrono::LocalResult::Single(dt) = chrono::Utc.timestamp_millis_opt(m) {
                        Ok(Value::String(dt.to_rfc3339()))
                    } else {
                        Ok(Value::String(format!("{}ms", m)))
                    }
                }))));
                Ok(Value::Object(fields))
            } else {
                Err("fromSeconds expects Int".to_string())
            }
        })
    );

    m.insert(
        "parse".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("time.parse requires 1 argument (date string)".to_string());
            }
            if let Value::String(s) = &args[0] {
                use chrono::TimeZone;
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                    let m = dt.timestamp_millis();
                    let mut fields = HashMap::new();
                    fields.insert("millis".to_string(), Value::Int(m));
                    fields.insert("toMillis".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m))))));
                    fields.insert("toSeconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m / 1000))))));
                    fields.insert("toString".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                        if let chrono::LocalResult::Single(dt) = chrono::Utc.timestamp_millis_opt(m) {
                            Ok(Value::String(dt.to_rfc3339()))
                        } else {
                            Ok(Value::String(format!("{}ms", m)))
                        }
                    }))));
                    return Ok(Value::Object(fields));
                }
                if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(s) {
                    let m = dt.timestamp_millis();
                    let mut fields = HashMap::new();
                    fields.insert("millis".to_string(), Value::Int(m));
                    fields.insert("toMillis".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m))))));
                    fields.insert("toSeconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Int(m / 1000))))));
                    fields.insert("toString".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                        if let chrono::LocalResult::Single(dt) = chrono::Utc.timestamp_millis_opt(m) {
                            Ok(Value::String(dt.to_rfc3339()))
                        } else {
                            Ok(Value::String(format!("{}ms", m)))
                        }
                    }))));
                    return Ok(Value::Object(fields));
                }
                return Err(format!("Could not parse time string: {}", s));
            }
            Err("time.parse requires a string".to_string())
        }),
    );

    static PROCESS_EPOCH: std::sync::OnceLock<StdInstant> = std::sync::OnceLock::new();
    let _ = PROCESS_EPOCH.get_or_init(StdInstant::now);

    let perf_counter_fn = Value::NativeCallback(|_args| {
        let elapsed = PROCESS_EPOCH.get().unwrap().elapsed().as_secs_f64();
        Ok(Value::Float(elapsed))
    });

    m.insert("perfCounter".to_string(), perf_counter_fn.clone());
    m.insert("counter".to_string(), perf_counter_fn);

    m.insert(
        "instant".to_string(),
        Value::NativeCallback(|_args| {
            let start = StdInstant::now();
            let mut fields = HashMap::new();
            fields.insert("elapsed".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                let elapsed = start.elapsed();
                let secs_f64 = elapsed.as_secs_f64();
                let millis_f64 = (elapsed.as_nanos() as f64) / 1_000_000.0;
                let millis_i64 = elapsed.as_millis() as i64;
                let nanos_i64 = elapsed.as_nanos() as i64;

                let mut d_fields = HashMap::new();
                d_fields.insert("millis".to_string(), Value::Int(millis_i64));
                d_fields.insert("nanos".to_string(), Value::Int(nanos_i64));
                d_fields.insert("seconds".to_string(), Value::Float(secs_f64));

                let to_millis = Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Float(millis_f64)))));
                d_fields.insert("toMillis".to_string(), to_millis.clone());
                d_fields.insert("toMilliseconds".to_string(), to_millis);

                d_fields.insert("toSeconds".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::Float(secs_f64))))));

                let to_string = format!("{:.4}s", secs_f64);
                d_fields.insert("toString".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| Ok(Value::String(to_string.clone()))))));

                Ok(Value::Object(d_fields))
            }))));
            Ok(Value::Object(fields))
        })
    );

    m
}
