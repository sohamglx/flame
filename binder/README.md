# flamebinder

`flamebinder` is the official Rust integration crate for the **Flame** programming language.

Flame is designed to be the ultimate embedding and metaprogramming language for Rust applications. Unlike Lua, which lacks static typing and a structured memory model, Flame provides **static typing**, **compile-time annotations / metaprogramming**, and **modern syntax** while remaining lightweight and embeddable directly inside your Rust tools, game engines, and servers.

---

## Why Embed Flame in Rust?

- **Game Engine Modding & Scripting**: Enable users to write type-safe game mods, enemy AI, quests, and event triggers without recompiling the host Rust binary.
- **Extensible Plugin Systems**: Add an official plugin architecture to your CLI tools, IDEs, or backend services where third-party developers write secure `.fm` plugins.
- **Dynamic Business Rules**: Evaluate dynamic configurations, mathematical formulas, and business rules in-process with near-native speed.
- **Full Project Embedding**: Embed entire Flame applications built with `fmp build` with access to packages, modules, and native dependencies.

---

## Installation

Add `flamebinder` to your `Cargo.toml`:

```toml
[dependencies]
flamebinder = "0.1.0"
```

### Feature Flags

Customize which Flame capabilities are compiled into your host binary:

```toml
[dependencies]
# Default includes the standard library (networking, utilities, async timers)
flamebinder = { version = "0.1.0", features = ["std"] }

# Or enable full OS automation and hardware integrations
# flamebinder = { version = "0.1.0", features = ["full"] }
```

| Feature | Description |
|---|---|
| `std` *(default)* | Standard library utilities, HTTP/WebSocket networking, and collections |
| `full` | Enables OS automation, system hardware, and native platform bindings |

---

## Core Capabilities

### 1. In-Process VM Script Execution (`Binder`)

Embed Flame directly in-memory to execute scripts and invoke functions:

```rust
use flamebinder::{Binder, Value, ValueExt};

fn main() -> Result<(), String> {
    // 1. Initialize an in-process Flame Binder and load a script
    let mut binder = Binder::load("scripts/game_logic.fm")?;

    // 2. Call exported Flame functions directly
    let args = vec![Value::from("Player1"), Value::from(100)];
    let result = binder.call("on_player_hit", args)?;

    if let Some(remaining_hp) = result.as_int() {
        println!("Player HP remaining: {}", remaining_hp);
    }

    Ok(())
}
```

---

### 2. Exposing Host Rust Functions to Flame (`register_fn`)

Host applications (like game engines or plugin hosts) can register native Rust closures that Flame scripts can call:

```rust
use flamebinder::{Binder, Value, ValueExt};

fn main() -> Result<(), String> {
    let mut binder = Binder::new();

    // Expose a native Rust function to Flame scripts
    binder.register_fn("give_player_gold", |args| {
        let player = args.get(0).and_then(|v| v.as_str()).unwrap_or("Unknown");
        let amount = args.get(1).and_then(|v| v.as_int()).unwrap_or(0);

        println!("⚡ Rust Host: Awarding {} gold to {}", amount, player);
        Ok(Value::from(true))
    });

    // Run Flame script that calls the registered Rust function
    binder.load_source(r#"
        export fn complete_quest(player_name: String) {
            give_player_gold(player_name, 250);
        }
    "#, "quest.fm")?;

    binder.call("complete_quest", vec![Value::from("Hero")])?;

    Ok(())
}
```

---

### 3. Evaluating Dynamic Expressions (`eval`) and Global State

```rust
use flamebinder::{Binder, Value, ValueExt};

fn main() -> Result<(), String> {
    let mut binder = Binder::new();

    // Set host variables accessible to Flame
    binder.set_global("base_damage", Value::from(50));
    binder.set_global("multiplier", Value::from(1.5));

    // Dynamically evaluate expressions
    let final_damage = binder.eval("base_damage * multiplier")?;
    println!("Damage: {:?}", final_damage.as_float()); // 75.0

    Ok(())
}
```

---

### 4. Embedding Full Flame Projects (`load_project`)

Instead of just single files, embed an entire multi-file Flame project built with `fmp`:

```rust
use flamebinder::Binder;

fn main() -> Result<(), String> {
    // Loads flame.toml, resolves .flame/pkg dependencies, and compiles all files in src/
    let mut binder = Binder::load_project("./my_flame_app")?;

    // Now all project structs, enums, and functions are initialized
    let res = binder.call("app_entrypoint", vec![])?;
    println!("App result: {:?}", res);

    Ok(())
}
```

---

### 5. Running with Compiled Runtimes (`CompiledRuntime`)

If your Flame application was compiled into a standalone native binary using `fmp build`:

```rust
use flamebinder::CompiledRuntime;

fn main() -> Result<(), String> {
    // Resolves target/release/<app> from the project directory
    let runtime = CompiledRuntime::from_project("./my_game_mod_runtime", true)?;

    // Run dynamic mod scripts using the compiled runtime
    let output = runtime.run_script_stdout("mods/custom_boss.fm", &[])?;
    println!("Mod output:\n{}", output);

    Ok(())
}
```

---

## Rust <-> Flame Value Conversions

`flamebinder` re-exports `Value` and implements standard `From` traits and `ValueExt`:

```rust
use flamebinder::{Value, ValueExt};

// From Rust primitives to Flame Value:
let v1: Value = 42.into();
let v2: Value = "Flame".into();
let v3: Value = true.into();
let v4: Value = 3.14.into();

// From Flame Value to Rust primitives:
let n: Option<i64> = v1.as_int();
let s: Option<&str> = v2.as_str();
let b: Option<bool> = v3.as_bool();
let f: Option<f64> = v4.as_float();
```

---

## License

ISC License
