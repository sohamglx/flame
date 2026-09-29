use super::core::Runner;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::vm::*;
use std::path::PathBuf;

fn run_flame(code: &str) -> Result<Value, String> {
        let mut lexer = Lexer::new(code);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            if tok.kind == crate::lexer::TokenKind::EOF {
                tokens.push(tok);
                break;
            }
            tokens.push(tok);
        }

        let mut parser = Parser::new(tokens, "test.flame".to_string());
        let stmts = parser.parse().map_err(|diag| diag.message)?;
        let mut runner = Runner::new(PathBuf::from("test.flame"));
        runner.run(&stmts)
    }

    #[test]
    fn mut_ref_through_enum_field() {
        let code = r#"fn change(&mut name: String) {
    print("in change, before:", name)
    name = "core"
    print("in change, after:", name)
}

enum Config {
    modules(Formula)
}

fn main() {
    let mut f: Formula = formula {
        name: "std",
        v: "1.0.0",
        description: "std lib of flame"
    }

    let mut con: Config = Config.modules(f)

    change(&mut con.name)
    print("final:", con.name)
}
main()"#;

        run_flame(code).unwrap();
    }

    #[test]
    fn std_thread_execution() {
        let code = r#"import std.thread

fn main() {
    let (tx, rx) = thread.channel()
    tx.send("test_message")
    rx.recv()
}
main()"#;

        let result = run_flame(code).unwrap();
        assert_eq!(result.to_string(), "test_message");
    }

    #[test]
    fn annotation_decl_and_stripping_test() {
        let code = r#"
annotation Benchmark(name: String) -> Formula {
    return formula { name: name }
}

@Test
fn test_my_func() {
    return 42
}

fn main() -> i64 {
    return 100
}
main()
"#;
        let result = run_flame(code).unwrap();
        assert_eq!(result.to_string(), "100");
    }

    #[test]
    fn let_decl_annotation_executes() {
        let code = r#"
annotation Entity(table: String) -> String {
    print("Registering entity")
    return table
}

@Entity(table: "users")
let User = formula {
    id: 9
    name: "9"
}
"#;
        let mut lexer = Lexer::new(code);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            if tok.kind == crate::lexer::TokenKind::EOF {
                tokens.push(tok);
                break;
            }
            tokens.push(tok);
        }

        let mut parser = Parser::new(tokens, "test.flame".to_string());
        let stmts = parser.parse().map_err(|diag| diag.message).unwrap();
        let mut runner = Runner::new(PathBuf::from("test.flame"));
        let result = runner.run(&stmts).unwrap();
        assert_eq!(result.to_string(), "nil");
    }

    #[test]
    fn explicit_type_conversion_methods_test() {
        let code = r#"
fn main() {
    let num_str = "42"
    let val = num_str.toInt()
    let hex = "1A".toInt(16)
    let flt = "3.14159".toFloat()
    let prec = 3.14159.toString(2)
    let bool_val = "true".toBool()
    return val + hex
}
main()
"#;
        let result = run_flame(code).unwrap();
        assert_eq!(result.to_string(), "68"); // 42 + 26 = 68
    }

    #[test]
    fn custom_annotation_logger_test() {
        let code = r#"
export annotation Logger(prefix: String) -> String {
    print($"[LOGGER INIT] Prefix configured: {prefix}")
    prefix
}

@Logger(prefix: "flame-cli")
fn main() {
    print("Inside main")
}
"#;
        let mut lexer = Lexer::new(code);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            if tok.kind == crate::lexer::TokenKind::EOF {
                tokens.push(tok);
                break;
            }
            tokens.push(tok);
        }
        let mut parser = Parser::new(tokens, "test.flame".to_string());
        let stmts = parser.parse().map_err(|diag| diag.message).unwrap();
        let mut runner = Runner::new(PathBuf::from("test.flame"));
        let result = runner.run(&stmts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_imports_toml_fmi_json_relative() {
        let temp_dir = std::env::temp_dir().join(format!("flame_test_res_{}", std::process::id()));
        let sub_dir = temp_dir.join("sub");
        std::fs::create_dir_all(&sub_dir).unwrap();

        let toml_path = temp_dir.join("flame.toml");
        std::fs::write(&toml_path, r#"
[package]
name = "my_app"
version = "1.2.3"
tags = ["fast", "typed"]
"#).unwrap();

        let fmi_path = temp_dir.join("meta.fmi");
        std::fs::write(&fmi_path, r#"
{
    "schema": "fmi-v1",
    "id": 101,
    "items": ["alpha", "beta"]
}
"#).unwrap();

        let arr_json_path = temp_dir.join("users.json");
        std::fs::write(&arr_json_path, r#"
[
    {"id": 1, "name": "Alice"},
    {"id": 2, "name": "Bob"}
]
"#).unwrap();

        let script_file = sub_dir.join("main.fm");
        let script = r#"
import "../flame.toml" as config
import "../meta.fmi" as meta
import "../users.json" as users

fn main() {
    let pkg_name = config.package.name
    let first_tag = config.package.tags[0]
    let meta_schema = meta.schema
    let first_item = meta.items[0]
    let first_user_name = users[0].name
    let second_user_id = users[1]["id"]
    return $"{pkg_name}|{first_tag}|{meta_schema}|{first_item}|{first_user_name}|{second_user_id}"
}
main()
"#;
        let mut lexer = Lexer::new(script);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            if tok.kind == crate::lexer::TokenKind::EOF {
                tokens.push(tok);
                break;
            }
            tokens.push(tok);
        }
        let mut parser = Parser::new(tokens, script_file.to_str().unwrap().to_string());
        let stmts = parser.parse().map_err(|diag| diag.message).unwrap();
        let mut runner = Runner::new(script_file.clone());
        let result = runner.run(&stmts).unwrap();
        assert_eq!(result.to_string(), "my_app|fast|fmi-v1|alpha|Alice|2");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resource_imports_dot_slash() {
        let temp_dir = std::env::temp_dir().join(format!("flame_test_dot_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let cfg_path = temp_dir.join("local.toml");
        std::fs::write(&cfg_path, r#"
title = "Local Demo"
count = 42
"#).unwrap();

        let script_file = temp_dir.join("main.fm");
        let script = r#"
import "./local.toml" as local

fn main() {
    return $"{local.title}:{local.count}"
}
main()
"#;
        let mut lexer = Lexer::new(script);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            if tok.kind == crate::lexer::TokenKind::EOF {
                tokens.push(tok);
                break;
            }
            tokens.push(tok);
        }
        let mut parser = Parser::new(tokens, script_file.to_str().unwrap().to_string());
        let stmts = parser.parse().map_err(|diag| diag.message).unwrap();
        let mut runner = Runner::new(script_file.clone());
        let result = runner.run(&stmts).unwrap();
        assert_eq!(result.to_string(), "Local Demo:42");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_string_interpolation_escapes() {
        let code = r#"
fn main() {
    let name = "Soham"
    let s1 = $"Hello {name}"
    let s2 = $"JSON: \{{ \"name\": \"Soham\" \}}"
    let s3 = $"JSON: \{ \"name\": \"Soham\" \}"
    let s4 = $"literal {{"
    let s5 = $"literal }}"
    let x = 10
    let s6 = $"value = {x}"
    return $"{s1}|{s2}|{s3}|{s4}|{s5}|{s6}"
}
main()
"#;
        let result = run_flame(code).unwrap();
        assert_eq!(
            result.to_string(),
            r#"Hello Soham|JSON: { "name": "Soham" }|JSON: { "name": "Soham" }|literal {|literal }|value = 10"#
        );
    }

    #[test]
    fn test_multiline_string_interpolation_escapes() {
        let code = r#"
fn main() {
    let x = 42
    let s = $"""
        \{{ "value": {x} \}}
        literal {{ and }}
    """
    return s
}
main()
"#;
        let result = run_flame(code).unwrap();
        assert!(result.to_string().contains(r#"{ "value": 42 }"#));
        assert!(result.to_string().contains("literal { and }"));
    }

    #[test]
    fn test_interpolation_nested_if_expr() {
        let code = r#"
fn main() {
    let x = 5
    let msg = $"result = { if x > 0 { 100 } else { 200 } }"
    return msg
}
main()
"#;
        let result = run_flame(code).unwrap();
        assert_eq!(result.to_string(), "result = 100");
    }

    #[test]
    fn test_if_expression_execution() {
        let code = r#"
fn main() {
    let b = 0
    let a = if b == 0 { 10 } else { 20 }
    let multi = if b == 0 {
        let x = 5
        let y = 15
        x + y
    } else {
        0
    }
    let chained = if b == 1 {
        100
    } else if b == 0 {
        200
    } else {
        300
    }
    let connected = true
    let status = if connected {
        "connected"
    } else {
        "offline"
    }
    return $"{a}|{multi}|{chained}|{status}"
}
main()
"#;
        let result = run_flame(code).unwrap();
        assert_eq!(result.to_string(), "10|20|200|connected");
    }



    