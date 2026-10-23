//! Port of the `rules[]` table in config.def.h: assign newly-managed clients
//! to a tag based on their WM_CLASS (or title, for a couple of entries),
//! optionally switching focus to that tag. The three scratchpad-only entries
//! (`scratchpad_terminal`, `scratchpad_btop`, `Pavucontrol`) are *not* here:
//! they are registered as `NamedScratchPad`s instead (see src/main.rs), which
//! already handles ownership of those windows.
use penrose::{
    Result, WinId,
    core::{State, conn::Query},
    x::{
        XConn,
        query::{ClassName, Title},
    },
};

struct Rule {
    class: Option<&'static str>,
    title: Option<&'static str>,
    tag: &'static str,
    switch_to_tag: bool,
}

const TAGS: [&str; 10] = ["一", "二", "三", "四", "五", "六", "七", "八", "九", "零"];

const RULES: &[Rule] = &[
    Rule { class: Some("Alacritty"), title: None, tag: TAGS[0], switch_to_tag: true },
    Rule { class: Some("kitty"), title: None, tag: TAGS[0], switch_to_tag: true },
    Rule { class: Some("Code"), title: None, tag: TAGS[1], switch_to_tag: true },
    Rule { class: Some("nvim"), title: None, tag: TAGS[1], switch_to_tag: true },
    Rule { class: Some("jetbrains-studio"), title: None, tag: TAGS[1], switch_to_tag: true },
    Rule { class: Some("Nemo"), title: None, tag: TAGS[2], switch_to_tag: true },
    Rule { class: Some("Ld-linux-x86-64.so.2"), title: None, tag: TAGS[3], switch_to_tag: true },
    Rule { class: Some("Gimp"), title: None, tag: TAGS[3], switch_to_tag: true },
    Rule { class: Some("firefox"), title: None, tag: TAGS[5], switch_to_tag: true },
    Rule { class: Some("Opera"), title: None, tag: TAGS[5], switch_to_tag: true },
    Rule { class: Some("Steam"), title: None, tag: TAGS[7], switch_to_tag: true },
    Rule { class: Some("RuneScape"), title: None, tag: TAGS[7], switch_to_tag: true },
    Rule { class: Some("discord"), title: None, tag: TAGS[8], switch_to_tag: true },
    Rule { class: Some("Hexchat"), title: None, tag: TAGS[8], switch_to_tag: true },
    Rule { class: Some("PkgBrowser"), title: None, tag: TAGS[9], switch_to_tag: true },
    Rule { class: None, title: Some("Event Tester"), tag: TAGS[9], switch_to_tag: true },
    Rule { class: None, title: Some("lazygit"), tag: TAGS[9], switch_to_tag: true },
];

/// The `manage_hook` for `Config`: a plain `fn` already satisfies penrose's
/// blanket `ManageHook` impl for `FnMut(WinId, &mut State<C>, &mut C) -> Result<()>`.
pub fn apply_rules<X: XConn>(id: WinId, state: &mut State<X>, x: &mut X) -> Result<()> {
    for rule in RULES {
        let matched = match (rule.class, rule.title) {
            (Some(class), _) => ClassName(class).run(id, x)?,
            (None, Some(title)) => Title(title).run(id, x)?,
            _ => false,
        };

        if matched {
            state.client_set.move_client_to_tag(&id, rule.tag);
            if rule.switch_to_tag {
                state.client_set.focus_tag(rule.tag);
            }
            break;
        }
    }

    Ok(())
}
