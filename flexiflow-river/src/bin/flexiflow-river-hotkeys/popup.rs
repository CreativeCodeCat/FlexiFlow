//! Measuring and drawing the popup, the way the X11 FlexiFlow's
//! `hotkeys` does: a title and a search field over a line, then the keys
//! in columns, each group under a heading marked in its accent and each
//! key as a row of badges followed by what it does.
use flexiflow_river::{
    canvas::{Canvas, argb, mix},
    config::Config,
    font::FontSet,
};

use crate::lines::{HotkeyLine, capitalized, hotkey_columns, hotkey_filter, hotkey_parts};

/// The popup's title, at its top left beside the search field.
const TITLE: &str = "Keybindings";

/// The search field's text before anything is typed.
const PLACEHOLDER: &str = "\u{F002}  Type to filter";

/// Accents for the groups that `hotkeys.groups` gives no color: KDE
/// Breeze's accents, all different.
const PALETTE: [&str; 11] = [
    "#3DAEE9", "#27AE60", "#9B59B6", "#F67400", "#FDBC4B", "#1ABC9C", "#DA4453", "#E93A9A", "#2980B9", "#C9CE3B",
    "#A1A9B1",
];

/// How much of a group's accent tints its key badges.
const BADGE_TINT: f32 = 0.18;

/// The popup's border width, in `hotkeys.dim`.
pub const BORDER: i32 = 1;

/// The popup's colors and measurements, worked out once when it opens.
pub struct Style {
    bg: u32,
    fg: u32,
    pub dim: u32,
    key_fg: u32,
    key_bg: u32,
    /// per group, by position: its accent, and its badges' background
    accents: Vec<(u32, u32)>,
    /// per group, by position: its heading
    pub labels: Vec<String>,
    /// the X11 bar's `lrpad`: the font's height
    pad: i32,
    /// a row's height: the font's height plus the bar's padding
    bh: i32,
    /// where the list starts, below the title and separator
    top: i32,
}

impl Style {
    pub fn new(cfg: &Config, font: &FontSet, lines: &[HotkeyLine]) -> Self {
        let hk = &cfg.hotkeys;
        let defaults = flexiflow_river::config::Hotkeys::default();
        let clr = |name: &str, fallback: &str| {
            [name, fallback]
                .into_iter()
                .find(|c| flexiflow_river::config::parse_color(c).is_some())
                .map_or(argb(&cfg.colors.norm.bg), argb)
        };
        let key_bg = clr(&hk.key_bg, &defaults.key_bg);
        let (mut accents, mut labels) = (Vec::new(), Vec::new());
        for line in lines {
            let HotkeyLine::Group(i, name) = line else { continue };
            let group = hk.groups.iter().find(|g| g.name == *name);
            let accent = clr(group.map_or("", |g| g.color.as_str()), PALETTE[i % PALETTE.len()]);
            accents.push((accent, mix(key_bg, accent, BADGE_TINT)));
            labels.push(match group {
                Some(g) if !g.label.is_empty() => g.label.clone(),
                _ => capitalized(name),
            });
        }
        let pad = font.height;
        let bh = font.height + cfg.barpadding;
        Self {
            bg: clr(&hk.bg, &defaults.bg),
            fg: clr(&hk.fg, &defaults.fg),
            dim: clr(&hk.dim, &defaults.dim),
            key_fg: clr(&hk.key_fg, &defaults.key_fg),
            key_bg,
            accents,
            labels,
            pad,
            bh,
            top: pad / 2 + bh + pad,
        }
    }

    /// The extra space above a heading that doesn't start its column.
    fn group_gap(&self) -> i32 {
        self.pad / 2
    }

    /// The gap between a column's keys and its descriptions, and between
    /// columns.
    fn gap(&self) -> i32 {
        self.pad * 2
    }

    /// The padding either side of a key badge's text, and the space
    /// between badges.
    fn badge_pad(&self) -> (i32, i32) {
        (self.pad / 3, 3)
    }
}

/// `text`'s width plus `pad`, like the X11 bar's `textw`.
fn textw(font: &mut FontSet, text: &str, pad: i32) -> i32 {
    font.width(text) + pad
}

/// The width of `keys` drawn as badges.
fn badges_w(style: &Style, font: &mut FontSet, keys: &str) -> i32 {
    let (pad, space) = style.badge_pad();
    let parts = hotkey_parts(keys);
    let n = parts.len() as i32;
    parts.into_iter().map(|p| textw(font, p, 0) + pad * 2).sum::<i32>() + space * (n - 1).max(0)
}

/// A column's badge and description widths; a heading spans both.
fn column_w(style: &Style, font: &mut FontSet, col: &[&HotkeyLine]) -> (i32, i32) {
    let (mut keys, mut descs, mut heading) = (0, 0, 0);
    for line in col {
        match line {
            HotkeyLine::Group(i, _) => heading = heading.max(textw(font, &style.labels[*i], style.pad)),
            HotkeyLine::Key(_, k, d) => {
                keys = keys.max(badges_w(style, font, k));
                descs = descs.max(textw(font, d, 0));
            }
        }
    }
    (keys, descs.max(heading - keys - style.gap()))
}

/// A column's height: a row per line, plus a gap above each heading but
/// its first.
fn column_h(style: &Style, col: &[&HotkeyLine]) -> i32 {
    col.iter()
        .enumerate()
        .map(|(i, line)| match line {
            HotkeyLine::Group(..) if i > 0 => style.bh + style.group_gap(),
            _ => style.bh,
        })
        .sum()
}

/// The popup's size for `lines` in columns of about `per_col` lines.
fn size(style: &Style, font: &mut FontSet, lines: &[&HotkeyLine], per_col: usize) -> (i32, i32) {
    let (pad, gap) = (style.pad, style.gap());
    let columns = hotkey_columns(lines, per_col);
    let widths: i32 = columns
        .iter()
        .map(|col| {
            let (k, d) = column_w(style, font, col);
            k + gap + d
        })
        .sum();
    let header = textw(font, TITLE, 0) + gap + textw(font, PLACEHOLDER, pad) * 2;
    let w = (widths + gap * (columns.len() as i32 - 1)).max(header) + pad * 2;
    let rows = columns.iter().map(|c| column_h(style, c)).max().unwrap_or(0);
    (w, style.top + rows + pad)
}

/// `hotkeys.columns` columns, with the lines shared out evenly between
/// them - more if they'd be taller than 90% of the area, fewer if the
/// popup would be wider than it. Returns the popup's size and how many
/// lines go in a column. The popup keeps the size of the unfiltered list
/// while filtering, so it doesn't jump.
pub fn layout(cfg: &Config, style: &Style, font: &mut FontSet, lines: &[HotkeyLine], area: (i32, i32)) -> (i32, i32, usize) {
    let (aw, ah) = area;
    let fit = ((ah * 9 / 10 - style.top - style.pad) / style.bh).max(2) as usize;
    let fewest = lines.len().div_ceil(fit);
    let mut ncols = cfg.hotkeys.columns.max(fewest);
    let all: Vec<&HotkeyLine> = lines.iter().collect();
    loop {
        let mut per_col = lines.len().div_ceil(ncols.max(1));
        // headings moved off a column's bottom take rows too: lengthen
        // the columns until they fit in as many as planned
        while hotkey_columns(&all, per_col).len() > ncols {
            per_col += 1;
        }
        let (w, h) = size(style, font, &all, per_col);
        if w <= aw * 19 / 20 || ncols <= fewest.max(1) {
            return (w, h, per_col);
        }
        ncols -= 1;
    }
}

/// Draw the popup, `lines` filtered by `query`, filling `c`.
pub fn draw(c: &mut Canvas, style: &Style, font: &mut FontSet, lines: &[HotkeyLine], query: &str, per_col: usize) {
    let (w, h) = (c.w, c.h);
    let (pad, bh, gap) = (style.pad, style.bh, style.gap());
    let (badge_pad, badge_space) = style.badge_pad();

    c.fill(0, 0, w, h, style.bg);

    // the title, the search field to its right, and a line under both
    let y = pad / 2;
    let titlew = textw(font, TITLE, 0);
    c.text(font, pad, y, titlew, bh, 0, TITLE, style.fg, style.bg);
    let fieldw = textw(font, PLACEHOLDER, pad) * 2;
    let (text, fg) = if query.is_empty() {
        (PLACEHOLDER.to_string(), style.dim)
    } else {
        (format!("\u{F002}  {query}_"), style.fg)
    };
    c.text(font, w - pad - fieldw, y + 2, fieldw, bh - 4, pad / 2, &text, fg, style.key_bg);
    c.fill(pad, y + bh + pad / 2, w - pad * 2, 1, style.dim);

    let shown = hotkey_filter(lines, query, &style.labels);
    if shown.is_empty() {
        let none = format!("No keybindings match \u{201C}{query}\u{201D}");
        c.text(font, pad, style.top, w - pad * 2, bh, 0, &none, style.dim, style.bg);
        return;
    }

    let mut x = pad;
    for col in hotkey_columns(&shown, per_col) {
        let (keysw, descw) = column_w(style, font, &col);
        let mut y = style.top;
        for (i, line) in col.iter().enumerate() {
            match line {
                HotkeyLine::Group(group, _) => {
                    if i > 0 {
                        y += style.group_gap();
                    }
                    // a slim bar in the accent, then the heading in it
                    let (accent, _) = style.accents[*group];
                    c.fill(x, y + bh / 5, 3, bh - bh / 5 * 2, accent);
                    c.text(font, x + 3, y, keysw + gap + descw - 3, bh, pad / 2, &style.labels[*group], accent, style.bg);
                }
                HotkeyLine::Key(group, keys, desc) => {
                    let (_, badge_bg) = style.accents[*group];
                    let mut bx = x;
                    for part in hotkey_parts(keys) {
                        let bw = textw(font, part, 0) + badge_pad * 2;
                        c.text(font, bx, y + 3, bw, bh - 6, badge_pad, part, style.key_fg, badge_bg);
                        bx += bw + badge_space;
                    }
                    c.text(font, x + keysw + gap, y, descw, bh, 0, desc, style.fg, style.bg);
                }
            }
            y += bh;
        }
        x += keysw + gap + descw + gap;
    }
}
