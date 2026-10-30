use super::{JsonCompletion, JsonHover};

const KEYWORDS: &[(&str, &str)] = &[
    (
        "mut",
        "Keyword for mutable reference/variable. Example: `let mut x = 5`",
    ),
    (
        "default",
        "Declares a default method implementation inside a trait. Example:\n```flame\ntrait Describable {\n    default fn describe(&self) -> String {\n        \"default description\"\n    }\n}\n```",
    ),
    (
        "and",
        "Logical AND operator. Evaluates to true only if both operands are true. Example: `if is_online and is_ready { ... }`",
    ),
    (
        "or",
        "Logical OR operator. Evaluates to true if either operand is true. Example: `if is_offline or is_faulted { ... }`",
    ),
    (
        "not",
        "Logical NOT keyword. Inverts a boolean expression. Example: `if not is_faulted { ... }`",
    ),
    (
        "?:",
        "Nil-coalescing operator (Elvish operator). Returns the left-hand side if non-nil, otherwise evaluates and returns the fallback right-hand expression. Example: `let speed = motor?.speed ?: 0`",
    ),
    (
        "?.",
        "Safe-navigation member access operator. Evaluates member access if receiver is non-nil, otherwise short-circuits to nil without panicking. Example: `let speed = motor?.speed`",
    ),
    (
        "++",
        "Increment operator (supports prefix `++var` and postfix `var++`). Example: `count++`",
    ),
    (
        "--",
        "Decrement operator (supports prefix `--var` and postfix `var--`). Example: `timeout--`",
    ),
    (
        "+=",
        "Compound addition assignment operator. Example: `speed += 10`",
    ),
    (
        "-=",
        "Compound subtraction assignment operator. Example: `speed -= 10`",
    ),
    (
        "while",
        "Looping construct based on condition. Example: `while condition { ... }`",
    ),
    (
        "loop",
        "Infinite looping construct. Example: `loop { ... }`",
    ),
    (
        "for",
        "Iterating construct. Example: `for i in 0..10 { ... }`",
    ),
    (
        "in",
        "Used in for-loops or containment checks. Example: `for x in list`",
    ),
    (
        "as",
        "Type casting or import alias keyword. Example: `let n = val as Int`",
    ),
    ("type", "Defines a type alias. Example: `type UserID = Int`"),
    (
        "where",
        "Generic type constraint clause. Example: `fn test<T>(x: T) where T: Debug {}`",
    ),
    (
        "match",
        "Pattern matching. Example: `match x { 1 => ..., _ => ... }`",
    ),
    ("return", "Returns from a function. Example: `return 5`"),
    ("break", "Breaks out of a loop. Example: `break`"),
    (
        "continue",
        "Skips to the next iteration of a loop. Example: `continue`",
    ),
    (
        "defer",
        "Defers execution until the end of the scope. Example: `defer file.close()`",
    ),
    ("import", "Imports a module. Example: `import std.fs`"),
    (
        "export",
        "Exports a function, struct, enum, or annotation for external modules. Example: `export fn process() {}`",
    ),
    (
        "package",
        "Declares the package namespace for the current file. Required for folder-based imports. Example: `package utils`",
    ),
    (
        "native",
        "Native dependencies import prefix. Example: `import native.mysql`",
    ),
    (
        "std",
        "Standard library import prefix. Example: `import std.math`",
    ),
    (
        "thread",
        "Spawns a new background thread block. Example: `thread { thread.sleep(3000) }`",
    ),
    (
        "yield",
        "Yields execution or a value from a generator. Example: `yield value`",
    ),
    (
        "self",
        "Refers to the instance of the enclosing type within methods. Example: `self.name`",
    ),
    (
        "Self",
        "Refers to the enclosing type itself within a trait or impl block. Example: `fn new() -> Self`",
    ),
    (
        "formula",
        "Declares a static map/dictionary structure. Example: `formula { key: \"value\" }`",
    ),
    (
        "annotation",
        "Declares a reusable custom annotation function. Example: `annotation Benchmark(name: String) -> Formula {}`",
    ),
    (
        "@Application",
        "```flame\nannotation @Application(features: [\"String\"])\n```\n**Application Entry Point**\n\nMarks this function as the application's entry point. The function is invoked automatically when the program starts. Configuration options such as `features` enable optional standard library modules and control application-wide compiler/runtime behavior.",
    ),
    (
        "@Test",
        "```flame\nannotation @Test(timeout: Int = 1000, skip: Bool = false)\n```\n**Unit Test**\n\nMarks this function as a test case. The compiler will aggregate all `@Test` functions and execute them in a secure test harness when you run `flame test`.\n\n**Parameters:**\n- `timeout: Int`: Timeout in milliseconds. Test fails if execution exceeds this.\n- `skip: Bool`: If true, skips executing this test.",
    ),
    (
        "@ExpectPanic",
        "```flame\nannotation @ExpectPanic\n```\n**Expected Failure**\n\nAsserts that the test function MUST terminate with a panic or error; fails if the function completes successfully.",
    ),
    (
        "@Benchmark",
        "```flame\nannotation @Benchmark(warmup: Int = 10, iterations: Int = 100, group: String = \"\", name: String = \"\")\n```\n**Performance Benchmark**\n\nMarks this function as a performance benchmark executed by `flame bench`. Automatically handles warmup iterations, monotonic measurement sampling, percentiles (p50, p95, p99), throughput ops/sec, memory per op, and group comparisons.\n\n**Parameters:**\n- `warmup: Int`: Number of unmeasured warmup iterations (default: 10).\n- `iterations: Int`: Number of measured sample iterations (default: 100).\n- `group: String`: Grouping label for comparative benchmark tables.\n- `name: String`: Custom display label for reports.",
    ),
    (
        "@Cli",
        "```flame\nannotation @Cli\n```\n**CLI Application**\n\nMarks the application as a Command Line Interface tool, enabling automatic parsing of command line arguments into structures.",
    ),
    (
        "@Platform",
        "```flame\nannotation @Platform(target: String)\n```\n**Conditional Compilation**\n\nConditionally compiles the annotated declaration only if the active build target matches the given substring.\n\n**Example:**\n```flame\n@Platform(\"windows\")\nfn get_os_name() -> String {\n    \"Windows\"\n}\n```",
    ),
    (
        "@Docs",
        "```flame\nannotation @Docs(String...)\n```\n**Documentation Provider**\n\nProvides rich IDE hover documentation for functions, structs, and enums, supporting markdown syntax.\n\n**Example:**\n```flame\n@Docs(\"Computes the sum of two numbers.\")\nfn sum(a: Int, b: Int) -> Int {\n    a + b\n}\n```",
    ),
    (
        "@Requires",
        "```flame\nannotation @Requires(String...)\n```\n**Dependency Requirement**\n\nSpecifies system, hardware, or module dependencies required by this function or module. The compiler makes the dependency visible inside the function scope without globally importing it. It is loaded when the function executes and safely unloaded afterwards.\n\n**Example:**\n```flame\n@Requires(\"std.fs\")\n```",
    ),
    (
        "@Permission",
        "```flame\nannotation @Permission(String...)\n```\n**Access Permission**\n\nRequests specific runtime permissions (e.g., `\"net\"`, `\"fs\"`, `\"env\"`).\n\n**Rules:**\n- If no `@Permission` is specified anywhere in the project, permissions are auto-allowed.\n- If specified on any function, the user must explicitly allow all mentioned permissions at runtime (via terminal prompt or `flame.toml`), otherwise execution stops immediately.\n- When used on an `@Test` function, permissions are automatically granted.\n\n**Example:**\n```flame\n@Permission(\"net\", \"fs\")\n```",
    ),
    (
        "@Command",
        "```flame\nannotation @Command\n```\n**CLI Command**\n\nRegisters a function as an executable command within a `@Cli` application. Associates the function with a specific command-line keyword.",
    ),
    (
        "@Suggestions",
        "```flame\nannotation @Suggestions([{name: String, kind: String}])\n```\n**Package Suggestions**\n\nProvides custom suggestions for IDE autocompletion when typing the package name (e.g. `mypackage.`). Used inside `PackageDecl` to suggest objects or functions exported by the package.\n\n**Parameters:**\n- `args: Array`: An array of objects, where each object has `name` (the property to suggest) and `kind` (the completion kind, such as `\"function\"`, `\"method\"`, `\"property\"`, or `\"object\"`).",
    ),
    (
        "features",
        "**features: String[]**\n\nEnables optional standard library capabilities for the application. Enabled features are available throughout the program and only the required runtime dependencies are included in AOT builds.",
    ),
    ("Formula", "Type: A map-like literal data structure."),
    ("Int", "Type: A 64-bit signed integer."),
    ("Float", "Type: A 64-bit floating point number."),
    ("String", "Type: A UTF-8 text string."),
    ("Bool", "Type: A boolean value (true or false)."),
    ("Byte", "Type: An 8-bit binary byte or byte buffer."),
    ("Bytes", "Type: Contiguous dynamic byte buffer."),
    ("Nil", "Type: Represents the absence of a value."),
    (
        "ThreadHandler",
        "Type: A handle to a spawned background thread.",
    ),
    (
        "Result",
        "Type: Generic result type representing either success (`Ok(T)`) or failure (`Err(E)`). Example: `Result<Int, Error>`",
    ),
    (
        "Option",
        "Type: Generic optional type representing either a value (`Some(T)`) or nothing (`None`). Example: `Option<String>`",
    ),
    (
        "Ok",
        "Enum constructor for `Result<T, E>`. Wraps a successful value. Example: `Ok(42)`",
    ),
    (
        "Err",
        "Enum constructor for `Result<T, E>`. Wraps an error value. Example: `Err(\"failed\")`",
    ),
    (
        "Some",
        "Enum constructor for `Option<T>`. Wraps an optional value. Example: `Some(\"hello\")`",
    ),
    (
        "None",
        "Enum constructor for `Option<T>`. Represents absence of a value. Example: `None`",
    ),
];

pub const BLAZE_BUILTINS: &[(&str, &str, &str)] = &[
    (
        "println",
        "fn println<T>(value: T = \"\")",
        "Prints a value to standard output followed by a newline.\n\n**Example:**\n```flame\nprintln(\"Hello, Flame!\")\n```",
    ),
    (
        "print",
        "fn print<T>(value: T = \"\")",
        "Prints a value to standard output without trailing newline.\n\n**Example:**\n```flame\nprint(\"Processing...\")\n```",
    ),
    (
        "eprint",
        "fn eprint<T>(value: T = \"\")",
        "Prints a value to standard error.\n\n**Example:**\n```flame\neprint(\"Error: operation failed\")\n```",
    ),
    (
        "assert",
        "fn assert(condition: Bool, message: String? = nil)",
        "```flame\nfn assert(condition: Bool, message: String? = nil)\n```\nAsserts that condition evaluates to `true`. Terminates execution with an optional error message if false.\n\n**Example:**\n```flame\nassert(x > 0)\nassert(x > 0, \"x must be positive\")\n```",
    ),
    (
        "assertEq",
        "fn assertEq<T>(actual: T, expected: T, message: String? = nil)",
        "Asserts that `actual` equals `expected`. Terminates execution with an optional error message if values differ.\n\n**Example:**\n```flame\nassertEq(status_code, 200)\nassertEq(status_code, 200, \"Status must be 200 OK\")\n```",
    ),
    (
        "assertNe",
        "fn assertNe<T>(actual: T, unexpected: T, message: String? = nil)",
        "Asserts that `actual` does NOT equal `unexpected`. Terminates execution with an optional error message if equal.\n\n**Example:**\n```flame\nassertNe(result, nil)\nassertNe(result, nil, \"Result must not be nil\")\n```",
    ),
    (
        "assertTrue",
        "fn assertTrue(condition: Bool, message: String? = nil)",
        "Asserts that `condition` is `true`.\n\n**Example:**\n```flame\nassertTrue(list.isEmpty())\nassertTrue(is_ready, \"Worker must be ready\")\n```",
    ),
    (
        "assertFalse",
        "fn assertFalse(condition: Bool, message: String? = nil)",
        "Asserts that `condition` is `false`.\n\n**Example:**\n```flame\nassertFalse(file.exists())\nassertFalse(has_errors, \"No errors permitted\")\n```",
    ),
    (
        "panic",
        "fn panic(message: String)",
        "Terminates program execution immediately with an unrecoverable error message and diagnostic line trace.\n\n**Example:**\n```flame\npanic(\"Invalid state\")\n```",
    ),
    (
        "range",
        "fn range(start: Int, end: Int, step: Int = 1) -> [Int]",
        "Generates a vector of integers from `start` up to `end`.\n\n**Example:**\n```flame\nfor i in range(0, 5) { print(i) }\n```",
    ),
    (
        "mockData",
        "fn mockData(schema: String = \"default\") -> Formula",
        "Generates mock object data for testing.\n\n**Example:**\n```flame\nlet user = mockData(\"user\")\n```",
    ),
    (
        "mockApi",
        "fn mockApi(url: String = \"*\", body: String = \"{}\", status: Int = 200) -> Formula",
        "Configures mock responses for API endpoints during tests.\n\n**Example:**\n```flame\nlet res = mockApi(\"/api/v1/users\", \"{\\\"id\\\": 1}\", 200)\n```",
    ),
    (
        "mockFunction",
        "fn mockFunction<T>(name: String, return_value: T)",
        "Overrides a named function in the current environment to return `return_value` during tests.\n\n**Example:**\n```flame\nmockFunction(\"fetch_user\", formula { name: \"Alex\" })\n```",
    ),
    (
        "input",
        "fn input(prompt: String = \"\") -> String",
        "Takes a line of text input from standard input.\n\n**Example:**\n```flame\nlet name = input(\"Name: \")\n```",
    ),
    (
        "len",
        "fn len<T>(items: T) -> Int",
        "Returns the number of elements in an array or collection.\n\n**Example:**\n```flame\nlet count = len(items)\n```",
    ),
    (
        "length",
        "fn length<T>(items: T) -> Int",
        "Returns the number of elements in an array or collection.\n\n**Example:**\n```flame\nlet count = length(items)\n```",
    ),
    (
        "isEmpty",
        "fn isEmpty<T>(items: T) -> Bool",
        "Returns `true` if the collection contains no elements.\n\n**Example:**\n```flame\nif isEmpty(items) { print(\"empty\") }\n```",
    ),
];

pub fn get_keyword_completions(
    current_line: &str,
    raw_word: &str,
    prefix: &str,
    tc_opt: Option<&crate::typechecker::TypeChecker>,
) -> Vec<JsonCompletion> {
    let mut comps = Vec::new();

    if raw_word.starts_with('@') {
        let clean_prefix = prefix.trim_start_matches('@');

        // Workspace annotations first
        if let Some(tc) = tc_opt {
            for ann_name in &tc.annotations {
                if ann_name.starts_with(clean_prefix) || clean_prefix.is_empty() {
                    comps.push(JsonCompletion {
                        sort_text: Some("0_".to_string()),
                        label: format!("@{}", ann_name),
                        kind: "annotation".to_string(),
                        detail: "workspace annotation".to_string(),
                        documentation: tc
                            .hover_info
                            .values()
                            .find(|doc| doc.contains(&format!("annotation @{}", ann_name)))
                            .cloned(),
                    });
                }
            }
        }

        let annotations = [
            "@Application",
            "@Test",
            "@ExpectPanic",
            "@Benchmark",
            "@Cli",
            "@Command",
            "@Requires",
            "@Permission",
            "@Suggestions",
            "@Docs",
            "@Platform",
        ];
        for ann in annotations {
            if ann.starts_with(raw_word) {
                let label = ann.to_string();
                let clean_label = ann.trim_start_matches('@');
                if clean_label.starts_with(clean_prefix) || clean_prefix.is_empty() {
                    comps.push(JsonCompletion {
                        sort_text: Some("1_".to_string()),
                        label,
                        kind: "annotation".to_string(),
                        detail: "annotation".to_string(),
                        documentation: None,
                    });
                }
            }
        }
        return comps;
    }

    if current_line.contains("@Application")
        && current_line.contains("features")
        && current_line.contains('[')
    {
        let features = [
            "\"http\"", "\"tcp\"", "\"udp\"", "\"ws\"", "\"mqtt\"", "\"url\"",
        ];
        for feat in features {
            if feat.starts_with(prefix) || prefix.is_empty() || feat.contains(prefix) {
                // If prefix already has a quote, we don't want to insert double quotes.
                let label = if prefix.starts_with('"') {
                    feat.trim_start_matches('"').to_string()
                } else {
                    feat.to_string()
                };
                comps.push(JsonCompletion {
                    sort_text: None,
                    label,
                    kind: "value".to_string(),
                    detail: "feature module".to_string(),
                    documentation: None,
                });
            }
        }
        return comps;
    }

    for (name, sig, doc) in BLAZE_BUILTINS {
        if name.starts_with(prefix) || prefix.is_empty() {
            comps.push(JsonCompletion {
                sort_text: Some("0_".to_string()),
                label: name.to_string(),
                kind: "function".to_string(),
                detail: sig.to_string(),
                documentation: Some(doc.to_string()),
            });
        }
    }

    comps.extend(
        KEYWORDS
            .iter()
            .filter(|(kw, _)| {
                let is_alphabetic = kw
                    .chars()
                    .all(|c| c.is_alphabetic() || c == '_' || c == '@');
                is_alphabetic && (kw.starts_with(prefix) || prefix.is_empty())
            })
            .map(|(kw, doc)| JsonCompletion {
                sort_text: if kw.starts_with('@') {
                    Some("2_".to_string())
                } else {
                    None
                },
                label: kw.to_string(),
                kind: if kw.starts_with('@') {
                    "annotation".to_string()
                } else {
                    "keyword".to_string()
                },
                detail: if kw.starts_with('@') {
                    "annotation".to_string()
                } else {
                    "keyword".to_string()
                },
                documentation: Some(doc.to_string()),
            }),
    );

    comps
}

pub fn get_keyword_hover(word: &str) -> Option<JsonHover> {
    if let Some((_, sig, doc)) = BLAZE_BUILTINS.iter().find(|(name, _, _)| *name == word) {
        return Some(JsonHover {
            label: sig.to_string(),
            documentation: Some(doc.to_string()),
        });
    }

    let clean_word = word.trim_start_matches('@');
    let hover = KEYWORDS
        .iter()
        .find(|(kw, _)| *kw == word || kw.trim_start_matches('@') == clean_word)
        .map(|(kw, doc)| {
            let formatted_doc = if doc.starts_with("```") {
                doc.to_string()
            } else if let Some((desc, ex)) = doc.split_once("Example: `") {
                let clean_ex = ex.trim_end_matches('`');
                let kind = if desc.starts_with("Function:") || desc.starts_with("Method:") {
                    "fn"
                } else {
                    "keyword"
                };
                format!(
                    "```flame\n{} {}\n```\n{}\n\n**Example:**\n```flame\n{}\n```",
                    kind,
                    kw,
                    desc.trim(),
                    clean_ex
                )
            } else if doc.starts_with("Type:") {
                format!("```flame\ntype {}\n```\n{}", kw, doc.trim())
            } else if doc.starts_with("Function:") || doc.starts_with("Method:") {
                format!("```flame\nfn {}\n```\n{}", kw, doc.trim())
            } else if kw.starts_with('@') {
                format!("```flame\nannotation {}\n```\n{}", kw, doc.trim())
            } else {
                format!("```flame\nkeyword {}\n```\n{}", kw, doc.trim())
            };

            JsonHover {
                label: kw.to_string(),
                documentation: Some(formatted_doc),
            }
        });
    hover
}
