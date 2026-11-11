//! Draws lines of text with a font.
//! A face is a font at a size, with options for the digits and for a slant.

use ab_glyph::{point, Font, FontArc, GlyphId, PxScale, ScaleFont};

use super::ops::Plane;

/// The most side bearing, as a fraction of the em, that is trimmed from each side of a letter.
const TRIM: f32 = 0.02;
/// The part of the side bearing, as a fraction of the em, that is always kept.
const MIN_BEARING: f32 = 0.025;
/// The extra width of a space, as a fraction of the em.
const WORD_SPACING: f32 = 0.06;

/// A font at a size.
#[derive(Clone)]
pub struct Face {
    pub font: FontArc,
    /// The size of the em in pixels.
    pub em: f32,
    /// The glyph id of the first lining figure, when the digits are drawn from the lining figures.
    figures: Option<u16>,
    /// The shear factor of the letters, where 0 is upright.
    slant: f32,
}

/// A glyph drawn into a plane, with the position of the plane.
pub struct Raster {
    pub plane: Plane,
    /// The offset of the left edge of the plane from the pen.
    pub left: f32,
    /// The offset of the top edge of the plane from the baseline.
    pub top: f32,
}

impl Face {
    /// Creates an upright face at a size.
    pub fn new(font: FontArc, em: f32) -> Face {
        Face { font, em, figures: None, slant: 0.0 }
    }

    /// Returns a copy of the face that slants its letters.
    pub fn slanted(&self, slant: f32) -> Face {
        Face { slant, ..self.clone() }
    }

    /// Returns a face that draws the digits from the lining figures, which start at the glyph id `first`.
    pub fn with_figures(self, first: u16) -> Face {
        Face { figures: Some(first), ..self }
    }

    /// Returns a copy of the face at another size.
    pub fn sized(&self, em: f32) -> Face {
        Face { em, ..self.clone() }
    }

    /// The scale for `ab_glyph`, which sizes the height of the font and not the em.
    /// It is corrected so that the em has the size of the face.
    fn scale(&self) -> PxScale {
        let units = self.font.units_per_em().unwrap_or(1000.0);
        PxScale::from(self.em * self.font.height_unscaled() / units)
    }

    /// Tells whether the font has a drawable glyph for the character.
    pub fn has(&self, c: char) -> bool {
        let id = self.id(c);
        // The glyph id 0 is the glyph that stands for a missing character.
        id.0 != 0 && self.font.outline(id).is_some()
    }

    /// Returns the glyph of a character, taking the lining figures into account for the digits.
    fn id(&self, c: char) -> GlyphId {
        match (self.figures, c.to_digit(10)) {
            (Some(first), Some(digit)) => GlyphId(first + digit as u16),
            _ => self.font.glyph_id(c),
        }
    }

    /// The horizontal advance of a character in pixels.
    pub fn advance(&self, c: char) -> f32 {
        self.font.as_scaled(self.scale()).h_advance(self.id(c))
    }

    /// Returns the tightened advance of a character and the shift of its left edge.
    /// Large side bearings are trimmed, which makes the text denser, and spaces are widened a little.
    pub fn spaced(&self, c: char) -> (f32, f32) {
        let advance = self.advance(c);
        if c.is_whitespace() {
            return (advance + (WORD_SPACING + 2.0 * TRIM) * self.em, 0.0);
        }
        let Some(outline) = self.font.outline(self.id(c)) else { return (advance, 0.0) };
        // The conversion from font units to pixels.
        let per = self.em / self.font.units_per_em().unwrap_or(1000.0);
        // Only the part of a bearing above the minimum is trimmed, and by at most the trim limit.
        let trim = |bearing: f32| (bearing - MIN_BEARING * self.em).clamp(0.0, TRIM * self.em);
        let (left, right) = (trim(outline.bounds.min.x * per), trim(advance - outline.bounds.max.x * per));
        (advance - left - right, left)
    }

    /// The kerning between two characters in pixels.
    pub fn kern(&self, a: char, b: char) -> f32 {
        self.font.as_scaled(self.scale()).kern(self.id(a), self.id(b))
    }

    /// Draws a character at the origin.
    pub fn glyph(&self, c: char) -> Option<Raster> {
        let (plane, left, top) = self.placed(c, 0.0, 0.0)?;
        Some(Raster { plane, left: left as f32, top: top as f32 })
    }

    /// Draws a character with its pen at `(x, y)`.
    /// Returns the plane of coverage and the integer position of its top left corner, or `None` for a glyph without an outline.
    pub fn placed(&self, c: char, x: f32, y: f32) -> Option<(Plane, i32, i32)> {
        let glyph = self.id(c).with_scale_and_position(self.scale(), point(x, y));
        let outlined = self.font.outline_glyph(glyph)?;
        let bounds = outlined.px_bounds();
        let (w, h) = ((bounds.width() as usize).max(1), (bounds.height() as usize).max(1));
        let mut plane = Plane::new(w, h);
        outlined.draw(|px, py, coverage| {
            let (px, py) = (px as usize, py as usize);
            if px < w && py < h {
                plane.v[py * w + px] = coverage.min(1.0);
            }
        });
        Some((plane, bounds.min.x as i32, bounds.min.y as i32))
    }

    /// Returns the box that the ink of a character covers, as left, top, right and bottom, with the pen at the origin.
    pub fn ink_box(&self, c: char) -> Option<(f32, f32, f32, f32)> {
        let glyph = self.id(c).with_scale_and_position(self.scale(), point(0.0, 0.0));
        let b = self.font.outline_glyph(glyph)?.px_bounds();
        Some((b.min.x, b.min.y, b.max.x, b.max.y))
    }

    /// Draws a line of text into a plane that is cropped to the ink.
    /// The tracking is extra space between letters, as a fraction of the em.
    /// Returns `None` when no character has an outline.
    pub fn line(&self, text: &str, tracking: f32) -> Option<Plane> {
        let chars: Vec<char> = text.chars().collect();
        let mut pens = Vec::with_capacity(chars.len());
        let mut pen = 0.0f32;
        for (i, &c) in chars.iter().enumerate() {
            if i > 0 {
                pen += self.kern(chars[i - 1], c) + tracking * self.em;
            }
            let (advance, shift) = self.spaced(c);
            pens.push(pen - shift);
            pen += advance;
        }
        let rasters: Vec<(f32, Raster)> = chars.iter().zip(&pens).filter_map(|(&c, &x)| Some((x, self.glyph(c)?))).collect();
        if rasters.is_empty() {
            return None;
        }
        // The bounds of all the glyphs together, to size the plane of the line.
        let x0 = rasters.iter().map(|(x, r)| x + r.left).fold(f32::INFINITY, f32::min).floor();
        let y0 = rasters.iter().map(|(_, r)| r.top).fold(f32::INFINITY, f32::min).floor();
        let x1 = rasters.iter().map(|(x, r)| x + r.left + r.plane.w as f32).fold(f32::NEG_INFINITY, f32::max).ceil();
        let y1 = rasters.iter().map(|(_, r)| r.top + r.plane.h as f32).fold(f32::NEG_INFINITY, f32::max).ceil();
        let mut out = Plane::new((x1 - x0) as usize + 1, (y1 - y0) as usize + 1);
        // The glyphs are combined with a maximum, so that overlapping letters do not get darker.
        for (x, r) in &rasters {
            let ox = (x + r.left - x0).round() as usize;
            let oy = (r.top - y0).round() as usize;
            for yy in 0..r.plane.h {
                for xx in 0..r.plane.w {
                    let (tx, ty) = (ox + xx, oy + yy);
                    if tx < out.w && ty < out.h {
                        let v = &mut out.v[ty * out.w + tx];
                        *v = v.max(r.plane.v[yy * r.plane.w + xx]);
                    }
                }
            }
        }
        let out = if self.slant > 0.0 { sheared(&out, self.slant) } else { out };
        // Crop to the pixels that are at least half covered.
        let (bx0, bx1, by0, by1) = out.bbox(0.5)?;
        Some(out.crop(bx0, by0, bx1, by1))
    }
}

/// Shears a plane to the right, more towards the top, so that the letters lean forward.
/// The bottom row stays in place, and fractional shifts are shared between two pixels.
fn sheared(p: &Plane, slant: f32) -> Plane {
    let extra = (p.h as f32 * slant).ceil() as usize;
    let mut out = Plane::new(p.w + extra, p.h);
    for y in 0..p.h {
        let shift = (p.h - 1 - y) as f32 * slant;
        let (whole, part) = (shift.floor() as usize, shift.fract());
        for x in 0..p.w {
            let v = p.at(x, y);
            out.v[y * out.w + x + whole] += v * (1.0 - part);
            if x + whole + 1 < out.w {
                out.v[y * out.w + x + whole + 1] += v * part;
            }
        }
    }
    out
}
