// SPDX-License-Identifier: GPL-3.0-or-later
// No console window behind the app; a terminal that starts it with arguments is attached below.
#![windows_subsystem = "windows"]

/// Connects the output to the terminal the app was started from, if there is one.
#[cfg(windows)]
fn attach_console() {
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(process: u32) -> i32;
    }
    const PARENT: u32 = u32::MAX;
    // SAFETY: a plain Win32 call without pointers; failing (no parent console) changes nothing.
    unsafe {
        AttachConsole(PARENT);
    }
}

#[cfg(not(windows))]
fn attach_console() {}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            attach_console();
            println!("OpenRef {}", openref::VERSION);
        }
        Some(other) if other.starts_with('-') => {
            attach_console();
            eprintln!("OpenRef: unknown option '{other}'. Usage: OpenRef [board.oref | images…] or OpenRef --version");
            std::process::exit(2);
        }
        _ => {
            let files = args.into_iter().map(std::path::PathBuf::from).collect();
            if let Err(error) = openref::app::run(files) {
                attach_console();
                eprintln!("OpenRef could not open its window: {error}");
                std::process::exit(1);
            }
        }
    }
}
