//! The window manager's state, and the river protocol events that feed
//! it. River tells us about windows, outputs and seats as they come and
//! go, then starts a manage sequence (`manage`), where window management
//! state may change, followed by render sequences (`render`), where only
//! positions, stacking and borders may. Everything that wants to change
//! state - bindings, IPC requests, pointer operations - is queued and
//! applied at the start of the next manage sequence.
use std::collections::HashMap;

use flexiflow_river::{
    config::{Arg, Config, Func, parse_color},
    ipc::{BarState, OutputState},
    layouts::{self, Gaps, R},
    protocols::{
        layer_shell::{
            river_layer_shell_output_v1::{self, RiverLayerShellOutputV1},
            river_layer_shell_seat_v1::{self, RiverLayerShellSeatV1},
            river_layer_shell_v1::RiverLayerShellV1,
        },
        window_management::{
            river_node_v1::RiverNodeV1,
            river_output_v1::{self, RiverOutputV1},
            river_pointer_binding_v1::{self, RiverPointerBindingV1},
            river_seat_v1::{self, Modifiers, RiverSeatV1},
            river_window_manager_v1::{self, RiverWindowManagerV1},
            river_window_v1::{self, Edges, RiverWindowV1},
        },
        xkb_bindings::{
            river_xkb_binding_v1::{self, RiverXkbBindingV1},
            river_xkb_bindings_v1::RiverXkbBindingsV1,
        },
    },
};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, backend::ObjectId,
    event_created_child, globals::GlobalListContents, protocol::wl_registry,
};

pub type Wid = ObjectId;
pub type Oid = ObjectId;

/// Something to do in the next manage sequence.
#[derive(Debug)]
pub enum Action {
    /// `config.keys[i]` was pressed
    Key(usize),
    /// `config.buttons[i]` was pressed on a window
    Button(usize),
    /// a handler from the IPC socket or a menu, on the given output
    Run { output: Option<u32>, func: Func, arg: Arg },
}

/// An interactive pointer move or resize of a floating window.
#[derive(Debug, Clone)]
pub struct Op {
    pub win: Wid,
    pub resize: bool,
    /// the window's floating geometry when the operation started
    pub start: R,
    pub delta: (i32, i32),
}

pub struct Window {
    pub proxy: RiverWindowV1,
    pub node: RiverNodeV1,
    pub app_id: String,
    pub title: String,
    pub parent: Option<Wid>,
    pub identifier: String,
    pub pid: Option<i32>,
    /// rules haven't been applied yet
    pub new: bool,
    pub tags: u32,
    pub output: Option<Oid>,
    pub floating: bool,
    pub fullscreen: bool,
    pub cfact: f32,
    /// floating geometry, of the content (borders sit outside it); a
    /// size of 0 lets the window pick its own, and it's centered once
    /// it has
    pub geom: R,
    pub center: bool,
    /// the content dimensions river last reported
    pub dims: (i32, i32),
    /// min/max dimensions the window asked for
    pub hint: (i32, i32, i32, i32),
    pub isterminal: bool,
    pub noswallow: bool,
    /// the terminal this window swallowed, hidden until it closes
    pub swallowed: Option<Wid>,
    /// this is a terminal hidden by the window that swallowed it
    pub hidden_by: Option<Wid>,
    /// `config.scratchpads` slot this window fills
    pub scratch: Option<usize>,
    /// where the current layout puts the window's content
    pub target: Option<R>,
    // what was last sent, so nothing is sent twice
    pub sent_dims: Option<(i32, i32)>,
    pub sent_tiled: Option<bool>,
    pub sent_fullscreen: bool,
    pub shown: Option<bool>,
}

impl Window {
    fn new(proxy: RiverWindowV1, node: RiverNodeV1) -> Self {
        Self {
            proxy,
            node,
            app_id: String::new(),
            title: String::new(),
            parent: None,
            identifier: String::new(),
            pid: None,
            new: true,
            tags: 0,
            output: None,
            floating: false,
            fullscreen: false,
            cfact: 1.0,
            geom: (0, 0, 0, 0),
            center: true,
            dims: (0, 0),
            hint: (0, 0, 0, 0),
            isterminal: false,
            noswallow: false,
            swallowed: None,
            hidden_by: None,
            scratch: None,
            target: None,
            sent_dims: None,
            sent_tiled: None,
            sent_fullscreen: false,
            shown: None,
        }
    }

    /// A window that can't be resized: its min and max are the same.
    pub fn fixed(&self) -> bool {
        let (minw, minh, maxw, maxh) = self.hint;
        minw > 0 && minw == maxw && minh == maxh
    }
}

/// What dwm's pertag patch keeps per tag (index 0 is for views of more
/// than one tag).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagState {
    pub layout: usize,
    pub prevlayout: usize,
    pub mfact: f32,
    pub nmaster: i32,
    pub showbar: bool,
    pub gaps: (i32, i32, i32, i32, bool),
}

impl TagState {
    pub fn gaps(&self, smartgaps: bool) -> Gaps {
        let (oh, ov, ih, iv, enabled) = self.gaps;
        Gaps { oh, ov, ih, iv, enabled, smartgaps }
    }
}

pub struct Output {
    pub proxy: RiverOutputV1,
    pub layer: Option<RiverLayerShellOutputV1>,
    /// the wl_output global this is
    pub global: u32,
    pub rect: R,
    /// what's left of `rect` after the bar's (and any other layer
    /// surface's) exclusive zone
    pub area: Option<R>,
    pub tagset: [u32; 2],
    pub seltags: usize,
    pub pertag: Vec<TagState>,
    pub curtag: usize,
    pub prevtag: usize,
}

impl Output {
    pub fn tagset(&self) -> u32 {
        self.tagset[self.seltags]
    }

    pub fn tag(&self) -> &TagState {
        &self.pertag[self.curtag]
    }

    pub fn tag_mut(&mut self) -> &mut TagState {
        &mut self.pertag[self.curtag]
    }

    pub fn area(&self) -> R {
        self.area.unwrap_or(self.rect)
    }
}

pub struct Seat {
    pub proxy: RiverSeatV1,
    pub layer: Option<RiverLayerShellSeatV1>,
    pub keys: Vec<RiverXkbBindingV1>,
    pub buttons: Vec<RiverPointerBindingV1>,
    /// the bindings were made but aren't enabled yet
    pub enable: bool,
    pub hovered: Option<Wid>,
    pub pointer: (i32, i32),
    /// the pointer moved since the last manage sequence
    pub moved: bool,
    /// a pointer_enter waiting on whether the pointer moved
    pub entered: Option<Wid>,
    /// windows clicked on since the last manage sequence
    pub interacted: Option<Wid>,
    pub op: Option<Op>,
    pub op_released: bool,
    /// a layer surface (a launcher, a lock screen) holds keyboard focus
    pub layer_focus: bool,
}

pub struct Wm {
    pub qh: QueueHandle<Wm>,
    pub manager: RiverWindowManagerV1,
    pub xkb: Option<RiverXkbBindingsV1>,
    pub layer_shell: Option<RiverLayerShellV1>,
    pub config: Config,
    pub windows: HashMap<Wid, Window>,
    /// dwm's client list: master first
    pub clients: Vec<Wid>,
    /// focus history, most recent first
    pub stack: Vec<Wid>,
    pub outputs: Vec<(Oid, Output)>,
    pub seats: Vec<(ObjectId, Seat)>,
    pub selmon: Option<Oid>,
    pub focused: Option<Wid>,
    pub actions: Vec<Action>,
    pub status: String,
    pub hidevacant: bool,
    /// scratchpad slots whose command was spawned, waiting on the window
    pub scratch_pending: Vec<usize>,
    /// the state the bar was last sent
    pub bar: Option<BarState>,
    pub bar_dirty: bool,
    pub in_manage: bool,
    pub restarting: bool,
    pub finished: bool,
    /// saved by `restart`, for the windows river tells us about
    pub restored: Option<crate::restart::Saved>,
    pub keyboards: crate::keyboard::Keyboards,
}

impl Wm {
    pub fn new(qh: QueueHandle<Wm>, manager: RiverWindowManagerV1) -> Self {
        Self {
            qh,
            manager,
            xkb: None,
            layer_shell: None,
            config: Config::load_home(),
            windows: HashMap::new(),
            clients: vec![],
            stack: vec![],
            outputs: vec![],
            seats: vec![],
            selmon: None,
            focused: None,
            actions: vec![],
            status: String::new(),
            hidevacant: false,
            scratch_pending: vec![],
            bar: None,
            bar_dirty: true,
            in_manage: false,
            restarting: false,
            finished: false,
            restored: None,
            keyboards: Default::default(),
        }
    }

    pub fn ntags(&self) -> usize {
        self.config.tags.len().min(31 - self.config.scratchpads.len())
    }

    pub fn tagmask(&self) -> u32 {
        (1u32 << self.ntags()) - 1
    }

    /// The reserved tag bit of scratchpad slot `i`.
    pub fn scratchtag(&self, i: usize) -> u32 {
        1 << (self.ntags() + i)
    }

    pub fn output(&self, id: &Oid) -> Option<&Output> {
        self.outputs.iter().find(|(o, _)| o == id).map(|(_, o)| o)
    }

    pub fn output_mut(&mut self, id: &Oid) -> Option<&mut Output> {
        self.outputs.iter_mut().find(|(o, _)| o == id).map(|(_, o)| o)
    }

    pub fn sel_output(&self) -> Option<&Output> {
        self.output(self.selmon.as_ref()?)
    }

    pub fn sel_output_mut(&mut self) -> Option<&mut Output> {
        let id = self.selmon.clone()?;
        self.output_mut(&id)
    }

    pub fn is_visible(&self, w: &Wid) -> bool {
        let Some(win) = self.windows.get(w) else { return false };
        if win.hidden_by.is_some() || win.new {
            return false;
        }
        let Some(o) = win.output.as_ref().and_then(|o| self.output(o)) else {
            return false;
        };
        win.tags & o.tagset() != 0
    }

    /// The window focus goes to on `output`: the most recently focused
    /// one that's visible there.
    pub fn top_of_stack(&self, output: &Oid) -> Option<Wid> {
        self.stack
            .iter()
            .find(|w| {
                self.is_visible(w)
                    && self.windows[*w].output.as_ref() == Some(output)
            })
            .cloned()
    }

    /// The window focused on `output`, if it has one.
    pub fn sel_on(&self, output: &Oid) -> Option<Wid> {
        match &self.focused {
            Some(f) if self.windows.get(f).and_then(|w| w.output.as_ref()) == Some(output) => {
                Some(f.clone())
            }
            _ => self.top_of_stack(output),
        }
    }

    pub fn tiled(&self, w: &Wid) -> bool {
        let win = &self.windows[w];
        let floating_layout = win
            .output
            .as_ref()
            .and_then(|o| self.output(o))
            .is_some_and(|o| self.config.layouts[o.tag().layout].arrange.is_none());
        !win.floating && !win.fullscreen && !floating_layout
    }

    pub fn focus(&mut self, w: Option<Wid>) {
        if let Some(w) = &w {
            if let Some(o) = self.windows.get(w).and_then(|w| w.output.clone()) {
                self.selmon = Some(o);
            }
            self.stack.retain(|s| s != w);
            self.stack.insert(0, w.clone());
        }
        if self.focused != w {
            self.focused = w;
            self.bar_dirty = true;
        }
    }

    pub fn remove_window(&mut self, w: &Wid) {
        let Some(win) = self.windows.remove(w) else { return };
        win.node.destroy();
        win.proxy.destroy();
        // a swallowed terminal comes back where the window was
        if let Some(term) = win.swallowed.filter(|t| self.windows.contains_key(t)) {
            let t = self.windows.get_mut(&term).unwrap();
            t.hidden_by = None;
            t.tags = win.tags;
            t.output = win.output.clone();
            t.shown = None;
            if let Some(pos) = self.clients.iter().position(|c| c == w) {
                self.clients[pos] = term.clone();
            }
            self.stack.retain(|s| s != w);
            self.stack.insert(0, term.clone());
            if self.focused.as_ref() == Some(w) {
                self.focused = Some(term);
            }
        }
        self.clients.retain(|c| c != w);
        self.stack.retain(|c| c != w);
        if self.focused.as_ref() == Some(w) {
            self.focused = None;
        }
        for (_, seat) in &mut self.seats {
            if seat.hovered.as_ref() == Some(w) {
                seat.hovered = None;
            }
            if seat.op.as_ref().is_some_and(|op| &op.win == w) {
                seat.op = None;
                seat.proxy.op_end();
            }
        }
        self.bar_dirty = true;
    }

    fn remove_output(&mut self, id: &Oid) {
        let Some(pos) = self.outputs.iter().position(|(o, _)| o == id) else {
            return;
        };
        let (_, out) = self.outputs.remove(pos);
        if let Some(l) = out.layer {
            l.destroy();
        }
        out.proxy.destroy();
        let fallback = self.outputs.first().map(|(o, _)| o.clone());
        for win in self.windows.values_mut() {
            if win.output.as_ref() == Some(id) {
                win.output = fallback.clone();
                win.shown = None;
            }
        }
        if self.selmon.as_ref() == Some(id) {
            self.selmon = fallback;
        }
        self.bar_dirty = true;
    }

    fn new_output(&mut self, proxy: RiverOutputV1) {
        let ntags = self.ntags();
        let c = &self.config;
        let tag = TagState {
            layout: 0,
            prevlayout: 1.min(c.layouts.len() - 1),
            mfact: c.mfact,
            nmaster: c.nmaster,
            showbar: c.showbar,
            gaps: (c.gaps, c.gaps, c.gaps, c.gaps, true),
        };
        let layer = self.layer_shell.as_ref().map(|l| l.get_output(&proxy, &self.qh, proxy.id()));
        let out = Output {
            layer,
            global: 0,
            rect: (0, 0, 0, 0),
            area: None,
            tagset: [1, 1],
            seltags: 0,
            pertag: vec![tag; ntags + 1],
            curtag: 1,
            prevtag: 1,
            proxy: proxy.clone(),
        };
        self.outputs.push((proxy.id(), out));
        if self.selmon.is_none() {
            self.selmon = Some(proxy.id());
        }
        self.bar_dirty = true;
    }

    fn new_seat(&mut self, proxy: RiverSeatV1) {
        let layer = self.layer_shell.as_ref().map(|l| l.get_seat(&proxy, &self.qh, ()));
        let mut seat = Seat {
            proxy: proxy.clone(),
            layer,
            keys: vec![],
            buttons: vec![],
            enable: true,
            hovered: None,
            pointer: (0, 0),
            moved: false,
            entered: None,
            interacted: None,
            op: None,
            op_released: false,
            layer_focus: false,
        };
        if let Some(xkb) = &self.xkb {
            for (i, k) in self.config.keys.iter().enumerate() {
                let mods = Modifiers::from_bits_truncate(k.mod_);
                seat.keys.push(xkb.get_xkb_binding(&proxy, k.keysym, mods, &self.qh, i));
            }
        }
        for (i, b) in self.config.buttons.iter().enumerate() {
            if !b.clicks(flexiflow_river::config::Clk::ClientWin) {
                continue;
            }
            // X buttons 1, 2 and 3 are BTN_LEFT, BTN_MIDDLE and BTN_RIGHT
            let code = match b.button {
                1 => 0x110,
                2 => 0x112,
                3 => 0x111,
                _ => continue,
            };
            let mods = Modifiers::from_bits_truncate(b.mask);
            seat.buttons.push(proxy.get_pointer_binding(code, mods, &self.qh, i));
        }
        let cursor = &self.config.cursor;
        if !cursor.theme.is_empty() {
            proxy.set_xcursor_theme(cursor.theme.clone(), cursor.size);
        }
        self.seats.push((proxy.id(), seat));
    }

    fn seat_mut(&mut self, id: &ObjectId) -> Option<&mut Seat> {
        self.seats.iter_mut().find(|(s, _)| s == id).map(|(_, s)| s)
    }

    // ------------------------------------------------------- sequences --

    /// A manage sequence: everything that changed since the last one.
    pub fn manage(&mut self) {
        self.in_manage = true;
        crate::restart::restore_outputs(self);
        for (_, seat) in &mut self.seats {
            if std::mem::take(&mut seat.enable) {
                seat.keys.iter().for_each(|k| k.enable());
                seat.buttons.iter().for_each(|b| b.enable());
            }
        }
        let new: Vec<Wid> = self
            .clients
            .iter()
            .filter(|w| self.windows[*w].new)
            .cloned()
            .collect();
        for w in new {
            self.manage_new(w);
        }
        self.pointer_focus();
        for action in std::mem::take(&mut self.actions) {
            crate::actions::run(self, action);
        }
        self.pointer_ops();
        // focus somewhere visible, preferring what had it
        let keep = self.focused.clone().filter(|f| self.is_visible(f));
        let next = keep.or_else(|| self.selmon.clone().and_then(|o| self.top_of_stack(&o)));
        self.focus(next);
        self.arrange();
        self.apply_focus();
        // windows river announces from now on are new ones
        self.restored = None;
        self.in_manage = false;
        self.manager.manage_finish();
    }

    /// A render sequence: windows have their new dimensions.
    pub fn render(&mut self) {
        let bw = self.config.borderpx;
        let mut tiled = vec![];
        let mut floating = vec![];
        let mut fullscreen = vec![];
        for w in &self.clients {
            if !self.is_visible(w) {
                continue;
            }
            let tiled_w = self.tiled(w);
            let (area, parent) = {
                let win = &self.windows[w];
                let area = win.output.as_ref().and_then(|o| self.output(o)).map(|o| o.area());
                let parent = win.parent.as_ref().and_then(|p| self.windows.get(p)).map(|p| p.target.unwrap_or(p.geom));
                (area, parent)
            };
            let win = self.windows.get_mut(w).unwrap();
            if win.fullscreen {
                fullscreen.push(w.clone());
                continue;
            }
            if tiled_w {
                if let Some((x, y, ..)) = win.target {
                    win.node.set_position(x, y);
                }
                tiled.push(w.clone());
            } else {
                if win.center && win.dims.0 > 0 {
                    if let Some((ax, ay, aw, ah)) = parent.filter(|p| p.2 > 0).or(area) {
                        let (w, h) = win.dims;
                        win.geom = (ax + (aw - w) / 2, ay + (ah - h) / 2, w, h);
                    }
                    win.center = false;
                }
                win.node.set_position(win.geom.0, win.geom.1);
                floating.push(w.clone());
            }
        }
        // floating windows above tiled ones, the most recently focused
        // on top, and fullscreen windows above them all
        floating.sort_by_key(|w| std::cmp::Reverse(self.stack.iter().position(|s| s == w)));
        for w in tiled.iter().chain(&floating).chain(&fullscreen) {
            self.windows[w].node.place_top();
        }
        for w in self.clients.iter() {
            let win = &self.windows[w];
            if !self.is_visible(w) || win.fullscreen {
                continue;
            }
            let scheme = if self.focused.as_ref() == Some(w) {
                &self.config.colors.sel
            } else {
                &self.config.colors.norm
            };
            let (r, g, b, a) = parse_color(&scheme.border).unwrap_or((0, 0, 0, 0xff));
            // 32-bit premultiplied channels: 0xff becomes 0xffffffff
            let ch = |c: u8| (c as u32 * a as u32 / 255) * 0x0101_0101;
            win.proxy.set_borders(Edges::all(), bw, ch(r), ch(g), ch(b), a as u32 * 0x0101_0101);
        }
        self.manager.render_finish();
    }

    /// Apply rules, swallowing and scratchpads to a window river just
    /// told us about, and put it in the client list.
    fn manage_new(&mut self, w: Wid) {
        let selmon = self.selmon.clone();
        let tagmask = self.tagmask();
        let restored = self.restored.as_mut().and_then(|r| {
            let win = &self.windows[&w];
            r.windows.remove(&win.identifier)
        });
        let win = self.windows.get_mut(&w).unwrap();
        win.new = false;
        win.output = selmon.clone();
        let mut tags = 0;
        let mut focus_tag = false;
        let mut monitor = -1;
        for rule in &self.config.rules {
            if rule.matches(&win.app_id, &win.title) {
                win.isterminal = rule.isterminal;
                win.noswallow = rule.noswallow;
                win.floating = rule.isfloating;
                tags |= rule.tags;
                focus_tag |= rule.focus;
                if rule.monitor >= 0 {
                    monitor = rule.monitor;
                }
            }
        }
        if win.parent.is_some() || win.fixed() {
            win.floating = true;
        }
        if monitor >= 0
            && let Some((o, _)) = self.outputs.get(monitor as usize)
        {
            win.output = Some(o.clone());
        }
        let out_tagset = win
            .output
            .as_ref()
            .and_then(|o| self.outputs.iter().find(|(i, _)| i == o))
            .map_or(1, |(_, o)| o.tagset());
        win.tags = if tags & tagmask != 0 { tags & tagmask } else { out_tagset };

        // a scratchpad's window: its own tag, floating, centered
        let slot = self.config.scratchpads.iter().position(|s| {
            win.app_id == s.class || (win.app_id.is_empty() && win.title == s.class)
        });
        let mut scratch_shown = false;
        if let Some(slot) = slot {
            win.scratch = Some(slot);
            win.tags = 1 << (self.config.tags.len().min(31 - self.config.scratchpads.len()) + slot);
            win.floating = true;
            win.output = selmon.clone();
            scratch_shown = self.scratch_pending.contains(&slot);
            self.scratch_pending.retain(|s| *s != slot);
        }

        if let Some(saved) = restored {
            win.tags = saved.tags;
            win.floating = saved.floating;
            win.fullscreen = saved.fullscreen;
            win.cfact = saved.cfact;
            win.geom = saved.geom;
            win.center = saved.geom.2 == 0;
            win.output = self
                .outputs
                .iter()
                .find(|(_, o)| o.global == saved.output)
                .map(|(i, _)| i.clone())
                .or(win.output.clone());
            focus_tag = false;
        }

        let (output, wtags) = (win.output.clone(), win.tags);
        if scratch_shown && let Some(o) = output.as_ref().and_then(|o| self.output_mut(o)) {
            o.tagset[o.seltags] |= wtags;
        }
        if !self.swallow(&w) {
            // dwm's attach(): new windows go first, into the master area;
            // restored ones keep their old place, at the end
            self.clients.retain(|c| c != &w);
            if self.restored.is_some() {
                self.clients.push(w.clone());
            } else {
                self.clients.insert(0, w.clone());
            }
        }
        if focus_tag
            && let Some(o) = output.as_ref().and_then(|o| self.output_mut(o))
            && o.tagset() & wtags == 0
        {
            crate::actions::view_on(self, &output.clone().unwrap(), wtags);
        }
        if self.is_visible(&w) {
            self.focus(Some(w));
        }
        self.bar_dirty = true;
    }

    /// dwm's swallow patch: a window started from a terminal takes the
    /// terminal's place, which comes back when the window closes.
    fn swallow(&mut self, w: &Wid) -> bool {
        let win = &self.windows[w];
        if win.isterminal || win.noswallow || (win.floating && !self.config.swallowfloating) {
            return false;
        }
        let Some(pid) = win.pid else { return false };
        let term = self
            .clients
            .iter()
            .find(|t| {
                let t = &self.windows[*t];
                t.isterminal
                    && t.swallowed.is_none()
                    && t.pid.is_some_and(|tp| is_descendant(pid, tp))
            })
            .cloned();
        let Some(term) = term else { return false };
        let (tags, output) = {
            let t = &self.windows[&term];
            (t.tags, t.output.clone())
        };
        let pos = self.clients.iter().position(|c| c == &term).unwrap();
        self.clients.retain(|c| c != w);
        let pos = pos.min(self.clients.len().saturating_sub(1));
        self.clients[pos] = w.clone();
        let t = self.windows.get_mut(&term).unwrap();
        t.hidden_by = Some(w.clone());
        let win = self.windows.get_mut(w).unwrap();
        win.swallowed = Some(term.clone());
        win.tags = tags;
        win.output = output;
        win.floating = false;
        if self.focused.as_ref() == Some(&term) {
            self.focused = None;
        }
        true
    }

    /// dwm's focus-follows-mouse: focus the window the pointer moved
    /// onto, and whatever was clicked.
    fn pointer_focus(&mut self) {
        let mut focus = None;
        for (_, seat) in &mut self.seats {
            let moved = std::mem::take(&mut seat.moved);
            if let Some(w) = seat.interacted.take() {
                focus = Some(w);
            } else if let Some(w) = seat.entered.take()
                && moved
                && self.config.focusfollowsmouse
                && seat.op.is_none()
            {
                focus = Some(w);
            }
        }
        let lock = self.config.lock_fullscreen
            && self.focused.as_ref().is_some_and(|f| self.windows[f].fullscreen);
        if let Some(w) = focus.filter(|w| self.is_visible(w))
            && !lock
        {
            self.focus(Some(w));
        }
    }

    fn pointer_ops(&mut self) {
        let mut ended = vec![];
        for (sid, seat) in &mut self.seats {
            let Some(op) = seat.op.clone() else { continue };
            let Some(win) = self.windows.get_mut(&op.win) else { continue };
            let (x, y, w, h) = op.start;
            let (dx, dy) = op.delta;
            if op.resize {
                let (minw, minh, maxw, maxh) = win.hint;
                let clamp = |v: i32, min: i32, max: i32| {
                    let v = v.max(min.max(1));
                    if max > 0 { v.min(max) } else { v }
                };
                win.geom = (x, y, clamp(w + dx, minw, maxw), clamp(h + dy, minh, maxh));
            } else {
                win.geom = (x + dx, y + dy, w, h);
            }
            if std::mem::take(&mut seat.op_released) {
                seat.proxy.op_end();
                if op.resize {
                    win.proxy.inform_resize_end();
                }
                seat.op = None;
                ended.push(sid.clone());
            }
        }
    }

    /// Lay out every output, showing and hiding windows as their tags
    /// come into and out of view.
    fn arrange(&mut self) {
        let bw = self.config.borderpx;
        let smartgaps = self.config.smartgaps;
        for (oid, out) in &self.outputs {
            let tag = out.tag();
            let layout = &self.config.layouts[tag.layout];
            let tiled: Vec<Wid> = self
                .clients
                .iter()
                .filter(|w| {
                    self.is_visible(w)
                        && self.windows[*w].output.as_ref() == Some(oid)
                        && !self.windows[*w].floating
                        && !self.windows[*w].fullscreen
                })
                .cloned()
                .collect();
            let rects = match &layout.arrange {
                Some(a) => {
                    let cfacts: Vec<f32> = tiled.iter().map(|w| self.windows[w].cfact).collect();
                    let nmaster = tag.nmaster.max(0) as u32;
                    layouts::arrange(a, out.area(), nmaster, tag.mfact, tag.gaps(smartgaps), &cfacts)
                }
                None => vec![],
            };
            for (w, (x, y, ww, hh)) in tiled.iter().zip(rects) {
                let win = self.windows.get_mut(w).unwrap();
                win.target = Some((x + bw, y + bw, (ww - 2 * bw).max(1), (hh - 2 * bw).max(1)));
            }
            if layout.arrange.is_none() {
                for w in &tiled {
                    let win = self.windows.get_mut(w).unwrap();
                    win.target = None;
                }
            }
        }

        let ids: Vec<Wid> = self.clients.clone();
        for w in ids.iter().chain(self.hidden_terminals().iter()) {
            let visible = self.is_visible(w);
            let tiled = visible && self.tiled(w);
            let output = self.windows[w]
                .output
                .as_ref()
                .and_then(|o| self.output(o))
                .map(|o| o.proxy.clone());
            let win = self.windows.get_mut(w).unwrap();
            if win.shown != Some(visible) {
                if visible { win.proxy.show() } else { win.proxy.hide() }
                win.shown = Some(visible);
            }
            if !visible {
                continue;
            }
            if win.fullscreen != win.sent_fullscreen {
                if win.fullscreen {
                    if let Some(o) = &output {
                        win.proxy.fullscreen(o);
                    }
                    win.proxy.inform_fullscreen();
                } else {
                    win.proxy.exit_fullscreen();
                    win.proxy.inform_not_fullscreen();
                    win.sent_dims = None;
                }
                win.sent_fullscreen = win.fullscreen;
            }
            if win.fullscreen {
                continue;
            }
            if win.sent_tiled != Some(tiled) {
                win.proxy.set_tiled(if tiled { Edges::all() } else { Edges::None });
                win.sent_tiled = Some(tiled);
            }
            let dims = if tiled {
                win.target.map(|(.., w, h)| (w, h))
            } else {
                if win.target.take().is_some() && win.geom.2 == 0 {
                    // floated out of the layout: keep its tiled size
                    win.center = true;
                }
                Some((win.geom.2.max(0), win.geom.3.max(0)))
            };
            if let Some(d) = dims
                && win.sent_dims != Some(d)
            {
                win.proxy.propose_dimensions(d.0, d.1);
                win.sent_dims = Some(d);
            }
        }
        self.bar_dirty = true;
    }

    fn hidden_terminals(&self) -> Vec<Wid> {
        self.windows
            .iter()
            .filter(|(_, w)| w.hidden_by.is_some())
            .map(|(id, _)| id.clone())
            .collect()
    }

    fn apply_focus(&mut self) {
        let focused = self.focused.clone().and_then(|f| self.windows.get(&f).map(|w| w.proxy.clone()));
        for (_, seat) in &self.seats {
            match &focused {
                Some(w) => seat.proxy.focus_window(w),
                None => seat.proxy.clear_focus(),
            }
        }
        if let (Some(o), Some(_)) = (self.sel_output(), &self.layer_shell)
            && let Some(l) = &o.layer
        {
            l.set_default();
        }
    }

    /// The `focused` request's answer: the focused window's identifier
    /// and content geometry as `<id> <x>,<y> <w>x<h>`, or empty.
    pub fn focused_line(&self) -> String {
        let Some(win) = self.focused.as_ref().and_then(|f| self.windows.get(f)) else {
            return String::new();
        };
        let (x, y, w, h) = if win.fullscreen {
            win.output.as_ref().and_then(|o| self.output(o)).map_or(win.geom, |o| o.rect)
        } else {
            let (x, y) = win.target.map_or((win.geom.0, win.geom.1), |t| (t.0, t.1));
            (x, y, win.dims.0, win.dims.1)
        };
        format!("{} {x},{y} {w}x{h}", win.identifier)
    }

    /// What the bar should draw now.
    pub fn bar_state(&self) -> BarState {
        let outputs = self
            .outputs
            .iter()
            .map(|(oid, o)| {
                let mut occupied = 0;
                for w in &self.clients {
                    let win = &self.windows[w];
                    if win.output.as_ref() == Some(oid) {
                        occupied |= win.tags;
                    }
                }
                let sel = self.sel_on(oid).map(|w| &self.windows[&w]);
                let layout = &self.config.layouts[o.tag().layout];
                let ltsymbol = if layout.arrange.as_deref() == Some("monocle") {
                    let n = self
                        .clients
                        .iter()
                        .filter(|w| self.is_visible(w) && self.windows[*w].output.as_ref() == Some(oid))
                        .count();
                    if n > 0 { format!("[{n}]") } else { layout.symbol.clone() }
                } else {
                    layout.symbol.clone()
                };
                OutputState {
                    global: o.global,
                    selected: self.selmon.as_ref() == Some(oid),
                    showbar: o.tag().showbar,
                    tags: o.tagset() & self.tagmask(),
                    occupied: occupied & self.tagmask(),
                    urgent: 0,
                    seltags: sel.map_or(0, |w| w.tags),
                    ltsymbol,
                    title: sel.map(|w| w.title.clone()),
                    floating: sel.is_some_and(|w| w.floating),
                }
            })
            .collect();
        BarState { outputs, status: self.status.clone(), hidevacant: self.hidevacant }
    }
}

/// Whether process `pid` is `ancestor` or one of its descendants.
fn is_descendant(mut pid: i32, ancestor: i32) -> bool {
    for _ in 0..64 {
        if pid == ancestor {
            return true;
        }
        if pid <= 1 {
            return false;
        }
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            return false;
        };
        // the parent pid is the second field after the parenthesized name
        let Some(ppid) = stat
            .rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().nth(1))
            .and_then(|p| p.parse().ok())
        else {
            return false;
        };
        pid = ppid;
    }
    false
}

// ------------------------------------------------------------- dispatch --

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Wm {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<RiverWindowManagerV1, ()> for Wm {
    fn event(
        wm: &mut Self,
        _: &RiverWindowManagerV1,
        event: river_window_manager_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        use river_window_manager_v1::Event;
        match event {
            Event::Unavailable => {
                log::error!("another window manager is already running");
                std::process::exit(1);
            }
            Event::Finished => wm.finished = true,
            Event::ManageStart => wm.manage(),
            Event::RenderStart => wm.render(),
            Event::SessionLocked | Event::SessionUnlocked => {}
            Event::Window { id } => {
                let node = id.get_node(qh, ());
                wm.clients.push(id.id());
                wm.windows.insert(id.id(), Window::new(id, node));
            }
            Event::Output { id } => wm.new_output(id),
            Event::Seat { id } => wm.new_seat(id),
            _ => {}
        }
    }

    event_created_child!(Wm, RiverWindowManagerV1, [
        river_window_manager_v1::EVT_WINDOW_OPCODE => (RiverWindowV1, ()),
        river_window_manager_v1::EVT_OUTPUT_OPCODE => (RiverOutputV1, ()),
        river_window_manager_v1::EVT_SEAT_OPCODE => (RiverSeatV1, ()),
    ]);
}

impl Dispatch<RiverWindowV1, ()> for Wm {
    fn event(
        wm: &mut Self,
        proxy: &RiverWindowV1,
        event: river_window_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use river_window_v1::Event;
        let id = proxy.id();
        if let Event::Closed = event {
            wm.remove_window(&id);
            return;
        }
        let Some(win) = wm.windows.get_mut(&id) else { return };
        match event {
            Event::DimensionsHint { min_width, min_height, max_width, max_height } => {
                win.hint = (min_width, min_height, max_width, max_height);
            }
            Event::Dimensions { width, height } => {
                win.dims = (width, height);
                if !wm.tiled_or_fullscreen(&id) {
                    let win = wm.windows.get_mut(&id).unwrap();
                    win.geom.2 = width;
                    win.geom.3 = height;
                }
            }
            Event::AppId { app_id } => win.app_id = app_id.unwrap_or_default(),
            Event::Title { title } => {
                win.title = title.unwrap_or_default();
                wm.bar_dirty = true;
            }
            Event::Parent { parent } => win.parent = parent.map(|p| p.id()),
            Event::DecorationHint { .. } => proxy.use_ssd(),
            Event::FullscreenRequested { .. } => {
                win.fullscreen = true;
                if !wm.in_manage {
                    wm.manager.manage_dirty();
                }
            }
            Event::ExitFullscreenRequested => win.fullscreen = false,
            Event::PointerMoveRequested { .. } | Event::PointerResizeRequested { .. } => {}
            Event::UnreliablePid { unreliable_pid } => win.pid = Some(unreliable_pid),
            Event::Identifier { identifier } => win.identifier = identifier,
            _ => {}
        }
    }
}

impl Wm {
    fn tiled_or_fullscreen(&self, w: &Wid) -> bool {
        self.windows.get(w).is_some_and(|win| win.fullscreen || (self.tiled(w) && win.target.is_some()))
    }
}

impl Dispatch<RiverOutputV1, ()> for Wm {
    fn event(
        wm: &mut Self,
        proxy: &RiverOutputV1,
        event: river_output_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use river_output_v1::Event;
        let id = proxy.id();
        match event {
            Event::Removed => wm.remove_output(&id),
            Event::WlOutput { name } => {
                if let Some(o) = wm.output_mut(&id) {
                    o.global = name;
                }
            }
            Event::Position { x, y } => {
                if let Some(o) = wm.output_mut(&id) {
                    o.rect.0 = x;
                    o.rect.1 = y;
                }
                wm.sort_outputs();
            }
            Event::Dimensions { width, height } => {
                if let Some(o) = wm.output_mut(&id) {
                    o.rect.2 = width;
                    o.rect.3 = height;
                }
            }
            _ => {}
        }
        wm.bar_dirty = true;
    }
}

impl Wm {
    /// Outputs left to right, then top to bottom, for `focusmon`.
    fn sort_outputs(&mut self) {
        self.outputs.sort_by_key(|(_, o)| (o.rect.0, o.rect.1));
    }
}

impl Dispatch<RiverLayerShellOutputV1, Oid> for Wm {
    fn event(
        wm: &mut Self,
        _: &RiverLayerShellOutputV1,
        event: river_layer_shell_output_v1::Event,
        output: &Oid,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let river_layer_shell_output_v1::Event::NonExclusiveArea { x, y, width, height } = event
            && let Some(o) = wm.output_mut(output)
        {
            o.area = Some((x, y, width, height));
        }
    }
}

impl Dispatch<RiverLayerShellSeatV1, ()> for Wm {
    fn event(
        wm: &mut Self,
        proxy: &RiverLayerShellSeatV1,
        event: river_layer_shell_seat_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use river_layer_shell_seat_v1::Event;
        for (_, seat) in &mut wm.seats {
            if seat.layer.as_ref() == Some(proxy) {
                seat.layer_focus = matches!(event, Event::FocusExclusive);
            }
        }
    }
}

impl Dispatch<RiverSeatV1, ()> for Wm {
    fn event(
        wm: &mut Self,
        proxy: &RiverSeatV1,
        event: river_seat_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use river_seat_v1::Event;
        let id = proxy.id();
        if let Event::Removed = event {
            if let Some(pos) = wm.seats.iter().position(|(s, _)| s == &id) {
                let (_, seat) = wm.seats.remove(pos);
                seat.keys.iter().for_each(|k| k.destroy());
                seat.buttons.iter().for_each(|b| b.destroy());
                if let Some(l) = seat.layer {
                    l.destroy();
                }
                seat.proxy.destroy();
            }
            return;
        }
        let Some(seat) = wm.seat_mut(&id) else { return };
        match event {
            Event::PointerEnter { window } => {
                seat.hovered = Some(window.id());
                seat.entered = Some(window.id());
            }
            Event::PointerLeave => seat.hovered = None,
            Event::WindowInteraction { window } => seat.interacted = Some(window.id()),
            Event::PointerPosition { x, y } => {
                if seat.pointer != (x, y) {
                    seat.moved = true;
                }
                seat.pointer = (x, y);
            }
            Event::OpDelta { dx, dy } => {
                if let Some(op) = &mut seat.op {
                    op.delta = (dx, dy);
                }
            }
            Event::OpRelease => seat.op_released = true,
            _ => {}
        }
    }
}

impl Dispatch<RiverXkbBindingV1, usize> for Wm {
    fn event(
        wm: &mut Self,
        _: &RiverXkbBindingV1,
        event: river_xkb_binding_v1::Event,
        key: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let river_xkb_binding_v1::Event::Pressed = event {
            wm.actions.push(Action::Key(*key));
        }
    }
}

impl Dispatch<RiverPointerBindingV1, usize> for Wm {
    fn event(
        wm: &mut Self,
        _: &RiverPointerBindingV1,
        event: river_pointer_binding_v1::Event,
        button: &usize,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let river_pointer_binding_v1::Event::Pressed = event {
            wm.actions.push(Action::Button(*button));
        }
    }
}

wayland_client::delegate_noop!(Wm: ignore RiverNodeV1);
wayland_client::delegate_noop!(Wm: ignore RiverXkbBindingsV1);
wayland_client::delegate_noop!(Wm: ignore RiverLayerShellV1);
