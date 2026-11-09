use super::banners;
use super::ops::{dilate, gaussian, resize, Filter, Mask, Plane, Rgba};
use super::text::Face;
use super::titles::{place, search, SCALE, SS, TITLE};

pub type Box4 = [usize; 4];

pub const PRESS_ANY_BUTTON_BOX: Box4 = [44, 0, 212, 32];
pub const PRESS_ANY_BUTTON_KEEP: [Box4; 2] = [[0, 0, 47, 32], [211, 0, 256, 32]];
pub const MENU_BOXES: [Box4; 8] = [[0, 0, 170, 24], [0, 24, 170, 50], [0, 50, 170, 73], [0, 73, 170, 97], [0, 97, 170, 126], [170, 26, 256, 40], [170, 67, 256, 84], [170, 88, 256, 124]];
pub const MENU_KEEP: [Box4; 2] = [[170, 0, 256, 26], [170, 40, 256, 67]];
pub const COPYRIGHT_BOX: Box4 = [0, 0, 512, 32];
pub const GMAP_BUTTONS: [(Box4, usize, Ink); 8] = [
    ([147, 11, 181, 30], 0, Ink::Dark),
    ([186, 21, 221, 34], 1, Ink::Light),
    ([147, 61, 181, 75], 2, Ink::Dark),
    ([187, 61, 221, 74], 1, Ink::Dark),
    ([66, 91, 101, 110], 0, Ink::Light),
    ([107, 91, 141, 110], 0, Ink::Dark),
    ([146, 100, 181, 116], 2, Ink::Light),
    ([187, 91, 221, 110], 0, Ink::Dark),
];
pub const OVERVIEW: [(Box4, usize, Ink); 9] = [
    ([192, 23, 270, 39], 0, Ink::Light),
    ([191, 81, 238, 98], 1, Ink::Dark),
    ([180, 202, 202, 215], 2, Ink::Light),
    ([228, 202, 251, 216], 3, Ink::Light),
    ([182, 220, 201, 233], 4, Ink::Light),
    ([318, 186, 406, 202], 5, Ink::Opaque),
    ([318, 202, 406, 219], 6, Ink::Opaque),
    ([318, 222, 406, 239], 7, Ink::Opaque),
    ([318, 239, 406, 256], 8, Ink::Opaque),
];
pub const HALT_BOX: Box4 = [168, 406, 210, 427];
pub const TM_BOX: Box4 = [450, 63, 462, 70];
pub const BUBBLE_A: Box4 = [139, 490, 151, 500];
pub const TUTORIAL: [(Box4, usize, Ink); 11] = [
    ([394, 8, 428, 28], 0, Ink::Light),
    ([394, 49, 430, 69], 0, Ink::Dark),
    ([394, 88, 428, 108], 0, Ink::Light),
    ([4, 195, 76, 207], 0, Ink::Light),
    ([4, 235, 76, 248], 0, Ink::Dark),
    ([4, 183, 76, 195], 1, Ink::Dark),
    ([4, 224, 76, 236], 1, Ink::Dark),
    ([312, 102, 340, 117], 2, Ink::Dark),
    ([352, 102, 380, 117], 3, Ink::Dark),
    ([312, 174, 340, 189], 2, Ink::Dark),
    ([352, 174, 380, 189], 3, Ink::Dark),
];
pub const SYSTEM_ICONS: [(Box4, &str); 7] = [
    ([139, 2, 149, 14], "A"),
    ([155, 2, 165, 14], "B"),
    ([171, 2, 181, 14], "X"),
    ([187, 2, 197, 14], "Y"),
    ([91, 378, 102, 391], "L"),
    ([107, 378, 118, 391], "R"),
    ([125, 378, 146, 391], "START"),
];
pub const SOFT: banners::Tone = banners::Tone { share: 1.0, grey: 0.92 };
const EDGE_CONTRAST: f32 = 0.12;
const FIT_MARGIN: f32 = 4.0;
const SPLIT_CONTRAST: f32 = 0.15;
const LETTER_SHARE: usize = 10;
const LINE_SHARE: f32 = 0.8;
const LOGO_SMOOTH: f32 = 0.4;
const LOGO_EDGE: (f32, f32) = (0.22, 0.62);
const TM_ROWS: (usize, usize) = (64, 69);
const CONDENSED: f32 = 0.7;
const CALM_PASSES: usize = 2;
const CALM_REACH: usize = 2;
const CALM_SPACE: f32 = 1.5;
const CALM_RANGE: f32 = 0.1;
const FILL_SIGMA: f32 = 3.0;

fn kept(core: &Plane, keep: &[Box4]) -> Mask {
    let inked = dilate(&Mask::from(core, |v| v > 0.1), 1);
    let mut mask = Mask::new(core.w, core.h);
    for k in keep {
        for y in k[1]..k[3].min(core.h) {
            for x in k[0]..k[2].min(core.w) {
                mask.set(x, y, inked.at(x, y));
            }
        }
    }
    mask
}

pub fn sharp_art(image: &Rgba) -> Rgba {
    let (w, h) = (image.w, image.h);
    let alpha = image.channel(3);
    let channels = [0, 1, 2, 3].map(|k| if k == 3 { alpha.clone() } else { image.channel(k).zip(&alpha, |v, a| v * a) });
    let shaped = channels.map(|p| {
        let up = resize(&p, w * SCALE, h * SCALE, Filter::Bicubic);
        let around = |pick: fn(f32, f32) -> f32, start: f32| {
            let mut out = Plane::new(w, h);
            for y in 0..h {
                for x in 0..w {
                    let mut v = start;
                    for yy in y.saturating_sub(1)..(y + 2).min(h) {
                        for xx in x.saturating_sub(1)..(x + 2).min(w) {
                            v = pick(v, p.at(xx, yy));
                        }
                    }
                    out.set(x, y, v);
                }
            }
            resize(&out, w * SCALE, h * SCALE, Filter::Bilinear)
        };
        let (lo, hi) = (around(f32::min, 1.0), around(f32::max, 0.0));
        Plane {
            w: up.w,
            h: up.h,
            v: (0..up.v.len())
                .map(|i| {
                    let (u, l, r) = (up.v[i], lo.v[i], hi.v[i] - lo.v[i]);
                    if r <= EDGE_CONTRAST {
                        return u.clamp(0.0, 1.0);
                    }
                    let mut t = ((u - l) / r).clamp(0.0, 1.0);
                    for _ in 0..2 {
                        t = t * t * (3.0 - 2.0 * t);
                    }
                    l + r * t
                })
                .collect(),
        }
    });
    let mut out = Rgba::new(w * SCALE, h * SCALE);
    for (i, p) in out.v.iter_mut().enumerate() {
        let a = shaped[3].v[i].clamp(0.0, 1.0);
        *p = if a > 1e-4 { [(shaped[0].v[i] / a).clamp(0.0, 1.0), (shaped[1].v[i] / a).clamp(0.0, 1.0), (shaped[2].v[i] / a).clamp(0.0, 1.0), a] } else { [0.0; 4] };
    }
    out
}

fn premultiplied(p: [f32; 4]) -> [f32; 4] {
    [p[0] * p[3], p[1] * p[3], p[2] * p[3], p[3]]
}

fn unpremultiplied(p: [f32; 4]) -> [f32; 4] {
    let a = p[3].clamp(0.0, 1.0);
    if a > 1e-4 {
        [p[0] / a, p[1] / a, p[2] / a, a]
    } else {
        [0.0; 4]
    }
}

pub fn calm(image: &Rgba) -> Rgba {
    let (w, h) = (image.w, image.h);
    let mut pixels: Vec<[f32; 4]> = image.v.iter().map(|&p| premultiplied(p)).collect();
    for _ in 0..CALM_PASSES {
        pixels = (0..w * h)
            .map(|i| {
                let (x, y, p) = (i % w, i / w, pixels[i]);
                let (mut sum, mut weight) = ([0.0; 4], 0.0);
                for yy in y.saturating_sub(CALM_REACH)..(y + CALM_REACH + 1).min(h) {
                    for xx in x.saturating_sub(CALM_REACH)..(x + CALM_REACH + 1).min(w) {
                        let q = pixels[yy * w + xx];
                        let (dx, dy) = (xx as f32 - x as f32, yy as f32 - y as f32);
                        let near: f32 = (0..4).map(|c| (q[c] - p[c]).powi(2)).sum();
                        let k = (-(dx * dx + dy * dy) / (2.0 * CALM_SPACE * CALM_SPACE) - near / (2.0 * CALM_RANGE * CALM_RANGE)).exp();
                        (0..4).for_each(|c| sum[c] += k * q[c]);
                        weight += k;
                    }
                }
                sum.map(|v| v / weight)
            })
            .collect();
    }
    Rgba { w, h, v: pixels.into_iter().map(unpremultiplied).collect() }
}

fn median4(mut samples: Vec<[f32; 4]>) -> [f32; 4] {
    [0, 1, 2, 3].map(|k| {
        samples.sort_by(|a, b| a[k].total_cmp(&b[k]));
        samples[samples.len() / 2][k]
    })
}

fn gap(a: [f32; 4], b: [f32; 4]) -> f32 {
    (0..4).map(|k| (a[k] - b[k]).abs()).sum()
}

struct Split {
    letter: [f32; 4],
    rows: Vec<[f32; 4]>,
    mask: Plane,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Ink {
    Light,
    Dark,
    Opaque,
}

fn split(region: &Rgba, ink: Ink) -> Option<Split> {
    let (w, h) = (region.w, region.h);
    let pixels: Vec<[f32; 4]> = region.v.iter().map(|&p| premultiplied(p)).collect();
    let lum: Vec<f32> = pixels.iter().map(|p| if ink == Ink::Opaque { p[3] } else { p[0] * 0.299 + p[1] * 0.587 + p[2] * 0.114 }).collect();
    let mut order: Vec<usize> = (0..pixels.len()).collect();
    order.sort_by(|&a, &b| lum[a].total_cmp(&lum[b]));
    if ink != Ink::Dark {
        order.reverse();
    }
    let extreme = order.len() / LETTER_SHARE;
    if extreme < 5 || (lum[order[0]] - lum[order[order.len() - 1]]).abs() < SPLIT_CONTRAST {
        return None;
    }
    let letter = median4(order[..extreme].iter().map(|&i| pixels[i]).collect());
    let mut far: Vec<(f32, [f32; 4])> = pixels.iter().map(|&p| (gap(p, letter), p)).collect();
    far.sort_by(|a, b| b.0.total_cmp(&a.0));
    let reach = gap(letter, median4(far.iter().take(far.len().div_ceil(2)).map(|&(_, p)| p).collect())) * 0.5;
    let found: Vec<Option<[f32; 4]>> = (0..h)
        .map(|y| {
            let plain: Vec<[f32; 4]> = pixels[y * w..(y + 1) * w].iter().copied().filter(|&p| gap(p, letter) > reach).collect();
            (plain.len() >= 3).then(|| median4(plain))
        })
        .collect();
    let rows: Vec<[f32; 4]> = (0..h)
        .map(|y| {
            found[y].unwrap_or_else(|| {
                let mut far: Vec<(f32, [f32; 4])> = pixels[y * w..(y + 1) * w].iter().map(|&p| (gap(p, letter), p)).collect();
                far.sort_by(|a, b| b.0.total_cmp(&a.0));
                median4(far.iter().take(w.div_ceil(2)).map(|&(_, p)| p).collect())
            })
        })
        .collect();
    let mut mask = Plane::new(w, h);
    for y in 0..h {
        let bg = rows[y];
        let span: Vec<f32> = (0..4).map(|k| letter[k] - bg[k]).collect();
        let length: f32 = span.iter().map(|v| v * v).sum::<f32>().max(1e-4);
        for x in 0..w {
            let p = pixels[y * w + x];
            let t = (0..4).map(|k| (p[k] - bg[k]) * span[k]).sum::<f32>() / length;
            mask.set(x, y, ((t - 0.15) / 0.5).clamp(0.0, 1.0));
        }
    }
    let lines_x: Vec<usize> = (0..w).filter(|&x| (0..h).filter(|&y| mask.at(x, y) > 0.5).count() as f32 >= LINE_SHARE * h as f32).collect();
    let lines_y: Vec<usize> = (0..h).filter(|&y| (0..w).filter(|&x| mask.at(x, y) > 0.5).count() as f32 >= LINE_SHARE * w as f32).collect();
    for x in lines_x {
        (0..h).for_each(|y| mask.set(x, y, 0.0));
    }
    for y in lines_y {
        (0..w).for_each(|x| mask.set(x, y, 0.0));
    }
    Some(Split { letter, rows, mask })
}

fn fill(region: &Rgba, found: &Split, erase: &Mask) -> Rgba {
    let (w, h) = (region.w, region.h);
    let pixels: Vec<[f32; 4]> = region.v.iter().map(|&p| premultiplied(p)).collect();
    let known: Vec<bool> = (0..w * h).map(|i| !erase.v[i] && found.mask.v[i] == 0.0).collect();
    let reach = (FILL_SIGMA * 3.0).ceil() as usize;
    let smooth: Vec<Option<[f32; 4]>> = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let (mut sum, mut weight) = ([0.0; 4], 0.0);
            for yy in y.saturating_sub(reach)..(y + reach + 1).min(h) {
                for xx in x.saturating_sub(reach)..(x + reach + 1).min(w) {
                    let j = yy * w + xx;
                    if known[j] {
                        let (dx, dy) = (xx as f32 - x as f32, yy as f32 - y as f32);
                        let k = (-(dx * dx + dy * dy) / (2.0 * FILL_SIGMA * FILL_SIGMA)).exp();
                        (0..4).for_each(|c| sum[c] += k * pixels[j][c]);
                        weight += k;
                    }
                }
            }
            (weight > 1e-3).then(|| sum.map(|v| v / weight))
        })
        .collect();
    let mut out = region.clone();
    for i in (0..w * h).filter(|&i| erase.v[i]) {
        out.v[i] = unpremultiplied(smooth[i].unwrap_or(found.rows[i / w]));
    }
    out
}

fn word(face: &Face, text: &str, height: f32, room: (f32, f32)) -> Option<Plane> {
    let line = face.sized(40.0 * SS as f32).line(text, 0.0)?;
    let tall = (height / line.h as f32).min(room.1 / line.h as f32);
    let squeeze = (room.0 / (line.w as f32 * tall)).clamp(CONDENSED, 1.0);
    let tall = tall.min(room.0 / (line.w as f32 * squeeze));
    Some(resize(&line, ((line.w as f32 * tall * squeeze).round() as usize).max(1), ((line.h as f32 * tall).round() as usize).max(1), Filter::Lanczos).map(|v| v.clamp(0.0, 1.0)))
}

fn stamp(out: &mut Rgba, ink: &Plane, (ox, oy): (usize, usize), colour: [f32; 3]) {
    for y in 0..ink.h {
        for x in 0..ink.w {
            let m = ink.at(x, y);
            if m <= 0.0 {
                continue;
            }
            let p = &mut out.v[(oy + y) * out.w + ox + x];
            for k in 0..3 {
                p[k] = p[k] * (1.0 - m) + colour[k].clamp(0.0, 1.0) * m;
            }
            p[3] = p[3].max(m);
        }
    }
}

pub fn buttons(image: &Rgba, entries: &[(Box4, &str, Ink)], face: &Face, enlarge: fn(&Rgba) -> Rgba) -> Rgba {
    let mut cleaned = image.clone();
    let mut drawn = Vec::new();
    for &(b, text, ink) in entries {
        let region = image.crop(b[0], b[1], b[2], b[3]);
        let Some(found) = split(&region, ink) else { continue };
        let inked: Vec<usize> = (0..region.h).filter(|&y| (0..region.w).filter(|&x| found.mask.at(x, y) > 0.5).count() >= 2).collect();
        let (Some(&y0), Some(&last)) = (inked.first(), inked.last()) else { continue };
        let y1 = last + 1;
        let erase = dilate(&Mask::from(&found.mask, |v| v > 0.05), 1);
        cleaned.paste(&fill(&region, &found, &erase), b[0], b[1]);
        let room = (((b[2] - b[0]) * SCALE) as f32 - 2.0 * FIT_MARGIN, ((b[3] - b[1]) * SCALE) as f32 - 2.0 * FIT_MARGIN);
        let Some(ink) = word(face, text, ((y1 - y0) * SCALE) as f32, room) else { continue };
        let x = b[0] * SCALE + ((b[2] - b[0]) * SCALE - ink.w) / 2;
        let centre = (b[1] as f32 + (y0 + y1) as f32 / 2.0) * SCALE as f32;
        let y = ((centre - ink.h as f32 / 2.0).round() as usize).clamp(b[1] * SCALE, (b[3] * SCALE).saturating_sub(ink.h));
        let a = found.letter[3].max(1e-4);
        drawn.push((x, y, ink, [found.letter[0] / a, found.letter[1] / a, found.letter[2] / a]));
    }
    let mut out = enlarge(&cleaned);
    for (x, y, ink, colour) in drawn {
        stamp(&mut out, &ink, (x, y), colour);
    }
    out
}

pub fn redraw(image: &Rgba, words: &[(Box4, &str)], keep: &[Box4], tone: banners::Tone, face: &Face) -> Option<Rgba> {
    let core = banners::core(image);
    let keep_mask = kept(&core, keep);
    let text = core.zip(&keep_mask.as_plane(), |c, k| if k > 0.0 { 0.0 } else { c });
    let probe = face.sized(40.0 * SS as f32);
    let mut big = Plane::new(image.w * SCALE, image.h * SCALE);
    for &(b, words) in words {
        let (x0, x1, y0, y1) = text.crop(b[0], b[1], b[2], b[3]).bbox(0.5)?;
        let bounds = (b[0] + x0, b[0] + x1, b[1] + y0, b[1] + y1);
        let mask = probe.line(words, 0.0)?;
        let fit = search(&mask, &text, (b[1], b[3]), bounds, &TITLE)?;
        let canvas = place(&mask, &text, bounds, &fit, SCALE, TITLE.edge)?;
        for (v, c) in big.v.iter_mut().zip(&canvas.v) {
            *v = v.max(*c);
        }
    }
    let mut cleared = image.clone();
    for (p, &k) in cleared.v.iter_mut().zip(&keep_mask.v) {
        if k {
            *p = [0.0; 4];
        }
    }
    let mut out = banners::redraw(&cleared, &text, banners::Kind::Plain, tone, &big);
    if keep_mask.any() {
        let enlarged = sharp_art(image);
        for (i, p) in out.v.iter_mut().enumerate() {
            let (x, y) = (i % out.w / SCALE, i / out.w / SCALE);
            if keep_mask.at(x, y) {
                *p = enlarged.v[i];
            }
        }
    }
    Some(out)
}

pub fn logo(image: &Rgba, face: &Face) -> Rgba {
    let fine = SCALE * 4;
    let up = resize(&image.lum(), image.w * fine, image.h * fine, Filter::Bicubic);
    let level = gaussian(&up, LOGO_SMOOTH * fine as f32).map(|v| {
        let x = ((v - LOGO_EDGE.0) / (LOGO_EDGE.1 - LOGO_EDGE.0)).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    });
    let level = level.down(4);
    let mut out = Rgba::new(level.w, level.h);
    for (p, &l) in out.v.iter_mut().zip(&level.v) {
        *p = [l, l, l, 1.0];
    }
    let b = TM_BOX;
    for y in b[1] * SCALE..b[3] * SCALE {
        out.v[y * out.w + b[0] * SCALE..y * out.w + b[2] * SCALE].fill([0.0, 0.0, 0.0, 1.0]);
    }
    let room = (((b[2] - b[0]) * SCALE) as f32, ((b[3] - b[1]) * SCALE) as f32);
    if let Some(ink) = word(face, "TM", ((TM_ROWS.1 - TM_ROWS.0) * SCALE) as f32, room) {
        let x = b[0] * SCALE + ((b[2] - b[0]) * SCALE).saturating_sub(ink.w) / 2;
        let y = ((TM_ROWS.0 + TM_ROWS.1) * SCALE / 2).saturating_sub(ink.h / 2);
        stamp(&mut out, &ink, (x, y), [1.0; 3]);
    }
    out
}
