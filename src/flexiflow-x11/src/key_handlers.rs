use std::cmp::max;
use std::ffi::{CStr, CString, c_char, c_int, c_uint};
use std::process::Command;
use std::ptr::null_mut;

use x11::xlib::{
    self, ButtonRelease, CWY, ConfigureRequest, ControlMask, CopyFromParent,
    CurrentTime, DestroyAll, EnterWindowMask, Expose, ExposureMask, False,
    GrabModeAsync, GrabSuccess, KeySym, MapRequest, Mod1Mask, Mod4Mask,
    MotionNotify, NoEventMask, ShiftMask, SubstructureRedirectMask, True,
    XCheckMaskEvent, XConfigureWindow, XEvent, XGrabPointer, XGrabServer,
    XKeysymToString, XKillClient, XMaskEvent, XSetCloseDownMode,
    XSetErrorHandler, XSync, XUngrabPointer, XUngrabServer, XWarpPointer,
    XWindowChanges,
};

use crate::config::key::Key;
use crate::core::{
    HANDLER, MOUSEMASK, XNONE, arrange, attach, attachstack, detach,
    detachstack, drawbar, drawbars, focus, getrootptr, height, is_visible,
    nexttiled, pop, recttomon, resize, resizebarwin, restack, sendevent,
    setfullscreen, textw, unfocus, updatebarpos, width, xerror, xerrordummy,
};
use crate::drw;
use x11::keysym::{XK_BackSpace, XK_Escape, XK_KP_Enter, XK_Return};
use crate::enums::{Col, Scheme, WM};
use crate::util::{restart_exe, spawn_child};
use crate::{Arg, Client, Clr, Gaps, Layout, Monitor, PowerMenuItem};
use crate::{State, cfor};

/// Shows or hides the bar's tags that have no windows and aren't in view.
pub(crate) fn togglevacant(state: &mut State, _arg: *const Arg) {
    state.hidevacant = !state.hidevacant;
    drawbars(state);
}

pub(crate) fn togglebar(state: &mut State, _arg: *const Arg) {
    unsafe {
        let monitor = &mut *state.selmon;
        monitor.pertag.showbars[monitor.pertag.curtag] = !monitor.showbar;
        monitor.showbar = monitor.pertag.showbars[monitor.pertag.curtag];
        updatebarpos(state, state.selmon);
        resizebarwin(state, state.selmon);
        if state.config.showsystray {
            let mut wc = XWindowChanges {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                border_width: 0,
                sibling: 0,
                stack_mode: 0,
            };
            if !(monitor).showbar {
                wc.y = -state.bh;
            } else if (monitor).showbar {
                wc.y = 0;
                if !(monitor).topbar {
                    wc.y = (monitor).mh - state.bh;
                }
            }
            XConfigureWindow(
                state.dpy,
                state.systray().win,
                CWY as u32,
                &mut wc,
            );
        }
        arrange(state, state.selmon);
    }
}

pub(crate) fn focusstack(state: &mut State, arg: *const Arg) {
    unsafe {
        let mut c: *mut Client = null_mut();
        let mut i: *mut Client;

        if (*state.selmon).sel.is_null()
            || ((*(*state.selmon).sel).isfullscreen
                && state.config.lock_fullscreen)
        {
            return;
        }
        if (*arg).i() > 0 {
            cfor!((c = (*(*state.selmon).sel).next; !c.is_null() && !is_visible(c); c = (*c).next) {});
            if c.is_null() {
                cfor!((c = (*state.selmon).clients; !c.is_null() && !is_visible(c); c = (*c).next) {});
            }
        } else {
            cfor!((i = (*state.selmon).clients; i != (*state.selmon).sel; i = (*i).next) {
                if is_visible(i) {
                    c = i;
                }
            });
            if c.is_null() {
                cfor!((; !i.is_null(); i = (*i).next) {
                    if is_visible(i) {
                        c = i;
                    }
                });
            }
        }
        if !c.is_null() {
            focus(state, c);
            restack(state, state.selmon);
        }
    }
}

/// Increase the number of windows in the master area.
pub(crate) fn incnmaster(state: &mut State, arg: *const Arg) {
    unsafe {
        let monitor = &mut *state.selmon;
        monitor.pertag.nmasters[monitor.pertag.curtag] =
            std::cmp::max(monitor.nmaster + (*arg).i(), 0);
        monitor.nmaster = monitor.pertag.nmasters[monitor.pertag.curtag];
        arrange(state, state.selmon);
    }
}

/// Set the fraction of the screen occupied by the master window. An `arg.f`
/// greater than 1.0 sets the fraction absolutely, while fractional values add
/// to the current value. Total values are restricted to the range [0.05, 0.95]
/// to leave at least 5% of the screen for other windows.
pub(crate) fn setmfact(state: &mut State, arg: *const Arg) {
    unsafe {
        if arg.is_null()
            || (*(*state.selmon).lt[(*state.selmon).sellt])
                .arrange
                .is_none()
        {
            return;
        }
        let f = if (*arg).f() < 1.0 {
            (*arg).f() + (*state.selmon).mfact
        } else {
            (*arg).f() - 1.0
        };
        if !(0.05..=0.95).contains(&f) {
            return;
        }
        let monitor = &mut *state.selmon;
        monitor.pertag.mfacts[monitor.pertag.curtag] = f;
        monitor.mfact = monitor.pertag.mfacts[monitor.pertag.curtag];
        arrange(state, state.selmon);
    }
}

/// Apply `f` to the selected monitor's current-tag gaps, syncing the
/// pertag-remembered copy the same way `incnmaster`/`setmfact` do, then
/// re-arrange. Shared by the `incr*gaps` family below.
fn adjust_gaps(state: &mut State, f: impl FnOnce(&mut Gaps)) {
    unsafe {
        let monitor = &mut *state.selmon;
        let g = &mut monitor.pertag.gaps[monitor.pertag.curtag];
        f(g);
        g.oh = g.oh.max(0);
        g.ov = g.ov.max(0);
        g.ih = g.ih.max(0);
        g.iv = g.iv.max(0);
        monitor.gaps = *g;
        arrange(state, state.selmon);
    }
}

/// Adjust all four gaps (outer + inner, horizontal + vertical) by `arg.i`.
pub(crate) fn incrgaps(state: &mut State, arg: *const Arg) {
    let d = unsafe { (*arg).i() };
    adjust_gaps(state, |g| {
        g.oh += d;
        g.ov += d;
        g.ih += d;
        g.iv += d;
    });
}

/// Adjust both inner gaps (between stacked clients) by `arg.i`.
pub(crate) fn incrigaps(state: &mut State, arg: *const Arg) {
    let d = unsafe { (*arg).i() };
    adjust_gaps(state, |g| {
        g.ih += d;
        g.iv += d;
    });
}

/// Adjust both outer gaps (screen edges) by `arg.i`.
pub(crate) fn incrogaps(state: &mut State, arg: *const Arg) {
    let d = unsafe { (*arg).i() };
    adjust_gaps(state, |g| {
        g.oh += d;
        g.ov += d;
    });
}

/// Adjust the outer horizontal gap (top/bottom edge) by `arg.i`.
pub(crate) fn incrohgaps(state: &mut State, arg: *const Arg) {
    let d = unsafe { (*arg).i() };
    adjust_gaps(state, |g| g.oh += d);
}

/// Adjust the outer vertical gap (left/right edge) by `arg.i`.
pub(crate) fn incrovgaps(state: &mut State, arg: *const Arg) {
    let d = unsafe { (*arg).i() };
    adjust_gaps(state, |g| g.ov += d);
}

/// Adjust the inner horizontal gap (between stacked clients) by `arg.i`.
pub(crate) fn incrihgaps(state: &mut State, arg: *const Arg) {
    let d = unsafe { (*arg).i() };
    adjust_gaps(state, |g| g.ih += d);
}

/// Adjust the inner vertical gap (between master/stack columns) by `arg.i`.
pub(crate) fn incrivgaps(state: &mut State, arg: *const Arg) {
    let d = unsafe { (*arg).i() };
    adjust_gaps(state, |g| g.iv += d);
}

/// Reset all four gaps on the current tag back to `state.config.gaps`.
pub(crate) fn defaultgaps(state: &mut State, _arg: *const Arg) {
    let d = state.config.gaps;
    adjust_gaps(state, |g| {
        g.oh = d;
        g.ov = d;
        g.ih = d;
        g.iv = d;
    });
}

/// Toggle gaps on/off for the current tag, keeping the configured pixel
/// values (`enabled = false` just makes `Gaps::effective` return zeroes).
pub(crate) fn togglegaps(state: &mut State, _arg: *const Arg) {
    adjust_gaps(state, |g| g.enabled = !g.enabled);
}

/// Adjust the focused client's `cfact` (its share of the master/stack
/// column relative to its siblings there) by `arg.f`. `arg.f == 0.0` resets
/// it back to the default of `1.0`; values are floored just above `0.25`
/// so a client never collapses to nothing.
pub(crate) fn setcfact(state: &mut State, arg: *const Arg) {
    unsafe {
        if arg.is_null() {
            return;
        }
        let sel = (*state.selmon).sel;
        if sel.is_null()
            || (*(*state.selmon).lt[(*state.selmon).sellt])
                .arrange
                .is_none()
        {
            return;
        }
        let d = (*arg).f();
        if d == 0.0 {
            (*sel).cfact = 1.0;
        } else {
            let next = (*sel).cfact + d;
            if next <= 0.25 {
                return;
            }
            (*sel).cfact = next;
        }
        arrange(state, state.selmon);
    }
}

/// Move the selected window to the master area. The current master is pushed to
/// the top of the stack.
pub(crate) fn zoom(state: &mut State, _arg: *const Arg) {
    unsafe {
        let mut c = (*state.selmon).sel;
        if (*(*state.selmon).lt[(*state.selmon).sellt])
            .arrange
            .is_none()
            || c.is_null()
            || (*c).isfloating
        {
            return;
        }
        if c == nexttiled((*state.selmon).clients) {
            c = nexttiled((*c).next);
            if c.is_null() {
                return;
            }
        }
        pop(state, c);
    }
}

/// View the tag identified by `arg.ui`.
pub(crate) fn view(state: &mut State, arg: *const Arg) {
    log::trace!("view");
    unsafe {
        if (*arg).ui() & state.tagmask()
            == (*state.selmon).tagset[(*state.selmon).seltags]
        {
            return;
        }
        (*state.selmon).seltags ^= 1; // toggle sel tagset

        // Safety: we were gonna dereference it anyway
        let pertag = &mut (*state.selmon).pertag;
        if ((*arg).ui() & state.tagmask()) != 0 {
            (*state.selmon).tagset[(*state.selmon).seltags] =
                (*arg).ui() & state.tagmask();
            pertag.prevtag = pertag.curtag;

            if (*arg).ui() == !0 {
                pertag.curtag = 0;
            } else {
                let mut i;
                cfor!((i = 0; ((*arg).ui() & (1 << i)) == 0; i += 1) {});
                pertag.curtag = i + 1;
            }
        } else {
            std::mem::swap(&mut pertag.prevtag, &mut pertag.curtag);
        }

        (*state.selmon).nmaster = pertag.nmasters[pertag.curtag];
        (*state.selmon).mfact = pertag.mfacts[pertag.curtag];
        (*state.selmon).sellt = pertag.sellts[pertag.curtag];
        (*state.selmon).lt[(*state.selmon).sellt] =
            pertag.ltidxs[pertag.curtag][(*state.selmon).sellt];
        (*state.selmon).lt[(*state.selmon).sellt ^ 1] =
            pertag.ltidxs[pertag.curtag][(*state.selmon).sellt ^ 1];

        if (*state.selmon).showbar != pertag.showbars[pertag.curtag] {
            togglebar(state, null_mut());
        }

        focus(state, null_mut());
        arrange(state, state.selmon);
    }
}

pub(crate) fn killclient(state: &mut State, _arg: *const Arg) {
    unsafe {
        if (*state.selmon).sel.is_null() {
            return;
        }

        if !sendevent(
            state,
            (*(*state.selmon).sel).win,
            state.wmatom[WM::Delete as usize],
            NoEventMask as i32,
            state.wmatom[WM::Delete as usize] as i64,
            CurrentTime as i64,
            0,
            0,
            0,
        ) {
            XGrabServer(state.dpy);
            XSetErrorHandler(Some(xerrordummy));
            XSetCloseDownMode(state.dpy, DestroyAll);
            XKillClient(state.dpy, (*(*state.selmon).sel).win);
            XSync(state.dpy, False);
            XSetErrorHandler(Some(xerror));
            XUngrabServer(state.dpy);
        }
    }
}

pub(crate) fn setlayout(state: &mut State, arg: *const Arg) {
    log::trace!("setlayout: {arg:?}");
    unsafe {
        let monitor = &mut *state.selmon;
        if arg.is_null()
            || (*arg).l().is_none()
            || !std::ptr::eq(
                &state.config.layouts[(*arg).l().unwrap()],
                monitor.lt[monitor.sellt],
            )
        {
            monitor.pertag.sellts[monitor.pertag.curtag] ^= 1;
            monitor.sellt = monitor.pertag.sellts[monitor.pertag.curtag];
        }
        if !arg.is_null() && (*arg).l().is_some() {
            monitor.pertag.ltidxs[monitor.pertag.curtag][monitor.sellt] =
                &state.config.layouts[(*arg).l().unwrap()];
            monitor.lt[monitor.sellt] =
                monitor.pertag.ltidxs[monitor.pertag.curtag][monitor.sellt];
        }
        monitor.ltsymbol = (*monitor.lt[monitor.sellt]).symbol.clone();
        if !monitor.sel.is_null() {
            arrange(state, state.selmon);
        } else {
            drawbar(state, state.selmon);
        }
    }
}

/// Cycle to the next (`arg.i == 1`) or previous (`arg.i == -1`) entry in
/// `state.config.layouts`, wrapping around. `setlayout` only jumps to a
/// specific index or toggles the two remembered slots, so this wraps
/// that with the "current index" lookup needed to cycle through an
/// arbitrary number of layouts.
pub(crate) fn cyclelayout(state: &mut State, arg: *const Arg) {
    unsafe {
        if arg.is_null() {
            return;
        }
        let dir = (*arg).i();
        let monitor = &*state.selmon;
        let cur = monitor.lt[monitor.sellt];
        let n = state.config.layouts.len() as c_int;
        let idx = state
            .config
            .layouts
            .iter()
            .position(|l| std::ptr::eq(l, cur))
            .unwrap_or(0) as c_int;
        let next = ((idx + dir) % n + n) % n;
        setlayout(state, &Arg::L(Some(next as usize)) as *const Arg);
    }
}

/// Shell run once the layout menu's pick is known as `$i`: ask this
/// instance to switch to that layout (see `clientmessage`).
const LAYOUT_PICKED: &str = r#"exec FlexiFlow -l "$i""#;

/// Shell run once the power menu's pick is known as `$i`: the items'
/// commands follow the menu lines as `$2`, `$3`, ..., so shift the
/// picked one into `$1` and run it.
const POWER_PICKED: &str = r#"shift $((i + 1)) && exec sh -c "$1""#;

/// Pipes `state.config.layouts` into the `menu` program, one
/// `"<symbol> <description>"` per line, and switches to whichever one
/// the user picks.
///
/// The whole pipeline runs in the background under `sh`, so the event
/// loop keeps going while the menu is open. Esc or a missing menu
/// program just ends the pipeline without switching.
pub(crate) fn layoutmenu(state: &mut State, _arg: *const Arg) {
    let layouts = &state.config.layouts;
    let script =
        menu_script(&state.config.menu, "Layout", layouts.len(), LAYOUT_PICKED);
    let lines = layout_menu_text(layouts);
    if let Err(e) =
        spawn_child(Command::new("sh").args(["-c", &script, "sh", &lines]))
    {
        log::warn!("layoutmenu: failed to spawn sh: {e}");
    }
}

/// Pipes `state.config.powermenu.items`' labels into the `menu` program
/// and runs the picked item's command through `sh -c`. The same
/// background pipeline as `layoutmenu`: Esc or a missing menu program
/// runs nothing.
pub(crate) fn powermenu(state: &mut State, _arg: *const Arg) {
    let items = &state.config.powermenu.items;
    let script =
        menu_script(&state.config.menu, "Power", items.len(), POWER_PICKED);
    let lines = power_menu_text(items);
    let commands = items.iter().map(|item| item.command.as_str());
    if let Err(e) = spawn_child(
        Command::new("sh")
            .args(["-c", &script, "sh", &lines])
            .args(commands),
    ) {
        log::warn!("powermenu: failed to spawn sh: {e}");
    }
}

/// The popup's title, at its top left beside the search field.
const HOTKEYS_TITLE: &str = "Keybindings";

/// The search field's text before anything is typed.
const HOTKEYS_PLACEHOLDER: &str = "\u{F002}  Type to filter";

/// Accents for the `hotkeys` popup's groups that `Hotkeys::groups`
/// gives no color: KDE Breeze's accents, all different.
const HOTKEYS_PALETTE: [&str; 11] = [
    "#3DAEE9", "#27AE60", "#9B59B6", "#F67400", "#FDBC4B", "#1ABC9C",
    "#DA4453", "#E93A9A", "#2980B9", "#C9CE3B", "#A1A9B1",
];

/// How much of a group's accent tints its key badges.
const HOTKEYS_BADGE_TINT: f32 = 0.18;

/// Shows every key binding that has a `desc` in a popup centred on the
/// selected monitor, like awesome's hotkeys popup: each group under a
/// heading marked in its accent, each key as a row of badges followed
/// by what it does, filling columns top to bottom.
///
/// Typing filters it to the keys whose combination, description or
/// group match; Backspace deletes, and Escape, Return, any key held with
/// Super, Ctrl or Alt, or a click outside it closes it.
///
/// It grabs the keyboard and pointer and waits for those itself, as
/// `movemouse` does, still handling map, configure and expose requests
/// meanwhile.
pub(crate) fn hotkeys(state: &mut State, _arg: *const Arg) {
    let order: Vec<String> = state
        .config
        .hotkeys
        .groups
        .iter()
        .map(|g| g.name.clone())
        .chain(state.config.hotkeys_groups.iter().cloned())
        .collect();
    let keys = state.config.keys.clone();
    let lines = hotkey_lines(&keys, &order);
    if lines.is_empty() {
        return;
    }
    unsafe {
        let m = &*state.selmon;
        let style = HotkeysStyle::new(state, &lines);
        let pad = state.lrpad;

        // `Hotkeys::columns` columns, with the lines shared out evenly
        // between them - more if they'd be taller than 90% of the
        // monitor, fewer if the popup would be wider than it. The popup
        // keeps the size of the unfiltered list while filtering, so it
        // doesn't jump
        let fit = ((m.wh * 9 / 10 - style.top - pad) / state.bh).max(2)
            as usize;
        let fewest = lines.len().div_ceil(fit);
        let mut ncols = state.config.hotkeys.columns.max(fewest);
        let all: Vec<&HotkeyLine> = lines.iter().collect();
        let (w, h, per_col) = loop {
            let mut per_col = lines.len().div_ceil(ncols);
            // headings moved off a column's bottom take rows too:
            // lengthen the columns until they fit in as many as planned
            while hotkey_columns(&all, per_col).len() > ncols {
                per_col += 1;
            }
            let (w, h) = hotkeys_size(state, &style, &all, per_col);
            if w <= m.ww * 19 / 20 || ncols <= fewest.max(1) {
                break (w, h, per_col);
            }
            ncols -= 1;
        };
        // the drawable is only as tall as the bar after a screen change
        let (dw, dh) =
            (state.drw.w.max(w as c_uint), state.drw.h.max(h as c_uint));
        if (dw, dh) != (state.drw.w, state.drw.h) {
            drw::resize(&mut state.drw, dw, dh);
        }

        let mut query = String::new();
        draw_hotkeys(state, &style, &lines, &query, per_col, w, h);

        let x = m.wx + (m.ww - w) / 2 - HOTKEYS_BORDER;
        let y = m.wy + (m.wh - h) / 2 - HOTKEYS_BORDER;
        let mut wa: xlib::XSetWindowAttributes = std::mem::zeroed();
        wa.override_redirect = True;
        wa.border_pixel = style.dim.pixel;
        wa.event_mask = ExposureMask;
        let win = xlib::XCreateWindow(
            state.dpy,
            state.root,
            x,
            y,
            w as c_uint,
            h as c_uint,
            HOTKEYS_BORDER as c_uint,
            xlib::XDefaultDepth(state.dpy, state.screen),
            CopyFromParent as c_uint,
            xlib::XDefaultVisual(state.dpy, state.screen),
            xlib::CWOverrideRedirect | xlib::CWBorderPixel | xlib::CWEventMask,
            &mut wa,
        );
        xlib::XMapRaised(state.dpy, win);
        drw::map(&state.drw, win, 0, 0, w as c_uint, h as c_uint);

        let keyboard = xlib::XGrabKeyboard(
            state.dpy,
            state.root,
            True,
            GrabModeAsync,
            GrabModeAsync,
            CurrentTime,
        );
        let pointer = XGrabPointer(
            state.dpy,
            state.root,
            False,
            xlib::ButtonPressMask as c_uint,
            GrabModeAsync,
            GrabModeAsync,
            XNONE as u64,
            state.cursors.normal.cursor,
            CurrentTime,
        );
        if keyboard == GrabSuccess && pointer == GrabSuccess {
            let mut ev = XEvent { type_: 0 };
            loop {
                XMaskEvent(
                    state.dpy,
                    xlib::KeyPressMask
                        | xlib::ButtonPressMask
                        | ExposureMask
                        | SubstructureRedirectMask,
                    &mut ev,
                );
                match ev.type_ {
                    // clicking inside, say on the search field, keeps it
                    // open
                    xlib::ButtonPress => {
                        let (px, py) = (ev.button.x_root, ev.button.y_root);
                        let outside = px < x
                            || py < y
                            || px >= x + w + HOTKEYS_BORDER * 2
                            || py >= y + h + HOTKEYS_BORDER * 2;
                        if outside {
                            break;
                        }
                    }
                    xlib::KeyPress => {
                        let mut buf = [0 as c_char; 8];
                        let mut keysym: KeySym = 0;
                        let n = xlib::XLookupString(
                            &mut ev.key,
                            buf.as_mut_ptr(),
                            buf.len() as c_int,
                            &mut keysym,
                            null_mut(),
                        );
                        let held = ev.key.state
                            & (Mod4Mask | ControlMask | Mod1Mask);
                        let typed = (n == 1).then(|| buf[0] as u8 as char);
                        let keysym = keysym as c_uint;
                        if held != 0
                            || [XK_Escape, XK_Return, XK_KP_Enter]
                                .contains(&keysym)
                        {
                            break;
                        } else if keysym == XK_BackSpace {
                            query.pop();
                        } else if let Some(c) = typed
                            && (c.is_ascii_graphic() || c == ' ')
                        {
                            query.push(c);
                        } else {
                            // a modifier on its own, or a key that types
                            // nothing
                            continue;
                        }
                        draw_hotkeys(
                            state, &style, &lines, &query, per_col, w, h,
                        );
                        drw::map(
                            &state.drw,
                            win,
                            0,
                            0,
                            w as c_uint,
                            h as c_uint,
                        );
                    }
                    EXPOSE if ev.expose.window == win => drw::map(
                        &state.drw,
                        win,
                        0,
                        0,
                        w as c_uint,
                        h as c_uint,
                    ),
                    CONFIGURE_REQUEST | EXPOSE | MAP_REQUEST => {
                        HANDLER[ev.type_ as usize](state, &mut ev)
                    }
                    _ => {}
                }
            }
        }
        if keyboard == GrabSuccess {
            xlib::XUngrabKeyboard(state.dpy, CurrentTime);
        }
        if pointer == GrabSuccess {
            XUngrabPointer(state.dpy, CurrentTime);
        }
        xlib::XDestroyWindow(state.dpy, win);
        XSync(state.dpy, False);
        // closing the popup uncovers windows: don't let the pointer
        // landing on one steal focus
        let mut ev = XEvent { type_: 0 };
        while XCheckMaskEvent(state.dpy, EnterWindowMask, &mut ev) != 0 {}
    }
}

/// The popup's border width, in `Hotkeys::dim`.
const HOTKEYS_BORDER: c_int = 1;

/// The `hotkeys` popup's colors and measurements, worked out once each
/// time it opens.
struct HotkeysStyle {
    bg: Clr,
    fg: Clr,
    dim: Clr,
    key_fg: Clr,
    key_bg: Clr,
    /// per group, by position: its accent, and its badges' background
    accents: Vec<(Clr, Clr)>,
    /// per group, by position: its heading
    labels: Vec<String>,
    /// where the list starts, below the title and separator
    top: c_int,
}

impl HotkeysStyle {
    fn new(state: &State, lines: &[HotkeyLine]) -> Self {
        let cfg = &state.config.hotkeys;
        let clr = |name: &str, fallback: &str| {
            [name, fallback]
                .into_iter()
                .filter_map(|n| CString::new(n).ok())
                .find_map(|n| drw::scm_try_create(&state.drw, &[n]))
                .map(|s| s[0])
                .unwrap_or(state.scheme[(Scheme::Norm, Col::Bg)])
        };
        let defaults = crate::Hotkeys::default();
        let bg = clr(&cfg.bg, &defaults.bg);
        let key_bg = clr(&cfg.key_bg, &defaults.key_bg);
        let (mut accents, mut labels) = (Vec::new(), Vec::new());
        for line in lines {
            let HotkeyLine::Group(i, name) = line else { continue };
            let group = cfg.groups.iter().find(|g| g.name == *name);
            let palette = HOTKEYS_PALETTE[i % HOTKEYS_PALETTE.len()];
            let accent = clr(group.map_or("", |g| g.color.as_str()), palette);
            accents.push((
                accent,
                drw::clr_mix(&state.drw, &accent, &key_bg, HOTKEYS_BADGE_TINT),
            ));
            labels.push(match group {
                Some(g) if !g.label.is_empty() => g.label.clone(),
                _ => capitalized(name),
            });
        }
        let pad = state.lrpad;
        Self {
            bg,
            fg: clr(&cfg.fg, &defaults.fg),
            dim: clr(&cfg.dim, &defaults.dim),
            key_fg: clr(&cfg.key_fg, &defaults.key_fg),
            key_bg,
            accents,
            labels,
            top: pad / 2 + state.bh + pad,
        }
    }

    /// A scheme drawing `fg` text on `bg`.
    fn scheme(fg: Clr, bg: Clr) -> Vec<Clr> {
        vec![fg, bg, bg]
    }
}

/// `name` with its first letter in capitals.
fn capitalized(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().chain(chars).collect()
    })
}

/// The extra space above a heading that doesn't start its column.
fn hotkeys_group_gap(state: &State) -> c_int {
    state.lrpad / 2
}

/// The gap between a column's keys and its descriptions, and between
/// columns.
fn hotkeys_gap(state: &State) -> c_int {
    state.lrpad * 2
}

/// The padding either side of a key badge's text, and the space between
/// badges.
fn hotkeys_badge_pad(state: &State) -> (c_int, c_int) {
    (state.lrpad / 3, 3)
}

/// The width of `keys` drawn as badges.
fn hotkeys_badges_w(state: &mut State, keys: &str) -> c_int {
    let (pad, space) = hotkeys_badge_pad(state);
    let parts = hotkey_parts(keys);
    let n = parts.len() as c_int;
    parts
        .into_iter()
        .map(|p| textw(&mut state.drw, p, 0) + pad * 2)
        .sum::<c_int>()
        + space * (n - 1).max(0)
}

/// A column's badge and description widths; a heading spans both.
fn hotkeys_column_w(
    state: &mut State,
    style: &HotkeysStyle,
    col: &[&HotkeyLine],
) -> (c_int, c_int) {
    let (mut keys, mut descs, mut heading) = (0, 0, 0);
    for line in col {
        match line {
            HotkeyLine::Group(i, _) => {
                heading = heading
                    .max(textw(&mut state.drw, &style.labels[*i], state.lrpad))
            }
            HotkeyLine::Key(_, k, d) => {
                keys = keys.max(hotkeys_badges_w(state, k));
                descs = descs.max(textw(&mut state.drw, d, 0));
            }
        }
    }
    (keys, descs.max(heading - keys - hotkeys_gap(state)))
}

/// A column's height: a row per line, plus a gap above each heading
/// but its first.
fn hotkeys_column_h(state: &State, col: &[&HotkeyLine]) -> c_int {
    col.iter()
        .enumerate()
        .map(|(i, line)| match line {
            HotkeyLine::Group(..) if i > 0 => {
                state.bh + hotkeys_group_gap(state)
            }
            _ => state.bh,
        })
        .sum()
}

/// The popup's size for `lines` in columns of about `per_col` lines.
fn hotkeys_size(
    state: &mut State,
    style: &HotkeysStyle,
    lines: &[&HotkeyLine],
    per_col: usize,
) -> (c_int, c_int) {
    let pad = state.lrpad;
    let gap = hotkeys_gap(state);
    let columns = hotkey_columns(lines, per_col);
    let widths: c_int = columns
        .iter()
        .map(|col| {
            let (k, d) = hotkeys_column_w(state, style, col);
            k + gap + d
        })
        .sum();
    let header = textw(&mut state.drw, HOTKEYS_TITLE, 0)
        + gap
        + textw(&mut state.drw, HOTKEYS_PLACEHOLDER, pad) * 2;
    let w = (widths + gap * (columns.len() as c_int - 1)).max(header)
        + pad * 2;
    let rows =
        columns.iter().map(|c| hotkeys_column_h(state, c)).max().unwrap_or(0);
    (w, style.top + rows + pad)
}

/// Draws the popup, `lines` filtered by `query`, into `state.drw`.
unsafe fn draw_hotkeys(
    state: &mut State,
    style: &HotkeysStyle,
    lines: &[HotkeyLine],
    query: &str,
    per_col: usize,
    w: c_int,
    h: c_int,
) {
    let pad = state.lrpad;
    let bh = state.bh;
    let gap = hotkeys_gap(state);
    let (badge_pad, badge_space) = hotkeys_badge_pad(state);
    let scheme = HotkeysStyle::scheme;

    unsafe {
        // the background
        drw::setscheme(&mut state.drw, scheme(style.bg, style.bg));
        drw::rect(&mut state.drw, 0, 0, w as c_uint, h as c_uint, true, false);

        // the title, the search field to its right, and a line under both
        let y = pad / 2;
        let titlew = textw(&mut state.drw, HOTKEYS_TITLE, 0);
        drw::setscheme(&mut state.drw, scheme(style.fg, style.bg));
        drw::text(
            &mut state.drw,
            pad,
            y,
            titlew as c_uint,
            bh as c_uint,
            0,
            HOTKEYS_TITLE,
            0,
        );
        let fieldw = textw(&mut state.drw, HOTKEYS_PLACEHOLDER, pad) * 2;
        let (text, fg) = if query.is_empty() {
            (HOTKEYS_PLACEHOLDER.to_string(), style.dim)
        } else {
            (format!("\u{F002}  {query}_"), style.fg)
        };
        drw::setscheme(&mut state.drw, scheme(fg, style.key_bg));
        drw::text(
            &mut state.drw,
            w - pad - fieldw,
            y + 2,
            fieldw as c_uint,
            (bh - 4) as c_uint,
            (pad / 2) as c_uint,
            &text,
            0,
        );
        drw::setscheme(&mut state.drw, scheme(style.dim, style.bg));
        drw::rect(
            &mut state.drw,
            pad,
            y + bh + pad / 2,
            (w - pad * 2) as c_uint,
            1,
            true,
            false,
        );

        let shown = hotkey_filter(lines, query, &style.labels);
        if shown.is_empty() {
            drw::setscheme(&mut state.drw, scheme(style.dim, style.bg));
            drw::text(
                &mut state.drw,
                pad,
                style.top,
                (w - pad * 2) as c_uint,
                bh as c_uint,
                0,
                &format!("No keybindings match \u{201C}{query}\u{201D}"),
                0,
            );
            return;
        }

        let mut x = pad;
        for col in hotkey_columns(&shown, per_col) {
            let (keysw, descw) = hotkeys_column_w(state, style, &col);
            let mut y = style.top;
            for (i, line) in col.iter().enumerate() {
                match line {
                    HotkeyLine::Group(group, _) => {
                        if i > 0 {
                            y += hotkeys_group_gap(state);
                        }
                        // a slim bar in the accent, then the heading in it
                        let (accent, _) = style.accents[*group];
                        drw::setscheme(
                            &mut state.drw,
                            scheme(accent, style.bg),
                        );
                        drw::rect(
                            &mut state.drw,
                            x,
                            y + bh / 5,
                            3,
                            (bh - bh / 5 * 2) as c_uint,
                            true,
                            false,
                        );
                        drw::text(
                            &mut state.drw,
                            x + 3,
                            y,
                            (keysw + gap + descw - 3) as c_uint,
                            bh as c_uint,
                            (pad / 2) as c_uint,
                            &style.labels[*group],
                            0,
                        );
                    }
                    HotkeyLine::Key(group, keys, desc) => {
                        let (_, badge_bg) = style.accents[*group];
                        drw::setscheme(
                            &mut state.drw,
                            scheme(style.key_fg, badge_bg),
                        );
                        let mut bx = x;
                        for part in hotkey_parts(keys) {
                            let bw =
                                textw(&mut state.drw, part, 0) + badge_pad * 2;
                            drw::text(
                                &mut state.drw,
                                bx,
                                y + 3,
                                bw as c_uint,
                                (bh - 6) as c_uint,
                                badge_pad as c_uint,
                                part,
                                0,
                            );
                            bx += bw + badge_space;
                        }
                        drw::setscheme(
                            &mut state.drw,
                            scheme(style.fg, style.bg),
                        );
                        drw::text(
                            &mut state.drw,
                            x + keysw + gap,
                            y,
                            descw as c_uint,
                            bh as c_uint,
                            0,
                            desc,
                            0,
                        );
                    }
                }
                y += bh;
            }
            x += keysw + gap + descw + gap;
        }
    }
}

/// `keys`' modifiers and key, one per badge: `Super+Shift+J` is `Super`,
/// `Shift` and `J`. A `+` at the end is the key itself.
fn hotkey_parts(keys: &str) -> Vec<&str> {
    match keys.strip_suffix("++") {
        Some(mods) => mods.split('+').chain(["+"]).collect(),
        None => keys.split('+').filter(|p| !p.is_empty()).collect(),
    }
}

/// The lines whose keys, description, group or heading contain `query`,
/// ignoring case, under their headings; all of them for an empty query.
/// A group whose name or heading matches keeps all its keys.
fn hotkey_filter<'a, 'b>(
    lines: &'b [HotkeyLine<'a>],
    query: &str,
    labels: &[String],
) -> Vec<&'b HotkeyLine<'a>> {
    let query = query.trim().to_lowercase();
    let has = |s: &str| s.to_lowercase().contains(&query);
    let mut shown = Vec::new();
    let mut heading = None;
    let mut whole_group = false;
    for line in lines {
        match line {
            HotkeyLine::Group(i, name) => {
                heading = Some(line);
                whole_group =
                    has(name) || labels.get(*i).is_some_and(|l| has(l));
            }
            HotkeyLine::Key(_, keys, desc) => {
                if whole_group || has(keys) || has(desc) {
                    if let Some(h) = heading.take() {
                        shown.push(h);
                    }
                    shown.push(line);
                }
            }
        }
    }
    shown
}

/// A line of the hotkeys popup: a group's heading, or one of its keys
/// with its description. Both carry the group's position, which picks
/// its color.
#[derive(Debug, PartialEq)]
enum HotkeyLine<'a> {
    Group(usize, &'a str),
    Key(usize, String, &'a str),
}

/// Every described key under its group's heading, the groups in the
/// order `order` lists them and then in the order they're first bound,
/// and keys without a group last under "other". Keys that share their group, modifiers and description,
/// like the tag keys, share a line: `Super+0-9`.
fn hotkey_lines<'a>(keys: &'a [Key], order: &[String]) -> Vec<HotkeyLine<'a>> {
    let mut groups: Vec<(&str, Vec<(c_uint, &str, Vec<String>)>)> = Vec::new();
    let described = keys.iter().filter(|k| !k.desc.is_empty());
    for key in described
        .clone()
        .filter(|k| !k.group.is_empty())
        .chain(described.filter(|k| k.group.is_empty()))
    {
        let group = if key.group.is_empty() { "other" } else { &key.group };
        let rows = match groups.iter().position(|(g, _)| *g == group) {
            Some(i) => &mut groups[i].1,
            None => {
                groups.push((group, Vec::new()));
                &mut groups.last_mut().unwrap().1
            }
        };
        let name = keysym_name(key.keysym);
        match rows
            .iter_mut()
            .find(|(m, d, _)| *m == key.mod_ && *d == key.desc)
        {
            Some((_, _, names)) => names.push(name),
            None => rows.push((key.mod_, &key.desc, vec![name])),
        }
    }
    // stable, so unlisted groups keep their order after the listed ones
    groups.sort_by_key(|(g, _)| {
        order.iter().position(|o| o == g).unwrap_or(order.len())
    });
    let mut lines = Vec::new();
    for (i, (group, rows)) in groups.into_iter().enumerate() {
        lines.push(HotkeyLine::Group(i, group));
        for (mods, desc, names) in rows {
            let keys = match names.as_slice() {
                [one] => one.clone(),
                [a, b] => format!("{a}/{b}"),
                // the tag keys, 1 to 9 then 0, are all ten digits
                all if all.len() == 10
                    && all.iter().all(|n| {
                        n.len() == 1 && n.as_bytes()[0].is_ascii_digit()
                    }) =>
                {
                    "0-9".to_string()
                }
                [first, .., last] => format!("{first}-{last}"),
                [] => unreachable!(),
            };
            lines.push(HotkeyLine::Key(
                i,
                format!("{}{keys}", mod_names(mods)),
                desc,
            ));
        }
    }
    lines
}

/// `lines` split into columns of about `per_col` lines, never ending a
/// column on a heading: it moves to the top of the next one instead.
fn hotkey_columns<'a, 'b>(
    lines: &[&'b HotkeyLine<'a>],
    per_col: usize,
) -> Vec<Vec<&'b HotkeyLine<'a>>> {
    let mut columns: Vec<Vec<&HotkeyLine>> = vec![Vec::new()];
    for &line in lines {
        let col = columns.last_mut().unwrap();
        let heading_at_bottom =
            matches!(line, HotkeyLine::Group(..)) && col.len() + 1 >= per_col;
        if col.len() >= per_col || (heading_at_bottom && !col.is_empty()) {
            columns.push(Vec::new());
        }
        columns.last_mut().unwrap().push(line);
    }
    columns
}

/// `mods` as `"Super+Ctrl+Alt+Shift+"`, leaving out the ones not held.
fn mod_names(mods: c_uint) -> String {
    [
        (Mod4Mask, "Super"),
        (ControlMask, "Ctrl"),
        (Mod1Mask, "Alt"),
        (ShiftMask, "Shift"),
    ]
    .iter()
    .filter(|(mask, _)| mods & mask != 0)
    .map(|(_, name)| format!("{name}+"))
    .collect()
}

/// A short name for `keysym`: the symbol itself for punctuation, letters
/// in capitals and media keys without their `XF86` prefix.
fn keysym_name(keysym: KeySym) -> String {
    let name = unsafe {
        let s = XKeysymToString(keysym);
        if s.is_null() {
            return format!("{keysym:#x}");
        }
        CStr::from_ptr(s).to_string_lossy().into_owned()
    };
    match name.as_str() {
        "comma" => ",".into(),
        "period" => ".".into(),
        "slash" => "/".into(),
        "bracketleft" => "[".into(),
        "bracketright" => "]".into(),
        "space" => "Space".into(),
        s if s.len() == 1 => s.to_uppercase(),
        s => s.strip_prefix("XF86").unwrap_or(s).to_string(),
    }
}

/// A `sh -c` script that pipes the menu lines in `$1` into `menu` (with
/// `-p prompt -l lines` added), works out which line was picked as `$i`,
/// counting from 0, and then runs `picked`. Nothing runs if the menu is
/// dismissed or prints something that isn't one of its lines.
///
/// Only `menu`'s words are spliced into the script, each one quoted; the
/// lines stay in `$1`, so no label can be mistaken for shell.
fn menu_script(
    menu: &[String],
    prompt: &str,
    lines: usize,
    picked: &str,
) -> String {
    let lines = lines.to_string();
    let argv = menu
        .iter()
        .map(String::as_str)
        .chain(["-p", prompt, "-l", &lines])
        .map(sh_quote)
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"c=$(printf '%s' "$1" | {argv}) && i=$(printf '%s' "$1" | c="$c" awk '$0 == ENVIRON["c"] {{ print NR - 1; exit }}') && [ -n "$i" ] && {picked}"#
    )
}

/// `word` as a single-quoted shell word.
fn sh_quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// One menu line: `text` behind fleximenu's `:<color> ` prefix when
/// there's a `color`, which draws the text up to the next space (a
/// layout symbol, a label's icon) in a box of that color while selected.
fn menu_line(color: &str, text: &str) -> String {
    if color.is_empty() {
        format!("{text}\n")
    } else {
        format!(":{color} {text}\n")
    }
}

fn power_menu_text(items: &[PowerMenuItem]) -> String {
    items
        .iter()
        .map(|item| menu_line(&item.color, &item.label))
        .collect()
}

/// `"<symbol> <description>"` per configured layout, in order.
fn layout_menu_text(layouts: &[Layout]) -> String {
    layouts
        .iter()
        .map(|l| {
            // no arrange function: vanilla dwm's floating layout
            let desc = l.arrange.as_ref().map_or("NULL Layout", |a| a.1);
            menu_line(&l.color, &format!("{} {desc}", l.symbol))
        })
        .collect()
}

pub(crate) fn togglefloating(state: &mut State, _arg: *const Arg) {
    log::trace!("togglefloating: {_arg:?}");
    unsafe {
        if (*state.selmon).sel.is_null() {
            return;
        }
        if (*(*state.selmon).sel).isfullscreen {
            // no support for fullscreen windows
            return;
        }
        (*(*state.selmon).sel).isfloating = !(*(*state.selmon).sel).isfloating
            || (*(*state.selmon).sel).isfixed;
        if (*(*state.selmon).sel).isfloating {
            let sel = &mut *(*state.selmon).sel;
            resize(state, sel, sel.x, sel.y, sel.w, sel.h, 0);
        }
        arrange(state, state.selmon);
    }
}

/// Push clients up (`Arg::I(+N)`) and down (`Arg::I(-N)`) the stack.
///
/// From the [stacker patch](https://dwm.suckless.org/patches/stacker/). This
/// should only be called with an ISINC arg, in their parlance, so also inline
/// their stackpos function, in the branch where this is true
pub(crate) fn pushstack(state: &mut State, arg: *const Arg) {
    fn modulo(n: c_int, m: c_int) -> c_int {
        if n % m < 0 { (n % m) + m } else { n % m }
    }
    unsafe {
        // begin stackpos
        if (*state.selmon).clients.is_null() {
            return;
        }
        if (*state.selmon).sel.is_null() {
            return;
        }
        let mut i;
        let mut c;
        cfor!((
            (i, c) = (0, (*state.selmon).clients);
            c != (*state.selmon).sel;
            (i, c) = (i + is_visible(c) as c_int, (*c).next)) {});
        let mut n;
        cfor!((n = i; !c.is_null(); (n, c) = (n + is_visible(c) as c_int, (*c).next)) {});
        let mut stackpos = modulo(i + (*arg).i(), n);
        // end stackpos

        let sel = (*state.selmon).sel;
        match stackpos.cmp(&0) {
            std::cmp::Ordering::Less => return,
            std::cmp::Ordering::Equal => {
                detach(sel);
                attach(sel);
            }
            std::cmp::Ordering::Greater => {
                let mut p;
                cfor!((
                (p, c) = (null_mut(), (*state.selmon).clients);
                !c.is_null();
                (p, c) = (c, (*c).next)) {
                    stackpos -= (is_visible(c) && c != sel) as c_int;
                    if stackpos == 0 {
                        break;
                    }
                });
                let c = if !c.is_null() { c } else { p };
                detach(sel);
                (*sel).next = (*c).next;
                (*c).next = sel;
            }
        }
        arrange(state, state.selmon);
    }
}

pub(crate) fn tag(state: &mut State, arg: *const Arg) {
    unsafe {
        if !(*state.selmon).sel.is_null() && (*arg).ui() & state.tagmask() != 0
        {
            (*(*state.selmon).sel).tags = (*arg).ui() & state.tagmask();
            focus(state, null_mut());
            arrange(state, state.selmon);
        }
    }
}

/// `tag`, then `view` the same tag, so the focused window is sent there
/// and you follow it.
pub(crate) fn tagview(state: &mut State, arg: *const Arg) {
    tag(state, arg);
    view(state, arg);
}

fn dirtomon(state: &State, dir: i32) -> *mut Monitor {
    unsafe {
        let mut m;

        if dir > 0 {
            m = (*state.selmon).next;
            if m.is_null() {
                m = state.mons;
            }
        } else if state.selmon == state.mons {
            cfor!((m = state.mons; !(*m).next.is_null(); m = (*m).next) {});
        } else {
            cfor!((m = state.mons; (*m).next != state.selmon; m = (*m).next) {});
        }
        m
    }
}

pub(crate) fn focusmon(state: &mut State, arg: *const Arg) {
    unsafe {
        if (*state.mons).next.is_null() {
            return;
        }
        let m = dirtomon(state, (*arg).i());
        if m == state.selmon {
            return;
        }
        unfocus(state, (*state.selmon).sel, false);
        state.selmon = m;
        focus(state, null_mut());
    }
}

fn sendmon(state: &mut State, c: *mut Client, m: *mut Monitor) {
    unsafe {
        if (*c).mon == m {
            return;
        }

        unfocus(state, c, true);
        detach(c);
        detachstack(c);
        (*c).mon = m;
        // assign tags of target monitor
        (*c).tags = (*m).tagset[(*m).seltags];
        attach(c);
        attachstack(c);
        focus(state, null_mut());
        arrange(state, null_mut());
    }
}

pub(crate) fn tagmon(state: &mut State, arg: *const Arg) {
    unsafe {
        if (*state.selmon).sel.is_null() || (*state.mons).next.is_null() {
            return;
        }
        sendmon(state, (*state.selmon).sel, dirtomon(state, (*arg).i()));
    }
}

pub(crate) fn toggleview(state: &mut State, arg: *const Arg) {
    unsafe {
        let monitor = &mut *state.selmon;
        let newtagset =
            monitor.tagset[monitor.seltags] ^ ((*arg).ui() & state.tagmask());

        if newtagset != 0 {
            monitor.tagset[monitor.seltags] = newtagset;

            if newtagset == !0 {
                monitor.pertag.prevtag = monitor.pertag.curtag;
                monitor.pertag.curtag = 0;
            }

            // test if the user did not select the same tag
            if (newtagset & (1 << (monitor.pertag.curtag - 1))) == 0 {
                monitor.pertag.prevtag = monitor.pertag.curtag;
                let mut i;
                cfor!((i = 0; (newtagset & (1 << i)) == 0; i += 1) {});
                monitor.pertag.curtag = i + 1;
            }

            // apply settings for this view
            monitor.nmaster = monitor.pertag.nmasters[monitor.pertag.curtag];
            monitor.mfact = monitor.pertag.mfacts[monitor.pertag.curtag];
            monitor.sellt = monitor.pertag.sellts[monitor.pertag.curtag];
            monitor.lt[monitor.sellt] =
                monitor.pertag.ltidxs[monitor.pertag.curtag][monitor.sellt];
            monitor.lt[monitor.sellt ^ 1] =
                monitor.pertag.ltidxs[monitor.pertag.curtag][monitor.sellt ^ 1];

            if monitor.showbar != monitor.pertag.showbars[monitor.pertag.curtag]
            {
                togglebar(state, null_mut());
            }

            focus(state, null_mut());
            arrange(state, state.selmon);
        }
    }
}

pub(crate) fn quit(state: &mut State, _arg: *const Arg) {
    state.running = false;
}

/// Like `quit`, but tells `main` to re-exec the running binary afterwards
/// instead of exiting - picks up both a rebuilt binary on disk and a
/// reloaded config.lua, since both are read fresh on the next startup.
/// Managed windows survive: `cleanup`'s `unmanage` only reparents them
/// back to root, it doesn't close them.
pub(crate) fn restart(state: &mut State, _arg: *const Arg) {
    // once the event loop exits there's no going back, and a failed exec
    // ends the whole session - so stay up if there's nothing to exec
    if restart_exe().is_none() {
        log::error!("restart: no FlexiFlow binary to re-exec, not restarting");
        return;
    }
    state.restart = true;
    state.running = false;
}

// these are shared between movemouse and resizemouse
const CONFIGURE_REQUEST: i32 = ConfigureRequest;
const EXPOSE: i32 = Expose;
const MAP_REQUEST: i32 = MapRequest;
const MOTION_NOTIFY: i32 = MotionNotify;

pub(crate) fn movemouse(state: &mut State, _arg: *const Arg) {
    log::trace!("movemouse");
    unsafe {
        let c = (*state.selmon).sel;
        if c.is_null() {
            return;
        }
        let c = &mut *c; // reborrow now that it's not null
        if c.isfullscreen {
            return; // no support for moving fullscreen windows with mouse
        }
        restack(state, state.selmon);
        let ocx = c.x;
        let ocy = c.y;
        if XGrabPointer(
            state.dpy,
            state.root,
            False,
            MOUSEMASK as u32,
            GrabModeAsync,
            GrabModeAsync,
            XNONE as u64,
            state.cursors.move_.cursor,
            CurrentTime,
        ) != GrabSuccess
        {
            return;
        }
        let mut x = 0;
        let mut y = 0;
        if getrootptr(state, &mut x, &mut y) == 0 {
            return;
        }
        // nil init?
        let mut ev = XEvent { type_: 0 };
        let mut lasttime = 0;

        // emulating do-while
        loop {
            XMaskEvent(
                state.dpy,
                MOUSEMASK | ExposureMask | SubstructureRedirectMask,
                &mut ev,
            );
            match ev.type_ {
                CONFIGURE_REQUEST | EXPOSE | MAP_REQUEST => {
                    HANDLER[ev.type_ as usize](state, &mut ev);
                }
                MOTION_NOTIFY => {
                    if ev.motion.time - lasttime <= 1000 / 60 {
                        continue;
                    }
                    lasttime = ev.motion.time;
                    let mut nx = ocx + (ev.motion.x - x);
                    let mut ny = ocy + (ev.motion.y - y);
                    if ((*state.selmon).wx - nx).abs()
                        < state.config.snap as c_int
                    {
                        nx = (*state.selmon).wx;
                    } else if (((*state.selmon).wx + (*state.selmon).ww)
                        - (nx + width(c)))
                    .abs()
                        < state.config.snap as c_int
                    {
                        nx = (*state.selmon).wx + (*state.selmon).ww - width(c);
                    }
                    if ((*state.selmon).wy - ny).abs()
                        < state.config.snap as c_int
                    {
                        ny = (*state.selmon).wy;
                    } else if (((*state.selmon).wy + (*state.selmon).wh)
                        - (ny + height(c)))
                    .abs()
                        < state.config.snap as c_int
                    {
                        ny =
                            (*state.selmon).wy + (*state.selmon).wh - height(c);
                    }
                    if !c.isfloating
                        && (*(*state.selmon).lt[(*state.selmon).sellt])
                            .arrange
                            .is_some()
                        && ((nx - c.x).abs() > state.config.snap as c_int
                            || (ny - c.y).abs() > state.config.snap as c_int)
                    {
                        togglefloating(state, null_mut());
                    }
                    if (*(*state.selmon).lt[(*state.selmon).sellt])
                        .arrange
                        .is_none()
                        || c.isfloating
                    {
                        resize(state, c, nx, ny, c.w, c.h, 1);
                    }
                }
                _ => {}
            }
            if ev.type_ == ButtonRelease {
                break;
            }
        }
        XUngrabPointer(state.dpy, CurrentTime);
        let m = recttomon(state, c.x, c.y, c.w, c.h);
        if m != state.selmon {
            sendmon(state, c, m);
            state.selmon = m;
            focus(state, null_mut());
        }
    }
}

pub(crate) fn resizemouse(state: &mut State, _arg: *const Arg) {
    log::trace!("resizemouse");
    unsafe {
        let c = (*state.selmon).sel;
        if c.is_null() {
            return;
        }
        let c = &mut *c;
        if c.isfullscreen {
            return; // no support for resizing fullscreen window with mouse
        }
        restack(state, state.selmon);
        let ocx = c.x;
        let ocy = c.y;
        if XGrabPointer(
            state.dpy,
            state.root,
            False,
            MOUSEMASK as u32,
            GrabModeAsync,
            GrabModeAsync,
            XNONE as u64,
            state.cursors.resize.cursor,
            CurrentTime,
        ) != GrabSuccess
        {
            return;
        }
        XWarpPointer(
            state.dpy,
            XNONE as u64,
            c.win,
            0,
            0,
            0,
            0,
            c.w + c.bw - 1,
            c.h + c.bw - 1,
        );

        let mut ev = XEvent { type_: 0 };
        let mut lasttime = 0;

        // do-while
        loop {
            XMaskEvent(
                state.dpy,
                MOUSEMASK | ExposureMask | SubstructureRedirectMask,
                &mut ev,
            );
            match ev.type_ {
                CONFIGURE_REQUEST | EXPOSE | MAP_REQUEST => {
                    HANDLER[ev.type_ as usize](state, &mut ev);
                }
                MOTION_NOTIFY => {
                    if ev.motion.time - lasttime <= 1000 / 60 {
                        continue;
                    }
                    lasttime = ev.motion.time;
                    let nw = max(ev.motion.x - ocx - 2 * c.bw + 1, 1);
                    let nh = max(ev.motion.y - ocy - 2 * c.bw + 1, 1);
                    if (*c.mon).wx + nw >= (*state.selmon).wx
                        && (*c.mon).wx + nw
                            <= (*state.selmon).wx + (*state.selmon).ww
                        && (*c.mon).wy + nh >= (*state.selmon).wy
                        && (*c.mon).wy + nh
                            <= (*state.selmon).wy + (*state.selmon).wh
                        && !c.isfloating
                        && (*(*state.selmon).lt[(*state.selmon).sellt])
                            .arrange
                            .is_some()
                        && ((nw - c.w).abs() > state.config.snap as c_int
                            || (nh - c.h).abs() > state.config.snap as c_int)
                    {
                        togglefloating(state, null_mut());
                    }
                    if (*(*state.selmon).lt[(*state.selmon).sellt])
                        .arrange
                        .is_none()
                        || c.isfloating
                    {
                        resize(state, c, c.x, c.y, nw, nh, 1);
                    }
                }
                _ => {}
            }
            if ev.type_ == ButtonRelease {
                break;
            }
        }

        XWarpPointer(
            state.dpy,
            XNONE as u64,
            c.win,
            0,
            0,
            0,
            0,
            c.w + c.bw - 1,
            c.h + c.bw - 1,
        );
        XUngrabPointer(state.dpy, CurrentTime);
        while XCheckMaskEvent(state.dpy, EnterWindowMask, &mut ev) != 0 {}
        let m = recttomon(state, c.x, c.y, c.w, c.h);
        if m != state.selmon {
            sendmon(state, c, m);
            state.selmon = m;
            focus(state, null_mut());
        }
    }
}

pub(crate) fn spawn(state: &mut State, arg: *const Arg) {
    unsafe {
        let argv = (*arg).v();

        (*state.selmon).tagset[(*state.selmon).seltags] &= !state.scratch_mask();

        let mut cmd = Command::new(argv[0].clone());
        let cmd = if argv.len() > 1 { cmd.args(&argv[1..]) } else { &mut cmd };

        // a binding to a missing program shouldn't take the WM down with it
        if let Err(e) = spawn_child(cmd) {
            log::warn!("spawn: failed to spawn {:?}: {e}", argv[0]);
        }
    }
}

/// Move the current window to the tag specified by `arg.ui`.
pub(crate) fn toggletag(state: &mut State, arg: *const Arg) {
    unsafe {
        if (*state.selmon).sel.is_null() {
            return;
        }
        let newtags =
            (*(*state.selmon).sel).tags ^ ((*arg).ui() & state.tagmask());
        if newtags != 0 {
            (*(*state.selmon).sel).tags = newtags;
            focus(state, null_mut());
            arrange(state, state.selmon);
        }
    }
}

/// Toggle fullscreen for a window.
///
/// adapted from: https://old.reddit.com/r/dwm/comments/avhkgb/fullscreen_mode/
/// for fixing problems with steam games
pub(crate) fn fullscreen(state: &mut State, _: *const Arg) {
    unsafe {
        if (*state.selmon).sel.is_null() {
            return;
        }
        setfullscreen(
            state,
            (*state.selmon).sel,
            !(*(*state.selmon).sel).isfullscreen,
        )
    }
}

/// Toggle the scratchpad slot `arg.i` (an index into `config.scratchpads`):
/// show/hide it if a client is already tagged for this slot, otherwise
/// spawn that slot's configured command. Generalizes upstream's single
/// global scratch tag to however many slots are configured (flexiflow
/// wants three, each toggled independently by its own key).
pub(crate) fn togglescratch(state: &mut State, arg: *const Arg) {
    unsafe {
        if arg.is_null() {
            return;
        }
        let i = (*arg).i() as usize;
        let Some(spec) = state.config.scratchpads.get(i) else {
            log::warn!("togglescratch: no scratchpad configured at index {i}");
            return;
        };
        let tag = state.scratchtag(i);

        let mut c: *mut Client;
        let mut found = false;
        cfor!((
        c = (*state.selmon).clients;
        !c.is_null();
        c = (*c).next) {
            found = ((*c).tags & tag) != 0;
            if found {
                break;
            }
        });
        if found {
            let newtagset = (*state.selmon).tagset[(*state.selmon).seltags] ^ tag;
            if newtagset != 0 {
                (*state.selmon).tagset[(*state.selmon).seltags] = newtagset;
                focus(state, null_mut());
                arrange(state, state.selmon);
            }
            if is_visible(c) {
                focus(state, c);
                restack(state, state.selmon);
            }
        } else {
            let cmd = spec.cmd.clone();
            spawn(state, &Arg::V(cmd) as *const Arg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LayoutFn;

    fn layout(symbol: &str, arrange: Option<&str>, color: &str) -> Layout {
        Layout {
            symbol: symbol.to_string(),
            arrange: arrange
                .map(|a| LayoutFn::try_from(a.to_string()).unwrap()),
            color: color.to_string(),
        }
    }

    fn described(mod_: c_uint, keysym: u32, desc: &str, group: &str) -> Key {
        Key {
            desc: desc.to_string(),
            group: group.to_string(),
            ..Key::new(mod_, keysym, view, Arg::I(0))
        }
    }

    #[test]
    fn hotkey_lines_name_the_described_keys_under_their_groups() {
        use x11::keysym::{XK_Return, XK_comma, XK_j, XK_m};
        let keys = vec![
            described(Mod4Mask, XK_Return, "Open a terminal", "launcher"),
            described(Mod4Mask | ShiftMask, XK_j, "", "client"),
            described(
                Mod4Mask | ControlMask,
                XK_comma,
                "Previous layout",
                "layout",
            ),
            described(Mod4Mask, XK_m, "Run a program", "launcher"),
        ];
        assert_eq!(
            hotkey_lines(&keys, &[]),
            [
                HotkeyLine::Group(0, "launcher"),
                HotkeyLine::Key(0, "Super+Return".into(), "Open a terminal"),
                HotkeyLine::Key(0, "Super+M".into(), "Run a program"),
                HotkeyLine::Group(1, "layout"),
                HotkeyLine::Key(1, "Super+Ctrl+,".into(), "Previous layout"),
            ]
        );
    }

    #[test]
    fn hotkey_lines_join_keys_and_put_ungrouped_ones_last() {
        use x11::keysym::{XF86XK_AudioPlay, XK_0, XK_1, XK_2};
        let keys = vec![
            described(0, XF86XK_AudioPlay, "Play", ""),
            described(Mod4Mask, XK_1, "View tag", "tag"),
            described(Mod4Mask, XK_2, "View tag", "tag"),
            described(Mod4Mask, XK_0, "View tag", "tag"),
        ];
        assert_eq!(
            hotkey_lines(&keys, &[]),
            [
                HotkeyLine::Group(0, "tag"),
                HotkeyLine::Key(0, "Super+1-0".into(), "View tag"),
                HotkeyLine::Group(1, "other"),
                HotkeyLine::Key(1, "AudioPlay".into(), "Play"),
            ]
        );
    }

    #[test]
    fn hotkey_lines_put_the_listed_groups_first() {
        use x11::keysym::{XK_a, XK_b, XK_c};
        let keys = vec![
            described(Mod4Mask, XK_a, "A", "apps"),
            described(Mod4Mask, XK_b, "B", "tag"),
            described(Mod4Mask, XK_c, "C", "flexiflow"),
        ];
        let order = ["flexiflow".to_string(), "tag".to_string()];
        let groups: Vec<_> = hotkey_lines(&keys, &order)
            .into_iter()
            .filter_map(|l| match l {
                HotkeyLine::Group(_, g) => Some(g),
                HotkeyLine::Key(..) => None,
            })
            .collect();
        assert_eq!(groups, ["flexiflow", "tag", "apps"]);
    }

    #[test]
    fn hotkey_lines_show_all_ten_tag_keys_as_a_digit_range() {
        use x11::keysym::{XK_0, XK_9};
        let keys: Vec<_> = (XK_0..=XK_9)
            .map(|k| described(Mod4Mask, k, "View tag", "tag"))
            .collect();
        assert_eq!(
            hotkey_lines(&keys, &[])[1],
            HotkeyLine::Key(0, "Super+0-9".into(), "View tag")
        );
    }

    #[test]
    fn hotkey_parts_split_the_modifiers_from_the_key() {
        assert_eq!(hotkey_parts("Super+Shift+J"), ["Super", "Shift", "J"]);
        assert_eq!(hotkey_parts("AudioPlay"), ["AudioPlay"]);
        assert_eq!(hotkey_parts("Super++"), ["Super", "+"]);
    }

    #[test]
    fn hotkey_filter_keeps_matching_keys_under_their_headings() {
        let lines = [
            HotkeyLine::Group(0, "media"),
            HotkeyLine::Key(0, "AudioRaiseVolume".into(), "Volume up"),
            HotkeyLine::Key(0, "AudioPlay".into(), "Play/pause"),
            HotkeyLine::Group(1, "gaps"),
            HotkeyLine::Key(1, "Super+G".into(), "Toggle gaps"),
        ];
        let labels = ["Media".to_string(), "Gaps".to_string()];
        assert_eq!(
            hotkey_filter(&lines, "VOLUME", &labels),
            [&lines[0], &lines[1]]
        );
        // a group's heading matching keeps all its keys
        assert_eq!(
            hotkey_filter(&lines, "gap", &labels),
            [&lines[3], &lines[4]]
        );
        assert_eq!(hotkey_filter(&lines, "", &labels).len(), lines.len());
        assert!(hotkey_filter(&lines, "nothing", &labels).is_empty());
    }

    #[test]
    fn hotkey_columns_move_a_heading_off_the_bottom() {
        let key = |d| HotkeyLine::Key(0, String::new(), d);
        let lines = [
            HotkeyLine::Group(0, "a"),
            key("1"),
            key("2"),
            HotkeyLine::Group(1, "b"),
            key("3"),
        ];
        let lines: Vec<_> = lines.iter().collect();
        let columns = hotkey_columns(&lines, 4);
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].len(), 3);
        assert_eq!(columns[1][0], &HotkeyLine::Group(1, "b"));
    }

    #[test]
    fn layout_menu_text_puts_each_color_before_its_symbol() {
        let layouts = vec![
            layout("[]=", Some("tile"), "b"),
            layout("[M]", Some("monocle"), ""),
            layout("><>", None, "h"),
        ];
        assert_eq!(
            layout_menu_text(&layouts),
            ":b []= Tiled Layout\n[M] Monocle Layout\n:h ><> NULL Layout\n"
        );
    }

    #[test]
    fn power_menu_text_lists_the_labels_without_their_commands() {
        let item = |label: &str, command: &str, color: &str| PowerMenuItem {
            label: label.to_string(),
            command: command.to_string(),
            color: color.to_string(),
        };
        let items = vec![
            item("\u{F011} Shutdown", "systemctl poweroff", "r"),
            item(
                "Logout",
                "loginctl terminate-session \"$XDG_SESSION_ID\"",
                "",
            ),
        ];
        assert_eq!(power_menu_text(&items), ":r \u{F011} Shutdown\nLogout\n");
    }

    #[test]
    fn sh_quote_keeps_single_quotes_inside_the_word() {
        assert_eq!(
            sh_quote("FiraCode Nerd Font:size=8"),
            "'FiraCode Nerd Font:size=8'"
        );
        assert_eq!(sh_quote("it's"), r"'it'\''s'");
    }

    /// Runs a `menu_script` under `sh` with a stand-in menu that prints
    /// whatever `pick` (a `sed` script) selects from its input, and
    /// returns what the script printed.
    fn run_menu(
        pick: &str,
        lines: &str,
        picked: &str,
        args: &[&str],
    ) -> String {
        let menu =
            ["sh", "-c", &format!("sed -n '{pick}'"), "menu"].map(String::from);
        let script = menu_script(&menu, "Test", 3, picked);
        let out = Command::new("sh")
            .args(["-c", &script, "sh", lines])
            .args(args)
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap()
    }

    #[test]
    fn menu_script_finds_the_position_of_the_picked_line() {
        // shell syntax in a line is just text to match
        let lines = ":b []= Tiled\n:y it's $HOME `x` [@]\n><> NULL\n";
        assert_eq!(run_menu("2p", lines, r#"printf %s "$i""#, &[]), "1");
        assert_eq!(run_menu("3p", lines, r#"printf %s "$i""#, &[]), "2");
    }

    #[test]
    fn menu_script_runs_nothing_for_a_line_the_menu_never_had() {
        let lines = "one\ntwo\n";
        assert_eq!(run_menu("s/.*/three/p;q", lines, "echo ran", &[]), "");
    }

    #[test]
    fn power_picked_runs_the_command_paired_with_the_pick() {
        let lines = "Logout\nRestart\nShutdown\n";
        let commands = ["echo logout", "echo restart", "echo shutdown"];
        assert_eq!(run_menu("2p", lines, POWER_PICKED, &commands), "restart\n");
    }
}
