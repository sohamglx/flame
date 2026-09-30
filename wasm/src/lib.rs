use flamelang::lexer::{Lexer, TokenKind};
use flamelang::parser::Parser;
use flamelang::runner::Runner;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Serialize)]
struct EvalResponse {
    success: bool,
    output: String,
    result: Option<String>,
    error: Option<String>,
}

#[unsafe(no_mangle)]
pub extern "C" fn flame_alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flame_free(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        unsafe {
            drop(Vec::from_raw_parts(ptr, 0, size));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flame_eval(ptr: *const u8, len: usize) -> *mut u8 {
    let source = match unsafe { std::str::from_utf8(std::slice::from_raw_parts(ptr, len)) } {
        Ok(s) => s,
        Err(e) => {
            return create_response(&EvalResponse {
                success: false,
                output: String::new(),
                result: None,
                error: Some(format!("UTF-8 decode error: {}", e)),
            });
        }
    };

    let mut runner = Runner::new(PathBuf::from("playground.fm"));
    let capture_buf = Arc::new(Mutex::new(Vec::new()));
    runner.output_capture = Some(capture_buf.clone());

    // Tokenize
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        let is_eof = tok.kind == TokenKind::EOF;
        tokens.push(tok);
        if is_eof {
            break;
        }
    }

    // Parse
    let mut parser = Parser::new(tokens, "playground.fm".to_string());
    let stmts = match parser.parse() {
        Ok(s) => s,
        Err(e) => {
            let logs = capture_buf.lock().map(|g| g.join("\n")).unwrap_or_default();
            return create_response(&EvalResponse {
                success: false,
                output: logs,
                result: None,
                error: Some(format!("Parse error [line {}, col {}]: {}", e.span.line, e.span.col, e.message)),
            });
        }
    };

    // Run
    match runner.run(&stmts) {
        Ok(val) => {
            let logs = capture_buf.lock().map(|g| g.join("\n")).unwrap_or_default();
            let res_str = match val {
                flamelang::vm::Value::Nil => None,
                _ => Some(flamelang::native_std::fmt::stringify_value(&val)),
            };
            create_response(&EvalResponse {
                success: true,
                output: logs,
                result: res_str,
                error: None,
            })
        }
        Err(e) => {
            let logs = capture_buf.lock().map(|g| g.join("\n")).unwrap_or_default();
            create_response(&EvalResponse {
                success: false,
                output: logs,
                result: None,
                error: Some(format!("Runtime error: {}", e)),
            })
        }
    }
}

fn create_response(resp: &EvalResponse) -> *mut u8 {
    let json = serde_json::to_string(resp).unwrap_or_else(|_| "{\"success\":false,\"output\":\"\",\"result\":null,\"error\":\"Serialization error\"}".to_string());
    let bytes = json.as_bytes();
    let total_len = 4 + bytes.len();
    let mut out = Vec::with_capacity(total_len);
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
    let ptr = out.as_mut_ptr();
    std::mem::forget(out);
    ptr
}
