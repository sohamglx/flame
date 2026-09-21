use crate::vm::Value;
use std::collections::HashMap;
use sysinfo;

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    m.insert(
        "name".to_string(),
        Value::NativeCallback(|_args| Ok(Value::String(std::env::consts::OS.to_string()))),
    );

    m.insert(
        "arch".to_string(),
        Value::NativeCallback(|_args| Ok(Value::String(std::env::consts::ARCH.to_string()))),
    );

    m.insert(
        "family".to_string(),
        Value::NativeCallback(|_args| Ok(Value::String(std::env::consts::FAMILY.to_string()))),
    );

    m.insert(
        "hostname".to_string(),
        Value::NativeCallback(|_args| {
            Ok(Value::String(
                sysinfo::System::host_name().unwrap_or_default(),
            ))
        }),
    );

    m.insert(
        "mem".to_string(),
        Value::NativeCallback(|_args| {
            let mut s = sysinfo::System::new();
            s.refresh_memory();

            let totalmem = s.total_memory();
            let freemem = s.free_memory();
            let avmem = s.available_memory();
            let usedmem = s.used_memory();
            let mut map = HashMap::new();
            map.insert("totalMemory".to_string(), Value::Int(totalmem as i64));
            map.insert("freeMemory".to_string(), Value::Int(freemem as i64));
            map.insert("availableMemory".to_string(), Value::Int(avmem as i64));
            map.insert("usedMemory".to_string(), Value::Int(usedmem as i64));

            Ok(Value::Formula(map))
        }),
    );

    m
}
