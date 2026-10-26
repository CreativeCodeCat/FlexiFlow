//! The system tray, the Wayland way: apps (Discord, Steam, blueman,
//! nm-applet --indicator, ...) publish a StatusNotifierItem on D-Bus and
//! register it with `org.kde.StatusNotifierWatcher`. Under X11 the bar
//! was an XEmbed tray; here it serves the watcher itself (unless another
//! bar already does) and reads each item's icon. Clicks go back to the
//! item: left activates it, middle is its secondary action, scrolling
//! scrolls it, and right opens its menu (com.canonical.dbusmenu) in
//! `config.menu`, like the layout and power menus.
use std::{
    io::Write as _,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use zbus::{
    blocking::{self, Connection, fdo::DBusProxy},
    proxy::CacheProperties,
    message::Header,
    object_server::SignalEmitter,
    zvariant::{OwnedValue, Structure, Value},
};

const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const ITEM: &str = "org.kde.StatusNotifierItem";

pub use flexiflow_river::canvas::Icon;

#[derive(Clone, Debug)]
pub struct Item {
    /// `busname/object/path`, as the watcher lists it
    pub id: String,
    bus: String,
    path: String,
    pub icon: Option<Icon>,
    pub passive: bool,
    menu: Option<String>,
    is_menu: bool,
}

/// The tray's items, in registration order, shared with the bar.
#[derive(Clone)]
pub struct Tray {
    items: Arc<Mutex<Vec<Item>>>,
    wake: Arc<Mutex<UnixStream>>,
    conn: Option<Connection>,
    size: u32,
    menu: Vec<String>,
}

impl Tray {
    /// The items to draw, without the passive (hidden) ones.
    pub fn items(&self) -> Vec<Item> {
        self.items.lock().unwrap().iter().filter(|i| !i.passive && i.icon.is_some()).cloned().collect()
    }

    fn changed(&self) {
        let _ = self.wake.lock().unwrap().write_all(&[2]);
    }

    fn add(&self, bus: String, path: String) {
        // `org.blueman.Tray` and `:1.132` are the same item
        let bus = match (&self.conn, bus.starts_with(':')) {
            (Some(conn), false) => DBusProxy::new(conn)
                .ok()
                .and_then(|d| d.get_name_owner(bus.as_str().try_into().ok()?).ok())
                .map_or(bus, |o| o.to_string()),
            _ => bus,
        };
        let id = format!("{bus}{path}");
        {
            let mut items = self.items.lock().unwrap();
            if items.iter().any(|i| i.id == id) {
                return;
            }
            items.push(Item { id: id.clone(), bus, path, icon: None, passive: false, menu: None, is_menu: false });
        }
        let tray = self.clone();
        std::thread::spawn(move || tray.follow(&id));
    }

    fn remove_bus(&self, bus: &str) {
        let mut items = self.items.lock().unwrap();
        let before = items.len();
        items.retain(|i| i.bus != bus);
        if items.len() != before {
            drop(items);
            self.changed();
        }
    }

    fn proxy(&self, bus: &str, path: &str, iface: &'static str) -> Option<blocking::Proxy<'static>> {
        blocking::proxy::Builder::new(self.conn.as_ref()?)
            .destination(bus.to_string())
            .ok()?
            .path(path.to_string())
            .ok()?
            .interface(iface)
            .ok()?
            .cache_properties(CacheProperties::No)
            .build()
            .ok()
    }

    /// Read item `id`'s properties now and again on each signal it sends
    /// (NewIcon, NewStatus, ...), until it goes away.
    fn follow(&self, id: &str) {
        let Some((bus, path)) = self.items.lock().unwrap().iter().find(|i| i.id == id).map(|i| (i.bus.clone(), i.path.clone()))
        else {
            return;
        };
        let Some(proxy) = self.proxy(&bus, &path, ITEM) else { return };
        self.refresh(id, &proxy);
        let Ok(signals) = proxy.receive_all_signals() else { return };
        for _ in signals {
            if !self.items.lock().unwrap().iter().any(|i| i.id == id) {
                return;
            }
            self.refresh(id, &proxy);
        }
    }

    fn refresh(&self, id: &str, proxy: &blocking::Proxy) {
        let status: String = proxy.get_property("Status").unwrap_or_default();
        let menu = proxy.get_property::<zbus::zvariant::OwnedObjectPath>("Menu").ok().map(|p| p.to_string());
        let is_menu: bool = proxy.get_property("ItemIsMenu").unwrap_or(false);
        let pixmaps: Vec<(i32, i32, Vec<u8>)> = proxy.get_property("IconPixmap").unwrap_or_default();
        let name: String = proxy.get_property("IconName").unwrap_or_default();
        let theme_path: String = proxy.get_property("IconThemePath").unwrap_or_default();
        let icon = (!name.is_empty())
            .then(|| icon_from_theme(&name, &theme_path, self.size))
            .flatten()
            .or_else(|| icon_from_pixmaps(&pixmaps, self.size))
            .map(|i| scale(&i, self.size));
        log::debug!(
            "tray: {id}: status {status:?}, icon {name:?} ({} pixmaps): {}",
            pixmaps.len(),
            icon.as_ref().map_or("none".to_string(), |i| format!("{}x{}", i.w, i.h))
        );
        let mut items = self.items.lock().unwrap();
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.icon = icon;
            item.passive = status == "Passive";
            item.menu = menu.filter(|m| m != "/");
            item.is_menu = is_menu;
        }
        drop(items);
        self.changed();
    }

    /// Mouse `button` 1-5 clicked on `item`.
    pub fn click(&self, item: &Item, button: u32) {
        let tray = self.clone();
        let item = item.clone();
        // D-Bus calls can block on a slow app: off the drawing thread
        std::thread::spawn(move || {
            let Some(proxy) = tray.proxy(&item.bus, &item.path, ITEM) else { return };
            let menu = || match &item.menu {
                Some(m) => tray.menu(&item.bus, m),
                None => drop(proxy.call_method("ContextMenu", &(0i32, 0i32))),
            };
            match button {
                1 if item.is_menu => menu(),
                1 => {
                    if proxy.call_method("Activate", &(0i32, 0i32)).is_err() {
                        menu();
                    }
                }
                2 => drop(proxy.call_method("SecondaryActivate", &(0i32, 0i32))),
                3 => menu(),
                4 | 5 => {
                    let delta = if button == 4 { -1i32 } else { 1 };
                    drop(proxy.call_method("Scroll", &(delta, "vertical")));
                }
                _ => {}
            }
        });
    }

    /// Show the item's dbusmenu in `config.menu`, flattened to one list
    /// (`Parent > Child` for submenus), and click what's picked.
    fn menu(&self, bus: &str, path: &str) {
        let Some(proxy) = self.proxy(bus, path, "com.canonical.dbusmenu") else { return };
        let _: Result<bool, _> = proxy.call("AboutToShow", &(0i32,));
        let props: Vec<&str> = vec![];
        let Ok((_, layout)) = proxy.call::<_, _, (u32, OwnedValue)>("GetLayout", &(0i32, -1i32, props)) else {
            return;
        };
        let mut entries = vec![];
        flatten(&layout, "", &mut entries);
        if entries.is_empty() {
            return;
        }
        let lines: String = entries.iter().map(|(_, l)| format!("{l}\n")).collect();
        let Some(picked) = pick(&self.menu, "Tray", &lines) else { return };
        let Some((id, _)) = entries.iter().find(|(_, l)| *l == picked) else { return };
        let _: Result<(), _> = proxy.call("Event", &(*id, "clicked", Value::from(0i32), 0u32));
    }
}

/// Run the dmenu-style `menu` over `lines`, returning the line picked.
fn pick(menu: &[String], prompt: &str, lines: &str) -> Option<String> {
    let (cmd, args) = menu.split_first()?;
    let n = lines.lines().count().to_string();
    let mut child = Command::new(cmd)
        .args(args)
        .args(["-p", prompt, "-l", &n])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(lines.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string();
    (!s.is_empty()).then_some(s)
}

/// dbusmenu's `(ia{sv}av)` layout node as `(id, label)` entries.
fn flatten(node: &Value, prefix: &str, out: &mut Vec<(i32, String)>) {
    let node = match node {
        Value::Value(v) => v.as_ref(),
        v => v,
    };
    let Value::Structure(s) = node else { return };
    let fields = Structure::fields(s);
    let (Some(Value::I32(id)), Some(Value::Dict(props)), Some(Value::Array(children))) =
        (fields.first(), fields.get(1), fields.get(2))
    else {
        return;
    };
    let prop = |k: &str| -> Option<Value> {
        props.iter().find(|(key, _)| matches!(key, Value::Str(s) if s.as_str() == k)).map(|(_, v)| match v {
            Value::Value(v) => (**v).clone(),
            v => v.clone(),
        })
    };
    let visible = !matches!(prop("visible"), Some(Value::Bool(false)));
    let enabled = !matches!(prop("enabled"), Some(Value::Bool(false)));
    let separator = matches!(prop("type"), Some(Value::Str(t)) if t.as_str() == "separator");
    // GTK-style mnemonics: `_Quit` is "Quit", `__` a literal underscore
    let label = match prop("label") {
        Some(Value::Str(l)) => l.as_str().replace("__", "\u{0}").replace('_', "").replace('\u{0}', "_"),
        _ => String::new(),
    };
    if !visible || separator {
        return;
    }
    let toggle = match (prop("toggle-type"), prop("toggle-state")) {
        (Some(Value::Str(t)), Some(Value::I32(1))) if !t.as_str().is_empty() => "\u{2713} ",
        _ => "",
    };
    let here = if label.is_empty() {
        prefix.to_string()
    } else if prefix.is_empty() {
        format!("{toggle}{label}")
    } else {
        format!("{prefix} > {toggle}{label}")
    };
    if children.is_empty() {
        if *id != 0 && enabled && !label.is_empty() {
            out.push((*id, here));
        }
    } else {
        for child in children.iter() {
            flatten(child, &here, out);
        }
    }
}

/// The pixmap closest to `size` (preferring one at least as big), from
/// ARGB32 in network byte order.
fn icon_from_pixmaps(pixmaps: &[(i32, i32, Vec<u8>)], size: u32) -> Option<Icon> {
    let size = size as i32;
    let (w, h, argb) = pixmaps
        .iter()
        .filter(|(w, h, d)| *w > 0 && *h > 0 && d.len() == (*w * *h * 4) as usize)
        .min_by_key(|(w, ..)| if *w >= size { *w - size } else { 10_000 + size - *w })?;
    let rgba = argb.chunks(4).flat_map(|p| [p[1], p[2], p[3], p[0]]).collect();
    Some(Icon { w: *w as u32, h: *h as u32, rgba })
}

/// The icon theme GTK is set to, then the usual fallbacks.
fn themes() -> Vec<String> {
    let mut themes = vec![];
    if let Some(home) = std::env::var_os("HOME") {
        let ini = Path::new(&home).join(".config/gtk-3.0/settings.ini");
        if let Ok(s) = std::fs::read_to_string(ini)
            && let Some(t) = s.lines().find_map(|l| l.trim().strip_prefix("gtk-icon-theme-name"))
        {
            themes.push(t.trim_start_matches([' ', '=']).trim().to_string());
        }
    }
    themes.extend(["breeze", "breeze-dark", "Adwaita", "hicolor"].map(String::from));
    themes
}

/// `name` from the item's own theme path, the icon themes, or pixmaps;
/// a PNG at least `size` big where there's one, else an SVG.
fn icon_from_theme(name: &str, theme_path: &str, size: u32) -> Option<Icon> {
    if name.starts_with('/') {
        return load(Path::new(name), size);
    }
    let mut roots: Vec<PathBuf> = vec![];
    if !theme_path.is_empty() {
        roots.push(theme_path.into());
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let bases = [format!("{home}/.local/share/icons"), format!("{home}/.icons"), "/usr/share/icons".into()];
    for theme in themes() {
        for base in &bases {
            roots.push(Path::new(base).join(&theme));
        }
    }
    let mut found: Vec<PathBuf> = vec![];
    for root in &roots {
        for ext in ["png", "svg"] {
            let direct = root.join(format!("{name}.{ext}"));
            if direct.exists() {
                found.push(direct);
            }
        }
        let Ok(dirs) = std::fs::read_dir(root) else { continue };
        for d in dirs.flatten() {
            for sub in ["apps", "status", "devices", "panel", "categories", ""] {
                for ext in ["png", "svg"] {
                    let p = d.path().join(sub).join(format!("{name}.{ext}"));
                    if p.exists() {
                        found.push(p);
                    }
                }
            }
        }
        if !found.is_empty() {
            break;
        }
    }
    for ext in ["png", "svg", "xpm"] {
        let p = PathBuf::from(format!("/usr/share/pixmaps/{name}.{ext}"));
        if found.is_empty() && p.exists() {
            found.push(p);
        }
    }
    // a big enough PNG, then any SVG, then the biggest PNG
    let px = |p: &PathBuf| {
        p.to_string_lossy()
            .split('/')
            .find_map(|c| c.split_once('x').and_then(|(w, _)| w.parse::<u32>().ok()))
            .unwrap_or(0)
    };
    let png = |p: &&PathBuf| p.extension().is_some_and(|e| e == "png");
    let best = found
        .iter()
        .filter(png)
        .filter(|p| px(p) >= size)
        .min_by_key(|p| px(p))
        .or_else(|| found.iter().find(|p| p.extension().is_some_and(|e| e == "svg")))
        .or_else(|| found.iter().filter(png).max_by_key(|p| px(p)))?;
    load(best, size)
}

fn load(path: &Path, size: u32) -> Option<Icon> {
    match path.extension()?.to_str()? {
        "png" => load_png(path),
        "svg" => load_svg(path, size),
        _ => None,
    }
}

fn load_png(path: &Path) -> Option<Icon> {
    let mut decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let px = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => px.chunks(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        _ => return None,
    };
    Some(Icon { w: info.width, h: info.height, rgba })
}

fn load_svg(path: &Path, size: u32) -> Option<Icon> {
    let data = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
    let s = tree.size();
    let scale = size as f32 / s.width().max(s.height());
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    // tiny-skia is premultiplied
    let rgba = pixmap
        .data()
        .chunks(4)
        .flat_map(|p| {
            let a = p[3] as u32;
            let un = |c: u8| (c as u32 * 255).checked_div(a).map_or(0, |v| v.min(255) as u8);
            [un(p[0]), un(p[1]), un(p[2]), p[3]]
        })
        .collect();
    Some(Icon { w: size, h: size, rgba })
}

/// `icon` fitted into `size`x`size`, averaging the pixels each output
/// pixel covers, keeping its aspect ratio.
pub fn scale(icon: &Icon, size: u32) -> Icon {
    if icon.w == size && icon.h == size {
        return icon.clone();
    }
    let k = size as f32 / icon.w.max(icon.h) as f32;
    let (w, h) = (((icon.w as f32 * k).round() as u32).max(1), ((icon.h as f32 * k).round() as u32).max(1));
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (x0, x1) = ((x as f32 / k) as u32, (((x + 1) as f32 / k).ceil() as u32).min(icon.w).max(1));
            let (y0, y1) = ((y as f32 / k) as u32, (((y + 1) as f32 / k).ceil() as u32).min(icon.h).max(1));
            let mut acc = [0f32; 4];
            let mut n = 0f32;
            for sy in y0..y1.max(y0 + 1).min(icon.h) {
                for sx in x0..x1.max(x0 + 1).min(icon.w) {
                    let i = ((sy * icon.w + sx) * 4) as usize;
                    let a = icon.rgba[i + 3] as f32;
                    // weight colors by alpha so transparent edges don't darken
                    for (c, v) in acc.iter_mut().take(3).enumerate() {
                        *v += icon.rgba[i + c] as f32 * a;
                    }
                    acc[3] += a;
                    n += 1.0;
                }
            }
            let a = acc[3].max(1.0);
            rgba.extend([acc[0] / a, acc[1] / a, acc[2] / a, acc[3] / n.max(1.0)].map(|v| v.round() as u8));
        }
    }
    Icon { w, h, rgba }
}

struct Watcher {
    tray: Tray,
}

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    /// `service` is a bus name with the item at /StatusNotifierItem, or
    /// (as libappindicator and Electron do) just the item's object path
    /// on the sender's own connection.
    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] hdr: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        let sender = hdr.sender().map(|s| s.to_string()).unwrap_or_default();
        let (bus, path) = if service.starts_with('/') {
            (sender, service.to_string())
        } else {
            (service.to_string(), "/StatusNotifierItem".to_string())
        };
        let id = format!("{bus}{path}");
        // `add` makes blocking D-Bus calls, which would deadlock the
        // object server this runs on
        let tray = self.tray.clone();
        std::thread::spawn(move || tray.add(bus, path));
        let _ = Self::status_notifier_item_registered(&emitter, &id).await;
    }

    async fn register_status_notifier_host(&self, _service: &str) {}

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.tray.items.lock().unwrap().iter().map(|i| i.id.clone()).collect()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(emitter: &SignalEmitter<'_>, service: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(emitter: &SignalEmitter<'_>, service: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Start the tray, waking the bar through `wake` whenever it changes.
/// Without a session bus the tray just stays empty.
pub fn start(wake: UnixStream, size: u32, menu: Vec<String>) -> Tray {
    let mut tray = Tray {
        items: Arc::new(Mutex::new(vec![])),
        wake: Arc::new(Mutex::new(wake)),
        conn: None,
        size,
        menu,
    };
    let conn = match Connection::session() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("tray: no session bus: {e}");
            return tray;
        }
    };
    tray.conn = Some(conn.clone());
    let t = tray.clone();
    std::thread::spawn(move || {
        if let Err(e) = t.run(&conn) {
            log::warn!("tray: {e}");
        }
    });
    tray
}

impl Tray {
    fn run(&self, conn: &Connection) -> zbus::Result<()> {
        let dbus = DBusProxy::new(conn)?;
        let ours = !dbus.name_has_owner(WATCHER.try_into()?)?;
        if ours {
            conn.object_server().at("/StatusNotifierWatcher", Watcher { tray: self.clone() })?;
            conn.request_name(WATCHER)?;
            if let Ok(iface) = conn.object_server().interface::<_, Watcher>("/StatusNotifierWatcher") {
                let _ = zbus::block_on(Watcher::status_notifier_host_registered(iface.signal_emitter()));
            }
            self.discover(&dbus);
        } else {
            // another bar is the watcher: show what's registered with it
            let watcher = self.proxy(WATCHER, "/StatusNotifierWatcher", "org.kde.StatusNotifierWatcher").unwrap();
            let host = format!("org.kde.StatusNotifierHost-{}", std::process::id());
            conn.request_name(host.as_str())?;
            let _: Result<(), _> = watcher.call("RegisterStatusNotifierHost", &(host.as_str(),));
            let ids: Vec<String> = watcher.get_property("RegisteredStatusNotifierItems").unwrap_or_default();
            for id in ids {
                self.add_id(&id);
            }
            let t = self.clone();
            std::thread::spawn(move || {
                let Ok(signals) = watcher.receive_all_signals() else { return };
                for msg in signals {
                    let member = msg.header().member().map(|m| m.to_string());
                    if let (Some("StatusNotifierItemRegistered"), Ok(id)) =
                        (member.as_deref(), msg.body().deserialize::<String>())
                    {
                        t.add_id(&id);
                    }
                }
            });
        }
        // an item goes when its app's connection does
        for change in dbus.receive_name_owner_changed()? {
            let Ok(args) = change.args() else { continue };
            if args.new_owner().is_none() {
                let name = args.name().to_string();
                self.remove_bus(&name);
            }
        }
        Ok(())
    }

    /// Find the items apps published while there was no watcher to
    /// register them with (before the bar started, or while it was
    /// restarting): they don't register again by themselves. Each
    /// connection is searched on a thread of its own, so one app that
    /// never answers holds up only itself.
    fn discover(&self, dbus: &DBusProxy) {
        let Ok(names) = dbus.list_names() else { return };
        for name in names.into_iter().filter(|n| n.starts_with(':')) {
            let tray = self.clone();
            let name = name.to_string();
            std::thread::spawn(move || {
                for path in tray.find_items(&name, "/", 0) {
                    tray.add(name.clone(), path);
                }
            });
        }
    }

    /// The object paths under `path` on `bus` with a StatusNotifierItem.
    fn find_items(&self, bus: &str, path: &str, depth: usize) -> Vec<String> {
        let Some(proxy) = self.proxy(bus, path, "org.freedesktop.DBus.Introspectable") else { return vec![] };
        let Ok(xml) = proxy.call::<_, _, String>("Introspect", &()) else { return vec![] };
        let mut found = vec![];
        // only this object's own interfaces, which come before its child
        // nodes: some apps describe their children inline
        let own = xml.split("<node name=").next().unwrap_or("");
        if own.contains(&format!("<interface name=\"{ITEM}\"")) {
            found.push(path.to_string());
        }
        if depth < 6 {
            for child in xml.split("<node name=\"").skip(1).filter_map(|s| s.split('"').next()) {
                let sub = if path == "/" { format!("/{child}") } else { format!("{path}/{child}") };
                found.extend(self.find_items(bus, &sub, depth + 1));
            }
        }
        found
    }

    /// An id as a watcher lists it: `bus/path`, or a bare bus name.
    fn add_id(&self, id: &str) {
        match id.find('/') {
            Some(i) => self.add(id[..i].to_string(), id[i..].to_string()),
            None => self.add(id.to_string(), "/StatusNotifierItem".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixmaps_closest_to_the_size_win_and_become_rgba() {
        let px = |n: i32| (n, n, [0xffu8, 0x10, 0x20, 0x30].repeat((n * n) as usize));
        let icon = icon_from_pixmaps(&[px(16), px(32), px(64)], 22).unwrap();
        assert_eq!(icon.w, 32);
        assert_eq!(&icon.rgba[..4], &[0x10, 0x20, 0x30, 0xff]);
    }

    #[test]
    fn scaling_averages_down_to_the_size() {
        let icon = Icon { w: 4, h: 2, rgba: [255u8, 0, 0, 255].repeat(8) };
        let s = scale(&icon, 2);
        assert_eq!((s.w, s.h), (2, 1));
        assert_eq!(&s.rgba[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn menus_flatten_into_labelled_leaves() {
        use zbus::zvariant::{Array, Dict, Signature, StructureBuilder};
        fn node(id: i32, label: &str, children: Vec<Value<'static>>) -> Value<'static> {
            let mut props = Dict::new(&Signature::Str, &Signature::Variant);
            props.append(Value::from("label".to_string()), Value::Value(Box::new(Value::from(label.to_string())))).unwrap();
            let mut arr = Array::new(&Signature::Variant);
            for c in children {
                arr.append(Value::Value(Box::new(c))).unwrap();
            }
            StructureBuilder::new()
                .add_field(id)
                .append_field(Value::Dict(props))
                .append_field(Value::Array(arr))
                .build()
                .unwrap()
                .into()
        }
        let root = node(0, "", vec![node(1, "_Open", vec![]), node(2, "More", vec![node(3, "Quit", vec![])])]);
        let mut out = vec![];
        flatten(&root, "", &mut out);
        assert_eq!(out, vec![(1, "Open".to_string()), (3, "More > Quit".to_string())]);
    }
}
