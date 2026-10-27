//! The Lua config, read the same way the X11 FlexiFlow reads it: the
//! baked-in [`DEFAULT_LUA`] runs first (defining `key`, `rule`, ... and a
//! default `flexiflow` table), then the user's file, and whatever
//! `flexiflow` ends up as is the config. An X11 FlexiFlow config.lua
//! loads unchanged; the settings that only meant something under X
//! (`resize_hints`, `snap`, ...) are read and ignored.
use std::{
    collections::HashMap,
    error::Error,
    fs::{create_dir_all, read_to_string, write},
    path::{Path, PathBuf},
};

use mlua::{Lua, LuaSerdeExt as _};

pub mod keysyms;

/// The baked-in default config, also used to seed
/// `~/.config/flexiflow/config.lua` on first run.
pub const DEFAULT_LUA: &str = include_str!("config.lua");

/// The functions a key or button can be bound to, installed in Lua as
/// strings naming themselves (`view` is `"view"`).
pub const HANDLERS: [&str; 38] = [
    "focusmon", "focusstack", "pushstack", "incnmaster", "killclient",
    "quit", "restart", "setlayout", "setmfact", "spawn", "tag", "tagview",
    "tagmon", "togglebar", "togglevacant", "togglefloating", "toggletag",
    "toggleview", "view", "zoom", "movemouse", "resizemouse", "fullscreen",
    "togglescratch", "cyclelayout", "layoutmenu", "powermenu", "hotkeys",
    "incrgaps", "incrigaps", "incrogaps", "incrohgaps", "incrovgaps",
    "incrihgaps", "incrivgaps", "defaultgaps", "togglegaps", "setcfact",
];

/// What was clicked, for `buttons`: the bar's parts, or a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Clk {
    TagBar,
    LtSymbol,
    StatusText,
    WinTitle,
    ClientWin,
    RootWin,
    PowerButton,
    MenuButton,
}

const CLICKS: [(&str, Clk); 8] = [
    ("ClkTagBar", Clk::TagBar),
    ("ClkLtSymbol", Clk::LtSymbol),
    ("ClkStatusText", Clk::StatusText),
    ("ClkWinTitle", Clk::WinTitle),
    ("ClkClientWin", Clk::ClientWin),
    ("ClkRootWin", Clk::RootWin),
    ("ClkPowerButton", Clk::PowerButton),
    ("ClkMenuButton", Clk::MenuButton),
];

/// X's modifier masks, which share their values with river's
/// `river_seat_v1.modifiers` (Mod2, num lock, is never part of a binding).
const MODS: [(&str, u32); 7] = [
    ("ShiftMask", 1),
    ("ControlMask", 4),
    ("Mod1Mask", 8),
    ("Mod2Mask", 16),
    ("Mod3Mask", 32),
    ("Mod4Mask", 64),
    ("Mod5Mask", 128),
];

/// X's mouse buttons 1-5; scrolling (4 and 5) only reaches the bar.
const BUTTONS: [(&str, u32); 5] = [
    ("Button1", 1),
    ("Button2", 2),
    ("Button3", 3),
    ("Button4", 4),
    ("Button5", 5),
];

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Arg {
    I(i32),
    Ui(u32),
    F(f32),
    /// argv for `spawn`
    V(Vec<String>),
    /// a `layouts` index for `setlayout`
    L(Option<usize>),
}

impl Default for Arg {
    /// `{L = nil}` reaches serde as an empty table, so a missing or empty
    /// arg is the only optional variant
    fn default() -> Self {
        Arg::L(None)
    }
}

impl Arg {
    pub fn i(&self) -> i32 {
        match *self {
            Arg::I(i) => i,
            Arg::Ui(u) => u as i32,
            Arg::F(f) => f as i32,
            _ => 0,
        }
    }

    pub fn ui(&self) -> u32 {
        match *self {
            Arg::Ui(u) => u,
            Arg::I(i) => i as u32,
            _ => 0,
        }
    }

    pub fn f(&self) -> f32 {
        match *self {
            Arg::F(f) => f,
            Arg::I(i) => i as f32,
            _ => 0.0,
        }
    }
}

/// A handler's name, checked against [`HANDLERS`].
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String")]
pub struct Func(pub String);

impl TryFrom<String> for Func {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if HANDLERS.contains(&value.as_str()) {
            Ok(Func(value))
        } else {
            Err(format!("no key `{value}`"))
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Key {
    pub mod_: u32,
    pub keysym: u32,
    pub func: Func,
    #[serde(default)]
    pub arg: Arg,
    /// what the binding does, listed by `hotkeys`; bindings without one
    /// are left out of that list
    #[serde(default)]
    pub desc: String,
    /// the heading `hotkeys` lists it under
    #[serde(default)]
    pub group: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Button {
    pub click: u32,
    pub mask: u32,
    pub button: u32,
    pub func: Func,
    #[serde(default)]
    pub arg: Arg,
}

impl Button {
    pub fn clicks(&self, click: Clk) -> bool {
        self.click == click as u32
    }
}

/// A rule's `class` is matched against a window's app_id, and `title`
/// against its title, each as a substring like dwm's `strstr`. Wayland
/// has no `instance`; a rule that needs one never matches.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(default)]
pub struct Rule {
    pub class: String,
    pub instance: String,
    pub title: String,
    pub tags: u32,
    pub isfloating: bool,
    pub isterminal: bool,
    pub noswallow: bool,
    pub monitor: i32,
    /// Switch to the matched window's tag when it opens.
    pub focus: bool,
}

impl Rule {
    pub fn matches(&self, app_id: &str, title: &str) -> bool {
        self.instance.is_empty()
            && (self.class.is_empty() || app_id.contains(&self.class))
            && (self.title.is_empty() || title.contains(&self.title))
    }
}

/// One scratchpad slot: `class` is matched against a window's app_id
/// (or, failing that, its title) to recognize it, `cmd` is spawned if it
/// isn't running yet.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ScratchSpec {
    pub class: String,
    pub cmd: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Layout {
    pub symbol: String,
    /// One of [`crate::layouts::ARRANGE`]; none for the floating layout.
    #[serde(default, deserialize_with = "arrange")]
    pub arrange: Option<String>,
    /// fleximenu highlight color for this layout in the layout menu
    #[serde(default)]
    pub color: String,
}

fn arrange<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<String>, D::Error> {
    let name = Option::<String>::deserialize(d)?;
    match name {
        Some(n) if !crate::layouts::ARRANGE.iter().any(|(a, _)| *a == n) => {
            Err(serde::de::Error::custom(format!("no layout `{n}`")))
        }
        n => Ok(n),
    }
}

use serde::Deserialize as _;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct PowerMenu {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub items: Vec<PowerMenuItem>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct PowerMenuItem {
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub color: String,
}

/// The `hotkeys` popup's look: its colors, how many columns it lists
/// the keys in, and the order, headings and accent colors of its groups.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(default)]
pub struct Hotkeys {
    /// the popup's background
    pub bg: String,
    /// descriptions, the title and typed search text
    pub fg: String,
    /// the border, the search placeholder and the separator line
    pub dim: String,
    /// key badges' text
    pub key_fg: String,
    /// key badges' background, tinted with their group's accent
    pub key_bg: String,
    /// how many columns to list the keys in: more if they'd be taller
    /// than the output, fewer if the popup would be wider than it
    pub columns: usize,
    /// the groups in the order they're shown; any not listed follow in
    /// the order they're first bound
    pub groups: Vec<HotkeyGroup>,
}

impl Default for Hotkeys {
    /// KDE's Breeze Dark, like the X11 popup
    fn default() -> Self {
        Self {
            bg: "#202326".into(),
            fg: "#FCFCFC".into(),
            dim: "#A1A9B1".into(),
            key_fg: "#FCFCFC".into(),
            key_bg: "#292C30".into(),
            columns: 4,
            groups: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct HotkeyGroup {
    pub name: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub color: String,
}

/// An `[fg, bg, border]` color triple.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "Vec<String>")]
pub struct Scheme {
    pub fg: String,
    pub bg: String,
    pub border: String,
}

impl TryFrom<Vec<String>> for Scheme {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        let [fg, bg, border] = <[String; 3]>::try_from(value).map_err(|v| {
            format!("expected 3 colors (fg, bg, border), got {}", v.len())
        })?;
        for c in [&fg, &bg, &border] {
            parse_color(c).ok_or_else(|| format!("not a color: {c:?}"))?;
        }
        Ok(Self { fg, bg, border })
    }
}

/// The bar's color schemes. `norm` and `sel` are required; the menu and
/// power buttons and the layout symbol fall back to `norm`, the title
/// to `sel`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(try_from = "HashMap<String, Scheme>")]
pub struct Colors {
    pub norm: Scheme,
    pub sel: Scheme,
    pub layout: Scheme,
    pub power: Scheme,
    pub menu: Scheme,
    pub title: Scheme,
}

impl TryFrom<HashMap<String, Scheme>> for Colors {
    type Error = String;

    fn try_from(mut v: HashMap<String, Scheme>) -> Result<Self, Self::Error> {
        let norm = v.remove("norm").ok_or("missing key norm")?;
        let sel = v.remove("sel").ok_or("missing key sel")?;
        Ok(Self {
            layout: v.remove("layout").unwrap_or_else(|| norm.clone()),
            power: v.remove("power").unwrap_or_else(|| norm.clone()),
            menu: v.remove("menu").unwrap_or_else(|| norm.clone()),
            title: v.remove("title").unwrap_or_else(|| sel.clone()),
            norm,
            sel,
        })
    }
}

/// The keyboard's xkb layout, as `setxkbmap` takes it: `layout = "gb"`,
/// `options = "caps:escape"`, and so on. Every keyboard gets it, ones
/// plugged in later too.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct Keyboard {
    pub layout: String,
    pub variant: String,
    pub model: String,
    pub options: String,
    /// Turn num lock on, as `numlockx on` did under X11.
    pub numlock: bool,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Cursor {
    #[serde(default)]
    pub theme: String,
    #[serde(default = "default_cursor_size")]
    pub size: u32,
}

impl Default for Cursor {
    fn default() -> Self {
        Self { theme: String::new(), size: default_cursor_size() }
    }
}

fn default_cursor_size() -> u32 {
    24
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    /// Border width of windows, in pixels
    pub borderpx: i32,
    /// Whether to show the bar
    pub showbar: bool,
    /// Whether to show the bar at the top or bottom
    pub topbar: bool,
    /// Factor of master area size [0.05..0.95]
    pub mfact: f32,
    /// Number of windows in master area
    pub nmaster: i32,
    /// Default gap size in pixels (outer and inner, horizontal and
    /// vertical all start equal to this)
    #[serde(default = "default_gaps")]
    pub gaps: i32,
    /// Collapse outer gaps to 0 when a tag has a single window
    #[serde(default)]
    pub smartgaps: bool,
    /// Extra vertical padding added to the bar's height, on top of the
    /// font's own height
    #[serde(default = "default_barpadding")]
    pub barpadding: i32,
    /// Keep focus on a fullscreen window
    #[serde(default)]
    pub lock_fullscreen: bool,
    /// Focus whichever window the pointer moves onto, like dwm.
    #[serde(default = "yes")]
    pub focusfollowsmouse: bool,
    pub fonts: Vec<String>,
    pub tags: Vec<String>,
    pub colors: Colors,
    /// Per-tag `[fg, bg, border]` bar colors; a tag past the end of this
    /// list uses `colors.sel`.
    #[serde(default)]
    pub tagcolors: Vec<Scheme>,
    pub keys: Vec<Key>,
    pub rules: Vec<Rule>,
    /// Swallow floating windows by default
    #[serde(default)]
    pub swallowfloating: bool,
    pub buttons: Vec<Button>,
    pub layouts: Vec<Layout>,
    #[serde(default)]
    pub scratchpads: Vec<ScratchSpec>,
    /// Commands (argv lists) to spawn once on startup.
    #[serde(default, deserialize_with = "river_argvs")]
    pub autostart: Vec<Vec<String>>,
    /// The bar program, spawned on startup and on `restart`; empty for
    /// no bar.
    #[serde(default = "default_barcmd")]
    pub barcmd: Vec<String>,
    #[serde(default)]
    pub powermenu: PowerMenu,
    /// The symbol of the bar's menu button, drawn left of the tags.
    #[serde(default)]
    pub menubutton: String,
    /// The dmenu-style menu the layout, power and hotkeys menus open in;
    /// `-p <prompt>` and `-l <lines>` are added for each.
    #[serde(default = "default_menu")]
    pub menu: Vec<String>,
    /// The command run when a named area of the status text
    /// (`^a<name>^`) is clicked, with the area's name and the mouse
    /// button (1-5) added.
    #[serde(default = "default_statusclick", deserialize_with = "river_argv")]
    pub statusclick: Vec<String>,
    #[serde(default)]
    pub hotkeys: Hotkeys,
    /// More of the `hotkeys` group order, after `hotkeys.groups`.
    #[serde(default)]
    pub hotkeys_groups: Vec<String>,
    #[serde(default)]
    pub cursor: Cursor,
    /// The keyboard layout and options; river's default (US) without it.
    #[serde(default)]
    pub keyboard: Option<Keyboard>,
    /// Whether the bar shows the system tray (StatusNotifierItems).
    #[serde(default = "yes")]
    pub showsystray: bool,
    /// 0: the tray follows the focused output; n: it stays on the n-th
    /// output from the left.
    #[serde(default)]
    pub systraypinning: usize,
    /// When pinned to an output that isn't there: the first output if
    /// true, else the last.
    #[serde(default = "yes")]
    pub systraypinningfailfirst: bool,
    /// The tray left of the status text instead of right of it.
    #[serde(default)]
    pub systrayonleft: bool,
    /// Pixels between tray icons.
    #[serde(default = "default_systrayspacing")]
    pub systrayspacing: i32,
    /// The tray icons' height in pixels, centred in the bar; 0, or more
    /// than the bar's height, for the bar's height.
    #[serde(default)]
    pub systrayiconsize: i32,
}

fn default_systrayspacing() -> i32 {
    2
}

/// The X11 FlexiFlow's status program, and ours. A config shared with
/// it names `flexiflow_bar`, which only knows how to talk to X11.
const X11_STATUS: &str = "flexiflow_bar";
const RIVER_STATUS: &str = "flexiflow-river-status";

fn to_river(mut argv: Vec<String>) -> Vec<String> {
    if let Some(cmd) = argv.first_mut()
        && cmd == X11_STATUS
    {
        *cmd = RIVER_STATUS.to_string();
    }
    argv
}

fn river_argv<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    Ok(to_river(Vec::deserialize(d)?))
}

fn river_argvs<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Vec<String>>, D::Error> {
    Ok(Vec::<Vec<String>>::deserialize(d)?.into_iter().map(to_river).collect())
}

fn yes() -> bool {
    true
}

fn default_gaps() -> i32 {
    10
}

fn default_barpadding() -> i32 {
    2
}

fn default_menu() -> Vec<String> {
    vec!["fleximenu".to_string()]
}

fn default_statusclick() -> Vec<String> {
    vec!["flexiflow-river-status".to_string(), "--click".to_string()]
}

fn default_barcmd() -> Vec<String> {
    vec!["flexiflow-river-bar".to_string()]
}

/// `#RRGGBB` (or `#RRGGBBAA`) as `(r, g, b, a)`.
pub fn parse_color(s: &str) -> Option<(u8, u8, u8, u8)> {
    let hex = s.strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    match hex.len() {
        6 => Some((byte(0)?, byte(2)?, byte(4)?, 0xff)),
        8 => Some((byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
        _ => None,
    }
}

fn new_lua() -> Lua {
    let lua = Lua::new();
    let globals = lua.globals();
    // which window manager is reading the config, so one config.lua
    // shared with the X11 FlexiFlow can pick per-session commands
    globals.set("wm", "river").unwrap();
    for name in HANDLERS.iter().chain(crate::layouts::ARRANGE.iter().map(|(n, _)| n)) {
        globals.set(*name, *name).unwrap();
    }
    for (name, value) in MODS.iter().chain(&BUTTONS).chain(&keysyms::KEYSYMS) {
        globals.set(*name, *value).unwrap();
    }
    for (name, clk) in CLICKS {
        globals.set(name, clk as u32).unwrap();
    }
    lua.load(DEFAULT_LUA).set_name("default config.lua").exec().unwrap();
    lua
}

impl Config {
    pub fn from_lua(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let lua = new_lua();
        let path = path.as_ref();
        lua.load(read_to_string(path)?)
            .set_name(path.to_string_lossy())
            .exec()?;
        Ok(lua.from_value(lua.globals().get("flexiflow")?)?)
    }

    /// `~/.config/flexiflow/config.lua`, the same file the X11
    /// FlexiFlow reads, so one config drives both.
    pub fn path() -> Option<PathBuf> {
        let base = match std::env::var("XDG_CONFIG_HOME") {
            Ok(x) if !x.is_empty() => PathBuf::from(x),
            _ => PathBuf::from(std::env::var("HOME").ok()?).join(".config"),
        };
        Some(base.join("flexiflow").join("config.lua"))
    }

    /// The user's config, seeding it with the default on first run; the
    /// default if it can't be read.
    pub fn load_home() -> Self {
        let Some(path) = Self::path() else {
            log::warn!("unable to determine config directory");
            return Self::default();
        };
        if !path.exists() {
            seed_default_config(&path);
            return Self::default();
        }
        Self::from_lua(&path).unwrap_or_else(|e| {
            log::error!("failed to read config file {path:?}: {e}");
            Self::default()
        })
    }

    /// The `[fg, bg, border]` colors tag `i` is drawn in while selected.
    pub fn tagscheme(&self, i: usize) -> &Scheme {
        self.tagcolors.get(i).unwrap_or(&self.colors.sel)
    }
}

impl Default for Config {
    fn default() -> Self {
        let lua = new_lua();
        lua.from_value(lua.globals().get("flexiflow").unwrap()).unwrap()
    }
}

fn seed_default_config(path: &Path) {
    let Some(dir) = path.parent() else { return };
    match create_dir_all(dir).and_then(|()| write(path, DEFAULT_LUA)) {
        Ok(()) => log::info!("wrote default config to {path:?}"),
        Err(e) => log::warn!("could not write default config to {path:?}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_config_loads() {
        let c = Config::default();
        assert_eq!(c.tags.len(), 10);
        assert!(c.keys.iter().any(|k| k.func.0 == "view"));
        assert_eq!(c.layouts[0].arrange.as_deref(), Some("tile"));
    }

    #[test]
    fn an_x11_flexiflow_config_loads_unchanged() {
        let dir = std::env::temp_dir()
            .join(format!("flexiflow-river-x11-config-{}", std::process::id()));
        create_dir_all(&dir).unwrap();
        let path = dir.join("config.lua");
        let x11 = include_str!("../tests/x11-config.lua");
        write(&path, x11).unwrap();
        let got = Config::from_lua(&path);
        let _ = std::fs::remove_dir_all(&dir);
        let got = got.unwrap();
        assert!(got.keys.len() > 50);
        // its status program is swapped for ours
        assert!(got.autostart.contains(&vec!["flexiflow-river-status".to_string()]));
        assert!(!got.autostart.iter().any(|a| a[0] == "flexiflow_bar"));
    }

    #[test]
    fn colors_parse() {
        assert_eq!(parse_color("#FF59F9"), Some((0xff, 0x59, 0xf9, 0xff)));
        assert_eq!(parse_color("FF59F9"), None);
        assert_eq!(parse_color("#12345"), None);
    }
}
