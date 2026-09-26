pub fn get_native_module_def(module: &str) -> Option<crate::vm::NativeModuleDef> {
    let defs = crate::native_std::get_module_defs();
    let search = if module.starts_with("std.") {
        module.to_string()
    } else {
        format!("std.{}", module)
    };
    defs.into_iter()
        .find(|d| d.name == search || d.name == module)
}

pub fn get_std_module_methods(module: &str) -> Option<Vec<String>> {
    get_std_module_symbols(module).map(|syms| syms.into_iter().map(|(name, _)| name).collect())
}

pub fn get_std_module_symbols(module: &str) -> Option<Vec<(String, String)>> {
    let mut parts = module.split('.');
    let base = parts.next()?;

    let base_module = if base == "std" {
        match parts.next() {
            Some(m) if !m.is_empty() => m,
            _ => {
                // If it's just "std" or "std.", suggest the standard modules
                return Some(
                    vec![
                        "os",
                        "fmt",
                        "fs",
                        "byte",
                        "net",
                        "thread",
                        "time",
                        "process",
                        "json",
                        "math",
                        "unit",
                        "window",
                        "desktop",
                        "env",
                        "camera",
                        "web",
                        "annotation",
                    ]
                    .into_iter()
                    .map(|s| (s.to_string(), "module".to_string()))
                    .collect(),
                );
            }
        }
    } else {
        base
    };

    let mut map = match base_module {
        "thread" => Some(crate::native_std::thread::init()),
        "process" => Some(crate::native_std::process::init()),
        "fs" => Some(crate::native_std::fs::init()),
        "byte" => Some(crate::native_std::byte::init()),
        "net" => {
            let sub = parts.next().unwrap_or("ws");
            Some(crate::native_std::net::init(sub))
        }
        "ws" => Some(crate::native_std::net::ws::init()),
        "json" => Some(crate::native_std::json::init()),
        "math" => Some(crate::native_std::math::init()),
        "time" => Some(crate::native_std::time::init()),
        "fmt" => Some(crate::native_std::fmt::init()),
        "os" => Some(crate::native_std::os::init()),
        "desktop" => Some(crate::native_std::desktop::init()),
        "window" => Some(crate::native_std::window::init()),
        "env" => Some(crate::native_std::env::init()),
        "camera" => Some(crate::native_std::camera::init()),
        "unit" => Some(crate::native_std::unit::init()),
        "web" => Some(crate::native_std::web::init()),
        "annotation" | "annotations" => Some(crate::native_std::annotation::init()),
        _ => None,
    }?;

    for part in parts {
        match map.get(part) {
            Some(crate::vm::Value::Object(inner)) | Some(crate::vm::Value::Formula(inner)) => {
                map = inner.clone();
            }
            _ => return None,
        }
    }

    let symbols = map
        .into_iter()
        .map(|(k, v)| {
            let kind = match v {
                crate::vm::Value::NativeCallback(_) | crate::vm::Value::Function { .. } => {
                    "function".to_string()
                }
                _ => {
                    if k.chars().next().map_or(false, |c| c.is_ascii_uppercase()) {
                        "struct".to_string()
                    } else {
                        "property".to_string()
                    }
                }
            };
            (k, kind)
        })
        .collect();

    Some(symbols)
}
