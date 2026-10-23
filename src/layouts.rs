//! Direct ports of the layout algorithms in flexiflow's `vanitygaps.c` (dwm's
//! vanitygaps patch). Each pure `fn(..) -> Vec<R>` below is that same math;
//! `tiled_clients`/`place` walk the real client linked list, collect each
//! client's `cfact`, and resize each client in turn, subtracting `2 * bw`
//! the way vanilla dwm's `resize()` expects it pre-subtracted.
use std::ffi::{c_int, c_uint};

use crate::{Client, Gaps, Monitor, State, cfor, is_visible, nexttiled, resize};

/// (x, y, w, h)
type R = (c_int, c_int, c_int, c_int);

/// Distribute `total` pixels across `weights.len()` slots proportionally to
/// `weights` (a client's `cfact`), dumping any rounding remainder on the
/// last slot so the sum always matches `total` exactly.
fn weighted_split(total: c_int, weights: &[f32]) -> Vec<c_int> {
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

fn rect(x: c_int, y: c_int, w: c_int, h: c_int) -> R {
    (x, y, w.max(0), h.max(0))
}

// ---------------------------------------------------------------- tile ----

fn tile_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
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

fn bstack_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
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

fn bstack_horiz_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
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

fn centered_master_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
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
    let (mut ly, mut rx2, mut ry2, mut lw, mut rw) = (my0, 0, my0, 0, 0);

    if nm > 0 && n > nm {
        if n_stack > 1 {
            mw = ((rw - 2 * ov - 2 * iv) as f32 * mfact) as c_int;
            lw = (rw - mw - 2 * ov - 2 * iv) / 2;
            rw = (rw - mw - 2 * ov - 2 * iv) - lw;
            mx += lw + iv;
        } else {
            mw = ((mw - iv) as f32 * mfact) as c_int;
            lw = 0;
            rw = rw - mw - iv - 2 * ov;
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
    let (mut li, mut ri) = (0usize, 0usize);
    for i in 0..n {
        if i < n_master {
            let h = m_h[i];
            out.push(rect(mx, my, mw, h));
            my += h + ih;
        } else if (i - n_master) % 2 != 0 {
            let h = l_h[li];
            out.push(rect(lx, ly, lw, h));
            ly += h + ih;
            li += 1;
        } else {
            let h = r_h[ri];
            out.push(rect(rx2, ry2, rw, h));
            ry2 += h + ih;
            ri += 1;
        }
    }
    out
}

fn centered_floating_master_(
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

fn deck_(r: R, n: usize, nmaster: c_uint, mfact: f32, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
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
    for i in 0..n {
        if i < nm {
            let h = m_h[i];
            out.push(rect(mx, my, mw, h));
            my += h + ih;
        } else {
            // deck: every stacked client gets the exact same rect (cards in
            // a deck)
            out.push(rect(sx, sy, sw, sh));
        }
    }
    out
}

// ------------------------------------------------------------ fibonacci ----

/// `spiral` (s=false) / `dwindle` (s=true) - direct port of `fibonacci()`.
fn fibonacci_(r: R, n: usize, gaps: Gaps, s: bool) -> Vec<R> {
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

fn gapless_grid_(r: R, n: usize, gaps: Gaps) -> Vec<R> {
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

fn grid_(r: R, n: usize, gaps: Gaps) -> Vec<R> {
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

fn horiz_grid_(r: R, n: usize, gaps: Gaps, cfacts: &[f32]) -> Vec<R> {
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

fn nrow_grid_(r: R, n: usize, nmaster: c_uint, gaps: Gaps) -> Vec<R> {
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

fn monocle_(r: R, n: usize, gaps: Gaps) -> Vec<R> {
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

// ------------------------------------------------------------- harness ----

/// Collect the visible tiled clients on `m`, in stacking order, along with
/// each one's `cfact`.
unsafe fn tiled_clients(m: *mut Monitor) -> (Vec<*mut Client>, Vec<f32>) {
    unsafe {
        let mut clients = vec![];
        let mut c = nexttiled((*m).clients);
        while !c.is_null() {
            clients.push(c);
            c = nexttiled((*c).next);
        }
        let cfacts = clients.iter().map(|&c| (*c).cfact).collect();
        (clients, cfacts)
    }
}

/// Resize every client in `clients` to its matching rect in `rects`,
/// subtracting each client's own border width the way vanilla dwm's
/// `resize()` expects (it does not inset for borders itself).
unsafe fn place(state: &mut State, clients: &[*mut Client], rects: &[R]) {
    unsafe {
        for (&c, &(x, y, w, h)) in clients.iter().zip(rects) {
            let bw = (*c).bw;
            resize(state, c, x, y, w - 2 * bw, h - 2 * bw, 0);
        }
    }
}

macro_rules! tiled_layout {
    ($name:ident, |$r:ident, $n:ident, $nmaster:ident, $mfact:ident, $gaps:ident, $cfacts:ident| $body:expr) => {
        pub(crate) fn $name(state: &mut State, m: *mut Monitor) {
            unsafe {
                let (clients, cfacts) = tiled_clients(m);
                let n = clients.len();
                if n == 0 {
                    return;
                }
                let $r: R = ((*m).wx, (*m).wy, (*m).ww, (*m).wh);
                let $n = n;
                let $nmaster = (*m).nmaster as c_uint;
                let $mfact = (*m).mfact;
                let $gaps = (*m).gaps;
                let $cfacts = cfacts.as_slice();
                let rects = $body;
                place(state, &clients, &rects);
            }
        }
    };
}

tiled_layout!(tile, |r, n, nmaster, mfact, gaps, cfacts| tile_(
    r, n, nmaster, mfact, gaps, cfacts
));
tiled_layout!(bstack, |r, n, nmaster, mfact, gaps, cfacts| bstack_(
    r, n, nmaster, mfact, gaps, cfacts
));
tiled_layout!(bstack_horiz, |r, n, nmaster, mfact, gaps, cfacts| bstack_horiz_(
    r, n, nmaster, mfact, gaps, cfacts
));
tiled_layout!(centered_master, |r, n, nmaster, mfact, gaps, cfacts| centered_master_(
    r, n, nmaster, mfact, gaps, cfacts
));
tiled_layout!(centered_floating_master, |r, n, nmaster, mfact, gaps, cfacts| centered_floating_master_(
    r, n, nmaster, mfact, gaps, cfacts
));
tiled_layout!(deck, |r, n, nmaster, mfact, gaps, cfacts| deck_(
    r, n, nmaster, mfact, gaps, cfacts
));
tiled_layout!(horiz_grid, |r, n, _nmaster, _mfact, gaps, cfacts| horiz_grid_(
    r, n, gaps, cfacts
));
tiled_layout!(nrow_grid, |r, n, nmaster, _mfact, gaps, _cfacts| nrow_grid_(
    r, n, nmaster, gaps
));

pub(crate) fn spiral(state: &mut State, m: *mut Monitor) {
    unsafe {
        let (clients, _) = tiled_clients(m);
        let n = clients.len();
        if n == 0 {
            return;
        }
        let r: R = ((*m).wx, (*m).wy, (*m).ww, (*m).wh);
        let rects = fibonacci_(r, n, (*m).gaps, false);
        place(state, &clients, &rects);
    }
}

pub(crate) fn dwindle(state: &mut State, m: *mut Monitor) {
    unsafe {
        let (clients, _) = tiled_clients(m);
        let n = clients.len();
        if n == 0 {
            return;
        }
        let r: R = ((*m).wx, (*m).wy, (*m).ww, (*m).wh);
        let rects = fibonacci_(r, n, (*m).gaps, true);
        place(state, &clients, &rects);
    }
}

pub(crate) fn gapless_grid(state: &mut State, m: *mut Monitor) {
    unsafe {
        let (clients, _) = tiled_clients(m);
        let n = clients.len();
        if n == 0 {
            return;
        }
        let r: R = ((*m).wx, (*m).wy, (*m).ww, (*m).wh);
        let rects = gapless_grid_(r, n, (*m).gaps);
        place(state, &clients, &rects);
    }
}

pub(crate) fn grid(state: &mut State, m: *mut Monitor) {
    unsafe {
        let (clients, _) = tiled_clients(m);
        let n = clients.len();
        if n == 0 {
            return;
        }
        let r: R = ((*m).wx, (*m).wy, (*m).ww, (*m).wh);
        let rects = grid_(r, n, (*m).gaps);
        place(state, &clients, &rects);
    }
}

pub(crate) fn monocle(state: &mut State, m: *mut Monitor) {
    unsafe {
        let mut n = 0;
        let mut c;
        cfor!((c = (*m).clients; !c.is_null(); c = (*c).next) {
            if is_visible(c) {
                n += 1;
            }
        });
        if n > 0 {
            // override layout symbol, matching dwm's stock monocle()
            (*m).ltsymbol = format!("[{n}]");
        }
        let (clients, _) = tiled_clients(m);
        let n = clients.len();
        if n == 0 {
            return;
        }
        let r: R = ((*m).wx, (*m).wy, (*m).ww, (*m).wh);
        let rects = monocle_(r, n, (*m).gaps);
        place(state, &clients, &rects);
    }
}
