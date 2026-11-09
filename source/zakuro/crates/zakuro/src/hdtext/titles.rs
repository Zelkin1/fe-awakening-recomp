use std::collections::HashMap;

use super::ops::{luminance, median_colour, resize, Filter, Mask, Plane, Rgba};
use super::text::Face;

pub const SS: usize = 8;
pub const SCALE: usize = 4;
const SEARCH: usize = 4;

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

pub struct Search {
    pub factors: (f32, f32, f32),
    pub wanted: (f32, f32),
    pub stretch: (f32, f32),
    pub overshoot: f32,
    pub lifts: (f32, f32, f32),
    pub room: Option<f32>,
    pub edge: f32,
}

pub const TITLE: Search = Search { factors: (0.72, 1.121, 0.03), wanted: (0.7, 1.45), stretch: (0.8, 1.2), overshoot: f32::INFINITY, lifts: (-2.0, 2.01, 0.5), room: Some(1.5), edge: 1.0 };

pub struct Fit {
    pub error: f32,
    pub scale: f32,
    pub stretch: f32,
    pub dy: f32,
}

fn steps((start, end, step): (f32, f32, f32)) -> Vec<f32> {
    let mut out = Vec::new();
    let mut v = start;
    while v <= end {
        out.push(v);
        v += step;
    }
    out
}

fn origin(core: &Plane, b: (usize, usize, usize, usize), size: (usize, usize), dy: f32, ss: usize) -> Option<(usize, usize)> {
    let (x0, x1, by0, by1) = b;
    let (sw, sh) = size;
    let ox = (((x0 + x1) as f32 / 2.0 * ss as f32 - sw as f32 / 2.0).round() as isize).clamp(0, (core.w * ss) as isize - sw as isize);
    let oy = (((by0 + by1) as f32 / 2.0 + dy) * ss as f32 - sh as f32 / 2.0).round() as isize;
    (oy >= 0 && oy as usize + sh <= core.h * ss && ox >= 0).then_some((ox as usize, oy as usize))
}

pub fn search(mask: &Plane, core: &Plane, rows: (usize, usize), b: (usize, usize, usize, usize), s: &Search) -> Option<Fit> {
    let (r0, r1) = rows;
    let base = (b.3 - b.2) as f32 * SS as f32 / mask.h as f32;
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
            let norm = 1.0 / (SEARCH * SEARCH) as f32;
            let (mut dot, mut energy) = (0.0f32, target_energy);
            for (g, t) in grid.iter().zip(&target.v) {
                let g = g * norm;
                dot += g * t;
                energy += g * g;
            }
            let error = 1.0 - 2.0 * dot / energy.max(1e-6);
            if best.as_ref().is_none_or(|f| error < f.error) {
                best = Some(Fit { error, scale, stretch, dy });
            }
        }
    }
    best
}

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

fn core_of(image: &Rgba, glow: [f32; 3]) -> Plane {
    let glow_lum = luminance(glow).max(1e-3);
    Plane { w: image.w, h: image.h, v: image.v.iter().map(|p| (p[3] * (1.0 - luminance([p[0], p[1], p[2]]) / glow_lum)).clamp(0.0, 1.0)).collect() }
}

fn glow_colour(image: &Rgba) -> [f32; 3] {
    let halo = Mask { w: image.w, h: image.h, v: image.v.iter().map(|p| p[3] > 0.2 && p[3] < 0.8).collect() };
    if halo.count() > 20 { median_colour(image, &halo).map(|c| [c[0], c[1], c[2]]).unwrap_or([1.0; 3]) } else { [1.0; 3] }
}

pub fn redraw(image: &Rgba, texts: &[String], face: &Face) -> Option<Rgba> {
    let glow = glow_colour(image);
    let core = core_of(image, glow);
    let alpha = image.channel(3);
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
        for (b, f) in big.v.iter_mut().zip(&canvas.v) {
            *b = b.max(*f);
        }
    }
    Some(super::banners::redraw(image, &core, super::banners::Kind::Plain, super::banners::Tone { share: 1.0, grey: 1.0 }, &big))
}

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
        return Some(vec![first, if lang == 'I' { place } else { format!("- {place} -") }]);
    }
    let mut first = messages.get(&format!("MCID_{rest}_T"))?.clone();
    let mut second = messages.get(&format!("MCID_{rest}"))?.clone();
    if let Some((_, _, n)) = names.iter().find(|(l, n, _)| *l == lang && *n == rest) {
        second = n.to_string();
    }
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
