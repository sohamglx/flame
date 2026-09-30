# Contributing to Flame

Welcome to the Flame programming language! We are excited to have you contribute. This guide will walk you through the codebase architecture, how the modular components interact, where everything is located, and step-by-step instructions on how to add features, write tests, configure IDE support, and submit contributions.

---

## Table of Contents

1. [Architecture & Codebase Tour](#architecture--codebase-tour)
   - [Where Everything Lives](#where-everything-lives)
   - [Detailed Component Breakdown](#detailed-component-breakdown)
2. [Core Architectural Rules & Conventions](#core-architectural-rules--conventions)
   - [Pattern Matching Syntax: Destructuring with Parentheses](#pattern-matching-syntax-destructuring-with-parentheses)
   - [Enums and Struct Implementations](#enums-and-struct-implementations)
   - [IDE Styling: TextMate Grammar vs Semantic Tokens](#ide-styling-textmate-grammar-vs-semantic-tokens)
   - [Compiler Diagnostics Architecture](#compiler-diagnostics-architecture)
3. [Getting Started as a Contributor](#getting-started-as-a-contributor)
   - [Prerequisites](#prerequisites)
   - [Building and Running Locally](#building-and-running-locally)
   - [Running the Test Suite](#running-the-test-suite)
4. [How to Add Features: Step-by-Step Guides](#how-to-add-features-step-by-step-guides)
   - [Guide 1: Adding a New Syntax Element or Keyword](#guide-1-adding-a-new-syntax-element-or-keyword)
   - [Guide 2: Adding a Standard Library (`std`) Module](#guide-2-adding-a-standard-library-std-module)
   - [Guide 3: Adding or Modifying `flame.toml` Options (`[options]`)](#guide-3-adding-or-modifying-flametoml-options-options)
   - [Guide 4: Emitting Rich Compiler Diagnostics](#guide-4-emitting-rich-compiler-diagnostics)
   - [Guide 5: Adding Built-in Annotations & Syntax Styling](#guide-5-adding-built-in-annotations--syntax-styling)
   - [Guide 6: Working on the IDE Extension & Syntax Highlighting](#guide-6-working-on-the-ide-extension--syntax-highlighting)
   - [Guide 7: Adding or Modifying a CLI Subcommand](#guide-7-adding-or-modifying-a-cli-subcommand)
   - [Guide 8: Contributing to Documentation (`docs/`)](#guide-8-contributing-to-documentation-docs)
   - [Guide 9: Adding Common Utility Functions](#guide-9-adding-common-utility-functions)
5. [Testing & Quality Standards](#testing--quality-standards)
6. [Submitting a Pull Request](#submitting-a-pull-request)

---

## Architecture & Codebase Tour

Flame is written in Rust (2024 edition). The repository is structured into modular, component-based directories so that each phase of the language pipeline has clear boundaries and responsibilities.

### Where Everything Lives

```text
flame/
├── Cargo.toml               # Package configuration and feature flags
├── src/
│   ├── lib.rs               # Library root re-exporting core modules
│   ├── main.rs              # CLI entry point and top-level argument dispatcher
│   ├── lexer.rs             # Lexical analysis: token scanner and span tracking
│   ├── parser/              # Syntax analysis: AST definitions and recursive-descent parser
│   ├── diagnostics.rs       # Centralized diagnostic engine (colored spans, errors, warnings, notes)
│   ├── typechecker/         # Semantic analysis: static typing, scopes, inference, options enforcement
│   ├── runner/              # Tree-walking runtime: expression evaluator, statement execution, plugins
│   ├── vm.rs                # Runtime value models (Value, Env, CValue, FFI)
│   ├── stdlib.rs            # Built-in global functions and stdlib loader
│   ├── native_std/          # Native Rust standard library implementations (fs, net, math, os, etc.)
│   ├── package_manager/     # Package manager: dependency resolution, downloading, FMI generation
│   ├── ide/                 # IDE intelligence: keyword documentation, definition finder, semantic tokens
│   ├── cli/                 # CLI subcommand implementations and JSON language server engine
│   ├── utils/               # Centralized shared utilities (manifests, formatting, fs, text)
│   ├── compiler.rs          # AOT native binary compilation pipeline
│   ├── web/                 # Fullstack web compiler and embedded dev server
│   ├── blaze.rs             # Blaze build system integration
│   ├── formatter.rs         # Flame code formatter
│   └── test_engine.rs       # Integrated test harness execution engine
├── Blaze/
│   └── std/                 # High-level standard library written in Flame (.fm modules)
├── ide/                     # Visual Studio Code extension (syntaxes, snippets, extension.js)
├── docs/                    # Starlight/Astro official documentation site
├── macro/                   # Procedural macros for native plugins (#[flame_export], etc.)
├── binder/                  # Official Rust integration crate (flamebinder)
├── wasm/                    # WebAssembly browser runtime and playground bindings
└── examples/                # Example Flame programs, CLI tools, and plugins
```

---

### Detailed Component Breakdown

#### 1. [`src/parser/`](file:///home/spidey/Projects/flame/src/parser) & [`src/lexer.rs`](file:///home/spidey/Projects/flame/src/lexer.rs) — Syntax & AST

- [`src/lexer.rs`](file:///home/spidey/Projects/flame/src/lexer.rs): Token scanner tracking line and column numbers using `Span { start, end, line, col }`.
- [`src/parser/ast.rs`](file:///home/spidey/Projects/flame/src/parser/ast.rs): Definitions for all Abstract Syntax Tree (AST) nodes: `Expr`, `Stmt`, `BinaryOp`, `UnaryOp`, `LiteralValue`, `Param`, `Annotation`, `MatchArm`, `EnumVariant`, and span implementations.
- [`src/parser/parser.rs`](file:///home/spidey/Projects/flame/src/parser/parser.rs): Recursive-descent parser that consumes a token stream and produces AST statements (`Vec<Stmt>`).
- [`src/parser/utils.rs`](file:///home/spidey/Projects/flame/src/parser/utils.rs): Indentation stripping (`strip_common_indentation`), test annotation detection, and platform conditional filtering (`filter_platform_stmts`).
- [`src/parser/mod.rs`](file:///home/spidey/Projects/flame/src/parser/mod.rs): Re-exports AST nodes and parser interfaces.

#### 2. [`src/diagnostics.rs`](file:///home/spidey/Projects/flame/src/diagnostics.rs) — Compiler Diagnostics Engine

- Centralized diagnostics engine that formats compiler errors, warnings, and informational notices.
- Struct `Diagnostic`:
  - `severity`: `DiagnosticSeverity::Error`, `Warning`, or `Info`.
  - `message`: High-level summary of the diagnostic.
  - `filepath`: Path to the source file where the issue occurred.
  - `span`: Precise `Span` (line, column, byte offsets).
  - `label`: Optional inline label attached to the underline pointer (e.g. `^ not allowed here`).
  - `suggestion`: Optional actionable fix displayed below the snippet.
  - `note`: Optional contextual note explaining compiler behavior or rules.
- Helper constructors:
  - `Diagnostic::new_error(message, filepath, span, label, suggestion)`
  - `Diagnostic::new_warning(message, filepath, span, label, suggestion)`
  - `.with_note(note)`
- Visual output includes cyan line numbers and borders, bold red `✖ error:` or bold yellow `⚠ warning:` headers, caret underlines (`^^^^`), and formatted help hints.

#### 3. [`src/typechecker/`](file:///home/spidey/Projects/flame/src/typechecker) — Semantic Analysis & Type System

- [`src/typechecker/types.rs`](file:///home/spidey/Projects/flame/src/typechecker/types.rs): Core type definitions (`Type`, `VarInfo`, `ParamInfo`, `FunctionSig`, `StructInfo`, `EnumInfo`, `CommandInfo`).
- [`src/typechecker/checker.rs`](file:///home/spidey/Projects/flame/src/typechecker/checker.rs): `TypeChecker` struct, `new()`, `check_program()`, symbol resolution, and hover registry.
- [`src/typechecker/builtins.rs`](file:///home/spidey/Projects/flame/src/typechecker/builtins.rs): Registration of built-in primitive methods (String, Array, Map, Num, Int, etc.) and `get_std_module_type()`.
- [`src/typechecker/stmts.rs`](file:///home/spidey/Projects/flame/src/typechecker/stmts.rs): Top-level declaration collection and statement type checking:
  - Validates struct, enum, and function declarations.
  - Enforces `rust-plugins = "deny" | "warn"` restrictions when inspecting import statements.
  - Enforces pattern matching destructuring rules.
- [`src/typechecker/exprs.rs`](file:///home/spidey/Projects/flame/src/typechecker/exprs.rs): Expression type inference (`infer_expr_type()`, `infer_binary_type()`, calls, indexing, member access):
  - Enforces `closure-types = "default" | "strict"`. In `strict` mode, closure parameter types and counts are strictly validated.
- [`src/typechecker/helpers.rs`](file:///home/spidey/Projects/flame/src/typechecker/helpers.rs): Type compatibility rules (`is_compatible()`), assignment validation, type formatting, and scope management (`push_scope()`, `pop_scope()`, `define_var()`).
- [`src/typechecker/tests.rs`](file:///home/spidey/Projects/flame/src/typechecker/tests.rs): Unit tests for semantic analysis and typing rules.

#### 4. [`src/runner/`](file:///home/spidey/Projects/flame/src/runner) — Execution Engine & Runtime

- [`src/runner/core.rs`](file:///home/spidey/Projects/flame/src/runner/core.rs): `Runner` struct initialization (`new()`), execution entry point (`run()`), and thread cloning.
- [`src/runner/callbacks.rs`](file:///home/spidey/Projects/flame/src/runner/callbacks.rs): Asynchronous callback queue processing and native callback invocation.
- [`src/runner/stmts.rs`](file:///home/spidey/Projects/flame/src/runner/stmts.rs): Statement execution (`execute_statement()` for loops, conditionals, assignments, pattern matches).
- [`src/runner/exprs.rs`](file:///home/spidey/Projects/flame/src/runner/exprs.rs): Expression evaluator (`eval_expr()`) handling literals, operators, function calls, closures, formula maps, etc.
- [`src/runner/target.rs`](file:///home/spidey/Projects/flame/src/runner/target.rs): Reference path resolution, mutation target lookup, and write-back semantics.
- [`src/runner/plugins.rs`](file:///home/spidey/Projects/flame/src/runner/plugins.rs): Dynamic native plugin loading, native C ABI method calling, CLI argument binding, and runtime enforcement of manifest policies (`rust-plugins = "deny"`).
- [`src/runner/tests.rs`](file:///home/spidey/Projects/flame/src/runner/tests.rs): Runtime execution unit tests.

#### 5. [`src/vm.rs`](file:///home/spidey/Projects/flame/src/vm.rs) & [`src/stdlib.rs`](file:///home/spidey/Projects/flame/src/stdlib.rs) — Values & Built-ins

- [`src/vm.rs`](file:///home/spidey/Projects/flame/src/vm.rs): Core value types (`Value::Int`, `Num`, `String`, `Bool`, `Array`, `Map`, `Struct`, `Enum`, `Closure`, `NativeCallback`), environment scopes (`Env`), and FFI data structures (`CValue`).
- [`src/stdlib.rs`](file:///home/spidey/Projects/flame/src/stdlib.rs): Built-in global functions (`println`, `print`, `assert`, `panic`, `type_of`, etc.) and the registration bridge to load `std.*` modules.

#### 6. [`src/native_std/`](file:///home/spidey/Projects/flame/src/native_std) & [`Blaze/std/`](file:///home/spidey/Projects/flame/Blaze/std) — Standard Library Layers

Flame provides standard library functionality at two levels:
- **Native Rust Standard Library ([`src/native_std/`](file:///home/spidey/Projects/flame/src/native_std))**: Low-level, high-performance implementations in Rust for I/O and OS integration:
  - `fs/`: File system operations, directory walking, metadata.
  - `net/`: TCP, UDP, HTTP, and WebSocket clients and servers.
  - `math/`: Trigonometry, random numbers, numerical algorithms.
  - `os/`: Environment variables, processes, system signals.
  - `time/`: Timers, durations, dates, clocks.
- **High-Level Flame Standard Library ([`Blaze/std/`](file:///home/spidey/Projects/flame/Blaze/std))**: Pure Flame modules (`.fm`) providing idiomatic Flame wrappers and utilities:
  - `fs.fm`, `net.fm`, `web.fm`, `time.fm`, `json.fm`, `annotations.fm`.

#### 7. [`src/package_manager/`](file:///home/spidey/Projects/flame/src/package_manager), [`macro/`](file:///home/spidey/Projects/flame/macro), & [`binder/`](file:///home/spidey/Projects/flame/binder) — Packages & Native Plugins

- [`src/package_manager/meta.rs`](file:///home/spidey/Projects/flame/src/package_manager/meta.rs): Metadata definitions (`FlameMeta`, `FlameFunctionMeta`, `PluginSpec`).
- [`src/package_manager/manifest.rs`](file:///home/spidey/Projects/flame/src/package_manager/manifest.rs): `flame.toml` plugin entry parser and plugin listing.
- [`src/package_manager/downloader.rs`](file:///home/spidey/Projects/flame/src/package_manager/downloader.rs): Package archive streaming downloader with terminal progress bar.
- [`src/package_manager/ops.rs`](file:///home/spidey/Projects/flame/src/package_manager/ops.rs): Package lifecycle (`add_package`, `remove_package`, `install_all_packages`, `ensure_dependencies_installed`). Enforces plugin options during package operations.
- [`src/package_manager/native.rs`](file:///home/spidey/Projects/flame/src/package_manager/native.rs): Native plugin compilation using `cargo build`, FMI inspection, and `.fmi` interface generation.
- [`macro/`](file:///home/spidey/Projects/flame/macro): Procedural macro crate defining `#[flame_export]`, used by Rust crates to export functions and types to Flame.
- [`binder/`](file:///home/spidey/Projects/flame/binder): Official `flamebinder` crate for embedding Flame within external Rust applications.

#### 8. [`src/ide/`](file:///home/spidey/Projects/flame/src/ide) & [`ide/`](file:///home/spidey/Projects/flame/ide) — IDE Support & Syntax Highlighting

- [`src/ide/semantic_tokens.rs`](file:///home/spidey/Projects/flame/src/ide/semantic_tokens.rs): Extracts declared enums and enum variants (including built-ins `Option`, `Result`, `Error`, `Some`, `None`, `Ok`, `Err`), mapping them to token types 3 (enum) and 10 (enumMember) to be rendered in blue.
- [`src/ide/keywords.rs`](file:///home/spidey/Projects/flame/src/ide/keywords.rs): Registry of Flame keywords, syntax descriptions, and autocompletion templates for hover documentation.
- [`src/ide/scanner.rs`](file:///home/spidey/Projects/flame/src/ide/scanner.rs): Document scanner extracting variables, types, and functions.
- [`src/ide/definition.rs`](file:///home/spidey/Projects/flame/src/ide/definition.rs): Symbol definition locator across workspace files, Blaze standard modules, and built-ins.
- [`ide/syntaxes/flame.tmLanguage.json`](file:///home/spidey/Projects/flame/ide/syntaxes/flame.tmLanguage.json): TextMate grammar for VSCode syntax highlighting.
- [`ide/snippets/flame.json`](file:///home/spidey/Projects/flame/ide/snippets/flame.json): VSCode snippet templates.
- [`ide/package.json`](file:///home/spidey/Projects/flame/ide/package.json): Extension manifest defining semantic token scopes and themes.

#### 9. [`src/cli/`](file:///home/spidey/Projects/flame/src/cli) — Command Line Interface

- [`src/cli/commands/run.rs`](file:///home/spidey/Projects/flame/src/cli/commands/run.rs): `flame run` and `flame run --watch` file execution.
- [`src/cli/commands/build.rs`](file:///home/spidey/Projects/flame/src/cli/commands/build.rs): `flame build` binary compilation and plugin policy checks.
- [`src/cli/commands/test.rs`](file:///home/spidey/Projects/flame/src/cli/commands/test.rs): `flame test` test runner.
- [`src/cli/commands/project.rs`](file:///home/spidey/Projects/flame/src/cli/commands/project.rs): `flame new`, `flame package`, and scaffold generation with default `flame.toml` templates.
- [`src/cli/commands/system.rs`](file:///home/spidey/Projects/flame/src/cli/commands/system.rs): `flame doctor`, `flame update`, `flame uninstall`.
- [`src/cli/commands/help.rs`](file:///home/spidey/Projects/flame/src/cli/commands/help.rs): CLI help menus and descriptions.
- [`src/cli/ide.rs`](file:///home/spidey/Projects/flame/src/cli/ide.rs): Language server backend (`flame check --json`, `flame definition --json`, `flame complete --json`).

#### 10. [`src/utils/`](file:///home/spidey/Projects/flame/src/utils) — Shared Utilities

- [`src/utils/manifest.rs`](file:///home/spidey/Projects/flame/src/utils/manifest.rs): Manifest engine:
  - `find_manifest_root(start_dir)`: Traverses parent directories to locate `flame.toml`.
  - `parse_manifest_section(content, section)`: Extracts key-value mappings from TOML sections.
  - `get_rust_plugins_mode(manifest_root)`: Returns `"default"`, `"warn"`, or `"deny"`.
  - `is_rust_plugins_denied(manifest_root)` & `is_rust_plugins_warn(manifest_root)`.
  - `get_closure_types_mode(manifest_root)`: Returns `"default"` or `"strict"`.
  - `is_strict_closure_types(manifest_root)`.
  - `get_declared_plugins(manifest_root)`: Set of declared plugin identifiers.
- [`src/utils/format.rs`](file:///home/spidey/Projects/flame/src/utils/format.rs): Byte formatting, transfer speed formatting, table cleanups.
- [`src/utils/fs.rs`](file:///home/spidey/Projects/flame/src/utils/fs.rs): Recursive directory copying (`copy_dir_all`).
- [`src/utils/text.rs`](file:///home/spidey/Projects/flame/src/utils/text.rs): Comment and string stripping, balanced bracket extractors.

#### 11. [`src/web/`](file:///home/spidey/Projects/flame/src/web) & [`wasm/`](file:///home/spidey/Projects/flame/wasm) — Fullstack Web & WebAssembly

- [`src/web/compiler.rs`](file:///home/spidey/Projects/flame/src/web/compiler.rs): Fullstack web compiler converting Flame frontend and backend components into reactive JS/HTML/CSS assets.
- [`src/web/server.rs`](file:///home/spidey/Projects/flame/src/web/server.rs): Embedded development server for web applications.
- [`wasm/`](file:///home/spidey/Projects/flame/wasm): WebAssembly compilation target enabling Flame in the browser playground.

#### 12. [`docs/`](file:///home/spidey/Projects/flame/docs) — Documentation Website

- Built with Astro and Starlight.
- Content is located in [`docs/src/content/docs/`](file:///home/spidey/Projects/flame/docs/src/content/docs).
- Navigation sidebar is configured in [`docs/astro.config.mjs`](file:///home/spidey/Projects/flame/docs/astro.config.mjs).
- Syntax highlighting uses Shiki with TextMate grammar loaded from [`docs/src/syntax/flame.tmLanguage.json`](file:///home/spidey/Projects/flame/docs/src/syntax/flame.tmLanguage.json).

---

## Core Architectural Rules & Conventions

Before making changes, familiarize yourself with these core language rules and architectural conventions:

### Pattern Matching Syntax: Destructuring with Parentheses

In Flame, pattern matching destructuring must always use parentheses `()`. Destructuring with curly braces `{}` is strictly forbidden:

```flame
// CORRECT: Destructuring using parentheses ()
match res {
    Option.Some(value) => println("Value: " + value),
    Option.None => println("No value"),
}

match command {
    1(value) => println("Matched variant 1 with: " + value),
    _ => println("Other"),
}

// INCORRECT: Curly braces {} are NOT supported in pattern destructuring
// match res { 1 {value} => ... }  <-- Compile error!
```

### Enums and Struct Implementations

- **Enums**: Enums represent algebraic data types with optional variant payloads:
  ```flame
  enum Status {
      Pending,
      Success(Str),
      Failed(Int, Str),
  }
  ```
- **Implementations**: In Flame, `impl` blocks only apply to structs. Enums cannot have `impl` blocks:
  ```flame
  struct Point {
      x: Num,
      y: Num,
  }

  impl Point {
      fn new(x: Num, y: Num) -> Point {
          return Point { x: x, y: y };
      }
  }
  ```

### IDE Styling: TextMate Grammar vs Semantic Tokens

Flame uses a hybrid styling architecture to ensure crisp syntax coloring in VSCode:

1. **TextMate Grammar ([`ide/syntaxes/flame.tmLanguage.json`](file:///home/spidey/Projects/flame/ide/syntaxes/flame.tmLanguage.json))**:
   - Primary authority for static tokens, keywords, operators, and annotations.
   - Annotations (`@Annotation` and `annotation Name`) are scoped to `keyword.control.flame`, rendering them in **red** (matching control flow keywords).
   - Annotations must **never** be colored like functions.
2. **Semantic Tokens ([`src/ide/semantic_tokens.rs`](file:///home/spidey/Projects/flame/src/ide/semantic_tokens.rs))**:
   - Used exclusively for dynamically declared workspace types.
   - Declared enums and their variants (e.g. `Option`, `Result`, `Some`, `None`, and user-defined enums) are tokenized as token type 3 (`enum`) and token type 10 (`enumMember`), which map to `constant.language.flame` and `variable.other.enummember.flame` in [`ide/package.json`](file:///home/spidey/Projects/flame/ide/package.json), rendering them in **blue**.
   - **Crucial Rule**: The semantic token scanner must **never** emit tokens for `@` annotations to prevent overriding the TextMate red control styling with function or variable colors.

### Compiler Diagnostics Architecture

All user-facing compiler errors and warnings should use the centralized diagnostic engine in [`src/diagnostics.rs`](file:///home/spidey/Projects/flame/src/diagnostics.rs). Avoid printing raw error strings to stderr.

- Use `Diagnostic::new_error` for fatal compilation/typechecking failures.
- Use `Diagnostic::new_warning` for non-fatal issues (such as `rust-plugins = "warn"`).
- Always attach a precise source `Span`, a contextual `label`, and when possible, an actionable `suggestion` and explanatory `note`.

---

## Getting Started as a Contributor

### Prerequisites

- [Rust](https://rustup.rs/) (version 1.80+ or latest stable)
- [Node.js](https://nodejs.org/) (v18+ for documentation and IDE development)
- `git`

### Building and Running Locally

Clone the repository:

```bash
git clone https://github.com/shoya-129/flame.git
cd flame
```

Compile the project:

```bash
cargo check
cargo build
```

Run the Flame CLI:

```bash
cargo run -- --help
cargo run -- doctor
cargo run -- run examples/src/main.fm
```

### Running the Test Suite

Always run the full test suite before committing changes:

```bash
cargo test
```

You can also run tests for a specific module:

```bash
cargo test typechecker
cargo test runner
cargo test parser
cargo test ide
```

---

## How to Add Features: Step-by-Step Guides

### Guide 1: Adding a New Syntax Element or Keyword

When adding a new keyword or syntax construct (e.g. a `repeat` loop or a new expression operator):

1. **Tokenize in [`src/lexer.rs`](file:///home/spidey/Projects/flame/src/lexer.rs)**:
   - Add the token variant to `TokenKind` (e.g. `TokenKind::Repeat`).
   - In `Lexer::next_token()`, recognize the symbol or add it to the keyword lookup table.

2. **Define AST Node in [`src/parser/ast.rs`](file:///home/spidey/Projects/flame/src/parser/ast.rs)**:
   - Add a variant to `Expr` or `Stmt` (e.g., `Stmt::RepeatStmt { count: Expr, body: Vec<Stmt>, span: Span }`).
   - Implement the `span()` method for the new variant.

3. **Parse in [`src/parser/parser.rs`](file:///home/spidey/Projects/flame/src/parser/parser.rs)**:
   - Add parsing logic in `Parser::parse_statement()` or `Parser::parse_expression()`.
   - On syntax errors, emit rich diagnostics using `Diagnostic::new_error`.

4. **Typecheck in [`src/typechecker/`](file:///home/spidey/Projects/flame/src/typechecker)**:
   - For statements: add a check in [`src/typechecker/stmts.rs`](file:///home/spidey/Projects/flame/src/typechecker/stmts.rs) (`check_stmt`).
   - For expressions: add an inference branch in [`src/typechecker/exprs.rs`](file:///home/spidey/Projects/flame/src/typechecker/exprs.rs) (`infer_expr_type`).
   - Validate types with `self.expect_assignable(...)` or `self.is_compatible(...)`.

5. **Execute in [`src/runner/`](file:///home/spidey/Projects/flame/src/runner)**:
   - For statements: handle execution in [`src/runner/stmts.rs`](file:///home/spidey/Projects/flame/src/runner/stmts.rs) (`execute_statement`).
   - For expressions: handle evaluation in [`src/runner/exprs.rs`](file:///home/spidey/Projects/flame/src/runner/exprs.rs) (`eval_expr`).

6. **IDE & Documentation**:
   - Register hover help and completion snippet in `const KEYWORDS` in [`src/ide/keywords.rs`](file:///home/spidey/Projects/flame/src/ide/keywords.rs).
   - Add keyword patterns to [`ide/syntaxes/flame.tmLanguage.json`](file:///home/spidey/Projects/flame/ide/syntaxes/flame.tmLanguage.json).

7. **Add Unit Tests**:
   - Add typechecking tests in [`src/typechecker/tests.rs`](file:///home/spidey/Projects/flame/src/typechecker/tests.rs).
   - Add runtime execution tests in [`src/runner/tests.rs`](file:///home/spidey/Projects/flame/src/runner/tests.rs).

---

### Guide 2: Adding a Standard Library (`std`) Module

You can add standard library functionality either in native Rust or in pure Flame:

#### Option A: Native Rust Module ([`src/native_std/`](file:///home/spidey/Projects/flame/src/native_std))

1. **Create module file in `src/native_std/<module_name>.rs`**:
   ```rust
   use crate::vm::Value;
   use std::collections::HashMap;

   pub fn init() -> HashMap<String, Value> {
       let mut m = HashMap::new();
       m.insert(
           "hash".to_string(),
           Value::NativeCallback(|args| {
               let input = match args.get(0) {
                   Some(Value::String(s)) => s,
                   _ => return Err("Expected string argument".to_string()),
               };
               Ok(Value::String(format!("hashed_{}", input)))
           }),
       );
       m
   }
   ```

2. **Register in [`src/native_std/mod.rs`](file:///home/spidey/Projects/flame/src/native_std/mod.rs)**:
   - Add `pub mod <module_name>;`.

3. **Register in [`src/stdlib.rs`](file:///home/spidey/Projects/flame/src/stdlib.rs)**:
   - In `register_std_module()`:
     ```rust
     "std.<module_name>" => Some(crate::native_std::<module_name>::init()),
     ```

4. **Register types in [`src/typechecker/builtins.rs`](file:///home/spidey/Projects/flame/src/typechecker/builtins.rs)**:
   - Add function signatures to `get_std_module_type()`.

5. **Register IDE discovery**:
   - Add module to `list_std_modules()` in [`src/cli/ide.rs`](file:///home/spidey/Projects/flame/src/cli/ide.rs).
   - Add method autocompletions in `get_std_module_methods()` in [`src/ide/modules.rs`](file:///home/spidey/Projects/flame/src/ide/modules.rs).

#### Option B: Pure Flame Module ([`Blaze/std/`](file:///home/spidey/Projects/flame/Blaze/std))

1. Create `<module_name>.fm` in [`Blaze/std/`](file:///home/spidey/Projects/flame/Blaze/std).
2. Export structs, functions, or constants using `export`:
   ```flame
   export fn compute(val: Int) -> Int {
       return val * 2;
   }
   ```
3. Users can import it directly via `import std.<module_name>`.

---

### Guide 3: Adding or Modifying `flame.toml` Options (`[options]`)

Flame allows projects to configure compiler and runtime behaviors through the `[options]` table in `flame.toml`.

#### 1. Manifest Helpers in [`src/utils/manifest.rs`](file:///home/spidey/Projects/flame/src/utils/manifest.rs)

Add helper functions to query the option:

```rust
pub fn get_my_option(manifest_root: &Path) -> String {
    let manifest_path = manifest_root.join("flame.toml");
    if let Ok(content) = std::fs::read_to_string(&manifest_path) {
        let options = parse_manifest_section(&content, "options");
        if let Some(val) = options.get("my-option") {
            return val.trim_matches('"').trim_matches('\'').to_string();
        }
    }
    "default".to_string()
}
```

#### 2. Compiler & Typechecker Enforcement

- In [`src/typechecker/stmts.rs`](file:///home/spidey/Projects/flame/src/typechecker/stmts.rs) or [`exprs.rs`](file:///home/spidey/Projects/flame/src/typechecker/exprs.rs), locate the manifest root with `find_manifest_root()` and read the option value.
- If the option is violated, emit an error or warning using [`src/diagnostics.rs`](file:///home/spidey/Projects/flame/src/diagnostics.rs).

#### 3. CLI & Runtime Enforcement

- In [`src/cli/commands/run.rs`](file:///home/spidey/Projects/flame/src/cli/commands/run.rs) and [`build.rs`](file:///home/spidey/Projects/flame/src/cli/commands/build.rs), enforce option policies before compiling or executing.
- In [`src/runner/plugins.rs`](file:///home/spidey/Projects/flame/src/runner/plugins.rs), verify runtime permissions.

#### 4. Project Scaffolding in [`src/cli/commands/project.rs`](file:///home/spidey/Projects/flame/src/cli/commands/project.rs)

Update the project generation template so `flame new` generates clean, commented configuration:

```toml
[options]
# Closure typing mode: "default" allows dynamic types; "strict" enforces full parameter types
closure-types = "default"
# Rust native plugin policy: "default" allows native plugins; "warn" permits with warning; "deny" blocks execution
rust-plugins = "default"
```

#### 5. Documentation in [`docs/src/content/docs/packages-and-native/flame-toml.mdx`](file:///home/spidey/Projects/flame/docs/src/content/docs/packages-and-native/flame-toml.mdx)

Document the new option, all allowed values, default behaviors, and usage examples.

---

### Guide 4: Emitting Rich Compiler Diagnostics

To format clear, actionable diagnostic output:

```rust
use crate::diagnostics::Diagnostic;
use crate::lexer::Span;

// Emitting a compiler error:
let diag = Diagnostic::new_error(
    "Native Rust plugins are denied by project configuration".to_string(),
    filepath.clone(),
    span,
    Some("import of native plugin denied here".to_string()),
    Some("remove this import or change `rust-plugins` in flame.toml".to_string()),
)
.with_note("Project flame.toml has `rust-plugins = \"deny\"` in [options].".to_string());

diag.print(&source);

// Emitting a compiler warning:
let warn_diag = Diagnostic::new_warning(
    "Native Rust plugin imported while under warning mode".to_string(),
    filepath.clone(),
    span,
    Some("native plugin imported here".to_string()),
    None,
)
.with_note("Set `rust-plugins = \"deny\"` to strictly disallow native plugins.".to_string());

warn_diag.print(&source);
```

---

### Guide 5: Adding Built-in Annotations & Syntax Styling

1. **AST Representation**:
   - Annotations are parsed as `Annotation { name, args, span, name_span }` in [`src/parser/ast.rs`](file:///home/spidey/Projects/flame/src/parser/ast.rs).
2. **Typechecker Validation**:
   - In [`src/typechecker/checker.rs`](file:///home/spidey/Projects/flame/src/typechecker/checker.rs), validate annotation placement, allowed target definitions, and argument count.
3. **Runtime Execution**:
   - In [`src/runner/stmts.rs`](file:///home/spidey/Projects/flame/src/runner/stmts.rs) or [`callbacks.rs`](file:///home/spidey/Projects/flame/src/runner/callbacks.rs), inspect annotations attached to statements or declarations.
4. **Syntax Highlighting (Red Control Scope)**:
   - Ensure the annotation regex in [`ide/syntaxes/flame.tmLanguage.json`](file:///home/spidey/Projects/flame/ide/syntaxes/flame.tmLanguage.json) maps `@Annotation` to `keyword.control.flame`.
   - **Do not** add annotation tokens to [`src/ide/semantic_tokens.rs`](file:///home/spidey/Projects/flame/src/ide/semantic_tokens.rs); they must retain red control coloring.
5. **IDE Hover & Autocomplete**:
   - Add the annotation description and examples to `const KEYWORDS` in [`src/ide/keywords.rs`](file:///home/spidey/Projects/flame/src/ide/keywords.rs).

---

### Guide 6: Working on the IDE Extension & Syntax Highlighting

The VSCode extension lives in the [`ide/`](file:///home/spidey/Projects/flame/ide) directory.

1. **TextMate Grammar ([`ide/syntaxes/flame.tmLanguage.json`](file:///home/spidey/Projects/flame/ide/syntaxes/flame.tmLanguage.json))**:
   - Defines syntax highlighting rules for comments, strings, keywords, numbers, types, and annotations.
   - When updating grammar rules, copy the updated JSON file to [`docs/src/syntax/flame.tmLanguage.json`](file:///home/spidey/Projects/flame/docs/src/syntax/flame.tmLanguage.json) to keep documentation code blocks in sync.
2. **Semantic Tokens ([`src/ide/semantic_tokens.rs`](file:///home/spidey/Projects/flame/src/ide/semantic_tokens.rs))**:
   - Emits dynamic tokens for enums and enum variants (colored blue in VSCode via `constant.language.flame` / `variable.other.enummember.flame`).
3. **Snippets ([`ide/snippets/flame.json`](file:///home/spidey/Projects/flame/ide/snippets/flame.json))**:
   - Define code snippets for functions, structs, match statements, and annotations.
4. **Testing Extension Locally**:
   - Open [`ide/`](file:///home/spidey/Projects/flame/ide) in VSCode and launch the Extension Development Host (`F5`).

---

### Guide 7: Adding or Modifying a CLI Subcommand

Flame's CLI dispatcher is in [`src/main.rs`](file:///home/spidey/Projects/flame/src/main.rs), and subcommands live in [`src/cli/commands/`](file:///home/spidey/Projects/flame/src/cli/commands):

1. **Create/Update subcommand in `src/cli/commands/<command>.rs`**:
   ```rust
   pub fn run_my_command(args: &[String]) {
       println!("Executing custom command!");
   }
   ```
2. **Export in [`src/cli/commands/mod.rs`](file:///home/spidey/Projects/flame/src/cli/commands/mod.rs)**:
   ```rust
   pub mod my_command;
   pub use my_command::*;
   ```
3. **Dispatch in [`src/main.rs`](file:///home/spidey/Projects/flame/src/main.rs)**:
   ```rust
   "my-command" => {
       run_my_command(&args[2..]);
   }
   ```
4. **Update Help Menu in [`src/cli/commands/help.rs`](file:///home/spidey/Projects/flame/src/cli/commands/help.rs)**:
   - Add command syntax and description to `print_help()`.

---

### Guide 8: Contributing to Documentation (`docs/`)

The Flame documentation website is built with Astro and Starlight.

1. **Install Dependencies & Run Dev Server**:
   ```bash
   cd docs
   npm install
   npm run dev
   ```
   Open `http://localhost:4321` in your browser.

2. **Add or Edit Content**:
   - Documentation articles live in [`docs/src/content/docs/`](file:///home/spidey/Projects/flame/docs/src/content/docs).
   - Use `.mdx` or `.md` format.
   - Code blocks with ` ```flame ` will automatically use the Flame Shiki grammar.

3. **Configure Navigation**:
   - Add new pages to the sidebar configuration in [`docs/astro.config.mjs`](file:///home/spidey/Projects/flame/docs/astro.config.mjs).

---

### Guide 9: Adding Common Utility Functions

To keep the codebase modular and DRY:

- Do **not** duplicate file discovery, TOML parsing, string formatting, or directory copying across modules.
- Place shared functions in [`src/utils/`](file:///home/spidey/Projects/flame/src/utils):
  - [`src/utils/manifest.rs`](file:///home/spidey/Projects/flame/src/utils/manifest.rs) for project root detection and TOML section reading.
  - [`src/utils/format.rs`](file:///home/spidey/Projects/flame/src/utils/format.rs) for byte, speed, and table formatters.
  - [`src/utils/fs.rs`](file:///home/spidey/Projects/flame/src/utils/fs.rs) for recursive file operations.
  - [`src/utils/text.rs`](file:///home/spidey/Projects/flame/src/utils/text.rs) for code string parsing and balanced bracket extractors.
- Re-export them in [`src/utils/mod.rs`](file:///home/spidey/Projects/flame/src/utils/mod.rs) so any module can access them via `crate::utils::*`.

---

## Testing & Quality Standards

1. **Zero Compiler Warnings**: All code must compile cleanly under `cargo check`.
2. **Preserve Compatibility**: Public APIs in `mod.rs` files should maintain backward compatibility.
3. **Write Unit Tests**: For any new language feature or bug fix:
   - Add typechecker tests in [`src/typechecker/tests.rs`](file:///home/spidey/Projects/flame/src/typechecker/tests.rs).
   - Add runtime execution tests in [`src/runner/tests.rs`](file:///home/spidey/Projects/flame/src/runner/tests.rs).
4. **Run Full Test Suite**:
   ```bash
   cargo test
   ```
   All existing and new tests must pass without regressions.

---

## Submitting a Pull Request

1. **Fork & Branch**: Create a feature branch with a descriptive name (`git checkout -b feature/my-feature`).
2. **Verify Locally**:
   ```bash
   cargo check
   cargo test
   ```
3. **Commit Messages**: Use clear, conventional commit messages (e.g. `feat: add repeat loop syntax`, `fix: enforce pattern matching destructuring with parentheses`).
4. **Open PR**: Submit your pull request to the `main` branch with a description of the problem solved, architectural considerations, and test coverage added.

Thank you for helping build Flame! 🔥
