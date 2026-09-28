use crate::vm::Value;
use std::collections::HashMap;
use sysinfo;

pub fn init() -> HashMap<String, Value> {
    let mut m = HashMap::new();

    m.insert(
        "name".to_string(),
        Value::NativeCallback(|_args| Ok(Value::String(std::env::consts::OS.to_string()))),
    );

    m.insert(
        "arch".to_string(),
        Value::NativeCallback(|_args| Ok(Value::String(std::env::consts::ARCH.to_string()))),
    );

    m.insert(
        "family".to_string(),
        Value::NativeCallback(|_args| Ok(Value::String(std::env::consts::FAMILY.to_string()))),
    );

    m.insert(
        "hostname".to_string(),
        Value::NativeCallback(|_args| {
            Ok(Value::String(
                sysinfo::System::host_name().unwrap_or_default(),
            ))
        }),
    );

    m.insert(
        "mem".to_string(),
        Value::NativeCallback(|_args| {
            let mut s = sysinfo::System::new();
            s.refresh_memory();

            let totalmem = s.total_memory();
            let freemem = s.free_memory();
            let avmem = s.available_memory();
            let usedmem = s.used_memory();
            let mut map = HashMap::new();
            map.insert("totalMemory".to_string(), Value::Int(totalmem as i64));
            map.insert("freeMemory".to_string(), Value::Int(freemem as i64));
            map.insert("availableMemory".to_string(), Value::Int(avmem as i64));
            map.insert("usedMemory".to_string(), Value::Int(usedmem as i64));

            Ok(Value::Formula(map))
        }),
    );

    static LAST_CPU: std::sync::Mutex<Option<(std::time::Instant, f64)>> = std::sync::Mutex::new(None);

    fn get_process_cpu_time() -> f64 {
        #[cfg(target_os = "linux")]
        {
            unsafe {
                let mut usage: libc::rusage = std::mem::zeroed();
                if libc::getrusage(libc::RUSAGE_SELF, &mut usage) == 0 {
                    let user_cpu = usage.ru_utime.tv_sec as f64 + (usage.ru_utime.tv_usec as f64) / 1_000_000.0;
                    let sys_cpu = usage.ru_stime.tv_sec as f64 + (usage.ru_stime.tv_usec as f64) / 1_000_000.0;
                    return user_cpu + sys_cpu;
                }
            }
        }
        0.0
    }

    fn calculate_cpu_percent(current_cpu_secs: f64) -> f64 {
        let now = std::time::Instant::now();
        let mut guard = LAST_CPU.lock().unwrap();
        if let Some((last_time, last_cpu)) = *guard {
            let dt = now.duration_since(last_time).as_secs_f64();
            let dcpu = (current_cpu_secs - last_cpu).max(0.0);
            *guard = Some((now, current_cpu_secs));
            if dt > 0.0001 {
                return ((dcpu / dt) * 100.0 * 100.0).round() / 100.0;
            }
            0.0
        } else {
            *guard = Some((now, current_cpu_secs));
            0.0
        }
    }

    m.insert(
        "processMemory".to_string(),
        Value::NativeCallback(|_args| {
            #[cfg(target_os = "linux")]
            {
                let total_cpu_secs = get_process_cpu_time();
                let cpu_percent = calculate_cpu_percent(total_cpu_secs);

                if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
                    let parts: Vec<&str> = statm.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let page_size = 4096;
                        let vmsize = parts[0].parse::<u64>().unwrap_or(0) * page_size;
                        let rss = parts[1].parse::<u64>().unwrap_or(0) * page_size;
                        let mut map = HashMap::new();
                        map.insert("rss".to_string(), Value::Int(rss as i64));
                        let rss_mb = (rss as f64) / (1024.0 * 1024.0);
                        map.insert("rssMb".to_string(), Value::Float((rss_mb * 100.0).round() / 100.0));
                        let vm_mb = (vmsize as f64) / (1024.0 * 1024.0);
                        map.insert("virtualMb".to_string(), Value::Float((vm_mb * 100.0).round() / 100.0));
                        map.insert("cpu".to_string(), Value::Float(cpu_percent));
                        map.insert("cpuTime".to_string(), Value::Float((total_cpu_secs * 1000.0).round() / 1000.0));
                        return Ok(Value::Formula(map));
                    }
                }
            }

            let mut s = sysinfo::System::new();
            let pid = sysinfo::Pid::from_u32(std::process::id());
            s.refresh_process(pid);
            let mut map = HashMap::new();
            if let Some(proc) = s.process(pid) {
                let memory_bytes = proc.memory();
                let virtual_bytes = proc.virtual_memory();
                let cpu_p = proc.cpu_usage() as f64;
                map.insert("rss".to_string(), Value::Int(memory_bytes as i64));
                map.insert("rssMb".to_string(), Value::Float(((memory_bytes as f64) / (1024.0 * 1024.0) * 100.0).round() / 100.0));
                map.insert("virtualMb".to_string(), Value::Float(((virtual_bytes as f64) / (1024.0 * 1024.0) * 100.0).round() / 100.0));
                map.insert("cpu".to_string(), Value::Float(cpu_p));
                map.insert("cpuTime".to_string(), Value::Float(0.0));
            } else {
                map.insert("rss".to_string(), Value::Int(0));
                map.insert("rssMb".to_string(), Value::Float(0.0));
                map.insert("virtualMb".to_string(), Value::Float(0.0));
                map.insert("cpu".to_string(), Value::Float(0.0));
                map.insert("cpuTime".to_string(), Value::Float(0.0));
            }
            Ok(Value::Formula(map))
        }),
    );

    m.insert(
        "memoryUsage".to_string(),
        Value::NativeCallback(|_args| {
            #[cfg(target_os = "linux")]
            if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
                let parts: Vec<&str> = statm.split_whitespace().collect();
                if parts.len() >= 2 {
                    let page_size = 4096;
                    let rss = parts[1].parse::<u64>().unwrap_or(0) * page_size;
                    let mb = (rss as f64) / (1024.0 * 1024.0);
                    return Ok(Value::Float((mb * 100.0).round() / 100.0));
                }
            }
            let mut s = sysinfo::System::new();
            let pid = sysinfo::Pid::from_u32(std::process::id());
            s.refresh_process(pid);
            if let Some(proc) = s.process(pid) {
                let mb = (proc.memory() as f64) / (1024.0 * 1024.0);
                Ok(Value::Float((mb * 100.0).round() / 100.0))
            } else {
                Ok(Value::Float(0.0))
            }
        }),
    );

    m.insert(
        "cpuUsage".to_string(),
        Value::NativeCallback(|_args| {
            let cpu_time = get_process_cpu_time();
            if cpu_time > 0.0 {
                return Ok(Value::Float(calculate_cpu_percent(cpu_time)));
            }
            let mut s = sysinfo::System::new();
            let pid = sysinfo::Pid::from_u32(std::process::id());
            s.refresh_process(pid);
            if let Some(proc) = s.process(pid) {
                Ok(Value::Float((proc.cpu_usage() as f64 * 100.0).round() / 100.0))
            } else {
                Ok(Value::Float(0.0))
            }
        }),
    );

    m
}
