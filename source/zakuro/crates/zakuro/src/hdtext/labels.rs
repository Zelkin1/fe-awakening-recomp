use std::collections::{HashMap, HashSet};

use super::banners;
use super::data::Label;
use super::ops::{dilate, distance, erode, fill_holes, gaussian, label, largest, luminance, median, median_colour, median_of, percentile, resize, sobel_magnitude, uniform, Filter, Mask, Plane, Rgba, Rng};
use super::text::Face;
use super::titles::{SCALE, SS};

const EDGE: f32 = 0.05;
const SMALLEST: usize = 12;
const TOLERANCE: f32 = 0.07;
const CELL_LIGHT: f32 = 0.8;
const CELL_DARK: f32 = 0.45;
const MARGIN: isize = 2;
const SHADOW: (f32, f32, f32) = (0.6, 0.3, 0.7);
const TRACKINGS: [f32; 3] = [0.0, 0.02, 0.04];

fn rgb(p: [f32; 4]) -> [f32; 3] {
    [p[0], p[1], p[2]]
}

fn mask(w: usize, h: usize, f: impl Fn(usize, usize) -> bool) -> Mask {
    let mut m = Mask::new(w, h);
    for y in 0..h {
        for x in 0..w {
            m.v[y * w + x] = f(x, y);
        }
    }
    m
}

fn rect(w: usize, h: usize, b: (usize, usize, usize, usize)) -> Mask {
    mask(w, h, |x, y| x >= b.0 && x < b.2 && y >= b.1 && y < b.3)
}

fn padded_erode(m: &Mask, pad: usize, iterations: usize, grow: bool) -> Mask {
    let (w, h) = (m.w + 2 * pad, m.h + 2 * pad);
    let big = mask(w, h, |x, y| m.at((x as isize - pad as isize).clamp(0, m.w as isize - 1) as usize, (y as isize - pad as isize).clamp(0, m.h as isize - 1) as usize));
    let done = if grow { fill_holes(&erode(&dilate(&big, iterations), iterations)) } else { erode(&big, iterations) };
    mask(m.w, m.h, |x, y| done.at(x + pad, y + pad))
}

fn interp(x: f32, xs: &[f32], ys: &[f32]) -> f32 {
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[xs.len() - 1] {
        return ys[ys.len() - 1];
    }
    let i = xs.partition_point(|&v| v <= x).min(xs.len() - 1).max(1);
    let (x0, x1, y0, y1) = (xs[i - 1], xs[i], ys[i - 1], ys[i]);
    if x1 == x0 { y0 } else { y0 + (y1 - y0) * (x - x0) / (x1 - x0) }
}

fn pieces(m: &Mask, eight: bool) -> Vec<Vec<usize>> {
    let (labels, count) = label(m, eight);
    let mut out = vec![Vec::new(); count];
    for (i, &l) in labels.iter().enumerate() {
        if l > 0 {
            out[l as usize - 1].push(i);
        }
    }
    out
}

fn span(piece: &[usize], w: usize) -> (usize, usize) {
    let xs = piece.iter().map(|i| i % w);
    let ys = piece.iter().map(|i| i / w);
    let (x0, x1) = (xs.clone().min().unwrap_or(0), xs.max().unwrap_or(0));
    let (y0, y1) = (ys.clone().min().unwrap_or(0), ys.max().unwrap_or(0));
    (x1 - x0, y1 - y0)
}

fn plate_box(region: &Rgba) -> (usize, usize, usize, usize) {
    let lum = region.lum();
    let dark = Mask::from(&lum, |v| v < 0.45);
    let solid = padded_erode(&dark, 3, 2, true);
    largest(&solid).and_then(|m| m.bounds()).unwrap_or((0, 0, region.w, region.h))
}

struct Letters {
    core: Plane,
    colour: [f32; 3],
    profile: Option<Vec<[f32; 3]>>,
}

fn letters(region: &Rgba) -> Option<Letters> {
    let (w, h) = (region.w, region.h);
    let lum = region.lum();
    let inside = rect(w, h, plate_box(region));
    let values: Vec<f32> = lum.v.iter().zip(&inside.v).filter(|(_, &i)| i).map(|(&l, _)| l).collect();
    let (high, low) = (percentile(&values, 80.0), percentile(&values, 45.0));
    let light = mask(w, h, |x, y| inside.at(x, y) && lum.at(x, y) > high);
    let dark = mask(w, h, |x, y| inside.at(x, y) && lum.at(x, y) < low);
    let colour = rgb(median_colour(region, &light)?);
    let plate = rgb(median_colour(region, &dark)?);
    let direction = [colour[0] - plate[0], colour[1] - plate[1], colour[2] - plate[2]];
    let length = direction.iter().map(|d| d * d).sum::<f32>().max(1e-4);
    let mut core = Plane::new(w, h);
    for y in 0..h {
        for x in 0..w {
            if !inside.at(x, y) || x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                continue;
            }
            let p = region.at(x, y);
            let t = ((0..3).map(|k| (p[k] - plate[k]) * direction[k]).sum::<f32>() / length).clamp(0.0, 1.0);
            core.set(x, y, ((t - 0.15) / 0.7).clamp(0.0, 1.0));
        }
    }
    for piece in pieces(&Mask::from(&core, |v| v > 0.3), false) {
        let (sx, sy) = span(&piece, w);
        if (sx <= 1 && sy as f32 >= h as f32 * 0.6) || (sy <= 1 && sx as f32 >= w as f32 * 0.6) {
            for &i in &piece {
                core.v[i] = 0.0;
            }
        }
    }
    let mut rows = Vec::new();
    let mut values: Vec<[f32; 3]> = Vec::new();
    for y in 0..h {
        let strong: Vec<[f32; 4]> = (0..w).filter(|&x| core.at(x, y) > 0.6).map(|x| region.at(x, y)).collect();
        if strong.len() >= 2 {
            let mut c = [0.0; 3];
            for (k, v) in c.iter_mut().enumerate() {
                *v = median_of(&mut strong.iter().map(|p| p[k]).collect::<Vec<f32>>());
            }
            rows.push(y as f32);
            values.push(c);
        }
    }
    let profile = (rows.len() >= 3).then(|| {
        let (top, bottom) = (rows[0], rows[rows.len() - 1]);
        (0..16)
            .map(|i| {
                let at = top + (bottom - top) * i as f32 / 15.0;
                let mut c = [0.0; 3];
                for (k, v) in c.iter_mut().enumerate() {
                    *v = interp(at, &rows, &values.iter().map(|q| q[k]).collect::<Vec<f32>>());
                }
                c
            })
            .collect()
    });
    Some(Letters { core, colour, profile })
}

fn dividers(region: &Rgba, area: &Mask, plain_lum: f32) -> Mask {
    let (w, h) = (region.w, region.h);
    let lum = region.lum();
    let mut lines = Mask::new(w, h);
    let Some((x0, y0, x1, y1)) = area.bounds() else { return lines };
    let off = |v: f32| v < plain_lum - 0.08;
    let on = |v: f32| v > plain_lum + 0.08;
    let mostly = |vals: &[f32], f: &dyn Fn(f32) -> bool| vals.iter().filter(|&&v| f(v)).count() as f32 / vals.len().max(1) as f32 > 0.85;
    let mut rows: Vec<usize> = [y0 as isize, y0 as isize + 1, y1 as isize - 2, y1 as isize - 1].iter().filter(|&&y| y >= y0 as isize && y < y1 as isize).map(|&y| y as usize).collect();
    rows.dedup();
    for y in rows {
        let vals: Vec<f32> = (x0..x1).map(|x| lum.at(x, y)).collect();
        if mostly(&vals, &off) || mostly(&vals, &on) {
            (x0..x1).for_each(|x| lines.set(x, y, true));
        }
    }
    let mut cols: Vec<usize> = [x0 as isize, x0 as isize + 1, x1 as isize - 2, x1 as isize - 1].iter().filter(|&&x| x >= x0 as isize && x < x1 as isize).map(|&x| x as usize).collect();
    cols.dedup();
    for x in cols {
        let vals: Vec<f32> = (y0..y1).map(|y| lum.at(x, y)).collect();
        if mostly(&vals, &off) || mostly(&vals, &on) {
            (y0..y1).for_each(|y| lines.set(x, y, true));
        }
    }
    lines
}

fn plate_colour(region: &Rgba, core: &Plane) -> Option<[f32; 4]> {
    let (w, h) = (region.w, region.h);
    let lum = region.lum();
    let inside = rect(w, h, plate_box(region));
    let mut interior = padded_erode(&inside, 1, 1, false);
    interior = mask(w, h, |x, y| interior.at(x, y) && region.at(x, y)[3] > 0.98 && lum.at(x, y) < 0.5);
    let near = dilate(&Mask::from(core, |v| v > 0.05), 2);
    let away = interior.and(&near.not());
    if away.count() >= 6 {
        interior = away;
    }
    if interior.count() < 4 {
        return None;
    }
    let mean = uniform(&lum, 3);
    let square = uniform(&lum.map(|v| v * v), 3);
    let flat = mask(w, h, |x, y| interior.at(x, y) && (square.at(x, y) - mean.at(x, y).powi(2)).max(0.0).sqrt() < 0.025);
    match largest(&flat) {
        Some(largest) => median_colour(region, &largest),
        None => median_colour(region, &interior),
    }
}

fn without_letters(region: &Rgba, core: &Plane, shared: Option<[f32; 4]>) -> Rgba {
    let (w, h) = (region.w, region.h);
    let inked = Mask::from(core, |v| v > 0.05);
    if !dilate(&inked, 2).any() {
        return region.clone();
    }
    let lum = region.lum();
    let b = plate_box(region);
    let inside = rect(w, h, b);
    let interior = erode(&inside, 1);
    let mut keep = interior.and(&dilate(&inked, 3).not());
    if keep.count() < 4 {
        keep = interior.and(&inked.not());
    }
    let whole = Mask { w, h, v: vec![true; w * h] };
    let plain = shared.or_else(|| plate_colour(region, core)).or_else(|| median_colour(region, if keep.count() > 0 { &keep } else { &whole })).unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let plain_lum = luminance(rgb(plain));
    let mut out = region.clone();
    let lines = dividers(region, &inside, plain_lum);
    let area = inside.and(&lines.not());
    for (i, &a) in area.v.iter().enumerate() {
        if a {
            out.v[i] = plain;
        }
    }
    let mut brighter = mask(w, h, |x, y| lum.at(x, y) - plain_lum > 0.1 && !area.at(x, y));
    for piece in pieces(&brighter.clone(), false) {
        if piece.len() > 40 {
            piece.iter().for_each(|&i| brighter.v[i] = false);
        }
    }
    let near3 = dilate(&inked, 3);
    for i in 0..out.v.len() {
        if brighter.v[i] && near3.v[i] && !area.v[i] {
            out.v[i] = plain;
        }
    }
    let mut darker = mask(w, h, |x, y| plain_lum - lum.at(x, y) > 0.05 && !area.at(x, y) && region.at(x, y)[3] > 0.98);
    for piece in pieces(&darker.clone(), false) {
        let (sx, sy) = span(&piece, w);
        if piece.len() > 12 && (sx >= 8 || sy >= 8) {
            piece.iter().for_each(|&i| darker.v[i] = false);
        }
    }
    let inside_plate = dilate(&area, 2);
    for i in 0..out.v.len() {
        if darker.v[i] && near3.v[i] && inside_plate.v[i] {
            out.v[i] = plain;
        }
    }
    out
}

fn uniform_1d(m: &Mask, size: usize, vertical: bool) -> Plane {
    let (w, h) = (m.w, m.h);
    let n = if vertical { h } else { w };
    let reflect = |i: isize| -> usize {
        let mut i = i;
        loop {
            if i < 0 {
                i = -i - 1;
            } else if i >= n as isize {
                i = 2 * n as isize - i - 1;
            } else {
                return i as usize;
            }
        }
    };
    let mut out = Plane::new(w, h);
    let start = -(size as isize / 2);
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for k in 0..size as isize {
                let (xx, yy) = if vertical { (x, reflect(y as isize + start + k)) } else { (reflect(x as isize + start + k), y) };
                if m.at(xx, yy) {
                    s += 1.0;
                }
            }
            out.set(x, y, s / size as f32);
        }
    }
    out
}

fn dilate_line(m: &Mask, length: usize, vertical: bool) -> Mask {
    if length == 0 {
        return Mask::new(m.w, m.h);
    }
    let low = -(length as isize / 2);
    mask(m.w, m.h, |x, y| {
        (0..length as isize).any(|k| {
            let o = low + k;
            let (xx, yy) = if vertical { (x as isize, y as isize - o) } else { (x as isize - o, y as isize) };
            xx >= 0 && yy >= 0 && (xx as usize) < m.w && (yy as usize) < m.h && m.at(xx as usize, yy as usize)
        })
    })
}

fn button_letters(region: &Rgba, wanted: &str) -> (Plane, [f32; 3], Mask) {
    let (w, h) = (region.w, region.h);
    let lum = region.lum();
    let reference: Vec<f32> = (0..h)
        .map(|y| {
            let mut c = [0.0; 3];
            for (k, v) in c.iter_mut().enumerate() {
                *v = median_of(&mut (0..w).map(|x| region.at(x, y)[k]).collect::<Vec<f32>>());
            }
            luminance(c)
        })
        .collect();
    let mut difference = Plane::new(w, h);
    for (y, r) in reference.iter().enumerate() {
        for x in 0..w {
            difference.set(x, y, lum.at(x, y) - r);
        }
    }
    let (lighter, darker) = (percentile(&difference.v, 97.0), -percentile(&difference.v, 3.0));
    let polarity = match wanted {
        "light" => 1.0,
        "dark" => -1.0,
        _ if lighter >= darker => 1.0,
        _ => -1.0,
    };
    let difference = difference.map(|d| d * polarity);
    let line = if polarity > 0.0 { lighter } else { darker } * 0.5;
    let strong = Mask::from(&difference, |d| d > line);
    let colour = if strong.count() > 3 { median_colour(region, &strong).map(rgb).unwrap_or([1.0; 3]) } else { [1.0; 3] };
    let contrast = if strong.count() > 3 { median_of(&mut difference.v.iter().zip(&strong.v).filter(|(_, &s)| s).map(|(&d, _)| d).collect::<Vec<f32>>()) } else { 0.2 }.max(0.05);
    let mut core = difference.map(|d| ((d / contrast - 0.15) / 0.7).clamp(0.0, 1.0));
    let on = Mask::from(&core, |v| v > 0.2);
    let (half_h, half_w) = ((h as f32 * 0.5) as usize, (w as f32 * 0.5) as usize);
    let tall_avg = uniform_1d(&on, half_h.max(3), true);
    let wide_avg = uniform_1d(&on, half_w.max(3), false);
    let tall = mask(w, h, |x, y| on.at(x, y) && tall_avg.at(x, y) > 0.95);
    let wide = mask(w, h, |x, y| on.at(x, y) && wide_avg.at(x, y) > 0.95);
    let lines = dilate_line(&tall, half_h, true).and(&on).or(&dilate_line(&wide, half_w, false).and(&on));
    let around = dilate(&lines, 1).and(&erode(&on, 1).not());
    for i in 0..core.v.len() {
        if around.v[i] || lines.v[i] {
            core.v[i] = 0.0;
        }
    }
    let filled: Vec<bool> = (0..h).map(|y| (0..w).filter(|&x| core.at(x, y) > 0.3).count() >= 3).collect();
    let mut bands: Vec<(usize, usize)> = Vec::new();
    let mut start = None;
    for y in 0..=h {
        let f = filled.get(y).copied().unwrap_or(false);
        match (f, start) {
            (true, None) => start = Some(y),
            (false, Some(s)) => {
                bands.push((s, y));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(&(b0, b1)) = bands.iter().fold(None, |best: Option<&(usize, usize)>, b| if best.is_none_or(|c| b.1 - b.0 > c.1 - c.0) { Some(b) } else { best }) {
        let (b0, b1) = (b0.saturating_sub(1), (b1 + 1).min(h));
        for y in 0..h {
            if y < b0 || y >= b1 {
                (0..w).for_each(|x| core.set(x, y, 0.0));
            }
        }
    }
    let erase = mask(w, h, |x, y| core.at(x, y) > 0.03 && x >= 2 && x + 2 < w);
    let (labels, _) = label(&Mask::from(&core, |v| v > 0.3), false);
    let mut edges = HashSet::new();
    for x in 0..w {
        edges.insert(labels[x]);
        edges.insert(labels[(h - 1) * w + x]);
    }
    for y in 0..h {
        edges.insert(labels[y * w]);
        edges.insert(labels[y * w + w - 1]);
    }
    for (i, &l) in labels.iter().enumerate() {
        if l > 0 && edges.contains(&l) {
            core.v[i] = 0.0;
        }
    }
    (core, colour, erase)
}

fn box_mean(p: &Plane, radius: usize) -> Plane {
    let (w, h) = (p.w, p.h);
    let size = 2 * radius + 1;
    let r = radius as isize;
    let mut rows = Plane::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let s: f32 = (-r..=r).map(|k| p.at((x as isize + k).clamp(0, w as isize - 1) as usize, y)).sum();
            rows.set(x, y, s / size as f32);
        }
    }
    let mut out = Plane::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let s: f32 = (-r..=r).map(|k| rows.at(x, (y as isize + k).clamp(0, h as isize - 1) as usize)).sum();
            out.set(x, y, s / size as f32);
        }
    }
    out
}

fn guided(channel: &Plane, guide: &Plane, radius: usize, eps: f32) -> Plane {
    let (mean_g, mean_c) = (box_mean(guide, radius), box_mean(channel, radius));
    let covariance = box_mean(&guide.zip(channel, |a, b| a * b), radius).zip(&mean_g.zip(&mean_c, |a, b| a * b), |a, b| a - b);
    let variance = box_mean(&guide.zip(guide, |a, b| a * b), radius).zip(&mean_g, |a, g| a - g * g);
    let a = covariance.zip(&variance, |c, v| c / (v + eps));
    let b = mean_c.zip(&a.zip(&mean_g, |a, g| a * g), |m, ag| m - ag);
    box_mean(&a, radius).zip(guide, |a, g| a * g).zip(&box_mean(&b, radius), |x, y| x + y)
}

fn smeared(region: &Rgba, keep: &Plane, sigma: f32, floor: f32) -> (Plane, [Plane; 4]) {
    let weight = gaussian(keep, sigma);
    let estimate = [0, 1, 2, 3].map(|c| gaussian(&region.channel(c).zip(keep, |v, k| v * k), sigma).zip(&weight, |v, wt| v / wt.max(floor)));
    (weight, estimate)
}

fn button_without_letters(region: &Rgba, core: &Plane, erase: &Mask) -> Rgba {
    let (w, h) = (region.w, region.h);
    if w < 7 || h < 7 {
        return region.clone();
    }
    let face = rect(w, h, (3, 3, w - 3, h - 3));
    let solid = mask(w, h, |x, y| region.at(x, y)[3] > 0.98);
    let grown = dilate(&Mask::from(core, |v| v > 0.03).or(erase), 2);
    let marks = grown.and(&face);
    let keep = face.and(&marks.not()).and(&solid).as_plane();
    let mut filled = region.clone();
    let rim = mask(w, h, |x, y| grown.at(x, y) && !face.at(x, y) && solid.at(x, y) && x >= 2 && x + 2 < w && y >= 2 && y + 2 < h);
    for x in 0..w {
        let bad: Vec<usize> = (0..h).filter(|&y| rim.at(x, y)).collect();
        if bad.is_empty() {
            continue;
        }
        let good: Vec<usize> = (0..h).filter(|&y| !rim.at(x, y) && solid.at(x, y)).collect();
        if good.len() < 2 {
            continue;
        }
        let xs: Vec<f32> = good.iter().map(|&y| y as f32).collect();
        for c in 0..4 {
            let ys: Vec<f32> = good.iter().map(|&y| region.at(x, y)[c]).collect();
            for &y in &bad {
                filled.v[y * w + x][c] = interp(y as f32, &xs, &ys);
            }
        }
    }
    let mut remaining = marks.clone();
    for sigma in [1.0, 1.5, 2.5, 4.0, 6.0] {
        let (weight, estimate) = smeared(region, &keep, sigma, 1e-4);
        for i in 0..filled.v.len() {
            if remaining.v[i] && weight.v[i] > 0.2 {
                filled.v[i] = [estimate[0].v[i], estimate[1].v[i], estimate[2].v[i], estimate[3].v[i]];
                remaining.v[i] = false;
            }
        }
        if !remaining.any() {
            break;
        }
    }
    if remaining.any() {
        let (_, estimate) = smeared(region, &keep, 8.0, 1e-6);
        for i in 0..filled.v.len() {
            if remaining.v[i] {
                filled.v[i] = [estimate[0].v[i], estimate[1].v[i], estimate[2].v[i], estimate[3].v[i]];
            }
        }
    }
    let guide = filled.lum();
    let smooth = [0, 1, 2].map(|c| guided(&filled.channel(c), &guide, 3, 0.003));
    let mut out = filled.clone();
    for y in 2..h - 2 {
        for x in 2..w - 2 {
            let i = y * w + x;
            out.v[i] = [smooth[0].v[i], smooth[1].v[i], smooth[2].v[i], filled.v[i][3]];
        }
    }
    out
}

fn flatten(image: &Rgba, spared: &Mask) -> (Rgba, Plane) {
    let (w, h) = (image.w, image.h);
    let smooth = [0, 1, 2].map(|c| median(&image.channel(c), 3));
    let mut gradient = Plane::new(w, h);
    for s in &smooth {
        let g = sobel_magnitude(s);
        for (d, v) in gradient.v.iter_mut().zip(&g.v) {
            *d = d.max(v / 4.0);
        }
    }
    let opaque = mask(w, h, |x, y| image.at(x, y)[3] > 0.98 && !spared.at(x, y));
    let cells = mask(w, h, |x, y| gradient.at(x, y) < EDGE && opaque.at(x, y));
    let (labels, count) = label(&cells, false);
    let mut flat = image.clone();
    let mut grain = Plane::new(w, h);
    if count == 0 {
        return (flat, grain);
    }
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); count + 1];
    for (i, &l) in labels.iter().enumerate() {
        if l > 0 {
            members[l as usize].push(i);
        }
    }
    let lum = image.lum();
    let mut table = vec![[0.0f32; 3]; count + 1];
    let mut big = vec![false; count + 1];
    let mut spread = vec![0.0f32; count + 1];
    for l in 1..=count {
        let m = &members[l];
        for (k, t) in table[l].iter_mut().enumerate() {
            *t = median_of(&mut m.iter().map(|&i| image.v[i][k]).collect::<Vec<f32>>());
        }
        let level = luminance(table[l]);
        big[l] = m.len() >= SMALLEST && (level > CELL_LIGHT || level < CELL_DARK);
        let mean = m.iter().map(|&i| lum.v[i]).sum::<f32>() / m.len() as f32;
        spread[l] = (m.iter().map(|&i| (lum.v[i] - mean).powi(2)).sum::<f32>() / m.len() as f32).max(0.0).sqrt();
    }
    let mut difference = Plane::new(w, h);
    for i in 0..difference.v.len() {
        let t = table[labels[i] as usize];
        let p = image.v[i];
        let mut d = (0..3).map(|k| (p[k] - t[k]).abs()).fold(0.0f32, f32::max);
        let light_cell = luminance(t) > CELL_LIGHT;
        let darker = lum.v[i] - luminance(t) < 0.0;
        if light_cell && !darker {
            d = 0.0;
        }
        difference.v[i] = d;
    }
    let faint = Mask::from(&difference, |d| d >= TOLERANCE * 0.4);
    let (faint_labels, _) = label(&faint, true);
    let touching: HashSet<u32> = (0..difference.v.len()).filter(|&i| difference.v[i] >= TOLERANCE && faint.v[i]).map(|i| faint_labels[i]).filter(|&l| l > 0).collect();
    for i in 0..flat.v.len() {
        let l = labels[i] as usize;
        let kept = touching.contains(&faint_labels[i]);
        if big[l] && !kept && opaque.v[i] {
            let t = table[l];
            flat.v[i] = [t[0], t[1], t[2], image.v[i][3]];
        }
        if big[l] && opaque.v[i] {
            grain.v[i] = spread[l].min(0.025);
        }
    }
    (flat, grain)
}

fn add_grain(image: &mut Rgba, grain: &Plane, seed: u64) {
    let (w, h) = (image.w, image.h);
    let amount = resize(grain, w, h, Filter::Nearest);
    let mut rng = Rng::new(seed);
    let fine = gaussian(&Plane { w, h, v: (0..w * h).map(|_| rng.normal()).collect() }, 0.8);
    let coarse = gaussian(&Plane { w, h, v: (0..w * h).map(|_| rng.normal()).collect() }, 3.0);
    let std = |p: &Plane| {
        let mean = p.sum() / p.v.len() as f32;
        (p.v.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / p.v.len() as f32).sqrt().max(1e-6)
    };
    let (sf, sc) = (std(&fine), std(&coarse));
    for i in 0..image.v.len() {
        let n = fine.v[i] / sf * 0.6 + coarse.v[i] / sc * 0.2;
        for k in 0..3 {
            image.v[i][k] = (image.v[i][k] * (1.0 + n * amount.v[i])).clamp(0.0, 1.0);
        }
    }
}

fn premultiplied(image: &Rgba) -> [Plane; 4] {
    let a = image.channel(3);
    [0, 1, 2, 3].map(|c| if c == 3 { a.clone() } else { image.channel(c).zip(&a, |v, a| v * a) })
}

fn unpremultiply(channels: &[Plane; 4]) -> Rgba {
    let (w, h) = (channels[3].w, channels[3].h);
    let mut out = Rgba::new(w, h);
    for i in 0..w * h {
        let a = channels[3].v[i].clamp(0.0, 1.0);
        let mut p = [0.0, 0.0, 0.0, a];
        if a > 1e-4 {
            for (k, v) in p.iter_mut().take(3).enumerate() {
                *v = (channels[k].v[i] / a).clamp(0.0, 1.0);
            }
        }
        out.v[i] = p;
    }
    out
}

fn enlarge(image: &Rgba) -> Rgba {
    let pre = premultiplied(image);
    unpremultiply(&pre.map(|c| resize(&c, image.w * SCALE, image.h * SCALE, Filter::Bicubic).map(|v| v.clamp(0.0, 1.0))))
}

pub fn sharp_enlarge(image: &Rgba, squeeze: f32) -> Rgba {
    let axis = |n: usize| -> Vec<(usize, usize, f32)> {
        (0..n * SCALE)
            .map(|i| {
                let u = (i as f32 + 0.5) / SCALE as f32 - 0.5;
                let base = u.floor();
                let f = ((u - base - 0.5) * squeeze + 0.5).clamp(0.0, 1.0);
                let b = base as isize;
                (b.clamp(0, n as isize - 1) as usize, (b + 1).clamp(0, n as isize - 1) as usize, f)
            })
            .collect()
    };
    let (ys, xs) = (axis(image.h), axis(image.w));
    let pre = premultiplied(image);
    let (w, h) = (image.w * SCALE, image.h * SCALE);
    let channels = [0, 1, 2, 3].map(|c| {
        let p = &pre[c];
        let mut out = Plane::new(w, h);
        for (y, &(y0, y1, fy)) in ys.iter().enumerate() {
            for (x, &(x0, x1, fx)) in xs.iter().enumerate() {
                let top = p.at(x0, y0) * (1.0 - fx) + p.at(x1, y0) * fx;
                let bottom = p.at(x0, y1) * (1.0 - fx) + p.at(x1, y1) * fx;
                out.v[y * w + x] = top * (1.0 - fy) + bottom * fy;
            }
        }
        out
    });
    unpremultiply(&channels)
}

fn squeeze_needed(face: &Face, text: &str, core: &Plane) -> f32 {
    let Some((x0, x1, y0, y1)) = core.bbox(0.4) else { return 1.0 };
    let Some(m) = face.sized(40.0 * SS as f32).line(text, 0.0) else { return 1.0 };
    ((x1 - x0) as f32 / (y1 - y0).max(1) as f32) / (m.w as f32 / m.h as f32)
}

fn group(label: &Label) -> &str {
    if label.group.is_empty() { label.text } else { label.group }
}

struct Drawn {
    x0: usize,
    y0: usize,
    mask: Plane,
    colour: Vec<[f32; 3]>,
    shadowed: bool,
    engraved: Option<[f32; 3]>,
    inside: Mask,
    size: String,
}

fn kron(m: &Mask) -> Mask {
    m.scale_up(SCALE)
}

fn bounds_of(p: &Plane, threshold: f32) -> Option<(isize, isize, isize, isize)> {
    let (r0, r1) = p.rows_above(threshold)?;
    let (c0, c1) = p.cols_above(threshold)?;
    Some((c0 as isize, c1 as isize, r0 as isize, r1 as isize))
}

fn shift(p: &Plane, dx: isize, dy: isize) -> Plane {
    let mut out = Plane::new(p.w, p.h);
    out.paste(p, dx, dy);
    out
}

pub fn redraw(image: &Rgba, labels: &[Label], sans: &Face, condensed: &Face) -> Rgba {
    let (w, h) = (image.w, image.h);
    let region = |l: &Label| image.crop(l.box_[0], l.box_[1], l.box_[2], l.box_[3]);
    let mut clean = image.clone();
    let mut spared = Mask::new(w, h);
    let mut found: Vec<Option<(Plane, [f32; 3])>> = Vec::new();
    let mut erasing: Vec<Option<Mask>> = Vec::new();
    let mut profiles: HashMap<String, Vec<Vec<[f32; 3]>>> = HashMap::new();
    for l in labels {
        let r = region(l);
        if l.button {
            let (core, colour, erase) = button_letters(&r, l.letters);
            erasing.push(Some(erase));
            for y in l.box_[1]..l.box_[3] {
                for x in l.box_[0]..l.box_[2] {
                    spared.set(x, y, true);
                }
            }
            found.push(Some((core, colour)));
        } else {
            erasing.push(None);
            match letters(&r) {
                Some(found_letters) => {
                    if l.shine {
                        if let Some(p) = found_letters.profile {
                            profiles.entry(group(l).to_string()).or_default().push(p);
                        }
                    }
                    found.push(Some((found_letters.core, found_letters.colour)));
                }
                None => found.push(None),
            }
        }
    }
    let mut colours: HashMap<String, Vec<[f32; 3]>> = HashMap::new();
    for (l, f) in labels.iter().zip(&found) {
        if let Some((_, c)) = f {
            colours.entry(group(l).to_string()).or_default().push(*c);
        }
    }
    let shared: HashMap<String, [f32; 3]> = colours.iter().map(|(g, cs)| (g.clone(), [0, 1, 2].map(|k| median_of(&mut cs.iter().map(|c| c[k]).collect::<Vec<f32>>())))).collect();
    let measured: Vec<Option<[f32; 4]>> = labels.iter().zip(&found).map(|(l, f)| if l.button { None } else { f.as_ref().and_then(|(core, _)| plate_colour(&region(l), core)) }).collect();
    let plain_for: Vec<Option<[f32; 4]>> = labels
        .iter()
        .zip(&measured)
        .map(|(l, m)| {
            let m = (*m)?;
            let alike: Vec<[f32; 4]> = labels.iter().zip(&measured).filter_map(|(o, om)| om.filter(|om| group(o) == group(l) && (0..3).map(|k| (om[k] - m[k]).abs()).fold(0.0f32, f32::max) < 0.08)).collect();
            Some([0, 1, 2, 3].map(|k| median_of(&mut alike.iter().map(|c| c[k]).collect::<Vec<f32>>())))
        })
        .collect();
    let condensed_groups: HashSet<String> =
        labels.iter().zip(&found).filter(|(l, f)| !l.button && f.as_ref().is_some_and(|(core, _)| squeeze_needed(sans, l.text, core) < 0.75)).map(|(l, _)| group(l).to_string()).collect();
    let mut drawn: Vec<Drawn> = Vec::new();
    for (i, l) in labels.iter().enumerate() {
        let Some((core, _)) = &found[i] else { continue };
        let face = if condensed_groups.contains(group(l)) { condensed } else { sans };
        let Some(layout) = banners::layout(face, l.text, core, &TRACKINGS) else { continue };
        let r = region(l);
        let cleaned = if l.button { button_without_letters(&r, core, erasing[i].as_ref().unwrap()) } else { without_letters(&r, core, plain_for[i]) };
        clean.paste(&cleaned, l.box_[0], l.box_[1]);
        let inked = Mask::from(core, |v| v > 0.5);
        let under = median_colour(&cleaned, &inked).map(rgb).unwrap_or([1.0; 3]);
        let base = shared[group(l)];
        let shadowed = luminance(base) > luminance(under);
        let mut colour = vec![base; layout.v.len()];
        if l.shine {
            if let Some(list) = profiles.get(group(l)).filter(|p| !p.is_empty()) {
                let profile: Vec<[f32; 3]> = (0..16).map(|j| [0, 1, 2].map(|k| median_of(&mut list.iter().map(|p| p[j][k]).collect::<Vec<f32>>()))).collect();
                if let Some((top, bottom)) = layout.rows_above(0.3) {
                    let marks: Vec<f32> = (0..16).map(|j| j as f32).collect();
                    for y in 0..layout.h {
                        let at = ((y as f32 - top as f32) / (bottom as f32 - top as f32).max(1.0)).clamp(0.0, 1.0) * 15.0;
                        let c = [0, 1, 2].map(|k| interp(at, &marks, &profile.iter().map(|p| p[k]).collect::<Vec<f32>>()));
                        for x in 0..layout.w {
                            colour[y * layout.w + x] = c;
                        }
                    }
                }
            }
        }
        let inside = if l.button {
            let solid = mask(r.w, r.h, |x, y| r.at(x, y)[3] > 0.98);
            erode(&fill_holes(&solid), 2)
        } else {
            let b = plate_box(&r);
            let area = rect(r.w, r.h, b);
            let plain = median_colour(&r, &area).map(rgb).unwrap_or([0.0; 3]);
            area.and(&dividers(&r, &area, luminance(plain)).not())
        };
        let size = if !l.size.is_empty() { l.size.to_string() } else { group(l).to_string() };
        drawn.push(Drawn { x0: l.box_[0], y0: l.box_[1], mask: layout, colour, shadowed, engraved: l.button.then_some(under), inside: kron(&inside), size });
    }
    let mut group_scale: HashMap<String, f32> = HashMap::new();
    for d in &drawn {
        let scale = match (bounds_of(&d.mask, 0.15), d.inside.bounds()) {
            (Some((c0, c1, r0, r1)), Some((x0, y0, x1, y1))) => {
                let room_w = (x1 - x0) as f32 - 2.0 * MARGIN as f32;
                let room_h = (y1 - y0) as f32 - 2.0 * MARGIN as f32;
                (room_w / (c1 - c0 + 1) as f32).min(room_h / (r1 - r0 + 1) as f32).min(1.0)
            }
            _ => 1.0,
        };
        let entry = group_scale.entry(d.size.clone()).or_insert(1.0);
        *entry = entry.min(scale);
    }
    for d in drawn.iter_mut() {
        let scale = group_scale[&d.size];
        if scale < 0.999 {
            let (mw, mh) = (d.mask.w, d.mask.h);
            let resized = resize(&d.mask, ((mw as f32 * scale).round() as usize).max(1), ((mh as f32 * scale).round() as usize).max(1), Filter::Lanczos).map(|v| v.clamp(0.0, 1.0));
            let mut m = Plane::new(mw, mh);
            m.paste(&resized, ((mw - resized.w) / 2) as isize, ((mh - resized.h) / 2) as isize);
            d.mask = m;
        }
        if let (Some((c0, c1, r0, r1)), Some((x0, y0, x1, y1))) = (bounds_of(&d.mask, 0.15), d.inside.bounds()) {
            let (x0, y0, x1, y1) = (x0 as isize, y0 as isize, x1 as isize - 1, y1 as isize - 1);
            let (dx, dy) = if d.engraved.is_some() {
                ((x0 + x1 - c0 - c1) / 2, (y0 + y1 - r0 - r1) / 2)
            } else {
                ((x0 + MARGIN - c0).max(0) - (c1 - (x1 - MARGIN)).max(0), (y0 + MARGIN - r0).max(0) - (r1 - (y1 - MARGIN)).max(0))
            };
            if dx != 0 || dy != 0 {
                d.mask = shift(&d.mask, dx, dy);
            }
        }
    }
    let (flat, grain) = flatten(&clean, &spared);
    let mut crisp = mask(w, h, |x, y| grain.at(x, y) > 0.0 || spared.at(x, y));
    for l in labels {
        for y in l.box_[1]..l.box_[3] {
            for x in l.box_[0]..l.box_[2] {
                crisp.set(x, y, true);
            }
        }
    }
    let crisp = dilate(&crisp, 2).and(&spared.not());
    let weight = resize(&gaussian(&crisp.as_plane(), 0.7), w * SCALE, h * SCALE, Filter::Bilinear);
    let (soft, sharp) = (enlarge(&flat), sharp_enlarge(&flat, 3.0));
    let mut out = Rgba::new(w * SCALE, h * SCALE);
    for i in 0..out.v.len() {
        let t = weight.v[i];
        for k in 0..4 {
            out.v[i][k] = soft.v[i][k] * (1.0 - t) + sharp.v[i][k] * t;
        }
    }
    add_grain(&mut out, &grain, 7);
    let (offset, blur, strength) = SHADOW;
    for d in &drawn {
        let (mw, mh) = (d.mask.w, d.mask.h);
        let (ox, oy) = (d.x0 * SCALE, d.y0 * SCALE);
        let clip = dilate(&d.inside, 1).as_plane();
        let mut area = out.crop(ox, oy, ox + mw, oy + mh);
        if let Some(engraved) = d.engraved {
            let step = ((0.6 * SCALE as f32).round() as isize).max(1);
            let light = gaussian(&shift(&d.mask, step, step), 0.2 * SCALE as f32).map(|v| (v * 1.2).clamp(0.0, 1.0));
            let away = distance(&Mask::from(&d.mask, |v| v <= 0.5)).0;
            let ring = gaussian(&away.map(|v| if v <= 0.45 * SCALE as f32 { 1.0 } else { 0.0 }), 0.15 * SCALE as f32).map(|v| v * 0.85);
            let bright = engraved.map(|c| (c * 1.6 + 0.15).clamp(0.0, 1.0));
            for i in 0..area.v.len() {
                let p = &mut area.v[i];
                for k in 0..3 {
                    p[k] = p[k] * (1.0 - light.v[i]) + bright[k] * light.v[i];
                    p[k] = p[k] * (1.0 - ring.v[i]) + engraved[k] * 0.4 * ring.v[i];
                    p[k] = p[k] * (1.0 - d.mask.v[i]) + d.colour[i][k] * d.mask.v[i];
                }
            }
        } else {
            let step = if d.shadowed { (offset * SCALE as f32).round() as isize } else { 0 };
            let shadow = gaussian(&shift(&d.mask, step, step), blur * SCALE as f32).zip(&clip, |s, c| s * if d.shadowed { strength } else { 0.0 } * c);
            let m = d.mask.zip(&clip, |m, c| m * c);
            for i in 0..area.v.len() {
                let p = &mut area.v[i];
                for (k, value) in p.iter_mut().take(3).enumerate() {
                    *value = *value * (1.0 - shadow.v[i]) * (1.0 - m.v[i]) + d.colour[i][k] * m.v[i];
                }
                p[3] = p[3].max(m.v[i].max(shadow.v[i]));
            }
        }
        out.paste(&area, ox, oy);
    }
    out
}

struct Repeat {
    pixels: usize,
    dx: isize,
    dy: isize,
    area: (usize, usize, usize, usize),
    same: Mask,
    boxed: Mask,
}

pub fn mirror_copies(original: &[[u8; 4]], w: usize, h: usize, out: &mut Rgba, labels: &[Label], crisp: &Rgba) {
    const MARGIN_PX: usize = 10;
    let px = |x: usize, y: usize| original[y * w + x];
    let window = |x: usize, y: usize| -> [u8; 64] {
        let mut k = [0u8; 64];
        for dy in 0..4 {
            for dx in 0..4 {
                k[(dy * 4 + dx) * 4..(dy * 4 + dx) * 4 + 4].copy_from_slice(&px(x + dx, y + dy));
            }
        }
        k
    };
    let mut index: HashMap<[u8; 64], Vec<(usize, usize)>> = HashMap::new();
    for y in 0..h.saturating_sub(3) {
        for x in 0..w.saturating_sub(3) {
            index.entry(window(x, y)).or_default().push((y, x));
        }
    }
    let lum = Plane { w, h, v: original.iter().map(|p| luminance([p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0])).collect() };
    let mean = uniform(&lum, 3);
    let square = uniform(&lum.map(|v| v * v), 3);
    let detail = Mask { w, h, v: (0..w * h).map(|i| (square.v[i] - mean.v[i].powi(2)).max(0.0).sqrt() > 0.06).collect() };
    let (piece_map, _) = label(&Mask { w, h, v: original.iter().map(|p| p[3] as f32 / 255.0 > 0.05).collect() }, true);
    let mut protected = Mask::new(w, h);
    for l in labels {
        let [x0, y0, x1, y1] = l.box_;
        for y in y0.saturating_sub(MARGIN_PX)..(y1 + MARGIN_PX).min(h) {
            for x in x0.saturating_sub(MARGIN_PX)..(x1 + MARGIN_PX).min(w) {
                protected.set(x, y, true);
            }
        }
    }
    for l in labels {
        let [x0, y0, x1, y1] = l.box_;
        let (ex0, ey0, ex1, ey1) = (x0.saturating_sub(MARGIN_PX), y0.saturating_sub(MARGIN_PX), (x1 + MARGIN_PX).min(w), (y1 + MARGIN_PX).min(h));
        let mut offsets: HashSet<(isize, isize)> = HashSet::new();
        let mut ky = 0;
        while ky + 4 < ey1 - ey0 {
            let mut kx = 0;
            while kx + 4 < ex1 - ex0 {
                let (gx, gy) = (ex0 + kx, ey0 + ky);
                let patch: Vec<[u8; 4]> = (0..16).map(|i| px(gx + i % 4, gy + i / 4)).collect();
                if patch.iter().all(|p| p[3] == 255) {
                    let values: Vec<f32> = patch.iter().flat_map(|p| [p[0] as f32, p[1] as f32, p[2] as f32]).collect();
                    let m = values.iter().sum::<f32>() / values.len() as f32;
                    let spread = (values.iter().map(|v| (v - m).powi(2)).sum::<f32>() / values.len() as f32).sqrt();
                    if spread >= 20.0 {
                        for &(hy, hx) in index.get(&window(gx, gy)).map(|v| v.as_slice()).unwrap_or(&[]) {
                            offsets.insert((hx as isize - gx as isize, hy as isize - gy as isize));
                        }
                    }
                }
                kx += 2;
            }
            ky += 2;
        }
        let mut found: Vec<Repeat> = Vec::new();
        for &(dx, dy) in &offsets {
            if dx.unsigned_abs() < ex1 - ex0 && dy.unsigned_abs() < ey1 - ey0 {
                continue;
            }
            let (ax0, ay0) = ((ex0 as isize).max(-dx), (ey0 as isize).max(-dy));
            let (ax1, ay1) = ((ex1 as isize).min(w as isize - dx), (ey1 as isize).min(h as isize - dy));
            if ax1 - ax0 < 4 || ay1 - ay0 < 4 {
                continue;
            }
            let (ax0, ay0, ax1, ay1) = (ax0 as usize, ay0 as usize, ax1 as usize, ay1 as usize);
            let (rw, rh) = (ax1 - ax0, ay1 - ay0);
            let tx = |x: usize| (ax0 as isize + x as isize + dx) as usize;
            let ty = |y: usize| (ay0 as isize + y as isize + dy) as usize;
            let same = mask(rw, rh, |x, y| px(tx(x), ty(y)) == px(ax0 + x, ay0 + y) && px(ax0 + x, ay0 + y)[3] > 0 && !protected.at(tx(x), ty(y)));
            let detail_same = (0..rw * rh).filter(|&i| same.v[i] && detail.at(ax0 + i % rw, ay0 + i / rw)).count();
            if same.count() < 30 || detail_same < 25 {
                continue;
            }
            let in_box = (0..rw * rh)
                .filter(|&i| {
                    let (x, y) = (ax0 + i % rw, ay0 + i / rw);
                    same.v[i] && detail.at(x, y) && x >= x0 && x < x1 && y >= y0 && y < y1
                })
                .count();
            if in_box < 8 {
                continue;
            }
            let Some(kept) = largest(&same) else { continue };
            let Some((bx0, by0, bx1, by1)) = kept.bounds() else { continue };
            let boxed = rect(rw, rh, (bx0, by0, bx1, by1));
            let filled_there = (0..rw * rh).filter(|&i| boxed.v[i] && px(tx(i % rw), ty(i / rw))[3] > 0).count();
            if (kept.count() as f32) < 0.6 * filled_there as f32 {
                continue;
            }
            let grown = dilate(&kept, 3);
            let near_clear = (0..rw * rh).filter(|&i| grown.v[i] && px(tx(i % rw), ty(i / rw))[3] < 128).count();
            if near_clear < 6 {
                continue;
            }
            let own = piece_map[((y0 + y1) / 2) * w + (x0 + x1) / 2];
            if (0..rw * rh).any(|i| kept.v[i] && piece_map[ty(i / rw) * w + tx(i % rw)] == own) {
                continue;
            }
            found.push(Repeat { pixels: kept.count(), dx, dy, area: (ax0, ay0, ax1, ay1), same: kept, boxed });
        }
        found.sort_by(|a, b| b.pixels.cmp(&a.pixels).then((a.dx, a.dy).cmp(&(b.dx, b.dy))));
        let mut taken: Vec<(isize, isize)> = Vec::new();
        for Repeat { dx, dy, area: (ax0, ay0, ax1, ay1), same, boxed, .. } in found {
            if taken.iter().any(|&(tx, ty)| (dx - tx).abs() <= 4 && (dy - ty).abs() <= 4) {
                continue;
            }
            taken.push((dx, dy));
            let (rw, rh) = (ax1 - ax0, ay1 - ay0);
            let tx = |x: usize| (ax0 as isize + x as isize + dx) as usize;
            let ty = |y: usize| (ay0 as isize + y as isize + dy) as usize;
            let free = mask(rw, rh, |x, y| !protected.at(tx(x), ty(y)));
            let around = kron(&free.and(&dilate(&boxed, 4)).and(&mask(rw, rh, |x, y| px(tx(x), ty(y))[3] > 0)));
            let allowed = kron(&free.and(&dilate(&boxed, 1)));
            let same_big = kron(&same);
            let (sx0, sy0) = (ax0 * SCALE, ay0 * SCALE);
            let (dx0, dy0) = (tx(0) * SCALE, ty(0) * SCALE);
            let bw = rw * SCALE;
            for y in 0..rh * SCALE {
                for x in 0..bw {
                    let i = y * bw + x;
                    let target = (dy0 + y) * out.w + dx0 + x;
                    if around.v[i] {
                        out.v[target] = crisp.v[target];
                    }
                }
            }
            for y in 0..rh * SCALE {
                for x in 0..bw {
                    let i = y * bw + x;
                    let target = (dy0 + y) * out.w + dx0 + x;
                    let source = out.v[(sy0 + y) * out.w + sx0 + x];
                    let keep_alpha = out.v[target][3];
                    if same_big.v[i] && allowed.v[i] {
                        out.v[target] = source;
                    }
                    out.v[target][3] = if same_big.v[i] { source[3] } else { keep_alpha };
                }
            }
        }
    }
}

