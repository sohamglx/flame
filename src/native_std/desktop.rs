use crate::vm::Value;
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use std::collections::HashMap;
use std::process::Command;

fn parse_key(name: &str) -> Result<Key, String> {
    let name = name.trim_matches('"').to_lowercase();

    Ok(match name.as_str() {
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" => Key::Alt,
        "cmd" | "command" => Key::Meta,
        "win" | "super" => Key::Meta,
        "option" => Key::Alt,

        "enter" => Key::Return,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "escape" | "esc" => Key::Escape,

        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,

        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,

        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,

        _ => {
            if name.len() == 1 {
                Key::Unicode(name.chars().next().unwrap())
            } else {
                return Err(format!("Unknown key '{}'", name));
            }
        }
    })
}

fn open_target(target: &str, extra_args: &[String]) -> bool {
    // If extra args were explicitly passed: run target directly with those args
    if !extra_args.is_empty() {
        if Command::new(target)
            .args(extra_args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            return true;
        }
        #[cfg(target_os = "windows")]
        {
            if Command::new("cmd")
                .args(["/C", target])
                .args(extra_args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
            {
                return true;
            }
        }
    }

    // Special handling for browser internal URL schemes like brave://, chrome://, edge://
    let lower_target = target.to_lowercase();
    if lower_target.starts_with("brave://") {
        for candidate in ["brave", "brave-browser", "brave-bin"] {
            if Command::new(candidate)
                .arg(target)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
            {
                return true;
            }
        }
    } else if lower_target.starts_with("chrome://") || lower_target.starts_with("chromium://") {
        for candidate in ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "chrome"] {
            if Command::new(candidate)
                .arg(target)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
            {
                return true;
            }
        }
    } else if lower_target.starts_with("edge://") {
        for candidate in ["msedge", "microsoft-edge", "microsoft-edge-stable"] {
            if Command::new(candidate)
                .arg(target)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
            {
                return true;
            }
        }
    }

    // Standard cross-platform opener for URLs (http, https, file, etc.) and file paths
    #[cfg(target_os = "linux")]
    {
        if Command::new("xdg-open")
            .arg(target)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            return true;
        }
        if Command::new("gio")
            .args(["open", target])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            return true;
        }
    }

    #[cfg(target_os = "macos")]
    {
        if Command::new("open")
            .arg(target)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            return true;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if Command::new("cmd")
            .args(["/C", "start", "", target])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .is_ok()
        {
            return true;
        }
    }

    // Finally, if target was an application name (e.g. "code", "firefox", "gedit")
    if Command::new(target)
        .args(extra_args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
    {
        return true;
    }

    false
}

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    let mut mouse = HashMap::new();
    let mut keyboard = HashMap::new();

    m.insert(
        "open".into(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("desktop.open expects at least 1 argument (target, [args])".to_string());
            }
            let target = match &args[0] {
                Value::String(s) => s.trim_matches('"').to_string(),
                v => v.to_string().trim_matches('"').to_string(),
            };
            let extra_args: Vec<String> = if args.len() > 1 {
                match &args[1] {
                    Value::Tuple(arr) => arr
                        .iter()
                        .map(|v| match v {
                            Value::String(s) => s.trim_matches('"').to_string(),
                            other => other.to_string().trim_matches('"').to_string(),
                        })
                        .collect(),
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };

            let success = open_target(&target, &extra_args);
            Ok(Value::Bool(success))
        }),
    );

    // ---------------- Mouse ----------------

    mouse.insert(
        "move".into(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("mouseMove expects 2 arguments (x, y)".to_string());
            }
            let x = if let Value::Int(i) = args[0] {
                i as i32
            } else {
                0
            };
            let y = if let Value::Int(i) = args[1] {
                i as i32
            } else {
                0
            };
            let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
            let _ = enigo.move_mouse(x, y, Coordinate::Abs);
            Ok(Value::Nil)
        }),
    );

    mouse.insert(
        "click".to_string(),
        Value::NativeCallback(|args| {
            let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
            let button = if args.is_empty() {
                Button::Left
            } else if let Value::String(s) = &args[0] {
                match s.as_str() {
                    "right" => Button::Right,
                    "middle" => Button::Middle,
                    _ => Button::Left,
                }
            } else {
                Button::Left
            };
            let _ = enigo.button(button, Direction::Click);
            Ok(Value::Nil)
        }),
    );

    // ---------------- Keyboard ----------------

    keyboard.insert(
        "write".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("keyboardType expects 1 argument (text)".to_string());
            }
            let text = args[0].to_string().trim_matches('"').to_string();
            let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
            let _ = enigo.text(&text);
            Ok(Value::Nil)
        }),
    );

    keyboard.insert(
        "hotkey".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 2 {
                return Err("keyboard.hotkey expects at least 2 keys".to_string());
            }

            let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;

            let keys: Result<Vec<_>, _> = args.iter().map(|v| parse_key(&v.to_string())).collect();

            let keys = keys?;

            for key in &keys[..keys.len() - 1] {
                enigo
                    .key(*key, Direction::Press)
                    .map_err(|e| e.to_string())?;
            }

            enigo
                .key(keys[keys.len() - 1], Direction::Click)
                .map_err(|e| e.to_string())?;

            for key in keys[..keys.len() - 1].iter().rev() {
                enigo
                    .key(*key, Direction::Release)
                    .map_err(|e| e.to_string())?;
            }
            Ok(Value::Nil)
        }),
    );

    keyboard.insert(
        "key".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err(
                    "automation.key expects at least 1 argument (key, [action])".to_string()
                );
            }

            let key = parse_key(&args[0].to_string())?;

            let direction = if args.len() >= 2 {
                match args[1].to_string().trim_matches('"') {
                    "press" => Direction::Press,
                    "release" => Direction::Release,
                    _ => Direction::Click,
                }
            } else {
                Direction::Click
            };

            let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
            enigo.key(key, direction).map_err(|e| e.to_string())?;

            Ok(Value::Nil)
        }),
    );

    m.insert("mouse".into(), Value::Object(mouse));

    m.insert("keyboard".into(), Value::Object(keyboard));
    m
}
