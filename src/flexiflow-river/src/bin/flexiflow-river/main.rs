//! flexiflow-river: FlexiFlow's window management - dwm's tags, layouts,
//! rules, scratchpads and swallowing, configured in the same config.lua -
//! as a window manager for river 0.5, which leaves all window management
//! policy to a client like this one.
//!
//! From scripts, like the X11 FlexiFlow:
//!
//! - `flexiflow-river -b "text"` sets the bar's status text
//! - `flexiflow-river -l N` switches the focused output to `layouts[N]`
//! - `flexiflow-river -c <func> [<arg json>]` runs any key handler, say
//!   `flexiflow-river -c view '{"Ui":4}'`
//!
//! `flexiflow-river --focused` prints the focused window's identifier
//! and geometry, for `grim` (see `flexiflow-shot`).
//!
//! `flexiflow-river --session` starts the whole session: river, running
//! this as its window manager (see [`session`]).
use std::{
    io::{BufRead as _, BufReader, ErrorKind, Write as _},
    os::{
        fd::{AsFd as _, AsRawFd as _},
        unix::net::{UnixListener, UnixStream},
    },
    process::Child,
    sync::Mutex,
};

use flexiflow_river::{
    config::{Arg, Func},
    ipc::{self, Request},
    protocols::{
        layer_shell::river_layer_shell_v1::RiverLayerShellV1,
        window_management::river_window_manager_v1::RiverWindowManagerV1,
        xkb_bindings::river_xkb_bindings_v1::RiverXkbBindingsV1,
    },
};
use wayland_client::{Connection, globals::registry_queue_init};

mod actions;
mod keyboard;
mod menus;
mod restart;
mod wm;

use wm::{Action, Wm};

/// Everything `spawn` started, reaped as it exits.
pub static CHILDREN: Mutex<Vec<Child>> = Mutex::new(vec![]);

fn reap() {
    CHILDREN.lock().unwrap().retain_mut(|c| !matches!(c.try_wait(), Ok(Some(_))));
}

fn usage() -> ! {
    eprintln!("usage: flexiflow-river [--session | --focused | -b <status text> | -l <layout> | -c <func> [<arg json>]]");
    std::process::exit(2);
}

/// `-b`, `-l` and `-c` hand a request to the running instance.
fn client(args: &[String]) -> ! {
    let req = match args {
        [flag] if flag == "--focused" => match ipc::ask(&Request::Focused) {
            Ok(line) if !line.is_empty() => {
                println!("{line}");
                std::process::exit(0)
            }
            Ok(_) => std::process::exit(1),
            Err(e) => {
                eprintln!("flexiflow-river: can't reach the running instance: {e}");
                std::process::exit(1)
            }
        },
        [flag, text] if flag == "-b" => Request::Status(text.clone()),
        [flag, n] if flag == "-l" => Request::Action {
            output: None,
            func: Func("setlayout".into()),
            arg: Arg::L(Some(n.parse().unwrap_or_else(|_| usage()))),
        },
        [flag, func, rest @ ..] if flag == "-c" && rest.len() <= 1 => {
            let line = format!("action - {func} {}", rest.first().map_or("", |s| s));
            Request::parse(&line).unwrap_or_else(|e| {
                eprintln!("flexiflow-river: {e}");
                std::process::exit(2)
            })
        }
        _ => usage(),
    };
    if let Err(e) = ipc::send(&req) {
        eprintln!("flexiflow-river: can't reach the running instance: {e}");
        std::process::exit(1);
    }
    std::process::exit(0);
}

struct Ipc {
    listener: UnixListener,
    /// connections that haven't sent their request yet
    pending: Vec<BufReader<UnixStream>>,
    subscribers: Vec<UnixStream>,
}

impl Ipc {
    fn bind() -> std::io::Result<Self> {
        let path = ipc::socket_path();
        // a live instance would have answered; this one is left over
        if UnixStream::connect(&path).is_err() {
            let _ = std::fs::remove_file(&path);
        }
        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener, pending: vec![], subscribers: vec![] })
    }

    fn accept(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((s, _)) => {
                    let _ = s.set_nonblocking(true);
                    self.pending.push(BufReader::new(s));
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    log::warn!("ipc: {e}");
                    break;
                }
            }
        }
    }

    /// Read whatever requests have arrived, running them on `wm`.
    fn read(&mut self, wm: &mut Wm) {
        let mut keep = vec![];
        for mut r in std::mem::take(&mut self.pending) {
            let mut line = String::new();
            match r.read_line(&mut line) {
                Ok(0) => continue,
                Ok(_) if line.ends_with('\n') => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // put back what was read; a partial line waits
                    if !line.is_empty() {
                        log::warn!("ipc: partial request dropped");
                    }
                    keep.push(r);
                    continue;
                }
                _ => continue,
            }
            match Request::parse(&line) {
                Ok(Request::Status(text)) => {
                    wm.status = text;
                    wm.bar_dirty = true;
                }
                Ok(Request::Action { output, func, arg }) => {
                    wm.actions.push(Action::Run { output, func, arg });
                    wm.manager.manage_dirty();
                }
                Ok(Request::Focused) => {
                    let mut s = r.into_inner();
                    let _ = s.set_nonblocking(false);
                    let _ = writeln!(s, "{}", wm.focused_line());
                }
                Ok(Request::Pointer) => {
                    let (px, py) = wm.seats.first().map_or((0, 0), |(_, s)| s.pointer);
                    let (ax, ay, ..) = wm.sel_output().map_or((0, 0, 0, 0), |o| o.area());
                    let mut s = r.into_inner();
                    let _ = s.set_nonblocking(false);
                    let _ = writeln!(s, "{} {}", px - ax, py - ay);
                }
                Ok(Request::Subscribe) => {
                    let s = r.into_inner();
                    let _ = s.set_nonblocking(false);
                    self.subscribers.push(s);
                    wm.bar = None;
                    wm.bar_dirty = true;
                }
                Err(e) => log::warn!("ipc: {e}"),
            }
        }
        self.pending = keep;
    }

    /// Send the bar what it should draw, if that changed.
    fn publish(&mut self, wm: &mut Wm) {
        if !std::mem::take(&mut wm.bar_dirty) {
            return;
        }
        let state = wm.bar_state();
        if wm.bar.as_ref() == Some(&state) {
            return;
        }
        let line = serde_json::to_string(&state).unwrap() + "\n";
        self.subscribers.retain_mut(|s| s.write_all(line.as_bytes()).is_ok());
        wm.bar = Some(state);
    }
}

fn poll(fds: &mut [libc::pollfd], timeout_ms: i32) {
    unsafe {
        libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout_ms);
    }
}

/// Where `make install` puts the `FlexiFlow` symlink to river.
const RIVER_AS_FLEXIFLOW: &str = "/usr/lib/flexiflow-river/FlexiFlow";

/// Start river with this as its window manager, under the name
/// FlexiFlow. Fetch tools name the window manager after the process
/// that owns the Wayland socket - fastfetch by its `argv[0]`, others by
/// its process name, which comes from the executable's file name - and
/// that's river. So river starts as `FlexiFlow`, through a symlink of
/// that name, which gives it both.
fn session() -> ! {
    use std::os::unix::process::CommandExt as _;
    let via_link = std::path::Path::new(RIVER_AS_FLEXIFLOW).exists();
    let exe = if via_link { RIVER_AS_FLEXIFLOW } else { "river" };
    let err = std::process::Command::new(exe)
        .arg0("FlexiFlow")
        .args(["-c", "flexiflow-river"])
        .exec();
    eprintln!("flexiflow-river: can't start river: {err}");
    std::process::exit(1);
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--session"] {
        session();
    }
    if !args.is_empty() {
        client(&args);
    }
    // a menu's thread waits on its own child, so children can't be left
    // for the kernel to reap
    unsafe {
        libc::signal(libc::SIGCHLD, libc::SIG_DFL);
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }

    let conn = Connection::connect_to_env().unwrap_or_else(|e| {
        eprintln!("flexiflow-river: can't connect to a Wayland display: {e}");
        std::process::exit(1)
    });
    let (globals, mut queue) = registry_queue_init::<Wm>(&conn).unwrap();
    let qh = queue.handle();
    let manager: RiverWindowManagerV1 = globals.bind(&qh, 4..=6, ()).unwrap_or_else(|e| {
        eprintln!(
            "flexiflow-river: no river_window_manager_v1 ({e}); \
             this needs river 0.5 or later, started without another window manager"
        );
        std::process::exit(1)
    });
    let mut wm = Wm::new(qh.clone(), manager);
    wm.xkb = globals.bind::<RiverXkbBindingsV1, _, _>(&qh, 1..=3, ()).ok();
    if wm.xkb.is_none() {
        log::error!("no river_xkb_bindings_v1: key bindings won't work");
    }
    wm.layer_shell = globals.bind::<RiverLayerShellV1, _, _>(&qh, 1..=1, ()).ok();
    wm.keyboards.config = globals.bind(&qh, 1..=3, ()).ok();
    wm.request_keymap();
    wm.restored = restart::load();
    if let Some(saved) = &wm.restored {
        wm.status = saved.status.clone();
        wm.hidevacant = saved.hidevacant;
    }

    let mut ipc = Ipc::bind().unwrap_or_else(|e| {
        eprintln!("flexiflow-river: can't listen on {:?}: {e}", ipc::socket_path());
        std::process::exit(1)
    });

    if std::env::var_os(restart::RESTARTED).is_none() {
        // the bar runs once per session: it outlives restarts,
        // reconnecting to the new instance by itself
        if !wm.config.barcmd.is_empty() {
            actions::spawn(&wm.config.barcmd);
        }
        for argv in &wm.config.autostart {
            actions::spawn(argv);
        }
    }
    // SAFETY: only the main thread changes the environment
    unsafe { std::env::remove_var(restart::RESTARTED) };

    loop {
        if wm.finished {
            if wm.restarting {
                drop(ipc);
                restart::exec();
            }
            break;
        }
        queue.dispatch_pending(&mut wm).unwrap();
        ipc.publish(&mut wm);
        if let Err(e) = queue.flush() {
            log::error!("lost the connection to river: {e}");
            break;
        }
        let Some(guard) = queue.prepare_read() else { continue };
        let mut fds: Vec<libc::pollfd> = [guard.connection_fd().as_raw_fd(), ipc.listener.as_fd().as_raw_fd()]
            .into_iter()
            .chain(ipc.pending.iter().map(|r| r.get_ref().as_raw_fd()))
            .map(|fd| libc::pollfd { fd, events: libc::POLLIN, revents: 0 })
            .collect();
        poll(&mut fds, 1000);
        if fds[0].revents != 0 {
            if let Err(e) = guard.read() {
                log::error!("lost the connection to river: {e}");
                break;
            }
        } else {
            drop(guard);
        }
        if fds[1].revents != 0 {
            ipc.accept();
        }
        ipc.read(&mut wm);
        reap();
    }
    let _ = std::fs::remove_file(ipc::socket_path());
}
