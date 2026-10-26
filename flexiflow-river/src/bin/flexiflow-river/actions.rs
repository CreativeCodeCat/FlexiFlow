//! The key/button handlers config.lua binds, the same set and the same
//! behaviour as the X11 FlexiFlow's key_handlers.rs. They only run inside
//! a manage sequence (see `Wm::manage`), so each may change any state.
use std::{
    os::unix::process::CommandExt as _,
    process::{Command, Stdio},
};

use flexiflow_river::{
    config::{Arg, Clk},
    layouts::ARRANGE,
};

use crate::wm::{Action, Op, Oid, Wm};

pub fn run(wm: &mut Wm, action: Action) {
    let (func, arg, target) = match action {
        Action::Key(i) => {
            let Some(k) = wm.config.keys.get(i) else { return };
            (k.func.0.clone(), k.arg.clone(), None)
        }
        Action::Button(i) => {
            let Some(b) = wm.config.buttons.get(i).cloned() else { return };
            debug_assert!(b.clicks(Clk::ClientWin));
            // the window under the pointer, focused first like dwm
            let hovered = wm.seats.first().and_then(|(_, s)| s.hovered.clone());
            let Some(w) = hovered.filter(|w| wm.is_visible(w)) else { return };
            wm.focus(Some(w.clone()));
            (b.func.0, b.arg, Some(w))
        }
        Action::Run { output, func, arg } => {
            if let Some(global) = output
                && let Some((id, _)) = wm.outputs.iter().find(|(_, o)| o.global == global)
            {
                let id = id.clone();
                if wm.selmon.as_ref() != Some(&id) {
                    wm.selmon = Some(id.clone());
                    let top = wm.top_of_stack(&id);
                    wm.focus(top);
                }
            }
            (func.0, arg, None)
        }
    };
    let _ = target;
    dispatch(wm, &func, &arg);
}

fn dispatch(wm: &mut Wm, func: &str, arg: &Arg) {
    log::debug!("{func} {arg:?}");
    match func {
        "spawn" => {
            if let Arg::V(argv) = arg {
                spawn(argv);
            }
        }
        "view" => view(wm, arg.ui()),
        "toggleview" => toggleview(wm, arg.ui()),
        "tag" => tag(wm, arg.ui()),
        "tagview" => {
            tag(wm, arg.ui());
            view(wm, arg.ui());
        }
        "toggletag" => toggletag(wm, arg.ui()),
        "focusstack" => focusstack(wm, arg.i()),
        "pushstack" => pushstack(wm, arg.i()),
        "zoom" => zoom(wm),
        "killclient" => {
            if let Some(w) = wm.focused.clone() {
                wm.windows[&w].proxy.close();
            }
        }
        "togglefloating" => togglefloating(wm),
        "fullscreen" => {
            if let Some(w) = wm.focused.clone() {
                let win = wm.windows.get_mut(&w).unwrap();
                win.fullscreen = !win.fullscreen;
            }
        }
        "incnmaster" => {
            if let Some(o) = wm.sel_output_mut() {
                let t = o.tag_mut();
                t.nmaster = (t.nmaster + arg.i()).max(0);
            }
        }
        "setmfact" => setmfact(wm, arg.f()),
        "setcfact" => setcfact(wm, arg.f()),
        "setlayout" => setlayout(wm, arg),
        "cyclelayout" => cyclelayout(wm, arg.i()),
        "focusmon" => focusmon(wm, arg.i()),
        "tagmon" => tagmon(wm, arg.i()),
        "togglebar" => {
            if let Some(o) = wm.sel_output_mut() {
                let t = o.tag_mut();
                t.showbar = !t.showbar;
            }
        }
        "togglevacant" => wm.hidevacant = !wm.hidevacant,
        "togglescratch" => togglescratch(wm, arg.i() as usize),
        "movemouse" => pointer_op(wm, false),
        "resizemouse" => pointer_op(wm, true),
        "incrgaps" => gaps(wm, |g, d| *g = (g.0 + d, g.1 + d, g.2 + d, g.3 + d, g.4), arg.i()),
        "incrigaps" => gaps(wm, |g, d| *g = (g.0, g.1, g.2 + d, g.3 + d, g.4), arg.i()),
        "incrogaps" => gaps(wm, |g, d| *g = (g.0 + d, g.1 + d, g.2, g.3, g.4), arg.i()),
        "incrohgaps" => gaps(wm, |g, d| g.0 += d, arg.i()),
        "incrovgaps" => gaps(wm, |g, d| g.1 += d, arg.i()),
        "incrihgaps" => gaps(wm, |g, d| g.2 += d, arg.i()),
        "incrivgaps" => gaps(wm, |g, d| g.3 += d, arg.i()),
        "defaultgaps" => {
            let d = wm.config.gaps;
            gaps(wm, |g, d| *g = (d, d, d, d, g.4), d);
        }
        "togglegaps" => gaps(wm, |g, _| g.4 = !g.4, 0),
        "layoutmenu" => crate::menus::layoutmenu(wm),
        "powermenu" => crate::menus::powermenu(wm),
        "hotkeys" => spawn(&[sibling("flexiflow-river-hotkeys")]),
        "quit" => wm.manager.exit_session(),
        "restart" => crate::restart::restart(wm),
        other => log::warn!("{other}: not a handler"),
    }
    wm.bar_dirty = true;
}

/// Run `argv` in its own session, so it outlives us and doesn't share
/// our signals. The child is reaped by `reap`.
pub fn spawn(argv: &[String]) {
    let Some((cmd, args)) = argv.split_first() else { return };
    let mut c = Command::new(cmd);
    c.args(args).stdin(Stdio::null());
    unsafe {
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    match c.spawn() {
        Ok(child) => crate::CHILDREN.lock().unwrap().push(child),
        Err(e) => log::warn!("spawn {cmd:?}: {e}"),
    }
}

/// `name` from the directory this binary is in, if it's there (so a
/// build run from `target/` uses its own), else from $PATH.
fn sibling(name: &str) -> String {
    std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join(name)))
        .filter(|p| p.exists())
        .map_or_else(|| name.to_string(), |p| p.to_string_lossy().into_owned())
}

/// dwm's `view`, with the pertag patch's per-tag layout, mfact, nmaster,
/// gaps and bar: view `tags` on the focused output, or go back to the
/// previous view for 0.
fn view(wm: &mut Wm, tags: u32) {
    let Some(o) = wm.selmon.clone() else { return };
    view_on(wm, &o, tags);
}

pub fn view_on(wm: &mut Wm, o: &Oid, tags: u32) {
    let mask = wm.tagmask();
    let Some(out) = wm.output_mut(o) else { return };
    if tags & mask == out.tagset() {
        return;
    }
    out.seltags ^= 1;
    if tags & mask != 0 {
        out.tagset[out.seltags] = tags & mask;
        out.prevtag = out.curtag;
        out.curtag = if tags == !0 { 0 } else { tags.trailing_zeros() as usize + 1 };
    } else {
        std::mem::swap(&mut out.prevtag, &mut out.curtag);
    }
    wm.focused = None;
}

fn toggleview(wm: &mut Wm, tags: u32) {
    let mask = wm.tagmask();
    let Some(out) = wm.sel_output_mut() else { return };
    let new = out.tagset() ^ (tags & mask);
    if new == 0 {
        return;
    }
    out.tagset[out.seltags] = new;
    if new == mask {
        out.prevtag = out.curtag;
        out.curtag = 0;
    }
    // the current tag stays put while it's still in view
    if new & (1 << out.curtag.saturating_sub(1)) == 0 || out.curtag == 0 {
        out.prevtag = out.curtag;
        out.curtag = new.trailing_zeros() as usize + 1;
    }
    wm.focused = None;
}

fn tag(wm: &mut Wm, tags: u32) {
    let mask = wm.tagmask();
    if tags & mask == 0 {
        return;
    }
    if let Some(w) = wm.focused.clone() {
        wm.windows.get_mut(&w).unwrap().tags = tags & mask;
        wm.focused = None;
    }
}

fn toggletag(wm: &mut Wm, tags: u32) {
    let mask = wm.tagmask();
    if let Some(w) = wm.focused.clone() {
        let win = wm.windows.get_mut(&w).unwrap();
        let new = win.tags ^ (tags & mask);
        if new != 0 {
            win.tags = new;
            wm.focused = None;
        }
    }
}

/// The windows visible on the focused output, in client list order.
fn visible(wm: &Wm) -> Vec<crate::wm::Wid> {
    let sel = wm.selmon.clone();
    wm.clients
        .iter()
        .filter(|w| wm.is_visible(w) && wm.windows[*w].output == sel)
        .cloned()
        .collect()
}

fn focusstack(wm: &mut Wm, dir: i32) {
    let Some(sel) = wm.focused.clone() else { return };
    if wm.config.lock_fullscreen && wm.windows[&sel].fullscreen {
        return;
    }
    let vis = visible(wm);
    let Some(i) = vis.iter().position(|w| w == &sel) else { return };
    let n = vis.len() as i32;
    let next = vis[(i as i32 + dir).rem_euclid(n) as usize].clone();
    wm.focus(Some(next));
}

/// Move the focused window up or down the stack, wrapping around.
fn pushstack(wm: &mut Wm, dir: i32) {
    let Some(sel) = wm.focused.clone() else { return };
    let vis = visible(wm);
    let Some(i) = vis.iter().position(|w| w == &sel) else { return };
    let n = vis.len() as i32;
    let other = vis[(i as i32 + dir).rem_euclid(n) as usize].clone();
    let a = wm.clients.iter().position(|w| w == &sel).unwrap();
    let b = wm.clients.iter().position(|w| w == &other).unwrap();
    wm.clients.swap(a, b);
}

/// Move the focused window to the master area, or the next one there if
/// it's already master.
fn zoom(wm: &mut Wm) {
    let Some(sel) = wm.focused.clone() else { return };
    if !wm.tiled(&sel) {
        return;
    }
    let tiled: Vec<_> = visible(wm).into_iter().filter(|w| wm.tiled(w)).collect();
    let target = if tiled.first() == Some(&sel) {
        match tiled.get(1) {
            Some(w) => w.clone(),
            None => return,
        }
    } else {
        sel
    };
    wm.clients.retain(|w| w != &target);
    wm.clients.insert(0, target.clone());
    wm.focus(Some(target));
}

fn togglefloating(wm: &mut Wm) {
    let Some(sel) = wm.focused.clone() else { return };
    let win = wm.windows.get_mut(&sel).unwrap();
    if win.fullscreen {
        return;
    }
    win.floating = !win.floating || win.fixed();
    if win.floating {
        // float where it was tiled
        if let Some(t) = win.target {
            win.geom = t;
            win.center = false;
        }
    }
}

fn setmfact(wm: &mut Wm, f: f32) {
    let Some(o) = wm.sel_output_mut() else { return };
    let layout = o.tag().layout;
    if wm.config.layouts[layout].arrange.is_none() {
        return;
    }
    let o = wm.sel_output_mut().unwrap();
    let t = o.tag_mut();
    let f = if f < 1.0 { f + t.mfact } else { f - 1.0 };
    if (0.05..=0.95).contains(&f) {
        t.mfact = f;
    }
}

fn setcfact(wm: &mut Wm, d: f32) {
    let Some(sel) = wm.focused.clone() else { return };
    let win = wm.windows.get_mut(&sel).unwrap();
    if d == 0.0 {
        win.cfact = 1.0;
    } else if win.cfact + d > 0.25 {
        win.cfact += d;
    }
}

/// dwm's `setlayout` with pertag's two remembered layouts: a layout
/// index selects it, and none (or the one already selected) swaps back
/// to the previous one.
fn setlayout(wm: &mut Wm, arg: &Arg) {
    let n = wm.config.layouts.len();
    let Some(o) = wm.sel_output_mut() else { return };
    let t = o.tag_mut();
    match arg {
        Arg::L(Some(i)) if *i < n && *i != t.layout => {
            t.prevlayout = t.layout;
            t.layout = *i;
        }
        _ => std::mem::swap(&mut t.layout, &mut t.prevlayout),
    }
}

fn cyclelayout(wm: &mut Wm, dir: i32) {
    let n = wm.config.layouts.len() as i32;
    let Some(o) = wm.sel_output() else { return };
    let next = (o.tag().layout as i32 + dir.signum()).rem_euclid(n) as usize;
    setlayout(wm, &Arg::L(Some(next)));
}

fn focusmon(wm: &mut Wm, dir: i32) {
    let n = wm.outputs.len() as i32;
    if n < 2 {
        return;
    }
    let Some(i) = wm.outputs.iter().position(|(o, _)| Some(o) == wm.selmon.as_ref()) else {
        return;
    };
    let next = wm.outputs[(i as i32 + dir).rem_euclid(n) as usize].0.clone();
    wm.selmon = Some(next.clone());
    let top = wm.top_of_stack(&next);
    wm.focused = None;
    wm.focus(top);
}

/// Send the focused window to the next output, on the tags in view there.
fn tagmon(wm: &mut Wm, dir: i32) {
    let n = wm.outputs.len() as i32;
    let Some(sel) = wm.focused.clone() else { return };
    if n < 2 {
        return;
    }
    let Some(i) = wm.outputs.iter().position(|(o, _)| Some(o) == wm.selmon.as_ref()) else {
        return;
    };
    let (next, out) = &wm.outputs[(i as i32 + dir).rem_euclid(n) as usize];
    let (next, tags, area) = (next.clone(), out.tagset(), out.area());
    let win = wm.windows.get_mut(&sel).unwrap();
    win.output = Some(next);
    win.tags = tags;
    if win.floating {
        win.geom.0 = area.0 + (area.2 - win.geom.2) / 2;
        win.geom.1 = area.1 + (area.3 - win.geom.3) / 2;
    }
    wm.focused = None;
}

/// The scratchpad patch: show or hide scratchpad slot `i`'s window by
/// toggling its reserved tag in view, starting its command if there's
/// no such window yet.
fn togglescratch(wm: &mut Wm, i: usize) {
    let Some(spec) = wm.config.scratchpads.get(i) else {
        log::warn!("togglescratch: no scratchpad configured at index {i}");
        return;
    };
    let bit = wm.scratchtag(i);
    let found = wm.clients.iter().find(|w| wm.windows[*w].scratch == Some(i)).cloned();
    let Some(w) = found else {
        let cmd = spec.cmd.clone();
        wm.scratch_pending.push(i);
        spawn(&cmd);
        return;
    };
    let Some(sel) = wm.selmon.clone() else { return };
    // follow the focused output, like dwm's sloppy scratchpads
    let win = wm.windows.get_mut(&w).unwrap();
    if win.output.as_ref() != Some(&sel) {
        win.output = Some(sel.clone());
        win.center = true;
        let o = wm.output_mut(&sel).unwrap();
        o.tagset[o.seltags] |= bit;
    } else {
        let o = wm.output_mut(&sel).unwrap();
        let new = o.tagset() ^ bit;
        if new != 0 {
            o.tagset[o.seltags] = new;
        }
    }
    if wm.is_visible(&w) {
        wm.focus(Some(w));
    } else {
        wm.focused = None;
    }
}

/// `movemouse`/`resizemouse`: float the focused window and move or
/// resize it with the pointer until the buttons are released.
fn pointer_op(wm: &mut Wm, resize: bool) {
    let Some(w) = wm.focused.clone() else { return };
    let Some((_, seat)) = wm.seats.first_mut() else { return };
    if seat.op.is_some() {
        return;
    }
    let win = wm.windows.get_mut(&w).unwrap();
    if win.fullscreen {
        return;
    }
    if !win.floating {
        win.floating = true;
        if let Some(t) = win.target {
            win.geom = t;
        }
    }
    if win.geom.2 == 0 {
        win.geom.2 = win.dims.0;
        win.geom.3 = win.dims.1;
    }
    win.center = false;
    if resize {
        win.proxy.inform_resize_start();
    }
    seat.op = Some(Op { win: w, resize, start: win.geom, delta: (0, 0) });
    seat.op_released = false;
    seat.proxy.op_start_pointer();
}

type GapTuple = (i32, i32, i32, i32, bool);

fn gaps(wm: &mut Wm, f: impl FnOnce(&mut GapTuple, i32), d: i32) {
    let Some(o) = wm.sel_output_mut() else { return };
    let g = &mut o.tag_mut().gaps;
    f(g, d);
    g.0 = g.0.max(0);
    g.1 = g.1.max(0);
    g.2 = g.2.max(0);
    g.3 = g.3.max(0);
}

/// The layout menu's description for `arrange`.
pub fn layout_desc(arrange: Option<&str>) -> &'static str {
    arrange
        .and_then(|a| ARRANGE.iter().find(|(n, _)| *n == a))
        .map_or("NULL Layout", |(_, d)| d)
}
