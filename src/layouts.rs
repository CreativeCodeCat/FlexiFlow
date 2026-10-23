//! Direct ports of the layout algorithms in flexiflow's `vanitygaps.c` (dwm's
//! vanitygaps patch) as penrose [Layout] implementations.
//!
//! Every function here takes a work-area `Rect` (already shrunk to leave room
//! for the status bar) and returns one placement per client, following the
//! exact master/stack math from the C source so gap, `mfact`, `nmaster` and
//! `cfact` behaviour matches the original. Border width is *not* subtracted
//! here (unlike the C code) because penrose insets for the window border
//! itself when mapping clients.
use crate::state::{Gaps, SharedState};
use penrose::{
    WinId,
    core::layout::{Layout, Message},
    pure::{Stack, geometry::Rect},
};

/// Distribute `total` pixels across `weights.len()` slots proportionally to
/// `weights` (a client's `cfact`), dumping any rounding remainder on the last
/// slot so the sum always matches `total` exactly.
fn weighted_split(total: i32, weights: &[f32]) -> Vec<i32> {
    let n = weights.len();
    if n == 0 {
        return vec![];
    }
    let sum: f32 = weights.iter().sum();
    if sum <= 0.0 {
        let base = total / n as i32;
        let mut v = vec![base; n];
        if let Some(last) = v.last_mut() {
            *last += total - base * n as i32;
        }
        return v;
    }
    let mut sizes: Vec<i32> = weights
        .iter()
        .map(|w| ((w / sum) * total as f32).round() as i32)
        .collect();
    let diff = total - sizes.iter().sum::<i32>();
    if let Some(last) = sizes.last_mut() {
        *last += diff;
    }
    sizes
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::new(x, y, w.max(0) as u32, h.max(0) as u32)
}

// ---------------------------------------------------------------- tile ----

fn tile(r: Rect, ids: &[WinId], nmaster: u32, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<(WinId, Rect)> {
    let n = ids.len();
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);

    let mx0 = r.x + ov;
    let my0 = r.y + oh;
    let mut mw = r.w as i32 - 2 * ov;
    let mut sw = mw;
    let mh = r.h as i32 - 2 * oh - ih * (nm.saturating_sub(1)) as i32;
    let sh = r.h as i32 - 2 * oh - ih * ((n - nm).saturating_sub(1)) as i32;
    let mut sx0 = mx0;

    if nm > 0 && n > nm {
        sw = ((mw - iv) as f32 * (1.0 - mfact)) as i32;
        mw = mw - iv - sw;
        sx0 = mx0 + mw + iv;
    }

    let m_h = weighted_split(mh, &cfacts[..nm]);
    let s_h = weighted_split(sh, &cfacts[nm..]);

    let mut out = Vec::with_capacity(n);
    let (mut my, mut sy) = (my0, my0);
    for (i, id) in ids.iter().enumerate() {
        if i < nm {
            let h = m_h[i];
            out.push((*id, rect(mx0, my, mw, h)));
            my += h + ih;
        } else {
            let h = s_h[i - nm];
            out.push((*id, rect(sx0, sy, sw, h)));
            sy += h + ih;
        }
    }
    out
}

// -------------------------------------------------------------- bstack ----

fn bstack(r: Rect, ids: &[WinId], nmaster: u32, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<(WinId, Rect)> {
    let n = ids.len();
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);

    let mx0 = r.x + ov;
    let my0 = r.y + oh;
    let mut mh = r.h as i32 - 2 * oh;
    let mut sh = mh;
    let mw = r.w as i32 - 2 * ov - iv * (nm.saturating_sub(1)) as i32;
    let sw = r.w as i32 - 2 * ov - iv * ((n - nm).saturating_sub(1)) as i32;
    let sx0 = mx0;
    let mut sy0 = my0;

    if nm > 0 && n > nm {
        sh = ((mh - ih) as f32 * (1.0 - mfact)) as i32;
        mh = mh - ih - sh;
        sy0 = my0 + mh + ih;
    }

    let m_w = weighted_split(mw, &cfacts[..nm]);
    let s_w = weighted_split(sw, &cfacts[nm..]);

    let mut out = Vec::with_capacity(n);
    let (mut mx, mut sx) = (mx0, sx0);
    for (i, id) in ids.iter().enumerate() {
        if i < nm {
            let w = m_w[i];
            out.push((*id, rect(mx, my0, w, mh)));
            mx += w + iv;
        } else {
            let w = s_w[i - nm];
            out.push((*id, rect(sx, sy0, w, sh)));
            sx += w + iv;
        }
    }
    out
}

fn bstack_horiz(r: Rect, ids: &[WinId], nmaster: u32, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<(WinId, Rect)> {
    let n = ids.len();
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);

    let mx0 = r.x + ov;
    let my0 = r.y + oh;
    let mut mh = r.h as i32 - 2 * oh;
    let mut sh = r.h as i32 - 2 * oh - ih * ((n - nm).saturating_sub(1)) as i32;
    let mw = r.w as i32 - 2 * ov - iv * (nm.saturating_sub(1)) as i32;
    let sw = r.w as i32 - 2 * ov;
    let mut sy0 = my0;

    if nm > 0 && n > nm {
        sh = ((mh - ih) as f32 * (1.0 - mfact)) as i32;
        mh = mh - ih - sh;
        sy0 = my0 + mh + ih;
        sh = r.h as i32 - mh - 2 * oh - ih * (n - nm) as i32;
    }

    let m_w = weighted_split(mw, &cfacts[..nm]);
    let s_h = weighted_split(sh, &cfacts[nm..]);

    let mut out = Vec::with_capacity(n);
    let mut mx = mx0;
    let mut sy = sy0;
    for (i, id) in ids.iter().enumerate() {
        if i < nm {
            let w = m_w[i];
            out.push((*id, rect(mx, my0, w, mh)));
            mx += w + iv;
        } else {
            let h = s_h[i - nm];
            out.push((*id, rect(mx0, sy, sw, h)));
            sy += h + ih;
        }
    }
    out
}

// -------------------------------------------------------- centeredmaster ----

fn centered_master(r: Rect, ids: &[WinId], nmaster: u32, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<(WinId, Rect)> {
    let n = ids.len();
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = nmaster as usize;
    // mirrors `!m->nmaster || n < m->nmaster` from the C source: nmaster==0 means "everyone is master"
    let n_master = if nm == 0 { n } else { nm.min(n) };
    let n_stack = n.saturating_sub(n_master);

    let mut mx = r.x + ov;
    let my0 = r.y + oh;
    let mh = r.h as i32 - 2 * oh - ih * (n_master.saturating_sub(1)) as i32;
    let mut mw = r.w as i32 - 2 * ov;
    let lh = r.h as i32 - 2 * oh - ih * ((n_stack / 2).saturating_sub(1)) as i32;
    let rh = r.h as i32 - 2 * oh - ih * ((n_stack / 2) as i32 - if n_stack % 2 != 0 { 0 } else { 1 });

    let lx = r.x + ov;
    let (mut ly, mut rx, mut ry, mut lw, mut rw) = (my0, 0, my0, 0, 0);

    if nm > 0 && n > nm {
        if n_stack > 1 {
            mw = ((r.w as i32 - 2 * ov - 2 * iv) as f32 * mfact) as i32;
            lw = (r.w as i32 - mw - 2 * ov - 2 * iv) / 2;
            rw = (r.w as i32 - mw - 2 * ov - 2 * iv) - lw;
            mx += lw + iv;
        } else {
            mw = ((mw - iv) as f32 * mfact) as i32;
            lw = 0;
            rw = r.w as i32 - mw - iv - 2 * ov;
        }
        rx = mx + mw + iv;
        ry = my0;
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
    let r_h = weighted_split(rh, &right_cfacts);

    let mut out = Vec::with_capacity(n);
    let mut my = my0;
    let (mut li, mut ri) = (0usize, 0usize);
    for (i, id) in ids.iter().enumerate() {
        if i < n_master {
            let h = m_h[i];
            out.push((*id, rect(mx, my, mw, h)));
            my += h + ih;
        } else if (i - n_master) % 2 != 0 {
            let h = l_h[li];
            out.push((*id, rect(lx, ly, lw, h)));
            ly += h + ih;
            li += 1;
        } else {
            let h = r_h[ri];
            out.push((*id, rect(rx, ry, rw, h)));
            ry += h + ih;
            ri += 1;
        }
    }
    out
}

fn centered_floating_master(
    r: Rect,
    ids: &[WinId],
    nmaster: u32,
    mfact: f32,
    gaps: Gaps,
    cfacts: &[f32],
) -> Vec<(WinId, Rect)> {
    let n = ids.len();
    let (oh, ov, _ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);
    let mut mivf: f32 = 1.0;

    let mut mx = r.x + ov;
    let mut my = r.y + oh;
    let mut mh = r.h as i32 - 2 * oh;
    let mut mw = r.w as i32 - 2 * ov - iv * (n.saturating_sub(1)) as i32;
    let mut sx = mx;
    let mut sy = my;
    let mut sh = mh;
    let sw = r.w as i32 - 2 * ov - iv * ((n - nm).saturating_sub(1)) as i32;

    if nm > 0 && n > nm {
        mivf = 0.8;
        if r.w > r.h {
            mw = (r.w as f32 * mfact) as i32 - (iv as f32 * mivf * (nm.saturating_sub(1)) as f32) as i32;
            mh = (r.h as f32 * 0.9) as i32;
        } else {
            mw = (r.w as f32 * 0.9) as i32 - (iv as f32 * mivf * (nm.saturating_sub(1)) as f32) as i32;
            mh = (r.h as f32 * mfact) as i32;
        }
        mx = r.x + (r.w as i32 - mw) / 2;
        my = r.y + (r.h as i32 - mh - 2 * oh) / 2;
        sx = r.x + ov;
        sy = r.y + oh;
        sh = r.h as i32 - 2 * oh;
    }

    let m_w = weighted_split(mw, &cfacts[..nm]);
    let s_w = weighted_split(sw, &cfacts[nm..]);

    let mut out = Vec::with_capacity(n);
    for (i, id) in ids.iter().enumerate() {
        if i < nm {
            let w = m_w[i];
            out.push((*id, rect(mx, my, w, mh)));
            mx += w + (iv as f32 * mivf) as i32;
        } else {
            let w = s_w[i - nm];
            out.push((*id, rect(sx, sy, w, sh)));
            sx += w + iv;
        }
    }
    out
}

// ---------------------------------------------------------------- deck ----

fn deck(r: Rect, ids: &[WinId], nmaster: u32, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<(WinId, Rect)> {
    let n = ids.len();
    let (oh, ov, ih, iv) = gaps.effective(n);
    let nm = (nmaster as usize).min(n);

    let mx = r.x + ov;
    let my0 = r.y + oh;
    let mh = r.h as i32 - 2 * oh - ih * (nm.saturating_sub(1)) as i32;
    let mut mw = r.w as i32 - 2 * ov;
    let mut sx = mx;
    let sy = my0;
    let mut sh = mh;
    let sw;

    if nm > 0 && n > nm {
        let stack_w = ((mw - iv) as f32 * (1.0 - mfact)) as i32;
        sw = stack_w;
        mw = mw - iv - sw;
        sx = mx + mw + iv;
        sh = r.h as i32 - 2 * oh;
    } else {
        sw = mw;
    }

    let m_h = weighted_split(mh, &cfacts[..nm]);

    let mut out = Vec::with_capacity(n);
    let mut my = my0;
    for (i, id) in ids.iter().enumerate() {
        if i < nm {
            let h = m_h[i];
            out.push((*id, rect(mx, my, mw, h)));
            my += h + ih;
        } else {
            // deck: every stacked client gets the exact same rect (cards in a deck)
            out.push((*id, rect(sx, sy, sw, sh)));
        }
    }
    out
}

// ------------------------------------------------------------ fibonacci ----

/// `spiral` (s=false) / `dwindle` (s=true) — direct port of `fibonacci()`.
fn fibonacci(r: Rect, ids: &[WinId], gaps: Gaps, s: bool) -> Vec<(WinId, Rect)> {
    let n = ids.len();
    let (oh, ov, ih, iv) = gaps.effective(n);

    let mut nx = r.x + ov;
    let mut ny = r.y + oh;
    let mut nw = r.w as i32 - 2 * ov;
    let mut nh = r.h as i32 - 2 * oh;

    let mut hrest = 0;
    let mut wrest = 0;
    let mut recurse = true;
    let mut out = Vec::with_capacity(n);
    let mut i: usize = 0;

    for id in ids {
        if recurse {
            if (i % 2 == 1 && (nh - ih) / 2 <= 0) || (i % 2 == 0 && (nw - iv) / 2 <= 0) {
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
                    nw = (r.w as i32 - iv - 2 * ov) - ((r.w as i32 - iv - 2 * ov) as f32 * 0.5) as i32;
                    wrest = 0;
                }
                ny = r.y + oh;
            } else if i == 1 {
                nw = r.w as i32 - nw - iv - 2 * ov;
            }
            i += 1;
        }

        out.push((*id, rect(nx, ny, nw, nh)));
    }
    out
}

// -------------------------------------------------------------- grids ----

fn gapless_grid(r: Rect, ids: &[WinId], gaps: Gaps) -> Vec<(WinId, Rect)> {
    let n = ids.len();
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

    let mut ch = (r.h as i32 - 2 * oh - ih * (rows as i32 - 1)) / rows.max(1) as i32;
    let mut rrest = (r.h as i32 - 2 * oh - ih * (rows as i32 - 1)) - ch * rows as i32;
    let cw = (r.w as i32 - 2 * ov - iv * (cols as i32 - 1)) / cols as i32;
    let crest = (r.w as i32 - 2 * ov - iv * (cols as i32 - 1)) - cw * cols as i32;

    let mut x = r.x + ov;
    let mut y;
    let (mut cn, mut rn) = (0i32, 0i32);

    let mut out = Vec::with_capacity(n);
    for (i, id) in ids.iter().enumerate() {
        if i / rows + 1 > cols - n % cols {
            rows = n / cols + 1;
            ch = (r.h as i32 - 2 * oh - ih * (rows as i32 - 1)) / rows as i32;
            rrest = (r.h as i32 - 2 * oh - ih * (rows as i32 - 1)) - ch * rows as i32;
        }
        y = r.y + oh + rn * (ch + ih) + rn.min(rrest);
        let w = cw + if (cn as usize) < crest.max(0) as usize { 1 } else { 0 };
        let h = ch + if (rn as usize) < rrest.max(0) as usize { 1 } else { 0 };
        out.push((*id, rect(x, y, w, h)));

        rn += 1;
        if rn as usize >= rows {
            rn = 0;
            x += cw + ih + if (cn as usize) < crest.max(0) as usize { 1 } else { 0 };
            cn += 1;
        }
    }
    out
}

fn grid(r: Rect, ids: &[WinId], gaps: Gaps) -> Vec<(WinId, Rect)> {
    let n = ids.len();
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

    let ch = (r.h as i32 - 2 * oh - ih * (rows as i32 - 1)) / rows.max(1) as i32;
    let cw = (r.w as i32 - 2 * ov - iv * (cols as i32 - 1)) / cols.max(1) as i32;
    let chrest = (r.h as i32 - 2 * oh - ih * (rows as i32 - 1)) - ch * rows as i32;
    let cwrest = (r.w as i32 - 2 * ov - iv * (cols as i32 - 1)) - cw * cols as i32;

    let mut out = Vec::with_capacity(n);
    for (i, id) in ids.iter().enumerate() {
        let cc = i / rows.max(1);
        let cr = i % rows.max(1);
        let x = r.x + ov + cc as i32 * (cw + iv) + (cc as i32).min(cwrest);
        let y = r.y + oh + cr as i32 * (ch + ih) + (cr as i32).min(chrest);
        let w = cw + if cc < cwrest.max(0) as usize { 1 } else { 0 };
        let h = ch + if cr < chrest.max(0) as usize { 1 } else { 0 };
        out.push((*id, rect(x, y, w, h)));
    }
    out
}

fn horiz_grid(r: Rect, ids: &[WinId], gaps: Gaps, cfacts: &[f32]) -> Vec<(WinId, Rect)> {
    let n = ids.len();
    let (oh, ov, ih, iv) = gaps.effective(n);

    let ntop = if n <= 2 { n } else { n / 2 };

    let mx0 = r.x + ov;
    let my0 = r.y + oh;
    let mut mh = r.h as i32 - 2 * oh;
    let mut sh = mh;
    let mut mw = r.w as i32 - 2 * ov;
    let mut sw = mw;
    let mut sy0 = my0;

    if n > ntop {
        sh = (mh - ih) / 2;
        mh = mh - ih - sh;
        sy0 = my0 + mh + ih;
        mw = r.w as i32 - 2 * ov - iv * (ntop as i32 - 1);
        sw = r.w as i32 - 2 * ov - iv * ((n - ntop) as i32 - 1);
    }

    let m_w = weighted_split(mw, &cfacts[..ntop]);
    let s_w = weighted_split(sw, &cfacts[ntop..]);

    let mut out = Vec::with_capacity(n);
    let (mut mx, mut sx) = (mx0, mx0);
    for (i, id) in ids.iter().enumerate() {
        if i < ntop {
            let w = m_w[i];
            out.push((*id, rect(mx, my0, w, mh)));
            mx += w + iv;
        } else {
            let w = s_w[i - ntop];
            out.push((*id, rect(sx, sy0, w, sh)));
            sx += w + iv;
        }
    }
    out
}

fn nrow_grid(r: Rect, ids: &[WinId], nmaster: u32, gaps: Gaps) -> Vec<(WinId, Rect)> {
    let n = ids.len();
    let (oh, ov, ih, iv) = gaps.effective(n);

    let mut rows = nmaster as usize + 1;
    if n == 2 {
        rows = 1; // FORCE_VSPLIT: always split two clients side by side
    }
    if n < rows {
        rows = n;
    }
    let rows = rows.max(1);

    // Row height is fixed for the whole layout (nrowgrid does not redistribute
    // any rounding remainder between rows), only the column count per row and
    // the running "used width" within a row change as we go.
    let mut cols = n / rows;
    let mut uc = cols;
    let mut cy = r.y + oh;
    let ch = (r.h as i32 - 2 * oh - ih * (rows as i32 - 1)) / rows as i32;
    let mut uh = ch;

    let mut ri = 0usize;
    let mut ci = 0usize;
    let mut uw = 0i32;

    let mut out = Vec::with_capacity(n);
    for id in ids {
        if ci == cols {
            uw = 0;
            ci = 0;
            ri += 1;
            cols = (n - uc) / (rows - ri).max(1);
            uc += cols;
            cy = r.y + oh + uh + ih;
            uh += ch + ih;
        }

        let cx = r.x + ov + uw;
        let cw = (r.w as i32 - 2 * ov - uw) / (cols - ci).max(1) as i32;
        uw += cw + iv;

        out.push((*id, rect(cx, cy, cw, ch)));
        ci += 1;
    }
    out
}

// ------------------------------------------------------------- monocle ----

fn monocle(r: Rect, ids: &[WinId], gaps: Gaps) -> Vec<(WinId, Rect)> {
    // dwm's monocle() always uses the raw outer gap values directly and never
    // calls getgaps(), so `enablegaps`/`smartgaps` intentionally have no effect here.
    let (oh, ov) = (gaps.oh, gaps.ov);
    let x = r.x + ov;
    let y = r.y + oh;
    let w = r.w as i32 - 2 * ov;
    let h = r.h as i32 - 2 * oh;

    ids.iter().map(|id| (*id, rect(x, y, w, h))).collect()
}

// ------------------------------------------------------------- dispatch ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Tile,
    Monocle,
    Spiral,
    Dwindle,
    Deck,
    BStack,
    BStackHoriz,
    Grid,
    NRowGrid,
    HorizGrid,
    GaplessGrid,
    CenteredMaster,
    CenteredFloatingMaster,
}

impl Kind {
    pub fn symbol(&self) -> &'static str {
        match self {
            Kind::Tile => "[]=",
            Kind::Monocle => "[M]",
            Kind::Spiral => "[@]",
            Kind::Dwindle => "[D]",
            Kind::Deck => "H[]",
            Kind::BStack => "TTT",
            Kind::BStackHoriz => "===",
            Kind::Grid => "HHH",
            Kind::NRowGrid => "###",
            Kind::HorizGrid => "---",
            Kind::GaplessGrid => ":::",
            Kind::CenteredMaster => "|M|",
            Kind::CenteredFloatingMaster => ">M>",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DwmLayout {
    kind: Kind,
    state: SharedState,
}

impl DwmLayout {
    pub fn new(kind: Kind, state: SharedState) -> Box<dyn Layout> {
        Box::new(Self { kind, state })
    }
}

impl Layout for DwmLayout {
    fn name(&self) -> String {
        self.kind.symbol().to_string()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(self.clone())
    }

    fn layout(&mut self, s: &Stack<WinId>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(WinId, Rect)>) {
        let ids: Vec<WinId> = s.iter().copied().collect();
        if ids.is_empty() {
            return (None, vec![]);
        }

        let gaps = self.state.gaps();
        let mfact = self.state.mfact();
        let nmaster = self.state.nmaster();
        let cfacts = self.state.cfacts_for(&ids);

        let positions = match self.kind {
            Kind::Tile => tile(r, &ids, nmaster, mfact, gaps, &cfacts),
            Kind::BStack => bstack(r, &ids, nmaster, mfact, gaps, &cfacts),
            Kind::BStackHoriz => bstack_horiz(r, &ids, nmaster, mfact, gaps, &cfacts),
            Kind::CenteredMaster => centered_master(r, &ids, nmaster, mfact, gaps, &cfacts),
            Kind::CenteredFloatingMaster => centered_floating_master(r, &ids, nmaster, mfact, gaps, &cfacts),
            Kind::Deck => deck(r, &ids, nmaster, mfact, gaps, &cfacts),
            Kind::Spiral => fibonacci(r, &ids, gaps, false),
            Kind::Dwindle => fibonacci(r, &ids, gaps, true),
            Kind::GaplessGrid => gapless_grid(r, &ids, gaps),
            Kind::Grid => grid(r, &ids, gaps),
            Kind::HorizGrid => horiz_grid(r, &ids, gaps, &cfacts),
            Kind::NRowGrid => nrow_grid(r, &ids, nmaster, gaps),
            Kind::Monocle => monocle(r, &ids, gaps),
        };

        (None, positions)
    }

    fn handle_message(&mut self, _m: &Message) -> Option<Box<dyn Layout>> {
        // mfact / nmaster / gaps / cfact are all mutated directly from key
        // bindings against `SharedState` (see src/keys.rs) rather than through
        // penrose's layout-message mechanism, since in flexiflow they are
        // monitor-global settings rather than per-layout ones.
        None
    }
}
