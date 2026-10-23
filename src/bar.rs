//! Status bar replicating `flexiflow_bar`'s segments as native penrose_ui
//! widgets instead of shelling out to a script that prints dwm status2d
//! color codes. Colors are the same palette used in the ported `flexiflow_bar`
//! bash script, so the two look related.
use penrose::{Color, extensions::hooks::add_ewmh_hooks, util::spawn_for_output_with_args, x11rb::RustConn};
use penrose_ui::{
    bar::{Position, StatusBar, widgets::*},
    core::TextStyle,
};
use std::{fs, time::Duration};

const FOREGROUND: u32 = 0x283036ff;
const BLACK: u32 = 0x283036ff;

const PURPLE_BRIGHT: u32 = 0xC9A4FFff;
const CYAN_BRIGHT: u32 = 0x59FFF9ff;
const GREEN_BRIGHT: u32 = 0x59FF59ff;
const YELLOW_BRIGHT: u32 = 0xFFFF59ff;
const ORANGE_BRIGHT: u32 = 0xFF9C59ff;
const PINK_BRIGHT: u32 = 0xFF59F9ff;
const RED_BRIGHT: u32 = 0xFF5959ff;

pub const BAR_HEIGHT_PX: u32 = 26;
const FONT: &str = "Hack";
const POINT_SIZE: u8 = 10;

fn style(bg: u32) -> TextStyle {
    TextStyle {
        fg: Color::from(FOREGROUND),
        bg: Some(Color::from(bg)),
        padding: (8, 8),
    }
}

fn text(label: &'static str, bg: u32) -> Box<dyn Widget<RustConn>> {
    Box::new(Text::new(label, style(bg), false, false))
}

fn interval(bg: u32, interval_secs: u64, f: impl Fn() -> Option<String> + Send + 'static) -> Box<dyn Widget<RustConn>> {
    Box::new(IntervalText::new(
        style(bg),
        f,
        Duration::from_secs(interval_secs),
        false,
        false,
    ))
}

// -- segment data, ported from examples/bar_* --------------------------

fn updates_text() -> Option<String> {
    let out = spawn_for_output_with_args("paru", &["-Qu"]).ok()?;
    let n = out.lines().filter(|l| !l.trim().is_empty()).count();
    Some(match n {
        0 => " Fully Updated".to_string(),
        1 => format!(" {n} Update"),
        _ => format!(" {n} Updates"),
    })
}

fn kernel_text() -> Option<String> {
    let out = spawn_for_output_with_args("uname", &["-r"]).ok()?;
    Some(format!("  {}", out.trim()))
}

fn battery_text() -> Option<String> {
    let base = "/sys/class/power_supply";
    let bat = ["BAT0", "BAT1", "BAT"]
        .iter()
        .find(|b| std::path::Path::new(base).join(b).is_dir())?;

    let charge: u32 = fs::read_to_string(format!("{base}/{bat}/capacity")).ok()?.trim().parse().ok()?;
    let status = fs::read_to_string(format!("{base}/{bat}/status")).ok()?;
    let status = status.trim();

    let icon = if status != "Discharging" {
        " 󰂄"
    } else if charge > 90 {
        " 󰁹 "
    } else if charge > 80 {
        " 󰂂 "
    } else if charge > 70 {
        " 󰂁 "
    } else if charge > 60 {
        " 󰂀 "
    } else if charge > 50 {
        " 󰁿 "
    } else if charge > 40 {
        " 󰁾 "
    } else if charge > 30 {
        " 󰁽 "
    } else if charge > 20 {
        " 󰁼 "
    } else if charge > 10 {
        " 󰁻 "
    } else {
        " 󰁺 "
    };

    Some(format!("{icon} {charge}%"))
}

fn ram_text() -> Option<String> {
    let out = spawn_for_output_with_args("free", &["-h"]).ok()?;
    let used = out.lines().find(|l| l.starts_with("Mem:"))?.split_whitespace().nth(2)?;
    let used = used.trim_end_matches('i');
    Some(format!(" 󰍛  {used}"))
}

fn cpu_text() -> Option<String> {
    let read = || -> Option<(u64, u64, u64, u64)> {
        let s = fs::read_to_string("/proc/stat").ok()?;
        let first = s.lines().next()?;
        let mut it = first.split_whitespace().skip(1).filter_map(|x| x.parse::<u64>().ok());
        Some((it.next()?, it.next()?, it.next()?, it.next()?))
    };

    let (a1, b1, c1, idle1) = read()?;
    std::thread::sleep(Duration::from_millis(500));
    let (a2, b2, c2, idle2) = read()?;

    let prev_total = a1 + b1 + c1 + idle1;
    let total = a2 + b2 + c2 + idle2;
    let diff_total = total.saturating_sub(prev_total);
    if diff_total == 0 {
        return Some("  0%".to_string());
    }
    let diff_idle = idle2.saturating_sub(idle1);
    let cpu = 100 * (diff_total.saturating_sub(diff_idle)) / diff_total;

    Some(format!("  {cpu}%"))
}

fn keyboard_text() -> Option<String> {
    let out = spawn_for_output_with_args("setxkbmap", &["-query"]).ok()?;
    let layout = out.lines().find(|l| l.starts_with("layout:"))?.split_whitespace().nth(1)?;
    Some(format!(" 󰌌  {}", layout.to_uppercase()))
}

fn network_text() -> Option<String> {
    let base = std::path::Path::new("/sys/class/net");
    let iface = fs::read_dir(base)
        .ok()?
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().starts_with("wl"))?;

    let state = fs::read_to_string(iface.path().join("operstate")).ok()?;
    Some(if state.trim() == "up" {
        "   Connected".to_string()
    } else {
        " 󰖪  Disconnected".to_string()
    })
}

fn volume_text() -> Option<String> {
    let muted = spawn_for_output_with_args("pactl", &["get-sink-mute", "@DEFAULT_SINK@"]).ok()?;
    if muted.contains("yes") {
        return Some(" 󰝟  muted".to_string());
    }

    let out = spawn_for_output_with_args("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]).ok()?;
    let vol: u32 = out
        .split_whitespace()
        .find(|s| s.ends_with('%'))?
        .trim_end_matches('%')
        .parse()
        .ok()?;

    let icon = if vol >= 75 {
        ""
    } else if vol >= 50 {
        "󰕾"
    } else if vol >= 25 {
        "󰖀"
    } else {
        "󰕿"
    };

    Some(format!(" {icon}  {vol}%"))
}

fn clock_text() -> Option<String> {
    let out = spawn_for_output_with_args("date", &["+  %-d %b %Y   %I:%M %P"]).ok()?;
    Some(out.trim().to_string())
}

// -- tag colors, matching config.def.h's SchemeTag1..SchemeTag0 -----------

struct TagUi;

impl WorkspacesUi for TagUi {
    fn background_color(&self) -> Color {
        Color::from(BLACK)
    }

    fn colors_for_workspace(
        &self,
        meta: &WsMeta,
        focus_state: FocusState,
        screen_has_focus: bool,
    ) -> (Color, Color) {
        const TAG_COLORS: [u32; 10] = [
            CYAN_BRIGHT,
            RED_BRIGHT,
            ORANGE_BRIGHT,
            GREEN_BRIGHT,
            PINK_BRIGHT,
            CYAN_BRIGHT,
            RED_BRIGHT,
            ORANGE_BRIGHT,
            GREEN_BRIGHT,
            PINK_BRIGHT,
        ];
        const TAGS: [&str; 10] = ["一", "二", "三", "四", "五", "六", "七", "八", "九", "零"];

        let idx = TAGS.iter().position(|t| *t == meta.tag()).unwrap_or(0);
        let accent = Color::from(TAG_COLORS[idx]);
        let fg = Color::from(FOREGROUND);
        let bg = Color::from(BLACK);

        match focus_state {
            FocusState::FocusedOnThisScreen if screen_has_focus => (fg, accent),
            FocusState::FocusedOnThisScreen => (accent, bg),
            FocusState::FocusedOnOtherScreen => (accent, bg),
            FocusState::Unfocused if meta.occupied() => (accent, bg),
            FocusState::Unfocused => (Color::from(0x434C5Eff), bg),
        }
    }
}

/// Build the status bar and wire it into the given key bindings' spawn targets
/// (middle-click on the status text opens a terminal, as in flexiflow's
/// `ClkStatusText` button binding).
pub fn build() -> StatusBar<RustConn> {
    let tags_style = TextStyle {
        fg: Color::from(FOREGROUND),
        bg: Some(Color::from(BLACK)),
        padding: (6, 6),
    };

    let widgets: Vec<Box<dyn Widget<RustConn>>> = vec![
        Box::new(WorkspacesWidget::new_with_ui(TagUi)),
        Box::new(CurrentLayout::new(tags_style.clone())),
        Box::new(ActiveWindowName::new(60, tags_style, true, false)),
        interval(PURPLE_BRIGHT, 900, updates_text),
        text("", BLACK),
        interval(CYAN_BRIGHT, 3600, kernel_text),
        text("", BLACK),
        interval(GREEN_BRIGHT, 10, battery_text),
        text("", BLACK),
        interval(YELLOW_BRIGHT, 5, ram_text),
        text("", BLACK),
        interval(ORANGE_BRIGHT, 2, cpu_text),
        text("", BLACK),
        interval(PINK_BRIGHT, 30, keyboard_text),
        text("", BLACK),
        interval(PURPLE_BRIGHT, 3, network_text),
        text("", BLACK),
        interval(CYAN_BRIGHT, 2, volume_text),
        text("", BLACK),
        interval(GREEN_BRIGHT, 1, clock_text),
    ];

    StatusBar::try_new(Position::Top, BAR_HEIGHT_PX, Color::from(BLACK), FONT, POINT_SIZE, widgets)
        .expect("failed to create status bar")
}

/// EWMH hooks: not part of flexiflow_bar, but needed for general app
/// compatibility (taskbars, pagers, `_NET_WM_*` aware apps) the way dwm's
/// built-in EWMH support implicitly provided it.
pub fn with_ewmh(config: penrose::core::Config<RustConn>) -> penrose::core::Config<RustConn> {
    add_ewmh_hooks(config)
}
