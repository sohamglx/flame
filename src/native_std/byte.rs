use crate::native_std::fs::{extract_bytes, extract_bytes_from_slice, resolve_path};
use crate::vm::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    m.insert(
        "readBytes".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.readBytes expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            match fs::read(&path) {
                Ok(bytes) => Ok(Value::Bytes(bytes)),
                Err(e) => Err(format!("byte.readBytes error: {}", e)),
            }
        }),
    );

    m.insert(
        "writeBytes".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("byte.writeBytes expects 2 arguments (path, bytes)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let bytes = extract_bytes(&args[1]).map_err(|e| format!("byte.writeBytes error: {}", e))?;
            match fs::write(&path, bytes) {
                Ok(_) => Ok(Value::Nil),
                Err(e) => Err(format!("byte.writeBytes error: {}", e)),
            }
        }),
    );

    m.insert(
        "appendBytes".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("byte.appendBytes expects 2 arguments (path, bytes)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let bytes = extract_bytes(&args[1]).map_err(|e| format!("byte.appendBytes error: {}", e))?;
            match fs::OpenOptions::new().append(true).create(true).open(&path) {
                Ok(mut file) => match file.write_all(&bytes) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Err(format!("byte.appendBytes error: {}", e)),
                },
                Err(e) => Err(format!("byte.appendBytes error: {}", e)),
            }
        }),
    );

    m.insert(
        "readByte".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.readByte expects 1 argument (path)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let mut file = match fs::File::open(&path) {
                Ok(file) => file,
                Err(e) => return Err(format!("byte.readByte error: {}", e)),
            };
            let mut byte = [0u8; 1];
            match file.read_exact(&mut byte) {
                Ok(_) => Ok(Value::Byte(byte[0])),
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    Err("byte.readByte error: end of file".to_string())
                }
                Err(e) => Err(format!("byte.readByte error: {}", e)),
            }
        }),
    );

    m.insert(
        "writeByte".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("byte.writeByte expects 2 arguments (path, byte)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let byte = match &args[1] {
                Value::Byte(b) => *b,
                Value::Int(n) => {
                    if *n < 0 || *n > 255 {
                        return Err("byte.writeByte: byte must be between 0 and 255".to_string());
                    }
                    *n as u8
                }
                _ => {
                    return Err(format!(
                        "byte.writeByte: expected Byte, found {}",
                        args[1].type_name()
                    ));
                }
            };
            match fs::write(&path, [byte]) {
                Ok(_) => Ok(Value::Nil),
                Err(e) => Err(format!("byte.writeByte error: {}", e)),
            }
        }),
    );

    m.insert(
        "appendByte".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("byte.appendByte expects 2 arguments (path, byte)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let byte = match &args[1] {
                Value::Byte(b) => *b,
                Value::Int(n) => {
                    if *n < 0 || *n > 255 {
                        return Err("byte.appendByte: byte must be between 0 and 255".to_string());
                    }
                    *n as u8
                }
                _ => {
                    return Err(format!(
                        "byte.appendByte: expected Byte, found {}",
                        args[1].type_name()
                    ));
                }
            };
            match fs::OpenOptions::new().append(true).create(true).open(&path) {
                Ok(mut file) => match file.write_all(&[byte]) {
                    Ok(_) => Ok(Value::Nil),
                    Err(e) => Err(format!("byte.appendByte error: {}", e)),
                },
                Err(e) => Err(format!("byte.appendByte error: {}", e)),
            }
        }),
    );

    m.insert(
        "readByteAt".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("byte.readByteAt expects 2 arguments (path, offset)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let offset = match &args[1] {
                Value::Int(n) if *n >= 0 => *n as u64,
                _ => return Err("byte.readByteAt: offset must be a non-negative integer".to_string()),
            };
            let mut file = match fs::File::open(&path) {
                Ok(f) => f,
                Err(e) => return Err(format!("byte.readByteAt error: {}", e)),
            };
            if let Err(e) = file.seek(SeekFrom::Start(offset)) {
                return Err(format!("byte.readByteAt error: {}", e));
            }
            let mut byte = [0u8; 1];
            match file.read_exact(&mut byte) {
                Ok(_) => Ok(Value::Byte(byte[0])),
                Err(e) => Err(format!("byte.readByteAt error: {}", e)),
            }
        }),
    );

    m.insert(
        "writeByteAt".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 3 {
                return Err("byte.writeByteAt expects 3 arguments (path, offset, byte)".to_string());
            }
            let path = resolve_path(&args[0].to_string().trim_matches('"'));
            let offset = match &args[1] {
                Value::Int(n) if *n >= 0 => *n as u64,
                _ => {
                    return Err(
                        "byte.writeByteAt: offset must be a non-negative integer".to_string()
                    );
                }
            };
            let byte = match &args[2] {
                Value::Byte(b) => *b,
                Value::Int(n) => {
                    if *n < 0 || *n > 255 {
                        return Err("byte.writeByteAt: byte must be between 0 and 255".to_string());
                    }
                    *n as u8
                }
                _ => {
                    return Err(format!(
                        "byte.writeByteAt: expected Byte, found {}",
                        args[2].type_name()
                    ));
                }
            };
            let mut file = match fs::OpenOptions::new().write(true).create(true).open(&path) {
                Ok(f) => f,
                Err(e) => return Err(format!("byte.writeByteAt error: {}", e)),
            };
            if let Err(e) = file.seek(SeekFrom::Start(offset)) {
                return Err(format!("byte.writeByteAt error: {}", e));
            }
            match file.write_all(&[byte]) {
                Ok(_) => Ok(Value::Nil),
                Err(e) => Err(format!("byte.writeByteAt error: {}", e)),
            }
        }),
    );

    m.insert(
        "fromByte".to_string(),
        Value::NativeCallback(|args| {
            if let Some(val) = args.get(0) {
                match val {
                    Value::Byte(b) => Ok(Value::Byte(*b)),
                    Value::Int(n) => {
                        if *n < 0 || *n > 255 {
                            return Err(format!("byte value out of range (0..255): {}", n));
                        }
                        Ok(Value::Byte(*n as u8))
                    }
                    Value::String(s) => {
                        if let Some(b) = s.as_bytes().first() {
                            Ok(Value::Byte(*b))
                        } else {
                            Err("byte.fromByte expects non-empty string".to_string())
                        }
                    }
                    Value::Bytes(b) => {
                        if let Some(first) = b.first() {
                            Ok(Value::Byte(*first))
                        } else {
                            Err("byte.fromByte expects non-empty bytes".to_string())
                        }
                    }
                    _ => Err(format!("byte.fromByte expects Byte, Int, or String, found {}", val.type_name())),
                }
            } else {
                Err("byte.fromByte expects 1 argument".to_string())
            }
        }),
    );

    m.insert(
        "fromBytes".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.fromBytes expects at least 1 argument".to_string());
            }
            if args.len() == 1 {
                let bytes = extract_bytes(&args[0]).map_err(|e| format!("byte.fromBytes error: {}", e))?;
                Ok(Value::Bytes(bytes))
            } else {
                let bytes = extract_bytes_from_slice(&args).map_err(|e| format!("byte.fromBytes error: {}", e))?;
                Ok(Value::Bytes(bytes))
            }
        }),
    );

    m.insert(
        "fromHex".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.fromHex expects 1 argument (hex string)".to_string());
            }
            let s = args[0].to_string();
            let clean = s.trim_matches('"').trim();
            match parse_hex_string(clean) {
                Ok(b) => Ok(Value::Bytes(b)),
                Err(e) => Err(format!("byte.fromHex error: {}", e)),
            }
        }),
    );

    m.insert(
        "buffer".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.buffer expects at least 1 argument (size)".to_string());
            }
            let size = match &args[0] {
                Value::Int(n) if *n >= 0 => *n as usize,
                _ => return Err("byte.buffer: size must be a non-negative integer".to_string()),
            };
            let fill = if args.len() > 1 {
                match &args[1] {
                    Value::Byte(b) => *b,
                    Value::Int(n) if (0..=255).contains(n) => *n as u8,
                    _ => 0u8,
                }
            } else {
                0u8
            };
            Ok(Value::Bytes(vec![fill; size]))
        }),
    );

    m.insert(
        "fromInt16".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.fromInt16 expects 1 argument (number)".to_string());
            }
            let val = args[0].as_int().map_err(|e| format!("byte.fromInt16: {}", e))? as i16;
            let le = args.get(1).map(|v| v.is_truthy()).unwrap_or(true);
            let b = if le { val.to_le_bytes().to_vec() } else { val.to_be_bytes().to_vec() };
            Ok(Value::Bytes(b))
        }),
    );

    m.insert(
        "fromInt32".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.fromInt32 expects 1 argument (number)".to_string());
            }
            let val = args[0].as_int().map_err(|e| format!("byte.fromInt32: {}", e))? as i32;
            let le = args.get(1).map(|v| v.is_truthy()).unwrap_or(true);
            let b = if le { val.to_le_bytes().to_vec() } else { val.to_be_bytes().to_vec() };
            Ok(Value::Bytes(b))
        }),
    );

    m.insert(
        "fromInt64".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.fromInt64 expects 1 argument (number)".to_string());
            }
            let val = args[0].as_int().map_err(|e| format!("byte.fromInt64: {}", e))?;
            let le = args.get(1).map(|v| v.is_truthy()).unwrap_or(true);
            let b = if le { val.to_le_bytes().to_vec() } else { val.to_be_bytes().to_vec() };
            Ok(Value::Bytes(b))
        }),
    );

    m.insert(
        "fromFloat32".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.fromFloat32 expects 1 argument (number)".to_string());
            }
            let val = match &args[0] {
                Value::Float(f) => *f as f32,
                Value::Int(i) => *i as f32,
                _ => return Err("byte.fromFloat32 expects numeric value".to_string()),
            };
            let le = args.get(1).map(|v| v.is_truthy()).unwrap_or(true);
            let b = if le { val.to_le_bytes().to_vec() } else { val.to_be_bytes().to_vec() };
            Ok(Value::Bytes(b))
        }),
    );

    m.insert(
        "fromFloat64".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.fromFloat64 expects 1 argument (number)".to_string());
            }
            let val = match &args[0] {
                Value::Float(f) => *f,
                Value::Int(i) => *i as f64,
                _ => return Err("byte.fromFloat64 expects numeric value".to_string()),
            };
            let le = args.get(1).map(|v| v.is_truthy()).unwrap_or(true);
            let b = if le { val.to_le_bytes().to_vec() } else { val.to_be_bytes().to_vec() };
            Ok(Value::Bytes(b))
        }),
    );

    m.insert(
        "toInt16".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.toInt16 expects at least 1 argument (bytes)".to_string());
            }
            let bytes = extract_bytes(&args[0]).map_err(|e| format!("byte.toInt16: {}", e))?;
            let offset = args.get(1).and_then(|v| v.as_int().ok()).unwrap_or(0) as usize;
            let le = args.get(2).map(|v| v.is_truthy()).unwrap_or(true);
            if offset + 2 > bytes.len() {
                return Err(format!("byte.toInt16: buffer length {} too short for offset {}", bytes.len(), offset));
            }
            let slice: [u8; 2] = [bytes[offset], bytes[offset + 1]];
            let num = if le { i16::from_le_bytes(slice) } else { i16::from_be_bytes(slice) };
            Ok(Value::Int(num as i64))
        }),
    );

    m.insert(
        "toInt32".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.toInt32 expects at least 1 argument (bytes)".to_string());
            }
            let bytes = extract_bytes(&args[0]).map_err(|e| format!("byte.toInt32: {}", e))?;
            let offset = args.get(1).and_then(|v| v.as_int().ok()).unwrap_or(0) as usize;
            let le = args.get(2).map(|v| v.is_truthy()).unwrap_or(true);
            if offset + 4 > bytes.len() {
                return Err(format!("byte.toInt32: buffer length {} too short for offset {}", bytes.len(), offset));
            }
            let slice: [u8; 4] = [bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]];
            let num = if le { i32::from_le_bytes(slice) } else { i32::from_be_bytes(slice) };
            Ok(Value::Int(num as i64))
        }),
    );

    m.insert(
        "toInt64".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.toInt64 expects at least 1 argument (bytes)".to_string());
            }
            let bytes = extract_bytes(&args[0]).map_err(|e| format!("byte.toInt64: {}", e))?;
            let offset = args.get(1).and_then(|v| v.as_int().ok()).unwrap_or(0) as usize;
            let le = args.get(2).map(|v| v.is_truthy()).unwrap_or(true);
            if offset + 8 > bytes.len() {
                return Err(format!("byte.toInt64: buffer length {} too short for offset {}", bytes.len(), offset));
            }
            let mut slice = [0u8; 8];
            slice.copy_from_slice(&bytes[offset..offset + 8]);
            let num = if le { i64::from_le_bytes(slice) } else { i64::from_be_bytes(slice) };
            Ok(Value::Int(num))
        }),
    );

    m.insert(
        "toFloat32".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.toFloat32 expects at least 1 argument (bytes)".to_string());
            }
            let bytes = extract_bytes(&args[0]).map_err(|e| format!("byte.toFloat32: {}", e))?;
            let offset = args.get(1).and_then(|v| v.as_int().ok()).unwrap_or(0) as usize;
            let le = args.get(2).map(|v| v.is_truthy()).unwrap_or(true);
            if offset + 4 > bytes.len() {
                return Err(format!("byte.toFloat32: buffer length {} too short for offset {}", bytes.len(), offset));
            }
            let slice: [u8; 4] = [bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]];
            let num = if le { f32::from_le_bytes(slice) } else { f32::from_be_bytes(slice) };
            Ok(Value::Float(num as f64))
        }),
    );

    m.insert(
        "toFloat64".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("byte.toFloat64 expects at least 1 argument (bytes)".to_string());
            }
            let bytes = extract_bytes(&args[0]).map_err(|e| format!("byte.toFloat64: {}", e))?;
            let offset = args.get(1).and_then(|v| v.as_int().ok()).unwrap_or(0) as usize;
            let le = args.get(2).map(|v| v.is_truthy()).unwrap_or(true);
            if offset + 8 > bytes.len() {
                return Err(format!("byte.toFloat64: buffer length {} too short for offset {}", bytes.len(), offset));
            }
            let mut slice = [0u8; 8];
            slice.copy_from_slice(&bytes[offset..offset + 8]);
            let num = if le { f64::from_le_bytes(slice) } else { f64::from_be_bytes(slice) };
            Ok(Value::Float(num))
        }),
    );

    m.insert(
        "toString".to_string(),
        Value::NativeCallback(|args| {
            if let Some(val) = args.get(0) {
                let bytes = extract_bytes(val).map_err(|e| format!("byte.toString error: {}", e))?;
                Ok(Value::String(String::from_utf8_lossy(&bytes).into_owned()))
            } else {
                Err("byte.toString expects 1 argument".to_string())
            }
        }),
    );

    m.insert(
        "toHex".to_string(),
        Value::NativeCallback(|args| {
            if let Some(val) = args.get(0) {
                let bytes = extract_bytes(val).map_err(|e| format!("byte.toHex error: {}", e))?;
                let hex_str = bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();
                Ok(Value::String(hex_str))
            } else {
                Err("byte.toHex expects 1 argument".to_string())
            }
        }),
    );

    m.insert(
        "toInt".to_string(),
        Value::NativeCallback(|args| {
            if let Some(val) = args.get(0) {
                match val {
                    Value::Byte(b) => Ok(Value::Int(*b as i64)),
                    Value::Int(n) => Ok(Value::Int(*n)),
                    _ => Err(format!("byte.toInt expects Byte or Int, found {}", val.type_name())),
                }
            } else {
                Err("byte.toInt expects 1 argument".to_string())
            }
        }),
    );

    m
}

pub fn parse_hex_string(s: &str) -> Result<Vec<u8>, String> {
    let mut clean = s.trim().to_string();
    if clean.starts_with("0x") || clean.starts_with("0X") {
        clean = clean[2..].to_string();
    }
    clean = clean
        .replace("0x", "")
        .replace("0X", "")
        .replace([',', ' ', '\t', '\n', '\r'], "");
    if clean.len() % 2 != 0 {
        return Err("hex string must have an even number of hex characters".to_string());
    }
    let mut bytes = Vec::with_capacity(clean.len() / 2);
    for i in (0..clean.len()).step_by(2) {
        let byte_str = &clean[i..i + 2];
        match u8::from_str_radix(byte_str, 16) {
            Ok(b) => bytes.push(b),
            Err(e) => return Err(format!("invalid hex byte '{}': {}", byte_str, e)),
        }
    }
    Ok(bytes)
}
