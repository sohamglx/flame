pub mod byte;
#[cfg(feature = "camera")]
pub mod camera;
#[cfg(any(feature = "os", feature = "automation"))]
pub mod desktop;
pub mod env;
pub mod fmt;
pub mod fs;
pub mod json;
pub mod math;
#[cfg(feature = "net")]
pub mod net;
#[cfg(feature = "os")]
pub mod os;
pub mod process;
pub mod thread;
#[cfg(feature = "utils")]
pub mod time;
pub mod unit;
#[cfg(any(feature = "os", feature = "automation"))]
pub mod window;

use crate::vm::NativeModuleDef;
use crate::vm::{Env, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub fn get_module_defs() -> Vec<NativeModuleDef> {
    #[allow(unused_mut)]
    let mut defs = vec![fmt::def()];

    #[cfg(feature = "utils")]
    defs.push(time::def());

    #[cfg(feature = "net")]
    defs.extend(net::get_module_defs());

    defs
}

/// Helper to define native callbacks in a module
pub fn define_module(env: Arc<Mutex<Env>>, name: &str, init: fn() -> HashMap<String, Value>) {
    let mut e = env.lock().unwrap();
    let mut map = init();
    map.insert("__module__".to_string(), Value::String(name.to_string()));

    // Register top level namespace
    e.define(
        name.strip_prefix("std.").unwrap_or(name).to_string(),
        Value::Formula(map.clone()),
        false,
    );

    // Legacy global functions registration to not break parsing/typing if they rely on it
    for (k, v) in map {
        e.define(k, v, false);
    }
}
