//! FlexiFlow: a tiling window manager based on dwm.

use flexiflow::{cleanup, run, scan, setup};
#[cfg(target_os = "linux")]
use xcb::Connection;

use flexiflow::checkotherwm;
use flexiflow::util::{die, run_autostart};

pub use flexiflow::enums;

#[cfg(test)]
mod tests;

/// The name reported for `_NET_WM_NAME` on the EWMH check window (see
/// `core.rs`'s `setup`) — this is what tools like fastfetch (and
/// `xprop -root _NET_SUPPORTING_WM_CHECK`) read as the running window
/// manager's name.
const WM_NAME: &str = "FlexiFlow";

/// `FlexiFlow -b TEXT`: like dwm's `xsetroot -name`, just stores
/// `TEXT` as the root window's `WM_NAME` and exits - this is how the
/// already-running instance picks up new status bar text, including the
/// inline color codes `core.rs`'s `drawbar` parses out of it.
/// `flexiflow_bar` runs this every tick.
fn set_status_text(text: &str) {
    unsafe {
        let dpy = x11::xlib::XOpenDisplay(std::ptr::null_mut());
        if dpy.is_null() {
            die("FlexiFlow: cannot open display");
        }
        let root = x11::xlib::XDefaultRootWindow(dpy);
        let c_text = std::ffi::CString::new(text).unwrap_or_default();
        x11::xlib::XStoreName(dpy, root, c_text.as_ptr());
        x11::xlib::XCloseDisplay(dpy);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "-b" {
        set_status_text(args.get(2).map(String::as_str).unwrap_or(""));
        return;
    }

    // default to "warn" so real problems (a bad config file, a
    // spawn failure, ...) are visible without having to know to set
    // RUST_LOG first - env_logger's own default is "off", so without
    // this, every log::warn!/error! in the whole codebase is silent
    // unless the user already knew to opt in
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .init();

    // SAFETY: called once, single-threaded, before any other threads exist.
    unsafe {
        std::env::set_var("XDG_CURRENT_DESKTOP", WM_NAME);
        std::env::set_var("XDG_SESSION_DESKTOP", WM_NAME);
        std::env::set_var("DESKTOP_SESSION", WM_NAME);
    }

    let dpy = unsafe { x11::xlib::XOpenDisplay(std::ptr::null_mut()) };
    if dpy.is_null() {
        die("FlexiFlow: cannot open display");
    }

    checkotherwm(dpy);
    let mut state = setup(dpy);
    run_autostart(&state.config.autostart);

    #[cfg(target_os = "linux")]
    {
        let Ok((xcon, _)) = Connection::connect(None) else {
            die("FlexiFlow: cannot get xcb connection");
        };
        state.xcon = Box::into_raw(Box::new(xcon));
    }

    scan(&mut state);
    run(&mut state);

    let restart = state.restart;
    cleanup(state);

    if restart {
        restart_self();
    }
}

/// Re-executes the running binary in place (`execvp`), used by the
/// `restart` key handler. This replaces the process image, so it picks
/// up both a rebuild on disk (if the binary at this path changed since
/// launch) and a reloaded config.lua (loaded fresh in `setup()` on every
/// startup) - no partial/live config reload needed. Managed windows are
/// unaffected: by the time this runs, `cleanup` has already reparented
/// them back to root without closing them, and the new process's `scan`
/// re-adopts them, just like on a normal startup.
fn restart_self() -> ! {
    use std::os::unix::process::CommandExt as _;

    let exe = std::env::current_exe()
        .unwrap_or_else(|_| std::env::args().next().unwrap_or_default().into());
    let err = std::process::Command::new(exe)
        .args(std::env::args_os().skip(1))
        .exec();
    die(&format!("FlexiFlow: failed to restart: {err}"));
}
