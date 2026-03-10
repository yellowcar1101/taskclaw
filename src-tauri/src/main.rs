// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // First thing: write a log so we know main() was reached.
    // If this file never appears, the process is killed before main() runs (AV/policy).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let _ = std::fs::write(dir.join("startup.log"), "main() reached\n");
        }
    }
    taskclaw_lib::run()
}
