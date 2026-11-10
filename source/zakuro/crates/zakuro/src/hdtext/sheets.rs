use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use super::ops::{median_of, resize, Filter, Plane};
use super::text::Face;

const SCALE: usize = 4;
const GAP: usize = 1;
const RANGES: [(u32, u32); 6] = [(0x21, 0x7E), (0xA1, 0x17F), (0x2010, 0x2027), (0x2030, 0x203A), (0x20AC, 0x20AC), (0x2122, 0x2122)];

#[derive(Clone, Copy)]
pub struct Glyph {
    pub code: u32,
    pub sheet: usize,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub left: i32,
    pub top: i32,
    pub advance: i32,
}

pub struct Bfnt {
    pub width: usize,
    pub height: usize,
    pub glyphs: Vec<Glyph>,
    pub sheets: Vec<usize>,
    pub sheets_at: usize,
}

fn u16_at(d: &[u8], at: usize) -> usize {
    u16::from_le_bytes([d[at], d[at + 1]]) as usize
}

pub fn parse(data: &[u8]) -> Option<Bfnt> {
    if data.len() < 0x30 {
        return None;
    }
    let (width, height) = (u16_at(data, 0x10), u16_at(data, 0x12));
    let count = u16_at(data, 0x22);
    let sheets_at = u32::from_le_bytes(data.get(0x24..0x28)?.try_into().ok()?) as usize;
    let mut glyphs = Vec::with_capacity(count);
    for i in 0..count {
        let at = 0x30 + i * 16;
        let e = data.get(at..at + 16)?;
        glyphs.push(Glyph {
            code: u16_at(e, 0) as u32,
            sheet: u16_at(e, 2),
            x: u16_at(e, 4),
            y: u16_at(e, 6),
            w: e[8] as usize,
            h: e[9] as usize,
            left: e[10] as i8 as i32,
            top: e[11] as i8 as i32,
            advance: e[12] as i8 as i32,
        });
    }
    let size = width * height / 2;
    let mut sheets = Vec::new();
    let mut at = sheets_at;
    while size > 0 && at + size <= data.len() {
        sheets.push(at);
        at += size;
    }
    Some(Bfnt { width, height, glyphs, sheets, sheets_at })
}

#[derive(Clone, Copy, PartialEq)]
pub enum Measure {
    Capitals,
    Digits,
}

pub struct Style {
    pub measure: Measure,
    pub natural_space: bool,
    pub keep_digit_advance: bool,
}

pub struct Redrawn {
    pub file: Vec<u8>,
    pub sheets: Vec<(u64, Plane)>,
}

fn wanted(code: u32) -> bool {
    RANGES.iter().any(|&(low, high)| (low..=high).contains(&code))
}

fn cell_of(images: &[Plane], g: &Glyph) -> Plane {
    images[g.sheet].crop(g.x, g.y, g.x + g.w, g.y + g.h)
}

fn ink_height(p: &Plane) -> f32 {
    (0..p.h).map(|y| p.row_max(y)).sum()
}

fn soften(cell: &Plane) -> Plane {
    let factor = SCALE * 4;
    let up = resize(cell, cell.w * factor, cell.h * factor, Filter::Bicubic);
    let shaped = up.map(|v| {
        let x = ((v - 0.3) / 0.4).clamp(0.0, 1.0);
        x * x * (3.0 - 2.0 * x)
    });
    shaped.down(4)
}

fn encode_a4(p: &Plane) -> Vec<u8> {
    let mut out = vec![0u8; p.w * p.h / 2];
    for y in 0..p.h {
        for x in 0..p.w {
            let nibble = (p.at(x, y).clamp(0.0, 1.0) * 15.0).round() as u8;
            let bit = zakuro_gpu::format::morton_offset(x as u32, y as u32, p.w as u32, 4) as usize;
            out[bit / 8] |= if bit.is_multiple_of(8) { nibble } else { nibble << 4 };
        }
    }
    out
}

enum Kind {
    Blank,
    Vector(char),
    Bitmap,
}

struct Made {
    glyph: Glyph,
    fine: Plane,
    coarse: Plane,
}

fn vector(fine: &Face, c: char, shift: f32) -> Option<(i32, usize, usize, i32, Plane)> {
    let (plane, ox, oy) = fine.placed(c, shift * SCALE as f32, 0.0)?;
    let s = SCALE as f32;
    let left = (ox as f32 / s).floor() as i32;
    let right = ((ox + plane.w as i32) as f32 / s).ceil() as i32;
    let top = (-oy as f32 / s).ceil() as i32;
    let bottom = ((oy + plane.h as i32) as f32 / s).ceil() as i32;
    let (w, h) = ((right - left).max(1) as usize, (top + bottom).max(1) as usize);
    let mut picture = Plane::new(w * SCALE, h * SCALE);
    picture.paste(&plane, (ox - left * SCALE as i32) as isize, (oy + top * SCALE as i32) as isize);
    Some((left, w, h, top, picture))
}

fn bitmap(images: &[Plane], g: &Glyph) -> Plane {
    let (x0, y0) = (g.x.saturating_sub(2), g.y.saturating_sub(2));
    let (x1, y1) = ((g.x + g.w + 2).min(images[g.sheet].w), (g.y + g.h + 2).min(images[g.sheet].h));
    let around = soften(&images[g.sheet].crop(x0, y0, x1, y1));
    let (ox, oy) = ((g.x - x0) * SCALE, (g.y - y0) * SCALE);
    around.crop(ox, oy, ox + g.w * SCALE, oy + g.h * SCALE)
}

fn pack(sizes: &[(usize, usize)], width: usize, height: usize, sheets: usize) -> Option<Vec<(usize, usize, usize)>> {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| (Reverse(sizes[i].1), Reverse(sizes[i].0)));
    let mut places = vec![(0, 0, 0); sizes.len()];
    let (mut sheet, mut x, mut y, mut shelf) = (0, 0, 0, 0);
    for i in order {
        let (w, h) = sizes[i];
        if x + w > width {
            (x, y, shelf) = (0, y + shelf + GAP, 0);
        }
        if y + h > height {
            (sheet, x, y, shelf) = (sheet + 1, 0, 0, 0);
        }
        if sheet >= sheets || w > width || h > height {
            return None;
        }
        places[i] = (sheet, x, y);
        x += w + GAP;
        shelf = shelf.max(h);
    }
    Some(places)
}

pub fn redraw(face: &Face, data: &[u8], font: &Bfnt, images: &[Plane], style: &Style) -> Option<Redrawn> {
    let by_code: HashMap<u32, Glyph> = font.glyphs.iter().map(|g| (g.code, *g)).collect();
    let probe = face.sized(400.0);
    let reference: Vec<char> = match style.measure {
        Measure::Capitals => vec!['H'],
        Measure::Digits => "0123456789".chars().collect(),
    };
    let mut originals: Vec<f32> = reference.iter().filter_map(|&c| by_code.get(&(c as u32))).filter(|g| g.sheet < images.len()).map(|g| ink_height(&cell_of(images, g))).filter(|&h| h > 0.0).collect();
    let mut drawn: Vec<f32> = reference.iter().filter_map(|&c| probe.ink_box(c)).map(|(_, top, _, bottom)| bottom - top).collect();
    if originals.is_empty() || drawn.is_empty() {
        return None;
    }
    let mut em = 400.0 * median_of(&mut originals) / median_of(&mut drawn);
    if style.measure == Measure::Capitals {
        let tallest = by_code.get(&('É' as u32)).filter(|g| g.sheet < images.len()).map(|g| {
            let cell = cell_of(images, g);
            let first = (0..cell.h).find(|&y| cell.row_max(y) > 0.5).unwrap_or(0);
            g.top as f32 - first as f32
        });
        if let (Some(room), Some((_, top, _, _))) = (tallest, probe.ink_box('É')) {
            em = em.min(400.0 * (room + 0.5) / -top);
        }
    }
    let face = probe.sized(em);
    let fine = probe.sized(em * SCALE as f32);
    let natural_digits = "0123456789".chars().filter(|&c| face.has(c)).map(|c| face.advance(c)).fold(0.0f32, f32::max).round() as i32;
    let digit_advance = if style.keep_digit_advance { natural_digits.max(by_code.get(&('0' as u32)).map_or(0, |g| g.advance)) } else { natural_digits };
    let kinds: Vec<Kind> = font
        .glyphs
        .iter()
        .map(|g| {
            if g.w == 0 || g.h == 0 || g.sheet >= images.len() {
                return Kind::Blank;
            }
            let strongest = cell_of(images, g).max();
            if strongest < 0.03 {
                return Kind::Blank;
            }
            match char::from_u32(g.code) {
                Some(c) if wanted(g.code) && face.has(c) && strongest >= 0.1 => Kind::Vector(c),
                _ => Kind::Bitmap,
            }
        })
        .collect();
    let shrink: Vec<f32> = font
        .glyphs
        .iter()
        .zip(&kinds)
        .map(|(g, kind)| match kind {
            Kind::Vector(c) if style.measure == Measure::Digits && !c.is_alphanumeric() => match probe.ink_box(*c) {
                Some((_, top, _, bottom)) if bottom > top => (ink_height(&cell_of(images, g)) / ((bottom - top) * em / 400.0)).min(1.0),
                _ => 1.0,
            },
            _ => 1.0,
        })
        .collect();
    let made: Mutex<Vec<Option<Made>>> = Mutex::new((0..font.glyphs.len()).map(|_| None).collect());
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(g) = font.glyphs.get(i) else { break };
                let one = match kinds[i] {
                    Kind::Blank => {
                        let mut glyph = *g;
                        if style.natural_space && (g.code == 0x20 || g.code == 0xA0) {
                            glyph.advance = face.spaced(' ').0.round() as i32;
                        }
                        Some(Made { glyph, fine: Plane::new(1, 1), coarse: Plane::new(1, 1) })
                    }
                    Kind::Vector(c) => {
                        let smaller = (shrink[i] < 1.0).then(|| (probe.sized(em * shrink[i]), probe.sized(em * shrink[i] * SCALE as f32)));
                        let (face, fine) = smaller.as_ref().map_or((&face, &fine), |(coarse, fine)| (coarse, fine));
                        let (spaced, cut) = face.spaced(c);
                        let (advance, shift) = if c.is_ascii_digit() {
                            (digit_advance, (digit_advance as f32 - face.advance(c)) / 2.0)
                        } else {
                            let advance = spaced.round().max(1.0) as i32;
                            (advance, (advance as f32 - spaced) / 2.0 - cut)
                        };
                        vector(fine, c, shift).map(|(left, w, h, top, picture)| Made {
                            glyph: Glyph { w, h, left, top, advance, ..*g },
                            coarse: picture.down(SCALE),
                            fine: picture,
                        })
                    }
                    Kind::Bitmap => Some(Made { glyph: *g, fine: bitmap(images, g), coarse: cell_of(images, g) }),
                };
                made.lock().unwrap()[i] = one;
            });
        }
    });
    let mut made: Vec<Made> = made.into_inner().unwrap().into_iter().collect::<Option<Vec<Made>>>()?;
    if made.iter().any(|m| m.glyph.w > 255 || m.glyph.h > 255 || [m.glyph.left, m.glyph.top, m.glyph.advance].iter().any(|v| !(-128..=127).contains(v))) {
        return None;
    }
    let blank = made.iter().zip(&kinds).filter(|(_, k)| matches!(k, Kind::Blank)).fold((1, 1), |(w, h), (m, _)| (m.glyph.w.max(w), m.glyph.h.max(h)));
    let inked: Vec<usize> = (0..made.len()).filter(|&i| !matches!(kinds[i], Kind::Blank)).collect();
    let mut sizes: Vec<(usize, usize)> = vec![blank];
    sizes.extend(inked.iter().map(|&i| (made[i].glyph.w, made[i].glyph.h)));
    let places = pack(&sizes, font.width, font.height, font.sheets.len())?;
    let mut fine_sheets: Vec<Plane> = (0..font.sheets.len()).map(|_| Plane::new(font.width * SCALE, font.height * SCALE)).collect();
    let mut coarse_sheets: Vec<Plane> = (0..font.sheets.len()).map(|_| Plane::new(font.width, font.height)).collect();
    for (m, k) in made.iter_mut().zip(&kinds) {
        if matches!(k, Kind::Blank) {
            (m.glyph.sheet, m.glyph.x, m.glyph.y) = places[0];
        }
    }
    for (slot, &i) in inked.iter().enumerate() {
        let (sheet, x, y) = places[slot + 1];
        let m = &mut made[i];
        (m.glyph.sheet, m.glyph.x, m.glyph.y) = (sheet, x, y);
        fine_sheets[sheet].paste(&m.fine, (x * SCALE) as isize, (y * SCALE) as isize);
        coarse_sheets[sheet].paste(&m.coarse, x as isize, y as isize);
    }
    let mut file = data[..font.sheets_at].to_vec();
    for (i, m) in made.iter().enumerate() {
        let g = &m.glyph;
        let at = 0x30 + i * 16;
        file[at..at + 2].copy_from_slice(&(g.code as u16).to_le_bytes());
        file[at + 2..at + 4].copy_from_slice(&(g.sheet as u16).to_le_bytes());
        file[at + 4..at + 6].copy_from_slice(&(g.x as u16).to_le_bytes());
        file[at + 6..at + 8].copy_from_slice(&(g.y as u16).to_le_bytes());
        file[at + 8] = g.w as u8;
        file[at + 9] = g.h as u8;
        file[at + 10] = g.left as i8 as u8;
        file[at + 11] = g.top as i8 as u8;
        file[at + 12] = g.advance as i8 as u8;
        file[at + 13] = 0;
    }
    let mut sheets = Vec::with_capacity(coarse_sheets.len());
    for (coarse, fine) in coarse_sheets.iter().zip(fine_sheets) {
        let encoded = encode_a4(coarse);
        sheets.push((cityhasher::hash(&encoded), fine));
        file.extend_from_slice(&encoded);
    }
    if file.len() < data.len() {
        file.extend_from_slice(&data[file.len()..]);
    }
    Some(Redrawn { file, sheets })
}
