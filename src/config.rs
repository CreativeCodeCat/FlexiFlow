use std::{
    collections::HashMap,
    error::Error,
    ffi::{CString, c_float, c_int, c_uint},
    fs::{create_dir_all, read_to_string, write},
    path::{Path, PathBuf},
};

use env::{CLICKS, HANDLERS, KEYS, XKEYS};
use mlua::{Lua, LuaSerdeExt as _, Table};

use crate::{
    Button, Hotkeys, Layout, PowerMenu, Rule, ScratchSpec, config::key::Key,
    enums::Scheme,
};

mod env;
pub mod key;

#[derive(Debug, serde::Deserialize)]
#[serde(try_from = "HashMap<String, Vec<String>>")]
pub struct ColorMap(pub Vec<Vec<CString>>);

impl TryFrom<HashMap<String, Vec<String>>> for ColorMap {
    type Error = Box<dyn Error>;

    fn try_from(
        mut value: HashMap<String, Vec<String>>,
    ) -> Result<Self, Self::Error> {
        let mut ret = vec![vec![]; 6];
        let norm = value.remove("norm").ok_or("missing key norm")?;
        let sel = value.remove("sel").ok_or("missing key sel")?;

        if norm.len() != 3 {
            return Err("not enough colors for SchemeNorm".into());
        };
        ret[Scheme::Norm as usize] = norm
            .into_iter()
            .map(CString::new)
            .collect::<Result<_, _>>()?;

        if sel.len() != 3 {
            return Err("not enough colors for SchemeSel".into());
        };
        ret[Scheme::Sel as usize] = sel
            .into_iter()
            .map(CString::new)
            .collect::<Result<_, _>>()?;

        // the bar's two buttons and window title are optional: without
        // their own colors, the buttons are drawn like the rest of the bar
        // (SchemeNorm) and the title like the selection (SchemeSel)
        for (key, scheme, fallback) in [
            ("layout", Scheme::Layout, Scheme::Norm),
            ("power", Scheme::Power, Scheme::Norm),
            ("menu", Scheme::Menu, Scheme::Norm),
            ("title", Scheme::Title, Scheme::Sel),
        ] {
            ret[scheme as usize] = match value.remove(key) {
                Some(colors) if colors.len() != 3 => {
                    return Err(format!(
                        "colors.{key}: expected 3 colors (fg, bg, border), got {}",
                        colors.len()
                    )
                    .into());
                }
                Some(colors) => colors
                    .into_iter()
                    .map(CString::new)
                    .collect::<Result<_, _>>()?,
                None => ret[fallback as usize].clone(),
            };
        }

        Ok(Self(ret))
    }
}

/// Per-tag `[fg, bg, border]` color triples - lets each workspace get its
/// own accent color in the bar instead of every tag sharing one `SchemeSel`
/// color. Indexed by tag number; a tag past the end of this list (or when
/// the list is empty) just falls back to the normal `Scheme::Sel` color.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(try_from = "Vec<Vec<String>>")]
pub struct TagColors(pub Vec<Vec<CString>>);

impl TryFrom<Vec<Vec<String>>> for TagColors {
    type Error = Box<dyn Error>;

    fn try_from(value: Vec<Vec<String>>) -> Result<Self, Self::Error> {
        let mut ret = Vec::with_capacity(value.len());
        for (i, entry) in value.into_iter().enumerate() {
            if entry.len() != 3 {
                return Err(format!(
                    "tagcolors[{i}]: expected 3 colors (fg, bg, border), got {}",
                    entry.len()
                )
                .into());
            }
            ret.push(
                entry
                    .into_iter()
                    .map(CString::new)
                    .collect::<Result<_, _>>()?,
            );
        }
        Ok(Self(ret))
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    /// Border pixel of windows
    pub borderpx: c_uint,

    /// Snap pixel
    pub snap: c_uint,

    /// Whether to show the bar
    pub showbar: bool,

    /// Whether to show the bar at the top or bottom
    pub topbar: bool,

    /// Factor of master area size [0.05..0.95]
    pub mfact: c_float,

    /// Number of clients in master area
    pub nmaster: c_int,

    /// Default gap size in pixels (outer and inner, horizontal and
    /// vertical all start equal to this; each can be adjusted
    /// independently afterwards)
    #[serde(default = "default_gaps")]
    pub gaps: c_int,

    /// Collapse outer gaps to 0 when a tag has a single client
    #[serde(default)]
    pub smartgaps: bool,

    /// Extra vertical padding added to the bar's height, on top of the
    /// font's own height - gives the tags/status text room to breathe
    /// instead of sitting flush against the bar's top/bottom edge.
    #[serde(default = "default_barpadding")]
    pub barpadding: c_int,

    /// Respect size hints in tiled resizals
    pub resize_hints: bool,

    /// Force focus on the fullscreen window
    pub lock_fullscreen: bool,

    pub fonts: Vec<CString>,

    pub tags: Vec<String>,

    pub colors: ColorMap,

    /// Per-tag bar colors (see [`TagColors`]); optional, defaults to empty
    /// (every tag falls back to `Scheme::Sel`/`Scheme::Norm`).
    #[serde(default)]
    pub tagcolors: TagColors,

    pub keys: Vec<Key>,

    pub rules: Vec<Rule>,

    /// Swallow floating windows by default
    pub swallowfloating: bool,

    /// 0: sloppy systray follows selected monitor, >0: pin systray to monitor x
    pub systraypinning: c_uint,

    pub systrayonleft: bool,

    pub systrayspacing: c_uint,

    /// if pinning fails and this is true, display systray on the first monitor,
    /// else display systray on the last monitor
    pub systraypinningfailfirst: bool,

    pub showsystray: bool,

    pub buttons: Vec<Button>,

    pub layouts: Vec<Layout>,

    /// Scratchpad slots, generalized from upstream's single
    /// `scratchpadname`/`scratchpadcmd` - see `ScratchSpec`.
    #[serde(default)]
    pub scratchpads: Vec<ScratchSpec>,

    /// Commands (argv lists) to spawn once on startup.
    #[serde(default)]
    pub autostart: Vec<Vec<String>>,

    /// The bar's power button and its menu - see `PowerMenu`. Optional:
    /// without one, there's no button.
    #[serde(default)]
    pub powermenu: PowerMenu,

    /// The symbol of the bar's menu button, drawn left of the tags;
    /// bind `ClkMenuButton` to what it opens. Empty: no button.
    #[serde(default)]
    pub menubutton: String,

    /// The menu program, with its arguments, that the layout and power
    /// menus pipe their items into: dmenu-style, reading one item per
    /// line and printing the picked line. `-p <prompt>` and `-l <number
    /// of items>` are added for each menu.
    #[serde(default = "default_menu")]
    pub menu: Vec<String>,
    /// The command run when a named area of the status text
    /// (`^a<name>^`) is clicked, with the area's name and the mouse
    /// button (1-5) added: `flexiflow_bar --click volume 1`, by default.
    /// The click doesn't go on to the `ClkStatusText` bindings.
    #[serde(default = "default_statusclick")]
    pub statusclick: Vec<String>,

    /// The `hotkeys` popup's colors and groups - see `Hotkeys`.
    #[serde(default)]
    pub hotkeys: Hotkeys,
    /// More of the `hotkeys` popup's group order, after the ones
    /// `hotkeys.groups` lists; kept for older configs.
    #[serde(default)]
    pub hotkeys_groups: Vec<String>,
}

fn default_statusclick() -> Vec<String> {
    vec!["flexiflow_bar".to_string(), "--click".to_string()]
}

fn default_menu() -> Vec<String> {
    vec!["fleximenu".to_string()]
}

fn default_gaps() -> c_int {
    10
}

fn default_barpadding() -> c_int {
    2
}

unsafe impl Send for Config {}
unsafe impl Sync for Config {}

/// The baked-in default config, also used by `load_home` to seed
/// `~/.config/flexiflow/config.lua` on first run.
const DEFAULT_LUA: &str = include_str!("config.lua");

struct ConfigBuilder {
    lua: Lua,
    globals: Table,
}

impl ConfigBuilder {
    fn new() -> Self {
        let lua = Lua::new();
        let globals = lua.globals();

        // install handler functions for keybindings
        for (k, v) in HANDLERS {
            globals.set(k, v).unwrap();
        }

        // install key definitions
        for (k, v) in KEYS.iter().chain(XKEYS.iter()) {
            globals.set(*k, *v).unwrap();
        }

        // install click and button definitions
        for (k, v) in CLICKS {
            globals.set(k, v).unwrap();
        }

        for (k, v) in env::BUTTONS {
            globals.set(k, v).unwrap();
        }

        lua.load(DEFAULT_LUA).exec().unwrap();

        Self { lua, globals }
    }

    /// Load and eval `path` into the Lua interpreter in `self`.
    fn load(self, path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        self.lua.load(read_to_string(path)?).exec()?;
        Ok(self)
    }

    fn finish(self) -> Result<Config, Box<dyn Error>> {
        Ok(self.lua.from_value(self.globals.get("flexiflow")?)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        ConfigBuilder::new().finish().unwrap()
    }
}

impl Config {
    pub fn from_lua(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        ConfigBuilder::new().load(path)?.finish()
    }

    /// Attempt to load a config file on first usage from `$XDG_CONFIG_HOME`,
    /// then `$HOME`, before falling back to the default config.
    pub fn load_home() -> Self {
        let base = if let Ok(xdg_home) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_home)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config")
        } else {
            log::warn!("unable to determine config directory");
            return Config::default();
        };

        let config_path = base.join("flexiflow").join("config.lua");
        if !config_path.exists() {
            seed_default_config(&config_path, DEFAULT_LUA);
            return Config::default();
        }

        Config::from_lua(&config_path).unwrap_or_else(|e| {
            log::error!("failed to read config file {config_path:?}: {e:?}");
            Config::default()
        })
    }
}

/// Copy the baked-in default to `path` so a first-time user gets an
/// actual, editable file at the documented override location instead
/// of silently running on an in-memory default they'd have to dig up
/// from source to even see. Best-effort: a write failure (e.g. a
/// read-only home) just logs and falls back to the in-memory default,
/// same as before this existed.
fn seed_default_config(path: &Path, source: &str) {
    let Some(dir) = path.parent() else { return };
    match create_dir_all(dir).and_then(|()| write(path, source)) {
        Ok(()) => log::info!("wrote default config to {path:?}"),
        Err(e) => log::warn!("could not write default config to {path:?}: {e:?}"),
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::*;

    #[test]
    fn from_lua() {
        // an empty config file layered on the embedded default, so this
        // snapshots the default config as loaded from disk
        let dir = std::env::temp_dir()
            .join(format!("flexiflow-config-from-lua-{}", std::process::id()));
        let path = dir.join("config.lua");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "").unwrap();

        let got = Config::from_lua(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_debug_snapshot!(got)
    }

    fn colors(entries: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        entries
            .iter()
            .map(|(k, v)| {
                (k.to_string(), v.iter().map(|c| c.to_string()).collect())
            })
            .collect()
    }

    #[test]
    fn colormap_button_colors_fall_back_to_norm() {
        let norm: &[&str] = &["#111111", "#222222", "#333333"];
        let sel: &[&str] = &["#444444", "#555555", "#666666"];
        let power: &[&str] = &["#777777", "#888888", "#999999"];
        let cstrings = |c: &[&str]| -> Vec<CString> {
            c.iter().map(|c| CString::new(*c).unwrap()).collect()
        };

        let got = ColorMap::try_from(colors(&[
            ("norm", norm),
            ("sel", sel),
            ("power", power),
        ]))
        .unwrap();

        assert_eq!(got.0[Scheme::Layout as usize], cstrings(norm));
        assert_eq!(got.0[Scheme::Power as usize], cstrings(power));
        assert_eq!(got.0[Scheme::Title as usize], cstrings(sel));
    }

    #[test]
    fn colormap_rejects_a_button_without_three_colors() {
        let triple: &[&str] = &["#111111", "#222222", "#333333"];
        let got = ColorMap::try_from(colors(&[
            ("norm", triple),
            ("sel", triple),
            ("layout", &["#111111"]),
        ]));
        assert!(got.is_err());
    }

    #[test]
    fn seed_default_config_writes_the_embedded_default_verbatim() {
        let dir = std::env::temp_dir()
            .join(format!("flexiflow-config-test-{}", std::process::id()));
        let path = dir.join("config.lua");
        let _ = std::fs::remove_dir_all(&dir);

        seed_default_config(&path, DEFAULT_LUA);

        assert_eq!(read_to_string(&path).unwrap(), DEFAULT_LUA);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
