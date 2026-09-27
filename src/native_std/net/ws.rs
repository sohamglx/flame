use crate::vm::{Value, invoke_callback_val, NativeClosureType, NativeModuleDef, NativeFunctionDef, NativeTypeDef};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{broadcast, mpsc};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{SinkExt, StreamExt};

// Global Tokio runtime shared across all WebSocket operations
static WS_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn get_ws_runtime() -> &'static tokio::runtime::Runtime {
    WS_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("flame-ws-worker")
            .build()
            .expect("Failed to initialize Flame WebSocket tokio runtime")
    })
}

// Global active WebSocket counter & shutdown signal registry
static ACTIVE_WS_COUNT: AtomicU64 = AtomicU64::new(0);
static SHUTDOWN_SENDERS: OnceLock<Mutex<Vec<broadcast::Sender<()>>>> = OnceLock::new();

fn get_shutdown_senders() -> &'static Mutex<Vec<broadcast::Sender<()>>> {
    SHUTDOWN_SENDERS.get_or_init(|| Mutex::new(Vec::new()))
}

fn register_shutdown_sender() -> (broadcast::Sender<()>, broadcast::Receiver<()>) {
    let (tx, rx) = broadcast::channel(4);
    get_shutdown_senders().lock().unwrap().push(tx.clone());
    (tx, rx)
}

fn inc_ws_active() {
    let prev = ACTIVE_WS_COUNT.fetch_add(1, Ordering::SeqCst);
    if prev == 0 {
        crate::vm::set_event_loop_active(true);
    }
}

fn dec_ws_active() {
    let prev = ACTIVE_WS_COUNT.fetch_sub(1, Ordering::SeqCst);
    if prev <= 1 {
        crate::vm::set_event_loop_active(false);
    }
}

/// Gracefully shuts down all active WebSocket listeners and client connections.
pub fn shutdown_all_ws() {
    let senders = {
        let mut guard = get_shutdown_senders().lock().unwrap();
        let list = guard.clone();
        guard.clear();
        list
    };
    for tx in senders {
        let _ = tx.send(());
    }
    ACTIVE_WS_COUNT.store(0, Ordering::SeqCst);
    crate::vm::set_event_loop_active(false);
}

/// Helper to convert Flame Value into raw bytes
fn extract_payload_bytes(val: &Value) -> Result<Vec<u8>, String> {
    match val {
        Value::Bytes(b) => Ok(b.clone()),
        Value::Byte(b) => Ok(vec![*b]),
        Value::String(s) => Ok(s.as_bytes().to_vec()),
        Value::Tuple(items) => {
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
                    _ => return Err(format!("expected byte/int in byte array, found {}", item.type_name())),
                }
            }
            Ok(buf)
        }
        _ => Err(format!("expected String or Bytes, found {}", val.type_name())),
    }
}

/// Native module definitions for IDE completions, hover docs, and signatures
pub fn def() -> NativeModuleDef {
    NativeModuleDef {
        name: "std.net.ws".to_string(),
        description: "Production-grade, native, high-performance WebSocket subsystem supporting clients, servers, binary frames, streaming, and channel bridging.".to_string(),
        features: vec!["ws".to_string(), "net".to_string()],
        functions: vec![
            NativeFunctionDef {
                name: "connect".to_string(),
                description: "Connects to a remote WebSocket server. Returns a ClientSocket instance.\n\n**Example:**\n```flame\nimport std.net.ws\nlet socket = ws.connect(\"ws://127.0.0.1:8080\")\nsocket.send(\"Hello Server\")\nlet reply = socket.recv()\n```".to_string(),
                params: vec![("url".to_string(), "String".to_string())],
                return_type: "ClientSocket".to_string(),
            },
            NativeFunctionDef {
                name: "listen".to_string(),
                description: "Binds a high-performance WebSocket server listener. Returns a Server instance.\n\n**Example:**\n```flame\nimport std.net.ws\nlet s = ws.listen(\"127.0.0.1:8080\")\ns.onConnect(fn(client) {\n    println($\"Client connected: {client.id}\")\n    client.send(\"Welcome!\")\n})\ns.onMessage(fn(client, msg) {\n    s.broadcast($\"{client.id}: {msg}\")\n})\n```".to_string(),
                params: vec![("addr".to_string(), "String".to_string())],
                return_type: "Server".to_string(),
            },
        ],
        types: vec![
            NativeTypeDef {
                name: "ClientSocket".to_string(),
                description: "An active WebSocket client connection.".to_string(),
                fields: vec![
                    ("url".to_string(), "String".to_string()),
                    ("readyState".to_string(), "String".to_string()),
                ],
                methods: vec![
                    NativeFunctionDef {
                        name: "send".to_string(),
                        description: "Sends a UTF-8 text or binary payload through the WebSocket connection.".to_string(),
                        params: vec![("data".to_string(), "String | Bytes".to_string())],
                        return_type: "Nil".to_string(),
                    },
                    NativeFunctionDef {
                        name: "sendText".to_string(),
                        description: "Sends a UTF-8 text message through the WebSocket connection.".to_string(),
                        params: vec![("text".to_string(), "String".to_string())],
                        return_type: "Nil".to_string(),
                    },
                    NativeFunctionDef {
                        name: "sendBytes".to_string(),
                        description: "Sends raw binary data as a binary WebSocket frame.".to_string(),
                        params: vec![("bytes".to_string(), "Bytes".to_string())],
                        return_type: "Nil".to_string(),
                    },
                    NativeFunctionDef {
                        name: "recv".to_string(),
                        description: "Blocks until the next UTF-8 text message arrives from the WebSocket connection.".to_string(),
                        params: vec![],
                        return_type: "String".to_string(),
                    },
                    NativeFunctionDef {
                        name: "recvBytes".to_string(),
                        description: "Blocks until the next binary frame arrives and returns it as Bytes.".to_string(),
                        params: vec![],
                        return_type: "Bytes".to_string(),
                    },
                    NativeFunctionDef {
                        name: "ping".to_string(),
                        description: "Sends a WebSocket Ping frame with optional payload data.".to_string(),
                        params: vec![("data".to_string(), "String | Bytes?".to_string())],
                        return_type: "Nil".to_string(),
                    },
                    NativeFunctionDef {
                        name: "pong".to_string(),
                        description: "Sends a WebSocket Pong frame with optional payload data.".to_string(),
                        params: vec![("data".to_string(), "String | Bytes?".to_string())],
                        return_type: "Nil".to_string(),
                    },
                    NativeFunctionDef {
                        name: "close".to_string(),
                        description: "Gracefully closes the WebSocket connection with optional status code and reason.".to_string(),
                        params: vec![("code".to_string(), "Int?".to_string()), ("reason".to_string(), "String?".to_string())],
                        return_type: "Nil".to_string(),
                    },
                    NativeFunctionDef {
                        name: "messages".to_string(),
                        description: "Returns an async message Stream supporting `.forEach()`, `.onEach()`, and `.toChannel()`.".to_string(),
                        params: vec![],
                        return_type: "Stream".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onMessage".to_string(),
                        description: "Registers an asynchronous callback invoked when a text message is received.".to_string(),
                        params: vec![("callback".to_string(), "fn(msg: String)".to_string())],
                        return_type: "ClientSocket".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onBinary".to_string(),
                        description: "Registers an asynchronous callback invoked when a binary frame is received.".to_string(),
                        params: vec![("callback".to_string(), "fn(bytes: Bytes)".to_string())],
                        return_type: "ClientSocket".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onClose".to_string(),
                        description: "Registers an asynchronous callback invoked when the socket closes.".to_string(),
                        params: vec![("callback".to_string(), "fn(code: Int, reason: String)".to_string())],
                        return_type: "ClientSocket".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onError".to_string(),
                        description: "Registers an asynchronous callback invoked when a connection error occurs.".to_string(),
                        params: vec![("callback".to_string(), "fn(error: String)".to_string())],
                        return_type: "ClientSocket".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onPing".to_string(),
                        description: "Registers an asynchronous callback invoked when a Ping frame is received.".to_string(),
                        params: vec![("callback".to_string(), "fn(data: Bytes)".to_string())],
                        return_type: "ClientSocket".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onPong".to_string(),
                        description: "Registers an asynchronous callback invoked when a Pong frame is received.".to_string(),
                        params: vec![("callback".to_string(), "fn(data: Bytes)".to_string())],
                        return_type: "ClientSocket".to_string(),
                    },
                ],
            },
            NativeTypeDef {
                name: "Server".to_string(),
                description: "An active WebSocket server listener.".to_string(),
                fields: vec![
                    ("port".to_string(), "Int".to_string()),
                    ("host".to_string(), "String".to_string()),
                    ("address".to_string(), "String".to_string()),
                ],
                methods: vec![
                    NativeFunctionDef {
                        name: "onConnect".to_string(),
                        description: "Registers a callback invoked when a new client connects.\n\n**Example:**\n```flame\ns.onConnect(fn(client) {\n    println($\"New client {client.id} from {client.address}\")\n    client.send(\"Connected!\")\n})\n```".to_string(),
                        params: vec![("callback".to_string(), "fn(client: ServerClient)".to_string())],
                        return_type: "Server".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onMessage".to_string(),
                        description: "Registers a callback invoked when a connected client sends a text message.\n\n**Example:**\n```flame\ns.onMessage(fn(client, msg) {\n    s.broadcast($\"{client.id}: {msg}\")\n})\n```".to_string(),
                        params: vec![("callback".to_string(), "fn(client: ServerClient, msg: String)".to_string())],
                        return_type: "Server".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onBinary".to_string(),
                        description: "Registers a callback invoked when a connected client sends binary data.\n\n**Example:**\n```flame\ns.onBinary(fn(client, bytes) {\n    println($\"Received {bytes.len()} bytes from {client.id}\")\n})\n```".to_string(),
                        params: vec![("callback".to_string(), "fn(client: ServerClient, bytes: Bytes)".to_string())],
                        return_type: "Server".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onClose".to_string(),
                        description: "Registers a callback invoked when a client connection is closed.".to_string(),
                        params: vec![("callback".to_string(), "fn(client: ServerClient, code: Int, reason: String)".to_string())],
                        return_type: "Server".to_string(),
                    },
                    NativeFunctionDef {
                        name: "onError".to_string(),
                        description: "Registers a callback invoked when a client encounters an error.".to_string(),
                        params: vec![("callback".to_string(), "fn(client: ServerClient, error: String)".to_string())],
                        return_type: "Server".to_string(),
                    },
                    NativeFunctionDef {
                        name: "broadcast".to_string(),
                        description: "Broadcasts a text or binary payload to all currently connected clients.\n\n**Example:**\n```flame\ns.broadcast(\"Server announcement: rebooting in 5m\")\n```".to_string(),
                        params: vec![("data".to_string(), "String | Bytes".to_string())],
                        return_type: "Nil".to_string(),
                    },
                    NativeFunctionDef {
                        name: "connections".to_string(),
                        description: "Returns the current number of active client connections.".to_string(),
                        params: vec![],
                        return_type: "Int".to_string(),
                    },
                    NativeFunctionDef {
                        name: "close".to_string(),
                        description: "Gracefully stops accepting new connections and shuts down the server listener.".to_string(),
                        params: vec![],
                        return_type: "Nil".to_string(),
                    },
                ],
            },
        ],
    }
}

#[inline]
pub(crate) fn get_arg<'a>(args: &'a [Value], index: usize) -> Option<&'a Value> {
    if !args.is_empty() && matches!(args[0], Value::Object(_) | Value::Formula(_)) {
        args.get(index + 1)
    } else {
        args.get(index)
    }
}

#[inline]
pub(crate) fn get_self(args: &[Value]) -> Value {
    if !args.is_empty() && matches!(args[0], Value::Object(_) | Value::Formula(_)) {
        args[0].clone()
    } else {
        Value::Nil
    }
}

#[cfg(feature = "ws")]
pub fn init() -> HashMap<String, Value> {
    let mut map = HashMap::new();

    // ws.connect(url)
    let connect_fn = Value::NativeCallback(|args| {
        if let Some(url_val) = get_arg(&args, 0) {
            let url_str = match url_val {
                Value::String(s) => s.clone(),
                _ => return Err("connect expects a URL string".to_string()),
            };
            connect_client(&url_str)
        } else {
            Err("connect expects 1 argument (url)".to_string())
        }
    });

    // ws.listen(addr)
    let listen_fn = Value::NativeCallback(|args| {
        if let Some(addr_val) = get_arg(&args, 0) {
            let addr_str = match addr_val {
                Value::String(s) => s.clone(),
                _ => return Err("listen expects an address string (e.g. \"127.0.0.1:8080\")".to_string()),
            };
            listen_server(&addr_str)
        } else {
            Err("listen expects 1 argument (addr)".to_string())
        }
    });

    // Socket object with static methods connect & listen
    let mut socket_obj = HashMap::new();
    socket_obj.insert("connect".to_string(), connect_fn.clone());
    socket_obj.insert("listen".to_string(), listen_fn.clone());
    socket_obj.insert("new".to_string(), connect_fn.clone());

    let socket_val = Value::Object(socket_obj.clone());

    // Top-level exports
    map.insert("connect".to_string(), connect_fn.clone());
    map.insert("listen".to_string(), listen_fn.clone());
    map.insert("Socket".to_string(), socket_val.clone());
    map.insert("WebSocket".to_string(), socket_val.clone());

    // Also export ws namespace itself
    let mut ws_ns = HashMap::new();
    ws_ns.insert("connect".to_string(), connect_fn);
    ws_ns.insert("listen".to_string(), listen_fn);
    ws_ns.insert("Socket".to_string(), socket_val.clone());
    ws_ns.insert("WebSocket".to_string(), socket_val);
    map.insert("ws".to_string(), Value::Object(ws_ns));

    map
}

#[cfg(not(feature = "ws"))]
pub fn init() -> HashMap<String, Value> {
    HashMap::new()
}

// =========================================================================
// WebSocket Client Implementation
// =========================================================================

fn connect_client(url_str: &str) -> Result<Value, String> {
    let rt = get_ws_runtime();

    let url_owned = url_str.to_string();
    let connect_res = rt.block_on(async move {
        tokio_tungstenite::connect_async(&url_owned).await
    });

    let (ws_stream, _) = connect_res.map_err(|e| format!("WebSocket connect error: {}", e))?;
    let (mut ws_write, mut ws_read) = ws_stream.split();

    inc_ws_active();
    let (shutdown_tx, mut shutdown_rx) = register_shutdown_sender();
    let is_closed = Arc::new(AtomicBool::new(false));

    // Channel for outbound messages to write task
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(256);
    // Channel for inbound messages for synchronous recv() / recvBytes()
    let (in_tx, in_rx) = mpsc::channel::<Message>(256);
    let in_rx = Arc::new(tokio::sync::Mutex::new(in_rx));

    // Callbacks container
    let callbacks: Arc<Mutex<HashMap<String, Value>>> = Arc::new(Mutex::new(HashMap::new()));

    // Task 1: Outbound writer
    let is_closed_writer = is_closed.clone();
    rt.spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => {
                    let _ = ws_write.send(Message::Close(None)).await;
                    break;
                }
                msg_opt = out_rx.recv() => {
                    match msg_opt {
                        Some(msg) => {
                            let is_close_msg = matches!(msg, Message::Close(_));
                            if ws_write.send(msg).await.is_err() {
                                break;
                            }
                            if is_close_msg {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
        is_closed_writer.store(true, Ordering::SeqCst);
    });

    // Task 2: Inbound reader
    let cbs = callbacks.clone();
    let in_tx_reader = in_tx.clone();
    let is_closed_reader = is_closed.clone();
    let out_tx_close = out_tx.clone();

    rt.spawn(async move {
        while let Some(msg_res) = ws_read.next().await {
            match msg_res {
                Ok(msg) => {
                    match &msg {
                        Message::Text(text) => {
                            let cb_opt = cbs.lock().unwrap().get("onMessage").cloned();
                            if let Some(cb) = cb_opt {
                                let _ = invoke_callback_val(&cb, vec![Value::String(text.to_string())]);
                            }
                            let _ = in_tx_reader.send(msg).await;
                        }
                        Message::Binary(bin) => {
                            let cb_opt = cbs.lock().unwrap().get("onBinary").cloned();
                            if let Some(cb) = cb_opt {
                                let _ = invoke_callback_val(&cb, vec![Value::Bytes(bin.to_vec())]);
                            }
                            let _ = in_tx_reader.send(msg).await;
                        }
                        Message::Ping(data) => {
                            let cb_opt = cbs.lock().unwrap().get("onPing").cloned();
                            if let Some(cb) = cb_opt {
                                let _ = invoke_callback_val(&cb, vec![Value::Bytes(data.to_vec())]);
                            }
                            // Auto-reply with Pong per RFC 6455
                            let _ = out_tx_close.send(Message::Pong(data.clone())).await;
                        }
                        Message::Pong(data) => {
                            let cb_opt = cbs.lock().unwrap().get("onPong").cloned();
                            if let Some(cb) = cb_opt {
                                let _ = invoke_callback_val(&cb, vec![Value::Bytes(data.to_vec())]);
                            }
                        }
                        Message::Close(frame) => {
                            let (code, reason): (i64, String) = frame.as_ref().map(|f| (u16::from(f.code) as i64, f.reason.to_string()))
                                .unwrap_or((1000, "Normal Closure".to_string()));
                            let cb_opt = cbs.lock().unwrap().get("onClose").cloned();
                            if let Some(cb) = cb_opt {
                                let _ = invoke_callback_val(&cb, vec![Value::Int(code), Value::String(reason)]);
                            }
                            let _ = in_tx_reader.send(msg).await;
                            break;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    let cb_opt = cbs.lock().unwrap().get("onError").cloned();
                    if let Some(cb) = cb_opt {
                        let _ = invoke_callback_val(&cb, vec![Value::String(e.to_string())]);
                    }
                    break;
                }
            }
        }
        is_closed_reader.store(true, Ordering::SeqCst);
    });

    // Build ClientSocket object
    let mut sock_map = HashMap::new();
    sock_map.insert("url".to_string(), Value::String(url_str.to_string()));
    sock_map.insert("readyState".to_string(), Value::String("open".to_string()));

    // sock.send(data)
    let out_send = out_tx.clone();
    sock_map.insert("send".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        if let Some(val) = get_arg(&args, 0) {
            let msg = match val {
                Value::String(s) => Message::Text(s.clone().into()),
                Value::Bytes(b) => Message::Binary(b.clone().into()),
                _ => {
                    let bytes = extract_payload_bytes(val).map_err(|e| format!("send error: {}", e))?;
                    Message::Binary(bytes.into())
                }
            };
            out_send.blocking_send(msg).map_err(|e| format!("WebSocket send error: {}", e))?;
            Ok(Value::Nil)
        } else {
            Err("send expects 1 argument (string or bytes)".to_string())
        }
    }))));

    // sock.sendText(text)
    let out_send_text = out_tx.clone();
    sock_map.insert("sendText".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        if let Some(Value::String(s)) = get_arg(&args, 0) {
            out_send_text.blocking_send(Message::Text(s.clone().into()))
                .map_err(|e| format!("WebSocket sendText error: {}", e))?;
            Ok(Value::Nil)
        } else {
            Err("sendText expects 1 argument (string)".to_string())
        }
    }))));

    // sock.sendBytes(bytes)
    let out_send_bytes = out_tx.clone();
    sock_map.insert("sendBytes".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        if let Some(val) = get_arg(&args, 0) {
            let b = extract_payload_bytes(val).map_err(|e| format!("sendBytes error: {}", e))?;
            out_send_bytes.blocking_send(Message::Binary(b.into()))
                .map_err(|e| format!("WebSocket sendBytes error: {}", e))?;
            Ok(Value::Nil)
        } else {
            Err("sendBytes expects 1 argument (bytes)".to_string())
        }
    }))));

    // sock.ping(data)
    let out_ping = out_tx.clone();
    sock_map.insert("ping".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let payload = if let Some(val) = get_arg(&args, 0) {
            extract_payload_bytes(val).unwrap_or_default()
        } else {
            Vec::new()
        };
        out_ping.blocking_send(Message::Ping(payload.into()))
            .map_err(|e| format!("WebSocket ping error: {}", e))?;
        Ok(Value::Nil)
    }))));

    // sock.pong(data)
    let out_pong = out_tx.clone();
    sock_map.insert("pong".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let payload = if let Some(val) = get_arg(&args, 0) {
            extract_payload_bytes(val).unwrap_or_default()
        } else {
            Vec::new()
        };
        out_pong.blocking_send(Message::Pong(payload.into()))
            .map_err(|e| format!("WebSocket pong error: {}", e))?;
        Ok(Value::Nil)
    }))));

    // sock.recv()
    let in_recv = in_rx.clone();
    sock_map.insert("recv".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |_| {
        let rt = get_ws_runtime();
        let in_lock = in_recv.clone();
        let res = rt.block_on(async {
            let mut guard = in_lock.lock().await;
            guard.recv().await
        });
        match res {
            Some(Message::Text(t)) => Ok(Value::String(t.to_string())),
            Some(Message::Binary(b)) => Ok(Value::String(String::from_utf8_lossy(&b).to_string())),
            Some(Message::Close(frame)) => {
                let reason = frame.map(|f| f.reason.to_string()).unwrap_or_else(|| "Closed".to_string());
                Err(format!("WebSocket closed: {}", reason))
            }
            Some(_) => Ok(Value::String(String::new())),
            None => Err("WebSocket connection closed".to_string()),
        }
    }))));

    // sock.recvBytes()
    let in_recv_bytes = in_rx.clone();
    sock_map.insert("recvBytes".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |_| {
        let rt = get_ws_runtime();
        let in_lock = in_recv_bytes.clone();
        let res = rt.block_on(async {
            let mut guard = in_lock.lock().await;
            guard.recv().await
        });
        match res {
            Some(Message::Binary(b)) => Ok(Value::Bytes(b.to_vec())),
            Some(Message::Text(t)) => Ok(Value::Bytes(t.as_bytes().to_vec())),
            Some(_) => Ok(Value::Bytes(Vec::new())),
            None => Err("WebSocket connection closed".to_string()),
        }
    }))));

    // sock.close(code, reason)
    let out_close = out_tx.clone();
    let is_closed_sock = is_closed.clone();
    let shutdown_signal = shutdown_tx.clone();
    sock_map.insert("close".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |_args| {
        if !is_closed_sock.swap(true, Ordering::SeqCst) {
            let _ = shutdown_signal.send(());
            let _ = out_close.blocking_send(Message::Close(None));
            dec_ws_active();
        }
        Ok(Value::Nil)
    }))));

    // sock.onMessage(cb)
    let cbs_msg = callbacks.clone();
    sock_map.insert("onMessage".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_msg.lock().unwrap().insert("onMessage".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onMessage expects 1 argument (callback)".to_string())
        }
    }))));

    // sock.onBinary(cb)
    let cbs_bin = callbacks.clone();
    sock_map.insert("onBinary".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_bin.lock().unwrap().insert("onBinary".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onBinary expects 1 argument (callback)".to_string())
        }
    }))));

    // sock.onClose(cb)
    let cbs_close = callbacks.clone();
    sock_map.insert("onClose".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_close.lock().unwrap().insert("onClose".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onClose expects 1 argument (callback)".to_string())
        }
    }))));

    // sock.onError(cb)
    let cbs_err = callbacks.clone();
    sock_map.insert("onError".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_err.lock().unwrap().insert("onError".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onError expects 1 argument (callback)".to_string())
        }
    }))));

    // sock.onPing(cb)
    let cbs_ping = callbacks.clone();
    sock_map.insert("onPing".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_ping.lock().unwrap().insert("onPing".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onPing expects 1 argument (callback)".to_string())
        }
    }))));

    // sock.onPong(cb)
    let cbs_pong = callbacks.clone();
    sock_map.insert("onPong".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_pong.lock().unwrap().insert("onPong".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onPong expects 1 argument (callback)".to_string())
        }
    }))));

    // sock.messages() -> Stream
    let in_stream_rx = in_rx.clone();
    sock_map.insert("messages".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |_| {
        let stream_obj = create_stream_object(in_stream_rx.clone());
        Ok(stream_obj)
    }))));

    Ok(Value::Object(sock_map))
}

// =========================================================================
// Stream Object with .forEach(), .onEach(), and .toChannel()
// =========================================================================

fn create_stream_object(in_rx: Arc<tokio::sync::Mutex<mpsc::Receiver<Message>>>) -> Value {
    let mut map = HashMap::new();

    // stream.forEach(cb) / stream.onEach(cb)
    let rx_for_each = in_rx.clone();
    let for_each_fn = Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        if let Some(cb) = get_arg(&args, 0) {
            let cb_clone = cb.clone();
            let rx_clone = rx_for_each.clone();
            let rt = get_ws_runtime();

            rt.spawn(async move {
                loop {
                    let msg_opt = {
                        let mut guard = rx_clone.lock().await;
                        guard.recv().await
                    };
                    match msg_opt {
                        Some(Message::Text(t)) => {
                            let _ = invoke_callback_val(&cb_clone, vec![Value::String(t.to_string())]);
                        }
                        Some(Message::Binary(b)) => {
                            let _ = invoke_callback_val(&cb_clone, vec![Value::Bytes(b.to_vec())]);
                        }
                        Some(Message::Close(_)) | None => break,
                        _ => {}
                    }
                }
            });
            Ok(Value::Nil)
        } else {
            Err("forEach expects 1 argument (callback)".to_string())
        }
    })));

    map.insert("forEach".to_string(), for_each_fn.clone());
    map.insert("onEach".to_string(), for_each_fn);

    // stream.toChannel() -> returns (Sender, Receiver) bridging directly into Flame std.thread
    let rx_chan = in_rx.clone();
    map.insert("toChannel".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |_| {
        let mut counter = crate::vm::get_channel_counter().lock().unwrap();
        *counter += 1;
        let chan_id = *counter;

        let (tx, rx) = std::sync::mpsc::channel::<Value>();
        crate::vm::get_channels().lock().unwrap().insert(chan_id, tx.clone());
        crate::vm::get_receivers()
            .lock()
            .unwrap()
            .insert(chan_id, Arc::new(Mutex::new(rx)));

        let rt = get_ws_runtime();
        let rx_inner = rx_chan.clone();
        rt.spawn(async move {
            loop {
                let msg_opt = {
                    let mut guard = rx_inner.lock().await;
                    guard.recv().await
                };
                match msg_opt {
                    Some(Message::Text(t)) => {
                        if tx.send(Value::String(t.to_string())).is_err() {
                            break;
                        }
                    }
                    Some(Message::Binary(b)) => {
                        if tx.send(Value::Bytes(b.to_vec())).is_err() {
                            break;
                        }
                    }
                    Some(Message::Close(_)) | None => break,
                    _ => {}
                }
            }
        });

        Ok(Value::Tuple(vec![
            Value::Sender(chan_id),
            Value::Receiver(chan_id),
        ]))
    }))));

    Value::Object(map)
}

// =========================================================================
// WebSocket Server Implementation
// =========================================================================

fn listen_server(addr_str: &str) -> Result<Value, String> {
    let rt = get_ws_runtime();

    let addr_owned = addr_str.to_string();
    let listener_res = rt.block_on(async move {
        TcpListener::bind(&addr_owned).await
    });

    let listener = listener_res.map_err(|e| format!("WebSocket bind error on '{}': {}", addr_str, e))?;
    let local_addr = listener.local_addr().map_err(|e| e.to_string())?;

    inc_ws_active();
    let (shutdown_tx, mut shutdown_rx) = register_shutdown_sender();
    let is_closed = Arc::new(AtomicBool::new(false));

    // Connected clients registry: id -> sender
    let clients: Arc<Mutex<HashMap<u64, mpsc::Sender<Message>>>> = Arc::new(Mutex::new(HashMap::new()));
    let next_client_id = Arc::new(AtomicU64::new(1));

    // Server event callbacks
    let callbacks: Arc<Mutex<HashMap<String, Value>>> = Arc::new(Mutex::new(HashMap::new()));

    // Server accept loop task
    let clients_accept = clients.clone();
    let cbs_accept = callbacks.clone();
    let is_closed_server = is_closed.clone();
    let shutdown_rx_server = shutdown_tx.subscribe();

    rt.spawn(async move {
        let mut shutdown_sub = shutdown_rx_server;
        loop {
            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => {
                    break;
                }
                _ = shutdown_sub.recv() => {
                    break;
                }
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((stream, peer_addr)) => {
                            let client_id = next_client_id.fetch_add(1, Ordering::Relaxed);
                            let clients_inner = clients_accept.clone();
                            let cbs_inner = cbs_accept.clone();

                            tokio::spawn(async move {
                                let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                                    Ok(ws) => ws,
                                    Err(e) => {
                                        let cb_err = cbs_inner.lock().unwrap().get("onError").cloned();
                                        if let Some(cb) = cb_err {
                                            let _ = invoke_callback_val(&cb, vec![Value::Nil, Value::String(e.to_string())]);
                                        }
                                        return;
                                    }
                                };

                                let (mut write_half, mut read_half) = ws_stream.split();
                                let (client_tx, mut client_rx) = mpsc::channel::<Message>(256);

                                clients_inner.lock().unwrap().insert(client_id, client_tx.clone());

                                // Construct the client handle object passed to Flame callbacks
                                let client_val = create_server_client_val(client_id, peer_addr.to_string(), client_tx.clone());

                                // onConnect callback
                                let cb_conn = cbs_inner.lock().unwrap().get("onConnect").cloned();
                                if let Some(cb) = cb_conn {
                                    let _ = invoke_callback_val(&cb, vec![client_val.clone()]);
                                }

                                // Writer task for this client
                                let write_task = tokio::spawn(async move {
                                    while let Some(msg) = client_rx.recv().await {
                                        let is_close = matches!(msg, Message::Close(_));
                                        if write_half.send(msg).await.is_err() {
                                            break;
                                        }
                                        if is_close {
                                            break;
                                        }
                                    }
                                });

                                // Reader loop for this client
                                while let Some(msg_res) = read_half.next().await {
                                    match msg_res {
                                        Ok(msg) => {
                                            match msg {
                                                Message::Text(t) => {
                                                    let cb_msg = cbs_inner.lock().unwrap().get("onMessage").cloned();
                                                    if let Some(cb) = cb_msg {
                                                        let _ = invoke_callback_val(&cb, vec![client_val.clone(), Value::String(t.to_string())]);
                                                    }
                                                }
                                                Message::Binary(b) => {
                                                    let cb_bin = cbs_inner.lock().unwrap().get("onBinary").cloned();
                                                    if let Some(cb) = cb_bin {
                                                        let _ = invoke_callback_val(&cb, vec![client_val.clone(), Value::Bytes(b.to_vec())]);
                                                    }
                                                }
                                                Message::Ping(data) => {
                                                    let cb_ping = cbs_inner.lock().unwrap().get("onPing").cloned();
                                                    if let Some(cb) = cb_ping {
                                                        let _ = invoke_callback_val(&cb, vec![client_val.clone(), Value::Bytes(data.to_vec())]);
                                                    }
                                                    let _ = client_tx.send(Message::Pong(data)).await;
                                                }
                                                Message::Pong(data) => {
                                                    let cb_pong = cbs_inner.lock().unwrap().get("onPong").cloned();
                                                    if let Some(cb) = cb_pong {
                                                        let _ = invoke_callback_val(&cb, vec![client_val.clone(), Value::Bytes(data.to_vec())]);
                                                    }
                                                }
                                                Message::Close(frame) => {
                                                    let (code, reason): (i64, String) = frame.as_ref()
                                                        .map(|f| (u16::from(f.code) as i64, f.reason.to_string()))
                                                        .unwrap_or((1000, "Normal Closure".to_string()));
                                                    let cb_close = cbs_inner.lock().unwrap().get("onClose").cloned();
                                                    if let Some(cb) = cb_close {
                                                        let _ = invoke_callback_val(&cb, vec![client_val.clone(), Value::Int(code), Value::String(reason)]);
                                                    }
                                                    break;
                                                }
                                                _ => {}
                                            }
                                        }
                                        Err(e) => {
                                            let cb_err = cbs_inner.lock().unwrap().get("onError").cloned();
                                            if let Some(cb) = cb_err {
                                                let _ = invoke_callback_val(&cb, vec![client_val.clone(), Value::String(e.to_string())]);
                                            }
                                            break;
                                        }
                                    }
                                }

                                // Remove client upon disconnect
                                clients_inner.lock().unwrap().remove(&client_id);
                                let _ = write_task.await;
                            });
                        }
                        Err(_) => break,
                    }
                }
            }
        }
        is_closed_server.store(true, Ordering::SeqCst);
    });

    // Build Server object (the listener instance 's')
    let mut s_map = HashMap::new();
    s_map.insert("port".to_string(), Value::Int(local_addr.port() as i64));
    s_map.insert("host".to_string(), Value::String(local_addr.ip().to_string()));
    s_map.insert("address".to_string(), Value::String(local_addr.to_string()));

    // s.onConnect(cb)
    let cbs_conn = callbacks.clone();
    s_map.insert("onConnect".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_conn.lock().unwrap().insert("onConnect".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onConnect expects 1 argument (callback)".to_string())
        }
    }))));

    // s.onMessage(cb)
    let cbs_msg = callbacks.clone();
    s_map.insert("onMessage".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_msg.lock().unwrap().insert("onMessage".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onMessage expects 1 argument (callback)".to_string())
        }
    }))));

    // s.onBinary(cb)
    let cbs_bin = callbacks.clone();
    s_map.insert("onBinary".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_bin.lock().unwrap().insert("onBinary".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onBinary expects 1 argument (callback)".to_string())
        }
    }))));

    // s.onClose(cb)
    let cbs_close = callbacks.clone();
    s_map.insert("onClose".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_close.lock().unwrap().insert("onClose".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onClose expects 1 argument (callback)".to_string())
        }
    }))));

    // s.onError(cb)
    let cbs_err = callbacks.clone();
    s_map.insert("onError".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_err.lock().unwrap().insert("onError".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onError expects 1 argument (callback)".to_string())
        }
    }))));

    // s.onPing(cb)
    let cbs_ping = callbacks.clone();
    s_map.insert("onPing".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_ping.lock().unwrap().insert("onPing".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onPing expects 1 argument (callback)".to_string())
        }
    }))));

    // s.onPong(cb)
    let cbs_pong = callbacks.clone();
    s_map.insert("onPong".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let self_val = get_self(&args);
        if let Some(cb) = get_arg(&args, 0) {
            cbs_pong.lock().unwrap().insert("onPong".to_string(), cb.clone());
            Ok(self_val)
        } else {
            Err("onPong expects 1 argument (callback)".to_string())
        }
    }))));

    // s.broadcast(data)
    let clients_bcast = clients.clone();
    s_map.insert("broadcast".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        if let Some(val) = get_arg(&args, 0) {
            let msg = match val {
                Value::String(s) => Message::Text(s.clone().into()),
                Value::Bytes(b) => Message::Binary(b.clone().into()),
                _ => {
                    let bytes = extract_payload_bytes(val).map_err(|e| format!("broadcast error: {}", e))?;
                    Message::Binary(bytes.into())
                }
            };
            let mut to_remove = Vec::new();
            let guard = clients_bcast.lock().unwrap();
            for (id, tx) in guard.iter() {
                if tx.try_send(msg.clone()).is_err() {
                    to_remove.push(*id);
                }
            }
            drop(guard);
            if !to_remove.is_empty() {
                let mut guard = clients_bcast.lock().unwrap();
                for id in to_remove {
                    guard.remove(&id);
                }
            }
            Ok(Value::Nil)
        } else {
            Err("broadcast expects 1 argument (string or bytes)".to_string())
        }
    }))));

    // s.connections() -> Int
    let clients_count = clients.clone();
    s_map.insert("connections".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |_| {
        let count = clients_count.lock().unwrap().len() as i64;
        Ok(Value::Int(count))
    }))));

    // s.close() / s.shutdown()
    let is_closed_s = is_closed.clone();
    let shutdown_s = shutdown_tx.clone();
    let clients_close = clients.clone();
    let close_fn = Value::NativeClosure(NativeClosureType(Arc::new(move |_| {
        if !is_closed_s.swap(true, Ordering::SeqCst) {
            let _ = shutdown_s.send(());
            let guard = clients_close.lock().unwrap();
            for (_, tx) in guard.iter() {
                let _ = tx.try_send(Message::Close(None));
            }
            dec_ws_active();
        }
        Ok(Value::Nil)
    })));

    s_map.insert("close".to_string(), close_fn.clone());
    s_map.insert("shutdown".to_string(), close_fn);

    Ok(Value::Object(s_map))
}

// =========================================================================
// Server Client Connection Helper
// =========================================================================

fn create_server_client_val(id: u64, addr: String, tx: mpsc::Sender<Message>) -> Value {
    let mut map = HashMap::new();
    map.insert("id".to_string(), Value::Int(id as i64));
    map.insert("address".to_string(), Value::String(addr.clone()));
    map.insert("remote_addr".to_string(), Value::String(addr));

    // client.send(data)
    let tx_send = tx.clone();
    map.insert("send".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        if let Some(val) = get_arg(&args, 0) {
            let msg = match val {
                Value::String(s) => Message::Text(s.clone().into()),
                Value::Bytes(b) => Message::Binary(b.clone().into()),
                _ => {
                    let bytes = extract_payload_bytes(val).map_err(|e| format!("send error: {}", e))?;
                    Message::Binary(bytes.into())
                }
            };
            tx_send.try_send(msg).map_err(|e| format!("client.send error: {}", e))?;
            Ok(Value::Nil)
        } else {
            Err("client.send expects 1 argument (string or bytes)".to_string())
        }
    }))));

    // client.sendText(text)
    let tx_send_text = tx.clone();
    map.insert("sendText".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        if let Some(Value::String(s)) = get_arg(&args, 0) {
            tx_send_text.try_send(Message::Text(s.clone().into()))
                .map_err(|e| format!("client.sendText error: {}", e))?;
            Ok(Value::Nil)
        } else {
            Err("client.sendText expects 1 argument (string)".to_string())
        }
    }))));

    // client.sendBytes(bytes)
    let tx_send_bytes = tx.clone();
    map.insert("sendBytes".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        if let Some(val) = get_arg(&args, 0) {
            let bytes = extract_payload_bytes(val).map_err(|e| format!("client.sendBytes error: {}", e))?;
            tx_send_bytes.try_send(Message::Binary(bytes.into()))
                .map_err(|e| format!("client.sendBytes error: {}", e))?;
            Ok(Value::Nil)
        } else {
            Err("client.sendBytes expects 1 argument (bytes)".to_string())
        }
    }))));

    // client.ping(data)
    let tx_ping = tx.clone();
    map.insert("ping".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |args| {
        let payload = if let Some(val) = get_arg(&args, 0) {
            extract_payload_bytes(val).unwrap_or_default()
        } else {
            Vec::new()
        };
        tx_ping.try_send(Message::Ping(payload.into()))
            .map_err(|e| format!("client.ping error: {}", e))?;
        Ok(Value::Nil)
    }))));

    // client.close(code, reason)
    let tx_close = tx.clone();
    map.insert("close".to_string(), Value::NativeClosure(NativeClosureType(Arc::new(move |_args| {
        let _ = tx_close.try_send(Message::Close(None));
        Ok(Value::Nil)
    }))));

    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_server_client_echo() {
        let server_val = listen_server("127.0.0.1:0").expect("Failed to bind server");
        let server_obj = match &server_val {
            Value::Object(m) => m,
            _ => panic!("Expected Object for server"),
        };
        let port = match server_obj.get("port").unwrap() {
            Value::Int(p) => *p,
            _ => panic!("Expected Int port"),
        };

        // onMessage echo back
        let on_message = match server_obj.get("onMessage").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected onMessage closure"),
        };

        let handler = Arc::new(|args: Vec<Value>| {
            if args.len() >= 2 {
                let client = &args[0];
                let msg = &args[1];
                if let Value::Object(c_map) = client {
                    if let Some(Value::NativeClosure(NativeClosureType(send_cb))) = c_map.get("send") {
                        let _ = send_cb(vec![Value::String(format!("echo: {}", msg))]);
                    }
                }
            }
            Ok(Value::Nil)
        });

        let _ = on_message(vec![Value::NativeClosure(NativeClosureType(handler))]);

        // Connect client
        let client_val = connect_client(&format!("ws://127.0.0.1:{}", port)).expect("Failed to connect client");
        let client_obj = match &client_val {
            Value::Object(m) => m,
            _ => panic!("Expected Object for client"),
        };

        let send_fn = match client_obj.get("send").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected send closure"),
        };

        let recv_fn = match client_obj.get("recv").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected recv closure"),
        };

        let _ = send_fn(vec![Value::String("hello world".to_string())]).unwrap();
        let reply = recv_fn(vec![]).unwrap();
        match reply {
            Value::String(s) => assert_eq!(s, "echo: hello world"),
            other => panic!("Expected String, got {:?}", other),
        }

        // Close client and server
        let client_close = match client_obj.get("close").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected close closure"),
        };
        let _ = client_close(vec![]);

        let server_close = match server_obj.get("close").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected close closure"),
        };
        let _ = server_close(vec![]);
    }

    #[test]
    fn test_ws_binary_transfer() {
        let server_val = listen_server("127.0.0.1:0").expect("Failed to bind server");
        let server_obj = match &server_val {
            Value::Object(m) => m,
            _ => panic!("Expected Object for server"),
        };
        let port = match server_obj.get("port").unwrap() {
            Value::Int(p) => *p,
            _ => panic!("Expected Int port"),
        };

        let on_binary = match server_obj.get("onBinary").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected onBinary closure"),
        };

        let bin_handler = Arc::new(|args: Vec<Value>| {
            if args.len() >= 2 {
                let client = &args[0];
                let bytes = &args[1];
                if let Value::Object(c_map) = client {
                    if let Some(Value::NativeClosure(NativeClosureType(send_bytes))) = c_map.get("sendBytes") {
                        let _ = send_bytes(vec![bytes.clone()]);
                    }
                }
            }
            Ok(Value::Nil)
        });

        let _ = on_binary(vec![Value::NativeClosure(NativeClosureType(bin_handler))]);

        let client_val = connect_client(&format!("ws://127.0.0.1:{}", port)).expect("Failed to connect client");
        let client_obj = match &client_val {
            Value::Object(m) => m,
            _ => panic!("Expected Object for client"),
        };

        let send_bytes_fn = match client_obj.get("sendBytes").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected sendBytes closure"),
        };
        let recv_bytes_fn = match client_obj.get("recvBytes").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected recvBytes closure"),
        };

        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF, 42, 99];
        let _ = send_bytes_fn(vec![Value::Bytes(payload.clone())]).unwrap();
        let received = recv_bytes_fn(vec![]).unwrap();
        match received {
            Value::Bytes(b) => assert_eq!(b, payload),
            other => panic!("Expected Bytes, got {:?}", other),
        }

        let client_close = match client_obj.get("close").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected close closure"),
        };
        let _ = client_close(vec![]);

        let server_close = match server_obj.get("close").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected close closure"),
        };
        let _ = server_close(vec![]);
    }

    #[test]
    fn test_ws_broadcast_and_connections() {
        let server_val = listen_server("127.0.0.1:0").expect("Failed to bind server");
        let server_obj = match &server_val {
            Value::Object(m) => m,
            _ => panic!("Expected Object for server"),
        };
        let port = match server_obj.get("port").unwrap() {
            Value::Int(p) => *p,
            _ => panic!("Expected Int port"),
        };

        let client1 = connect_client(&format!("ws://127.0.0.1:{}", port)).expect("Client 1 failed");
        let client2 = connect_client(&format!("ws://127.0.0.1:{}", port)).expect("Client 2 failed");

        std::thread::sleep(std::time::Duration::from_millis(50));

        let conn_fn = match server_obj.get("connections").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected connections closure"),
        };
        let count = conn_fn(vec![]).unwrap();
        match count {
            Value::Int(c) => assert_eq!(c, 2),
            other => panic!("Expected Int(2), got {:?}", other),
        }

        let bcast_fn = match server_obj.get("broadcast").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!("Expected broadcast closure"),
        };
        let _ = bcast_fn(vec![Value::String("ALL_CALL".to_string())]).unwrap();

        let recv1 = match client1 {
            Value::Object(ref m) => match m.get("recv").unwrap() {
                Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
                _ => panic!(),
            },
            _ => panic!(),
        };
        let recv2 = match client2 {
            Value::Object(ref m) => match m.get("recv").unwrap() {
                Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
                _ => panic!(),
            },
            _ => panic!(),
        };

        match recv1(vec![]).unwrap() {
            Value::String(s) => assert_eq!(s, "ALL_CALL"),
            other => panic!("Expected String, got {:?}", other),
        }
        match recv2(vec![]).unwrap() {
            Value::String(s) => assert_eq!(s, "ALL_CALL"),
            other => panic!("Expected String, got {:?}", other),
        }

        let server_close = match server_obj.get("close").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!(),
        };
        let _ = server_close(vec![]);

        let client1_close = match client1 {
            Value::Object(ref m) => match m.get("close").unwrap() {
                Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
                _ => panic!(),
            },
            _ => panic!(),
        };
        let _ = client1_close(vec![]);

        let client2_close = match client2 {
            Value::Object(ref m) => match m.get("close").unwrap() {
                Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
                _ => panic!(),
            },
            _ => panic!(),
        };
        let _ = client2_close(vec![]);
    }

    #[test]
    fn test_ws_channel_bridging() {
        let server_val = listen_server("127.0.0.1:0").expect("Failed to bind server");
        let server_obj = match &server_val {
            Value::Object(m) => m,
            _ => panic!("Expected Object for server"),
        };
        let port = match server_obj.get("port").unwrap() {
            Value::Int(p) => *p,
            _ => panic!("Expected Int port"),
        };

        let client_val = connect_client(&format!("ws://127.0.0.1:{}", port)).expect("Client failed");
        let client_obj = match &client_val {
            Value::Object(m) => m,
            _ => panic!(),
        };

        let messages_fn = match client_obj.get("messages").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!(),
        };

        let stream_val = messages_fn(vec![]).unwrap();
        let stream_obj = match stream_val {
            Value::Object(m) => m,
            _ => panic!(),
        };

        let to_channel_fn = match stream_obj.get("toChannel").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!(),
        };

        let chan_tuple = to_channel_fn(vec![]).unwrap();
        let rx_id = match chan_tuple {
            Value::Tuple(items) => match items[1] {
                Value::Receiver(id) => id,
                _ => panic!(),
            },
            _ => panic!(),
        };

        let bcast_fn = match server_obj.get("broadcast").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!(),
        };
        let _ = bcast_fn(vec![Value::String("channel_data".to_string())]);

        let rx_arc = crate::vm::get_receivers().lock().unwrap().get(&rx_id).unwrap().clone();
        let val = rx_arc.lock().unwrap().recv().unwrap();
        match val {
            Value::String(s) => assert_eq!(s, "channel_data"),
            other => panic!("Expected String, got {:?}", other),
        }

        let client_close = match client_obj.get("close").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!(),
        };
        let _ = client_close(vec![]);

        let server_close = match server_obj.get("close").unwrap() {
            Value::NativeClosure(NativeClosureType(cb)) => cb.clone(),
            _ => panic!(),
        };
        let _ = server_close(vec![]);
    }

}

