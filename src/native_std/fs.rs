use crate::vm::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{PathBuf};
use std::env;

pub fn resolve_path(path_str: &str) -> PathBuf {
    let p = std::path::Path::new(path_str);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let base = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let direct = base.join(p);
    if direct.exists() {
        return direct;
    }
    let in_src = base.join("src").join(p);
    if in_src.exists() {
        return in_src;
    }
    direct
}

pub fn extract_bytes_from_slice(items: &[Value]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::Byte(b) => buf.push(*b),
            Value::Int(n) => {
                if *n < 0 || *n > 255 {
                    return Err(format!("byte value out of range (0..255): {}", n));
                }
                buf.push(*n as u8);
            }
            Value::String(s) => {
                let trimmed = s.trim();
                if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                    if let Ok(val) = u8::from_str_radix(&trimmed[2..], 16) {
                        buf.push(val);
                        continue;
                    }
                }
                for b in s.as_bytes() {
                    buf.push(*b);
                }
            }
            _ => return Err(format!("expected byte/int in byte array, found {}", item.type_name())),
        }
    }
    Ok(buf)
}

pub fn extract_bytes(val: &Value) -> Result<Vec<u8>, String> {
    match val {
        Value::Bytes(b) => Ok(b.clone()),
        Value::Byte(b) => Ok(vec![*b]),
        Value::Int(n) => {
            if *n < 0 || *n > 255 {
                return Err(format!("byte value out of range (0..255): {}", n));
            }
            Ok(vec![*n as u8])
        }
        Value::String(s) => Ok(s.as_bytes().to_vec()),
        Value::Tuple(items) => extract_bytes_from_slice(items),
        Value::SharedTuple(items) => extract_bytes_from_slice(items),
        _ => Err(format!("expected Bytes, Byte, [Byte], or String, found {}", val.type_name())),
    }
}

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    m.insert(
        "read".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.read expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            match fs::read_to_string(&path) {
                Ok(content) => Ok(Value::String(content)),
                Err(e) => Err(format!("fs.read error: {}", e)),
            }
        }),
    );

    m.insert(
        "write".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("fs.write expects 2 arguments (path, content)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            match &args[1] {
                Value::Bytes(b) => match fs::write(&path, b) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Err(format!("fs.write error: {}", e)),
                },
                Value::Byte(b) => match fs::write(&path, [*b]) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Err(format!("fs.write error: {}", e)),
                },
                Value::Tuple(items) if !items.is_empty() && items.iter().all(|it| matches!(it, Value::Byte(_) | Value::Int(_))) => {
                    let bytes = extract_bytes_from_slice(items).map_err(|e| format!("fs.write error: {}", e))?;
                    match fs::write(&path, bytes) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("fs.write error: {}", e)),
                    }
                }
                Value::SharedTuple(items) if !items.is_empty() && items.iter().all(|it| matches!(it, Value::Byte(_) | Value::Int(_))) => {
                    let bytes = extract_bytes_from_slice(items).map_err(|e| format!("fs.write error: {}", e))?;
                    match fs::write(&path, bytes) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("fs.write error: {}", e)),
                    }
                }
                Value::String(s) => match fs::write(&path, s.as_bytes()) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Err(format!("fs.write error: {}", e)),
                },
                v => match fs::write(&path, v.to_string().as_bytes()) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Err(format!("fs.write error: {}", e)),
                }
            }
        }),
    );

    m.insert(
        "readBytes".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.readBytes expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            match std::fs::read(&path) {
                Ok(bytes) => Ok(Value::Bytes(bytes)),
                Err(e) => Err(format!("fs.readBytes error: {}", e)),
            }
        }),
    );

    m.insert(
        "writeBytes".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("fs.writeBytes expects 2 arguments (path, bytes)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let bytes = extract_bytes(&args[1]).map_err(|e| format!("fs.writeBytes error: {}", e))?;
            match std::fs::write(&path, bytes) {
                Ok(_) => Ok(Value::Nil),
                Err(e) => Err(format!("fs.writeBytes error: {}", e)),
            }
        }),
    );

    m.insert(
        "appendBytes".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("fs.appendBytes expects 2 arguments (path, bytes)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let bytes = extract_bytes(&args[1]).map_err(|e| format!("fs.appendBytes error: {}", e))?;
            match std::fs::OpenOptions::new().append(true).create(true).open(&path) {
                Ok(mut file) => {
                    use std::io::Write;
                    match file.write_all(&bytes) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("fs.appendBytes error: {}", e)),
                    }
                },
                Err(e) => Err(format!("fs.appendBytes error: {}", e)),
            }
        }),
    );

    m.insert(
        "append".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("fs.append expects 2 arguments (path, content)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let bytes = match &args[1] {
                Value::Bytes(b) => b.clone(),
                Value::Byte(b) => vec![*b],
                Value::Tuple(items) if !items.is_empty() && items.iter().all(|it| matches!(it, Value::Byte(_) | Value::Int(_))) => {
                    extract_bytes_from_slice(items).map_err(|e| format!("fs.append error: {}", e))?
                }
                Value::SharedTuple(items) if !items.is_empty() && items.iter().all(|it| matches!(it, Value::Byte(_) | Value::Int(_))) => {
                    extract_bytes_from_slice(items).map_err(|e| format!("fs.append error: {}", e))?
                }
                Value::String(s) => s.as_bytes().to_vec(),
                v => v.to_string().into_bytes(),
            };
            match std::fs::OpenOptions::new().append(true).create(true).open(&path) {
                Ok(mut file) => {
                    use std::io::Write;
                    match file.write_all(&bytes) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("fs.append error: {}", e)),
                    }
                }
                Err(e) => Err(format!("fs.append error: {}", e)),
            }
        }),
    );

    m.insert(
        "exists".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.exists expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            Ok(Value::Bool(path.exists()))
        }),
    );

    m.insert(
        "readDir".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.readDir expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            match fs::read_dir(&path) {
                Ok(entries) => {
                    let mut files = Vec::new();
                    for entry in entries.flatten() {
                        if let Ok(name) = entry.file_name().into_string() {
                            files.push(Value::String(name));
                        }
                    }
                    Ok(Value::Tuple(files))
                }
                Err(e) => Err(format!("fs.readDir error: {}", e)),
            }
        }),
    );

    m.insert(
        "isDir".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.isDir expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            Ok(Value::Bool(path.is_dir()))
        }),
    );

    m.insert(
        "isFile".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.isFile expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            Ok(Value::Bool(path.is_file()))
        }),
    );

    m.insert(
        "size".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.size expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            match fs::metadata(&path) {
                Ok(meta) => Ok(Value::Int(meta.len() as i64)),
                Err(e) => Err(format!("fs.size error: {}", e)),
            }
        }),
    );

    m.insert(
        "remove".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.remove expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let p = path.as_path();
            if p.is_dir() {
                if let Err(e) = fs::remove_dir_all(p) {
                    return Err(format!("fs.remove error: {}", e));
                }
            } else {
                if let Err(e) = fs::remove_file(p) {
                    return Err(format!("fs.remove error: {}", e));
                }
            }
            Ok(Value::Nil)
        }),
    );

    m.insert(
        "delete".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.delete expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let p = path.as_path();
            if p.is_dir() {
                if let Err(e) = fs::remove_dir_all(p) {
                    return Err(format!("fs.delete error: {}", e));
                }
            } else {
                if let Err(e) = fs::remove_file(p) {
                    return Err(format!("fs.delete error: {}", e));
                }
            }
            Ok(Value::Nil)
        }),
    );
    
    m.insert(
        "mkdir".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.mkdir expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            match fs::create_dir(&path) {
                Ok(_) => Ok(Value::Nil),
                Err(e) => Err(format!("fs.mkdir error: {}", e)),
            }
        }),
    );

    m.insert(
        "mkdir_all".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("fs.mkdir_all expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            match fs::create_dir_all(&path) {
                Ok(_) => Ok(Value::Nil),
                Err(e) => Err(format!("fs.mkdir_all error: {}", e)),
            }
        }),
    );

    m.insert(
        "copy".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("fs.copy expects 2 arguments (src, dest)".to_string());
            }
            let src = resolve_path(&args[0].to_string().trim_matches('"'));
            let dest = resolve_path(&args[1].to_string().trim_matches('"'));
            match fs::copy(&src, &dest) {
                Ok(_) => Ok(Value::Nil),
                Err(e) => Err(format!("fs.copy error: {}", e)),
            }
        }),
    );

    m.insert(
        "open".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() { return Err("File.open expects 1 argument (path)".to_string()); }
            let path_str = args[0].to_string().trim_matches('"').to_string();
            
            let mut file_instance = std::collections::HashMap::new();
            file_instance.insert("path".to_string(), Value::String(path_str.clone()));
            
            let p1 = path_str.clone();
            file_instance.insert("read".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                match fs::read_to_string(resolve_path(&p1)) {
                    Ok(c) => Ok(Value::String(c)),
                    Err(e) => Err(format!("fs.read error: {}", e)),
                }
            }))));
            
            let p2 = path_str.clone();
            file_instance.insert("write".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |args2| {
                if args2.len() < 2 { return Err("write expects 1 argument".to_string()); }
                let path = resolve_path(&p2);
                match &args2[1] {
                    Value::Bytes(b) => match fs::write(&path, b) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("fs.write error: {}", e)),
                    },
                    Value::Byte(b) => match fs::write(&path, [*b]) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("fs.write error: {}", e)),
                    },
                    Value::Tuple(items) if !items.is_empty() && items.iter().all(|it| matches!(it, Value::Byte(_) | Value::Int(_))) => {
                        let bytes = extract_bytes_from_slice(items).map_err(|e| format!("fs.write error: {}", e))?;
                        match fs::write(&path, bytes) {
                            Ok(_) => Ok(Value::Nil),
                            Err(e) => Err(format!("fs.write error: {}", e)),
                        }
                    }
                    Value::SharedTuple(items) if !items.is_empty() && items.iter().all(|it| matches!(it, Value::Byte(_) | Value::Int(_))) => {
                        let bytes = extract_bytes_from_slice(items).map_err(|e| format!("fs.write error: {}", e))?;
                        match fs::write(&path, bytes) {
                            Ok(_) => Ok(Value::Nil),
                            Err(e) => Err(format!("fs.write error: {}", e)),
                        }
                    }
                    Value::String(s) => match fs::write(&path, s.as_bytes()) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("fs.write error: {}", e)),
                    },
                    v => match fs::write(&path, v.to_string().as_bytes()) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("fs.write error: {}", e)),
                    }
                }
            }))));
            
            let p3 = path_str.clone();
            file_instance.insert("append".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |args3| {
                if args3.len() < 2 { return Err("append expects 1 argument".to_string()); }
                let bytes = match &args3[1] {
                    Value::Bytes(b) => b.clone(),
                    Value::Byte(b) => vec![*b],
                    Value::Tuple(items) if !items.is_empty() && items.iter().all(|it| matches!(it, Value::Byte(_) | Value::Int(_))) => {
                        extract_bytes_from_slice(items).map_err(|e| format!("fs.append error: {}", e))?
                    }
                    Value::SharedTuple(items) if !items.is_empty() && items.iter().all(|it| matches!(it, Value::Byte(_) | Value::Int(_))) => {
                        extract_bytes_from_slice(items).map_err(|e| format!("fs.append error: {}", e))?
                    }
                    Value::String(s) => s.as_bytes().to_vec(),
                    v => v.to_string().into_bytes(),
                };
                use std::io::Write;
                match fs::OpenOptions::new().append(true).create(true).open(resolve_path(&p3)) {
                    Ok(mut f) => match f.write_all(&bytes) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("fs.append error: {}", e)),
                    },
                    Err(e) => Err(format!("fs.append error: {}", e)),
                }
            }))));

            let p_rb = path_str.clone();
            file_instance.insert("readBytes".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                match fs::read(resolve_path(&p_rb)) {
                    Ok(bytes) => Ok(Value::Bytes(bytes)),
                    Err(e) => Err(format!("File.readBytes error: {}", e)),
                }
            }))));

            let p_wb = path_str.clone();
            file_instance.insert("writeBytes".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |args_wb| {
                if args_wb.len() < 2 { return Err("writeBytes expects 1 argument".to_string()); }
                let bytes = extract_bytes(&args_wb[1]).map_err(|e| format!("File.writeBytes error: {}", e))?;
                match fs::write(resolve_path(&p_wb), bytes) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Err(format!("File.writeBytes error: {}", e)),
                }
            }))));

            let p_ab = path_str.clone();
            file_instance.insert("appendBytes".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |args_ab| {
                if args_ab.len() < 2 { return Err("appendBytes expects 1 argument".to_string()); }
                let bytes = extract_bytes(&args_ab[1]).map_err(|e| format!("File.appendBytes error: {}", e))?;
                use std::io::Write;
                match fs::OpenOptions::new().append(true).create(true).open(resolve_path(&p_ab)) {
                    Ok(mut f) => match f.write_all(&bytes) {
                        Ok(_) => Ok(Value::Nil),
                        Err(e) => Err(format!("File.appendBytes error: {}", e)),
                    },
                    Err(e) => Err(format!("File.appendBytes error: {}", e)),
                }
            }))));
            
            let p4 = path_str.clone();
            file_instance.insert("exists".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                Ok(Value::Bool(resolve_path(&p4).exists()))
            }))));
            
            let p5 = path_str.clone();
            file_instance.insert("size".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                match fs::metadata(resolve_path(&p5)) {
                    Ok(m) => Ok(Value::Int(m.len() as i64)),
                    Err(_) => Ok(Value::Int(0)),
                }
            }))));
            
            let p6 = path_str.clone();
            file_instance.insert("delete".to_string(), Value::NativeClosure(crate::vm::NativeClosureType(std::sync::Arc::new(move |_| {
                match fs::remove_file(resolve_path(&p6)) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Err(format!("fs.delete error: {}", e)),
                }
            }))));
            
            Ok(Value::Object(file_instance))
        })
    );

    m
}
