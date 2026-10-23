//! Shared, globally-mutable window-manager state.
//!
//! flexiflow (like vanilla dwm + the vanitygaps/pertag-less patch set it carries)
//! keeps gaps, `mfact` and `nmaster` as fields on the `Monitor` struct: they are
//! shared by every tag and every layout on that monitor, and `cfact` lives on each
//! `Client` so it follows a window between layouts. Penrose layouts are normally
//! self-contained structs with their own private fields, so to get the same
//! "global knob, read by every layout" behaviour we share one `Arc<Mutex<..>>`
//! across every layout instance we build.
use penrose::WinId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy)]
pub struct Gaps {
    pub oh: i32,
    pub ov: i32,
    pub ih: i32,
    pub iv: i32,
    pub enabled: bool,
    pub smartgaps: bool,
}

impl Gaps {
    const DEFAULT: i32 = 10;

    fn default_gaps() -> Self {
        Self {
            oh: Self::DEFAULT,
            ov: Self::DEFAULT,
            ih: Self::DEFAULT,
            iv: Self::DEFAULT,
            enabled: true,
            smartgaps: false,
        }
    }

    /// Effective gaps for `n` clients, mirroring `getgaps` in vanitygaps.c
    /// (outer gaps collapse to 0 when `smartgaps` is set and there's a single client).
    pub fn effective(&self, n: usize) -> (i32, i32, i32, i32) {
        let outer_enabled = !(self.smartgaps && n == 1) && self.enabled;
        let inner_enabled = self.enabled;
        (
            self.oh * outer_enabled as i32,
            self.ov * outer_enabled as i32,
            self.ih * inner_enabled as i32,
            self.iv * inner_enabled as i32,
        )
    }
}

#[derive(Debug, Clone)]
pub struct SharedState {
    gaps: Arc<Mutex<Gaps>>,
    mfact: Arc<Mutex<f32>>,
    nmaster: Arc<Mutex<u32>>,
    cfacts: Arc<Mutex<HashMap<WinId, f32>>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            gaps: Arc::new(Mutex::new(Gaps::default_gaps())),
            mfact: Arc::new(Mutex::new(0.50)),
            nmaster: Arc::new(Mutex::new(1)),
            cfacts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn gaps(&self) -> Gaps {
        *self.gaps.lock().unwrap()
    }

    pub fn mfact(&self) -> f32 {
        *self.mfact.lock().unwrap()
    }

    pub fn nmaster(&self) -> u32 {
        *self.nmaster.lock().unwrap()
    }

    pub fn cfact(&self, id: WinId) -> f32 {
        self.cfacts.lock().unwrap().get(&id).copied().unwrap_or(1.0)
    }

    pub fn cfacts_for(&self, ids: &[WinId]) -> Vec<f32> {
        ids.iter().map(|id| self.cfact(*id)).collect()
    }

    // -- mutators, all mirroring the Arg-taking functions in vanitygaps.c --

    pub fn set_gaps(&self, oh: i32, ov: i32, ih: i32, iv: i32) {
        let mut g = self.gaps.lock().unwrap();
        g.oh = oh.max(0);
        g.ov = ov.max(0);
        g.ih = ih.max(0);
        g.iv = iv.max(0);
    }

    pub fn incr_gaps(&self, d: i32) {
        let g = self.gaps();
        self.set_gaps(g.oh + d, g.ov + d, g.ih + d, g.iv + d);
    }

    pub fn incr_inner_gaps(&self, d: i32) {
        let g = self.gaps();
        self.set_gaps(g.oh, g.ov, g.ih + d, g.iv + d);
    }

    pub fn incr_outer_gaps(&self, d: i32) {
        let g = self.gaps();
        self.set_gaps(g.oh + d, g.ov + d, g.ih, g.iv);
    }

    pub fn incr_outer_h_gaps(&self, d: i32) {
        let g = self.gaps();
        self.set_gaps(g.oh + d, g.ov, g.ih, g.iv);
    }

    pub fn incr_outer_v_gaps(&self, d: i32) {
        let g = self.gaps();
        self.set_gaps(g.oh, g.ov + d, g.ih, g.iv);
    }

    pub fn incr_inner_h_gaps(&self, d: i32) {
        let g = self.gaps();
        self.set_gaps(g.oh, g.ov, g.ih + d, g.iv);
    }

    pub fn incr_inner_v_gaps(&self, d: i32) {
        let g = self.gaps();
        self.set_gaps(g.oh, g.ov, g.ih, g.iv + d);
    }

    pub fn default_gaps(&self) {
        self.set_gaps(Gaps::DEFAULT, Gaps::DEFAULT, Gaps::DEFAULT, Gaps::DEFAULT);
    }

    pub fn toggle_gaps(&self) {
        let mut g = self.gaps.lock().unwrap();
        g.enabled = !g.enabled;
    }

    pub fn set_mfact(&self, d: f32) {
        let mut m = self.mfact.lock().unwrap();
        *m = (*m + d).clamp(0.05, 0.95);
    }

    pub fn incr_nmaster(&self, d: i32) {
        let mut n = self.nmaster.lock().unwrap();
        *n = (*n as i32 + d).max(0) as u32;
    }

    pub fn set_cfact(&self, id: WinId, d: f32) {
        let mut facts = self.cfacts.lock().unwrap();
        if d == 0.0 {
            facts.remove(&id);
            return;
        }
        let f = facts.entry(id).or_insert(1.0);
        let next = *f + d;
        if next <= 0.25 {
            return;
        }
        *f = next;
    }
}
