use crate::vm::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static LOCAL_STORAGE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
static CURRENT_ROUTE: OnceLock<Mutex<String>> = OnceLock::new();

fn get_storage() -> &'static Mutex<HashMap<String, String>> {
    LOCAL_STORAGE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_route() -> &'static Mutex<String> {
    CURRENT_ROUTE.get_or_init(|| Mutex::new("/".to_string()))
}

fn web_main(_args: Vec<Value>) -> Result<Value, String> {
    println!("\x1b[1;36m    Target\x1b[0m Web client environment initialized (std.web)");
    println!(
        "\x1b[1;32m      Info\x1b[0m To compile for the browser: \x1b[1mflame build --target web\x1b[0m"
    );
    Ok(Value::Nil)
}

fn web_mount(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Err("web.mount expects at least 1 argument (selector, [page])".to_string());
    }
    let sel = args[0].to_string();
    println!("\x1b[1;36m   Mounted\x1b[0m page to selector '{}'", sel);
    Ok(Value::Nil)
}

fn web_navigate(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Err("web.navigate expects 1 argument (path)".to_string());
    }
    let path = args[0].to_string().trim_matches('"').to_string();
    if let Ok(mut r) = get_route().lock() {
        *r = path.clone();
    }
    println!("\x1b[1;36m Navigated\x1b[0m to route '{}'", path);
    Ok(Value::Nil)
}

fn json_response_cb(_args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Object(HashMap::new()))
}

fn text_response_cb(_args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::String("OK".to_string()))
}

fn signal_get_cb(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        Ok(args[0].clone())
    } else {
        Ok(Value::Nil)
    }
}

fn signal_set_cb(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        Ok(args[0].clone())
    } else {
        Ok(Value::Nil)
    }
}

fn timer_return_id_cb(_args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Int(1))
}

fn timer_return_nil_cb(_args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Nil)
}

fn dom_element_cb(_args: Vec<Value>) -> Result<Value, String> {
    let mut el = HashMap::new();
    el.insert("id".to_string(), Value::String(String::new()));
    el.insert("tagName".to_string(), Value::String(String::new()));
    el.insert("className".to_string(), Value::String(String::new()));
    el.insert("innerText".to_string(), Value::String(String::new()));
    el.insert("innerHTML".to_string(), Value::String(String::new()));
    el.insert("value".to_string(), Value::String(String::new()));
    Ok(Value::Object(el))
}

fn dom_list_cb(_args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Tuple(Vec::new()))
}

fn web_fetch(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Err("web.fetch expects at least 1 argument (url)".to_string());
    }
    let url = args[0].to_string().trim_matches('"').to_string();

    let mut res_map = HashMap::new();
    res_map.insert("url".to_string(), Value::String(url.clone()));
    res_map.insert("status".to_string(), Value::Int(200));
    res_map.insert("statusText".to_string(), Value::String("OK".to_string()));
    res_map.insert("ok".to_string(), Value::Bool(true));
    res_map.insert("json".to_string(), Value::NativeCallback(json_response_cb));
    res_map.insert("text".to_string(), Value::NativeCallback(text_response_cb));

    Ok(Value::Object(res_map))
}

fn web_signal(args: Vec<Value>) -> Result<Value, String> {
    let initial = if !args.is_empty() {
        args[0].clone()
    } else {
        Value::Nil
    };

    let mut sig_obj = HashMap::new();
    sig_obj.insert("value".to_string(), initial);
    sig_obj.insert("get".to_string(), Value::NativeCallback(signal_get_cb));
    sig_obj.insert("set".to_string(), Value::NativeCallback(signal_set_cb));
    Ok(Value::Object(sig_obj))
}

fn web_computed(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Err("web.computed expects 1 function argument".to_string());
    }
    Ok(args[0].clone())
}

fn web_effect(_args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Nil)
}

fn web_batch(_args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Nil)
}

fn web_tag(args: Vec<Value>) -> Result<Value, String> {
    let tag_name = if !args.is_empty() {
        args[0].to_string().trim_matches('"').to_string()
    } else {
        "div".to_string()
    };
    let props = if args.len() > 1 {
        args[1].clone()
    } else {
        Value::Object(HashMap::new())
    };
    let children = if args.len() > 2 {
        args[2].clone()
    } else {
        Value::Tuple(vec![])
    };

    let mut node = HashMap::new();
    node.insert("tag".to_string(), Value::String(tag_name));
    node.insert("attributes".to_string(), props);
    node.insert("children".to_string(), children);
    node.insert(
        "__type__".to_string(),
        Value::String("HtmlNode".to_string()),
    );

    Ok(Value::Object(node))
}

fn web_text(args: Vec<Value>) -> Result<Value, String> {
    let content = if !args.is_empty() {
        args[0].to_string().trim_matches('"').to_string()
    } else {
        "".to_string()
    };
    let mut node = HashMap::new();
    node.insert("tag".to_string(), Value::String("#text".to_string()));
    node.insert("content".to_string(), Value::String(content));
    node.insert(
        "__type__".to_string(),
        Value::String("HtmlNode".to_string()),
    );
    Ok(Value::Object(node))
}

fn web_storage_get(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::Nil);
    }
    let k = args[0].to_string().trim_matches('"').to_string();
    if let Ok(s) = get_storage().lock() {
        if let Some(v) = s.get(&k) {
            return Ok(Value::String(v.clone()));
        }
    }
    Ok(Value::Nil)
}

fn web_storage_set(args: Vec<Value>) -> Result<Value, String> {
    if args.len() >= 2 {
        let k = args[0].to_string().trim_matches('"').to_string();
        let v = args[1].to_string().trim_matches('"').to_string();
        if let Ok(mut s) = get_storage().lock() {
            s.insert(k, v);
        }
    }
    Ok(Value::Nil)
}

fn web_storage_remove(args: Vec<Value>) -> Result<Value, String> {
    if !args.is_empty() {
        let k = args[0].to_string().trim_matches('"').to_string();
        if let Ok(mut s) = get_storage().lock() {
            s.remove(&k);
        }
    }
    Ok(Value::Nil)
}

fn web_storage_clear(_args: Vec<Value>) -> Result<Value, String> {
    if let Ok(mut s) = get_storage().lock() {
        s.clear();
    }
    Ok(Value::Nil)
}

fn console_log(args: Vec<Value>) -> Result<Value, String> {
    let msg = args
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    println!("\x1b[1;34m[console.log]\x1b[0m {}", msg);
    Ok(Value::Nil)
}

fn console_warn(args: Vec<Value>) -> Result<Value, String> {
    let msg = args
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    println!("\x1b[1;33m[console.warn]\x1b[0m {}", msg);
    Ok(Value::Nil)
}

fn console_error(args: Vec<Value>) -> Result<Value, String> {
    let msg = args
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    eprintln!("\x1b[1;31m[console.error]\x1b[0m {}", msg);
    Ok(Value::Nil)
}

fn console_info(args: Vec<Value>) -> Result<Value, String> {
    let msg = args
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    println!("\x1b[1;36m[console.info]\x1b[0m {}", msg);
    Ok(Value::Nil)
}

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    m.insert("web".to_string(), Value::NativeCallback(web_main));
    m.insert("mount".to_string(), Value::NativeCallback(web_mount));
    m.insert("navigate".to_string(), Value::NativeCallback(web_navigate));
    m.insert("fetch".to_string(), Value::NativeCallback(web_fetch));
    m.insert("signal".to_string(), Value::NativeCallback(web_signal));
    m.insert("computed".to_string(), Value::NativeCallback(web_computed));
    m.insert("effect".to_string(), Value::NativeCallback(web_effect));
    m.insert("batch".to_string(), Value::NativeCallback(web_batch));
    m.insert("tag".to_string(), Value::NativeCallback(web_tag));
    m.insert("h".to_string(), Value::NativeCallback(web_tag));
    m.insert("text".to_string(), Value::NativeCallback(web_text));

    m.insert(
        "setInterval".to_string(),
        Value::NativeCallback(timer_return_id_cb),
    );
    m.insert(
        "clearInterval".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    m.insert(
        "setTimeout".to_string(),
        Value::NativeCallback(timer_return_id_cb),
    );
    m.insert(
        "clearTimeout".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    m.insert(
        "requestAnimationFrame".to_string(),
        Value::NativeCallback(timer_return_id_cb),
    );
    m.insert(
        "cancelAnimationFrame".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );

    m.insert(
        "storage_get".to_string(),
        Value::NativeCallback(web_storage_get),
    );
    m.insert(
        "storage_set".to_string(),
        Value::NativeCallback(web_storage_set),
    );
    m.insert(
        "storage_remove".to_string(),
        Value::NativeCallback(web_storage_remove),
    );
    m.insert(
        "storage_clear".to_string(),
        Value::NativeCallback(web_storage_clear),
    );

    let mut ls_obj = HashMap::new();
    ls_obj.insert("get".to_string(), Value::NativeCallback(web_storage_get));
    ls_obj.insert("set".to_string(), Value::NativeCallback(web_storage_set));
    ls_obj.insert(
        "remove".to_string(),
        Value::NativeCallback(web_storage_remove),
    );
    ls_obj.insert(
        "clear".to_string(),
        Value::NativeCallback(web_storage_clear),
    );
    m.insert("localStorage".to_string(), Value::Object(ls_obj.clone()));
    m.insert("sessionStorage".to_string(), Value::Object(ls_obj.clone()));

    let mut console_obj = HashMap::new();
    console_obj.insert("log".to_string(), Value::NativeCallback(console_log));
    console_obj.insert("warn".to_string(), Value::NativeCallback(console_warn));
    console_obj.insert("error".to_string(), Value::NativeCallback(console_error));
    console_obj.insert("info".to_string(), Value::NativeCallback(console_info));
    m.insert("console".to_string(), Value::Object(console_obj));

    let mut doc_obj = HashMap::new();
    doc_obj.insert(
        "getElementById".to_string(),
        Value::NativeCallback(dom_element_cb),
    );
    doc_obj.insert(
        "createElement".to_string(),
        Value::NativeCallback(dom_element_cb),
    );
    doc_obj.insert(
        "createTextNode".to_string(),
        Value::NativeCallback(dom_element_cb),
    );
    doc_obj.insert(
        "querySelector".to_string(),
        Value::NativeCallback(dom_element_cb),
    );
    doc_obj.insert(
        "querySelectorAll".to_string(),
        Value::NativeCallback(dom_list_cb),
    );
    doc_obj.insert(
        "addEventListener".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    doc_obj.insert(
        "removeEventListener".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    doc_obj.insert("title".to_string(), Value::String(String::new()));
    doc_obj.insert("body".to_string(), Value::Object(HashMap::new()));
    doc_obj.insert("head".to_string(), Value::Object(HashMap::new()));
    m.insert("document".to_string(), Value::Object(doc_obj));

    let mut win_obj = HashMap::new();
    win_obj.insert(
        "addEventListener".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    win_obj.insert(
        "removeEventListener".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    win_obj.insert("location".to_string(), Value::Object(HashMap::new()));
    win_obj.insert("history".to_string(), Value::Object(HashMap::new()));
    win_obj.insert("localStorage".to_string(), Value::Object(ls_obj.clone()));
    win_obj.insert("sessionStorage".to_string(), Value::Object(ls_obj));
    win_obj.insert("innerWidth".to_string(), Value::Int(1920));
    win_obj.insert("innerHeight".to_string(), Value::Int(1080));
    win_obj.insert(
        "alert".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    win_obj.insert(
        "prompt".to_string(),
        Value::NativeCallback(text_response_cb),
    );
    win_obj.insert(
        "confirm".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    win_obj.insert(
        "setTimeout".to_string(),
        Value::NativeCallback(timer_return_id_cb),
    );
    win_obj.insert(
        "clearTimeout".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    win_obj.insert(
        "setInterval".to_string(),
        Value::NativeCallback(timer_return_id_cb),
    );
    win_obj.insert(
        "clearInterval".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    win_obj.insert(
        "requestAnimationFrame".to_string(),
        Value::NativeCallback(timer_return_id_cb),
    );
    win_obj.insert(
        "cancelAnimationFrame".to_string(),
        Value::NativeCallback(timer_return_nil_cb),
    );
    m.insert("window".to_string(), Value::Object(win_obj));

    m
}
