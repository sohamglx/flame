#![cfg(feature = "cli")]
pub mod blaze;
pub mod cli;
pub mod compiler;
mod diagnostics;
mod formatter;
pub mod ide;
mod lexer;
pub mod native_std;
mod package_manager;
mod parser;
pub mod runner;
mod stdlib;
mod test_engine;
mod typechecker;
pub mod utils;
pub mod vm;

pub use cli::ide::{JsonCompletion, JsonHover};

use cli::commands::*;
use cli::ide::*;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let builder = std::thread::Builder::new()
        .name("flame-main".into())
        .stack_size(32 * 1024 * 1024);
    let handler = builder
        .spawn(move || {
            real_main();
        })
        .unwrap();
    if let Err(e) = handler.join() {
        std::panic::resume_unwind(e);
    }
}

fn real_main() {
    ctrlc::set_handler(move || {
        #[cfg(all(feature = "net", feature = "ws"))]
        crate::native_std::net::ws::shutdown_all_ws();
        std::process::exit(0);
    })
    .unwrap_or_else(|_| ());

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        if Path::new("src/main.fm").exists() {
            run_file("src/main.fm", false, &[]);
            return;
        }
        print_help();
        return;
    }

    let command = &args[1];
    match command.as_str() {
        "install" | "i" => {
            package_manager::install_all_packages(&args[2..]);
        }
        "add" => {
            package_manager::add_package(&args[2..]);
        }
        "remove" => {
            if args.len() < 3 {
                println!("\x1b[1;31merror:\x1b[0m please specify package name to remove.");
                println!("usage: fmp remove <package_name>");
                return;
            }
            package_manager::remove_package(&args[2]);
        }
        "new" => {
            if args.len() < 3 {
                println!("\x1b[1;31merror:\x1b[0m please specify the project or plugin name");
                println!("usage: fmp new <project_name> | fmp new --plugin <plugin_name>");
                return;
            }
            if args.contains(&"--plugin".to_string()) || args.contains(&"-p".to_string()) {
                let p_idx = args.iter().position(|r| r == "--plugin" || r == "-p").unwrap();
                let plugin_name = if let Some(n) = args.get(p_idx + 1).filter(|a| !a.starts_with('-')) {
                    n.as_str()
                } else if p_idx > 2 && !args[2].starts_with('-') {
                    args[2].as_str()
                } else {
                    "bridge"
                };
                init_native_bridge(plugin_name);
            } else {
                let project_name = &args[2];
                create_new_project(project_name);
            }
        }
        "doctor" => {
            run_doctor_command();
        }
        "build" => {
            build_project(&args);
        }
        "package" => {
            package_project(&args);
        }
        "check" => {
            run_check_command(&args);
        }
        "definition" => {
            run_definition_command(&args);
        }
        "update" => {
            run_update_command(&args);
        }
        "uninstall" => {
            run_uninstall_command(&args);
        }
        "format" | "fmt" => {
            if args.len() < 3 {
                println!("\x1b[1;31merror:\x1b[0m please specify a Flame file to format");
                println!("usage: fmp format <file_path.fm> [--stdout]");
                return;
            }
            let filepath = &args[2];
            let source = if args.contains(&"--stdin".to_string()) {
                use std::io::Read;
                let mut buf = String::new();
                let _ = std::io::stdin().read_to_string(&mut buf);
                buf
            } else {
                match fs::read_to_string(filepath) {
                    Ok(content) => content,
                    Err(err) => {
                        println!(
                            "\x1b[1;31merror:\x1b[0m failed to read '{}': {}",
                            filepath, err
                        );
                        return;
                    }
                }
            };
            let formatted = formatter::format_code(&source);
            if args.contains(&"--stdout".to_string()) {
                print!("{}", formatted);
            } else {
                if let Err(err) = fs::write(filepath, formatted) {
                    println!(
                        "\x1b[1;31merror:\x1b[0m failed to write '{}': {}",
                        filepath, err
                    );
                } else {
                    println!("Formatted {}", filepath);
                }
            }
        }
        "list-plugins" => {
            list_plugins_command(&args);
        }
        "run" => {
            let force_local = args.contains(&"--local".to_string());
            let is_watch = args.contains(&"--watch".to_string()) || args.contains(&"-w".to_string());

            let (filepath, script_args_start) = if args.len() > 2 {
                let mut idx = 2;
                while idx < args.len() && (args[idx] == "--local" || args[idx] == "--watch" || args[idx] == "-w") {
                    idx += 1;
                }

                if args.len() > idx
                    && (Path::new(&args[idx]).exists()
                        || args[idx].ends_with(".fm"))
                {
                    (args[idx].clone(), idx + 1)
                } else if Path::new("src/main.fm").exists() {
                    ("src/main.fm".to_string(), idx)
                } else {
                    println!(
                        "\x1b[1;31merror:\x1b[0m please specify a Flame file to run or create src/main.fm"
                    );
                    return;
                }
            } else if Path::new("src/main.fm").exists() {
                ("src/main.fm".to_string(), 2)
            } else {
                println!(
                    "\x1b[1;31merror:\x1b[0m please specify a Flame file to run or create src/main.fm"
                );
                println!("usage: fmp run [file_path.fm] [--watch]");
                return;
            };

            let mut filtered_script_args = Vec::new();
            for arg in args.iter().skip(script_args_start) {
                if arg != "--local" && arg != "--watch" && arg != "-w" {
                    filtered_script_args.push(arg.clone());
                }
            }

            if is_watch {
                run_file_watch(&filepath, force_local, &filtered_script_args);
            } else {
                run_file(&filepath, force_local, &filtered_script_args);
            }
        }
        "test" => {
            run_tests(&args);
        }
        "gen" => {
            if args.len() < 3 || args[2] != "fmi" {
                println!("\x1b[1;31merror:\x1b[0m unknown subcommand");
                println!("usage: fmp gen fmi <rust_file>");
                return;
            }
            if args.len() < 4 {
                println!("\x1b[1;31merror:\x1b[0m expected rust file path");
                println!("usage: fmp gen fmi <rust_file>");
                return;
            }
            let filepath = &args[3];
            package_manager::gen_fmi_from_rust_file(std::path::Path::new(filepath));
        }
        "version" | "--version" | "-version" | "--v" | "-v" | "-V" => {
            println!("Flame {} (Fifth Spark)", env!("CARGO_PKG_VERSION"));
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        _ => {
            // Check if argument is a Flame source file
            let p = Path::new(command);
            if p.exists() && p.extension().map_or(false, |ext| ext == "flame") {
                let force_local = args.contains(&"--local".to_string());
                let actual_cmd = if command == "--local" && args.len() >= 3 {
                    &args[2]
                } else {
                    command
                };

                let script_args_start = if command == "--local" { 3 } else { 2 };
                // Wait, if it's `flame target.fm --local`, `--local` might be args[2] and `target.fm` is args[1] (command).
                // Let's just filter out `--local` entirely from script_args!
                let mut filtered_script_args = Vec::new();
                for arg in args.iter().skip(script_args_start) {
                    if arg != "--local" {
                        filtered_script_args.push(arg.clone());
                    }
                }

                run_file(actual_cmd, force_local, &filtered_script_args);
            } else {
                println!("\x1b[1;31merror:\x1b[0m unknown command '{}'", command);
                print_help();
            }
        }
    }
}
