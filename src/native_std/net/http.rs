use crate::vm::{Value, NativeModuleDef, NativeFunctionDef, NativeTypeDef};
use std::collections::HashMap;

#[cfg(feature = "http")]
pub fn def() -> NativeModuleDef {
    NativeModuleDef {
        name: "std.net.http".to_string(),
        description: "HTTP client for sending requests".to_string(),
        features: vec!["http".to_string()],
        functions: vec![
            NativeFunctionDef {
                name: "get".to_string(),
                description: "Sends an HTTP GET request.".to_string(),
                params: vec![("url".to_string(), "String".to_string())],
                return_type: "Response".to_string(),
            },
            NativeFunctionDef {
                name: "post".to_string(),
                description: "Sends an HTTP POST request with a JSON body.".to_string(),
                params: vec![("url".to_string(), "String".to_string()), ("body".to_string(), "Unknown".to_string())],
                return_type: "Response".to_string(),
            },
        ],
        types: vec![
            NativeTypeDef {
                name: "Response".to_string(),
                description: "An HTTP response.".to_string(),
                fields: vec![
                    ("status".to_string(), "Int".to_string()),
                    ("ok".to_string(), "Bool".to_string()),
                ],
                methods: vec![
                    NativeFunctionDef {
                        name: "text".to_string(),
                        description: "Returns the response body as a string.".to_string(),
                        params: vec![],
                        return_type: "String".to_string(),
                    },
                    NativeFunctionDef {
                        name: "json".to_string(),
                        description: "Parses the response body as JSON.".to_string(),
                        params: vec![],
                        return_type: "Formula".to_string(),
                    },
                ],
            }
        ],
    }
}

#[cfg(not(feature = "http"))]
pub fn def() -> NativeModuleDef {
    NativeModuleDef {
        name: "std.net.http".to_string(),
        description: "HTTP client (feature not enabled)".to_string(),
        features: vec![],
        functions: vec![],
        types: vec![],
    }
}

#[cfg(feature = "http")]
pub fn init() -> HashMap<String, Value> {
    let mut map = HashMap::new();

    map.insert(
        "get".to_string(),
        Value::NativeCallback(|args| {
            if args.len() != 1 {
                return Err("http.get expects 1 argument (url)".to_string());
            }
            if let Value::String(url) = &args[0] {
                let url_clone = url.clone();
                let (status, text) = std::thread::spawn(move || {
                    let res = reqwest::blocking::get(&url_clone)
                        .map_err(|e| format!("HTTP GET error: {}", e))?;
                    
                    let status = res.status().as_u16();
                    let text = res.text().unwrap_or_default();
                    Ok::<_, String>((status, text))
                }).join().map_err(|_| "Thread panicked during HTTP GET".to_string())??;

                
                let mut response_obj = HashMap::new();
                response_obj.insert("status".to_string(), Value::Int(status as i64));
                response_obj.insert("ok".to_string(), Value::Bool(status >= 200 && status < 300));
                
                let text_val = Value::String(text.clone());
                response_obj.insert("text".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new({
                    let t = text_val.clone();
                    move |_| Ok(t.clone())
                }))));
                
                response_obj.insert("json".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(crate::native_std::json::json_to_value(&parsed))
                    } else {
                        Err("Failed to parse JSON".to_string())
                    }
                }))));

                Ok(Value::Object(response_obj))
            } else {
                Err("http.get expects a string url".to_string())
            }
        }),
    );

    map.insert(
        "post".to_string(),
        Value::NativeCallback(|args| {
            if args.len() != 2 {
                return Err("http.post expects 2 arguments (url, body)".to_string());
            }
            if let Value::String(url) = &args[0] {
                let body_str = if let Value::String(s) = &args[1] {
                    s.clone()
                } else {
                    crate::native_std::json::value_to_json(&args[1]).to_string()
                };
                
                let url_clone = url.clone();
                let (status, text) = std::thread::spawn(move || {
                    let client = reqwest::blocking::Client::new();
                    let res = client.post(&url_clone).body(body_str)
                        .header("Content-Type", "application/json")
                        .send()
                        .map_err(|e| format!("HTTP POST error: {}", e))?;
                    
                    let status = res.status().as_u16();
                    let text = res.text().unwrap_or_default();
                    Ok::<_, String>((status, text))
                }).join().map_err(|_| "Thread panicked during HTTP POST".to_string())??;
                
                let mut response_obj = HashMap::new();
                response_obj.insert("status".to_string(), Value::Int(status as i64));
                response_obj.insert("ok".to_string(), Value::Bool(status >= 200 && status < 300));
                
                let text_val = Value::String(text.clone());
                response_obj.insert("text".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new({
                    let t = text_val.clone();
                    move |_| Ok(t.clone())
                }))));
                
                response_obj.insert("json".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(crate::native_std::json::json_to_value(&parsed))
                    } else {
                        Err("Failed to parse JSON".to_string())
                    }
                }))));

                Ok(Value::Object(response_obj))
            } else {
                Err("http.post expects (url: String, body: Any)".to_string())
            }
        }),
    );

    map
}

#[cfg(not(feature = "http"))]
pub fn init() -> HashMap<String, Value> {
    HashMap::new()
}
