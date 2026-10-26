//! `restart`: re-exec ourselves in place, picking up a rebuilt binary or
//! an edited config. River keeps every window open while no window
//! manager is running, and gives the new instance the same windows with
//! the same `identifier`s, so each window's tags, output and floating
//! state are saved under it, along with each output's view, and read
//! back after the exec.
use std::{collections::HashMap, os::unix::process::CommandExt as _, path::PathBuf};

use crate::wm::{TagState, Wm};

/// Set in our environment across the exec, so autostart doesn't run twice.
pub const RESTARTED: &str = "FLEXIFLOW_RIVER_RESTARTED";

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SavedWindow {
    pub tags: u32,
    /// the wl_output global name
    pub output: u32,
    pub floating: bool,
    pub fullscreen: bool,
    pub cfact: f32,
    pub geom: (i32, i32, i32, i32),
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SavedOutput {
    pub tagset: [u32; 2],
    pub seltags: usize,
    pub pertag: Vec<TagState>,
    pub curtag: usize,
    pub prevtag: usize,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Saved {
    pub windows: HashMap<String, SavedWindow>,
    pub outputs: HashMap<u32, SavedOutput>,
    pub status: String,
    pub hidevacant: bool,
}

fn path() -> PathBuf {
    let mut p = flexiflow_river::ipc::socket_path();
    p.set_extension("restart.json");
    p
}

pub fn restart(wm: &mut Wm) {
    let mut saved = Saved {
        status: wm.status.clone(),
        hidevacant: wm.hidevacant,
        ..Default::default()
    };
    // in client list order, which the new instance keeps
    for w in &wm.clients {
        let win = &wm.windows[w];
        if win.identifier.is_empty() {
            continue;
        }
        let output = win.output.as_ref().and_then(|o| wm.output(o)).map_or(0, |o| o.global);
        saved.windows.insert(
            win.identifier.clone(),
            SavedWindow {
                tags: win.tags,
                output,
                floating: win.floating,
                fullscreen: win.fullscreen,
                cfact: win.cfact,
                geom: win.geom,
            },
        );
    }
    for (_, o) in &wm.outputs {
        saved.outputs.insert(
            o.global,
            SavedOutput {
                tagset: o.tagset,
                seltags: o.seltags,
                pertag: o.pertag.clone(),
                curtag: o.curtag,
                prevtag: o.prevtag,
            },
        );
    }
    if let Err(e) = std::fs::write(path(), serde_json::to_string(&saved).unwrap()) {
        log::warn!("restart: can't save state: {e}");
    }
    wm.restarting = true;
    wm.manager.stop();
}

/// Our own binary's path. Once `make install` or `cargo build` has
/// replaced it, Linux reports the old one as `<path> (deleted)`: the new
/// build is at `<path>`, which is what a restart is for.
fn own_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let s = exe.to_str()?;
    Some(s.strip_suffix(" (deleted)").map_or(exe.clone(), PathBuf::from))
}

/// Called once river has finished with us: become the new instance,
/// from our own path or else whichever `flexiflow-river` is on $PATH.
pub fn exec() -> ! {
    for exe in own_exe().into_iter().chain([PathBuf::from("flexiflow-river")]) {
        let err = std::process::Command::new(&exe)
            .args(std::env::args_os().skip(1))
            .env(RESTARTED, "1")
            .exec();
        log::error!("restart: can't exec {exe:?}: {err}");
    }
    std::process::exit(1);
}

/// What the previous instance saved, if this is a restart.
pub fn load() -> Option<Saved> {
    std::env::var_os(RESTARTED)?;
    let p = path();
    let s = std::fs::read_to_string(&p).ok()?;
    let _ = std::fs::remove_file(&p);
    serde_json::from_str(&s).ok()
}

/// Give a restored output back its view, once river says which
/// wl_output it is.
pub fn restore_outputs(wm: &mut Wm) {
    let Some(saved) = wm.restored.as_mut() else { return };
    let ntags = wm.config.tags.len();
    for (_, o) in &mut wm.outputs {
        if let Some(s) = saved.outputs.remove(&o.global)
            && s.pertag.len() == ntags + 1
        {
            o.tagset = s.tagset;
            o.seltags = s.seltags;
            o.pertag = s.pertag;
            o.curtag = s.curtag;
            o.prevtag = s.prevtag;
        }
    }
}
