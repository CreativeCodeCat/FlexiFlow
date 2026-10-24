//! flexiflow_bar: feeds FlexiFlow's status bar by running `FlexiFlow -b`
//! with the rendered text every tick, using the same inline
//! `^c#hex^`/`^b#hex^`/`^d^` color codes `core.rs`'s `drawbar` parses.

mod config;
mod segments;

use std::{fs, process::Command, time::Duration};

use flexiflow::util::die;

/// Hand `text` to `FlexiFlow -b`, so how the status gets stored lives in
/// one place. Any failure is fatal, same as losing the X connection was
/// back when this talked to X itself: a missing `FlexiFlow` or a dead
/// display won't fix itself on the next tick.
fn set_status(text: &str) {
    match Command::new("FlexiFlow").args(["-b", text]).status() {
        Ok(status) if status.success() => {}
        Ok(status) => {
            die(&format!("flexiflow_bar: `FlexiFlow -b` failed ({status})"))
        }
        Err(e) => {
            die(&format!("flexiflow_bar: cannot run `FlexiFlow -b`: {e}"))
        }
    }
}

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
    // default to "warn" (env_logger's own default is "off", silent)
    // so real problems are visible without needing RUST_LOG set
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .init();
    // whatever launched us may have left SIGCHLD ignored (FlexiFlow's
    // own autostart used to), which makes the kernel reap every child
    // itself - `FlexiFlow -b` and the segments' commands included - so
    // waiting on any of them fails with ECHILD
    unsafe {
        libc::signal(libc::SIGCHLD, libc::SIG_DFL);
    }
    kill_other_instances();

    let cfg = config::load();

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

        set_status(&chips.join(&sep));

        std::thread::sleep(Duration::from_millis(cfg.interval_ms));
    }
}
