use crate::vm::{Value, NativeModuleDef, NativeFunctionDef};
use std::collections::HashMap;

pub fn def() -> NativeModuleDef {
    NativeModuleDef {
        name: "std.fmt".to_string(),
        description: "Text formatting utilities".to_string(),
        features: vec![],
        types: vec![],
        functions: vec![
            NativeFunctionDef {
                name: "format".to_string(),
                description: "Formats a string using {} placeholders and additional arguments. Alternatively, use Flame's native string interpolation (e.g., $\"...{var}...\") without this function.".to_string(),
                params: vec![
                    ("template".to_string(), "String".to_string()),
                    ("args".to_string(), "Any...".to_string()),
                ],
                return_type: "String".to_string(),
            },
        ],
    }
}

pub fn stringify_value(v: &Value) -> String {
    match v {
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.to_string(), // In standard format, we omit quotes
        Value::Bool(b) => b.to_string(),
        Value::Nil => "nil".to_string(),
        Value::Byte(b) => format!("0x{:02X}", b),
        Value::Bytes(b) => format!("[{} bytes]", b.len()),
        Value::Tuple(t) => {
            let inner: Vec<String> = t.iter().map(|item| stringify_value(item)).collect();
            format!("({})", inner.join(", "))
        }
        Value::SharedTuple(t) => {
            let inner: Vec<String> = t.iter().map(|item| stringify_value(item)).collect();
            format!("({})", inner.join(", "))
        }
        Value::Formula(m) | Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().filter(|k| !k.starts_with("__")).collect();
            keys.sort();
            let mut inner = Vec::new();
            for k in keys {
                if let Some(val) = m.get(k) {
                    if !matches!(val, Value::NativeCallback(_) | Value::Function { .. }) {
                        let val_str = if let Value::String(s) = val {
                            format!("\"{}\"", s)
                        } else {
                            stringify_value(val)
                        };
                        inner.push(format!("{}: {}", k, val_str));
                    }
                }
            }
            if let Some(Value::String(mod_name)) = m.get("__module__") {
                format!("{} {{ {} }}", mod_name, inner.join(", "))
            } else {
                format!("Object {{ {} }}", inner.join(", "))
            }
        }
        Value::SharedObject(m) => {
            let mut keys: Vec<&String> = m.keys().filter(|k| !k.starts_with("__")).collect();
            keys.sort();
            let mut inner = Vec::new();
            for k in keys {
                if let Some(val) = m.get(k) {
                    if !matches!(val, Value::NativeCallback(_) | Value::Function { .. }) {
                        let val_str = if let Value::String(s) = val {
                            format!("\"{}\"", s)
                        } else {
                            stringify_value(val)
                        };
                        inner.push(format!("{}: {}", k, val_str));
                    }
                }
            }
            if let Some(Value::String(mod_name)) = m.get("__module__") {
                format!("{} {{ {} }}", mod_name, inner.join(", "))
            } else {
                format!("Object {{ {} }}", inner.join(", "))
            }
        }
        Value::StructInstance { name, fields } => {
            let mut keys: Vec<&String> = fields.keys().collect();
            keys.sort();
            let mut inner = Vec::new();
            for k in keys {
                if let Some(val) = fields.get(k) {
                    let val_str = if let Value::String(s) = val {
                        format!("\"{}\"", s)
                    } else {
                        stringify_value(val)
                    };
                    inner.push(format!("{}: {}", k, val_str));
                }
            }
            format!("{} {{ {} }}", name, inner.join(", "))
        }
        Value::EnumValue(enum_name, variant, data) => {
            match data {
                crate::vm::EnumData::Unit => format!("{}::{}", enum_name, variant),
                crate::vm::EnumData::Tuple(t) => {
                    let inner: Vec<String> = t.iter().map(|item| stringify_value(item)).collect();
                    format!("{}::{}({})", enum_name, variant, inner.join(", "))
                }
                crate::vm::EnumData::Struct(fields) => {
                    let mut keys: Vec<&String> = fields.keys().collect();
                    keys.sort();
                    let inner: Vec<String> = keys.iter().map(|k| format!("{}: {}", k, stringify_value(&fields[*k]))).collect();
                    format!("{}::{} {{ {} }}", enum_name, variant, inner.join(", "))
                }
            }
        }
        _ => format!("{:?}", v),
    }
}

pub fn format_string(template: &str, args: &[Value]) -> String {
    let mut result = String::new();
    let mut arg_idx = 0;
    
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                result.push('{');
                chars.next();
                continue;
            }
            
            let mut format_spec = String::new();
            while let Some(&next_c) = chars.peek() {
                if next_c == '}' {
                    chars.next();
                    break;
                } else {
                    format_spec.push(chars.next().unwrap());
                }
            }
            
            if arg_idx < args.len() {
                let val = &args[arg_idx];
                arg_idx += 1;
                
                if format_spec == "?" {
                    result.push_str(&format!("{:?}", val));
                } else {
                    result.push_str(&stringify_value(val));
                }
            } else {
                result.push_str("{}");
            }
        } else if c == '}' {
            if chars.peek() == Some(&'}') {
                result.push('}');
                chars.next();
            } else {
                result.push('}');
            }
        } else {
            result.push(c);
        }
    }
    
    result
}

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    m.insert(
        "format".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fmt.format requires at least 1 argument".to_string());
            }
            if let Value::String(s) = &args[0] {
                Ok(Value::String(format_string(s, &args[1..])))
            } else {
                Err("fmt.format first argument must be a string".to_string())
            }
        })
    );
    
    m
}
