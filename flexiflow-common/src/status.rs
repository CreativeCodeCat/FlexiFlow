//! The status text's inline codes, as `flexiflow-river-status` writes them and
//! the X11 FlexiFlow's bar draws them:
//!
//! - `^c#RRGGBB^` sets the foreground and `^b#RRGGBB^` the background;
//!   `^d^` resets both to the bar's normal colors
//! - `^o#RRGGBB^` starts a box bordered in that color, ending any box
//!   already open; `^o^` ends it, as does the end of the text
//! - `^f<font>^` draws the runs after it in that font, say
//!   `^fMonospace:size=9^`; `^f^` goes back to the bar's own
//! - `^a<name>^` makes the runs after it a click area called `name`
//!   (see `Config::statusclick`); `^a^` ends it
//!
//! Unrecognized codes are dropped silently.

/// A run of text in its own colors, font and click area.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub text: String,
    pub fg: String,
    pub bg: String,
    /// `None` for the bar's own font
    pub font: Option<String>,
    /// `None` outside any click area
    pub area: Option<String>,
}

/// A border around runs `start..end`, in `color`.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusBox {
    pub start: usize,
    pub end: usize,
    pub color: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct Parsed {
    pub runs: Vec<Run>,
    pub boxes: Vec<StatusBox>,
}

pub fn parse(text: &str, norm_fg: &str, norm_bg: &str) -> Parsed {
    let mut parsed = Parsed::default();
    let mut font: Option<String> = None;
    let mut area: Option<String> = None;
    let mut open: Option<(usize, String)> = None;
    let (mut fg, mut bg) = (norm_fg.to_string(), norm_bg.to_string());
    let mut buf = String::new();
    let mut chars = text.chars();
    let close = |open: &mut Option<(usize, String)>,
                 boxes: &mut Vec<StatusBox>,
                 end: usize| {
        if let Some((start, color)) = open.take()
            && end > start
        {
            boxes.push(StatusBox { start, end, color });
        }
    };
    let named = |name: &str| (!name.is_empty()).then(|| name.to_string());
    while let Some(c) = chars.next() {
        if c != '^' {
            buf.push(c);
            continue;
        }
        let code: String = chars.by_ref().take_while(|&c| c != '^').collect();
        if !buf.is_empty() {
            parsed.runs.push(Run {
                text: std::mem::take(&mut buf),
                fg: fg.clone(),
                bg: bg.clone(),
                font: font.clone(),
                area: area.clone(),
            });
        }
        let end = parsed.runs.len();
        if let Some(hex) = code.strip_prefix('c') {
            fg = hex.to_string();
        } else if let Some(hex) = code.strip_prefix('b') {
            bg = hex.to_string();
        } else if let Some(hex) = code.strip_prefix('o') {
            close(&mut open, &mut parsed.boxes, end);
            open = named(hex).map(|color| (end, color));
        } else if let Some(name) = code.strip_prefix('f') {
            font = named(name);
        } else if let Some(name) = code.strip_prefix('a') {
            area = named(name);
        } else if code == "d" {
            fg = norm_fg.to_string();
            bg = norm_bg.to_string();
        }
    }
    if !buf.is_empty() {
        parsed.runs.push(Run { text: buf, fg, bg, font, area });
    }
    close(&mut open, &mut parsed.boxes, parsed.runs.len());
    parsed
}

/// The gap around each status box, on all four sides: a fifth of the
/// bar's height `h`, but at least 2 pixels.
pub fn box_margin(h: i32) -> i32 {
    (h / 5).max(2)
}

/// Where each run starts, counting from the left of the status text,
/// and how wide the whole text is, for runs `widths` wide with `margin`
/// around each of `boxes`. Boxes side by side share one gap.
pub fn layout(widths: &[i32], boxes: &[StatusBox], margin: i32) -> (Vec<i32>, i32) {
    let mut xs = Vec::with_capacity(widths.len());
    let mut x = 0;
    let mut after_box = false;
    for (i, w) in widths.iter().enumerate() {
        if boxes.iter().any(|b| b.start == i) && !after_box {
            x += margin;
        }
        xs.push(x);
        x += w;
        after_box = boxes.iter().any(|b| b.end == i + 1);
        if after_box {
            x += margin;
        }
    }
    (xs, x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_split_text_into_runs() {
        let p = parse("^c#111111^^b#222222^ a ^d^b", "#fff", "#000");
        assert_eq!(p.runs.len(), 2);
        assert_eq!((&*p.runs[0].fg, &*p.runs[0].bg), ("#111111", "#222222"));
        assert_eq!((&*p.runs[1].fg, &*p.runs[1].bg), ("#fff", "#000"));
    }

    #[test]
    fn flexiflow_bar_chips_parse_into_boxes_areas_and_a_font() {
        let p = parse(
            "^fMono:size=9^^avolume^^o#A165E8^^c#283036^^b#C9A4FF^ 50% ^a^^o#2BC4C9^ x ",
            "#fff",
            "#000",
        );
        assert_eq!(p.runs.len(), 2);
        assert_eq!(p.runs[0].area.as_deref(), Some("volume"));
        assert_eq!(p.runs[1].area, None);
        assert!(p.runs.iter().all(|r| r.font.as_deref() == Some("Mono:size=9")));
        assert_eq!(p.boxes.len(), 2);
        assert_eq!((p.boxes[0].start, p.boxes[0].end), (0, 1));
        assert_eq!((p.boxes[1].start, p.boxes[1].end), (1, 2));
    }

    #[test]
    fn boxes_side_by_side_share_one_gap() {
        let boxes = [
            StatusBox { start: 0, end: 1, color: "#fff".into() },
            StatusBox { start: 1, end: 2, color: "#fff".into() },
        ];
        let (xs, w) = layout(&[10, 10], &boxes, 3);
        assert_eq!(xs, [3, 16]);
        assert_eq!(w, 29);
    }
}
