//! flexiflow-rs: a penrose port of the flexiflow (dwm fork) config in the
//! sibling `FlexiFlow` C project. See README.md for what is and isn't carried
//! over from config.def.h.
mod bar;
mod keys;
mod layouts;
mod rules;
mod state;

use layouts::Kind;
use penrose::{
    Color, Result,
    builtin::layout::transformers::ReserveTop,
    core::{Config, State, WindowManager, bindings::parse_keybindings_with_xmodmap, layout::LayoutStack},
    extensions::hooks::{NamedScratchPad, add_named_scratchpads, manage::FloatingCentered},
    pure::{Stack, geometry::Rect},
    x11rb::RustConn,
    x::{Atom, Prop, WinType, XConn, query::ClassName},
};
use state::SharedState;
use std::collections::HashMap;
use tracing_subscriber::prelude::*;

const TAGS: [&str; 10] = ["一", "二", "三", "四", "五", "六", "七", "八", "九", "零"];

/// The name reported for `_NET_WM_NAME` on the EWMH check window — this is
/// what tools like fastfetch (and `xprop -root _NET_SUPPORTING_WM_CHECK`)
/// read as the running window manager's name. flexiflow.c sets this to
/// "flexiflow" the same way (see its `wmcheckwin` setup); `add_ewmh_hooks`
/// from penrose instead hardcodes a plain (non-EWMH-check-window) "penrose"
/// WM_NAME on the root window, which fastfetch's detection doesn't look at.
const WM_NAME: &str = "flexiflow-rs";

/// Create the dedicated, unmapped "check window" EWMH expects and advertise
/// it via `_NET_SUPPORTING_WM_CHECK` on both the check window and the root
/// window, with `_NET_WM_NAME` set to `WM_NAME` on the check window.
fn set_wm_name_hook(_state: &mut State<RustConn>, x: &mut RustConn) -> Result<()> {
    let root = x.root();
    let check_win = x.create_window(WinType::CheckWin, Rect::new(0, 0, 1, 1), false)?;

    x.set_prop(
        check_win,
        Atom::NetWmName.as_ref(),
        Prop::UTF8String(vec![WM_NAME.to_owned()]),
    )?;
    x.set_prop(
        check_win,
        Atom::NetSupportingWmCheck.as_ref(),
        Prop::Window(vec![check_win]),
    )?;
    x.set_prop(
        root,
        Atom::NetSupportingWmCheck.as_ref(),
        Prop::Window(vec![check_win]),
    )
}

/// Subset of config.def.h's `autostart[]` worth spawning unconditionally;
/// anything spawned here that isn't installed just fails silently (same as
/// dwm's runAutostart, which doesn't check either).
const AUTOSTART: &[&[&str]] = &[
    &["setxkbmap", "-option", "caps:escape"],
    &["numlockx", "on"],
    &["dunst"],
    &["clipmenu-daemon"],
    &["nm-applet"],
    &["blueman-applet"],
    &["pasystray"],
    &["cbatticon"],
];

fn layout_stack(state: SharedState) -> LayoutStack {
    let kinds = [
        Kind::Tile,
        Kind::Monocle,
        Kind::Spiral,
        Kind::Dwindle,
        Kind::Deck,
        Kind::BStack,
        Kind::BStackHoriz,
        Kind::Grid,
        Kind::NRowGrid,
        Kind::HorizGrid,
        Kind::GaplessGrid,
        Kind::CenteredMaster,
        Kind::CenteredFloatingMaster,
    ];

    let layouts = kinds
        .into_iter()
        .map(|k| ReserveTop::wrap(layouts::DwmLayout::new(k, state.clone()), bar::BAR_HEIGHT_PX));

    Stack::try_from_iter(layouts).expect("at least one layout")
}

fn main() -> Result<()> {
    // For information fetchers (fastfetch, neofetch, ...): flexiflow.c does the
    // exact same three `setenv` calls (with "FlexiFlow" instead) in its own
    // `runAutostart`, and child processes we spawn (autostart apps, your shell,
    // anything launched from a terminal under this session) inherit these.
    // SAFETY: called once, single-threaded, before any other threads exist.
    unsafe {
        std::env::set_var("XDG_CURRENT_DESKTOP", WM_NAME);
        std::env::set_var("XDG_SESSION_DESKTOP", WM_NAME);
        std::env::set_var("DESKTOP_SESSION", WM_NAME);
    }

    tracing_subscriber::fmt()
        .with_env_filter("info")
        .finish()
        .init();

    let shared_state = SharedState::new();

    let (nsp_term, toggle_term) = NamedScratchPad::new(
        "terminal",
        "alacritty --title=scratchpad_terminal --class=scratchpad_terminal,scratchpad_terminal",
        ClassName("scratchpad_terminal"),
        FloatingCentered::new(0.7, 0.7),
        true,
    );
    let (nsp_btop, toggle_btop) = NamedScratchPad::new(
        "btop",
        "alacritty --class=scratchpad_btop,scratchpad_btop -e btop",
        ClassName("scratchpad_btop"),
        FloatingCentered::new(0.7, 0.7),
        true,
    );
    let (nsp_pavu, toggle_pavu) = NamedScratchPad::new(
        "pavucontrol",
        "pavucontrol",
        ClassName("Pavucontrol"),
        FloatingCentered::new(0.6, 0.6),
        true,
    );

    let key_bindings: HashMap<_, _> =
        keys::build(shared_state.clone(), toggle_term, toggle_btop, toggle_pavu);
    let key_bindings = parse_keybindings_with_xmodmap(key_bindings)?;

    let config = Config {
        normal_border: Color::from(0x434C5Eff),
        focused_border: Color::from(0xFF59F9ff),
        border_width: 2,
        default_layouts: layout_stack(shared_state),
        tags: TAGS.iter().map(|t| t.to_string()).collect(),
        floating_classes: vec![
            "Multimonitorlock-gui.py".to_string(),
            "floatmenu".to_string(),
            "Yad".to_string(),
            "Peek".to_string(),
            "Snapper".to_string(),
        ],
        manage_hook: Some(Box::new(rules::apply_rules::<RustConn>)),
        startup_hook: Some(Box::new(|_state: &mut penrose::core::State<RustConn>, _conn: &mut RustConn| {
            for cmd in AUTOSTART {
                let _ = penrose::util::spawn_with_args(cmd[0], &cmd[1..]);
            }
            Ok(())
        })),
        ..Config::default()
    };
    let mut config = bar::with_ewmh(config);
    config.compose_or_set_startup_hook(set_wm_name_hook);

    let conn = RustConn::new()?;
    let mut wm = WindowManager::new(config, key_bindings, HashMap::new(), conn)?;

    add_named_scratchpads(&mut wm, vec![nsp_term, nsp_btop, nsp_pavu]);

    let wm = bar::build().add_to(wm);

    wm.run()
}
