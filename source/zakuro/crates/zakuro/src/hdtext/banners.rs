use super::ops::{distance, gaussian, median_colour, median_of, percentile, resize, Filter, Mask, Plane, Rgba};
use super::text::Face;
use super::titles::{place, search, split_lines, Fit, Search, SCALE, SS};

pub const TRACKINGS: [f32; 3] = [0.0, 0.03, 0.06];
const BANNER: Search = Search { factors: (0.8, 1.121, 0.04), wanted: (0.6, 1.5), stretch: (0.7, 1.3), overshoot: 1.04, lifts: (-1.5, 1.51, 0.5), room: None, edge: 0.0 };

#[derive(Clone, Copy)]
pub struct Tone {
    pub share: f32,
    pub grey: f32,
}

pub const VIVID: Tone = Tone { share: 0.5, grey: 1.0 };

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Plain,
    Glow,
    Stone,
}

fn has_alpha(image: &Rgba) -> bool {
    image.v.iter().map(|p| p[3]).fold(1.0f32, f32::min) < 0.98
}

fn rgb(p: &[f32; 4]) -> [f32; 3] {
    [p[0], p[1], p[2]]
}

fn colours(image: &Rgba) -> ([f32; 3], [f32; 3]) {
    let solid = Mask { w: image.w, h: image.h, v: image.v.iter().map(|p| p[3] > 0.85).collect() };
    let edge = Mask { w: image.w, h: image.h, v: image.v.iter().map(|p| p[3] > 0.15 && p[3] < 0.6).collect() };
    let fill = if solid.count() > 10 { median_colour(image, &solid).map(|c| rgb(&c)).unwrap_or([1.0; 3]) } else { [1.0; 3] };
    let rim = if edge.count() > 10 { median_colour(image, &edge).map(|c| rgb(&c)).unwrap_or(fill) } else { fill };
    (fill, rim)
}

pub fn core(image: &Rgba) -> Plane {
    if !has_alpha(image) {
        let lum = image.lum();
        let top = percentile(&lum.v, 99.5).max(1e-3);
        return lum.map(|v| (v / top).clamp(0.0, 1.0));
    }
    let (fill, rim) = colours(image);
    let direction = [fill[0] - rim[0], fill[1] - rim[1], fill[2] - rim[2]];
    let length: f32 = direction.iter().map(|d| d * d).sum();
    if length < 1e-3 {
        return image.channel(3);
    }
    Plane {
        w: image.w,
        h: image.h,
        v: image.v.iter().map(|p| p[3] * ((0..3).map(|k| (p[k] - rim[k]) * direction[k]).sum::<f32>() / length).clamp(0.0, 1.0)).collect(),
    }
}

pub fn layout(face: &Face, text: &str, core: &Plane, trackings: &[f32]) -> Option<Plane> {
    let spans: Vec<(usize, usize)> = split_lines(core, 0.3).into_iter().filter(|s| s.1 - s.0 >= 3).collect();
    let (y0, y1) = (spans.first()?.0, spans.last()?.1);
    let (x0, x1, by0, by1) = core.crop(0, y0, core.w, y1).bbox(0.4)?;
    let b = (x0, x1, by0 + y0, by1 + y0);
    let probe = face.sized(40.0 * SS as f32);
    let mut best: Option<(Fit, Plane)> = None;
    for &tracking in trackings {
        let Some(mask) = probe.line(text, tracking) else { continue };
        let Some(fit) = search(&mask, core, (0, core.h), b, &BANNER) else { continue };
        if best.as_ref().is_none_or(|(f, _)| fit.error < f.error) {
            best = Some((fit, mask));
        }
    }
    let (fit, mask) = best?;
    place(&mask, core, b, &fit, SCALE, BANNER.edge)
}

const BIN: f32 = 0.25;
const NEAR: f32 = -4.0;
const FAR: f32 = 16.0;
const HOLD: f32 = 0.75;

fn signed_distance(inside: &Mask) -> Plane {
    let outside = distance(&inside.not()).0;
    let within = distance(inside).0;
    outside.zip(&within, |o, i| (o - i) / SCALE as f32)
}

fn premultiplied(p: [f32; 4]) -> [f32; 4] {
    [p[0] * p[3], p[1] * p[3], p[2] * p[3], p[3]]
}

fn median4(samples: &[[f32; 4]]) -> [f32; 4] {
    [0, 1, 2, 3].map(|k| median_of(&mut samples.iter().map(|p| p[k]).collect::<Vec<f32>>()))
}

struct Profile {
    bins: Vec<[f32; 4]>,
    fill: [f32; 4],
}

impl Profile {
    fn measure(image: &Rgba, letters: &Plane, tone: Tone, bloom: bool) -> Profile {
        let edge = if bloom { 0.85 } else { 0.5 };
        let sdf = signed_distance(&Mask::from(letters, |v| v >= edge));
        let count = ((FAR - NEAR) / BIN) as usize + 1;
        let mut samples: Vec<Vec<[f32; 4]>> = vec![Vec::new(); count];
        let mut inside = Vec::new();
        for y in 0..image.h {
            for x in 0..image.w {
                let d = sdf.at(x * SCALE + SCALE / 2, y * SCALE + SCALE / 2);
                let p = premultiplied(image.at(x, y));
                samples[(((d - NEAR) / BIN).round().max(0.0) as usize).min(count - 1)].push(p);
                if d < 0.0 {
                    inside.push(p);
                }
            }
        }
        let mut bins: Vec<Option<[f32; 4]>> = samples.iter().map(|s| (!s.is_empty()).then(|| median4(s))).collect();
        for i in 1..count {
            if bins[i].is_none() {
                bins[i] = bins[i - 1];
            }
        }
        for i in (0..count - 1).rev() {
            if bins[i].is_none() {
                bins[i] = bins[i + 1];
            }
        }
        let bins: Vec<[f32; 4]> = bins.into_iter().map(|b| b.unwrap_or([0.0; 4])).collect();
        let edge = bins[((1.0 - NEAR) / BIN) as usize];
        let contrast = |p: &[f32; 4]| (0..4).map(|k| (p[k] - edge[k]).abs()).sum::<f32>();
        inside.sort_by(|a, b| contrast(b).total_cmp(&contrast(a)));
        let chosen = &inside[..((inside.len() as f32 * tone.share).ceil() as usize).min(inside.len())];
        let median = if chosen.is_empty() { bins[0] } else { median4(chosen) };
        let fill = [median[0] * tone.grey, median[1] * tone.grey, median[2] * tone.grey, median[3]];
        Profile { bins, fill }
    }

    fn at(&self, d: f32) -> [f32; 4] {
        let t = ((d - NEAR) / BIN).clamp(0.0, (self.bins.len() - 1) as f32);
        let (i, f) = (t.floor() as usize, t.fract());
        let (a, b) = (self.bins[i], self.bins[(i + 1).min(self.bins.len() - 1)]);
        [0, 1, 2, 3].map(|k| a[k] + (b[k] - a[k]) * f)
    }
}

fn stone_texture(image: &Rgba, big: &Plane) -> [Plane; 3] {
    let (w, h) = (image.w, image.h);
    let letters = core(image);
    let outside = Mask::from(&letters, |c| c <= 0.6);
    let (_, nearest) = distance(&outside);
    let inside: Vec<[f32; 4]> = nearest.iter().map(|&(x, y)| image.at(x, y)).collect();
    let texture = [0, 1, 2].map(|k| resize(&Plane { w, h, v: inside.iter().map(|p| p[k]).collect() }, w * SCALE, h * SCALE, Filter::Bicubic));
    let lum = |r: f32, g: f32, b: f32| r * 0.299 + g * 0.587 + b * 0.114;
    let mut wanted: Vec<f32> = (0..w * h).filter(|&i| !outside.v[i]).map(|i| image.v[i]).map(|p| lum(p[0], p[1], p[2])).collect();
    let mut drawn: Vec<f32> = (0..big.v.len()).filter(|&i| big.v[i] > 0.5).map(|i| lum(texture[0].v[i], texture[1].v[i], texture[2].v[i])).collect();
    let gain = (median_of(&mut wanted) / median_of(&mut drawn).max(1e-3)).clamp(1.0, 1.5);
    texture.map(|t| t.map(|v| (v * gain).clamp(0.0, 1.0)))
}

fn soft(v: f32) -> f32 {
    1.0 - (-v).exp()
}

fn fit_bloom(image: &Rgba, small: &Plane, fill: [f32; 4]) -> (f32, f32) {
    let target = image.lum();
    let bright = fill[0] * 0.299 + fill[1] * 0.587 + fill[2] * 0.114;
    let mut best = (f32::INFINITY, 2.0, 1.0);
    let outside: Vec<usize> = (0..small.v.len()).filter(|&i| small.v[i] < 0.05).collect();
    for sigma in [2.0f32, 2.5, 3.0, 3.5, 4.0, 5.0, 6.0] {
        let blur = gaussian(small, sigma);
        for step in 1..=24 {
            let gain = step as f32 * 0.25;
            let error: f32 = outside.iter().map(|&i| (soft(gain * blur.v[i]) * bright - target.v[i]).powi(2)).sum();
            if error < best.0 {
                best = (error, sigma, gain);
            }
        }
    }
    (best.1, best.2)
}

pub fn redraw(image: &Rgba, letters: &Plane, kind: Kind, tone: Tone, big: &Plane) -> Rgba {
    let letters = resize(letters, image.w * SCALE, image.h * SCALE, Filter::Bicubic);
    let profile = Profile::measure(image, &letters, tone, kind == Kind::Glow);
    let ours = signed_distance(&Mask::from(big, |v| v >= 0.5));
    let texture = (kind == Kind::Stone).then(|| stone_texture(image, big));
    let mut halo: [Plane; 4] = [0, 1, 2, 3].map(|_| Plane::new(big.w, big.h));
    for (i, d) in ours.v.iter().enumerate() {
        let p = profile.at(d.max(HOLD));
        for k in 0..4 {
            halo[k].v[i] = p[k];
        }
    }
    if kind == Kind::Glow {
        let (sigma, gain) = fit_bloom(image, &big.down(SCALE), profile.fill);
        let bloom = gaussian(big, sigma * SCALE as f32);
        for (plane, colour) in halo.iter_mut().zip(profile.fill).take(3) {
            for (v, b) in plane.v.iter_mut().zip(&bloom.v) {
                *v = soft(gain * b) * colour;
            }
        }
    }
    let mut out = Rgba::new(big.w, big.h);
    for i in 0..big.v.len() {
        let c = big.v[i];
        let halo = [halo[0].v[i], halo[1].v[i], halo[2].v[i], halo[3].v[i]];
        let fill = texture.as_ref().map_or(profile.fill, |t| [t[0].v[i], t[1].v[i], t[2].v[i], 1.0]);
        let p: [f32; 4] = [0, 1, 2, 3].map(|k| c * fill[k] + (1.0 - c) * halo[k]);
        let a = p[3].clamp(0.0, 1.0);
        out.v[i] = if a > 1e-4 { [(p[0] / a).clamp(0.0, 1.0), (p[1] / a).clamp(0.0, 1.0), (p[2] / a).clamp(0.0, 1.0), a] } else { [0.0; 4] };
    }
    out
}
