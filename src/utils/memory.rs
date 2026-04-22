use std::process;

#[cfg(target_os = "linux")]
pub fn get_current_memory_bytes() -> u64 {
    use std::fs;
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = value.parse::<u64>() {
                        return kb * 1024;
                    }
                }
            }
        }
    }
    0
}

#[cfg(target_os = "macos")]
pub fn get_current_memory_bytes() -> u64 {
    use std::mem;
    use libc::{getrusage, RUSAGE_SELF, rusage};
    let mut usage: rusage = unsafe { mem::zeroed() };
    unsafe {
        getrusage(RUSAGE_SELF, &mut usage);
    }
    usage.ru_maxrss as u64 * 1024
}

#[cfg(target_os = "windows")]
pub fn get_current_memory_bytes() -> u64 {
    0
}

pub fn format_memory_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
