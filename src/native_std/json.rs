use crate::vm::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use serde::de::DeserializeSeed;
use serde_json::Value as JsonValue;

pub struct FastValueVisitor;

impl<'de> serde::de::Visitor<'de> for FastValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    #[inline]
    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }

    #[inline]
    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Int(v))
    }

    #[inline]
    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::Int(v as i64))
    }

    #[inline]
    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(Value::Float(v))
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_string()))
    }

    #[inline]
    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Nil)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Nil)
    }

    #[inline]
    fn visit_seq<A>(self, mut seq: A) -> Result<Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let capacity = seq.size_hint().unwrap_or(0);
        let mut vec = Vec::with_capacity(capacity);
        while let Some(elem) = seq.next_element_seed(FastValueSeed)? {
            vec.push(elem);
        }
        Ok(Value::SharedTuple(std::sync::Arc::new(vec)))
    }

    #[inline]
    fn visit_map<M>(self, mut access: M) -> Result<Value, M::Error>
    where
        M: serde::de::MapAccess<'de>,
    {
        let capacity = access.size_hint().unwrap_or(0);
        let mut map = HashMap::with_capacity(capacity);
        while let Some((key, val)) = access.next_entry_seed(
            std::marker::PhantomData::<String>,
            FastValueSeed,
        )? {
            map.insert(key, val);
        }
        Ok(Value::SharedObject(std::sync::Arc::new(map)))
    }
}

#[derive(Clone, Copy)]
pub struct FastValueSeed;

impl<'de> serde::de::DeserializeSeed<'de> for FastValueSeed {
    type Value = Value;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FastValueVisitor)
    }
}

#[inline]
pub fn parse_json_str(s: &str) -> Result<Value, String> {
    let mut de = serde_json::Deserializer::from_str(s);
    FastValueSeed.deserialize(&mut de).map_err(|e| format!("JSON parse error: {}", e))
}

#[inline]
pub fn parse_json_reader<R: std::io::Read>(reader: R) -> Result<Value, String> {
    let mut de = serde_json::Deserializer::from_reader(reader);
    FastValueSeed.deserialize(&mut de).map_err(|e| format!("JSON parse error: {}", e))
}

pub fn parse_json_file(path: &Path) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read JSON file '{}': {}", path.display(), e))?;
    let mut de = serde_json::Deserializer::from_slice(&bytes);
    FastValueSeed.deserialize(&mut de).map_err(|e| format!("JSON parse error in '{}': {}", path.display(), e))
}

pub fn init() -> HashMap<String, Value> {
    let mut map = HashMap::new();

    // json.parse(str)
    map.insert(
        "parse".to_string(),
        Value::NativeCallback(|args| {
            if let Some(val) = args.get(0) {
                match val {
                    Value::String(json_str) => parse_json_str(json_str),
                    Value::Formula(_) | Value::Object(_) => Ok(val.clone()),
                    _ => Err("json.parse expects a string or object".to_string()),
                }
            } else {
                Err("json.parse expects an argument".to_string())
            }
        }),
    );

    // json.fromJson(str_or_bytes)
    map.insert(
        "fromJson".to_string(),
        Value::NativeCallback(|args| {
            if let Some(val) = args.get(0) {
                match val {
                    Value::String(s) => parse_json_str(s),
                    Value::Bytes(b) => {
                        let reader = std::io::Cursor::new(b);
                        parse_json_reader(reader)
                    }
                    _ => Err("json.fromJson expects a string or bytes".to_string()),
                }
            } else {
                Err("json.fromJson expects an argument".to_string())
            }
        }),
    );

    // json.read(filepath) - streaming direct fast file reader
    map.insert(
        "read".to_string(),
        Value::NativeCallback(|args| {
            if let Some(val) = args.get(0) {
                match val {
                    Value::String(path_str) => parse_json_file(Path::new(path_str)),
                    _ => Err("json.read expects a file path string".to_string()),
                }
            } else {
                Err("json.read expects a file path argument".to_string())
            }
        }),
    );

    // json.stringify(val, [pretty])
    map.insert(
        "stringify".to_string(),
        Value::NativeCallback(|args| {
            if let Some(val) = args.get(0) {
                let pretty = args.get(1).map(|v| match v {
                    Value::Bool(b) => *b,
                    Value::Int(i) => *i > 0,
                    _ => false,
                }).unwrap_or(false);

                let json_val = value_to_json(val);
                let res = if pretty {
                    serde_json::to_string_pretty(&json_val).map_err(|e| e.to_string())?
                } else {
                    json_val.to_string()
                };
                Ok(Value::String(res))
            } else {
                Err("json.stringify expects an argument".to_string())
            }
        }),
    );

    // json.write(filepath, val, [pretty])
    map.insert(
        "write".to_string(),
        Value::NativeCallback(|args| {
            if args.len() >= 2 {
                let path_str = match &args[0] {
                    Value::String(s) => s,
                    _ => return Err("json.write expects a file path string as first argument".to_string()),
                };
                let pretty = args.get(2).map(|v| match v {
                    Value::Bool(b) => *b,
                    Value::Int(i) => *i > 0,
                    _ => false,
                }).unwrap_or(false);

                let json_val = value_to_json(&args[1]);
                let file = File::create(path_str).map_err(|e| format!("Failed to create file '{}': {}", path_str, e))?;
                let mut writer = BufWriter::new(file);
                if pretty {
                    serde_json::to_writer_pretty(&mut writer, &json_val).map_err(|e| e.to_string())?;
                } else {
                    serde_json::to_writer(&mut writer, &json_val).map_err(|e| e.to_string())?;
                }
                writer.flush().map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            } else {
                Err("json.write expects (filepath, value, [pretty])".to_string())
            }
        }),
    );

    // json.valid(str) - validates JSON syntax quickly without building full tree
    map.insert(
        "valid".to_string(),
        Value::NativeCallback(|args| {
            if let Some(Value::String(s)) = args.get(0) {
                let valid = serde_json::from_str::<serde::de::IgnoredAny>(s).is_ok();
                Ok(Value::Bool(valid))
            } else {
                Err("json.valid expects a string argument".to_string())
            }
        }),
    );

    map
}

pub fn json_to_value(json: &JsonValue) -> Value {
    match json {
        JsonValue::Null => Value::Nil,
        JsonValue::Bool(b) => Value::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Nil
            }
        }
        JsonValue::String(s) => Value::String(s.clone()),
        JsonValue::Array(arr) => {
            let mut vec = Vec::with_capacity(arr.len());
            for item in arr {
                vec.push(json_to_value(item));
            }
            Value::SharedTuple(std::sync::Arc::new(vec))
        }
        JsonValue::Object(obj) => {
            let mut map = HashMap::with_capacity(obj.len());
            for (k, v) in obj {
                map.insert(k.clone(), json_to_value(v));
            }
            Value::SharedObject(std::sync::Arc::new(map))
        }
    }
}

pub fn value_to_json(val: &Value) -> JsonValue {
    match val {
        Value::Nil => JsonValue::Null,
        Value::Bool(b) => JsonValue::Bool(*b),
        Value::Int(i) => JsonValue::Number((*i).into()),
        Value::Float(f) => {
            if let Some(n) = serde_json::Number::from_f64(*f) {
                JsonValue::Number(n)
            } else {
                JsonValue::Null
            }
        }
        Value::String(s) => JsonValue::String(s.clone()),
        Value::Tuple(arr) => {
            let mut vec = Vec::with_capacity(arr.len());
            for item in arr {
                vec.push(value_to_json(item));
            }
            JsonValue::Array(vec)
        }
        Value::SharedTuple(arr) => {
            let mut vec = Vec::with_capacity(arr.len());
            for item in arr.iter() {
                vec.push(value_to_json(item));
            }
            JsonValue::Array(vec)
        }
        Value::Object(obj) | Value::Formula(obj) => {
            let mut map = serde_json::Map::with_capacity(obj.len());
            for (k, v) in obj {
                map.insert(k.clone(), value_to_json(v));
            }
            JsonValue::Object(map)
        }
        Value::SharedObject(obj) => {
            let mut map = serde_json::Map::with_capacity(obj.len());
            for (k, v) in obj.iter() {
                map.insert(k.clone(), value_to_json(v));
            }
            JsonValue::Object(map)
        }
        _ => JsonValue::String(format!("{:?}", val)),
    }
}
