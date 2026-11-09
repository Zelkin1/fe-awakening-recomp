use super::ops::{distance, gaussian, luminance, median_colour, percentile, resize, Filter, Mask, Plane, Rgba};
use super::text::Face;
use super::titles::{place, search, split_lines, Fit, Search, SCALE, SS};

pub const TRACKINGS: [f32; 6] = [0.0, 0.03, 0.06, 0.1, 0.15, 0.22];
const BANNER: Search = Search { factors: (0.8, 1.121, 0.04), wanted: (0.6, 1.5), stretch: (0.7, 1.3), overshoot: 1.04, lifts: (-1.5, 1.51, 0.5), room: None, edge: 0.0 };

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

fn halo(core: &Plane, sigma: f32, gain: f32, factor: f32) -> Plane {
    if gain <= 0.0 {
        return Plane::new(core.w, core.h);
    }
    gaussian(core, sigma * factor).map(|v| (gain * v).clamp(0.0, 1.0))
}

fn fit_halo(small: &Plane, target: &Plane, intensity: f32) -> (f32, f32) {
    let misfit = |drawn: &Plane| -> f32 { drawn.v.iter().zip(&target.v).map(|(d, t)| (d.clamp(0.0, 1.0) - t).powi(2)).sum() };
    let mut best = (misfit(&small.map(|v| v * intensity)), 0.0, 0.0);
    for sigma in [0.5, 0.8, 1.2, 1.8, 2.5, 3.5, 5.0] {
        let blur = gaussian(small, sigma);
        for gain in [0.3, 0.6, 1.0, 1.5, 2.5, 4.0] {
            let drawn = small.zip(&blur, |c, b| (c + (1.0 - c) * (gain * b).clamp(0.0, 1.0)) * intensity);
            let error = misfit(&drawn);
            if error < best.0 {
                best = (error, sigma, gain);
            }
        }
    }
    (best.1, best.2)
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

fn opaque(level: &Plane, colour: impl Fn(usize, f32) -> [f32; 3]) -> Rgba {
    let mut out = Rgba::new(level.w, level.h);
    for (i, &l) in level.v.iter().enumerate() {
        let c = colour(i, l);
        out.v[i] = [c[0].clamp(0.0, 1.0), c[1].clamp(0.0, 1.0), c[2].clamp(0.0, 1.0), 1.0];
    }
    out
}

pub fn redraw(image: &Rgba, kind: Kind, big: &Plane) -> Rgba {
    let (w, h) = (image.w, image.h);
    let s = SCALE as f32;
    let small = big.down(SCALE);
    if !has_alpha(image) {
        let lum = image.lum();
        let line = percentile(&lum.v, 99.0);
        let bright = Mask { w, h, v: lum.v.iter().map(|&l| l > line).collect() };
        let colour = median_colour(image, &bright).map(|c| rgb(&c)).unwrap_or([1.0; 3]);
        let intensity = luminance(colour);
        if kind == Kind::Glow {
            let mut best = (f32::INFINITY, 0.0, 0.0, 0.0, 0.0);
            let blurs: Vec<(f32, Plane)> = [1.0f32, 1.5, 2.5, 3.5, 5.0].iter().map(|&sigma| (sigma, gaussian(&small, sigma))).collect();
            for soft in [0.0f32, 0.5, 0.8, 1.2] {
                let base = gaussian(&small, soft);
                for weight in [0.7, 0.85, 1.0] {
                    for (sigma, blur) in &blurs {
                        let sigma = *sigma;
                        for gain in [0.2, 0.4, 0.6, 0.9, 1.3] {
                            let error: f32 = (0..base.v.len()).map(|i| ((weight * base.v[i] + gain * blur.v[i]).clamp(0.0, 1.0) * intensity - lum.v[i]).powi(2)).sum();
                            if error < best.0 {
                                best = (error, soft, weight, sigma, gain);
                            }
                        }
                    }
                }
            }
            let (_, soft, weight, sigma, gain) = best;
            let base = gaussian(big, soft * s);
            let blur = gaussian(big, sigma * s);
            let level = base.zip(&blur, |b, g| (weight * b + gain * g).clamp(0.0, 1.0));
            return opaque(&level, |_, l| [l * colour[0], l * colour[1], l * colour[2]]);
        }
        let (sigma, gain) = fit_halo(&small, &lum, intensity);
        if kind == Kind::Stone {
            let core = core(image);
            let outside = Mask { w, h, v: core.v.iter().map(|&c| c <= 0.6).collect() };
            let (_, nearest) = distance(&outside);
            let inside: Vec<[f32; 4]> = nearest.iter().map(|&(x, y)| image.at(x, y)).collect();
            let channel = |k: usize| resize(&Plane { w, h, v: inside.iter().map(|p| p[k]).collect() }, w * SCALE, h * SCALE, Filter::Bicubic);
            let texture = [channel(0), channel(1), channel(2)];
            let ring = halo(big, sigma, gain, s);
            return opaque(big, |i, b| {
                let g = ring.v[i];
                [b * texture[0].v[i] + (1.0 - b) * g * colour[0], b * texture[1].v[i] + (1.0 - b) * g * colour[1], b * texture[2].v[i] + (1.0 - b) * g * colour[2]]
            });
        }
        let ring = halo(big, sigma, gain, s);
        return opaque(big, |i, b| {
            let l = b + (1.0 - b) * ring.v[i];
            [l * colour[0], l * colour[1], l * colour[2]]
        });
    }
    let (fill, rim) = colours(image);
    let alpha = image.channel(3);
    let away = distance(&Mask { w: big.w, h: big.h, v: big.v.iter().map(|&v| v < 0.5).collect() }).0.map(|d| d / s);
    let mut best = (f32::INFINITY, 0.0, 0.0, 0.0);
    let blurs: Vec<(f32, Plane)> = [0.5f32, 0.8, 1.2, 1.8, 2.5, 3.5, 5.0].iter().map(|&sigma| (sigma, gaussian(&small, sigma))).collect();
    for radius in [0.0f32, 0.5, 0.75, 1.0, 1.25, 1.5] {
        let outline = if radius > 0.0 { away.map(|d| if d <= radius { 1.0 } else { 0.0 }).down(SCALE) } else { Plane::new(w, h) };
        for (sigma, blur) in &blurs {
            let sigma = *sigma;
            for gain in [0.0f32, 0.3, 0.6, 1.0, 1.5, 2.5] {
                let mut misfit = 0.0f32;
                for i in 0..small.v.len() {
                    let (c, ring) = (small.v[i], outline.v[i].max((gain * blur.v[i]).clamp(0.0, 1.0)));
                    let drawn = c + (1.0 - c) * ring;
                    misfit += (drawn - alpha.v[i]).powi(2);
                    let p = image.v[i];
                    for k in 0..3 {
                        misfit += (c * fill[k] + (1.0 - c) * ring * rim[k] - p[k] * p[3]).powi(2);
                    }
                }
                if misfit < best.0 {
                    best = (misfit, radius, sigma, gain);
                }
            }
        }
    }
    let (_, radius, sigma, gain) = best;
    let outline = if radius > 0.0 { away.map(|d| if d <= radius { 1.0 } else { 0.0 }) } else { Plane::new(big.w, big.h) };
    let glow = halo(big, sigma, gain, s);
    let mut out = Rgba::new(big.w, big.h);
    for i in 0..big.v.len() {
        let (c, ring) = (big.v[i], outline.v[i].max(glow.v[i]));
        let a = c + (1.0 - c) * ring;
        let mut px = [0.0; 4];
        if a > 1e-4 {
            for k in 0..3 {
                px[k] = ((c * fill[k] + (1.0 - c) * ring * rim[k]) / a).clamp(0.0, 1.0);
            }
        }
        px[3] = a.clamp(0.0, 1.0);
        out.v[i] = px;
    }
    out
}
