//! Port of the `keys[]` table in config.def.h.
//!
//! Not ported (no clean penrose equivalent, or dwm-bitmask-tag specific):
//! - `toggleview`/`toggletag` (dwm tags are a bitmask so several can be shown
//!   at once; penrose workspaces are exclusive-view, XMonad style).
//! - `togglealttag`, `hidewin`/`restorewin`, dynamic `setborderpx`.
//! - `dragmfact`/`dragcfact` (mouse-drag resize) and the two-stage "chain"
//!   key prefixes (e.g. Print then w/s/f) — the chained actions below are
//!   bound directly to their own modifier combo instead of needing a prefix
//!   keypress first.
use crate::state::SharedState;
use penrose::{
    builtin::actions::{exit, modify_with, spawn},
    core::bindings::KeyEventHandler,
    extensions::hooks::ToggleNamedScratchPad,
    map,
    x11rb::RustConn,
};
use std::collections::HashMap;

const TAGS: [&str; 10] = ["一", "二", "三", "四", "五", "六", "七", "八", "九", "零"];

pub fn build(
    state: SharedState,
    toggle_term: ToggleNamedScratchPad,
    toggle_btop: ToggleNamedScratchPad,
    toggle_pavu: ToggleNamedScratchPad,
) -> HashMap<String, Box<dyn KeyEventHandler<RustConn>>> {
    macro_rules! gaps {
        ($method:ident $(, $arg:expr)?) => {{
            let state = state.clone();
            modify_with(move |_| state.$method($($arg)?))
        }};
    }

    let mut b = map! {
        map_keys: |k: &str| k.to_owned();

        // -- focus / stack -------------------------------------------------
        "M-j" => modify_with(|cs| cs.focus_down()),
        "M-k" => modify_with(|cs| cs.focus_up()),
        "M-S-j" => modify_with(|cs| cs.swap_down()),
        "M-S-k" => modify_with(|cs| cs.swap_up()),
        "M-q" => modify_with(|cs| cs.kill_focused()),
        "M-Tab" => modify_with(|cs| cs.focus_down()),
        "M-S-space" => modify_with(|cs| { if let Some(&id) = cs.current_client() { cs.float(id, cs.current_screen().geometry()).ok(); } }),

        // -- layout ----------------------------------------------------------
        "M-space" => modify_with(|cs| cs.next_layout()),
        "M-C-comma" => modify_with(|cs| cs.previous_layout()),
        "M-C-period" => modify_with(|cs| cs.next_layout()),
        "M-h" => gaps!(set_mfact, -0.05),
        "M-l" => gaps!(set_mfact, 0.05),
        "M-i" => gaps!(incr_nmaster, 1),
        "M-u" => gaps!(incr_nmaster, -1),
        "M-S-h" => {
            let state = state.clone();
            modify_with(move |cs| { if let Some(&id) = cs.current_client() { state.set_cfact(id, 0.25); } })
        },
        "M-S-l" => {
            let state = state.clone();
            modify_with(move |cs| { if let Some(&id) = cs.current_client() { state.set_cfact(id, -0.25); } })
        },
        "M-S-o" => {
            let state = state.clone();
            modify_with(move |cs| { if let Some(&id) = cs.current_client() { state.set_cfact(id, 0.0); } })
        },

        // -- gaps --------------------------------------------------------
        "M-S-u" => gaps!(incr_gaps, 1),
        "M-C-S-u" => gaps!(incr_gaps, -1),
        "M-S-i" => gaps!(incr_inner_gaps, 1),
        "M-C-S-i" => gaps!(incr_inner_gaps, -1),
        "M-C-o" => gaps!(incr_outer_gaps, 1),
        "M-C-S-o" => gaps!(incr_outer_gaps, -1),
        "M-C-Up" => gaps!(incr_outer_h_gaps, 1),
        "M-C-S-Up" => gaps!(incr_outer_h_gaps, -1),
        "M-C-Right" => gaps!(incr_outer_v_gaps, 1),
        "M-C-S-Right" => gaps!(incr_outer_v_gaps, -1),
        "M-C-Down" => gaps!(incr_inner_h_gaps, 1),
        "M-C-S-Down" => gaps!(incr_inner_h_gaps, -1),
        "M-C-Left" => gaps!(incr_inner_v_gaps, 1),
        "M-C-S-Left" => gaps!(incr_inner_v_gaps, -1),
        "M-C-S-d" => gaps!(default_gaps),
        "M-g" => gaps!(toggle_gaps),

        // -- screens / tags --------------------------------------------------
        "M-bracketright" => modify_with(|cs| cs.next_screen()),
        "M-bracketleft" => modify_with(|cs| cs.previous_screen()),
        "M-comma" => modify_with(|cs| cs.previous_screen()),
        "M-period" => modify_with(|cs| cs.next_screen()),
        "M-S-comma" => modify_with(|cs| cs.move_focused_to_screen(0)),
        "M-S-period" => modify_with(|cs| cs.move_focused_to_screen(1)),

        // -- scratchpads (Alt, no Super, matching the original `{A, -1, ...}` binds) --
        "A-Return" => Box::new(toggle_term),
        "A-b" => Box::new(toggle_btop),
        "A-p" => Box::new(toggle_pavu),

        // -- spawns, 1:1 with config.def.h ------------------------------------
        "M-Return" => spawn("alacritty"),
        "M-S-Return" => spawn("nemo"),
        "M-m" => spawn("menu"),
        "M-p" => spawn("passmenu"),
        "M-F1" => spawn("clipmenu_run"),
        "M-F2" => spawn("youtube_subs"),
        "M-S-e" => spawn("powermenu_run"),
        "M-C-h" => spawn("gnome_boxes"),
        "M-C-b" => spawn("google-chrome-stable"),
        "M-C-e" => spawn("code"),
        "M-C-d" => spawn("discord"),
        "M-C-x" => spawn("hexchat"),
        "M-C-i" => spawn("inkscape"),
        "M-C-k" => spawn("krita"),
        "M-C-p" => spawn("pkgbrowser"),
        "M-A-p" => spawn("piper"),
        "M-S-z" => spawn("multimonitorlock-gui"),
        "M-Print" => spawn("win-shot -w"),
        "M-S-Print" => spawn("win-shot -s"),
        "M-C-Print" => spawn("win-shot -f"),
        "M-A-s" => spawn("prime-run steam"),
        "M-A-m" => spawn("prime-run minecraft-launcher"),
        "M-A-r" => spawn("prime-run runescape-launcher"),

        // -- XF86 media/brightness keys --------------------------------------
        "XF86MonBrightnessUp" => spawn("brightnessctl set 5%+"),
        "XF86MonBrightnessDown" => spawn("brightnessctl set 5%-"),
        "XF86AudioLowerVolume" => spawn("amixer set Master 5%- unmute"),
        "XF86AudioRaiseVolume" => spawn("amixer set Master 5%+ unmute"),
        "XF86AudioMute" => spawn("amixer set Master toggle"),
        "XF86AudioPlay" => spawn("playerctl play-pause"),
        "XF86AudioPause" => spawn("playerctl play-pause"),
        "XF86AudioNext" => spawn("playerctl next"),
        "XF86AudioPrev" => spawn("playerctl previous"),

        // -- session -----------------------------------------------------
        "M-S-r" => exit(),
        "M-S-q" => exit(),
    };

    for (i, tag) in TAGS.iter().enumerate() {
        let _ = i;
        b.insert(format!("M-{}", tag_key(i)), modify_with(move |cs| cs.focus_tag(*tag)));
        b.insert(
            format!("M-S-{}", tag_key(i)),
            modify_with(move |cs| cs.move_focused_to_tag(*tag)),
        );
    }

    b
}

/// dwm's `TAGKEYS` binds tag N to the physical number key N (with `0` for the
/// tenth tag) regardless of what the tag is actually *named* (flexiflow names
/// them with CJK numerals) — keep that same physical-key mapping here.
fn tag_key(i: usize) -> &'static str {
    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"][i]
}
