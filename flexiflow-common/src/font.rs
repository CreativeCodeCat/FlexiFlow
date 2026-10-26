//! Fonts by fontconfig name, like Xft's `"Hack Nerd Font:size=10"`, drawn
//! with fontdue. A character the font doesn't have comes from whichever
//! font fontconfig picks for it, the way Xft falls back, so the tags'
//! CJK and the bar's Nerd Font icons draw whatever the main font is.
use std::{collections::HashMap, process::Command};

use fontdue::{Font, FontSettings, Metrics};

/// What Xft assumes when nothing sets `Xft.dpi`.
const DPI: f32 = 96.0;

/// Ask fontconfig for the file (and face index) best matching `pattern`.
fn fc_match(pattern: &str) -> Option<(String, u32)> {
    let out = Command::new("fc-match").args(["-f", "%{file}\t%{index}", pattern]).output().ok()?;
    let out = String::from_utf8(out.stdout).ok()?;
    let (file, index) = out.split_once('\t')?;
    Some((file.to_string(), index.trim().parse().unwrap_or(0)))
}

fn load(file: &str, index: u32) -> Option<Font> {
    let data = std::fs::read(file).ok()?;
    let settings = FontSettings { collection_index: index, ..Default::default() };
    Font::from_bytes(data, settings).ok()
}

/// The pixel size `pattern` asks for: its `pixelsize`, else its point
/// `size` at 96 dpi, else 12 pixels.
pub fn pixel_size(pattern: &str) -> f32 {
    let prop = |name: &str| {
        pattern.split(':').find_map(|p| p.strip_prefix(name)?.strip_prefix('=')?.parse::<f32>().ok())
    };
    prop("pixelsize").or_else(|| prop("size").map(|pt| pt * DPI / 72.0)).unwrap_or(12.0)
}

pub struct Glyph {
    pub metrics: Metrics,
    pub coverage: Vec<u8>,
}

/// One font name's faces: the main one, then fallbacks as characters
/// need them.
pub struct FontSet {
    pattern: String,
    pub px: f32,
    faces: Vec<Font>,
    /// which face each character was found in
    face_of: HashMap<char, usize>,
    glyphs: HashMap<char, Glyph>,
    pub ascent: i32,
    pub height: i32,
}

impl FontSet {
    pub fn new(pattern: &str) -> Option<Self> {
        let (file, index) = fc_match(pattern)?;
        let face = load(&file, index)?;
        let px = pixel_size(pattern);
        let lm = face.horizontal_line_metrics(px)?;
        Some(Self {
            pattern: pattern.to_string(),
            px,
            faces: vec![face],
            face_of: HashMap::new(),
            glyphs: HashMap::new(),
            ascent: lm.ascent.ceil() as i32,
            height: (lm.ascent - lm.descent).ceil() as i32,
        })
    }

    fn face(&mut self, c: char) -> usize {
        if let Some(&i) = self.face_of.get(&c) {
            return i;
        }
        let found = self.faces.iter().position(|f| f.lookup_glyph_index(c) != 0).or_else(|| {
            // the main font's family first, so a Nerd Font's own
            // variants win, then anything that has the character
            let family = self.pattern.split(':').next().unwrap_or("");
            let pattern = format!("{family}:charset={:x}", c as u32);
            let (file, index) = fc_match(&pattern)?;
            let face = load(&file, index).filter(|f| f.lookup_glyph_index(c) != 0)?;
            self.faces.push(face);
            Some(self.faces.len() - 1)
        });
        let i = found.unwrap_or(0);
        self.face_of.insert(c, i);
        i
    }

    pub fn glyph(&mut self, c: char) -> &Glyph {
        if !self.glyphs.contains_key(&c) {
            let face = self.face(c);
            let (metrics, coverage) = self.faces[face].rasterize(c, self.px);
            self.glyphs.insert(c, Glyph { metrics, coverage });
        }
        &self.glyphs[&c]
    }

    pub fn width(&mut self, text: &str) -> i32 {
        text.chars().map(|c| self.glyph(c).metrics.advance_width).sum::<f32>().round() as i32
    }
}

/// The bar's font, plus any the status text asks for with `^f`.
pub struct Fonts {
    pub main: FontSet,
    others: HashMap<String, Option<FontSet>>,
}

impl Fonts {
    pub fn new(patterns: &[String]) -> Self {
        let main = patterns
            .iter()
            .find_map(|p| FontSet::new(p))
            .or_else(|| FontSet::new("monospace:size=10"))
            .expect("fontconfig found no font at all");
        Self { main, others: HashMap::new() }
    }

    /// `name`'s font, or the bar's own for `None` or one that won't load.
    pub fn get(&mut self, name: Option<&str>) -> &mut FontSet {
        let Some(name) = name else { return &mut self.main };
        let loaded = self.others.entry(name.to_string()).or_insert_with(|| {
            let f = FontSet::new(name);
            if f.is_none() {
                log::warn!("status text: can't load font {name:?}");
            }
            f
        });
        match loaded {
            Some(f) => f,
            None => &mut self.main,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_come_from_the_pattern_like_xft() {
        assert_eq!(pixel_size("Hack Nerd Font:pixelsize=14"), 14.0);
        assert!((pixel_size("Hack Nerd Font:size=9") - 12.0).abs() < 0.01);
        assert_eq!(pixel_size("monospace"), 12.0);
    }
}
