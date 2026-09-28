pub fn print_help() {
    let bold = "\x1b[1m";
    let cyan = "\x1b[1;36m";
    let reset = "\x1b[0m";

    println!(
        "{}Flame Compiler & Package Manager (Version {}){} ",
        bold,
        env!("CARGO_PKG_VERSION"),
        reset
    );
    println!("Designed for systems programming with supreme DX.");
    println!();
    println!("{}USAGE:{} fmp <SUBCOMMAND> [args]", bold, reset);
    println!();
    println!("{}SUBCOMMANDS:{}", bold, reset);
    println!(
        "  {}install{} [--force]   Install all packages and compile native plugin interfaces",
        cyan, reset
    );
    println!(
        "  {}add{} <pkg> | add --plugin <path> (-p) | add --native <crate> (-n) Add dependency, plugin, or crate",
        cyan, reset
    );
    println!(
        "  {}remove{} <pkg>       Remove an installed package",
        cyan, reset
    );
    println!(
        "  {}new{} <name> | new --plugin <name> (-p) Create a new Flame package or native Rust plugin",
        cyan, reset
    );
    println!(
        "  {}build{} [--release] [--vfs] Compile project into application-specific native runtime",
        cyan, reset
    );
    println!(
        "  {}check{} <file> [--json] [--line N --col N]  Analyze a Flame file for diagnostics and IDE data",
        cyan, reset
    );
    println!(
        "  {}format{} <file> [--stdout]  Format a Flame source file",
        cyan, reset
    );
    println!(
        "  {}list-plugins{} [--json]  List configured plugins",
        cyan, reset
    );
    println!(
        "  {}run{} [file] [--watch] [--device] Run Flame script with instant execution or watch mode (-w)",
        cyan, reset
    );
    println!(
        "  {}flash{} [--target <board>] [--port <COM>] Build & burn bare-metal firmware to microcontroller",
        cyan, reset
    );
    println!(
        "  {}monitor{} [--port <COM>] [--baud 115200] Connect to hardware serial UART telemetry stream",
        cyan, reset
    );
    println!(
        "  {}test{}                Execute unit tests inside the current project",
        cyan, reset
    );
    println!(
        "  {}bench{} [filter]      Execute performance benchmarks declared with @Benchmark",
        cyan, reset
    );
    println!(
        "  {}new --plugin{} <name> Scaffold native Rust FFI bridges & Cargo configuration (alias: native init)",
        cyan, reset
    );
    println!(
        "  {}update{} [std]             Update Flame toolchain and Blaze standard library definitions",
        cyan, reset
    );
    println!(
        "  {}uninstall{}               Uninstall Flame toolchain and remove Blaze standard library",
        cyan, reset
    );
    println!(
        "  {}version, --v, --version{} Print installed Flame compiler version",
        cyan, reset
    );
    println!(
        "  {}help, -h, --help{}        Print help details",
        cyan, reset
    );
    println!();
}

