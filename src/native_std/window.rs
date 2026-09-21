use crate::vm::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WindowData {
    pub id: String,
    pub title: String,
    pub app_name: String,
    pub pid: i64,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

impl WindowData {
    pub fn to_value(&self) -> Value {
        let mut map = HashMap::new();
        map.insert("id".to_string(), Value::String(self.id.clone()));
        map.insert("title".to_string(), Value::String(self.title.clone()));
        map.insert("appName".to_string(), Value::String(self.app_name.clone()));
        map.insert("pid".to_string(), Value::Int(self.pid));
        map.insert("x".to_string(), Value::Int(self.x));
        map.insert("y".to_string(), Value::Int(self.y));
        map.insert("width".to_string(), Value::Int(self.width));
        map.insert("height".to_string(), Value::Int(self.height));
        Value::Formula(map)
    }
}

fn extract_target(arg: &Value) -> String {
    match arg {
        Value::String(s) => s.trim_matches('"').to_string(),
        Value::Int(i) => i.to_string(),
        Value::Formula(map) | Value::Object(map) => {
            if let Some(Value::String(id)) = map.get("id") {
                id.clone()
            } else if let Some(Value::Int(id)) = map.get("id") {
                id.to_string()
            } else if let Some(Value::String(title)) = map.get("title") {
                title.clone()
            } else {
                arg.to_string().trim_matches('"').to_string()
            }
        }
        _ => arg.to_string().trim_matches('"').to_string(),
    }
}

// =========================================================================
// Linux Platform Implementation
// =========================================================================
#[cfg(target_os = "linux")]
mod platform {
    use super::WindowData;
    use std::time::Duration;
    use x11rb::connection::Connection as _;

    fn get_atspi_bus_address() -> Option<String> {
        if let Ok(addr) = std::env::var("AT_SPI_BUS_ADDRESS") {
            if !addr.trim().is_empty() {
                return Some(addr.trim().to_string());
            }
        }

        // Try querying session bus org.a11y.Bus
        if let Ok(sess) = dbus::blocking::Connection::new_session() {
            let proxy =
                sess.with_proxy("org.a11y.Bus", "/org/a11y/bus", Duration::from_millis(500));
            let res: Result<(String,), _> = proxy.method_call("org.a11y.Bus", "GetAddress", ());
            if let Ok((addr,)) = res {
                if !addr.trim().is_empty() {
                    return Some(addr.trim().to_string());
                }
            }
        }

        // Fallback to standard runtime user directory
        let uid = unsafe { libc::getuid() };
        let default_path = format!("/run/user/{}/at-spi/bus", uid);
        if std::path::Path::new(&default_path).exists() {
            return Some(format!("unix:path={}", default_path));
        }

        None
    }

    fn list_windows_atspi() -> Vec<WindowData> {
        let addr = match get_atspi_bus_address() {
            Some(a) => a,
            None => return Vec::new(),
        };

        let conn = match dbus::blocking::Connection::new_address(&addr) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let root_proxy = conn.with_proxy(
            "org.a11y.atspi.Registry",
            "/org/a11y/atspi/accessible/root",
            Duration::from_millis(600),
        );

        let app_refs: Result<(Vec<(String, dbus::Path<'static>)>,), _> =
            root_proxy.method_call("org.a11y.atspi.Accessible", "GetChildren", ());
        let app_refs = match app_refs {
            Ok(r) => r.0,
            Err(_) => return Vec::new(),
        };

        let mut windows = Vec::new();
        let dbus_proxy = conn.with_proxy(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            Duration::from_millis(400),
        );

        for (bus_name, app_path) in app_refs {
            if bus_name.trim().is_empty() || dbus::strings::BusName::new(&bus_name).is_err() {
                continue;
            }

            let app_proxy = conn.with_proxy(&bus_name, &app_path, Duration::from_millis(500));

            let pid_res: Result<(u32,), _> = dbus_proxy.method_call(
                "org.freedesktop.DBus",
                "GetConnectionUnixProcessID",
                (&bus_name,),
            );
            let pid = pid_res.map(|(p,)| p as i64).unwrap_or(0);

            let app_name_res: Result<(dbus::arg::Variant<String>,), _> = app_proxy.method_call(
                "org.freedesktop.DBus.Properties",
                "Get",
                ("org.a11y.atspi.Accessible", "Name"),
            );
            let app_name: String = app_name_res.map(|(v,)| v.0).unwrap_or_else(|_| {
                if pid > 0 {
                    std::fs::read_to_string(format!("/proc/{}/comm", pid))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            });

            // Ignore background system daemons without user windows
            if app_name == "update-notifier"
                || app_name == "evolution-alarm-notify"
                || app_name == "ibus-extension-gtk3"
                || app_name == "xdg-desktop-portal-gtk"
            {
                continue;
            }

            let win_refs: Result<(Vec<(String, dbus::Path<'static>)>,), _> =
                app_proxy.method_call("org.a11y.atspi.Accessible", "GetChildren", ());

            if let Ok((wins,)) = win_refs {
                for (win_bus, win_path) in wins {
                    let actual_bus = if win_bus.trim().is_empty() {
                        bus_name.as_str()
                    } else {
                        win_bus.as_str()
                    };

                    if actual_bus.is_empty() || dbus::strings::BusName::new(actual_bus).is_err() {
                        continue;
                    }

                    let win_proxy =
                        conn.with_proxy(actual_bus, &win_path, Duration::from_millis(500));

                    let title_res: Result<(dbus::arg::Variant<String>,), _> = win_proxy
                        .method_call(
                            "org.freedesktop.DBus.Properties",
                            "Get",
                            ("org.a11y.atspi.Accessible", "Name"),
                        );
                    let title: String = title_res.map(|(v,)| v.0).unwrap_or_default();

                    let role_res: Result<(String,), _> =
                        win_proxy.method_call("org.a11y.atspi.Accessible", "GetRoleName", ());
                    let role: String = role_res.map(|(r,)| r).unwrap_or_default();

                    // Filter for application windows, frames, dialogs
                    if role == "frame" || role == "window" || role == "dialog" || role.is_empty() {
                        if title.is_empty() && app_name == "gnome-shell" {
                            continue;
                        }
                        if title == "Main stage" || title.starts_with("Desktop Icons") {
                            continue;
                        }

                        // Retrieve extents: (x, y, width, height)
                        let extents_res: Result<((i32, i32, i32, i32),), _> = win_proxy
                            .method_call("org.a11y.atspi.Component", "GetExtents", (0u32,));
                        let (x, y, width, height) = extents_res
                            .map(|((x, y, w, h),)| (x as i64, y as i64, w as i64, h as i64))
                            .unwrap_or((0, 0, 0, 0));

                        let id = format!("{}|{}", actual_bus, win_path);
                        windows.push(WindowData {
                            id,
                            title,
                            app_name: app_name.clone(),
                            pid,
                            x,
                            y,
                            width,
                            height,
                        });
                    }
                }
            }
        }

        windows
    }

    fn list_windows_x11() -> Vec<WindowData> {
        let mut windows = Vec::new();
        let (conn, screen_num) = match x11rb::connect(None) {
            Ok(c) => c,
            Err(_) => return windows,
        };

        use x11rb::protocol::xproto::*;
        let setup = conn.setup();
        let screen = match setup.roots.get(screen_num) {
            Some(s) => s,
            None => return windows,
        };

        let net_client_list = match conn.intern_atom(false, b"_NET_CLIENT_LIST") {
            Ok(cookie) => match cookie.reply() {
                Ok(reply) => reply.atom,
                Err(_) => return windows,
            },
            Err(_) => return windows,
        };

        let net_wm_name = conn
            .intern_atom(false, b"_NET_WM_NAME")
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.atom);
        let net_wm_pid = conn
            .intern_atom(false, b"_NET_WM_PID")
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.atom);
        let utf8_string = conn
            .intern_atom(false, b"UTF8_STRING")
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|r| r.atom);

        if let Ok(prop_cookie) = conn.get_property(
            false,
            screen.root,
            net_client_list,
            AtomEnum::WINDOW,
            0,
            1024,
        ) {
            if let Ok(prop_reply) = prop_cookie.reply() {
                if let Some(xids) = prop_reply.value32() {
                    for xid in xids {
                        let mut title = String::new();
                        if let Some(atom_name) = net_wm_name {
                            if let Ok(title_cookie) = conn.get_property(
                                false,
                                xid,
                                atom_name,
                                utf8_string.unwrap_or(AtomEnum::STRING.into()),
                                0,
                                512,
                            ) {
                                if let Ok(title_reply) = title_cookie.reply() {
                                    title = String::from_utf8_lossy(&title_reply.value).to_string();
                                }
                            }
                        }

                        if title.is_empty() {
                            if let Ok(title_cookie) = conn.get_property(
                                false,
                                xid,
                                AtomEnum::WM_NAME,
                                AtomEnum::STRING,
                                0,
                                512,
                            ) {
                                if let Ok(title_reply) = title_cookie.reply() {
                                    title = String::from_utf8_lossy(&title_reply.value).to_string();
                                }
                            }
                        }

                        let mut pid: i64 = 0;
                        if let Some(atom_pid) = net_wm_pid {
                            if let Ok(pid_cookie) =
                                conn.get_property(false, xid, atom_pid, AtomEnum::CARDINAL, 0, 1)
                            {
                                if let Ok(pid_reply) = pid_cookie.reply() {
                                    if let Some(p) =
                                        pid_reply.value32().and_then(|mut it| it.next())
                                    {
                                        pid = p as i64;
                                    }
                                }
                            }
                        }

                        let app_name = if pid > 0 {
                            std::fs::read_to_string(format!("/proc/{}/comm", pid))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };

                        let (mut x, mut y, mut width, mut height) = (0, 0, 0, 0);
                        if let Ok(geom_cookie) = conn.get_geometry(xid) {
                            if let Ok(geom) = geom_cookie.reply() {
                                width = geom.width as i64;
                                height = geom.height as i64;
                                if let Ok(coords_cookie) =
                                    conn.translate_coordinates(xid, screen.root, 0, 0)
                                {
                                    if let Ok(coords) = coords_cookie.reply() {
                                        x = coords.dst_x as i64;
                                        y = coords.dst_y as i64;
                                    }
                                }
                            }
                        }

                        if !title.is_empty() {
                            windows.push(WindowData {
                                id: format!("0x{:x}", xid),
                                title,
                                app_name,
                                pid,
                                x,
                                y,
                                width,
                                height,
                            });
                        }
                    }
                }
            }
        }

        windows
    }

    pub fn list_windows() -> Vec<WindowData> {
        // 1. Try AT-SPI (works seamlessly on GNOME/KDE/Sway/etc. Wayland & X11)
        let atspi_windows = list_windows_atspi();
        if !atspi_windows.is_empty() {
            return atspi_windows;
        }

        // 2. Try native X11 via x11rb
        let x11_windows = list_windows_x11();
        if !x11_windows.is_empty() {
            return x11_windows;
        }

        Vec::new()
    }

    pub fn resolve_window(target: &str) -> Option<WindowData> {
        let q = target.trim().to_lowercase();
        if q.is_empty() {
            return None;
        }

        let windows = list_windows();

        // 1. Exact ID
        if let Some(w) = windows.iter().find(|w| w.id == target) {
            return Some(w.clone());
        }

        // 2. PID
        if let Ok(pid) = target.parse::<i64>() {
            if let Some(w) = windows.iter().find(|w| w.pid == pid) {
                return Some(w.clone());
            }
        }

        // 3. Exact app_name (case-insensitive)
        if let Some(w) = windows.iter().find(|w| w.app_name.to_lowercase() == q) {
            return Some(w.clone());
        }

        // 4. Exact title (case-insensitive)
        if let Some(w) = windows.iter().find(|w| w.title.to_lowercase() == q) {
            return Some(w.clone());
        }

        // 5. App name contains query (e.g., "Brave" matches "Brave Browser")
        if let Some(w) = windows
            .iter()
            .find(|w| w.app_name.to_lowercase().contains(&q))
        {
            return Some(w.clone());
        }

        // 6. Title contains query
        if let Some(w) = windows.iter().find(|w| w.title.to_lowercase().contains(&q)) {
            return Some(w.clone());
        }

        // 7. ID contains query
        if let Some(w) = windows.iter().find(|w| w.id.to_lowercase().contains(&q)) {
            return Some(w.clone());
        }

        None
    }
    
    pub fn focus_window(target: &str) -> bool {
        let win = match resolve_window(target) {
            Some(w) => w,
            None => {
                let _ = std::process::Command::new("wmctrl")
                    .args(["-a", target])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
                return false;
            }
        };

        let mut success = false;

        // 1. Try AT-SPI Action "doDefault" / GrabFocus
        if let Some(addr) = get_atspi_bus_address() {
            if let Ok(conn) = dbus::blocking::Connection::new_address(&addr) {
                if let Some((bus_name, win_path)) = win.id.split_once('|') {
                    if dbus::strings::BusName::new(bus_name).is_ok()
                        && dbus::Path::new(win_path).is_ok()
                    {
                        let win_proxy =
                            conn.with_proxy(bus_name, win_path, Duration::from_millis(500));
                        let act_res: Result<(bool,), _> =
                            win_proxy.method_call("org.a11y.atspi.Action", "DoAction", (0i32,));
                        if let Ok((ok,)) = act_res {
                            if ok {
                                success = true;
                            }
                        }
                        let focus_res: Result<(bool,), _> =
                            win_proxy.method_call("org.a11y.atspi.Component", "GrabFocus", ());
                        if let Ok((f,)) = focus_res {
                            if f {
                                success = true;
                            }
                        }
                    }
                }
            }
        }

        // 2. Try X11 via x11rb
        if let Ok((conn, screen_num)) = x11rb::connect(None) {
            use x11rb::protocol::xproto::*;
            let setup = conn.setup();
            if let Some(screen) = setup.roots.get(screen_num) {
                if let Ok(net_active) = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW") {
                    if let Ok(reply) = net_active.reply() {
                        let xid = if win.id.starts_with("0x") {
                            u32::from_str_radix(win.id.trim_start_matches("0x"), 16).unwrap_or(0)
                        } else {
                            0
                        };

                        if xid > 0 {
                            let event = ClientMessageEvent {
                                response_type: CLIENT_MESSAGE_EVENT,
                                format: 32,
                                sequence: 0,
                                window: xid,
                                type_: reply.atom,
                                data: ClientMessageData::from([2, 0, 0, 0, 0]),
                            };
                            let _ = conn.send_event(
                                false,
                                screen.root,
                                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                                event,
                            );
                            let _ = conn.flush();
                            success = true;
                        }
                    }
                }
            }
        }

        // 3. Try wmctrl
        if !win.title.is_empty() {
            if let Ok(st) = std::process::Command::new("wmctrl")
                .args(["-a", &win.title])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                if st.success() {
                    success = true;
                }
            }
        }

        success
    }

    pub fn minimize_window(target: &str) -> bool {
        let win = match resolve_window(target) {
            Some(w) => w,
            None => return false,
        };

        let mut success = false;

        // 1. Try X11 via x11rb
        if let Ok((conn, screen_num)) = x11rb::connect(None) {
            use x11rb::protocol::xproto::*;
            let setup = conn.setup();
            if let Some(screen) = setup.roots.get(screen_num) {
                if let Ok(wm_change) = conn.intern_atom(false, b"WM_CHANGE_STATE") {
                    if let Ok(reply) = wm_change.reply() {
                        let xid = if win.id.starts_with("0x") {
                            u32::from_str_radix(win.id.trim_start_matches("0x"), 16).unwrap_or(0)
                        } else {
                            0
                        };

                        if xid > 0 {
                            let event = ClientMessageEvent {
                                response_type: CLIENT_MESSAGE_EVENT,
                                format: 32,
                                sequence: 0,
                                window: xid,
                                type_: reply.atom,
                                data: ClientMessageData::from([3, 0, 0, 0, 0]), // IconicState
                            };
                            let _ = conn.send_event(
                                false,
                                screen.root,
                                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                                event,
                            );
                            let _ = conn.flush();
                            success = true;
                        }
                    }
                }
            }
        }

        // 2. Try wmctrl
        if !win.title.is_empty() {
            if let Ok(st) = std::process::Command::new("wmctrl")
                .args(["-r", &win.title, "-b", "add,hidden"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                if st.success() {
                    success = true;
                }
            }
        }

        success
    }

    pub fn maximize_window(target: &str) -> bool {
        let win = match resolve_window(target) {
            Some(w) => w,
            None => return false,
        };

        let mut success = false;

        // 1. Try X11 via x11rb
        if let Ok((conn, screen_num)) = x11rb::connect(None) {
            use x11rb::protocol::xproto::*;
            let setup = conn.setup();
            if let Some(screen) = setup.roots.get(screen_num) {
                let net_wm_state = conn
                    .intern_atom(false, b"_NET_WM_STATE")
                    .ok()
                    .and_then(|c| c.reply().ok())
                    .map(|r| r.atom);
                let max_vert = conn
                    .intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_VERT")
                    .ok()
                    .and_then(|c| c.reply().ok())
                    .map(|r| r.atom);
                let max_horz = conn
                    .intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_HORZ")
                    .ok()
                    .and_then(|c| c.reply().ok())
                    .map(|r| r.atom);

                if let (Some(state), Some(v), Some(h)) = (net_wm_state, max_vert, max_horz) {
                    let xid = if win.id.starts_with("0x") {
                        u32::from_str_radix(win.id.trim_start_matches("0x"), 16).unwrap_or(0)
                    } else {
                        0
                    };

                    if xid > 0 {
                        let event = ClientMessageEvent {
                            response_type: CLIENT_MESSAGE_EVENT,
                            format: 32,
                            sequence: 0,
                            window: xid,
                            type_: state,
                            data: ClientMessageData::from([1, v, h, 1, 0]), // 1 = _NET_WM_STATE_ADD
                        };
                        let _ = conn.send_event(
                            false,
                            screen.root,
                            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                            event,
                        );
                        let _ = conn.flush();
                        success = true;
                    }
                }
            }
        }

        // 2. Try wmctrl
        if !win.title.is_empty() {
            if let Ok(st) = std::process::Command::new("wmctrl")
                .args(["-r", &win.title, "-b", "add,maximized_vert,maximized_horz"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                if st.success() {
                    success = true;
                }
            }
        }

        success
    }

    pub fn restore_window(target: &str) -> bool {
        let win = match resolve_window(target) {
            Some(w) => w,
            None => return false,
        };

        let mut success = false;

        // 1. Try X11 via x11rb
        if let Ok((conn, screen_num)) = x11rb::connect(None) {
            use x11rb::protocol::xproto::*;
            let setup = conn.setup();
            if let Some(screen) = setup.roots.get(screen_num) {
                let net_wm_state = conn
                    .intern_atom(false, b"_NET_WM_STATE")
                    .ok()
                    .and_then(|c| c.reply().ok())
                    .map(|r| r.atom);
                let max_vert = conn
                    .intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_VERT")
                    .ok()
                    .and_then(|c| c.reply().ok())
                    .map(|r| r.atom);
                let max_horz = conn
                    .intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_HORZ")
                    .ok()
                    .and_then(|c| c.reply().ok())
                    .map(|r| r.atom);

                if let (Some(state), Some(v), Some(h)) = (net_wm_state, max_vert, max_horz) {
                    let xid = if win.id.starts_with("0x") {
                        u32::from_str_radix(win.id.trim_start_matches("0x"), 16).unwrap_or(0)
                    } else {
                        0
                    };

                    if xid > 0 {
                        let event = ClientMessageEvent {
                            response_type: CLIENT_MESSAGE_EVENT,
                            format: 32,
                            sequence: 0,
                            window: xid,
                            type_: state,
                            data: ClientMessageData::from([0, v, h, 0, 0]), // 0 = _NET_WM_STATE_REMOVE
                        };
                        let _ = conn.send_event(
                            false,
                            screen.root,
                            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                            event,
                        );
                        let _ = conn.flush();
                        success = true;
                    }
                }
            }
        }

        // 2. Try wmctrl
        if !win.title.is_empty() {
            if let Ok(st) = std::process::Command::new("wmctrl")
                .args([
                    "-r",
                    &win.title,
                    "-b",
                    "remove,maximized_vert,maximized_horz,hidden",
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                if st.success() {
                    success = true;
                }
            }
        }

        success
    }

    pub fn move_window(target: &str, x: i64, y: i64) -> bool {
        let win = match resolve_window(target) {
            Some(w) => w,
            None => return false,
        };

        let mut success = false;

        // 1. AT-SPI SetPosition
        if let Some(addr) = get_atspi_bus_address() {
            if let Ok(conn) = dbus::blocking::Connection::new_address(&addr) {
                if let Some((bus_name, win_path)) = win.id.split_once('|') {
                    if dbus::strings::BusName::new(bus_name).is_ok()
                        && dbus::Path::new(win_path).is_ok()
                    {
                        let win_proxy =
                            conn.with_proxy(bus_name, win_path, Duration::from_millis(500));
                        let pos_res: Result<(bool,), _> = win_proxy.method_call(
                            "org.a11y.atspi.Component",
                            "SetPosition",
                            (x as i32, y as i32, 0u32),
                        );
                        if let Ok((ok,)) = pos_res {
                            if ok {
                                success = true;
                            }
                        }
                    }
                }
            }
        }

        // 2. X11 via x11rb
        if let Ok((conn, _screen_num)) = x11rb::connect(None) {
            use x11rb::protocol::xproto::*;
            let xid = if win.id.starts_with("0x") {
                u32::from_str_radix(win.id.trim_start_matches("0x"), 16).unwrap_or(0)
            } else {
                0
            };

            if xid > 0 {
                let values = ConfigureWindowAux::new().x(x as i32).y(y as i32);
                let _ = conn.configure_window(xid, &values);
                let _ = conn.flush();
                success = true;
            }
        }

        // 3. wmctrl
        if !win.title.is_empty() {
            if let Ok(st) = std::process::Command::new("wmctrl")
                .args(["-r", &win.title, "-e", &format!("0,{},{},-1,-1", x, y)])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                if st.success() {
                    success = true;
                }
            }
        }

        success
    }

    pub fn resize_window(target: &str, width: i64, height: i64) -> bool {
        let win = match resolve_window(target) {
            Some(w) => w,
            None => return false,
        };

        let mut success = false;

        // 1. AT-SPI SetSize
        if let Some(addr) = get_atspi_bus_address() {
            if let Ok(conn) = dbus::blocking::Connection::new_address(&addr) {
                if let Some((bus_name, win_path)) = win.id.split_once('|') {
                    if dbus::strings::BusName::new(bus_name).is_ok()
                        && dbus::Path::new(win_path).is_ok()
                    {
                        let win_proxy =
                            conn.with_proxy(bus_name, win_path, Duration::from_millis(500));
                        let size_res: Result<(bool,), _> = win_proxy.method_call(
                            "org.a11y.atspi.Component",
                            "SetSize",
                            (width as i32, height as i32),
                        );
                        if let Ok((ok,)) = size_res {
                            if ok {
                                success = true;
                            }
                        }
                    }
                }
            }
        }

        // 2. X11 via x11rb
        if let Ok((conn, _screen_num)) = x11rb::connect(None) {
            use x11rb::protocol::xproto::*;
            let xid = if win.id.starts_with("0x") {
                u32::from_str_radix(win.id.trim_start_matches("0x"), 16).unwrap_or(0)
            } else {
                0
            };

            if xid > 0 {
                let values = ConfigureWindowAux::new()
                    .width(width as u32)
                    .height(height as u32);
                let _ = conn.configure_window(xid, &values);
                let _ = conn.flush();
                success = true;
            }
        }

        // 3. wmctrl
        if !win.title.is_empty() {
            if let Ok(st) = std::process::Command::new("wmctrl")
                .args([
                    "-r",
                    &win.title,
                    "-e",
                    &format!("0,-1,-1,{},{}", width, height),
                ])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                if st.success() {
                    success = true;
                }
            }
        }

        success
    }

    pub fn close_window(target: &str) -> bool {
        let win = match resolve_window(target) {
            Some(w) => w,
            None => return false,
        };

        let mut success = false;

        // 1. X11 WM_DELETE_WINDOW
        if let Ok((conn, _screen_num)) = x11rb::connect(None) {
            use x11rb::protocol::xproto::*;
            let wm_delete = conn
                .intern_atom(false, b"WM_DELETE_WINDOW")
                .ok()
                .and_then(|c| c.reply().ok())
                .map(|r| r.atom);
            let wm_protocols = conn
                .intern_atom(false, b"WM_PROTOCOLS")
                .ok()
                .and_then(|c| c.reply().ok())
                .map(|r| r.atom);

            if let (Some(del), Some(protocols)) = (wm_delete, wm_protocols) {
                let xid = if win.id.starts_with("0x") {
                    u32::from_str_radix(win.id.trim_start_matches("0x"), 16).unwrap_or(0)
                } else {
                    0
                };

                if xid > 0 {
                    let event = ClientMessageEvent {
                        response_type: CLIENT_MESSAGE_EVENT,
                        format: 32,
                        sequence: 0,
                        window: xid,
                        type_: protocols,
                        data: ClientMessageData::from([del, 0, 0, 0, 0]),
                    };
                    let _ = conn.send_event(false, xid, EventMask::NO_EVENT, event);
                    let _ = conn.flush();
                    success = true;
                }
            }
        }

        // 2. Try wmctrl
        if !win.title.is_empty() {
            if let Ok(st) = std::process::Command::new("wmctrl")
                .args(["-c", &win.title])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
            {
                if st.success() {
                    success = true;
                }
            }
        }

        // 3. Process SIGTERM
        if win.pid > 0 {
            unsafe {
                if libc::kill(win.pid as libc::pid_t, libc::SIGTERM) == 0 {
                    success = true;
                }
            }
        }

        success
    }
}

// =========================================================================
// Windows Platform Implementation
// =========================================================================
#[cfg(target_os = "windows")]
mod platform {
    use super::WindowData;
    use std::ffi::c_void;

    type HWND = *mut c_void;
    type BOOL = i32;
    type LPARAM = isize;
    type WNDENUMPROC = unsafe extern "system" fn(HWND, LPARAM) -> BOOL;

    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    const SW_HIDE: i32 = 0;
    const SW_NORMAL: i32 = 1;
    const SW_MINIMIZE: i32 = 6;
    const SW_MAXIMIZE: i32 = 3;
    const SW_RESTORE: i32 = 9;
    const WM_CLOSE: u32 = 0x0010;

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(lpEnumFunc: WNDENUMPROC, lParam: LPARAM) -> BOOL;
        fn IsWindowVisible(hWnd: HWND) -> BOOL;
        fn GetWindowTextW(hWnd: HWND, lpString: *mut u16, nMaxCount: i32) -> i32;
        fn GetWindowTextLengthW(hWnd: HWND) -> i32;
        fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
        fn GetWindowThreadProcessId(hWnd: HWND, lpdwProcessId: *mut u32) -> u32;
        fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
        fn SetForegroundWindow(hWnd: HWND) -> BOOL;
        fn BringWindowToTop(hWnd: HWND) -> BOOL;
        fn MoveWindow(
            hWnd: HWND,
            X: i32,
            Y: i32,
            nWidth: i32,
            nHeight: i32,
            bRepaint: BOOL,
        ) -> BOOL;
        fn PostMessageW(hWnd: HWND, Msg: u32, wParam: usize, lParam: isize) -> BOOL;
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let list = &mut *(lparam as *mut Vec<WindowData>);
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let len = GetWindowTextLengthW(hwnd);
        if len == 0 {
            return 1;
        }

        let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
        let read = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
        let title = String::from_utf16_lossy(&buf[..read as usize]);

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);

        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetWindowRect(hwnd, &mut rect);

        let x = rect.left as i64;
        let y = rect.top as i64;
        let width = (rect.right - rect.left).max(0) as i64;
        let height = (rect.bottom - rect.top).max(0) as i64;

        let id = format!("{:p}", hwnd);

        list.push(WindowData {
            id,
            title,
            app_name: String::new(),
            pid: pid as i64,
            x,
            y,
            width,
            height,
        });

        1
    }

    pub fn list_windows() -> Vec<WindowData> {
        let mut list = Vec::new();
        unsafe {
            EnumWindows(enum_windows_callback, &mut list as *mut _ as LPARAM);
        }
        list
    }

    fn find_hwnd(target: &str) -> Option<HWND> {
        // Try parsing pointer
        if target.starts_with("0x") {
            if let Ok(addr) = usize::from_str_radix(target.trim_start_matches("0x"), 16) {
                return Some(addr as HWND);
            }
        }

        // Search by title substring
        let list = list_windows();
        let query = target.to_lowercase();
        for w in list {
            if w.title.to_lowercase().contains(&query) {
                if let Ok(addr) = usize::from_str_radix(w.id.trim_start_matches("0x"), 16) {
                    return Some(addr as HWND);
                }
            }
        }
        None
    }

    pub fn focus_window(target: &str) -> bool {
        if let Some(hwnd) = find_hwnd(target) {
            unsafe {
                ShowWindow(hwnd, SW_RESTORE);
                BringWindowToTop(hwnd);
                SetForegroundWindow(hwnd) != 0
            }
        } else {
            false
        }
    }

    pub fn minimize_window(target: &str) -> bool {
        if let Some(hwnd) = find_hwnd(target) {
            unsafe { ShowWindow(hwnd, SW_MINIMIZE) != 0 }
        } else {
            false
        }
    }

    pub fn maximize_window(target: &str) -> bool {
        if let Some(hwnd) = find_hwnd(target) {
            unsafe { ShowWindow(hwnd, SW_MAXIMIZE) != 0 }
        } else {
            false
        }
    }

    pub fn restore_window(target: &str) -> bool {
        if let Some(hwnd) = find_hwnd(target) {
            unsafe { ShowWindow(hwnd, SW_RESTORE) != 0 }
        } else {
            false
        }
    }

    pub fn move_window(target: &str, x: i64, y: i64) -> bool {
        if let Some(hwnd) = find_hwnd(target) {
            unsafe {
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                GetWindowRect(hwnd, &mut rect);
                let w = rect.right - rect.left;
                let h = rect.bottom - rect.top;
                MoveWindow(hwnd, x as i32, y as i32, w, h, 1) != 0
            }
        } else {
            false
        }
    }

    pub fn resize_window(target: &str, width: i64, height: i64) -> bool {
        if let Some(hwnd) = find_hwnd(target) {
            unsafe {
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                GetWindowRect(hwnd, &mut rect);
                MoveWindow(hwnd, rect.left, rect.top, width as i32, height as i32, 1) != 0
            }
        } else {
            false
        }
    }

    pub fn close_window(target: &str) -> bool {
        if let Some(hwnd) = find_hwnd(target) {
            unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) != 0 }
        } else {
            false
        }
    }
}

// =========================================================================
// macOS Platform Implementation
// =========================================================================
#[cfg(target_os = "macos")]
mod platform {
    use super::WindowData;
    use std::process::Command;

    pub fn list_windows() -> Vec<WindowData> {
        let script = r#"
            set output to ""
            tell application "System Events"
                set procs to (every process whose background only is false)
                repeat with p in procs
                    set pName to name of p
                    set pPid to unix id of p
                    repeat with w in (every window of p)
                        set wTitle to name of w
                        set {wx, wy} to position of w
                        set {ww, wh} to size of w
                        set output to output & pName & "<|>" & pPid & "<|>" & wTitle & "<|>" & wx & "<|>" & wy & "<|>" & ww & "<|>" & wh & "\n"
                    end repeat
                end repeat
            end tell
            return output
        "#;

        let mut windows = Vec::new();
        if let Ok(output) = Command::new("osascript").args(["-e", script]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for (idx, line) in stdout.lines().enumerate() {
                let parts: Vec<&str> = line.split("<|>").collect();
                if parts.len() >= 7 {
                    let app_name = parts[0].to_string();
                    let pid: i64 = parts[1].parse().unwrap_or(0);
                    let title = parts[2].to_string();
                    let x: i64 = parts[3].parse().unwrap_or(0);
                    let y: i64 = parts[4].parse().unwrap_or(0);
                    let width: i64 = parts[5].parse().unwrap_or(0);
                    let height: i64 = parts[6].parse().unwrap_or(0);

                    windows.push(WindowData {
                        id: idx.to_string(),
                        title,
                        app_name,
                        pid,
                        x,
                        y,
                        width,
                        height,
                    });
                }
            }
        }
        windows
    }

    pub fn focus_window(target: &str) -> bool {
        let script = format!(
            r#"
            tell application "System Events"
                set targetProc to first process whose name contains "{0}" or (exists (window 1 whose name contains "{0}"))
                set frontmost of targetProc to true
            end tell
            "#,
            target.replace('"', "\\\"")
        );
        Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn minimize_window(target: &str) -> bool {
        let script = format!(
            r#"
            tell application "System Events"
                repeat with p in (every process whose background only is false)
                    if name of p contains "{0}" then
                        set miniaturized of window 1 of p to true
                        return true
                    end if
                    repeat with w in (every window of p)
                        if name of w contains "{0}" then
                            set miniaturized of w to true
                            return true
                        end if
                    end repeat
                end repeat
            end tell
            return false
            "#,
            target.replace('"', "\\\"")
        );
        Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn maximize_window(target: &str) -> bool {
        let script = format!(
            r#"
            tell application "System Events"
                repeat with p in (every process whose background only is false)
                    if name of p contains "{0}" then
                        click (first button of window 1 of p whose subrole is "AXFullScreenButton" or subrole is "AXZoomButton")
                        return true
                    end if
                end repeat
            end tell
            return false
            "#,
            target.replace('"', "\\\"")
        );
        Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn restore_window(target: &str) -> bool {
        focus_window(target)
    }

    pub fn move_window(target: &str, x: i64, y: i64) -> bool {
        let script = format!(
            r#"
            tell application "System Events"
                repeat with p in (every process whose background only is false)
                    if name of p contains "{0}" or (exists (window 1 whose name contains "{0}")) then
                        set position of window 1 of p to {{{1}, {2}}}
                        return true
                    end if
                end repeat
            end tell
            return false
            "#,
            target.replace('"', "\\\""),
            x,
            y
        );
        Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn resize_window(target: &str, width: i64, height: i64) -> bool {
        let script = format!(
            r#"
            tell application "System Events"
                repeat with p in (every process whose background only is false)
                    if name of p contains "{0}" or (exists (window 1 whose name contains "{0}")) then
                        set size of window 1 of p to {{{1}, {2}}}
                        return true
                    end if
                end repeat
            end tell
            return false
            "#,
            target.replace('"', "\\\""),
            width,
            height
        );
        Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn close_window(target: &str) -> bool {
        let script = format!(
            r#"
            tell application "System Events"
                repeat with p in (every process whose background only is false)
                    if name of p contains "{0}" or (exists (window 1 whose name contains "{0}")) then
                        click (first button of window 1 of p whose subrole is "AXCloseButton")
                        return true
                    end if
                end repeat
            end tell
            return false
            "#,
            target.replace('"', "\\\"")
        );
        Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

// Fallback for other operating systems
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod platform {
    use super::WindowData;

    pub fn list_windows() -> Vec<WindowData> {
        Vec::new()
    }
    pub fn focus_window(_target: &str) -> bool {
        false
    }
    pub fn minimize_window(_target: &str) -> bool {
        false
    }
    pub fn maximize_window(_target: &str) -> bool {
        false
    }
    pub fn restore_window(_target: &str) -> bool {
        false
    }
    pub fn move_window(_target: &str, _x: i64, _y: i64) -> bool {
        false
    }
    pub fn resize_window(_target: &str, _width: i64, _height: i64) -> bool {
        false
    }
    pub fn close_window(_target: &str) -> bool {
        false
    }
}

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    m.insert(
        "list".to_string(),
        Value::NativeCallback(|_args| {
            let windows = platform::list_windows();
            let values: Vec<Value> = windows.iter().map(|w| w.to_value()).collect();
            Ok(Value::Tuple(values))
        }),
    );

    m.insert(
        "find".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("window.find expects 1 argument (query)".to_string());
            }
            let query = extract_target(&args[0]).to_lowercase();
            let windows = platform::list_windows();
            for w in windows {
                if w.id.to_lowercase() == query
                    || w.title.to_lowercase().contains(&query)
                    || w.app_name.to_lowercase().contains(&query)
                {
                    return Ok(w.to_value());
                }
            }
            Ok(Value::Nil)
        }),
    );

    m.insert(
        "focus".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("window.focus expects 1 argument (target)".to_string());
            }
            let target = extract_target(&args[0]);
            let res = platform::focus_window(&target);
            Ok(Value::Bool(res))
        }),
    );

    m.insert(
        "minimize".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("window.minimize expects 1 argument (target)".to_string());
            }
            let target = extract_target(&args[0]);
            let res = platform::minimize_window(&target);
            Ok(Value::Bool(res))
        }),
    );

    m.insert(
        "maximize".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("window.maximize expects 1 argument (target)".to_string());
            }
            let target = extract_target(&args[0]);
            let res = platform::maximize_window(&target);
            Ok(Value::Bool(res))
        }),
    );

    m.insert(
        "restore".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("window.restore expects 1 argument (target)".to_string());
            }
            let target = extract_target(&args[0]);
            let res = platform::restore_window(&target);
            Ok(Value::Bool(res))
        }),
    );

    m.insert(
        "move".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 3 {
                return Err("window.move expects 3 arguments (target, x, y)".to_string());
            }
            let target = extract_target(&args[0]);
            let x = match &args[1] {
                Value::Int(i) => *i,
                v => v.to_string().trim_matches('"').parse().unwrap_or(0),
            };
            let y = match &args[2] {
                Value::Int(i) => *i,
                v => v.to_string().trim_matches('"').parse().unwrap_or(0),
            };
            let res = platform::move_window(&target, x, y);
            Ok(Value::Bool(res))
        }),
    );

    m.insert(
        "resize".to_string(),
        Value::NativeCallback(|args| {
            if args.len() < 3 {
                return Err("window.resize expects 3 arguments (target, width, height)".to_string());
            }
            let target = extract_target(&args[0]);
            let width = match &args[1] {
                Value::Int(i) => *i,
                v => v.to_string().trim_matches('"').parse().unwrap_or(0),
            };
            let height = match &args[2] {
                Value::Int(i) => *i,
                v => v.to_string().trim_matches('"').parse().unwrap_or(0),
            };
            let res = platform::resize_window(&target, width, height);
            Ok(Value::Bool(res))
        }),
    );

    m.insert(
        "close".to_string(),
        Value::NativeCallback(|args| {
            if args.is_empty() {
                return Err("window.close expects 1 argument (target)".to_string());
            }
            let target = extract_target(&args[0]);
            let res = platform::close_window(&target);
            Ok(Value::Bool(res))
        }),
    );

    m
}
