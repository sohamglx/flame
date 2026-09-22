use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

pub struct WebServerConfig {
    pub dist_dir: PathBuf,
    pub port: u16,
    pub routes: Vec<(String, String)>, // (path, title)
    pub hot_reload: bool,
}

fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn handle_client(mut stream: TcpStream, dist_dir: &Path, hot_reload: bool) {
    let mut buffer = [0u8; 4096];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) => n,
        Err(_) => return,
    };

    if bytes_read == 0 {
        return;
    }

    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let first_line = request_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    if parts.len() < 2 || parts[0] != "GET" {
        let response = "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n";
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    let mut raw_path = parts[1];
    if let Some(pos) = raw_path.find('?') {
        raw_path = &raw_path[..pos];
    }
    if let Some(pos) = raw_path.find('#') {
        raw_path = &raw_path[..pos];
    }

    if raw_path == "/__flame_live_reload__" {
        let app_js = dist_dir.join("app.js");
        let mtime_str = if let Ok(m) = fs::metadata(&app_js) {
            if let Ok(t) = m.modified() {
                format!("{:?}", t)
            } else {
                "0".to_string()
            }
        } else {
            "0".to_string()
        };
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n{}",
            mtime_str.len(),
            mtime_str
        );
        let _ = stream.write_all(response.as_bytes());
        return;
    }

    let clean_path = raw_path.trim_start_matches('/');
    let target_file = if clean_path.is_empty() {
        dist_dir.join("index.html")
    } else {
        dist_dir.join(clean_path)
    };

    let is_index_request = clean_path.is_empty() || clean_path == "index.html";

    let (mut content, mime, status_line) = if target_file.is_file() {
        match fs::read(&target_file) {
            Ok(data) => (data, get_mime_type(&target_file), "HTTP/1.1 200 OK"),
            Err(_) => (b"Not Found".to_vec(), "text/plain", "HTTP/1.1 404 Not Found"),
        }
    } else {
        // SPA Fallback: serve index.html for client-side routing
        let index_file = dist_dir.join("index.html");
        match fs::read(&index_file) {
            Ok(data) => (data, "text/html; charset=utf-8", "HTTP/1.1 200 OK"),
            Err(_) => (b"Not Found".to_vec(), "text/plain", "HTTP/1.1 404 Not Found"),
        }
    };

    if hot_reload && (is_index_request || !target_file.is_file()) {
        if let Ok(html_str) = String::from_utf8(content.clone()) {
            let reload_script = r#"<script>
(function() {
  let lastMtime = null;
  setInterval(async () => {
    try {
      const res = await fetch('/__flame_live_reload__');
      if (res.ok) {
        const text = await res.text();
        if (lastMtime !== null && lastMtime !== text) {
          console.log('[Flame Hot-Reload] Change detected, reloading...');
          location.reload();
        }
        lastMtime = text;
      }
    } catch (e) {}
  }, 300);
})();
</script>
</body>"#;
            let modified_html = html_str.replace("</body>", reload_script);
            content = modified_html.into_bytes();
        }
    }

    let response_header = format!(
        "{}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Cache-Control: no-cache\r\n\
         Connection: close\r\n\r\n",
        status_line,
        mime,
        content.len()
    );

    let _ = stream.write_all(response_header.as_bytes());
    let _ = stream.write_all(&content);
    let _ = stream.flush();
}

pub fn serve_dist(config: WebServerConfig) -> Result<(), std::io::Error> {
    let bind_addr = format!("0.0.0.0:{}", config.port);
    let listener = match TcpListener::bind(&bind_addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("\x1b[1;31merror:\x1b[0m Failed to bind web server to {}: {}", bind_addr, e);
            return Err(e);
        }
    };

    println!();
    println!("\x1b[1;38;2;249;115;22m  ⚡ Flame Web Application Server\x1b[0m");
    println!("  ---------------------------------------------");
    println!("  \x1b[1;32m➜\x1b[0m  Local:    \x1b[1;36mhttp://localhost:{}/\x1b[0m", config.port);
    println!("  \x1b[1;32m➜\x1b[0m  Network:  \x1b[1;36mhttp://0.0.0.0:{}/\x1b[0m", config.port);
    if config.hot_reload {
        println!("  \x1b[1;32m➜\x1b[0m  Mode:     \x1b[1;35mHot Reload Active\x1b[0m");
    }
    if !config.routes.is_empty() {
        println!("  \x1b[1;32m➜\x1b[0m  Routes:");
        for (r, title) in &config.routes {
            if title.is_empty() {
                println!("     • \x1b[1m{}\x1b[0m", r);
            } else {
                println!("     • \x1b[1m{}\x1b[0m ({})", r, title);
            }
        }
    }
    println!("  ---------------------------------------------");
    println!("  \x1b[90mPress Ctrl+C to stop the server\x1b[0m\n");

    let dist_arc = Arc::new(config.dist_dir);
    let hot_reload = config.hot_reload;

    for stream_res in listener.incoming() {
        match stream_res {
            Ok(stream) => {
                let dist_clone = Arc::clone(&dist_arc);
                thread::spawn(move || {
                    handle_client(stream, &dist_clone, hot_reload);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }

    Ok(())
}
