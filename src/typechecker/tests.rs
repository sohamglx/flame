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

    #[test]
    fn test_if_expr_compatible_types_ok() {
        let src = r#"
        fn main() {
            let b = 0
            let a = if b == 0 { 10 } else { 20 }
            let status = if b == 0 { "connected" } else { "offline" }
            let chained = if b == 0 { 10 } else if b == 1 { 20 } else { 30 }
        }
        "#;
        let res = check_source(src);
        assert!(res.is_ok(), "Compatible if expression branches should pass typechecking: {:?}", res.err());
    }

    #[test]
    fn test_if_expr_incompatible_branch_types_fails() {
        let src = r#"
        fn main() {
            let b = 0
            let a = if b == 0 { 10 } else { "hello" }
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Incompatible branch types should fail typechecking");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("incompatible branch types in if expression: expected 'Int', found 'String'")));
    }

    #[test]
    fn test_if_expr_missing_else_branch_fails() {
        let src = r#"
        fn main() {
            let b = 0
            let a = if b == 0 { 10 }
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "If expression missing else branch should fail typechecking");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("if expression missing 'else' branch")));
    }

    #[test]
    fn test_incomplete_trait_implementation_fails() {
        let src = r#"
        trait Drawable {
            fn draw(&self) -> String
            fn area(&self) -> Float
        }

        struct Circle {
            radius: Float
        }

        impl Circle: Drawable {
            fn draw(&self) -> String {
                return "Circle"
            }
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Incomplete trait implementation should fail");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("not all trait items implemented, missing: `area`")));
    }

    #[test]
    fn test_trait_default_method_not_missing() {
        let src = r#"
        trait Describable {
            fn name(&self) -> String
            default fn describe(&self) -> String {
                return "default description"
            }
        }

        struct Item {
            title: String
        }

        impl Item: Describable {
            fn name(&self) -> String {
                return self.title
            }
        }
        "#;
        let res = check_source(src);
        assert!(res.is_ok(), "Trait implementation with default method should pass without implementing default: {:?}", res.err());
    }

    #[test]
    fn test_trait_composition_missing_super_method_fails() {
        let src = r#"
        trait A {
            fn foo(&self)
        }
        trait B {
            fn bar(&self)
        }
        trait Composed = A and B

        struct Foo {}

        impl Foo: Composed {
            fn foo(&self) {}
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Missing method from composed trait should fail");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("not all trait items implemented, missing: `bar`")));
    }

    #[test]
    fn test_trait_strictness_detached_impl_fails() {
        let src = r#"
        trait Greetable {
            fn greet(&self) -> String
        }

        struct Person {
            name: String
        }

        impl Person {
            fn greet(&self) -> String {
                return "Hello " + self.name
            }
        }

        impl Person: Greetable {
            // greet is in another detached impl block, which is not allowed under strict trait impl rules!
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Detached impl should not satisfy trait requirements in separate impl block");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("not all trait items implemented, missing: `greet`")));
    }

    #[test]
    fn test_concrete_type_generic_bound_warns_and_does_not_fail() {
        let src = r#"
        fn process<T: String>(val: T) -> T {
            return val
        }

        struct Box<T: Int> {
            val: T
        }
        "#;
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
        let (res, tc) = TypeChecker::new("test.fm".to_string()).check_program(&stmts);
        assert!(res.is_ok(), "Concrete type bounds should produce warnings, NOT block compilation: {:?}", res.err());
        assert_eq!(tc.diagnostics.len(), 2, "Expected 2 warnings for String and Int bounds");
        for d in &tc.diagnostics {
            assert_eq!(d.severity, crate::diagnostics::DiagnosticSeverity::Warning);
            assert!(d.message.contains("bounds on generic parameters are traits, not concrete types"));
            assert!(d.note.as_ref().unwrap().contains("has no effect"));
        }
    }

    #[test]
    fn test_trait_generic_bound_does_not_warn() {
        let src = r#"
        trait Printable {
            fn print(&self)
        }

        fn display<T: Printable>(val: T) {
            val.print()
        }
        "#;
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
        let (res, tc) = TypeChecker::new("test.fm".to_string()).check_program(&stmts);
        assert!(res.is_ok());
        assert!(tc.diagnostics.is_empty(), "Actual trait bound should not produce any warnings: {:?}", tc.diagnostics);
    }

    #[test]
    fn test_struct_init_missing_field_fails() {
        let src = r#"
        struct User {
            name: String,
            age: Int
        }

        fn test() {
            let u = User { name: "Alice" }
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Struct init with missing field must fail");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("missing field 'age' for struct 'User'")));
        assert!(diags.iter().any(|d| d.message.contains("not all struct fields initialized, missing: `age`")));
    }

    #[test]
    fn test_struct_constructor_missing_field_fails() {
        let src = r#"
        struct User {
            name: String,
            age: Int
        }

        fn test() {
            let u = User(name: "Alice")
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Struct constructor call with missing field must fail");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("missing field 'age' for struct 'User'")));
    }

    #[test]
    fn test_enum_variant_missing_field_fails() {
        let src = r#"
        enum Message {
            Move { x: Int, y: Int }
        }

        fn test() {
            let m = Message.Move { x: 10 }
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Enum variant with missing struct field must fail");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("missing field 'y' for variant 'Message.Move'")));
        assert!(diags.iter().any(|d| d.message.contains("not all variant fields initialized, missing: `y`")));
    }

    #[test]
    fn test_borrow_checker_mutable_and_immutable_borrow_fails() {
        let src = r#"
        fn test() {
            let mut x = 10
            let a = &x
            let b = &mut x
            println(a)
            println(b)
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Mutable borrow while immutably borrowed must fail borrow checker");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("cannot borrow 'x' as mutable because it is also borrowed as immutable")));
    }

    #[test]
    fn test_borrow_checker_two_mutable_borrows_fails() {
        let src = r#"
        fn test() {
            let mut x = 10
            let a = &mut x
            let b = &mut x
            a += 1
            b += 1
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Two active mutable borrows must fail borrow checker");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("cannot borrow 'x' as mutable more than once at a time")));
    }

    #[test]
    fn test_borrow_checker_return_reference_to_local_variable_fails() {
        let src = r#"
        fn test() -> &Int {
            let x = 42
            return &x
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Returning reference to local variable must fail borrow checker");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("cannot return reference to local variable 'x'")));
    }

    #[test]
    fn test_borrow_checker_use_after_move_fails() {
        let src = r#"
        fn test() {
            let a = "hello"
            let b = a
            println(a)
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Use after move must fail borrow checker");
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("use of moved value: 'a'")));
    }

    #[test]
    fn test_expect_panic_suppresses_code_editor_errors() {
        let src = r#"
        @ExpectPanic
        fn test_bypass() {
            let a = "hello"
            let b = a
            println(a)
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_ok(),
            "@ExpectPanic must suppress static compile errors in the editor so test harness catches it at runtime"
        );
    }

    #[test]
    fn test_repro_primitive_copy_false_positive() {
        let src = r#"
        fn test_prim() {
            let a = 42
            let b = a
            println(a)
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_ok(),
            "Primitive Int must have Copy semantics and not be moved: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_repro_branch_move_false_positive() {
        let src = r#"
        fn test_branch(cond: Bool) {
            let a = "hello"
            if cond {
                let b = a
            } else {
                println(a)
            }
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_ok(),
            "Moving in then branch must not invalidate variable in else branch: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_repro_while_loop_backedge_false_negative() {
        let src = r#"
        fn test_loop() {
            let a = "hello"
            let mut i = 0
            while i < 10 {
                let b = a
                i += 1
            }
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "Moving a variable in a loop must fail compile-time borrow checking"
        );
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("cannot move out of 'a' in a loop")));
    }

    #[test]
    fn test_repro_nested_place_overlap_false_negative() {
        let src = r#"
        struct Inner {
            val: String,
        }
        struct Outer {
            inner: Inner,
        }
        fn test_nested() {
            let mut o = Outer { inner: Inner { val: "hello" } }
            let r1 = &mut o.inner.val
            let r2 = &mut o.inner.val
            println(r1)
            println(r2)
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "Nested place expression must be detected for conflicting mutable borrows"
        );
        let diags = res.unwrap_err();
        assert!(diags.iter().any(|d| d.message.contains("cannot borrow 'o.inner.val' as mutable more than once")));
    }

    #[test]
    fn test_repro_disjoint_fields_false_positive() {
        let src = r#"
        struct User {
            first_name: String,
            last_name: String,
        }
        fn test_disjoint() {
            let mut u = User { first_name: "John", last_name: "Doe" }
            let r1 = &mut u.first_name
            let r2 = &mut u.last_name
            println(r1)
            println(r2)
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_ok(),
            "Disjoint struct fields must be borrowable simultaneously: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_docs_annotation_empty_body_with_return_type_fails() {
        let src = r#"
        @Docs("Closes this window.")
        fn close() -> Bool {}
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "@Docs annotated function with empty body and return type must fail with missing return: {:?}",
            res.ok()
        );
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("has an empty body and does not return a value")),
            "Expected empty body diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_docs_annotation_declaration_with_semicolon_succeeds() {
        let src = r#"
        @Docs("Closes this window.")
        fn close() -> Bool;
        "#;
        let res = check_source(src);
        assert!(
            res.is_ok(),
            "@Docs annotated function declaration with semicolon should succeed: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_function_declaration_without_docs_with_semicolon_succeeds() {
        let src = r#"
        fn calculate(a: Int, b: Int) -> Int;
        "#;
        let res = check_source(src);
        assert!(
            res.is_ok(),
            "Function declaration with semicolon should succeed: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_docs_with_valid_body_succeeds() {
        let src = r#"
        @Docs("Closes this window.")
        fn close() -> Bool {
            return true
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_ok(),
            "@Docs function with valid return must succeed: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_test_annotation_semantic_validation() {
        // Valid @Test passes typechecking
        let ok_src = r#"
        @Test
        fn testSomething() -> Bool {
            return true
        }
        "#;
        assert!(check_source(ok_src).is_ok());

        // Invalid @Test missing return must fail semantic validation
        let bad_src = r#"
        @Test
        fn testSomething() -> Bool {}
        "#;
        assert!(check_source(bad_src).is_err());
    }

    #[test]
    fn test_docs_annotation_non_empty_body_missing_return_fails() {
        let src = r#"
        @Docs("Closes this window.")
        fn close() -> Bool {
            let a = 7
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "@Docs annotated function with non-empty body and missing return must fail"
        );
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("missing a return value of type 'Bool'")),
            "Expected missing return value diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_docs_annotation_with_mismatched_return_fails() {
        let src = r#"
        @Docs("Closes this window.")
        fn close() -> Bool {
            "str"
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "@Docs annotated function with mismatched return type must fail"
        );
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("expected Bool, found String")),
            "Expected type mismatch diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_simple_function_empty_body_with_return_type_fails() {
        let src = r#"
        fn close() -> Bool {}
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "Simple function with return type and empty body must fail"
        );
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("has an empty body and does not return a value")),
            "Expected empty body diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_simple_function_missing_return_value_fails() {
        let src = r#"
        fn close() -> Bool {
            let x = 10
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "Simple function with return type but missing return must fail"
        );
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("is missing a return value of type 'Bool'")),
            "Expected missing return diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_simple_function_mismatched_return_type_fails() {
        let src = r#"
        fn close() -> Bool {
            "str"
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "Simple function returning wrong type must fail"
        );
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("expected Bool, found String")),
            "Expected type mismatch diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_simple_function_no_return_type_returning_value_fails() {
        let src = r#"
        fn close() {
            "str"
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "Function without return type mentioned returning a value must fail"
        );
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("expected Nil, found String")),
            "Expected expected Nil diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_simple_function_correct_return_succeeds() {
        let src = r#"
        fn add(a: Int, b: Int) -> Int {
            a + b
        }
        fn greet() {
            let msg = "hello"
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_ok(),
            "Correct return types and void functions must succeed: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_void_function_with_await_and_calls_succeeds() {
        let src = r#"
        async fn listen() -> Int {
            42
        }

        async fn main() {
            await listen()
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_ok(),
            "Awaiting inside a void function must be treated as a statement, not return value mismatch: {:?}",
            res.err()
        );
    }

    #[test]
    fn test_parse_type_name_rust_path() {
        let tc = TypeChecker::new("test.fm".to_string());
        let ty = tc.parse_type_name("std::io::Result<()>");
        assert_eq!(tc.format_type(&ty), "std::io::Result<()>");
    }


    #[test]
    fn test_impl_method_with_docs_and_body_checks_return_type() {
        let src = r#"
        struct Window {
            id: Int,
        }

        impl Window {
            @Docs("Closes this window.")
            fn close(&self) -> Bool {
                "str"
            }
        }
        "#;
        let res = check_source(src);
        assert!(
            res.is_err(),
            "Method in impl block returning mismatched type with @Docs must fail"
        );
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("expected Bool, found String")),
            "Expected type mismatch diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_async_func_call_without_await_fails() {
        let src = r#"
        async fn get_data() -> String {
            "hello"
        }

        async fn main() {
            let data = get_data();
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Calling async function without await must fail");
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("cannot call async function 'get_data' without 'await'")),
            "Expected async call diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_async_func_call_with_await_succeeds() {
        let src = r#"
        async fn get_data() -> String {
            "hello"
        }

        async fn main() {
            let data = await get_data();
        }
        "#;
        let res = check_source(src);
        assert!(res.is_ok(), "Calling async function with await in async fn must succeed: {:?}", res.err());
    }

    #[test]
    fn test_await_in_sync_function_fails() {
        let src = r#"
        async fn get_data() -> String {
            "hello"
        }

        fn sync_main() {
            let data = await get_data();
        }
        "#;
        let res = check_source(src);
        assert!(res.is_err(), "Calling await in sync function must fail");
        let diags = res.unwrap_err();
        assert!(
            diags.iter().any(|d| d.message.contains("'await' is only allowed in 'async' functions")),
            "Expected await outside async fn diagnostic, got: {:?}",
            diags
        );
    }

    #[test]
    fn test_if_statement_assignment_not_treated_as_return_value() {
        let src = r#"
        struct Formula {}

        fn check() -> Formula {
            let mut fields_valid = true;
            if true {
                fields_valid = false;
            }
            Formula {}
        }
        "#;
        let res = check_source(src);
        assert!(res.is_ok(), "Assignment inside if statement must not be treated as function return value: {:?}", res.err());
    }

    #[test]
    fn test_parameter_hover_info_recorded() {
        let src = r#"
        fn webhook(body: String, code: Int) -> Bool {
            true
        }
        "#;
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
        let (_, tc) = TypeChecker::new("test.fm".to_string()).check_program(&stmts);
        let hover_entries: Vec<_> = tc.hover_info.values().cloned().collect();
        assert!(
            hover_entries.iter().any(|h| h.contains("(parameter) body: String") && h.contains("Parameter of function `webhook`")),
            "Expected hover info for parameter body, got: {:?}",
            hover_entries
        );
    }



