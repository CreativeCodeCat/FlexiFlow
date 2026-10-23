//! flexiflow_bar: feeds FlexiFlow's status bar via `FlexiFlow`'s root-window
//! `WM_NAME` property (what `-s`/`XStoreName` sets), using the same inline
//! `^c#hex^`/`^b#hex^`/`^d^` color codes `core.rs`'s `drawbar` parses.
//! Talks to X directly instead of spawning `FlexiFlow -s` every tick.

mod config;
mod segments;

use std::{ffi::CString, fs, time::Duration};

use flexiflow::util::die;

fn render_chip(fg: &str, accent: &(String, String), icon: &str, label: &str) -> String {
    let (bg, bright) = accent;
    if icon.is_empty() {
        format!(" ^c{fg}^^b{bright}^ {label} ")
    } else {
        format!(" ^c{fg}^^b{bg}^ {icon} ^c{fg}^^b{bright}^ {label} ")
    }
}

/// Only one instance should be driving the status text at a time -
/// SIGTERM any other `flexiflow_bar` process (e.g. a leftover from a
/// previous session, or a duplicate autostart).
fn kill_other_instances() {
    let my_pid = std::process::id();
    let Ok(entries) = fs::read_dir("/proc") else { return };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        if pid == my_pid {
            continue;
        }
        let Ok(comm) = fs::read_to_string(entry.path().join("comm")) else { continue };
        if comm.trim() == "flexiflow_bar" {
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGTERM);
            }
        }
    }
}

fn main() {
    env_logger::init();
    kill_other_instances();

    let cfg = config::load();

    let dpy = unsafe { x11::xlib::XOpenDisplay(std::ptr::null_mut()) };
    if dpy.is_null() {
        die("flexiflow_bar: cannot open display");
    }
    let root = unsafe { x11::xlib::XDefaultRootWindow(dpy) };

    let no_accent = (cfg.colors.fg.clone(), cfg.colors.fg.clone());
    let sep = format!("^c{}^^b{}^ ", cfg.colors.sep_fg, cfg.colors.sep_bg);

    let mut cpu_state = segments::CpuState::default();

    loop {
        let mut chips = Vec::new();
        for (i, name) in cfg.segments.iter().enumerate() {
            let Some(pairs) = segments::render(name, &mut cpu_state) else {
                continue;
            };
            let accent = if cfg.colors.accents.is_empty() {
                &no_accent
            } else {
                &cfg.colors.accents[i % cfg.colors.accents.len()]
            };
            for (icon, label) in pairs {
                chips.push(render_chip(&cfg.colors.fg, accent, &icon, &label));
            }
        }

        if let Ok(c_text) = CString::new(chips.join(&sep)) {
            unsafe {
                x11::xlib::XStoreName(dpy, root, c_text.as_ptr());
                x11::xlib::XFlush(dpy);
            }
        }

        std::thread::sleep(Duration::from_millis(cfg.interval_ms));
    }
}
