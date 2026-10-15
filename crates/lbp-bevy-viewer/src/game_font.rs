//! D321 (docs/level-scripts.md part 72): the game's own fonts for the scripted Pod menu's
//! text, read from the user's game data at run time, and the menu card's pictures.
//!
//! The fonts (`gamedata/fonts/*.fnt`) are loose files on the disc, outside the archive the
//! snapshot indexes; they are looked up under each `--game-dir` root
//! (`USRDIR/gamedata/fonts/...` or `PS3_GAME/USRDIR/gamedata/fonts/...`, any case) and
//! accepted when their size is the MAP row's. A glyph's pixels are a distance field
//! (MEASURED, part 72: 0x80 at the outline, about 0x1e a font pixel across it, in the
//! first byte of uncompressed fonts and the only byte of compressed ones); text is
//! rasterized from it at the size it is drawn.
use lbp_catalog::Catalog;
use lbp_resource::font::{decode_glyph, parse_font_face, FontFace, FontLimits, GlyphPixels};
use lbp_runtime::{advance_pixels, FontAdvances, GooeyFont};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// The largest font file read (the disc's largest is 33.7 MB).
const MAX_FONT_BYTES: u64 = 64 << 20;
/// The distance field's rise per font pixel (MEASURED on Eigerdals Black's and Omnes
/// Black's 'S': 0x1c to 0x20 a pixel), the outline value and the width of the soft edge in
/// screen pixels: MEASURED (part 72, the game frame's "Me" against the field rasterized at
/// its projected size): the outline at 125 to 135, the edge about 2 pixels wide (the field
/// over 60 units a pixel at 1 pixel a font pixel).
const FIELD_PER_PIXEL: f32 = 30.;
const FIELD_EDGE: f32 = 130.;
const SOFT_EDGE_PIXELS: f32 = 2.;
/// The largest text image made (a guard).
const MAX_RASTER_PIXELS: usize = 4 << 20;

/// One loaded font face with its bytes (glyphs are decoded when text is rasterized).
pub struct GameFont {
    bytes: Vec<u8>,
    face: FontFace,
    by_char: HashMap<char, usize>,
    /// The ink height of 'H' in font pixels (the cap height), where the font has one.
    cap: Option<f32>,
}

/// A text rasterized at one size: white, its coverage in alpha, with the text's advance
/// centred across and its cap height centred down the image.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRaster {
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
}

impl GameFont {
    pub fn parse(bytes: Vec<u8>) -> Result<Self, String> {
        let face = parse_font_face(&bytes, FontLimits::default()).map_err(|e| e.to_string())?;
        let by_char = face
            .glyphs
            .iter()
            .enumerate()
            .filter_map(|(i, g)| Some((char::from_u32(u32::from(g.character))?, i)))
            .collect();
        let mut font = Self {
            bytes,
            face,
            by_char,
            cap: None,
        };
        font.cap = font.field('H').and_then(|(_, _, w, h, field)| {
            let rows = (0..h)
                .filter(|y| (0..w).any(|x| field[y * w + x] >= FIELD_EDGE as u8))
                .count();
            (rows > 0).then_some(rows as f32)
        });
        Ok(font)
    }

    /// The advances the layout measures text with.
    pub fn advances(&self) -> FontAdvances {
        FontAdvances::new(self.face.glyphs.iter().map(|g| (g.character, g.advance)))
    }

    /// A glyph's box (left, top above the baseline, width, height) and distance field.
    #[allow(clippy::type_complexity)]
    fn field(&self, c: char) -> Option<(i32, i32, usize, usize, Vec<u8>)> {
        let g = &self.face.glyphs[*self.by_char.get(&c)?];
        let bitmap = decode_glyph(&self.bytes, &self.face, g).ok()?;
        let field = match bitmap.pixels {
            GlyphPixels::Coverage(v) => v,
            GlyphPixels::TwoChannel(v) => v.iter().map(|p| p[0]).collect(),
        };
        Some((
            i32::from(g.box_left),
            i32::from(g.box_top),
            usize::from(g.box_width),
            usize::from(g.box_height),
            field,
        ))
    }

    /// Rasterizes `text` at `k` image pixels a font pixel (pen advances rounded up to
    /// whole font pixels, as the layout measures them). `None` for an empty text, a
    /// character the font lacks or a size out of range.
    pub fn raster(&self, text: &str, k: f32) -> Option<TextRaster> {
        if !(k.is_finite() && k > 0.01) || text.is_empty() {
            return None;
        }
        let mut glyphs = Vec::new();
        let mut pen = 0.;
        for c in text.chars() {
            let index = *self.by_char.get(&c)?;
            let (left, top, w, h, field) = self.field(c)?;
            glyphs.push((pen + left as f32, -top as f32, w, h, field));
            pen += advance_pixels(self.face.glyphs[index].advance);
        }
        // Font space: x right from the pen's start, y down from the baseline.
        let centre_x = pen / 2.;
        let centre_y = -self.cap.unwrap_or(0.) / 2.;
        let (mut half_w, mut half_h) = (pen / 2., 1f32);
        for (x, y, w, h, _) in &glyphs {
            half_w = half_w.max(centre_x - x).max(x + *w as f32 - centre_x);
            half_h = half_h.max(centre_y - y).max(y + *h as f32 - centre_y);
        }
        let width = (2. * half_w * k).ceil() as usize;
        let height = (2. * half_h * k).ceil() as usize;
        if width == 0 || height == 0 || width.checked_mul(height)? > MAX_RASTER_PIXELS {
            return None;
        }
        let (x0, y0) = (
            centre_x - width as f32 / k / 2.,
            centre_y - height as f32 / k / 2.,
        );
        let mut coverage = vec![0f32; width * height];
        let soft = FIELD_PER_PIXEL * SOFT_EDGE_PIXELS / k;
        for (gx, gy, w, h, field) in &glyphs {
            let sample = |u: f32, v: f32| {
                // Bilinear in glyph pixels, 0 outside the box.
                let (iu, iv) = (u.floor(), v.floor());
                let (fu, fv) = (u - iu, v - iv);
                let at = |x: f32, y: f32| {
                    if x < 0. || y < 0. || x >= *w as f32 || y >= *h as f32 {
                        0.
                    } else {
                        f32::from(field[y as usize * w + x as usize])
                    }
                };
                let top = at(iu, iv) * (1. - fu) + at(iu + 1., iv) * fu;
                let bottom = at(iu, iv + 1.) * (1. - fu) + at(iu + 1., iv + 1.) * fu;
                top * (1. - fv) + bottom * fv
            };
            let px0 = (((gx - x0) * k).floor() as isize - 1).max(0) as usize;
            let py0 = (((gy - y0) * k).floor() as isize - 1).max(0) as usize;
            let px1 = (((gx + *w as f32 - x0) * k).ceil() as usize + 1).min(width);
            let py1 = (((gy + *h as f32 - y0) * k).ceil() as usize + 1).min(height);
            for py in py0..py1 {
                let fy = y0 + (py as f32 + 0.5) / k - gy - 0.5;
                for px in px0..px1 {
                    let fx = x0 + (px as f32 + 0.5) / k - gx - 0.5;
                    let c = ((sample(fx, fy) - FIELD_EDGE) / soft + 0.5).clamp(0., 1.);
                    let cell = &mut coverage[py * width + px];
                    *cell = cell.max(c);
                }
            }
        }
        Some(TextRaster {
            width: width as u32,
            height: height as u32,
            alpha: coverage.iter().map(|c| (c * 255.).round() as u8).collect(),
        })
    }
}

impl TextRaster {
    /// White RGBA with the coverage as alpha.
    pub fn rgba(&self) -> Vec<u8> {
        self.alpha
            .iter()
            .flat_map(|&a| [255, 255, 255, a])
            .collect()
    }
}

/// A path under `base` matched one component at a time, ignoring ASCII case.
fn find_ignoring_case(base: &Path, relative: &str) -> Option<PathBuf> {
    let mut at = base.to_path_buf();
    for part in relative.split('/').filter(|p| !p.is_empty()) {
        let exact = at.join(part);
        if exact.exists() {
            at = exact;
            continue;
        }
        at = std::fs::read_dir(&at)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?
            .path();
    }
    Some(at)
}

/// The bytes of a loose file of the game's data at a MAP path, from the first of the
/// `;`-separated game roots that has it at the MAP row's size.
fn read_loose(game: &str, catalog: &Catalog, map_path: &str) -> Result<Vec<u8>, String> {
    let size = catalog
        .snapshot
        .resources
        .iter()
        .find(|r| r.map_row.path.eq_ignore_ascii_case(map_path))
        .map(|r| u64::from(r.map_row.declared_size))
        .ok_or_else(|| format!("{map_path}: no MAP row"))?;
    if size > MAX_FONT_BYTES {
        return Err(format!("{map_path}: {size} bytes is over the limit"));
    }
    for root in game.split(';').filter(|r| !r.is_empty()) {
        for prefix in ["USRDIR", "PS3_GAME/USRDIR"] {
            let Some(path) = find_ignoring_case(Path::new(root), &format!("{prefix}/{map_path}"))
            else {
                continue;
            };
            if std::fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() == size) {
                return std::fs::read(&path).map_err(|e| format!("{map_path}: {e}"));
            }
        }
    }
    Err(format!(
        "{map_path}: not under the game directories (a loose disc file)"
    ))
}

/// Loads the Gooey's fonts that are present; the others are reported.
pub fn load_fonts(game: &str, catalog: &Catalog) -> (BTreeMap<GooeyFont, GameFont>, Vec<String>) {
    let mut fonts = BTreeMap::new();
    let mut missing = Vec::new();
    for font in GooeyFont::ALL {
        match read_loose(game, catalog, font.map_path()).and_then(GameFont::parse) {
            Ok(f) => {
                fonts.insert(font, f);
            }
            Err(e) => missing.push(e),
        }
    }
    (fonts, missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic uncompressed font (revision 0x26e, no compression byte) with one glyph:
    /// 'H' as a 6 x 6 box whose middle 4 x 4 is inside, advance 5.5 font pixels.
    fn font() -> GameFont {
        let mut b = b"FNTb".to_vec();
        b.extend_from_slice(&0x26e_u32.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes()); // glyph index: none
        b.extend_from_slice(&0u32.to_be_bytes()); // pages: none
        b.extend_from_slice(&1u32.to_be_bytes()); // one glyph
        b.extend_from_slice(&u16::from(b'H').to_be_bytes());
        b.extend_from_slice(&352i16.to_be_bytes());
        b.extend_from_slice(&[0, 5, 6, 6]); // left 0, top 5, 6 x 6
        b.extend_from_slice(&0i32.to_be_bytes());
        b.extend_from_slice(&(-1i16).to_be_bytes());
        for y in 0..6 {
            for x in 0..6 {
                let inside = (1..5).contains(&x) && (1..5).contains(&y);
                b.extend_from_slice(&[if inside { 220 } else { 10 }, 0]);
            }
        }
        GameFont::parse(b).expect("font")
    }

    #[test]
    fn d321_text_rasters_from_the_distance_field_centred_on_its_advance() {
        let f = font();
        assert_eq!(f.cap, Some(4.));
        assert_eq!(f.advances().width("HH"), Some(12.));
        assert!(f.raster("Hx", 1.).is_none());
        let one = f.raster("H", 1.).expect("raster");
        // Advance 6 (rounded up) centred: the box spans 0..6, so 6 wide; the cap (rows 1
        // to 4 of the box, 5 above the baseline) centred: 3 up and 3 down.
        assert_eq!((one.width, one.height), (6, 6));
        let at = |r: &TextRaster, x: u32, y: u32| r.alpha[(y * r.width + x) as usize];
        // Inside the outline full, outside empty.
        assert_eq!(at(&one, 2, 4), 255);
        assert_eq!(at(&one, 0, 0), 0);
        // Twice the size: twice the pixels.
        let two = f.raster("H", 2.).expect("raster");
        assert_eq!((two.width, two.height), (12, 12));
        assert_eq!(two.rgba().len(), 12 * 12 * 4);
    }

    #[test]
    fn d321_loose_files_are_found_ignoring_case() {
        let dir = std::env::temp_dir().join(format!("d321-loose-{}", std::process::id()));
        let fonts = dir.join("USRDIR").join("GameData").join("FONTS");
        std::fs::create_dir_all(&fonts).expect("dir");
        std::fs::write(fonts.join("EigerdalsBla.FNT"), b"x").expect("file");
        // (On a case-insensitive file system the path may keep the asked case.)
        let found =
            find_ignoring_case(&dir, "USRDIR/gamedata/fonts/eigerdalsbla.fnt").expect("found");
        assert_eq!(std::fs::read(found).expect("read"), b"x");
        assert_eq!(
            find_ignoring_case(&dir, "USRDIR/gamedata/fonts/none.fnt"),
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
