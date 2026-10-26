use flexiflow::{events::Event, handlers, menubuttonw, tagshown, textw};
use x11::xlib::{Button1, XOpenDisplay};

use super::*;

use std::process::{Child, Command};

/// Kills Xvfb however the test ends: left running, it holds cargo's
/// output pipe open and `cargo test` never returns.
struct Xvfb(Child);

impl Drop for Xvfb {
    fn drop(&mut self) {
        let _ = self.0.kill();
        // ignore errors here, macos-latest seems to be stricter about
        // auto-reaping child processes (or something) and fails to
        // unwrap here.
        let _ = self.0.try_wait();
    }
}

#[test]
fn main() {
    // run on the default config, not whatever is in the user's
    // ~/.config: an empty config dir gets the default seeded into it
    let xdg = std::env::temp_dir()
        .join(format!("flexiflow-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&xdg);
    unsafe { std::env::set_var("XDG_CONFIG_HOME", &xdg) };

    // setup xvfb
    let _xvfb = Xvfb(Command::new("Xvfb").arg(":1").spawn().unwrap());

    // wait for xvfb to start
    let mut dpy = unsafe { XOpenDisplay(c":1.0".as_ptr()) };
    while dpy.is_null() {
        dpy = unsafe { XOpenDisplay(c":1.0".as_ptr()) };
    }

    checkotherwm(dpy);
    let mut state = setup(dpy);

    #[cfg(target_os = "linux")]
    {
        let (xcon, _) = Connection::connect(Some(":1.0"))
            .expect("cannot get xcb connection");
        state.xcon = Box::into_raw(Box::new(xcon));
    }

    scan(&mut state);

    // instead of calling `run`, manually send some XEvents

    // test that a left click on the layout symbol, right of the menu
    // button and the shown tags, cycles to the next layout
    let selmon = state.selmon;
    let mut x = menubuttonw(&mut state);
    for i in 0..state.config.tags.len() {
        if tagshown(&state, selmon, i) {
            x += textw(&mut state.drw, &state.config.tags[i], state.lrpad);
        }
    }
    let mut button = Event::button(
        unsafe { (*selmon).barwin },
        Button1,
        x + 5,
        state.dpy,
        0,
    )
    .into_button();
    handlers::buttonpress(&mut state, &mut button);
    unsafe {
        assert!(std::ptr::eq(
            (*selmon).lt[(*selmon).sellt],
            &state.config.layouts[1]
        ));
    }

    cleanup(state);
    let _ = std::fs::remove_dir_all(&xdg);
}
