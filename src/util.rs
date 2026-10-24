use libc::{c_void, size_t};
use std::io;
use std::process::{Child, Command};

/// Spawn each argv list in `cmds` once, ignoring any that fail to start
/// (missing binary, etc.) — mirrors dwm/flexiflow's `runAutostart`, which
/// doesn't check either.
pub fn run_autostart(cmds: &[Vec<String>]) {
    for argv in cmds {
        let Some((bin, args)) = argv.split_first() else {
            continue;
        };
        if let Err(e) = spawn_child(Command::new(bin).args(args)) {
            log::warn!("autostart: failed to spawn {bin:?}: {e}");
        }
    }
}

/// `cmd.spawn()`, with SIGCHLD back at its default disposition for the
/// duration of the call. Every process the WM starts goes through this.
///
/// `setup()` ignores SIGCHLD (with SA_NOCLDWAIT) so the WM never leaves
/// zombies, but that breaks two things if left in place while spawning:
/// - an ignored signal survives exec, so the child would inherit it and
///   everything *it* spawns would be reaped by the kernel, failing every
///   wait() it does with ECHILD (flexiflow_bar's `FlexiFlow -b`, its
///   segments' commands, ...)
/// - std's own spawn wait()s on a child whose exec failed (a missing
///   binary), and panics when that wait fails - taking the WM down.
///
/// Setting the default in the parent rather than in a `pre_exec` hook
/// also keeps std on its posix_spawn path, which reports a failed exec as
/// a plain error.
pub fn spawn_child(cmd: &mut Command) -> io::Result<Child> {
    unsafe {
        let mut dfl: libc::sigaction = std::mem::zeroed();
        let mut old: libc::sigaction = std::mem::zeroed();
        libc::sigemptyset(&mut dfl.sa_mask);
        dfl.sa_sigaction = libc::SIG_DFL;
        libc::sigaction(libc::SIGCHLD, &dfl, &mut old);

        let child = cmd.spawn();

        libc::sigaction(libc::SIGCHLD, &old, std::ptr::null_mut());
        // reap anything that exited while SIGCHLD wasn't being ignored
        while libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) > 0 {}
        child
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
