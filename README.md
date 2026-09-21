<h1 style="display:flex; align-items: center; gap: 8px;">
  <img src="./docs/public/flame.png" alt="Flame Logo" width="46" height="46" style="vertical-align: middle;" />
  Flame
</h1>

Flame is a statically typed, compiled programming language with **application-specific native runtimes**. Powered by the Blaze compiler and native Rust/Cargo/LLVM toolchain, Flame generates tailored native runtimes containing only the capabilities, packages, and native plugins your application actually imports and uses.

---

## Core Architectural Principle

> **Every Flame application receives a specialized native runtime containing only the runtime capabilities, packages, and native implementations actually required by that application.**
>
> Flame analyzes the application's imports and dependencies, resolves the required package `.fmi` interfaces, and constructs the appropriate runtime. The resulting runtime is built as native code through Rust, Cargo, and LLVM, allowing Flame to maintain a small, dependency-specific runtime and an optimized native binary.
>
> Normal builds use the real filesystem. VFS is used only when the developer explicitly requests a self-contained single executable.

---

## Why Flame?
Flame is designed to sit between high-level developer experience and native systems programming.

Write expressive Flame code while enjoying seamless access to native Rust crates, native plugins, and an application-specific runtime compiled into an optimized native executable.

---

## Architecture: True Multithreading & High Concurrency
Flame avoids the slow, bottlenecked single-threaded event loops common in legacy runtimes like Node.js:
- **True Multithreaded Execution**: Powered by Rust's native OS threads and multi-core **Tokio worker pools**, background computation and network tasks execute concurrently across all available CPU cores.
- **Execution vs. Concurrency Separation**: Instead of expensively cloning entire VM interpreter states per network request (which leads to state divergence and memory overhead), parallel worker pools package incoming requests and communicate over atomic message queues to a single deterministic Flame runtime engine.
- **Persistent Server Daemons**: The Flame runtime automatically engages multi-threaded daemon persistence whenever network bindings or async listeners (like Axum/Tokio) are active, keeping your server running responsive without arbitrary loops.

---

## Core Features
- **Application-Specific Native Runtimes**: Flame creates a specialized native runtime for each application, containing only the imported modules and plugins. No universal runtime bloat.
- **Native Dependencies & Local Plugins**: Add public Rust crates or build custom local Rust libraries (`./native`) directly into your project. Flame automatically inspects Rust interfaces to export struct types and signatures to JSON `.fmi` bindings!
- **World-Class IDE Intellisense**: The Flame VS Code Extension uses native `.fmi` definitions to provide rich type hover info, doc comments, and method autocompletion for standard and local native plugins.
- **Lightweight Package Manager**: Built-in dependency management capable of resolving packages cleanly (`fmp install`) without requiring developers to manually copy Rust code.

---

## Installation

### Quick Install (Recommended)

Choose your operating system and preferred shell to install Flame and the Blaze toolchain:

#### Windows (PowerShell)
Run in Windows PowerShell (Standard or Administrator):

```powershell
irm https://raw.githubusercontent.com/shoya-129/flame/main/install.ps1 | iex
```

Or from a local cloned repository:

```powershell
.\install.ps1
```

#### Linux & macOS (Bash)
Run the universal bash script (Linux, macOS, and Windows via Git Bash / WSL / MSYS):

```bash
curl -fsSL https://raw.githubusercontent.com/shoya-129/flame/main/install.sh | bash
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
- **`fmp build --release`**: Produces an optimized, production-grade standalone executable inside `target/release/` (uses normal filesystem, no VFS). Configures LLVM for maximum performance with full optimization (`opt-level = 3`), fat Link Time Optimization (`lto = "fat"`), single code generation unit across all crates (`codegen-units = 1`), stripped symbol tables (`strip = true`), and zero-overhead aborting panics (`panic = "abort"`).
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
- **`fmp native init [plugin_name]`**: Initialize a native Rust plugin workspace (`./native`) inside your current Flame project, generating a starter `Cargo.toml`, bridge code, and automatically registering the specified plugin name in your `flame.toml`.

---

## Documentation
Dive into the `docs` folder for detailed guides on language syntax, multithreaded ownership models, native crate integration, and plugin design:
- [Multithreaded Architecture & Safety](https://flamelang.vercel.app/concurrency/threads-and-channels/)
- [Asynchronous Concurrency & Network I/O](https://flamelang.vercel.app/concurrency/async-await/)
- [Developing Local Plugins & FFI](https://flamelang.vercel.app/packages-and-native/native-plugins/)
- [Using Native Rust Crates](https://flamelang.vercel.app/packages-and-native/native-rust-crates/)
- [Changelog & Version Codenames](./CHANGELOG.md)
  
## License
ISC
