//! The X11 harness around flexiflow-common's layout algorithms:
//! `tiled_clients`/`place` walk the real client linked list, collect each
//! client's `cfact`, and resize each client in turn, subtracting `2 * bw`
//! the way vanilla dwm's `resize()` expects it pre-subtracted.
use std::ffi::c_uint;

pub use flexiflow_common::layouts::*;

use crate::{Client, Monitor, State, cfor, is_visible, nexttiled, resize};

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
