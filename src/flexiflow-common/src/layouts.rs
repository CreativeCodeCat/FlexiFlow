//! flexiflow's vanitygaps-derived layout algorithms, carried over from
//! the X11 FlexiFlow unchanged. Each pure `fn(..) -> Vec<R>` computes
//! where the `n` tiled windows go inside the area `r`; `arrange` picks
//! one by name. Unlike the X11 version, the rects include each window's
//! border: the window manager insets them by the border width itself.
use std::ffi::{c_int, c_uint};

/// (x, y, w, h)
pub type R = (c_int, c_int, c_int, c_int);

/// Per-tag/per-output gap configuration, matching flexiflow's vanitygaps
/// behavior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaps {
    /// Outer horizontal gap (top/bottom screen edge)
    pub oh: c_int,
    /// Outer vertical gap (left/right screen edge)
    pub ov: c_int,
    /// Inner horizontal gap (between stacked windows)
    pub ih: c_int,
    /// Inner vertical gap (between master/stack columns)
    pub iv: c_int,
    pub enabled: bool,
    /// Collapse outer gaps to 0 when there's a single window
    pub smartgaps: bool,
}

impl Gaps {
    pub fn uniform(gap: c_int, smartgaps: bool) -> Self {
        Self { oh: gap, ov: gap, ih: gap, iv: gap, enabled: true, smartgaps }
    }

    /// Effective gaps for `n` windows.
    pub fn effective(&self, n: usize) -> (c_int, c_int, c_int, c_int) {
        let outer_enabled = !(self.smartgaps && n == 1) && self.enabled;
        let inner_enabled = self.enabled;
        (
            self.oh * outer_enabled as c_int,
            self.ov * outer_enabled as c_int,
            self.ih * inner_enabled as c_int,
            self.iv * inner_enabled as c_int,
        )
    }
}

/// The names `layouts[i].arrange` can take in config.lua, each with the
/// description the layout menu shows next to its symbol.
pub const ARRANGE: [(&str, &str); 13] = [
    ("tile", "Tiled Layout"),
    ("monocle", "Monocle Layout"),
    ("spiral", "Spiral Layout"),
    ("dwindle", "Dwindle Layout"),
    ("deck", "Deck Layout"),
    ("bstack", "Stack Layout"),
    ("bstack_horiz", "Stack Horz Layout"),
    ("grid", "Grid Layout"),
    ("nrow_grid", "Narrow Grid Layout"),
    ("horiz_grid", "Grid Horz Layout"),
    ("gapless_grid", "Gapless Grid Layout"),
    ("centered_master", "Centered Master Layout"),
    ("centered_floating_master", "Centered Floating Layout"),
];

/// Where `arrange` (one of [`ARRANGE`]'s names) puts `cfacts.len()`
/// tiled windows inside `r`.
pub fn arrange(
    arrange: &str,
    r: R,
    nmaster: c_uint,
    mfact: f32,
    gaps: Gaps,
    cfacts: &[f32],
) -> Vec<R> {
    let n = cfacts.len();
    if n == 0 {
        return vec![];
    }
    match arrange {
        "tile" => tile_(r, n, nmaster, mfact, gaps, cfacts),
        "monocle" => monocle_(r, n, gaps),
        "spiral" => fibonacci_(r, n, gaps, false),
        "dwindle" => fibonacci_(r, n, gaps, true),
        "deck" => deck_(r, n, nmaster, mfact, gaps, cfacts),
        "bstack" => bstack_(r, n, nmaster, mfact, gaps, cfacts),
        "bstack_horiz" => bstack_horiz_(r, n, nmaster, mfact, gaps, cfacts),
        "grid" => grid_(r, n, gaps),
        "nrow_grid" => nrow_grid_(r, n, nmaster, gaps),
        "horiz_grid" => horiz_grid_(r, n, gaps, cfacts),
        "gapless_grid" => gapless_grid_(r, n, gaps),
        "centered_master" => centered_master_(r, n, nmaster, mfact, gaps, cfacts),
        "centered_floating_master" => {
            centered_floating_master_(r, n, nmaster, mfact, gaps, cfacts)
        }
        _ => tile_(r, n, nmaster, mfact, gaps, cfacts),
    }
}

/// Distribute `total` pixels across `weights.len()` slots proportionally to
/// `weights` (a client's `cfact`), dumping any rounding remainder on the
/// last slot so the sum always matches `total` exactly.
pub fn weighted_split(total: c_int, weights: &[f32]) -> Vec<c_int> {
    let n = weights.len();
    if n == 0 {
        return vec![];
    }
    let sum: f32 = weights.iter().sum();
    if sum <= 0.0 {
        let base = total / n as c_int;
        let mut v = vec![base; n];
        if let Some(last) = v.last_mut() {
            *last += total - base * n as c_int;
        }
        return v;
    }
    let mut sizes: Vec<c_int> = weights
        .iter()
        .map(|w| ((w / sum) * total as f32).round() as c_int)
        .collect();
    let diff = total - sizes.iter().sum::<c_int>();
    if let Some(last) = sizes.last_mut() {
        *last += diff;
    }
    sizes
}

pub fn rect(x: c_int, y: c_int, w: c_int, h: c_int) -> R {
    (x, y, w.max(0), h.max(0))
}

// ---------------------------------------------------------------- tile ----

pub fn tile_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);

    let mx0 = rx + ov;
    let my0 = ry + oh;
    let mut mw = rw - 2 * ov;
    let mut sw = mw;
    let mh = rh - 2 * oh - ih * (nm.saturating_sub(1)) as c_int;
    let sh = rh - 2 * oh - ih * ((n - nm).saturating_sub(1)) as c_int;
    let mut sx0 = mx0;

    if nm > 0 && n > nm {
        sw = ((mw - iv) as f32 * (1.0 - mfact)) as c_int;
        mw = mw - iv - sw;
        sx0 = mx0 + mw + iv;
    }

    let m_h = weighted_split(mh, &cfacts[..nm]);
    let s_h = weighted_split(sh, &cfacts[nm..]);

    let mut out = Vec::with_capacity(n);
    let (mut my, mut sy) = (my0, my0);
    for i in 0..n {
        if i < nm {
            let h = m_h[i];
            out.push(rect(mx0, my, mw, h));
            my += h + ih;
        } else {
            let h = s_h[i - nm];
            out.push(rect(sx0, sy, sw, h));
            sy += h + ih;
        }
    }
    out
}

// -------------------------------------------------------------- bstack ----

pub fn bstack_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);

    let mx0 = rx + ov;
    let my0 = ry + oh;
    let mut mh = rh - 2 * oh;
    let mut sh = mh;
    let mw = rw - 2 * ov - iv * (nm.saturating_sub(1)) as c_int;
    let sw = rw - 2 * ov - iv * ((n - nm).saturating_sub(1)) as c_int;
    let sx0 = mx0;
    let mut sy0 = my0;

    if nm > 0 && n > nm {
        sh = ((mh - ih) as f32 * (1.0 - mfact)) as c_int;
        mh = mh - ih - sh;
        sy0 = my0 + mh + ih;
    }

    let m_w = weighted_split(mw, &cfacts[..nm]);
    let s_w = weighted_split(sw, &cfacts[nm..]);

    let mut out = Vec::with_capacity(n);
    let (mut mx, mut sx) = (mx0, sx0);
    for i in 0..n {
        if i < nm {
            let w = m_w[i];
            out.push(rect(mx, my0, w, mh));
            mx += w + iv;
        } else {
            let w = s_w[i - nm];
            out.push(rect(sx, sy0, w, sh));
            sx += w + iv;
        }
    }
    out
}

pub fn bstack_horiz_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);

    let mx0 = rx + ov;
    let my0 = ry + oh;
    let mut mh = rh - 2 * oh;
    let mut sh = rh - 2 * oh - ih * ((n - nm).saturating_sub(1)) as c_int;
    let mw = rw - 2 * ov - iv * (nm.saturating_sub(1)) as c_int;
    let sw = rw - 2 * ov;
    let mut sy0 = my0;

    if nm > 0 && n > nm {
        sh = ((mh - ih) as f32 * (1.0 - mfact)) as c_int;
        mh = mh - ih - sh;
        sy0 = my0 + mh + ih;
        sh = rh - mh - 2 * oh - ih * (n - nm) as c_int;
    }

    let m_w = weighted_split(mw, &cfacts[..nm]);
    let s_h = weighted_split(sh, &cfacts[nm..]);

    let mut out = Vec::with_capacity(n);
    let mut mx = mx0;
    let mut sy = sy0;
    for i in 0..n {
        if i < nm {
            let w = m_w[i];
            out.push(rect(mx, my0, w, mh));
            mx += w + iv;
        } else {
            let h = s_h[i - nm];
            out.push(rect(mx0, sy, sw, h));
            sy += h + ih;
        }
    }
    out
}

// -------------------------------------------------------- centeredmaster ----

pub fn centered_master_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = nmaster as usize;
    // mirrors `!m->nmaster || n < m->nmaster` from the C source: nmaster==0
    // means "everyone is master"
    let n_master = if nm == 0 { n } else { nm.min(n) };
    let n_stack = n.saturating_sub(n_master);

    let mut mx = rx + ov;
    let my0 = ry + oh;
    let mh = rh - 2 * oh - ih * (n_master.saturating_sub(1)) as c_int;
    let mut mw = rw - 2 * ov;
    let lh = rh - 2 * oh - ih * ((n_stack / 2).saturating_sub(1)) as c_int;
    let rh_ = rh - 2 * oh - ih * ((n_stack / 2) as c_int - if n_stack % 2 != 0 { 0 } else { 1 });

    let lx = rx + ov;
    let (mut ly, mut rx2, mut ry2, mut lw, mut rsw) = (my0, 0, my0, 0, 0);

    if nm > 0 && n > nm {
        if n_stack > 1 {
            mw = ((rw - 2 * ov - 2 * iv) as f32 * mfact) as c_int;
            lw = (rw - mw - 2 * ov - 2 * iv) / 2;
            rsw = (rw - mw - 2 * ov - 2 * iv) - lw;
            mx += lw + iv;
        } else {
            mw = ((mw - iv) as f32 * mfact) as c_int;
            lw = 0;
            rsw = rw - mw - iv - 2 * ov;
        }
        rx2 = mx + mw + iv;
        ry2 = my0;
    }

    let master_cfacts: Vec<f32> = cfacts[..n_master].to_vec();
    let mut left_cfacts = vec![];
    let mut right_cfacts = vec![];
    for (i, f) in cfacts[n_master..].iter().enumerate() {
        if i % 2 != 0 {
            left_cfacts.push(*f);
        } else {
            right_cfacts.push(*f);
        }
    }

    let m_h = weighted_split(mh, &master_cfacts);
    let l_h = weighted_split(lh, &left_cfacts);
    let r_h = weighted_split(rh_, &right_cfacts);

    let mut out = Vec::with_capacity(n);
    let mut my = my0;
    for &h in &m_h {
        out.push(rect(mx, my, mw, h));
        my += h + ih;
    }
    let (mut li, mut ri) = (0usize, 0usize);
    for i in n_master..n {
        if (i - n_master) % 2 != 0 {
            let h = l_h[li];
            out.push(rect(lx, ly, lw, h));
            ly += h + ih;
            li += 1;
        } else {
            let h = r_h[ri];
            out.push(rect(rx2, ry2, rsw, h));
            ry2 += h + ih;
            ri += 1;
        }
    }
    out
}

pub fn centered_floating_master_(
    r: R,
    n: usize,
    nmaster: c_uint,
    mfact: f32,
    gaps: Gaps,
    cfacts: &[f32],
) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, _ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);
    let mut mivf: f32 = 1.0;

    let mut mx = rx + ov;
    let mut my = ry + oh;
    let mut mh = rh - 2 * oh;
    let mut mw = rw - 2 * ov - iv * (n.saturating_sub(1)) as c_int;
    let mut sx = mx;
    let mut sy = my;
    let mut sh = mh;
    let sw = rw - 2 * ov - iv * ((n - nm).saturating_sub(1)) as c_int;

    if nm > 0 && n > nm {
        mivf = 0.8;
        if rw > rh {
            mw = (rw as f32 * mfact) as c_int - (iv as f32 * mivf * (nm.saturating_sub(1)) as f32) as c_int;
            mh = (rh as f32 * 0.9) as c_int;
        } else {
            mw = (rw as f32 * 0.9) as c_int - (iv as f32 * mivf * (nm.saturating_sub(1)) as f32) as c_int;
            mh = (rh as f32 * mfact) as c_int;
        }
        mx = rx + (rw - mw) / 2;
        my = ry + (rh - mh - 2 * oh) / 2;
        sx = rx + ov;
        sy = ry + oh;
        sh = rh - 2 * oh;
    }

    let m_w = weighted_split(mw, &cfacts[..nm]);
    let s_w = weighted_split(sw, &cfacts[nm..]);

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i < nm {
            let w = m_w[i];
            out.push(rect(mx, my, w, mh));
            mx += w + (iv as f32 * mivf) as c_int;
        } else {
            let w = s_w[i - nm];
            out.push(rect(sx, sy, w, sh));
            sx += w + iv;
        }
    }
    out
}

// ---------------------------------------------------------------- deck ----

pub fn deck_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);

    let mx = rx + ov;
    let my0 = ry + oh;
    let mh = rh - 2 * oh - ih * (nm.saturating_sub(1)) as c_int;
    let mut mw = rw - 2 * ov;
    let mut sx = mx;
    let sy = my0;
    let mut sh = mh;
    let sw;

    if nm > 0 && n > nm {
        let stack_w = ((mw - iv) as f32 * (1.0 - mfact)) as c_int;
        sw = stack_w;
        mw = mw - iv - sw;
        sx = mx + mw + iv;
        sh = rh - 2 * oh;
    } else {
        sw = mw;
    }

    let m_h = weighted_split(mh, &cfacts[..nm]);

    let mut out = Vec::with_capacity(n);
    let mut my = my0;
    for &h in &m_h {
        out.push(rect(mx, my, mw, h));
        my += h + ih;
    }
    // deck: every stacked client gets the exact same rect (cards in a
    // deck)
    for _ in nm..n {
        out.push(rect(sx, sy, sw, sh));
    }
    out
}

// ------------------------------------------------------------ fibonacci ----

/// `spiral` (s=false) / `dwindle` (s=true) - direct port of `fibonacci()`.
pub fn fibonacci_(r: R, n: usize, gaps: Gaps, s: bool) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);

    let mut nx = rx + ov;
    let mut ny = ry + oh;
    let mut nw = rw - 2 * ov;
    let mut nh = rh - 2 * oh;

    let mut hrest = 0;
    let mut wrest = 0;
    let mut recurse = true;
    let mut out = Vec::with_capacity(n);
    let mut i: usize = 0;

    for _ in 0..n {
        if recurse {
            if (i % 2 == 1 && (nh - ih) / 2 <= 0)
                || (i.is_multiple_of(2) && (nw - iv) / 2 <= 0)
            {
                recurse = false;
            }
            if recurse && i < n - 1 {
                if i % 2 == 1 {
                    let nv = (nh - ih) / 2;
                    hrest = nh - 2 * nv - ih;
                    nh = nv;
                } else {
                    let nv = (nw - iv) / 2;
                    wrest = nw - 2 * nv - iv;
                    nw = nv;
                }
                if i % 4 == 2 && !s {
                    nx += nw + iv;
                } else if i % 4 == 3 && !s {
                    ny += nh + ih;
                }
            }

            match i % 4 {
                0 => {
                    if s {
                        ny += nh + ih;
                        nh += hrest;
                    } else {
                        nh -= hrest;
                        ny -= nh + ih;
                    }
                }
                1 => {
                    nx += nw + iv;
                    nw += wrest;
                }
                2 => {
                    ny += nh + ih;
                    nh += hrest;
                    if i < n - 1 {
                        nw += wrest;
                    }
                }
                3 => {
                    if s {
                        nx += nw + iv;
                        nw -= wrest;
                    } else {
                        nw -= wrest;
                        nx -= nw + iv;
                        nh += hrest;
                    }
                }
                _ => unreachable!(),
            }

            if i == 0 {
                if n != 1 {
                    nw = (rw - iv - 2 * ov) - ((rw - iv - 2 * ov) as f32 * 0.5) as c_int;
                    wrest = 0;
                }
                ny = ry + oh;
            } else if i == 1 {
                nw = rw - nw - iv - 2 * ov;
            }
            i += 1;
        }

        out.push(rect(nx, ny, nw, nh));
    }
    out
}

// -------------------------------------------------------------- grids ----

pub fn gapless_grid_(r: R, n: usize, gaps: Gaps) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);

    let mut cols = 0usize;
    while cols <= n / 2 {
        if cols * cols >= n {
            break;
        }
        cols += 1;
    }
    let cols = if n == 5 { 2 } else { cols.max(1) };
    let mut rows = n / cols;

    let mut ch = (rh - 2 * oh - ih * (rows as c_int - 1)) / rows.max(1) as c_int;
    let mut rrest = (rh - 2 * oh - ih * (rows as c_int - 1)) - ch * rows as c_int;
    let cw = (rw - 2 * ov - iv * (cols as c_int - 1)) / cols as c_int;
    let crest = (rw - 2 * ov - iv * (cols as c_int - 1)) - cw * cols as c_int;

    let mut x = rx + ov;
    let mut y;
    let (mut cn, mut rn) = (0i32, 0i32);

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i / rows + 1 > cols - n % cols {
            rows = n / cols + 1;
            ch = (rh - 2 * oh - ih * (rows as c_int - 1)) / rows as c_int;
            rrest = (rh - 2 * oh - ih * (rows as c_int - 1)) - ch * rows as c_int;
        }
        y = ry + oh + rn * (ch + ih) + rn.min(rrest);
        let w = cw + if (cn as usize) < crest.max(0) as usize { 1 } else { 0 };
        let h = ch + if (rn as usize) < rrest.max(0) as usize { 1 } else { 0 };
        out.push(rect(x, y, w, h));

        rn += 1;
        if rn as usize >= rows {
            rn = 0;
            x += cw + ih + if (cn as usize) < crest.max(0) as usize { 1 } else { 0 };
            cn += 1;
        }
    }
    out
}

pub fn grid_(r: R, n: usize, gaps: Gaps) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);

    let mut rows = 0usize;
    while rows <= n / 2 {
        if rows * rows >= n {
            break;
        }
        rows += 1;
    }
    let cols = if rows > 0 && (rows - 1) * rows >= n {
        rows - 1
    } else {
        rows
    };

    let ch = (rh - 2 * oh - ih * (rows as c_int - 1)) / rows.max(1) as c_int;
    let cw = (rw - 2 * ov - iv * (cols as c_int - 1)) / cols.max(1) as c_int;
    let chrest = (rh - 2 * oh - ih * (rows as c_int - 1)) - ch * rows as c_int;
    let cwrest = (rw - 2 * ov - iv * (cols as c_int - 1)) - cw * cols as c_int;

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let cc = i / rows.max(1);
        let cr = i % rows.max(1);
        let x = rx + ov + cc as c_int * (cw + iv) + (cc as c_int).min(cwrest);
        let y = ry + oh + cr as c_int * (ch + ih) + (cr as c_int).min(chrest);
        let w = cw + if cc < cwrest.max(0) as usize { 1 } else { 0 };
        let h = ch + if cr < chrest.max(0) as usize { 1 } else { 0 };
        out.push(rect(x, y, w, h));
    }
    out
}

pub fn horiz_grid_(r: R, n: usize, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);

    let ntop = if n <= 2 { n } else { n / 2 };

    let mx0 = rx + ov;
    let my0 = ry + oh;
    let mut mh = rh - 2 * oh;
    let mut sh = mh;
    let mut mw = rw - 2 * ov;
    let mut sw = mw;
    let mut sy0 = my0;

    if n > ntop {
        sh = (mh - ih) / 2;
        mh = mh - ih - sh;
        sy0 = my0 + mh + ih;
        mw = rw - 2 * ov - iv * (ntop as c_int - 1);
        sw = rw - 2 * ov - iv * ((n - ntop) as c_int - 1);
    }

    let m_w = weighted_split(mw, &cfacts[..ntop]);
    let s_w = weighted_split(sw, &cfacts[ntop..]);

    let mut out = Vec::with_capacity(n);
    let (mut mx, mut sx) = (mx0, mx0);
    for i in 0..n {
        if i < ntop {
            let w = m_w[i];
            out.push(rect(mx, my0, w, mh));
            mx += w + iv;
        } else {
            let w = s_w[i - ntop];
            out.push(rect(sx, sy0, w, sh));
            sx += w + iv;
        }
    }
    out
}

pub fn nrow_grid_(r: R, n: usize, nmaster: c_uint, gaps: Gaps) -> Vec<R> {
    let (rx, ry, rw, rh) = r;
    let (oh, ov, ih, iv) = gaps.effective(n);

    let mut rows = nmaster as usize + 1;
    if n == 2 {
        rows = 1; // FORCE_VSPLIT: always split two clients side by side
    }
    if n < rows {
        rows = n;
    }
    let rows = rows.max(1);

    // Row height is fixed for the whole layout (nrowgrid does not
    // redistribute any rounding remainder between rows), only the column
    // count per row and the running "used width" within a row change as we
    // go.
    let mut cols = n / rows;
    let mut uc = cols;
    let mut cy = ry + oh;
    let ch = (rh - 2 * oh - ih * (rows as c_int - 1)) / rows as c_int;
    let mut uh = ch;

    let mut ri = 0usize;
    let mut ci = 0usize;
    let mut uw = 0i32;

    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if ci == cols {
            uw = 0;
            ci = 0;
            ri += 1;
            cols = (n - uc) / (rows - ri).max(1);
            uc += cols;
            cy = ry + oh + uh + ih;
            uh += ch + ih;
        }

        let cx = rx + ov + uw;
        let cw = (rw - 2 * ov - uw) / (cols - ci).max(1) as c_int;
        uw += cw + iv;

        out.push(rect(cx, cy, cw, ch));
        ci += 1;
    }
    out
}

// ------------------------------------------------------------- monocle ----

pub fn monocle_(r: R, n: usize, gaps: Gaps) -> Vec<R> {
    // dwm's monocle() always uses the raw outer gap values directly and
    // never calls getgaps(), so `enabled`/`smartgaps` intentionally have no
    // effect here.
    let (rx, ry, rw, rh) = r;
    let (oh, ov) = (gaps.oh, gaps.ov);
    let x = rx + ov;
    let y = ry + oh;
    let w = rw - 2 * ov;
    let h = rh - 2 * oh;

    vec![rect(x, y, w, h); n]
}


#[cfg(test)]
mod tests {
    use super::*;

    const NO_GAPS: Gaps = Gaps {
        oh: 0,
        ov: 0,
        ih: 0,
        iv: 0,
        enabled: false,
        smartgaps: false,
    };

    /// Regression test for a variable-shadowing bug: `centered_master_`
    /// used to declare a second, zero-initialized `rw` that shadowed the
    /// monitor-width `rw` from the function's top, so `mw`'s mfact
    /// calculation silently read 0 instead of the real width whenever
    /// there was more than one stack client - collapsing the master and
    /// right columns to zero width.
    #[test]
    fn centered_master_splits_width_across_all_three_columns() {
        let r: R = (0, 0, 1200, 800);
        let cfacts = [1.0, 1.0, 1.0];
        let rects = centered_master_(r, 3, 1, 0.5, NO_GAPS, &cfacts);

        assert_eq!(rects.len(), 3);
        for &(x, y, w, h) in &rects {
            assert!(w > 0, "zero/negative width rect: {:?}", (x, y, w, h));
            assert!(h > 0, "zero/negative height rect: {:?}", (x, y, w, h));
        }

        // left stack, master, right stack, left to right, none overlapping
        let mut by_x = rects.clone();
        by_x.sort_by_key(|&(x, ..)| x);
        let (lx, _, lw, _) = by_x[0];
        let (mx, _, mw, _) = by_x[1];
        let (rx, _, rw, _) = by_x[2];
        assert_eq!(lx, 0);
        assert_eq!(lx + lw, mx);
        assert_eq!(mx + mw, rx);
        assert_eq!(rx + rw, 1200);
    }
}
