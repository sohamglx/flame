use crate::vm::{Value, NativeModuleDef, NativeFunctionDef, NativeTypeDef};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

pub fn def() -> NativeModuleDef {
    NativeModuleDef {
        name: "std.thread".to_string(),
        description: "OS multi-threading and worker handles".to_string(),
        features: vec!["thread".to_string()],
        functions: vec![
            NativeFunctionDef {
                name: "spawn".to_string(),
                description: "Spawns a new OS kernel thread to execute the given function or callback.".to_string(),
                params: vec![("callback".to_string(), "Function".to_string())],
                return_type: "ThreadHandler".to_string(),
            },
            NativeFunctionDef {
                name: "sleep".to_string(),
                description: "Puts the current thread to sleep for the specified milliseconds.".to_string(),
                params: vec![("ms".to_string(), "Int".to_string())],
                return_type: "Nil".to_string(),
            },
            NativeFunctionDef {
                name: "id".to_string(),
                description: "Returns the real OS kernel thread ID of the current running thread.".to_string(),
                params: vec![],
                return_type: "Int".to_string(),
            },
            NativeFunctionDef {
                name: "yield".to_string(),
                description: "Yields execution to allow other threads or tasks to run.".to_string(),
                params: vec![],
                return_type: "Nil".to_string(),
            },
            NativeFunctionDef {
                name: "channel".to_string(),
                description: "Creates a synchronous cross-thread channel returning (Sender, Receiver).".to_string(),
                params: vec![],
                return_type: "(Sender, Receiver)".to_string(),
            },
        ],
        types: vec![
            NativeTypeDef {
                name: "ThreadHandler".to_string(),
                description: "Handle to a spawned OS worker thread.".to_string(),
                fields: vec![
                    ("id".to_string(), "Int".to_string()),
                    ("osId".to_string(), "Int".to_string()),
                    ("threadId".to_string(), "Int".to_string()),
                ],
                methods: vec![
                    NativeFunctionDef {
                        name: "join".to_string(),
                        description: "Waits for the OS thread to complete execution and returns its result.".to_string(),
                        params: vec![],
                        return_type: "Unknown".to_string(),
                    },
                ],
            },
        ],
    }
}

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    m.insert(
        "sleep".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("thread.sleep expects 1 argument (ms)".to_string());
            }
            if let Value::Int(ms) = args[0] {
                thread::sleep(Duration::from_millis(ms as u64));
                Ok(Value::Nil)
            } else {
                Err(format!("thread.sleep argument must be an Int, got {:?}", args.get(0)))
            }
        }),
    );

    m.insert(
        "spawn".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("thread.spawn expects 1 argument (function or callback)".to_string());
            }
            if let Value::Function { .. } = &args[0] {
                // Note: Real OS thread execution of Flame functions is intercepted directly
                // in runner.rs during member evaluation, where the Runner creates an isolated
                // environment snapshot and spawns a true std::thread with return-value futures.
                // This fallback is only reached if invoked outside an active evaluation loop.
                Ok(Value::ThreadHandler(0))
            } else if let Value::NativeCallback(cb) = args[0] {
                let mut counter = crate::vm::get_thread_counter().lock().unwrap();
                *counter += 1;
                let id = *counter;
                let handle = thread::spawn(move || {
                    let physical_tid = crate::vm::get_current_os_thread_id();
                    crate::vm::get_thread_os_ids().lock().unwrap().insert(id, physical_tid);
                    cb(vec![]).unwrap_or(Value::Nil)
                });
                crate::vm::get_threads().lock().unwrap().insert(id, handle);
                Ok(Value::ThreadHandler(id))
            } else {
                Err("thread.spawn argument must be a function or callback".to_string())
            }
        }),
    );

    m.insert(
        "id".to_string(),
        Value::NativeCallback(|_args| {
            Ok(Value::Int(crate::vm::get_current_os_thread_id()))
        }),
    );

    m.insert(
        "yield".to_string(),
        Value::NativeCallback(|_args| {
            thread::yield_now();
            Ok(Value::Nil)
        }),
    );

    m.insert(
        "yieldNow".to_string(),
        Value::NativeCallback(|_args| {
            thread::yield_now();
            Ok(Value::Nil)
        }),
    );

    m.insert(
        "channel".to_string(),
        Value::NativeCallback(|_args| {
            let mut counter = crate::vm::get_channel_counter().lock().unwrap();
            *counter += 1;
            let chan_id = *counter;

            let (tx, rx) = std::sync::mpsc::channel();
            crate::vm::get_channels().lock().unwrap().insert(chan_id, tx);
            crate::vm::get_receivers()
                .lock()
                .unwrap()
                .insert(chan_id, std::sync::Arc::new(std::sync::Mutex::new(rx)));

            Ok(Value::Tuple(vec![
                Value::Sender(chan_id),
                Value::Receiver(chan_id),
            ]))
        }),
    );

    m
}
