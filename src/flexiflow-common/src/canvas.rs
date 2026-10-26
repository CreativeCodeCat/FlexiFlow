//! Drawing into wl_shm buffers: rectangles, text cut short with "..."
//! the way dwm's `drw_text` does it, and icons. Shared by the bar and
//! the hotkeys popup.
use crate::{config::parse_color, font::FontSet};

/// An icon as non-premultiplied RGBA rows.
#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// An ARGB8888 buffer, as wl_shm wants it.
pub struct Canvas<'a> {
    pub buf: &'a mut [u8],
    pub w: i32,
    pub h: i32,
}

pub fn argb(color: &str) -> u32 {
    let (r, g, b, a) = parse_color(color).unwrap_or((0, 0, 0, 0xff));
    u32::from_be_bytes([a, r, g, b])
}

pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let [_, ar, ag, ab] = a.to_be_bytes();
    let [_, br, bg, bb] = b.to_be_bytes();
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    u32::from_be_bytes([0xff, m(ar, br), m(ag, bg), m(ab, bb)])
}

impl Canvas<'_> {
    fn put(&mut self, x: i32, y: i32, c: u32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = ((y * self.w + x) * 4) as usize;
        self.buf[i..i + 4].copy_from_slice(&c.to_le_bytes());
    }

    fn get(&self, x: i32, y: i32) -> u32 {
        let i = ((y * self.w + x) * 4) as usize;
        u32::from_le_bytes(self.buf[i..i + 4].try_into().unwrap())
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u32) {
        for yy in y.max(0)..(y + h).min(self.h) {
            for xx in x.max(0)..(x + w).min(self.w) {
                self.put(xx, yy, c);
            }
        }
    }

    /// Blend non-premultiplied RGBA `icon` over the canvas at `x`, `y`.
    pub fn image(&mut self, x: i32, y: i32, icon: &Icon) {
        for row in 0..icon.h as i32 {
            for col in 0..icon.w as i32 {
                let (px, py) = (x + col, y + row);
                if px < 0 || py < 0 || px >= self.w || py >= self.h {
                    continue;
                }
                let i = ((row * icon.w as i32 + col) * 4) as usize;
                let p = &icon.rgba[i..i + 4];
                if p[3] == 0 {
                    continue;
                }
                let over = u32::from_be_bytes([0xff, p[0], p[1], p[2]]);
                let under = self.get(px, py);
                self.put(px, py, mix(under, over, p[3] as f32 / 255.0));
            }
        }
    }

    /// dwm's `drw_rect`: filled, or a one-pixel outline.
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, filled: bool, c: u32) {
        if filled {
            self.fill(x, y, w, h, c);
        } else {
            self.fill(x, y, w, 1, c);
            self.fill(x, y + h - 1, w, 1, c);
            self.fill(x, y, 1, h, c);
            self.fill(x + w - 1, y, 1, h, c);
        }
    }

    /// dwm's `drw_text`: fill `w`x`h` in `bg`, then `text` in `fg` from
    /// `lpad` in, vertically centered, cut short with "..." if it
    /// doesn't fit. Returns the x just past the box.
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &mut self,
        font: &mut FontSet,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        lpad: i32,
        text: &str,
        fg: u32,
        bg: u32,
    ) -> i32 {
        self.fill(x, y, w, h, bg);
        let room = w - lpad;
        let mut shown: String = text.to_string();
        if font.width(&shown) > room {
            let ell = font.width("...");
            while !shown.is_empty() && font.width(&shown) + ell > room {
                shown.pop();
            }
            if font.width(&shown) + ell <= room {
                shown.push_str("...");
            }
        }
        let baseline = y + (h - font.height) / 2 + font.ascent;
        let mut pen = (x + lpad) as f32;
        for c in shown.chars() {
            let g = font.glyph(c);
            let gx = pen.round() as i32 + g.metrics.xmin;
            let gy = baseline - g.metrics.height as i32 - g.metrics.ymin;
            let (gw, gh) = (g.metrics.width as i32, g.metrics.height as i32);
            let advance = g.metrics.advance_width;
            for row in 0..gh {
                for col in 0..gw {
                    let a = g.coverage[(row * gw + col) as usize];
                    let (px, py) = (gx + col, gy + row);
                    if a == 0 || px < x || px >= x + w || py < y || py >= y + h || px >= self.w || py >= self.h {
                        continue;
                    }
                    let under = self.get(px, py);
                    self.put(px, py, mix(under, fg, a as f32 / 255.0));
                }
            }
            pen += advance;
        }
        x + w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_are_argb() {
        assert_eq!(argb("#FF59F9"), 0xffff59f9);
        assert_eq!(mix(0xff000000, 0xffffffff, 0.5), 0xff808080);
    }

    #[test]
    fn text_is_cut_short_with_an_ellipsis() {
        let Some(mut font) = FontSet::new("monospace:pixelsize=12") else { return };
        let mut buf = vec![0u8; 200 * 20 * 4];
        let mut c = Canvas { buf: &mut buf, w: 200, h: 20 };
        let end = c.text(&mut font, 0, 0, 40, 20, 0, "a very long window title", 0xffffffff, 0xff000000);
        assert_eq!(end, 40);
        assert!(buf.chunks(4).any(|p| p[0] != 0), "nothing was drawn");
    }
}
