//! Redraws the chapter title pictures in the new font.
//!
//! This file also holds the fitting search that the banners use, which finds the size, stretch and height at which a line of text matches the old letters best.

use std::collections::HashMap;

use super::ops::{luminance, median_colour, resize, Filter, Mask, Plane, Rgba};
use super::text::Face;

/// The supersampling factor of the text masks, relative to the original picture.
pub const SS: usize = 8;
/// The factor by which the redrawn picture is larger than the original picture.
pub const SCALE: usize = 4;
/// The factor, relative to the original picture, at which the fitting search compares the candidates.
const SEARCH: usize = 4;

/// Finds the lines of text in an alpha plane.
/// Returns the first row and the row after the last for every run of rows that has a value above the threshold.
pub fn split_lines(alpha: &Plane, threshold: f32) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start = None;
    for y in 0..=alpha.h {
        let on = y < alpha.h && alpha.row_max(y) > threshold;
        match (on, start) {
            (true, None) => start = Some(y),
            (false, Some(s)) => {
                spans.push((s, y));
                start = None;
            }
            _ => {}
        }
    }
    spans
}

/// The limits of the fitting search.
pub struct Search {
    /// The start, end and step of the sizes to try, as multiples of the height of the old letters.
    pub factors: (f32, f32, f32),
    /// The smallest and largest horizontal stretch that a size can need to reach the width of the old letters.
    /// The search skips sizes outside this range.
    pub wanted: (f32, f32),
    /// The smallest and largest horizontal stretch that is applied.
    pub stretch: (f32, f32),
    /// The largest multiple of the wanted stretch that is applied.
    pub overshoot: f32,
    /// The start, end and step of the vertical shifts to try, in pixels of the original picture.
    pub lifts: (f32, f32, f32),
    /// The space in pixels that the text must leave to the right when stretched.
    /// `None` sets no limit.
    pub room: Option<f32>,
    /// The space in pixels that the text must leave at the sides of the picture.
    pub edge: f32,
}

/// The limits for the chapter titles.
pub const TITLE: Search = Search { factors: (0.72, 1.121, 0.03), wanted: (0.7, 1.45), stretch: (0.8, 1.2), overshoot: f32::INFINITY, lifts: (-2.0, 2.01, 0.5), room: Some(1.5), edge: 1.0 };

/// The best fit that a search found.
pub struct Fit {
    /// The mismatch with the old letters.
    /// Lower is better.
    pub error: f32,
    /// The size of the text, as a scale of the mask.
    pub scale: f32,
    /// The horizontal stretch on top of the scale.
    pub stretch: f32,
    /// The vertical shift in pixels of the original picture.
    pub dy: f32,
}

/// Lists the values from the start up to and including the end in the given step.
fn steps((start, end, step): (f32, f32, f32)) -> Vec<f32> {
    let mut out = Vec::new();
    let mut v = start;
    while v <= end {
        out.push(v);
        v += step;
    }
    out
}

/// Returns the top left corner at which a scaled mask is centred on the old letters, shifted down by `dy`.
/// The result is in pixels at the factor `ss`.
/// Returns `None` when the mask leaves the picture.
fn origin(core: &Plane, b: (usize, usize, usize, usize), size: (usize, usize), dy: f32, ss: usize) -> Option<(usize, usize)> {
    let (x0, x1, by0, by1) = b;
    let (sw, sh) = size;
    let ox = (((x0 + x1) as f32 / 2.0 * ss as f32 - sw as f32 / 2.0).round() as isize).clamp(0, (core.w * ss) as isize - sw as isize);
    let oy = (((by0 + by1) as f32 / 2.0 + dy) * ss as f32 - sh as f32 / 2.0).round() as isize;
    (oy >= 0 && oy as usize + sh <= core.h * ss && ox >= 0).then_some((ox as usize, oy as usize))
}

/// Searches the size, stretch and vertical shift at which the text mask matches the old letters best.
/// The `rows` are the lines of the picture that are compared.
/// `b` is the bounding box of the old letters as `x0, x1, y0, y1`.
pub fn search(mask: &Plane, core: &Plane, rows: (usize, usize), b: (usize, usize, usize, usize), s: &Search) -> Option<Fit> {
    let (r0, r1) = rows;
    // The height of the old letters, as the scale at which the mask has the same height.
    let base = (b.3 - b.2) as f32 * SS as f32 / mask.h as f32;
    // The search resizes the mask once to the largest size that it tries.
    // It then makes the candidates from this smaller copy.
    let top = base * s.factors.1;
    let small_h = ((mask.h as f32 * top * SEARCH as f32 / SS as f32).round() as usize).max(4);
    let small_w = ((mask.w as f32 * small_h as f32 / mask.h as f32).round() as usize).max(1);
    let small = resize(mask, small_w, small_h, Filter::Lanczos);
    let target = core.crop(0, r0, core.w, r1);
    let target_energy: f32 = target.v.iter().map(|t| t * t).sum();
    let mut best: Option<Fit> = None;
    let mut grid = vec![0.0f32; target.v.len()];
    for factor in steps(s.factors) {
        let scale = base * factor;
        let wanted = (b.1 - b.0) as f32 * SS as f32 / (mask.w as f32 * scale);
        if wanted < s.wanted.0 || wanted > s.wanted.1 {
            continue;
        }
        let stretch = wanted.clamp(s.stretch.0, s.stretch.1).min(wanted * s.overshoot).min(s.room.map_or(f32::INFINITY, |room| (core.w as f32 - room) * SS as f32 / (mask.w as f32 * scale)));
        let sw = ((mask.w as f32 * scale * stretch * SEARCH as f32 / SS as f32).round() as usize).max(1);
        let sh = ((mask.h as f32 * scale * SEARCH as f32 / SS as f32).round() as usize).max(1);
        if sw as f32 > (core.w as f32 - s.edge) * SEARCH as f32 {
            continue;
        }
        let fitted = resize(&small, sw, sh, Filter::Lanczos);
        for dy in steps(s.lifts) {
            let Some((ox, oy)) = origin(core, b, (sw, sh), dy, SEARCH) else { continue };
            grid.iter_mut().for_each(|g| *g = 0.0);
            for j in 0..sh {
                let y = (oy + j) / SEARCH;
                if y < r0 || y >= r1 {
                    continue;
                }
                let row = &mut grid[(y - r0) * core.w..(y - r0 + 1) * core.w];
                for (i, v) in fitted.v[j * sw..(j + 1) * sw].iter().enumerate() {
                    row[(ox + i) / SEARCH] += v;
                }
            }
            // The search averages the candidate down to the original pixel grid and then compares it with the old letters.
            let norm = 1.0 / (SEARCH * SEARCH) as f32;
            let (mut dot, mut energy) = (0.0f32, target_energy);
            for (g, t) in grid.iter().zip(&target.v) {
                let g = g * norm;
                dot += g * t;
                energy += g * g;
            }
            // This equals the squared difference divided by the total energy, so zero is a perfect match.
            let error = 1.0 - 2.0 * dot / energy.max(1e-6);
            if best.as_ref().is_none_or(|f| error < f.error) {
                best = Some(Fit { error, scale, stretch, dy });
            }
        }
    }
    best
}

/// Draws the text mask on a canvas of the size of the picture times `ss`.
/// It uses the size, stretch and shift of the fit.
/// Returns `None` when the text is too wide or leaves the picture.
pub fn place(mask: &Plane, core: &Plane, b: (usize, usize, usize, usize), fit: &Fit, ss: usize, edge: f32) -> Option<Plane> {
    let sw = ((mask.w as f32 * fit.scale * fit.stretch * ss as f32 / SS as f32).round() as usize).max(1);
    let sh = ((mask.h as f32 * fit.scale * ss as f32 / SS as f32).round() as usize).max(1);
    if sw as f32 > (core.w as f32 - edge) * ss as f32 {
        return None;
    }
    let (ox, oy) = origin(core, b, (sw, sh), fit.dy, ss)?;
    let fitted = resize(mask, sw, sh, Filter::Lanczos).map(|v| v.clamp(0.0, 1.0));
    let mut canvas = Plane::new(core.w * ss, core.h * ss);
    canvas.paste(&fitted, ox as isize, oy as isize);
    Some(canvas)
}

/// Extracts the bright core of the letters by removing the glow colour from the alpha.
fn core_of(image: &Rgba, glow: [f32; 3]) -> Plane {
    let glow_lum = luminance(glow).max(1e-3);
    Plane { w: image.w, h: image.h, v: image.v.iter().map(|p| (p[3] * (1.0 - luminance([p[0], p[1], p[2]]) / glow_lum)).clamp(0.0, 1.0)).collect() }
}

/// Estimates the colour of the glow as the median colour of the semi-transparent pixels.
/// Falls back to white when there are too few of them.
fn glow_colour(image: &Rgba) -> [f32; 3] {
    let halo = Mask { w: image.w, h: image.h, v: image.v.iter().map(|p| p[3] > 0.2 && p[3] < 0.8).collect() };
    if halo.count() > 20 { median_colour(image, &halo).map(|c| [c[0], c[1], c[2]]).unwrap_or([1.0; 3]) } else { [1.0; 3] }
}

/// Redraws a title picture with one text per line.
/// Returns `None` when the number of lines in the picture differs from the number of texts, or when a line cannot be fitted.
pub fn redraw(image: &Rgba, texts: &[String], face: &Face) -> Option<Rgba> {
    let glow = glow_colour(image);
    let core = core_of(image, glow);
    let alpha = image.channel(3);
    // Runs of fewer than 4 rows are ignored.
    let lines: Vec<(usize, usize)> = split_lines(&alpha, 0.3).into_iter().filter(|s| s.1 - s.0 >= 4).collect();
    if lines.len() != texts.len() {
        return None;
    }
    let probe = face.sized(40.0 * SS as f32);
    let mut big = Plane::new(image.w * SCALE, image.h * SCALE);
    for (text, &(y0, y1)) in texts.iter().zip(&lines) {
        let band = core.crop(0, y0, core.w, y1);
        let (x0, x1, by0, by1) = band.bbox(0.5)?;
        let b = (x0, x1, by0 + y0, by1 + y0);
        let mask = probe.line(text, 0.0)?;
        let fit = search(&mask, &core, (y0, y1), b, &TITLE)?;
        let canvas = place(&mask, &core, b, &fit, SCALE, TITLE.edge)?;
        // The lines are combined by taking the stronger value of each pixel.
        for (b, f) in big.v.iter_mut().zip(&canvas.v) {
            *b = b.max(*f);
        }
    }
    Some(super::banners::redraw(image, &core, super::banners::Kind::Plain, super::banners::Tone { share: 1.0, grey: 1.0 }, &big))
}

/// Looks up the lines of text for the title picture of a chapter.
/// The `name` is the name of the picture, with an optional `B` for an encounter and an optional `P` or `X`, followed by three digits.
/// The `places` and `names` are overrides per language.
/// Returns `None` when the name has another form or a message is missing.
pub fn texts(messages: &HashMap<String, String>, name: &str, lang: char, places: &[(char, &str, &str)], names: &[(char, &str, &str)]) -> Option<Vec<String>> {
    let encounter = name.starts_with('B');
    let rest = name.trim_start_matches('B');
    let digits = rest.trim_start_matches(['P', 'X']);
    if digits.len() != 3 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if encounter {
        let place = places.iter().find(|(l, n, _)| *l == lang && *n == name).map(|(_, _, p)| p.to_string()).or_else(|| messages.get(&format!("MCID_T_{rest}")).cloned())?;
        let first = messages.get("MCID_ENCOUNT_T")?.clone();
        // The place is wrapped in dashes, except in Italian.
        return Some(vec![first, if lang == 'I' { place } else { format!("- {place} -") }]);
    }
    let mut first = messages.get(&format!("MCID_{rest}_T"))?.clone();
    let mut second = messages.get(&format!("MCID_{rest}"))?.clone();
    if let Some((_, _, n)) = names.iter().find(|(l, n, _)| *l == lang && *n == rest) {
        second = n.to_string();
    }
    // The abbreviated prefixes of the Spanish and German messages are written out in full.
    if lang == 'S' {
        if let Some(r) = first.strip_prefix("Cap. ") {
            first = format!("Capítulo {r}");
        }
    }
    if lang == 'G' {
        if let Some(r) = first.strip_prefix("Nq. ") {
            first = format!("Nebenquest {r}");
        }
    }
    Some(vec![first, second])
}
