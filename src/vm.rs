use crate::parser::{Annotation, Param, Stmt};
use std::collections::HashMap;
use std::fmt;
use std::process::Child;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
    Byte(u8),
    Bytes(Vec<u8>),
    Tuple(Vec<Value>),
    Formula(HashMap<String, Value>),
    Unit(HashMap<String, i32>),
    Quantity(f64, HashMap<String, i32>),
    ThreadHandler(u64),
    Sender(u64),
    Receiver(u64),
    ChildProcess(u64),
    CommandBuilder {
        program: String,
        args: Vec<String>,
    },
    RustServer {
        port: u16,
    },
    StructConstructor {
        name: String,
        fields: Vec<(String, String)>,
    },
    StructInstance {
        name: String,
        fields: HashMap<String, Value>,
    },
    Function {
        params: Vec<Param>,
        body: Vec<Stmt>,
        env: Arc<Mutex<Env>>,
        annotations: Vec<Annotation>,
    },
    Break,
    Return(Box<Value>),
    Moved(String),
    Ref(Box<Value>),
    RefPath(RefPath, bool),
    NativeObject {
        crate_name: String,
        type_name: String,
        ptr: usize,
    },
    Object(HashMap<String, Value>),
    NativeFunction(fn(*const CValue, usize) -> CValue),
    NativeCallback(fn(Vec<Value>) -> Result<Value, String>),
    NativeClosure(NativeClosureType),
    Range(i64, i64),
    EnumMeta(String, Vec<crate::parser::EnumVariant>),
    EnumValue(String, String, EnumData),
    VariantConstructor(String, crate::parser::EnumVariant),
}

#[derive(Debug, Clone)]
pub enum RefPath {
    Var(String, Arc<Mutex<Env>>),
    Field {
        owner: String,
        member: String,
        env: Arc<Mutex<Env>>,
    },
    Index {
        owner: String,
        index: usize,
        env: Arc<Mutex<Env>>,
    },
}

#[derive(Debug, Clone)]
pub enum EnumData {
    Unit,
    Tuple(Vec<Value>),
    Struct(HashMap<String, Value>),
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CValueTag {
    Null,
    Int,
    Float,
    Bool,
    String,
    NativeObject,
    Range,
    Function,
    EnumVariant,
    Array,
}

#[derive(Debug, Clone)]
pub struct NativeModuleDef {
    pub name: String,
    pub description: String,
    pub functions: Vec<NativeFunctionDef>,
    pub types: Vec<NativeTypeDef>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NativeFunctionDef {
    pub name: String,
    pub description: String,
    pub params: Vec<(String, String)>, // (name, type)
    pub return_type: String,
}

#[derive(Debug, Clone)]
pub struct NativeTypeDef {
    pub name: String,
    pub description: String,
    pub fields: Vec<(String, String)>,
    pub methods: Vec<NativeFunctionDef>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlameCallback {
    pub function_id: u64,
    pub module_id: u64,
}

pub struct CallbackRequest {
    pub callback: FlameCallback,
    pub args: Vec<CValue>,
    pub responder: std::sync::mpsc::Sender<CValue>,
}

static RUNTIME_QUEUE: OnceLock<(
    Mutex<std::sync::mpsc::Sender<CallbackRequest>>,
    Mutex<std::sync::mpsc::Receiver<CallbackRequest>>,
)> = OnceLock::new();
static CALLBACK_REGISTRY: OnceLock<Mutex<HashMap<u64, Value>>> = OnceLock::new();
static EVENT_LOOP_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
#[derive(Clone)]
pub struct NativeClosureType(pub std::sync::Arc<dyn Fn(Vec<Value>) -> Result<Value, String> + Send + Sync>);

impl std::fmt::Debug for NativeClosureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeClosure")
    }
}

pub fn set_event_loop_active(active: bool) {
    EVENT_LOOP_ACTIVE.store(active, std::sync::atomic::Ordering::SeqCst);
}

pub fn is_event_loop_active() -> bool {
    EVENT_LOOP_ACTIVE.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn register_callback_value(val: Value) -> u64 {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut reg = CALLBACK_REGISTRY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap();
    reg.insert(id, val);
    id
}

pub fn get_callback_value(id: u64) -> Option<Value> {
    let reg = CALLBACK_REGISTRY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap();
    reg.get(&id).cloned()
}

pub fn get_runtime_queue() -> &'static (
    Mutex<std::sync::mpsc::Sender<CallbackRequest>>,
    Mutex<std::sync::mpsc::Receiver<CallbackRequest>>,
) {
    RUNTIME_QUEUE.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel();
        (Mutex::new(tx), Mutex::new(rx))
    })
}

pub fn enqueue_callback(callback: FlameCallback, args: Vec<CValue>) -> Result<CValue, String> {
    let cb_val = get_callback_value(callback.function_id)
        .ok_or_else(|| format!("Callback ID {} not found", callback.function_id))?;

    let mut flame_args = Vec::new();
    for cval in args {
        flame_args.push(Value::unpack(cval, "", ""));
    }

    let mut runner = crate::runner::Runner::new(std::path::PathBuf::from("native_callback"));
    let res = runner.invoke_callback_value(&cb_val, flame_args)?;
    Ok(res.pack())
}

pub fn invoke_callback_val(cb_val: &Value, args: Vec<Value>) -> Result<Value, String> {
    let mut runner = crate::runner::Runner::new(std::path::PathBuf::from("native_callback"));
    runner.invoke_callback_value(cb_val, args)
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CValue {
    pub tag: CValueTag,
    pub int_val: i64,
    pub int_val2: i64,
    pub float_val: f64,
    pub bool_val: bool,
    pub string_ptr: *mut std::os::raw::c_char,
    pub obj_ptr: *mut std::ffi::c_void,
}

unsafe impl Send for CValue {}
unsafe impl Sync for CValue {}

impl CValue {
    pub fn null() -> Self {
        Self {
            tag: CValueTag::Null,
            int_val: 0,
            int_val2: 0,
            float_val: 0.0,
            bool_val: false,
            string_ptr: std::ptr::null_mut(),
            obj_ptr: std::ptr::null_mut(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        let c_str = std::ffi::CString::new(s).unwrap_or_default();
        let mut cv = Self::null();
        cv.tag = CValueTag::String;
        cv.string_ptr = c_str.into_raw();
        cv
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::NativeObject { type_name, .. } => write!(f, "<native object: {}>", type_name),
            Value::Moved(name) => write!(f, "<moved {}>", name),
            Value::Ref(inner) => inner.fmt(f),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nil => write!(f, "nil"),
            Value::Byte(b) => write!(f, "{}", b),
            Value::Bytes(bytes) => {
                let hex: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();

                write!(f, "bytes[{}]", hex.join(" "))
            }
            Value::Tuple(items) => {
                let s: Vec<String> = items.iter().map(|it| it.to_string()).collect();
                write!(f, "({})", s.join(", "))
            }
            Value::Formula(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let s: Vec<String> = keys.into_iter().map(|k| format!("{}: {}", k, map[k])).collect();
                write!(f, "formula {{ {} }}", s.join(", "))
            }
            Value::Unit(map) => {
                let mut terms = Vec::new();
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for k in keys {
                    let v = map[k];
                    if v == 1 { terms.push(k.clone()); }
                    else { terms.push(format!("{}^{}", k, v)); }
                }
                if terms.is_empty() {
                    write!(f, "unit {{}}")
                } else {
                    write!(f, "unit {{ {} }}", terms.join(" * "))
                }
            }
            Value::Quantity(v, map) => {
                let mut terms = Vec::new();
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for k in keys {
                    let val = map[k];
                    if val == 1 { terms.push(k.clone()); }
                    else { terms.push(format!("{}^{}", k, val)); }
                }
                if terms.is_empty() {
                    write!(f, "{}", v)
                } else {
                    write!(f, "{} {}", v, terms.join(" * "))
                }
            }
            Value::Object(map) => {
                let mut keys: Vec<String> = map.keys().cloned().collect();
                keys.sort();
                write!(
                    f,
                    "<object [{}]>",
                    keys.join(", ")
                )
            }
            Value::Range(start, end) => write!(f, "{}..{}", start, end),
            Value::ThreadHandler(id) => write!(f, "ThreadHandler({})", id),
            Value::Sender(id) => write!(f, "Sender({})", id),
            Value::Receiver(id) => write!(f, "Receiver({})", id),
            Value::ChildProcess(id) => write!(f, "ChildProcess({})", id),
            Value::CommandBuilder { program, args } => {
                write!(f, "CommandBuilder({} {})", program, args.join(" "))
            }
            Value::RustServer { port } => write!(f, "RustServer(127.0.0.1:{})", port),
            Value::StructConstructor { name, .. } => write!(f, "<struct constructor: {}>", name),
            Value::StructInstance { name, fields } => {
                let s: Vec<String> = fields.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{} {{ {} }}", name, s.join(", "))
            }
            Value::Function { .. } => write!(f, "<function>"),
            Value::Break => write!(f, "<break>"),
            Value::Return(val) => write!(f, "<return {}>", val),
            Value::NativeFunction(_) => write!(f, "<native function>"),
            Value::NativeCallback(_) => write!(f, "<native callback>"),
            Value::NativeClosure(_) => write!(f, "<native closure>"),
            Value::EnumMeta(name, _) => write!(f, "<enum {}>", name),
            Value::EnumValue(enum_name, variant_name, data) => match data {
                EnumData::Unit => {
                    write!(f, "{}.{}", enum_name, variant_name)
                }

                EnumData::Tuple(items) => {
                    let s: Vec<String> = items.iter().map(|it| it.to_string()).collect();

                    write!(f, "{}.{}({})", enum_name, variant_name, s.join(", "))
                }

                EnumData::Struct(map) => {
                    let s: Vec<String> = map.iter().map(|(k, v)| format!("{k}: {v}")).collect();

                    write!(f, "{}.{} {{ {} }}", enum_name, variant_name, s.join(", "))
                }
            },
            Value::VariantConstructor(enum_name, var) => {
                let var_name = match var {
                    crate::parser::EnumVariant::Unit(n) => n,
                    crate::parser::EnumVariant::Tuple(n, _) => n,
                    crate::parser::EnumVariant::Struct(n, _) => n,
                };
                write!(f, "<enum constructor: {}.{}>", enum_name, var_name)
            }
            Value::RefPath(path, mutable) => match path {
                RefPath::Var(name, _) => {
                    if *mutable {
                        write!(f, "&mut {name}")
                    } else {
                        write!(f, "&{name}")
                    }
                }
                RefPath::Field { owner, member, .. } => {
                    if *mutable {
                        write!(f, "&mut {owner}.{member}")
                    } else {
                        write!(f, "&{owner}.{member}")
                    }
                }
                RefPath::Index { owner, index, .. } => {
                    if *mutable {
                        write!(f, "&mut {owner}[{index}]")
                    } else {
                        write!(f, "&{owner}[{index}]")
                    }
                }
            },
        }
    }
}

impl Value {
    pub fn is_equal(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Byte(a), Value::Byte(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => {
                if a.len() != b.len() { return false; }
                a.iter().zip(b.iter()).all(|(x, y)| x.is_equal(y))
            },
            (Value::Formula(a), Value::Formula(b)) => {
                if a.len() != b.len() { return false; }
                a.iter().all(|(k, v)| b.get(k).map_or(false, |bv| v.is_equal(bv)))
            },
            (Value::Unit(a), Value::Unit(b)) => a == b,
            (Value::Quantity(av, au), Value::Quantity(bv, bu)) => {
                (av == bv || (av - bv).abs() < 0.001) && au == bu
            },
            (a, b) => a.to_string() == b.to_string(),
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Nil => false,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "Nil",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::String(_) => "String",
            Value::Tuple(_) => "Tuple",
            Value::Byte(_) => "Byte",
            Value::Bytes(_) => "Byte",
            Value::Object(_) => "Object",
            Value::Formula(_) => "Formula",
            Value::Unit(_) => "Unit",
            Value::Quantity(_, _) => "Quantity",
            Value::Ref(_) | Value::RefPath(_, _) => "Ref",
            Value::StructInstance { .. } => "StructInstance", // Will map to actual name downstream if needed
            _ => "Unknown",
        }
    }



    pub fn pack(&self) -> CValue {
        match self {
            Value::Int(i) => CValue {
                tag: CValueTag::Int,
                int_val: *i,
                int_val2: 0,
                float_val: 0.0,
                bool_val: false,
                string_ptr: std::ptr::null_mut(),
                obj_ptr: std::ptr::null_mut(),
            },
            Value::Float(f) => CValue {
                tag: CValueTag::Float,
                int_val: 0,
                int_val2: 0,
                float_val: *f,
                bool_val: false,
                string_ptr: std::ptr::null_mut(),
                obj_ptr: std::ptr::null_mut(),
            },
            Value::Bool(b) => CValue {
                tag: CValueTag::Bool,
                int_val: 0,
                int_val2: 0,
                float_val: 0.0,
                bool_val: *b,
                string_ptr: std::ptr::null_mut(),
                obj_ptr: std::ptr::null_mut(),
            },
            Value::Range(start, end) => CValue {
                tag: CValueTag::Range,
                int_val: *start,
                int_val2: *end,
                float_val: 0.0,
                bool_val: false,
                string_ptr: std::ptr::null_mut(),
                obj_ptr: std::ptr::null_mut(),
            },
            Value::String(s) => {
                let c_str = std::ffi::CString::new(s.clone()).unwrap_or_default();
                CValue {
                    tag: CValueTag::String,
                    int_val: 0,
                    int_val2: 0,
                    float_val: 0.0,
                    bool_val: false,
                    string_ptr: c_str.into_raw(),
                    obj_ptr: std::ptr::null_mut(),
                }
            }
            Value::NativeObject { ptr, .. } => CValue {
                tag: CValueTag::NativeObject,
                int_val: 0,
                int_val2: 0,
                float_val: 0.0,
                bool_val: false,
                string_ptr: std::ptr::null_mut(),
                obj_ptr: *ptr as *mut std::ffi::c_void,
            },
            Value::Return(inner) | Value::Ref(inner) => inner.pack(),
            Value::Function { .. } | Value::NativeCallback(_) | Value::NativeClosure(_) => {
                let fn_id = register_callback_value(self.clone());
                CValue {
                    tag: CValueTag::Function,
                    int_val: fn_id as i64,
                    int_val2: 0,
                    float_val: 0.0,
                    bool_val: false,
                    string_ptr: std::ptr::null_mut(),
                    obj_ptr: std::ptr::null_mut(),
                }
            }
            Value::EnumValue(enum_name, variant_name, data) => {
                let name = format!("{}::{}", enum_name, variant_name);
                let c_str = std::ffi::CString::new(name).unwrap_or_default();
                let inner_ptr = match data {
                    EnumData::Tuple(vals) if !vals.is_empty() => {
                        let inner_val = vals[0].clone();
                        Box::into_raw(Box::new(inner_val.pack()))
                    }
                    _ => std::ptr::null_mut(),
                };
                CValue {
                    tag: CValueTag::EnumVariant,
                    int_val: 0,
                    int_val2: 0,
                    float_val: 0.0,
                    bool_val: false,
                    string_ptr: c_str.into_raw(),
                    obj_ptr: inner_ptr as *mut std::ffi::c_void,
                }
            }
            Value::Tuple(arr) => {
                let mut cvals: Vec<CValue> = arr.iter().map(|v| v.pack()).collect();
                cvals.shrink_to_fit();
                let len = cvals.len();
                let ptr = cvals.as_mut_ptr();
                std::mem::forget(cvals);
                CValue {
                    tag: CValueTag::Array,
                    int_val: len as i64,
                    int_val2: 0,
                    float_val: 0.0,
                    bool_val: false,
                    string_ptr: std::ptr::null_mut(),
                    obj_ptr: ptr as *mut std::ffi::c_void,
                }
            }
            Value::Formula(_) | Value::Object(_) | Value::StructInstance { .. } => {
                let json_val = crate::native_std::json::value_to_json(self);
                let json_str = json_val.to_string();
                let c_str = std::ffi::CString::new(json_str).unwrap_or_default();
                CValue {
                    tag: CValueTag::String,
                    int_val: 0,
                    int_val2: 0,
                    float_val: 0.0,
                    bool_val: false,
                    string_ptr: c_str.into_raw(),
                    obj_ptr: std::ptr::null_mut(),
                }
            }
            _ => CValue::null(),
        }
    }

    pub fn unpack(cval: CValue, crate_name: &str, type_name: &str) -> Self {
        match cval.tag {
            CValueTag::Null => Value::Nil,
            CValueTag::Int => Value::Int(cval.int_val),
            CValueTag::Float => Value::Float(cval.float_val),
            CValueTag::Bool => Value::Bool(cval.bool_val),
            CValueTag::Range => Value::Range(cval.int_val, cval.int_val2),
            CValueTag::String => {
                if cval.string_ptr.is_null() {
                    Value::String(String::new())
                } else {
                    let s = unsafe {
                        std::ffi::CStr::from_ptr(cval.string_ptr)
                            .to_string_lossy()
                            .into_owned()
                    };
                    unsafe {
                        let _ = std::ffi::CString::from_raw(cval.string_ptr);
                    }
                    Value::String(s)
                }
            }
            CValueTag::EnumVariant => {
                let variant_name = if cval.string_ptr.is_null() {
                    String::new()
                } else {
                    unsafe {
                        std::ffi::CStr::from_ptr(cval.string_ptr)
                            .to_string_lossy()
                            .into_owned()
                    }
                };
                
                let mut parts = variant_name.splitn(2, "::");
                let enum_n = parts.next().unwrap_or("").to_string();
                let variant_n = parts.next().unwrap_or(&enum_n).to_string();

                let data = if cval.obj_ptr.is_null() {
                    EnumData::Unit
                } else {
                    let inner_cval = unsafe { Box::from_raw(cval.obj_ptr as *mut CValue) };
                    EnumData::Tuple(vec![Value::unpack(*inner_cval, crate_name, type_name)])
                };
                Value::EnumValue(enum_n, variant_n, data)
            }
            CValueTag::Array => {
                if cval.obj_ptr.is_null() || cval.int_val == 0 {
                    Value::Tuple(Vec::new())
                } else {
                    let cvals = unsafe {
                        Vec::from_raw_parts(
                            cval.obj_ptr as *mut CValue,
                            cval.int_val as usize,
                            cval.int_val as usize,
                        )
                    };
                    let mut vals = Vec::with_capacity(cvals.len());
                    for cv in cvals {
                        vals.push(Value::unpack(cv, crate_name, type_name));
                    }
                    Value::Tuple(vals)
                }
            }
            CValueTag::NativeObject => Value::NativeObject {
                crate_name: crate_name.to_string(),
                type_name: type_name.to_string(),
                ptr: cval.obj_ptr as usize,
            },
            CValueTag::Function => {
                let id = cval.int_val as u64;
                get_callback_value(id).unwrap_or(Value::Nil)
            }
        }
    }

    pub fn as_int(&self) -> Result<i64, String> {
        match self {
            Value::Int(i) => Ok(*i),
            _ => Err("expected int".into()),
        }
    }

    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err("expected bool".into()),
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, String> {
        match self {
            Value::Bytes(bytes) => Ok(bytes.clone()),

            Value::Tuple(values) => {
                let mut out = Vec::new();

                for value in values {
                    match value {
                        Value::Int(i) if *i >= 0 && *i <= 255 => {
                            out.push(*i as u8);
                        }
                        _ => {
                            return Err("expected tuple of integers between 0 and 255".into());
                        }
                    }
                }

                Ok(out)
            }

            _ => Err("expected bytes".into()),
        }
    }
}
pub static THREADS: OnceLock<Mutex<HashMap<u64, JoinHandle<Value>>>> = OnceLock::new();
pub static THREAD_COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();
pub static CHANNELS: OnceLock<Mutex<HashMap<u64, std::sync::mpsc::Sender<Value>>>> =
    OnceLock::new();
pub static RECEIVERS: OnceLock<Mutex<HashMap<u64, Arc<Mutex<std::sync::mpsc::Receiver<Value>>>>>> =
    OnceLock::new();
pub static CHANNEL_COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();
pub static CHILD_PROCESSES: OnceLock<Mutex<HashMap<u64, Child>>> = OnceLock::new();
pub static CHILD_PROCESS_COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();

pub fn get_threads() -> &'static Mutex<HashMap<u64, JoinHandle<Value>>> {
    THREADS.get_or_init(|| Mutex::new(HashMap::new()))
}
pub fn get_thread_counter() -> &'static Mutex<u64> {
    THREAD_COUNTER.get_or_init(|| Mutex::new(0))
}
pub fn get_channels() -> &'static Mutex<HashMap<u64, std::sync::mpsc::Sender<Value>>> {
    CHANNELS.get_or_init(|| Mutex::new(HashMap::new()))
}
pub fn get_receivers() -> &'static Mutex<HashMap<u64, Arc<Mutex<std::sync::mpsc::Receiver<Value>>>>>
{
    RECEIVERS.get_or_init(|| Mutex::new(HashMap::new()))
}
pub fn get_channel_counter() -> &'static Mutex<u64> {
    CHANNEL_COUNTER.get_or_init(|| Mutex::new(0))
}
pub fn get_child_processes() -> &'static Mutex<HashMap<u64, Child>> {
    CHILD_PROCESSES.get_or_init(|| Mutex::new(HashMap::new()))
}
pub fn get_child_process_counter() -> &'static Mutex<u64> {
    CHILD_PROCESS_COUNTER.get_or_init(|| Mutex::new(0))
}

pub fn wait_for_all_threads() {
    loop {
        let handle = {
            let mut registry = get_threads().lock().unwrap();

            if registry.is_empty() {
                break;
            }

            let id = *registry.keys().next().unwrap();

            registry.remove(&id)
        };

        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

#[derive(Debug, Clone)]
pub struct VarEntry {
    pub value: Value,
    pub is_mut: bool,
}

#[derive(Debug, Clone)]
pub struct Env {
    pub variables: HashMap<String, VarEntry>,
    pub parent: Option<Arc<Mutex<Env>>>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub fn fork(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            parent: None,
        }
    }

    pub fn snapshot(&self) -> Self {
        let mut vars = HashMap::new();
        if let Some(parent) = &self.parent {
            let parent_snapshot = parent.lock().unwrap().snapshot();
            for (k, entry) in parent_snapshot.variables {
                vars.insert(k, entry);
            }
        }
        for (k, entry) in &self.variables {
            vars.insert(k.clone(), entry.clone());
        }
        Self {
            variables: vars,
            parent: None,
        }
    }

    pub fn new_child(parent: Arc<Mutex<Env>>) -> Self {
        Self {
            variables: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(entry) = self.variables.get(name) {
            Some(entry.value.clone())
        } else if let Some(parent) = &self.parent {
            parent.lock().unwrap().get(name)
        } else {
            None
        }
    }

    pub fn define(&mut self, name: String, val: Value, is_mut: bool) {
        self.variables.insert(name, VarEntry { value: val, is_mut });
    }

    pub fn assign(&mut self, name: String, val: Value) -> Result<(), String> {
        if let Some(entry) = self.variables.get_mut(&name) {
            if !entry.is_mut {
                return Err(format!(
                    "cannot mutate immutable variable '{}'. Declare with 'let mut' to allow reassignment.",
                    name
                ));
            }
            entry.value = val;
            Ok(())
        } else if let Some(parent) = &self.parent {
            parent.lock().unwrap().assign(name, val)
        } else {
            Err(format!(
                "cannot assign to undeclared variable '{}'. Declare it first using 'let mut'.",
                name
            ))
        }
    }

    pub fn move_var(&mut self, name: &str) {
        if let Some(entry) = self.variables.get_mut(name) {
            entry.value = Value::Moved(name.to_string());
        } else if let Some(parent) = &self.parent {
            parent.lock().unwrap().move_var(name);
        }
    }

    pub fn to_formula_map(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        for (k, entry) in &self.variables {
            map.insert(k.clone(), entry.value.clone());
        }
        map
    }
}
