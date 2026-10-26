//! flexiflow-river-bar: the X11 FlexiFlow's built-in bar, as a layer-shell
//! panel on every output. It follows what flexiflow-river publishes over
//! its socket (reconnecting across restarts) and sends clicks back as
//! the `buttons` config.lua binds to the bar.
use std::{
    io::Read as _,
    os::{fd::AsRawFd as _, unix::net::UnixStream},
    sync::{Arc, Mutex},
    time::Duration,
};

use flexiflow_river::{
    config::{Arg, Clk, Config},
    ipc::{self, BarState, Request},
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
};

mod draw;
mod tray;

use draw::Region;
use flexiflow_river::canvas::Canvas;

struct Panel {
    output: wl_output::WlOutput,
    layer: Option<LayerSurface>,
    width: i32,
    configured: bool,
    regions: Vec<(i32, i32, Region)>,
    hover: Option<usize>,
    /// the tray items drawn, in order, if the tray is on this output
    tray: Vec<tray::Item>,
}

struct Bar {
    registry: RegistryState,
    seats: SeatState,
    outputs: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,
    pool: SlotPool,
    pointer: Option<wl_pointer::WlPointer>,
    config: Config,
    fonts: flexiflow_river::font::Fonts,
    height: i32,
    panels: Vec<Panel>,
    state: BarState,
    tray: Option<tray::Tray>,
}

impl Bar {
    fn global(&self, output: &wl_output::WlOutput) -> Option<u32> {
        self.outputs.info(output).map(|i| i.id)
    }

    fn wm_output(&self, output: &wl_output::WlOutput) -> Option<ipc::OutputState> {
        let global = self.global(output)?;
        self.state.outputs.iter().find(|o| o.global == global).cloned()
    }

    /// Show or hide each output's panel as `togglebar` says, and redraw.
    fn sync(&mut self, qh: &QueueHandle<Self>) {
        for i in 0..self.panels.len() {
            let output = self.panels[i].output.clone();
            let show = self.wm_output(&output).is_some_and(|o| o.showbar);
            let panel = &mut self.panels[i];
            match (show, panel.layer.is_some()) {
                (true, false) => {
                    let surface = self.compositor.create_surface(qh);
                    let layer = self.layer_shell.create_layer_surface(
                        qh,
                        surface,
                        Layer::Top,
                        Some("flexiflow-river-bar"),
                        Some(&output),
                    );
                    let edge = if self.config.topbar { Anchor::TOP } else { Anchor::BOTTOM };
                    layer.set_anchor(edge | Anchor::LEFT | Anchor::RIGHT);
                    layer.set_size(0, self.height as u32);
                    layer.set_exclusive_zone(self.height);
                    layer.commit();
                    panel.layer = Some(layer);
                    panel.configured = false;
                }
                (false, true) => {
                    panel.layer = None;
                    panel.regions.clear();
                }
                _ => {}
            }
            self.draw(i);
        }
    }

    /// Whether the tray goes on `output`: the focused one, or the one
    /// `systraypinning` pins it to, counting from the left.
    fn tray_on(&self, output: &wl_output::WlOutput) -> bool {
        let Some(global) = self.global(output) else { return false };
        let outs = &self.state.outputs;
        let pin = self.config.systraypinning;
        let host = if pin == 0 {
            outs.iter().find(|o| o.selected)
        } else if pin <= outs.len() {
            outs.get(pin - 1)
        } else if self.config.systraypinningfailfirst {
            outs.first()
        } else {
            outs.last()
        };
        host.is_some_and(|o| o.global == global)
    }

    fn draw(&mut self, i: usize) {
        let output = self.panels[i].output.clone();
        let Some(o) = self.wm_output(&output) else { return };
        let items = match &self.tray {
            Some(t) if self.tray_on(&output) => t.items(),
            _ => vec![],
        };
        let icons: Vec<tray::Icon> = items.iter().filter_map(|i| i.icon.clone()).collect();
        let panel = &mut self.panels[i];
        let Some(layer) = panel.layer.as_ref().filter(|_| panel.configured) else { return };
        let (w, h) = (panel.width, self.height);
        let Ok((buffer, buf)) = self.pool.create_buffer(w, h, w * 4, wl_shm::Format::Argb8888) else {
            return;
        };
        let mut canvas = Canvas { buf, w, h };
        panel.regions = draw::bar(&mut canvas, &mut self.fonts, &self.config, &self.state, &o, panel.hover, &icons);
        panel.tray = items;
        let surface = layer.wl_surface();
        surface.damage_buffer(0, 0, w, h);
        let _ = buffer.attach_to(surface);
        layer.commit();
    }

    fn click(&mut self, surface: &wl_surface::WlSurface, x: f64, button: u32) {
        let Some(panel) = self.panels.iter().find(|p| p.layer.as_ref().is_some_and(|l| l.wl_surface() == surface))
        else {
            return;
        };
        let x = x as i32;
        let Some((_, _, region)) = panel.regions.iter().find(|(x0, x1, _)| (*x0..*x1).contains(&x)) else {
            return;
        };
        let global = self.global(&panel.output);
        let click = match region {
            Region::Menu => Clk::MenuButton,
            Region::Tag(_) => Clk::TagBar,
            Region::Layout => Clk::LtSymbol,
            Region::Power => Clk::PowerButton,
            Region::Title => Clk::WinTitle,
            Region::Status(Some(area)) => {
                // a named status area runs `statusclick` instead
                let mut argv = self.config.statusclick.clone();
                argv.extend([area.clone(), button.to_string()]);
                spawn(&argv);
                return;
            }
            Region::Status(None) => Clk::StatusText,
            Region::Tray(i) => {
                if let (Some(tray), Some(item)) = (&self.tray, panel.tray.get(*i)) {
                    tray.click(item, button);
                }
                return;
            }
        };
        // the bar has no keyboard focus to see modifiers with, so only
        // the bindings without any reach it
        for b in self.config.buttons.iter().filter(|b| b.clicks(click) && b.button == button && b.mask == 0) {
            let mut arg = b.arg.clone();
            // dwm: a tag binding with no tags of its own is for the
            // clicked tag
            if let Region::Tag(i) = region
                && matches!(arg, Arg::I(0) | Arg::Ui(0))
            {
                arg = Arg::Ui(1 << i);
            }
            let req = Request::Action { output: global, func: b.func.clone(), arg };
            if let Err(e) = ipc::send(&req) {
                log::warn!("can't reach flexiflow-river: {e}");
            }
        }
    }
}

fn spawn(argv: &[String]) {
    if let Some((cmd, args)) = argv.split_first()
        && let Err(e) = std::process::Command::new(cmd).args(args).spawn()
    {
        log::warn!("{cmd:?}: {e}");
    }
}

impl CompositorHandler for Bar {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for Bar {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        self.panels.push(Panel { output, layer: None, width: 0, configured: false, regions: vec![], hover: None, tray: vec![] });
        self.sync(qh);
    }

    fn update_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.sync(qh);
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        self.panels.retain(|p| p.output != output);
    }
}

impl LayerShellHandler for Bar {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        for p in &mut self.panels {
            if p.layer.as_ref() == Some(layer) {
                p.layer = None;
            }
        }
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let Some(i) = self.panels.iter().position(|p| p.layer.as_ref() == Some(layer)) else { return };
        self.panels[i].width = configure.new_size.0.max(1) as i32;
        self.panels[i].configured = true;
        self.draw(i);
    }
}

impl SeatHandler for Bar {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, cap: Capability) {
        if cap == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seats.get_pointer(qh, &seat).ok();
        }
    }
    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, cap: Capability) {
        if cap == Capability::Pointer
            && let Some(p) = self.pointer.take()
        {
            p.release();
        }
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for Bar {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for e in events {
            let Some(i) = self.panels.iter().position(|p| p.layer.as_ref().is_some_and(|l| l.wl_surface() == &e.surface))
            else {
                continue;
            };
            match e.kind {
                PointerEventKind::Motion { .. } | PointerEventKind::Enter { .. } => {
                    let x = e.position.0 as i32;
                    let hover = self.panels[i].regions.iter().find_map(|(x0, x1, r)| match r {
                        Region::Tag(t) if (*x0..*x1).contains(&x) => Some(*t),
                        _ => None,
                    });
                    if self.panels[i].hover != hover {
                        self.panels[i].hover = hover;
                        self.draw(i);
                    }
                }
                PointerEventKind::Leave { .. } => {
                    if self.panels[i].hover.take().is_some() {
                        self.draw(i);
                    }
                }
                // BTN_LEFT, BTN_RIGHT, BTN_MIDDLE as X's buttons 1, 3, 2
                PointerEventKind::Press { button, .. } => {
                    let button = match button {
                        0x110 => 1,
                        0x112 => 2,
                        0x111 => 3,
                        _ => continue,
                    };
                    self.click(&e.surface, e.position.0, button);
                }
                PointerEventKind::Axis { vertical, .. } if vertical.discrete != 0 || vertical.absolute != 0.0 => {
                    let button = if vertical.absolute < 0.0 || vertical.discrete < 0 { 4 } else { 5 };
                    self.click(&e.surface, e.position.0, button);
                }
                _ => {}
            }
        }
    }
}

impl ShmHandler for Bar {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for Bar {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(Bar);
delegate_output!(Bar);
delegate_shm!(Bar);
delegate_seat!(Bar);
delegate_pointer!(Bar);
delegate_layer!(Bar);
delegate_registry!(Bar);

/// Follow flexiflow-river's state on a thread, waking the main loop
/// through `wake` with each update and retrying while it's down.
fn follow(latest: Arc<Mutex<Option<BarState>>>, mut wake: UnixStream) {
    use std::io::Write as _;
    std::thread::spawn(move || {
        loop {
            let _ = ipc::subscribe(|state| {
                *latest.lock().unwrap() = Some(state);
                let _ = wake.write_all(&[1]);
            });
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let conn = Connection::connect_to_env().unwrap_or_else(|e| {
        eprintln!("flexiflow-river-bar: can't connect to a Wayland display: {e}");
        std::process::exit(1)
    });
    let (globals, mut queue) = registry_queue_init(&conn).unwrap();
    let qh = queue.handle();
    let config = Config::load_home();
    let fonts = flexiflow_river::font::Fonts::new(&config.fonts);
    let height = fonts.main.height + config.barpadding;
    let shm = Shm::bind(&globals, &qh).expect("no wl_shm");
    let mut bar = Bar {
        registry: RegistryState::new(&globals),
        seats: SeatState::new(&globals, &qh),
        outputs: OutputState::new(&globals, &qh),
        compositor: CompositorState::bind(&globals, &qh).expect("no wl_compositor"),
        layer_shell: LayerShell::bind(&globals, &qh).expect("no layer shell: is flexiflow-river running?"),
        pool: SlotPool::new(1920 * height as usize * 4, &shm).expect("no shm pool"),
        shm,
        pointer: None,
        config,
        fonts,
        height,
        panels: vec![],
        state: BarState::default(),
        tray: None,
    };

    let latest = Arc::new(Mutex::new(None));
    let (mut wake_rx, wake_tx) = UnixStream::pair().unwrap();
    wake_rx.set_nonblocking(true).unwrap();
    if bar.config.showsystray {
        let size = match bar.config.systrayiconsize {
            s if s > 0 && s <= height => s,
            _ => height,
        } as u32;
        let wake = wake_tx.try_clone().unwrap();
        bar.tray = Some(tray::start(wake, size, bar.config.menu.clone()));
    }
    follow(latest.clone(), wake_tx);

    loop {
        queue.dispatch_pending(&mut bar).unwrap();
        if queue.flush().is_err() {
            break;
        }
        let Some(guard) = queue.prepare_read() else { continue };
        let mut fds = [
            libc::pollfd { fd: guard.connection_fd().as_raw_fd(), events: libc::POLLIN, revents: 0 },
            libc::pollfd { fd: wake_rx.as_raw_fd(), events: libc::POLLIN, revents: 0 },
        ];
        unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) };
        if fds[0].revents != 0 {
            if guard.read().is_err() {
                break;
            }
        } else {
            drop(guard);
        }
        if fds[1].revents != 0 {
            let mut drain = [0u8; 64];
            while wake_rx.read(&mut drain).is_ok_and(|n| n > 0) {}
            // the WM's state, the tray, or both changed
            if let Some(state) = latest.lock().unwrap().take() {
                bar.state = state;
            }
            bar.sync(&qh);
        }
    }
}
