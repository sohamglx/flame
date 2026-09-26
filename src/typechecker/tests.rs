use super::checker::TypeChecker;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn check_source(src: &str) -> Result<(), Vec<crate::diagnostics::Diagnostic>> {
        let mut lexer = Lexer::new(src);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == crate::lexer::TokenKind::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = Parser::new(tokens, "test.fm".to_string());
        let stmts = parser.parse().expect("Failed to parse");
        TypeChecker::new("test.fm".to_string()).check_program(&stmts).0
    }

    #[test]
    fn test_duplicate_enum_no_platform_fails() {
        let src = r#"
        enum Status {
            Active,
            Inactive,
        }

        enum Status {
            Pending,
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Duplicate enum without @Platform should fail");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("Duplicate enum definition: 'Status'")));
    }

    #[test]
    fn test_duplicate_struct_no_platform_fails() {
        let src = r#"
        struct User {
            name: String,
        }

        struct User {
            age: Int,
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Duplicate struct without @Platform should fail");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("Duplicate struct definition: 'User'")));
    }

    #[test]
    fn test_duplicate_enum_same_platform_fails() {
        let src = r#"
        @Platform("windows")
        enum Status {
            Active,
        }

        @Platform("windows")
        enum Status {
            Pending,
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Duplicate enum with same @Platform should fail");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("Duplicate enum definition: 'Status'")));
    }

    #[test]
    fn test_duplicate_struct_same_platform_fails() {
        let src = r#"
        @Platform("linux")
        struct Config {
            path: String,
        }

        @Platform("linux")
        struct Config {
            retries: Int,
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Duplicate struct with same @Platform should fail");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("Duplicate struct definition: 'Config'")));
    }

    #[test]
    fn test_duplicate_enum_different_platform_allowed() {
        let src = r#"
        @Platform("windows")
        enum Status {
            WinActive,
        }

        @Platform("linux")
        enum Status {
            LinActive,
        }
        "#;
        let res = check_source(src);
        assert!(res.is_ok(), "Duplicate enum with different @Platform should be allowed: {:?}", res.err());
    }

    #[test]
    fn test_duplicate_struct_different_platform_allowed() {
        let src = r#"
        @Platform("windows")
        struct Config {
            win_path: String,
        }

        @Platform("linux")
        struct Config {
            lin_path: String,
        }
        "#;
        let res = check_source(src);
        assert!(res.is_ok(), "Duplicate struct with different @Platform should be allowed: {:?}", res.err());
    }

    #[test]
    fn test_struct_enum_name_collision_fails() {
        let src = r#"
        struct Entry {
            id: Int,
        }

        enum Entry {
            File,
            Dir,
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Struct and enum with same name should collide");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("Duplicate type definition: 'Entry'")));
    }

    #[test]
    fn test_closure_parameter_types_and_calls() {
        let src = r#"
        fn onEach(callback: (msg: Unknown) -> Nil) {}
        fn onBinary(callback: (client: String, bytes: Byte)) {}

        onEach((m) {
            let x = m
        })

        onBinary((c, b) {
            let s = c
        })
        "#;
        let res = check_source(src);
        assert!(res.is_ok(), "Closure types and call compatibility should succeed: {:?}", res.err());
    }

    #[test]
    fn test_annotation_context_outside_annotation_decl_fails() {
        let src = r#"
        fn bad() {
            let ctx = annotation.context()
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "annotation.context() outside annotation decl must fail typecheck");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("annotation.context() can only be called inside of a custom annotation")));
    }

    #[test]
    fn test_annotation_context_inside_annotation_decl_allowed() {
        let src = r#"
        annotation Route(path: String) {
            let ctx = annotation.context()
            let handler = ctx.target.ref()
        }
        "#;
        let res = check_source(src);
        assert!(res.is_ok(), "AnnotationContext inside annotation declaration should be valid: {:?}", res.err());
    }

