<h1 style="display:flex; align-items: center; gap: 8px;">
  <img src="./docs/public/flame.png" alt="Flame Logo" width="46" height="46" style="vertical-align: middle;" />
  Flame
</h1>

[![License: ISC](https://img.shields.io/badge/License-ISC-blue.svg?style=flat-square)](https://opensource.org/licenses/ISC)
[![Documentation](https://img.shields.io/badge/docs-flamelang.vercel.app-ff4500?style=flat-square&logo=gitbook&logoColor=white)](https://flamelang.vercel.app)


Flame is a type-safe, simpler programming and scripting language built to **work alongside Rust, not replace it**. Powered by the **Blaze compiler** and native Rust/Cargo/LLVM toolchain, Flame generates **application-specific native runtimes** containing only the capabilities, packages, and native plugins your application actually imports and uses.

---

## The Vision: Built to Work Alongside Rust

Maintaining a large, monolithic Rust codebase is notoriously demanding. While Rust is unmatched for systems programming and low-level control, writing high-level application logic, glue code, rapid prototypes, dynamic configurations, or scripting in low-level Rust often leads to:
- Steep compile times and lengthy linking pauses.
- Borrow checker friction for straightforward, high-level business tasks.
- Heavy boilerplate for simple data manipulation and glue workflows.

> **Keep your core performance algorithms, hardware drivers, and low-level primitives in Rust. Use Flame for simpler application layers, rapid prototyping, and scripting where you need simplicity and developer velocity.**

Flame adopts a Rust-inspired object model and type system:
- **Familiar OOP**: Defines types using `struct` and methods using `impl`, with explicit `&self` receivers.
- **Borrowing & References**: Employs compile-time ownership, borrowing, and reference checks inspired by Rust, avoiding the latency spikes of a garbage collector (GC).
- **Super Embeddable**: Embed Flame scripts or complete packages directly into Rust host applications, game engines, and microservices using the `flamebinder` crate.
- **Smart ABI Plugin System**: Directly call native Rust crates and plugins using `.fmi` interface contracts without cumbersome manual FFI glue.
- **Batteries-Included Standard Library**: Provides rich out-of-the-box standard library modules (`std.fs`, `std.net.http`, `std.json`, `std.thread`) for real-world tasks.

---

## What Flame Is vs. What Flame Isn't

| What Flame Is | What Flame Isn't |
| :--- | :--- |
| **A simpler, type-safe language built to work alongside Rust** | **Not a Rust replacement** (not meant for kernel-level or micro-optimized low-level systems) |
| **Connected via a typed Smart ABI plugin system (`.fmi`)** | **Not claiming unqualified "zero-cost" interop** (FFI is statically linked and typed, but FFI boundaries exist) |
| **Memory-safe via ownership, borrows, and references** | **Not a garbage-collected language** (no GC runtime, no stop-the-world pauses) |
| **Constructs application-specific native runtimes** | **Not a universal bytecode VM or monolithic interpreter** (like Node.js, Python, or the JVM) |
| **Uses Rust, Cargo, and LLVM as its native build pipeline** | **Not a source-to-source Rust transpiler** |
| **Super embeddable into Rust applications** via `flamebinder` | **Not an isolated language silo** (designed for seamless multi-language cooperation) |
| **Rich standard library** for real-world tasks | **Not an MVP without real-world libraries** (includes HTTP, JSON, FS, threads, and async) |

> [!NOTE]
> **What "Runtime" Means in Flame:**
> Flame does not use a traditional bytecode VM or universal interpreter. “Runtime” refers to the native runtime components required by the application, which are compiled into the final native binary.

---

## Core Architectural Principle

> **Every Flame application receives a specialized native runtime containing only the runtime capabilities, packages, and native implementations actually required by that application.**
>
> Flame analyzes the application's imports and dependencies, resolves the required package `.fmi` interfaces, and constructs the appropriate runtime. The resulting runtime is built as native code through Rust, Cargo, and LLVM, allowing Flame to maintain a small, dependency-specific runtime and an optimized native binary.
>
> Normal builds use the real filesystem. VFS is used only when the developer explicitly requests a self-contained single executable.

### Compilation Pipeline

```text
             Flame source
                  ↓
                Blaze
                  ↓
          Dependency analysis
                  ↓
     ┌─────────────────────────┐
     │ Application-specific    │
     │ native runtime          │
     │                         │
     │ only required pieces    │
     └─────────────────────────┘
                  ↓
             Rust / Cargo
                  ↓
                LLVM
                  ↓
            Native binary
```

---

## Concurrency: Multi-Threaded Tokio & OS Compute Threads

Flame provides structured concurrency separating non-blocking I/O from compute-intensive operations:
- **Asynchronous Non-Blocking I/O**: Powered by Rust's multi-threaded **Tokio runtime**, network requests, file I/O, and socket tasks execute concurrently across worker threads without blocking OS threads.
- **Dedicated Compute Threads**: CPU-bound tasks can be offloaded to dedicated OS threads via `thread { ... }`, preventing heavy compute from starving network workers.
- **Thread Communication**: Results and event payloads are passed safely between concurrent threads using typed channels.

---

## Core Features
- **Application-Specific Native Runtimes**: Flame creates a specialized native runtime for each application, containing only the imported modules and plugins. No universal runtime bloat.
- **Native Dependencies & Local Plugins**: Add public Rust crates or build custom local Rust libraries (`./native`) directly into your project. Flame automatically inspects Rust interfaces to export struct types and signatures to `.fmi` bindings.
- **World-Class IDE Intellisense**: The Flame VS Code Extension uses native `.fmi` definitions to provide rich type hover info, doc comments, and method autocompletion for standard and local native plugins.
- **Lightweight Package Manager**: Built-in dependency management capable of resolving packages cleanly (`fmp install`) without requiring developers to manually copy Rust code.

---

## Installation

### Quick Install (Recommended)

Choose your operating system and preferred shell to install Flame and the Blaze toolchain:

#### Windows (PowerShell)
Run in Windows PowerShell (Standard or Administrator):

```powershell
irm https://raw.githubusercontent.com/sohamglx/flame/main/install.ps1 | iex
```

Or from a local cloned repository:

```powershell
.\install.ps1
```

#### Linux & macOS (Bash)
Run the universal bash script (Linux, macOS, and Windows via Git Bash / WSL / MSYS):

```bash
curl -fsSL https://raw.githubusercontent.com/sohamglx/flame/main/install.sh | bash
```

Or from a local cloned repository:

```bash
bash install.sh
```

#### Alternative: Cargo & npm
You can also install the binary directly from package registries:

```bash
# Via Cargo
cargo install --force flamelang

# Via npm
npm i -g flamelang
```

> [!TIP]
> **What the installer does:**
> The installer automatically builds `flame` (and `flamelang`), registers the `fmp` binary command in your permanent User PATH, and provisions the canonical `Blaze/std` definition interface directory into your local application data folder (`%LOCALAPPDATA%\Blaze` on Windows or `~/.blaze` on Unix) for instant Go-to-Definition and IDE LSP support.

---

## Verifying the Installation

Check that `fmp` is accessible from your terminal:

```bash
fmp --version
```

To view the complete help menu and all available flags:

```bash
fmp --help
```

---

## CLI & Toolchain Reference

The `fmp` (or `flamelang`) binary provides an all-in-one developer workspace toolkit:

### 1. Running & Building Code
- **`fmp install`**: Resolves and downloads dependencies declared in `flame.toml`, caching packages in `.flame/pkg/` and generating required `.fmi` interface metadata for native plugins.
- **`fmp run <file.fm> [--local]`**: Run a Flame script directly using the interactive compiler engine. When `--local` is specified, local plugins and static native bridges are compiled and executed in real time.
- **`fmp build [entry.fm]`**: Constructs an application-specific native runtime and compiles it into a dev executable inside `target/dev/` (uses normal filesystem, no VFS).
- **`fmp build --release`**: Produces an optimized, production-grade standalone executable inside `target/release/` (uses normal filesystem, no VFS). Configures LLVM with full optimization (`opt-level = 3`), fat Link Time Optimization (`lto = "fat"`), single code generation unit across all crates (`codegen-units = 1`), stripped symbol tables (`strip = true`), and aborting panics (`panic = "abort"`).
- **`fmp build --vfs` / `fmp build --vfs --release`**: Single-executable packaging mode where the application and package files are embedded directly into the binary via VFS.
- **`fmp <file.fm>`**: Quick-exec shorthand to parse and run any `.fm` source file.

### 2. Diagnostics & IDE Integration (`check --json`)
- **`fmp check <file.fm> [--json] [--line N --col N]`**: Performs instantaneous syntactic analysis, type inference, and static diagnostics without compiling or linking binaries.
- **Structured JSON Output**: When invoked with `--json`, it emits a comprehensive machine-readable payload used directly by the Flame VS Code Extension and Language Server Protocol (LSP):
  - **Precision Hover Metadata**: Passing `--line N --col N` instructs the type checker to resolve the exact AST node or symbol under the cursor. It reports inferred primitive types, struct definitions from native `.fmi` bridges, and identifies native AOT packages as `plugin` (e.g., `server: plugin`) rather than regular standard library modules.
  - **Intellisense & Autocomplete**: Extracts available methods, parameters, docstrings, and struct signatures from both standard libraries (`std.*`) and compiled native plugin interfaces (`native.*`).
  - **Real-time Diagnostics**: Returns syntax errors, undefined references, and type mismatches with exact line, column, and severity mappings.

**Example Usage & JSON Payload:**
```bash
fmp check automation/src/main.fm --json --line 47 --col 18
```

```json
{
  "file": "automation/src/main.fm",
  "diagnostics": [],
  "std_modules": ["thread", "process", "fs", "math", "time", "os", "env"],
  "native_modules": ["server"],
  "plugins": [
    {
      "name": "server",
      "source": "./native",
      "version": null,
      "is_local": true
    }
  ],
  "completions": [],
  "hover": {
    "label": "server",
    "documentation": "```flame\nserver: plugin\n```\nInferred type from AST"
  }
}
```

### 3. Code Formatting
- **`fmp format <file.fm>`**: Automatically reformats your source code to adhere to Flame's canonical formatting rules (clean indentation, comment preservation, and exact operator spacing without extraneous padding around dot notation or module accesses like `thread.sleep()`).
- **`fmp format --all`** (or `fmp format` directly in a workspace): Formats all `.fm` and `.flame` source files across your repository.

### 4. Package & Plugin Management
- **`fmp new <name>`**: Initialize a new Flame workspace directory with a ready-to-run manifest and directory structure.
- **`fmp add <url_or_name>`**: Download and register external Flame modules or repositories.
- **`fmp add --plugin <path_to_plugin> [--name <plugin_name>]`**: Register a local or external native Rust plugin inside your `flame.toml` manifest. If `--name` is omitted, Flame automatically discovers the plugin name by inspecting its `Cargo.toml` or extracting the directory name!
- **`fmp new -p [plugin_name]`**: Initialize a native Rust plugin workspace (`./native`) inside your current Flame project, generating a starter `Cargo.toml`, bridge code, and automatically registering the specified plugin name in your `flame.toml`.

---

## Documentation

Full documentation, tutorials, and API guides are available at **[https://flamelang.vercel.app](https://flamelang.vercel.app)**:
- [Introduction to Flame](https://flamelang.vercel.app/getting-started/introduction/)
- [Architecture & Analysis](https://flamelang.vercel.app/getting-started/architecture/)
- [Asynchronous Concurrency (async / await)](https://flamelang.vercel.app/concurrency/async-await/)
- [Threads & Concurrency](https://flamelang.vercel.app/concurrency/threads-and-channels/)
- [Flame Binder (Rust Integration)](https://flamelang.vercel.app/packages-and-native/rust-integration/)
- [Developing Local Plugins & FFI](https://flamelang.vercel.app/packages-and-native/native-plugins/)
- [Using Native Rust Crates](https://flamelang.vercel.app/packages-and-native/native-rust-crates/)
- [Changelog & Version Codenames](./CHANGELOG.md)
  
## License
ISC
