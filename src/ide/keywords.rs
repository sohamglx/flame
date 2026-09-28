use crate::{JsonCompletion, JsonHover};

const KEYWORDS: &[(&str, &str)] = &[
    ("let", "Declares a local variable. Example: `let x = 5`"),
    ("const", "Declares a constant. Example: `const x = 5`"),
    (
        "mut",
        "Keyword for mutable reference/variable. Example: `let mut x = 5`",
    ),
    ("fn", "Declares a function. Example: `fn do_something() {}`"),
    (
        "async",
        "Declares an asynchronous function. Example: `async fn do_something() {}`",
    ),
    (
        "struct",
        "Declares a struct. Example: `struct Point { x: Num, y: Num }`",
    ),
    (
        "enum",
        "Declares an enum. Example: `enum Color { Red, Green, Blue }`",
    ),
    (
        "trait",
        "Declares a trait (interface). Example: `trait Drawable { fn draw(); }`",
    ),
    (
        "impl",
        "Implements methods for a struct or enum. Example: `impl Point { fn new() -> Point {} }`",
    ),
    (
        "if",
        "Conditional execution. Example: `if condition { ... }`",
    ),
    (
        "else",
        "Alternative conditional execution. Example: `else { ... }`",
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
        "await",
        "Waits for an asynchronous task or thread to complete. Example: `await task`",
    ),
    (
        "println",
        "```flame\nfn println(value: Any)\n```\nPrints a value to standard output followed by a newline.\n\n**Example:**\n```flame\nprintln(\"Hello, Flame!\")\n```",
    ),
    (
        "print",
        "```flame\nfn print(value: Any)\n```\nPrints a value to standard output without trailing newline.\n\n**Example:**\n```flame\nprint(\"Processing...\")\n```",
    ),
    (
        "eprint",
        "```flame\nfn eprint(value: Any)\n```\nPrints a value to standard error.\n\n**Example:**\n```flame\neprint(\"Error: operation failed\")\n```",
    ),
    (
        "assert",
        "```flame\nfn assert(condition: Bool, message: String = \"\")\n```\nAsserts that condition evaluates to `true`. Terminates execution with an error if false.\n\n**Example:**\n```flame\nassert(x > 0, \"x must be positive\")\n```",
    ),
    (
        "assertEq",
        "```flame\nfn assertEq(actual: Any, expected: Any, message: String = \"\")\n```\nAsserts that `actual` equals `expected`. Terminates execution with an error if values differ.\n\n**Example:**\n```flame\nassertEq(status_code, 200)\n```",
    ),
    (
        "assertNe",
        "```flame\nfn assertNe(actual: Any, unexpected: Any, message: String = \"\")\n```\nAsserts that `actual` does NOT equal `unexpected`.\n\n**Example:**\n```flame\nassertNe(result, nil)\n```",
    ),
    (
        "assertTrue",
        "```flame\nfn assertTrue(condition: Bool, message: String = \"\")\n```\nAsserts that `condition` is `true`.\n\n**Example:**\n```flame\nassertTrue(list.is_empty())\n```",
    ),
    (
        "assertFalse",
        "```flame\nfn assertFalse(condition: Bool, message: String = \"\")\n```\nAsserts that `condition` is `false`.\n\n**Example:**\n```flame\nassertFalse(file.exists())\n```",
    ),
    (
        "panic",
        "```flame\nfn panic(message: String)\n```\nTerminates program execution immediately with an unrecoverable error message and diagnostic line trace. When called inside an `@Test` function, it halts the individual test case and marks it as failed with the specified message while allowing remaining test suites to proceed.\n\n**Example:**\n```flame\nif is_online and is_faulted {\n    panic(\"Invalid state: cannot be both online and faulted\")\n}\n```",
    ),
    (
        "typeof",
        "```flame\nfn typeof(value: Any) -> String\n```\nReturns the runtime type name of `value` as a String (e.g. `\"Int\"`, `\"String\"`, `\"Formula\"`).\n\n**Example:**\n```flame\nlet t = typeof(42)\n```",
    ),
    (
        "range",
        "```flame\nfn range(start: Int, end: Int, step: Int = 1) -> Vec<Int>\n```\nGenerates a vector of integers from `start` up to `end`.\n\n**Example:**\n```flame\nfor i in range(0, 5) { print(i) }\n```",
    ),
    (
        "sleep",
        "```flame\nfn sleep(ms: Int)\n```\nSuspends current thread execution for the specified milliseconds.\n\n**Example:**\n```flame\nsleep(1000)\n```",
    ),
    (
        "mockData",
        "```flame\nfn mockData(schema: String = \"default\") -> Formula\n```\nGenerates mock object data for testing. Supported schemas: `\"user\"`, `\"post\"`, `\"product\"`.\n\n**Example:**\n```flame\nlet user = mockData(\"user\")\n```",
    ),
    (
        "mockApi",
        "```flame\nfn mockApi(url: String = \"*\", body: String = \"{}\", status: Int = 200) -> Formula\n```\nConfigures mock responses for API endpoints during tests.\n\n**Example:**\n```flame\nlet res = mockApi(\"/api/v1/users\", \"{\\\"id\\\": 1}\", 200)\n```",
    ),
    (
        "mockFunction",
        "```flame\nfn mockFunction(name: String, return_value: Any)\n```\nOverrides a named function in the current environment to return `return_value` during tests.\n\n**Example:**\n```flame\nmockFunction(\"fetch_user\", formula { name: \"Alex\" })\n```",
    ),
    (
        "formula",
        "Declares a static map/dictionary structure. Example: `formula { key: \"value\" }`",
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
        "@Benchmark",
        "```flame\nannotation @Benchmark(warmup: Int = 10, iterations: Int = 100, group: String = \"\", name: String = \"\")\n```\n**Performance Benchmark**\n\nMarks this function as a performance benchmark executed by `flame bench`. Automatically handles warmup iterations, monotonic measurement sampling, percentiles (p50, p95, p99), throughput ops/sec, memory per op, and group comparisons.\n\n**Parameters:**\n- `warmup: Int`: Number of unmeasured warmup iterations (default: 10).\n- `iterations: Int`: Number of measured sample iterations (default: 100).\n- `group: String`: Grouping label for comparative benchmark tables.\n- `name: String`: Custom display label for reports.",
    ),
    (
        "@Embedded",
        "```flame\nannotation @Embedded(target: String)\n```\n**Embedded Target Definition**\n\nDirects the compiler to emit machine code tailored for a specific microcontroller architecture, such as `arduino-uno` or `rp2040`.\n\n**Parameters:**\n- `target: String`: The hardware architecture target name.",
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
    (
        "Formula",
        "Built-in Type: A map-like literal data structure.",
    ),
    ("Int", "Built-in Type: A 64-bit signed integer."),
    ("Float", "Built-in Type: A 64-bit floating point number."),
    ("String", "Built-in Type: A UTF-8 text string."),
    ("Bool", "Built-in Type: A boolean value (true or false)."),
    ("Nil", "Built-in Type: Represents the absence of a value."),
    ("Vec", "Built-in Type: A dynamically-sized array."),
    (
        "ThreadHandler",
        "Built-in Type: A handle to a spawned background thread.",
    ),
    (
        "input",
        "```flame\nfn input(prompt: String = \"\") -> String\n```\nTakes a line of text input from standard input.\n\nExample:\n```flame\nlet name = input(\"Name: \")\n```",
    ),
    (
        "push",
        "Built-in Method: Appends an element to the back of a collection. Example: `arr.push(100)`",
    ),
    (
        "pop",
        "Built-in Method: Removes the last element from a collection and returns it. Example: `let last = arr.pop()`",
    ),
    (
        "len",
        "Built-in Method: Returns the number of elements in the collection or string. Example: `let l = arr.len()`",
    ),
    (
        "isEmpty",
        "Built-in Method: Returns true if the collection or string contains no elements. Example: `if arr.is_empty() { ... }`",
    ),
    (
        "filter",
        "Built-in Method: Creates a new array containing elements that pass the provided test function. Example: `let filtered = arr.filter((x: Int) { return x > 10 })`",
    ),
    (
        "map",
        "Built-in Method: Creates a new array populated with the results of calling a provided function on every element. Example: `let mapped = arr.map((x: Int) { return x * 2 })`",
    ),
    (
        "insert",
        "Built-in Method: Inserts or updates a key-value pair in a Formula map. Example: `map.insert(\"role\", \"admin\")`",
    ),
    (
        "get",
        "Built-in Method: Retrieves a value by key from a Formula map. Example: `let val = map.get(\"key\")`",
    ),
    (
        "remove",
        "Built-in Method: Removes a key from a Formula map. Example: `map.remove(\"key\")`",
    ),
    (
        "clone",
        "Built-in Method: Explicitly copies a value to prevent ownership move. Example: `let copy = val.clone()`",
    ),
    (
        "contains",
        "Built-in Method: Checks if a string contains the given substring. Example: `str.contains(\"pattern\")`",
    ),
    (
        "startsWith",
        "Built-in Method: Checks if a string begins with the given prefix. Example: `str.starts_with(\"prefix\")`",
    ),
    (
        "endsWith",
        "Built-in Method: Checks if a string ends with the given suffix. Example: `str.endsWith(\"suffix\")`",
    ),
    (
        "replace",
        "Built-in Method: Replaces occurrences of a substring with another. Example: `str.replace(\"old\", \"new\")`",
    ),
    (
        "trim",
        "Built-in Method: Strips leading and trailing whitespace from a string. Example: `str.trim()`",
    ),
    (
        "toUpperCase",
        "Built-in Method: Converts a string to uppercase. Example: `str.toUpperCase()`",
    ),
    (
        "toLowerCase",
        "Built-in Method: Converts a string to lowercase. Example: `str.to_lowercase()`",
    ),
    (
        "annotation",
        "Declares a reusable custom annotation function. Example: `annotation Benchmark(name: String) -> Formula {}`",
    ),
    (
        "Cli",
        "```flame\nannotation @Cli\n```\n**CLI Application**\n\nMarks the application as a Command Line Interface tool, enabling automatic parsing of command line arguments into structures.",
    ),
    (
        "Command",
        "```flame\nannotation @Command\n```\n**CLI Command**\n\nRegisters a function as an executable command within a `@Cli` application. Associates the function with a specific command-line keyword.",
    ),
    (
        "Test",
        "```flame\nannotation @Test(timeout: Int = 1000, skip: Bool = false)\n```\n**Unit Test**\n\nMarks this function as a test case. The compiler will aggregate all `@Test` functions and execute them in a secure test harness when you run `flame test`.\n\n**Parameters:**\n- `timeout: Int`: Timeout in milliseconds. Test fails if execution exceeds this.\n- `skip: Bool`: If true, skips executing this test.",
    ),
    (
        "Benchmark",
        "```flame\nannotation @Benchmark(warmup: Int = 10, iterations: Int = 100, group: String = \"\", name: String = \"\")\n```\n**Performance Benchmark**\n\nMarks this function as a performance benchmark executed by `flame bench`. Automatically handles warmup iterations, monotonic measurement sampling, percentiles (p50, p95, p99), throughput ops/sec, memory per op, and group comparisons.\n\n**Parameters:**\n- `warmup: Int`: Number of unmeasured warmup iterations (default: 10).\n- `iterations: Int`: Number of measured sample iterations (default: 100).\n- `group: String`: Grouping label for comparative benchmark tables.\n- `name: String`: Custom display label for reports.",
    ),
    (
        "Setup",
        "```flame\nannotation @Setup\n```\n**Test Setup**\n\nRuns before every test in the module (equivalent to `beforeEach()`).\nIgnored during `flame run` and `flame build`.",
    ),
    (
        "Cleanup",
        "```flame\nannotation @Cleanup\n```\n**Test Cleanup**\n\nRuns after every test in the module (equivalent to `afterEach()`).\nIgnored during `flame run` and `flame build`.",
    ),
    (
        "BeforeAll",
        "```flame\nannotation @BeforeAll\n```\n**Module Initialization**\n\nRuns exactly once before any test executes (e.g., database connection setup).\nIgnored during `flame run` and `flame build`.",
    ),
    (
        "AfterAll",
        "```flame\nannotation @AfterAll\n```\n**Module Teardown**\n\nRuns exactly once after all tests complete (e.g., closing servers or cleaning temp files).\nIgnored during `flame run` and `flame build`.",
    ),
    (
        "Ignore",
        "```flame\nannotation @Ignore\n```\n**Skip Test**\n\nSkips test execution when running `flame test`.",
    ),
    (
        "Only",
        "```flame\nannotation @Only\n```\n**Focus Test**\n\nRestricts test execution to ONLY functions marked with `@Only` during `flame test`.",
    ),
    (
        "Parameterized",
        "```flame\nannotation @Parameterized(arguments: Vector<Tuple>)\n```\n**Parameterized Test**\n\nExpands a test into multiple independent test cases, passing each tuple element as parameters to the test function.",
    ),
    (
        "Embedded",
        "```flame\nannotation @Embedded(target: String)\n```\n**Embedded Target Definition**\n\nDirects the compiler to emit machine code tailored for a specific microcontroller architecture, such as `arduino-uno` or `rp2040`.\n\n**Parameters:**\n- `target: String`: The hardware architecture target name.",
    ),
    (
        "Benchmark",
        "```flame\nannotation @Benchmark\n```\n**Performance Benchmark**\n\nExecutes the function as a high-precision performance benchmark during `flame test`, reporting average, minimum, and maximum execution times.",
    ),
    (
        "Requires",
        "```flame\nannotation @Requires(String...)\n```\n**Dependency Requirement**\n\nSpecifies system, hardware, or module dependencies required by this function or module. The compiler makes the dependency visible inside the function scope without globally importing it. It is loaded when the function executes and safely unloaded afterwards.\n\n**Example:**\n```flame\n@Requires(\"std.fs\")\n```",
    ),
    (
        "Permission",
        "```flame\nannotation @Permission(String...)\n```\n**Access Permission**\n\nRequests specific runtime permissions (e.g., `\"net\"`, `\"fs\"`, `\"env\"`).\n\n**Rules:**\n- If no `@Permission` is specified anywhere in the project, permissions are auto-allowed.\n- If specified on any function, the user must explicitly allow all mentioned permissions at runtime (via terminal prompt or `flame.toml`), otherwise execution stops immediately.\n- When used on an `@Test` function, permissions are automatically granted.\n\n**Example:**\n```flame\n@Permission(\"net\", \"fs\")\n```",
    ),
    (
        "ExpectPanic",
        "```flame\nannotation @ExpectPanic\n```\n**Expected Failure**\n\nAsserts that the test function MUST terminate with a panic or error; fails if the function completes successfully.",
    ),
    (
        "toInt",
        "```flame\nfn toInt(radix: Int = 10) -> Int\n```\nConverts a String value to a signed integer. Throws a runtime error if digits are invalid.",
    ),
    (
        "tryInt",
        "```flame\nfn tryInt(radix: Int = 10) -> Int | Nil\n```\nAttempts to parse a String value to an integer, returning `nil` if parsing fails.",
    ),
    (
        "toFloat",
        "```flame\nfn toFloat() -> Float\n```\nConverts a String value to a floating-point number.",
    ),
    (
        "tryFloat",
        "```flame\nfn tryFloat() -> Float | Nil\n```\nAttempts to parse a String value to a float, returning `nil` if parsing fails.",
    ),
    (
        "toDouble",
        "```flame\nfn toDouble() -> Float\n```\nConverts a String value to a double-precision floating-point number.",
    ),
    (
        "tryDouble",
        "```flame\nfn tryDouble() -> Float | Nil\n```\nAttempts to parse a String value to a double, returning `nil` if parsing fails.",
    ),
    (
        "toBool",
        "```flame\nfn toBool() -> Bool\n```\nConverts a String (\"true\", \"false\", \"1\", \"0\", etc.) to a boolean value.",
    ),
    (
        "tryBool",
        "```flame\nfn tryBool() -> Bool | Nil\n```\nAttempts to convert a String to a boolean value, returning `nil` if unrecognized.",
    ),
    (
        "toChar",
        "```flame\nfn toChar() -> String\n```\nReturns a string consisting of the first character.",
    ),
    (
        "toByte",
        "```flame\nfn toByte() -> Byte\n```\nConverts a String or Int into a binary Byte or Byte array.",
    ),
    (
        "toString",
        "```flame\nfn toString(precision: Int = -1) -> String\n```\nConverts any value (integer, float, boolean, nil, byte array) into its String representation. For floats, specifying precision limits decimal digits.",
    ),
    (
        "toUtf8",
        "```flame\nfn toUtf8() -> String\n```\nDecodes a Byte array into a UTF-8 String. Panics if the bytes are not valid UTF-8.",
    ),
    (
        "tryUtf8",
        "```flame\nfn tryUtf8() -> String?\n```\nAttempts to decode a Byte array into a UTF-8 String. Returns `nil` if the bytes are not valid UTF-8.",
    ),
    (
        "delete",
        "```flame\nfn delete(path: String) -> Nil\n```\nDeletes a file or directory from the file system. Throws a runtime error if the path does not exist.\n\n**Example**:\n```flame\nfs.delete(\"temp.txt\")\n```",
    ),
    (
        "exists",
        "```flame\nfn exists(path: String) -> Bool\n```\nChecks if a file or directory exists at the specified path. Returns `true` if it exists, otherwise `false`.\n\n**Example**:\n```flame\nif fs.exists(\"config.toml\") {\n    let cfg = fs.read(\"config.toml\")\n}\n```",
    ),
    (
        "send",
        "```flame\nfn send(value: Any) -> Nil\n```\nSends a message value through the channel to the connected `Receiver`.\n\n**Example**:\n```flame\ntx.send(\"hello\")\n```",
    ),
    (
        "recv",
        "```flame\nfn recv() -> Any\n```\nBlocks the current thread until a message is received from the channel.\n\n**Example**:\n```flame\nlet msg = rx.recv()\n```",
    ),
    (
        "tryRecv",
        "```flame\nfn tryRecv() -> Any | Nil\n```\nAttempts to receive a message without blocking. Returns `nil` immediately if the channel is currently empty.\n\n**Example**:\n```flame\nlet msg = rx.tryRecv()\nif msg != nil {\n    println($\"Received: {msg}\")\n}\n```",
    ),
    (
        "writeBytes",
        "```flame\nfn writeBytes(path: String, bytes: Bytes | [Int]) -> Nil\n```\nWrites a Bytes buffer or list of byte integers to a file, replacing its contents if it already exists.\n\n**Example**:\n```flame\nfs.writeBytes(\"data.bin\", bytes)\n```",
    ),
    (
        "readBytes",
        "```flame\nfn readBytes(path: String) -> Bytes\n```\nReads the entire contents of a file as a binary Bytes buffer.\n\n**Example**:\n```flame\nlet data = fs.readBytes(\"archive.fmp\")\n```",
    ),
    (
        "appendBytes",
        "```flame\nfn appendBytes(path: String, bytes: Bytes | [Int]) -> Nil\n```\nAppends a Bytes buffer or list of byte integers to the end of a file.\n\n**Example**:\n```flame\nbyte.appendBytes(\"archive.fmp\", payload)\n```",
    ),
    (
        "writeByte",
        "```flame\nfn writeByte(path: String, byte: Int | Byte) -> Nil\n```\nWrites a single byte (0-255) to a file.\n\n**Example**:\n```flame\nbyte.writeByte(\"data.bin\", 65)\n```",
    ),
    (
        "readByte",
        "```flame\nfn readByte(path: String) -> Int\n```\nReads a single byte from a file at offset 0.\n\n**Example**:\n```flame\nlet b = byte.readByte(\"data.bin\")\n```",
    ),
    (
        "appendByte",
        "```flame\nfn appendByte(path: String, byte: Int | Byte) -> Nil\n```\nAppends a single byte (0-255) to the end of a file.\n\n**Example**:\n```flame\nbyte.appendByte(\"data.bin\", 255)\n```",
    ),
    (
        "writeByteAt",
        "```flame\nfn writeByteAt(path: String, offset: Int, byte: Int | Byte) -> Nil\n```\nWrites a single byte to a file at a specific offset.\n\n**Example**:\n```flame\nbyte.writeByteAt(\"archive.fmp\", 0, 70)\n```",
    ),
    (
        "readByteAt",
        "```flame\nfn readByteAt(path: String, offset: Int) -> Int\n```\nReads a single byte from a file at a specific offset.\n\n**Example**:\n```flame\nlet magic = byte.readByteAt(\"archive.fmp\", 3)\n```",
    ),
];

const EMBEDDED_LITERALS: &[(&str, &str, &str)] = &[
    (
        "arduino-uno",
        "Hardware Target (AVR ATmega328P)",
        "Zero-cost `#![no_std]` compiler target for classic Arduino Uno boards.",
    ),
    (
        "esp32",
        "Hardware Target (Xtensa/RISC-V)",
        "Zero-cost compiler target for ESP32 Wi-Fi & Bluetooth chips.",
    ),
    (
        "stm32",
        "Hardware Target (ARM Cortex-M)",
        "Zero-cost compiler target for STM32 32-bit microcontrollers.",
    ),
    (
        "rp2040",
        "Hardware Target (Raspberry Pi Silicon)",
        "Zero-cost compiler target for dual-core ARM Cortex-M0+ RP2040 chips.",
    ),
    (
        "atmega328p",
        "Hardware Target (Bare AVR IC)",
        "Direct chip compiler target for standalone ATmega328P microcontrollers.",
    ),
    (
        "mega",
        "Hardware Target (Arduino Mega 2560)",
        "Compiler target for ATmega2560 extended GPIO boards.",
    ),
    (
        "avr-nano",
        "Hardware Target (Arduino Nano)",
        "Compiler target for compact ATmega328P Nano breakout boards.",
    ),
    (
        "OUTPUT",
        "Pin Mode Parameter",
        "Configures digital pin as push-pull output driver.",
    ),
    (
        "INPUT",
        "Pin Mode Parameter",
        "Configures pin as high-impedance floating input.",
    ),
    (
        "INPUT_PULLUP",
        "Pin Mode Parameter",
        "Configures pin input with internal 20K pull-up resistor active.",
    ),
    (
        "PWM",
        "Pin Mode Parameter",
        "Configures pin for Pulse-Width Modulation wave output.",
    ),
    (
        "HIGH",
        "Digital Output Level",
        "Drives pin voltage to VCC (5V / 3.3V).",
    ),
    (
        "LOW",
        "Digital Output Level",
        "Drives pin voltage to Ground (0V).",
    ),
];

pub fn get_literal_completions(prefix: &str) -> Vec<JsonCompletion> {
    let clean = prefix.trim_matches('"').trim_matches('\'');
    EMBEDDED_LITERALS
        .iter()
        .filter(|(val, _, _)| val.starts_with(clean) || clean.is_empty())
        .map(|(val, kind, doc)| JsonCompletion {
            sort_text: None,
            label: format!("\"{}\"", val),
            kind: "value".to_string(),
            detail: kind.to_string(),
            documentation: Some(format!("```flame\n\"{}\"\n```\n**{}**\n{}", val, kind, doc)),
        })
        .collect()
}

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
            "@Benchmark",
            "@Embedded",
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
                        detail: "built-in annotation".to_string(),
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
                    "built-in annotation".to_string()
                } else {
                    "keyword".to_string()
                },
                documentation: Some(doc.to_string()),
            }),
    );

    if current_line.contains("target:") || current_line.contains("@Embedded") {
        let lits = get_literal_completions(prefix);
        for mut lit in lits {
            if prefix.starts_with('"') {
                lit.label = lit.label.trim_start_matches('"').to_string();
            }
            comps.push(lit);
        }
    }

    comps
}

pub fn get_keyword_hover(word: &str) -> Option<JsonHover> {
    let clean_word = word.trim_start_matches('@');
    let mut hover = KEYWORDS
        .iter()
        .find(|(kw, _)| *kw == word || kw.trim_start_matches('@') == clean_word)
        .map(|(kw, doc)| {
            let formatted_doc = if doc.starts_with("```") {
                doc.to_string()
            } else if let Some((desc, ex)) = doc.split_once("Example: `") {
                let clean_ex = ex.trim_end_matches('`');
                let kind = if desc.starts_with("Built-in Function:")
                    || desc.starts_with("Built-in Method:")
                {
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
            } else if doc.starts_with("Built-in Type:") {
                format!("```flame\ntype {}\n```\n{}", kw, doc.trim())
            } else if doc.starts_with("Built-in Function:") || doc.starts_with("Built-in Method:") {
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

    if hover.is_none() {
        let clean_lit = word.trim_matches('"').trim_matches('\'');
        if let Some((val, kind, doc)) = EMBEDDED_LITERALS
            .iter()
            .find(|(val, _, _)| *val == clean_lit)
        {
            hover = Some(JsonHover {
                label: format!("\"{}\"", val),
                documentation: Some(format!("```flame\n\"{}\"\n```\n**{}**\n{}", val, kind, doc)),
            });
        }
    }
    hover
}

