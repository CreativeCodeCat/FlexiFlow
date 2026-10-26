//! Drawing one output's bar the way the X11 FlexiFlow's `drawbar` does:
//! the menu button, the tags (each in its own accent), the layout symbol,
//! the power button, the focused window's title and, on the focused
//! output, the status text with its inline color codes and boxes.
use flexiflow_river::{
    canvas::{Canvas, Icon, argb, mix},
    config::{Config, Scheme, parse_color},
    font::Fonts,
    ipc::{BarState, OutputState},
    status,
};

/// How much of a tag's accent tints the bar background behind it while
/// it's under the pointer.
const HOVER_TINT: f32 = 0.3;

/// What a stretch of the bar is, for clicks.
#[derive(Debug, Clone, PartialEq)]
pub enum Region {
    Menu,
    Tag(usize),
    Layout,
    Power,
    Title,
    /// the status text, inside the named click area or not
    Status(Option<String>),
    /// the n-th system tray icon
    Tray(usize),
}

fn tagshown(bar: &BarState, o: &OutputState, i: usize) -> bool {
    !bar.hidevacant || (o.tags | o.occupied) & (1 << i) != 0
}

/// Draw output `o`'s bar, returning where each clickable part is.
pub fn bar(
    c: &mut Canvas,
    fonts: &mut Fonts,
    cfg: &Config,
    bar: &BarState,
    o: &OutputState,
    hover: Option<usize>,
    tray: &[Icon],
) -> Vec<(i32, i32, Region)> {
    let h = c.h;
    let lrpad = fonts.main.height;
    let col = |s: &Scheme| (argb(&s.fg), argb(&s.bg));
    let (norm_fg, norm_bg) = col(&cfg.colors.norm);
    let mut regions = vec![];
    c.fill(0, 0, c.w, h, norm_bg);

    // the status first, so it's what gets cut short if the bar's too narrow
    // the tray at the right end, or left of the status with
    // `systrayonleft`, like the X11 bar
    let spacing = cfg.systrayspacing.max(0);
    let trayw = if tray.is_empty() {
        0
    } else {
        tray.iter().map(|i| i.w as i32 + spacing).sum::<i32>() + spacing
    };
    let (tray_x, status_end) = if cfg.systrayonleft { (0, c.w) } else { (c.w - trayw, c.w - trayw) };
    let mut tw = 0;
    if o.selected {
        let (w, areas) = draw_status(c, fonts, cfg, &bar.status, status_end, h);
        tw = w;
        regions.extend(areas);
    }
    if !tray.is_empty() {
        let tray_x = if cfg.systrayonleft { c.w - tw - trayw } else { tray_x };
        c.fill(tray_x, 0, trayw, h, norm_bg);
        let mut x = tray_x + spacing;
        for (i, icon) in tray.iter().enumerate() {
            c.image(x, (h - icon.h as i32) / 2, icon);
            regions.insert(0, (x - spacing / 2, x + icon.w as i32 + spacing - spacing / 2, Region::Tray(i)));
            x += icon.w as i32 + spacing;
        }
        tw += trayw;
    }

    let mut x = 0;
    let textw = |fonts: &mut Fonts, s: &str| fonts.main.width(s) + lrpad;
    if !cfg.menubutton.is_empty() {
        let w = textw(fonts, &cfg.menubutton);
        let (fg, bg) = col(&cfg.colors.menu);
        c.text(&mut fonts.main, x, 0, w, h, lrpad / 2, &cfg.menubutton, fg, bg);
        regions.push((x, x + w, Region::Menu));
        x += w;
    }
    let boxs = fonts.main.height / 9;
    let boxw = fonts.main.height / 6 + 2;
    for (i, tag) in cfg.tags.iter().enumerate() {
        if !tagshown(bar, o, i) {
            continue;
        }
        let w = textw(fonts, tag);
        let selected = o.tags & (1 << i) != 0;
        let occupied = o.occupied & (1 << i) != 0;
        let accent = cfg.tagcolors.get(i);
        let (fg, bg) = if selected {
            col(cfg.tagscheme(i))
        } else if hover == Some(i) {
            let tint = accent.map_or(argb(&cfg.colors.sel.bg), |a| argb(&a.bg));
            (norm_fg, mix(norm_bg, tint, HOVER_TINT))
        } else if occupied && let Some(a) = accent {
            // has windows but isn't in view: the tag's accent as its text
            (argb(&a.bg), norm_bg)
        } else {
            (norm_fg, norm_bg)
        };
        c.text(&mut fonts.main, x, 0, w, h, lrpad / 2, tag, fg, bg);
        if occupied {
            let filled = o.selected && o.seltags & (1 << i) != 0;
            c.rect(x + boxs, boxs, boxw, boxw, filled, fg);
        }
        regions.push((x, x + w, Region::Tag(i)));
        x += w;
    }

    let w = textw(fonts, &o.ltsymbol);
    let (fg, bg) = col(&cfg.colors.layout);
    c.text(&mut fonts.main, x, 0, w, h, lrpad / 2, &o.ltsymbol, fg, bg);
    regions.push((x, x + w, Region::Layout));
    x += w;

    if !cfg.powermenu.symbol.is_empty() {
        let w = textw(fonts, &cfg.powermenu.symbol);
        let (fg, bg) = col(&cfg.colors.power);
        c.text(&mut fonts.main, x, 0, w, h, lrpad / 2, &cfg.powermenu.symbol, fg, bg);
        regions.push((x, x + w, Region::Power));
        x += w;
    }

    let w = c.w - tw - x;
    if w > h {
        if let Some(title) = &o.title {
            let (fg, bg) = col(if o.selected { &cfg.colors.title } else { &cfg.colors.norm });
            c.text(&mut fonts.main, x, 0, w, h, lrpad / 2, title, fg, bg);
            if o.floating {
                c.rect(x + boxs, boxs, boxw, boxw, false, fg);
            }
        }
        regions.push((x, x + w, Region::Title));
    }
    regions
}

/// The status text right-aligned against `x_end`, each run in its own
/// colors and font, with boxes around runs. Returns its width and click
/// areas.
fn draw_status(
    c: &mut Canvas,
    fonts: &mut Fonts,
    cfg: &Config,
    text: &str,
    x_end: i32,
    h: i32,
) -> (i32, Vec<(i32, i32, Region)>) {
    let lrpad = fonts.main.height;
    let status::Parsed { runs, boxes } = status::parse(text, &cfg.colors.norm.fg, &cfg.colors.norm.bg);
    let widths: Vec<i32> = runs.iter().map(|r| fonts.get(r.font.as_deref()).width(&r.text)).collect();
    let margin = status::box_margin(h);
    let (xs, total) = status::layout(&widths, &boxes, margin);
    let tw = total + lrpad / 2 + 2;
    let x0 = x_end - tw + (lrpad / 2 - 2).max(0);
    let norm_bg = argb(&cfg.colors.norm.bg);
    c.fill(x_end - tw, 0, tw, h, norm_bg);

    let mut regions = vec![(x_end - tw, x_end, Region::Status(None))];
    let boxed = |i: usize| boxes.iter().any(|b| (b.start..b.end).contains(&i));
    for (i, (run, &w)) in runs.iter().zip(&widths).enumerate() {
        let (y, rh) = if boxed(i) { (margin, h - 2 * margin) } else { (0, h) };
        let x = x0 + xs[i];
        let font = fonts.get(run.font.as_deref());
        c.text(font, x, y, w, rh, 0, &run.text, argb(&run.fg), argb(&run.bg));
        if let Some(area) = &run.area {
            // inside the whole-status region, so found first
            regions.insert(0, (x, x + w, Region::Status(Some(area.clone()))));
        }
    }
    for b in &boxes {
        let w: i32 = widths[b.start..b.end].iter().sum();
        if parse_color(&b.color).is_some() {
            c.rect(x0 + xs[b.start], margin, w, h - 2 * margin, false, argb(&b.color));
        }
    }
    (tw, regions)
}
