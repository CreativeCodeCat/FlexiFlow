//! flexiflow-river-status: the X11 FlexiFlow's flexiflow_bar, for river:
//! feeds flexiflow-river's bar its status text every
//! tick, written in the inline `^c#hex^`/`^b#hex^`/`^d^` color codes the
//! bar parses (see `flexiflow_river::status`).
//!
//! `flexiflow-river-status --click <segment> <button>`, which the bar runs when
//! a chip is clicked, runs that segment's `clicks` command instead.

mod config;
mod segments;

use std::{fs, process::Command, time::Duration};

use flexiflow_river::ipc::{self, Request};

/// Hand `text` to the running flexiflow-river. A failure is only a
/// warning: the window manager may be restarting, and the next tick
/// tries again.
fn set_status(text: &str) {
    if let Err(e) = ipc::send(&Request::Status(text.to_string())) {
        log::warn!("flexiflow-river-status: can't reach flexiflow-river: {e}");
    }
}

/// A chip starts straight with its own colors, so chips sit flush
/// against each other with no bar background showing between them. It's
/// boxed in `border` (see `Colors::border`), which also ends the last
/// chip's box.
fn render_chip(
    fg: &str,
    accent: &(String, String),
    border: &str,
    icon: &str,
    label: &str,
) -> String {
    let (bg, bright) = accent;
    let border = match border {
        "" => bg.as_str(),
        "none" => "",
        color => color,
    };
    if icon.is_empty() {
        format!("^o{border}^^c{fg}^^b{bright}^ {label} ")
    } else {
        format!(
            "^o{border}^^c{fg}^^b{bg}^ {icon} ^c{fg}^^b{bright}^ {label} "
        )
    }
}

/// The chips one after another, in `font` if there is one.
fn status_text(font: &str, chips: &[String]) -> String {
    let chips = chips.concat();
    if font.is_empty() {
        chips
    } else {
        format!("^f{font}^{chips}")
    }
}

/// Runs `segment`'s `clicks` command for mouse `button`, if it has one.
fn click(segment: &str, button: u32) {
    let cfg = config::load();
    let Some(command) = cfg.clicks.get(segment).map(|c| c.command(button))
    else {
        return;
    };
    if command.is_empty() {
        return;
    }
    if let Err(e) = Command::new("sh").args(["-c", command]).spawn() {
        log::warn!("flexiflow-river-status: cannot run {command:?}: {e}");
    }
}

/// Only one instance should be driving the status text at a time -
/// SIGTERM any other `flexiflow-river-status` process (`comm` keeps
/// only the first 15 bytes of its name) (e.g. a leftover from a
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
        if comm.trim() == "flexiflow-river-s" {
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
    // whatever launched us may have left SIGCHLD ignored (an autostart may have)), which makes the kernel reap every child
    // itself - `FlexiFlow -b` and the segments' commands included - so
    // waiting on any of them fails with ECHILD
    unsafe {
        libc::signal(libc::SIGCHLD, libc::SIG_DFL);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [flag, segment, button] = args.as_slice()
        && flag == "--click"
    {
        click(segment, button.parse().unwrap_or(0));
        return;
    }

    kill_other_instances();

    let cfg = config::load();

    let no_accent = (cfg.colors.fg.clone(), cfg.colors.fg.clone());

    let mut seg_state = segments::State::default();

    loop {
        let mut chips = Vec::new();
        for (i, name) in cfg.segments.iter().enumerate() {
            let Some(pairs) = segments::render(name, &mut seg_state) else {
                continue;
            };
            let accent = if cfg.colors.accents.is_empty() {
                &no_accent
            } else {
                &cfg.colors.accents[i % cfg.colors.accents.len()]
            };
            // chips of segments with `clicks` name their click area
            let area = if cfg.clicks.contains_key(name) { name } else { "" };
            for (icon, label) in pairs {
                chips.push(format!("^a{area}^"));
                chips.push(render_chip(
                    &cfg.colors.fg,
                    accent,
                    &cfg.colors.border,
                    &icon,
                    &label,
                ));
            }
        }

        set_status(&status_text(&cfg.font, &chips));

        std::thread::sleep(Duration::from_millis(cfg.interval_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chips_start_with_their_own_colors_so_they_sit_flush() {
        let accent = ("#A165E8".to_string(), "#C9A4FF".to_string());
        assert_eq!(
            render_chip("#283036", &accent, "none", "", "6.17"),
            "^o^^c#283036^^b#C9A4FF^ 6.17 "
        );
        assert_eq!(
            render_chip("#283036", &accent, "none", "V", "50%"),
            "^o^^c#283036^^b#A165E8^ V ^c#283036^^b#C9A4FF^ 50% "
        );
    }

    #[test]
    fn a_font_is_set_once_before_the_chips() {
        let chips = ["a".to_string(), "b".to_string()];
        assert_eq!(status_text("", &chips), "ab");
        assert_eq!(status_text("Mono:size=9", &chips), "^fMono:size=9^ab");
    }

    #[test]
    fn chips_are_boxed_in_their_darker_accent_unless_given_a_border() {
        let accent = ("#A165E8".to_string(), "#C9A4FF".to_string());
        assert!(
            render_chip("#283036", &accent, "", "", "6.17")
                .starts_with("^o#A165E8^")
        );
        assert!(
            render_chip("#283036", &accent, "#FFFFFF", "", "6.17")
                .starts_with("^o#FFFFFF^")
        );
    }
}
