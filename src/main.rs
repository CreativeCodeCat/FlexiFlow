//! flexiflow-rs: a tiling window manager based on dwm (via a fork of rwm).

use rwm::{cleanup, run, scan, setup};
#[cfg(target_os = "linux")]
use xcb::Connection;

use rwm::checkotherwm;
use rwm::util::{die, run_autostart};

pub use rwm::enums;

#[cfg(test)]
mod tests;

/// The name reported for `_NET_WM_NAME` on the EWMH check window (see
/// `core.rs`'s `setup`) — this is what tools like fastfetch (and
/// `xprop -root _NET_SUPPORTING_WM_CHECK`) read as the running window
/// manager's name, and what flexiflow.c itself set via these same three
/// env vars in its `runAutostart`.
const WM_NAME: &str = "flexiflow-rs";

fn main() {
    env_logger::init();

    // SAFETY: called once, single-threaded, before any other threads exist.
    unsafe {
        std::env::set_var("XDG_CURRENT_DESKTOP", WM_NAME);
        std::env::set_var("XDG_SESSION_DESKTOP", WM_NAME);
        std::env::set_var("DESKTOP_SESSION", WM_NAME);
    }

    let dpy = unsafe { x11::xlib::XOpenDisplay(std::ptr::null_mut()) };
    if dpy.is_null() {
        die("flexiflow-rs: cannot open display");
    }

    checkotherwm(dpy);
    let mut state = setup(dpy);
    run_autostart(&state.config.autostart);

    #[cfg(target_os = "linux")]
    {
        let Ok((xcon, _)) = Connection::connect(None) else {
            die("flexiflow-rs: cannot get xcb connection");
        };
        state.xcon = Box::into_raw(Box::new(xcon));
    }

    scan(&mut state);
    run(&mut state);
    cleanup(state);
}
