//! flexiflow-river-hotkeys: the X11 FlexiFlow's `hotkeys` popup for river,
//! which flexiflow-river runs for the `hotkeys` binding (`modkey` + `/`).
//! Every key binding with a description, as key badges under color-coded
//! headings, with type-to-filter search.
//!
//! It's a transparent layer-shell overlay over the focused output, with
//! the popup centered in it: the overlay takes the keyboard, so typing
//! filters it, and a click anywhere outside the popup lands on the
//! overlay and closes it. Backspace deletes; Escape, Return or any key
//! held with Super, Ctrl or Alt closes it too.
use flexiflow_river::{
    canvas::Canvas,
    config::Config,
    font::Fonts,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
};

mod lines;
mod popup;

use lines::{HotkeyLine, hotkey_lines};
use popup::{BORDER, Style};

struct App {
    registry: RegistryState,
    seats: SeatState,
    outputs: OutputState,
    shm: Shm,
    pool: Option<SlotPool>,
    layer: LayerSurface,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    config: &'static Config,
    fonts: Fonts,
    lines: Vec<HotkeyLine<'static>>,
    style: Style,
    /// the overlay's size, and the popup's size and lines per column
    area: Option<(i32, i32)>,
    popup: (i32, i32, usize),
    query: String,
    mods: Modifiers,
    exit: bool,
}

impl App {
    /// Where the popup sits in the overlay, border included.
    fn popup_rect(&self) -> (i32, i32, i32, i32) {
        let (aw, ah) = self.area.unwrap_or((0, 0));
        let (w, h, _) = self.popup;
        let (bw, bh) = (w + BORDER * 2, h + BORDER * 2);
        ((aw - bw) / 2, (ah - bh) / 2, bw, bh)
    }

    fn draw(&mut self) {
        let Some((aw, ah)) = self.area else { return };
        let (w, h, per_col) = self.popup;
        let (px, py, pw, ph) = self.popup_rect();

        // the popup on its own, then its border, then both into the
        // otherwise transparent overlay
        let mut inner = vec![0u8; (w * h * 4) as usize];
        let mut c = Canvas { buf: &mut inner, w, h };
        popup::draw(&mut c, &self.style, &mut self.fonts.main, &self.lines, &self.query, per_col);

        let Some(pool) = self.pool.as_mut() else { return };
        let Ok((buffer, buf)) = pool.create_buffer(aw, ah, aw * 4, wl_shm::Format::Argb8888) else {
            return;
        };
        buf.fill(0);
        let mut overlay = Canvas { buf, w: aw, h: ah };
        overlay.rect(px, py, pw, ph, false, self.style.dim);
        let row = (w * 4) as usize;
        for y in 0..h {
            let (oy, ox) = (py + BORDER + y, px + BORDER);
            if oy < 0 || oy >= ah || ox < 0 || ox + w > aw {
                continue;
            }
            let at = ((oy * aw + ox) * 4) as usize;
            overlay.buf[at..at + row].copy_from_slice(&inner[y as usize * row..(y as usize + 1) * row]);
        }
        let surface = self.layer.wl_surface();
        surface.damage_buffer(0, 0, aw, ah);
        let _ = buffer.attach_to(surface);
        self.layer.commit();
    }

    fn key(&mut self, event: KeyEvent) {
        let held = self.mods.logo || self.mods.ctrl || self.mods.alt;
        let sym = event.keysym;
        if held || [Keysym::Escape, Keysym::Return, Keysym::KP_Enter].contains(&sym) {
            self.exit = true;
            return;
        }
        if sym == Keysym::BackSpace {
            self.query.pop();
        } else if let Some(c) = event.utf8.as_deref().and_then(|t| {
            let mut chars = t.chars();
            chars.next().filter(|c| chars.next().is_none() && (c.is_ascii_graphic() || *c == ' '))
        }) {
            self.query.push(c);
        } else {
            // a modifier on its own, or a key that types nothing
            return;
        }
        self.draw();
    }
}

impl CompositorHandler for App {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.exit = true;
    }

    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface, cfg: LayerSurfaceConfigure, _: u32) {
        let area = (cfg.new_size.0.max(1) as i32, cfg.new_size.1.max(1) as i32);
        if self.area != Some(area) {
            self.area = Some(area);
            self.popup = popup::layout(self.config, &self.style, &mut self.fonts.main, &self.lines, area);
            self.pool = SlotPool::new((area.0 * area.1 * 4) as usize, &self.shm).ok();
        }
        self.draw();
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, cap: Capability) {
        if cap == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = self.seats.get_keyboard(qh, &seat, None).ok();
        }
        if cap == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seats.get_pointer(qh, &seat).ok();
        }
    }
    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, _: Capability) {}
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {}
    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        self.key(event);
    }
    fn repeat_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}
    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.mods = modifiers;
    }
}

impl PointerHandler for App {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        let (px, py, pw, ph) = self.popup_rect();
        for e in events {
            // clicking inside, say on the search field, keeps it open
            if let PointerEventKind::Press { .. } = e.kind {
                let (x, y) = (e.position.0 as i32, e.position.1 as i32);
                if x < px || y < py || x >= px + pw || y >= py + ph {
                    self.exit = true;
                }
            }
        }
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_seat!(App);
delegate_keyboard!(App);
delegate_pointer!(App);
delegate_layer!(App);
delegate_registry!(App);

/// Whether another popup is already open: one is enough.
fn already_open() -> bool {
    let me = std::process::id();
    let Ok(entries) = std::fs::read_dir("/proc") else { return false };
    entries.flatten().any(|e| {
        let pid: u32 = e.file_name().to_string_lossy().parse().unwrap_or(0);
        // `comm` keeps only the first 15 bytes of the name
        pid != 0
            && pid != me
            && std::fs::read_to_string(e.path().join("comm")).is_ok_and(|c| c.trim() == "flexiflow-river")
            && std::fs::read_link(e.path().join("exe"))
                .is_ok_and(|p| p.file_name().is_some_and(|n| n == "flexiflow-river-hotkeys"))
    })
}

/// `--png <file> <w>x<h>`: draw the popup as it would open on an area
/// that big into a PNG instead, to see it without opening it.
fn render_png(path: &str, size: &str) {
    let config: &'static Config = Box::leak(Box::new(Config::load_home()));
    let mut order: Vec<String> = config.hotkeys.groups.iter().map(|g| g.name.clone()).collect();
    order.extend(config.hotkeys_groups.iter().cloned());
    let lines = hotkey_lines(&config.keys, &order);
    let mut fonts = Fonts::new(&config.fonts);
    let style = Style::new(config, &fonts.main, &lines);
    let (aw, ah) = size.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))).unwrap_or((2560, 1440));
    let (w, h, per_col) = popup::layout(config, &style, &mut fonts.main, &lines, (aw, ah));
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let mut c = Canvas { buf: &mut buf, w, h };
    popup::draw(&mut c, &style, &mut fonts.main, &lines, "", per_col);
    // ARGB8888 little-endian is BGRA bytes
    let rgba: Vec<u8> = buf.chunks(4).flat_map(|p| [p[2], p[1], p[0], 255]).collect();
    let file = std::fs::File::create(path).expect("can't write the PNG");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().and_then(|mut wr| wr.write_image_data(&rgba)).expect("can't encode the PNG");
    println!("{w}x{h}, {per_col} lines a column");
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [flag, path, size] = args.as_slice()
        && flag == "--png"
    {
        return render_png(path, size);
    }
    if already_open() {
        return;
    }
    // lives as long as the popup, which is the whole process
    let config: &'static Config = Box::leak(Box::new(Config::load_home()));
    let mut order: Vec<String> = config.hotkeys.groups.iter().map(|g| g.name.clone()).collect();
    order.extend(config.hotkeys_groups.iter().cloned());
    let lines = hotkey_lines(&config.keys, &order);
    if lines.is_empty() {
        return;
    }
    let fonts = Fonts::new(&config.fonts);
    let style = Style::new(config, &fonts.main, &lines);

    let conn = Connection::connect_to_env().unwrap_or_else(|e| {
        eprintln!("flexiflow-river-hotkeys: can't connect to a Wayland display: {e}");
        std::process::exit(1)
    });
    let (globals, mut queue) = registry_queue_init(&conn).unwrap();
    let qh = queue.handle();
    let compositor = CompositorState::bind(&globals, &qh).expect("no wl_compositor");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("no layer shell");
    let surface = compositor.create_surface(&qh);
    // no output given: river puts it on the focused one
    let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Overlay, Some("flexiflow-river-hotkeys"), None);
    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    layer.set_size(0, 0);
    layer.commit();

    let mut app = App {
        registry: RegistryState::new(&globals),
        seats: SeatState::new(&globals, &qh),
        outputs: OutputState::new(&globals, &qh),
        shm: Shm::bind(&globals, &qh).expect("no wl_shm"),
        pool: None,
        layer,
        keyboard: None,
        pointer: None,
        config,
        fonts,
        lines,
        style,
        area: None,
        popup: (0, 0, 1),
        query: String::new(),
        mods: Modifiers::default(),
        exit: false,
    };
    while !app.exit {
        if queue.blocking_dispatch(&mut app).is_err() {
            break;
        }
    }
}
