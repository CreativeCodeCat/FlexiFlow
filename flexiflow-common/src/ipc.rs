//! The window manager's Unix socket, one per Wayland display. Every
//! message is one line of text:
//!
//! - `status <text>` sets the status text (`flexiflow-river -b`)
//! - `action <output> <func> <arg json>` runs a key handler on `output`
//!   (a wl_output global name, or `-` for the focused output), the way a
//!   keybinding would; `<arg json>` is an [`Arg`] such as `{"I":1}`
//! - `subscribe` keeps the connection open and sends a [`BarState`] as
//!   one JSON line now and after every change, which is how the bar
//!   learns what to draw
//! - `pointer` answers `<x> <y>`: where the pointer is, relative to the
//!   focused output's area left free by the bar, for menus that open at
//!   the pointer (`fleximenu -M`)
//! - `focused` answers `<identifier> <x>,<y> <w>x<h>`: the focused
//!   window's ext-foreign-toplevel identifier (what `grim -T` takes) and
//!   where it is (what `grim -g` takes), or an empty line for none
use std::{
    io::{BufRead as _, BufReader, Write as _},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use crate::config::{Arg, Func};

pub fn socket_path() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    let display =
        std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
    PathBuf::from(dir).join(format!("flexiflow-river.{display}.sock"))
}

/// What the bar draws on one output.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OutputState {
    /// the wl_output's global name, matching it with the bar's outputs
    pub global: u32,
    /// whether this is the focused output
    pub selected: bool,
    pub showbar: bool,
    /// tags in view
    pub tags: u32,
    /// tags with windows on them
    pub occupied: u32,
    /// tags with a window that wants attention
    pub urgent: u32,
    /// the focused window's tags
    pub seltags: u32,
    pub ltsymbol: String,
    /// the focused window's title, if any
    pub title: Option<String>,
    pub floating: bool,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BarState {
    pub outputs: Vec<OutputState>,
    pub status: String,
    /// leave out the tags with no windows that aren't in view
    pub hidevacant: bool,
}

/// One parsed request line.
#[derive(Debug, PartialEq)]
pub enum Request {
    Status(String),
    Action { output: Option<u32>, func: Func, arg: Arg },
    Subscribe,
    Pointer,
    Focused,
}

impl Request {
    pub fn parse(line: &str) -> Result<Self, String> {
        let line = line.trim_end_matches(['\n', '\r']);
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        match cmd {
            "status" => Ok(Request::Status(rest.to_string())),
            "subscribe" => Ok(Request::Subscribe),
            "pointer" => Ok(Request::Pointer),
            "focused" => Ok(Request::Focused),
            "action" => {
                let mut parts = rest.splitn(3, ' ');
                let output = parts.next().ok_or("missing output")?;
                let output = match output {
                    "-" => None,
                    o => Some(o.parse().map_err(|_| format!("bad output {o:?}"))?),
                };
                let func = parts.next().ok_or("missing func")?.to_string();
                let func = Func::try_from(func)?;
                let arg = match parts.next() {
                    None | Some("") => Arg::default(),
                    Some(json) => {
                        serde_json::from_str(json).map_err(|e| e.to_string())?
                    }
                };
                Ok(Request::Action { output, func, arg })
            }
            _ => Err(format!("unknown request {cmd:?}")),
        }
    }

    pub fn to_line(&self) -> String {
        match self {
            Request::Status(text) => {
                format!("status {}\n", text.replace('\n', " "))
            }
            Request::Subscribe => "subscribe\n".to_string(),
            Request::Pointer => "pointer\n".to_string(),
            Request::Focused => "focused\n".to_string(),
            Request::Action { output, func, arg } => format!(
                "action {} {} {}\n",
                output.map_or("-".to_string(), |o| o.to_string()),
                func.0,
                serde_json::to_string(arg).unwrap()
            ),
        }
    }
}

/// Send one request to the running window manager.
pub fn send(req: &Request) -> std::io::Result<()> {
    let mut s = UnixStream::connect(socket_path())?;
    s.write_all(req.to_line().as_bytes())
}

/// Send one request and return the line it's answered with.
pub fn ask(req: &Request) -> std::io::Result<String> {
    let mut s = UnixStream::connect(socket_path())?;
    s.write_all(req.to_line().as_bytes())?;
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line)?;
    Ok(line.trim_end_matches('\n').to_string())
}

/// Subscribe to the window manager's state, calling `f` with each
/// update until the connection ends.
pub fn subscribe(mut f: impl FnMut(BarState)) -> std::io::Result<()> {
    let mut s = UnixStream::connect(socket_path())?;
    s.write_all(Request::Subscribe.to_line().as_bytes())?;
    for line in BufReader::new(s).lines() {
        match serde_json::from_str(&line?) {
            Ok(state) => f(state),
            Err(e) => log::warn!("bad state from flexiflow-river: {e}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_round_trip() {
        for req in [
            Request::Status("^c#ffffff^hi".into()),
            Request::Subscribe,
            Request::Pointer,
            Request::Focused,
            Request::Action {
                output: Some(42),
                func: Func("view".into()),
                arg: Arg::Ui(4),
            },
            Request::Action {
                output: None,
                func: Func("spawn".into()),
                arg: Arg::V(vec!["foot".into(), "-e".into(), "a b".into()]),
            },
            Request::Action {
                output: None,
                func: Func("layoutmenu".into()),
                arg: Arg::L(None),
            },
        ] {
            assert_eq!(Request::parse(&req.to_line()).unwrap(), req);
        }
    }

    #[test]
    fn unknown_handlers_are_refused() {
        assert!(Request::parse("action - nope {}").is_err());
    }
}
