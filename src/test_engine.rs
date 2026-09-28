use crate::lexer::Lexer;
use crate::parser::{Parser, Stmt};
use crate::runner::Runner;

pub struct TestStats {
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub measured: usize,
    pub filtered: usize,
}

#[derive(Clone, Debug)]
pub struct BenchmarkConfig {
    pub warmup: usize,
    pub iterations: usize,
    pub group: Option<String>,
    pub custom_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BenchmarkSummary {
    pub name: String,
    pub avg_ns: f64,
    pub ops_per_sec: f64,
}

fn format_duration(ns: f64) -> String {
    if ns < 1_000.0 {
        format!("{:.2} ns", ns)
    } else if ns < 1_000_000.0 {
        format!("{:.2} µs", ns / 1_000.0)
    } else if ns < 1_000_000_000.0 {
        format!("{:.2} ms", ns / 1_000_000.0)
    } else {
        format!("{:.2} s", ns / 1_000_000_000.0)
    }
}

fn format_bytes(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{:.0} B", bytes)
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.2} KB", bytes / 1024.0)
    } else {
        format!("{:.2} MB", bytes / (1024.0 * 1024.0))
    }
}

fn format_number(num: f64) -> String {
    let int_part = num.round() as u64;
    let s = int_part.to_string();
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*c);
    }
    out
}

fn get_memory_bytes() -> u64 {
    #[cfg(target_os = "linux")]
    if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
        if let Some(rss_pages) = statm.split_whitespace().nth(1) {
            if let Ok(pages) = rss_pages.parse::<u64>() {
                return pages * 4096;
            }
        }
    }
    0
}

pub fn execute_test_suite(runner: &mut Runner, stmts: &[Stmt], filename: &str) -> TestStats {
    let mut stats = TestStats {
        passed: 0,
        failed: 0,
        ignored: 0,
        measured: 0,
        filtered: 0,
    };

    println!("\nrunning tests in \x1b[1m{}\x1b[0m:", filename);

    let mut before_all = Vec::new();
    let mut after_all = Vec::new();
    let mut setup = Vec::new();
    let mut cleanup = Vec::new();
    let mut test_cases = Vec::new();
    let mut has_only_test = false;

    for stmt in stmts {
        if let Stmt::FuncDecl {
            name, annotations, ..
        } = stmt
        {
            for anno in annotations {
                match anno.name.as_str() {
                    "BeforeAll" => before_all.push(name.clone()),
                    "AfterAll" => after_all.push(name.clone()),
                    "Setup" => setup.push(name.clone()),
                    "Cleanup" => cleanup.push(name.clone()),
                    "Test" | "test" | "Benchmark" | "benchmark" | "Parameterized" | "parameterized" | "ExpectPanic" | "expect_panic" | "Ignore" | "ignore"
                    | "Only" | "only" => {
                        if !test_cases.contains(&name.clone()) {
                            test_cases.push(name.clone());
                        }
                        if anno.name == "Only"
                            || anno
                                .args
                                .iter()
                                .any(|arg| arg.contains("only: true") || arg == "only: true")
                        {
                            has_only_test = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    for func_name in &before_all {
        let func_opt = runner.env.lock().unwrap().get(func_name);
        if let Some(func_val) = func_opt {
            if let Err(e) = runner.invoke_callback_value(&func_val, vec![]) {
                println!("  \x1b[1;31m[FAIL]\x1b[0m \x1b[1;36m@BeforeAll\x1b[0m {}", func_name);
                let span = runner.current_span.clone().unwrap_or(crate::lexer::Span { start: 0, end: 0, line: 1, col: 1 });
                crate::diagnostics::Diagnostic::new_error(e, runner.filepath.display().to_string(), span, None, None).print(&std::fs::read_to_string(&runner.filepath).unwrap_or_default());
                stats.failed += 1;
                return stats;
            }
        }
    }

    let mut benchmark_groups: std::collections::HashMap<String, Vec<BenchmarkSummary>> = std::collections::HashMap::new();

    for func_name in &test_cases {
        let mut is_ignore = false;
        let mut is_only = false;
        let mut is_benchmark = false;
        let mut benchmark_config = None;
        let mut is_expect_panic = false;
        let mut parameterized_args = None;

        for stmt in stmts {
            if let Stmt::FuncDecl {
                name, annotations, ..
            } = stmt
            {
                if name == func_name {
                    for anno in annotations {
                        match anno.name.as_str() {
                            "Ignore" | "ignore" => is_ignore = true,
                            "Only" | "only" => is_only = true,
                            "Benchmark" | "benchmark" => {
                                is_benchmark = true;
                                let mut warmup: usize = 10;
                                let mut iterations: usize = 100;
                                let mut group: Option<String> = None;
                                let mut custom_name: Option<String> = None;

                                for arg in &anno.args {
                                    if let Some((k, v)) = arg.split_once(':').or_else(|| arg.split_once('=')) {
                                        let key = k.trim();
                                        let val = v.trim().trim_matches('"').trim_matches('\'').trim();
                                        if key == "warmup" {
                                            if let Ok(w) = val.parse::<usize>() { warmup = w; }
                                        } else if key == "iterations" {
                                            if let Ok(it) = val.parse::<usize>() { iterations = it; }
                                        } else if key == "group" {
                                            group = Some(val.to_string());
                                        } else if key == "name" {
                                            custom_name = Some(val.to_string());
                                        }
                                    }
                                }
                                benchmark_config = Some(BenchmarkConfig {
                                    warmup,
                                    iterations,
                                    group,
                                    custom_name,
                                });
                            }
                            "ExpectPanic" | "expect_panic" => is_expect_panic = true,
                            "Parameterized" | "parameterized" => {
                                if !anno.args.is_empty() {
                                    parameterized_args = Some(anno.args[0].clone());
                                }
                            }
                            "Test" | "test" => {
                                for arg in &anno.args {
                                    let clean = arg.replace(" ", "");
                                    if clean.contains("skip:true") || clean.contains("skip=true") {
                                        is_ignore = true;
                                    }
                                    if clean.contains("only:true") || clean.contains("only=true") {
                                        is_only = true;
                                    }
                                    // Parse timeout
                                    if let Some(idx) = clean.find("timeout:") {
                                        if let Ok(_val) = clean[idx + 8..].parse::<u64>() {
                                            // Implement timeout logic if needed later
                                        }
                                    } else if let Some(idx) = clean.find("timeout=") {
                                        if let Ok(_val) = clean[idx + 8..].parse::<u64>() {
                                            // Implement timeout logic if needed later
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if has_only_test && !is_only {
            stats.filtered += 1;
            continue;
        }

        if is_ignore {
            println!(
                "  \x1b[33m[SKIP]\x1b[0m \x1b[1;36m@Ignore\x1b[0m {}",
                func_name
            );
            stats.ignored += 1;
            continue;
        }

        for setup_name in &setup {
            let setup_opt = runner.env.lock().unwrap().get(setup_name);
            if let Some(s_val) = setup_opt {
                let _ = runner.invoke_callback_value(&s_val, vec![]);
            }
        }

        let test_func_opt = runner.env.lock().unwrap().get(func_name);
        if let Some(f_val) = test_func_opt {
            if is_benchmark {
                let cfg = benchmark_config.unwrap_or(BenchmarkConfig {
                    warmup: 10,
                    iterations: 100,
                    group: None,
                    custom_name: None,
                });
                let display_name = cfg.custom_name.clone().unwrap_or_else(|| func_name.clone());

                // 1. Warmup iterations
                let mut warmup_failed = false;
                for _ in 0..cfg.warmup {
                    if let Err(e) = runner.invoke_callback_value(&f_val, vec![]) {
                        println!("  \x1b[1;31m[FAIL]\x1b[0m \x1b[1;36m@Benchmark\x1b[0m {} (warmup failed)", func_name);
                        let span = runner.current_span.clone().unwrap_or(crate::lexer::Span { start: 0, end: 0, line: 1, col: 1 });
                        crate::diagnostics::Diagnostic::new_error(e, runner.filepath.display().to_string(), span, None, None).print(&std::fs::read_to_string(&runner.filepath).unwrap_or_default());
                        stats.failed += 1;
                        warmup_failed = true;
                        break;
                    }
                }
                if warmup_failed {
                    continue;
                }

                // 2. Measurement phase with monotonic clock
                let mem_before = get_memory_bytes();
                let mut durations = Vec::with_capacity(cfg.iterations);
                let mut benchmark_failed = false;

                for _ in 0..cfg.iterations {
                    let t0 = std::time::Instant::now();
                    let res = runner.invoke_callback_value(&f_val, vec![]);
                    let elapsed = t0.elapsed();
                    match res {
                        Ok(val) => {
                            std::hint::black_box(val);
                            durations.push(elapsed.as_nanos() as f64);
                        }
                        Err(e) => {
                            println!("  \x1b[1;31m[FAIL]\x1b[0m \x1b[1;36m@Benchmark\x1b[0m {}", func_name);
                            let span = runner.current_span.clone().unwrap_or(crate::lexer::Span { start: 0, end: 0, line: 1, col: 1 });
                            crate::diagnostics::Diagnostic::new_error(e, runner.filepath.display().to_string(), span, None, None).print(&std::fs::read_to_string(&runner.filepath).unwrap_or_default());
                            stats.failed += 1;
                            benchmark_failed = true;
                            break;
                        }
                    }
                }

                let mem_after = get_memory_bytes();

                if !benchmark_failed && !durations.is_empty() {
                    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let count = durations.len();
                    let total_ns: f64 = durations.iter().sum();
                    let total_secs = total_ns / 1_000_000_000.0;
                    let avg_ns = total_ns / count as f64;
                    let min_ns = durations[0];
                    let max_ns = durations[count - 1];
                    let p50_ns = durations[(count * 50) / 100];
                    let p95_ns = durations[(count * 95) / 100];
                    let p99_ns = durations[(count * 99) / 100];
                    let ops_per_sec = if total_secs > 0.0 { (count as f64) / total_secs } else { 0.0 };
                    let allocated_bytes = mem_after.saturating_sub(mem_before);
                    let per_op_bytes = if count > 0 { allocated_bytes as f64 / count as f64 } else { 0.0 };

                    println!(
                        "  \x1b[1;32m[PASS]\x1b[0m \x1b[1;36m@Benchmark\x1b[0m {}",
                        func_name
                    );
                    println!("    \x1b[1mBenchmark:\x1b[0m {}", display_name);
                    println!();
                    println!("    Warmup       {:>10} iterations", format_number(cfg.warmup as f64));
                    println!("    Iterations   {:>10} iterations", format_number(count as f64));
                    println!();
                    println!("    \x1b[1mTime\x1b[0m");
                    println!("      total      {:>12}", format_duration(total_ns));
                    println!("      avg        {:>12}", format_duration(avg_ns));
                    println!("      min        {:>12}", format_duration(min_ns));
                    println!("      max        {:>12}", format_duration(max_ns));
                    println!("      p50        {:>12}", format_duration(p50_ns));
                    println!("      p95        {:>12}", format_duration(p95_ns));
                    println!("      p99        {:>12}", format_duration(p99_ns));
                    println!();
                    println!("    \x1b[1mThroughput\x1b[0m");
                    println!("      {:>10} ops/sec", format_number(ops_per_sec));
                    println!();
                    println!("    \x1b[1mMemory\x1b[0m");
                    println!("      allocated  {:>12}", format_bytes(allocated_bytes as f64));
                    println!("      per op     {:>12}", format_bytes(per_op_bytes));
                    println!();
                    println!("    Result: \x1b[1;32mPASS\x1b[0m\n");

                    if let Some(grp) = &cfg.group {
                        benchmark_groups.entry(grp.clone()).or_default().push(BenchmarkSummary {
                            name: display_name,
                            avg_ns,
                            ops_per_sec,
                        });
                    }

                    stats.measured += 1;
                }
            } else if let Some(arg_str) = parameterized_args {
                let mut l = Lexer::new(&arg_str);
                let mut tok_vec = Vec::new();
                loop {
                    let tok = l.next_token();
                    let e = tok.kind == crate::lexer::TokenKind::EOF;
                    tok_vec.push(tok);
                    if e {
                        break;
                    }
                }
                let mut p = Parser::new(tok_vec, "param_arg".to_string());
                if let Ok(expr) = p.parse_expr() {
                    let env_clone = runner.env.clone();
                    if let Ok(evaled) = runner.eval_expr(&expr, env_clone) {
                        let list = match evaled {
                            crate::vm::Value::Tuple(vec_val) => vec_val.clone(),
                            other => vec![other],
                        };
                        let mut all_ok = true;
                        let start = std::time::Instant::now();
                        for case in &list {
                            let call_args = match case {
                                crate::vm::Value::Tuple(tup) => tup.clone(),
                                single => vec![single.clone()],
                            };
                            if let Err(e) = runner.invoke_callback_value(&f_val, call_args) {
                                println!("  \x1b[1;31m[FAIL]\x1b[0m \x1b[1;36m@Parameterized\x1b[0m {} on argument {:?}", func_name, case);
                                let span = runner.current_span.clone().unwrap_or(crate::lexer::Span { start: 0, end: 0, line: 1, col: 1 });
                                crate::diagnostics::Diagnostic::new_error(e, runner.filepath.display().to_string(), span, None, None).print(&std::fs::read_to_string(&runner.filepath).unwrap_or_default());
                                all_ok = false;
                                break;
                            }
                        }
                        if all_ok {
                            println!(
                                "  \x1b[1;32m[PASS]\x1b[0m \x1b[1;36m@Parameterized\x1b[0m {} ({} parameter cases in {:.2}ms)",
                                func_name,
                                list.len(),
                                start.elapsed().as_secs_f64() * 1000.0
                            );
                            stats.passed += 1;
                        } else {
                            stats.failed += 1;
                        }
                    } else {
                        println!(
                            "  \x1b[1;31m[FAIL]\x1b[0m \x1b[1;36m@Parameterized\x1b[0m {}: failed to evaluate parameter argument expression",
                            func_name
                        );
                        stats.failed += 1;
                    }
                }
            } else {
                let start = std::time::Instant::now();
                let res = runner.invoke_callback_value(&f_val, vec![]);
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                if is_expect_panic {
                    match res {
                        Err(e) => {
                            println!("  \x1b[1;32m[PASS]\x1b[0m \x1b[1;36m@ExpectPanic\x1b[0m {} (expected panic occurred in {:.2}ms: {})", func_name, elapsed, e);
                            stats.passed += 1;
                        }
                        Ok(_) => {
                            println!(
                                "  \x1b[1;31m[FAIL]\x1b[0m \x1b[1;36m@ExpectPanic\x1b[0m {}: function completed without expected error/panic!",
                                func_name
                            );
                            stats.failed += 1;
                        }
                    }
                } else {
                    match res {
                        Ok(_) => {
                            println!(
                                "  \x1b[1;32m[PASS]\x1b[0m \x1b[1;36m@Test\x1b[0m {} ({:.2}ms)",
                                func_name, elapsed
                            );
                            stats.passed += 1;
                        }
                        Err(e) => {
                            println!("  \x1b[1;31m[FAIL]\x1b[0m \x1b[1;36m@Test\x1b[0m {}", func_name);
                            let span = runner.current_span.clone().unwrap_or(crate::lexer::Span { start: 0, end: 0, line: 1, col: 1 });
                            crate::diagnostics::Diagnostic::new_error(e, runner.filepath.display().to_string(), span, None, None).print(&std::fs::read_to_string(&runner.filepath).unwrap_or_default());
                            stats.failed += 1;
                        }
                    }
                }
            }
        }

        for cleanup_name in &cleanup {
            let cleanup_opt = runner.env.lock().unwrap().get(cleanup_name);
            if let Some(c_val) = cleanup_opt {
                let _ = runner.invoke_callback_value(&c_val, vec![]);
            }
        }
    }

    for func_name in &after_all {
        let after_opt = runner.env.lock().unwrap().get(func_name);
        if let Some(func_val) = after_opt {
            let _ = runner.invoke_callback_value(&func_val, vec![]);
        }
    }

    for (group_name, mut entries) in benchmark_groups {
        if entries.len() >= 2 {
            entries.sort_by(|a, b| a.avg_ns.partial_cmp(&b.avg_ns).unwrap_or(std::cmp::Ordering::Equal));
            println!("  ──────────────────────────────────────────────────────────");
            println!("  \x1b[1;36mBenchmark Group: {}\x1b[0m", group_name);
            println!("  {:<24} {:>14} {:>14}", "Benchmark", "time/op", "ops/sec");
            println!("  ──────────────────────────────────────────────────────────");
            for entry in &entries {
                println!("  {:<24} {:>14} {:>14}", entry.name, format_duration(entry.avg_ns), format_number(entry.ops_per_sec));
            }
            println!("  ──────────────────────────────────────────────────────────");
            if entries.len() >= 2 && entries[0].avg_ns > 0.0 {
                let baseline = entries.last().unwrap();
                let best = &entries[0];
                if baseline.avg_ns > best.avg_ns {
                    let improvement = ((baseline.avg_ns - best.avg_ns) / baseline.avg_ns) * 100.0;
                    println!("  improvement                       \x1b[1;32m+{:.1}%\x1b[0m", improvement);
                    println!("  ──────────────────────────────────────────────────────────");
                }
            }
            println!();
        }
    }

    stats
}
