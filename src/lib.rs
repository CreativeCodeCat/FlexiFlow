//! FlexiFlow: a tiling window manager based on dwm. See `src/config.lua`
//! for keys/rules/autostart.
#![allow(clippy::missing_safety_doc, clippy::not_unsafe_ptr_arg_deref)]

use std::{
    ffi::{c_int, c_uint},
    fmt::Debug,
};

use config::key::FUNC_MAP;
use enums::Clk;
use layouts::{
    bstack, bstack_horiz, centered_floating_master, centered_master, deck,
    dwindle, gapless_grid, grid, horiz_grid, monocle, nrow_grid, spiral,
    tile,
};
use x11::xft::XftColor;

pub mod config;
pub mod drw;
pub mod enums;
pub mod events;
pub mod handlers;
pub mod key_handlers;
pub mod layouts;
pub mod util;
pub mod x;
pub mod xembed;

pub use core::*;
mod core;

pub use state::*;
mod state;

/// most applications want to start this way
pub const NORMAL_STATE: usize = 1;
/// application wants to start as an icon
pub const ICONIC_STATE: usize = 3;

// from Xutil.h
/// for windows that are not mapped
pub const WITHDRAWN_STATE: usize = 0;

pub type Window = u64;
pub type Clr = XftColor;

#[repr(C)]
#[derive(Clone, Debug, serde::Deserialize)]
pub enum Arg {
    I(c_int),
    Ui(c_uint),
    F(f32),
    /// Argument for execvp in spawn
    V(Vec<String>),
    /// CONFIG.layouts index for setlayout
    L(Option<usize>),
}

macro_rules! arg_getters {
    ($($field:ident => $fn:ident => $ty:ty$(,)*)*) => {
        $(pub fn $fn(&self) -> $ty {
            if let Self::$field(x) = self {
                return x.clone();
            }
            panic!("{self:?}");
        })*
    }
}

impl Arg {
    arg_getters! {
        I => i => c_int,
        Ui => ui => c_uint,
        F => f => f32,
        V => v => Vec<String>,
        L => l => Option<usize>,
    }
}

#[derive(Clone, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct ButtonFn(pub Option<fn(&mut State, *const Arg)>);

impl TryFrom<String> for ButtonFn {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(Some(
            FUNC_MAP
                .get(value.as_str())
                .cloned()
                .ok_or_else(|| format!("no key `{value}`"))?,
        )))
    }
}

impl Debug for ButtonFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ButtonFn")
            .field(&self.0.map(|_| "[func]"))
            .finish()
    }
}

#[repr(C)]
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Button {
    pub click: c_uint,
    pub mask: c_uint,
    pub button: c_uint,
    pub func: ButtonFn,
    #[serde(default = "default_button_arg")]
    pub arg: Arg,
}

/// Hack to get around `{L = nil}` equating to an empty table in Lua. If the
/// table's empty, treat it as the only optional Arg variant
fn default_button_arg() -> Arg {
    Arg::L(None)
}

impl Button {
    pub const fn new(
        click: Clk,
        mask: c_uint,
        button: c_uint,
        func: fn(&mut State, *const Arg),
        arg: Arg,
    ) -> Self {
        Self {
            click: click as c_uint,
            mask,
            button,
            func: ButtonFn(Some(func)),
            arg,
        }
    }
}

pub struct Cursor {
    pub cursor: x11::xlib::Cursor,
}

#[repr(C)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Rule {
    pub class: String,
    pub instance: String,
    pub title: String,
    pub tags: c_uint,
    pub isfloating: bool,
    pub isterminal: bool,
    pub noswallow: bool,
    pub monitor: c_int,
    /// Switch to the matched client's tag on manage, instead of only
    /// assigning it.
    #[serde(default)]
    pub focus: bool,
}

/// One scratchpad slot (flexiflow wants three: terminal, btop,
/// pavucontrol). `class` is matched against a client's `WM_CLASS` -
/// unlike a `--title` flag, not every scratchpad program lets you set
/// its title (pavucontrol doesn't), but `WM_CLASS` is reliable for all
/// of them. `cmd` is spawned when `togglescratch` finds no client
/// tagged with this slot yet.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ScratchSpec {
    pub class: String,
    pub cmd: Vec<String>,
}

pub struct Systray {
    pub win: Window,
    pub icons: *mut Client,
}

/// An `arrange` function, plus the description `layoutmenu` shows next
/// to its layout's symbol (the old scripts/layoutmenu's wording).
#[derive(Clone)]
pub struct LayoutFn(pub fn(&mut State, *mut Monitor), pub &'static str);

/// By hand rather than `#[serde(try_from = "String")]`: the derive
/// treats the `&'static str` field as borrowed from the input and would
/// demand `'de: 'static` of the whole config.
impl<'de> serde::Deserialize<'de> for LayoutFn {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.try_into().map_err(serde::de::Error::custom)
    }
}

impl TryFrom<String> for LayoutFn {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "tile" => Ok(Self(tile, "Tiled Layout")),
            "monocle" => Ok(Self(monocle, "Monocle Layout")),
            "bstack" => Ok(Self(bstack, "Stack Layout")),
            "bstack_horiz" => Ok(Self(bstack_horiz, "Stack Horz Layout")),
            "centered_master" => {
                Ok(Self(centered_master, "Centered Master Layout"))
            }
            "centered_floating_master" => Ok(Self(
                centered_floating_master,
                "Centered Floating Layout",
            )),
            "deck" => Ok(Self(deck, "Deck Layout")),
            "spiral" => Ok(Self(spiral, "Spiral Layout")),
            "dwindle" => Ok(Self(dwindle, "Dwindle Layout")),
            "gapless_grid" => Ok(Self(gapless_grid, "Gapless Grid Layout")),
            "grid" => Ok(Self(grid, "Grid Layout")),
            "horiz_grid" => Ok(Self(horiz_grid, "Grid Horz Layout")),
            "nrow_grid" => Ok(Self(nrow_grid, "Narrow Grid Layout")),
            s => Err(format!("unknown layout `{s}`")),
        }
    }
}

impl Debug for LayoutFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LayoutFn").field(&"[func]").finish()
    }
}

#[repr(C)]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Layout {
    pub symbol: String,
    #[serde(default)]
    pub arrange: Option<LayoutFn>,
}

/// Per-tag/per-monitor gap configuration, matching flexiflow's vanitygaps
/// behavior.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaps {
    /// Outer horizontal gap (top/bottom screen edge)
    pub oh: c_int,
    /// Outer vertical gap (left/right screen edge)
    pub ov: c_int,
    /// Inner horizontal gap (between stacked clients)
    pub ih: c_int,
    /// Inner vertical gap (between master/stack columns)
    pub iv: c_int,
    pub enabled: bool,
    /// Collapse outer gaps to 0 when there's a single client
    pub smartgaps: bool,
}

impl Gaps {
    /// Effective gaps for `n` clients.
    pub fn effective(&self, n: usize) -> (c_int, c_int, c_int, c_int) {
        let outer_enabled = !(self.smartgaps && n == 1) && self.enabled;
        let inner_enabled = self.enabled;
        (
            self.oh * outer_enabled as c_int,
            self.ov * outer_enabled as c_int,
            self.ih * inner_enabled as c_int,
            self.iv * inner_enabled as c_int,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Pertag {
    /// Current tag
    pub curtag: usize,
    /// Previous tag
    pub prevtag: usize,
    /// Number of windows in master area
    pub nmasters: Vec<c_int>,
    /// Proportion of monitor for master area
    pub mfacts: Vec<f32>,
    /// Selected layouts
    pub sellts: Vec<usize>,
    /// Matrix of tag and layout indices
    pub ltidxs: Vec<[*const Layout; 2]>,
    /// Whether to display the bar
    pub showbars: Vec<bool>,
    /// Gap configuration
    pub gaps: Vec<Gaps>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Monitor {
    pub ltsymbol: String,
    pub mfact: f32,
    pub nmaster: c_int,
    pub num: c_int,
    pub by: c_int,
    pub mx: c_int,
    pub my: c_int,
    pub mw: c_int,
    pub mh: c_int,
    pub wx: c_int,
    pub wy: c_int,
    pub ww: c_int,
    pub wh: c_int,
    pub seltags: usize,
    pub sellt: usize,
    pub tagset: [c_uint; 2usize],
    pub showbar: bool,
    pub topbar: bool,
    pub clients: *mut Client,
    pub sel: *mut Client,
    pub stack: *mut Client,
    pub next: *mut Monitor,
    pub barwin: Window,
    pub lt: [*const Layout; 2usize],
    pub pertag: Pertag,
    pub gaps: Gaps,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Client {
    pub name: String,
    pub mina: f32,
    pub maxa: f32,
    pub x: c_int,
    pub y: c_int,
    pub w: c_int,
    pub h: c_int,
    pub oldx: c_int,
    pub oldy: c_int,
    pub oldw: c_int,
    pub oldh: c_int,
    pub basew: c_int,
    pub baseh: c_int,
    pub incw: c_int,
    pub inch: c_int,
    pub maxw: c_int,
    pub maxh: c_int,
    pub minw: c_int,
    pub minh: c_int,
    pub hintsvalid: bool,
    pub bw: c_int,
    pub oldbw: c_int,
    /// Proportion of the master/stack column this client takes up
    /// relative to its siblings there. Follows the *client*, not the
    /// tag — unlike `Gaps`.
    pub cfact: f32,
    pub tags: c_uint,
    pub isfixed: bool,
    pub isfloating: bool,
    pub isurgent: bool,
    pub neverfocus: bool,
    pub oldstate: bool,
    pub isfullscreen: bool,
    pub isterminal: bool,
    pub noswallow: bool,
    pub pid: libc::pid_t,
    pub next: *mut Client,
    pub snext: *mut Client,
    pub swallowing: *mut Client,
    pub mon: *mut Monitor,
    pub win: Window,
}

pub struct Cursors {
    pub normal: Cursor,
    pub resize: Cursor,
    pub move_: Cursor,
}
