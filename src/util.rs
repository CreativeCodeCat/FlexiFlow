use libc::{c_void, size_t};
use std::process::Command;

/// Spawn each argv list in `cmds` once, ignoring any that fail to start
/// (missing binary, etc.) — mirrors dwm/flexiflow's `runAutostart`, which
/// doesn't check either.
pub fn run_autostart(cmds: &[Vec<String>]) {
    for argv in cmds {
        let Some((bin, args)) = argv.split_first() else {
            continue;
        };
        if let Err(e) = Command::new(bin).args(args).spawn() {
            log::warn!("autostart: failed to spawn {bin:?}: {e}");
        }
    }
}

pub fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

/// Attempt to allocate with `libc::calloc` and die if the result is null
pub fn ecalloc(nmemb: size_t, size: size_t) -> *mut c_void {
    log::trace!("ecalloc: nmemb = {nmemb}, size = {size}");
    let ret = unsafe { libc::calloc(nmemb, size) };
    if ret.is_null() {
        die("calloc:");
    }
    ret
}
