
use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::formats::{self, Blob, Image};
use super::model::{self, Attr, FLOAT};

type V3 = [f64; 3];

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mul(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

fn bounds(points: impl Iterator<Item = V3>) -> (V3, V3) {
    points.fold(([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]), |(lo, hi), p| {
        ([lo[0].min(p[0]), lo[1].min(p[1]), lo[2].min(p[2])], [hi[0].max(p[0]), hi[1].max(p[1]), hi[2].max(p[2])])
    })
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n % 2 == 1 { values[n / 2] } else { (values[n / 2 - 1] + values[n / 2]) / 2.0 }
}

fn percentile(values: &mut [f64], q: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    let at = (values.len() - 1) as f64 * q / 100.0;
    let (low, share) = (at.floor() as usize, at - at.floor());
    match values.get(low + 1) {
        Some(&next) => values[low] + (next - values[low]) * share,
        None => values[low],
    }
}

const HEEL: f64 = 0.03;
const LENGTH_PER_DEPTH: f64 = 2.35;
const LENGTH: (f64, f64) = (0.42, 0.60);
const AHEAD_LEAST: f64 = 0.40;
const BAND: (f64, f64) = (0.234, 0.338);
const OWN: f64 = 0.85;
const END: f64 = 0.12;
const SHOWN: f64 = 0.85;
const WIDEST: f64 = 0.21;
const NOISE: f64 = 1.5;
const LOW: f64 = 0.25;
const TUCK: f64 = 0.40;
const LIFT: f64 = 0.01;
const SKINNED: u32 = 0x240;
const INVERSE_BIND: usize = 0xA4;
const TAG: &[u8; 4] = b"FEET";

pub struct Style {
    pub name: &'static str,
    behind: &'static [[f64; 6]],
    ahead: &'static [[f64; 6]],
    smooth: f64,
}

const STYLES: [Style; 3] = [
    Style {
        name: "shoe",
        behind: &[[0.00, 0.86, 0.56, 0.18, 0.00, 0.85], [0.14, 1.00, 0.68, 0.16, 0.00, 0.85]],
        ahead: &[[0.00, 1.04, 0.70, 0.14, 0.00, 0.78], [0.42, 1.06, 0.49, 0.13, 0.00, 0.66], [0.78, 0.98, 0.39, 0.12, 0.01, 0.62], [1.00, 0.76, 0.31, 0.12, 0.05, 0.62]],
        smooth: 30.0,
    },
    Style {
        name: "sabaton",
        behind: &[[0.00, 0.88, 0.60, 0.07, 0.00, 0.82], [0.14, 1.02, 0.74, 0.07, 0.00, 0.80]],
        ahead: &[[0.00, 1.06, 0.78, 0.07, 0.00, 0.72], [0.40, 1.02, 0.54, 0.07, 0.00, 0.62], [0.76, 0.86, 0.42, 0.07, 0.01, 0.56], [1.00, 0.58, 0.32, 0.07, 0.05, 0.56]],
        smooth: 20.0,
    },
    Style {
        name: "sandal",
        behind: &[[0.00, 0.80, 0.48, 0.06, 0.00, 0.82], [0.14, 0.95, 0.62, 0.06, 0.00, 0.80]],
        ahead: &[[0.00, 1.00, 0.66, 0.06, 0.00, 0.70], [0.45, 0.98, 0.42, 0.06, 0.00, 0.62], [0.80, 0.86, 0.33, 0.06, 0.01, 0.62], [1.00, 0.64, 0.27, 0.06, 0.03, 0.64]],
        smooth: 30.0,
    },
];

const SABATONS: [&str; 13] = ["karm", "gene", "kgre", "kgri", "kpal", "kdar", "kdr1", "kdr2", "lod2", "snyp", "war2", "ksoc", "merc"];
const SANDALS: [&str; 5] = ["dmag", "sorc", "lagz", "baba", "swd2"];

pub fn style_of(path: &str) -> &'static Style {
    let class = path.split('_').nth(2).unwrap_or("");
    let name = if SABATONS.contains(&class) {
        "sabaton"
    } else if SANDALS.contains(&class) {
        "sandal"
    } else {
        "shoe"
    };
    STYLES.iter().find(|style| style.name == name).unwrap()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
enum Material {
    Sole,
    Upper,
}

const SOLE_EDGES: [usize; 3] = [0, 1, 5];

struct Noise(u64);

impl Noise {
    fn new() -> Noise {
        Noise(0x9E37_79B9_7F4A_7C15)
    }

    fn next(&mut self) -> f64 {
        let mut s = self.0;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.0 = s;
        ((s >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0) * NOISE
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

struct Found {
    x: usize,
    y: usize,
    colour: [f64; 3],
    spread: f64,
}

fn farthest(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs()).max((a[2] - b[2]).abs())
}

fn skin(c: [f64; 3]) -> bool {
    let warm = (c[0] - c[2]) / (c[0] + 1.0);
    c[0] >= c[1] && c[1] >= c[2] && warm > 0.12 && warm < 0.55 && c[0] > 70.0
}

fn least(keys: impl Iterator<Item = (f64, usize, usize)>) -> Option<(usize, usize)> {
    keys.min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2))).map(|(_, x, y)| (x, y))
}

struct Sums {
    wide: usize,
    table: Vec<f64>,
}

impl Sums {
    fn new(wide: usize, high: usize, value: impl Fn(usize, usize) -> f64) -> Sums {
        let mut table = vec![0.0; (wide + 1) * (high + 1)];
        for y in 0..high {
            let mut row = 0.0;
            for x in 0..wide {
                row += value(x, y);
                table[(y + 1) * (wide + 1) + x + 1] = table[y * (wide + 1) + x + 1] + row;
            }
        }
        Sums { wide, table }
    }

    fn sum(&self, x: usize, y: usize, w: usize, h: usize) -> f64 {
        let at = |x: usize, y: usize| self.table[y * (self.wide + 1) + x];
        at(x + w, y + h) - at(x, y + h) - at(x + w, y) + at(x, y)
    }
}

struct Texels<'a> {
    tex: &'a Image,
    mask: Vec<bool>,
    inside: Sums,
    colour: [Sums; 3],
    lum: Sums,
    lum2: Sums,
}

fn lum(c: [f64; 3]) -> f64 {
    c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114
}

impl<'a> Texels<'a> {
    fn new(tex: &'a Image, mask: Vec<bool>) -> Texels<'a> {
        let (w, h) = (tex.width, tex.height);
        let channel = |c: usize| Sums::new(w, h, |x, y| tex.at(x, y)[c]);
        Texels {
            inside: Sums::new(w, h, |x, y| mask[y * w + x] as u8 as f64),
            colour: [channel(0), channel(1), channel(2)],
            lum: Sums::new(w, h, |x, y| lum(tex.at(x, y))),
            lum2: Sums::new(w, h, |x, y| lum(tex.at(x, y)).powi(2)),
            tex,
            mask,
        }
    }

    fn plain(&self, sizes: &[(usize, usize)], pick: &dyn Fn(&[Found], (usize, usize)) -> Option<(usize, usize)>) -> Result<Rect, String> {
        let wide = self.tex.width;
        let (mut rows, mut columns) = ((usize::MAX, 0), (usize::MAX, 0));
        for (i, _) in self.mask.iter().enumerate().filter(|(_, &m)| m) {
            let (x, y) = (i % wide, i / wide);
            rows = (rows.0.min(y), rows.1.max(y));
            columns = (columns.0.min(x), columns.1.max(x));
        }
        if rows.0 == usize::MAX {
            return Err("no texels on the shins".into());
        }
        for &(w, h) in sizes {
            let mut found = Vec::new();
            for y in rows.0.max(1)..rows.1.saturating_sub(h) {
                for x in columns.0.max(1)..columns.1.saturating_sub(w) {
                    if self.inside.sum(x - 1, y - 1, w + 2, h + 2) < ((w + 2) * (h + 2)) as f64 - 0.5 {
                        continue;
                    }
                    let n = (w * h) as f64;
                    let colour = [0, 1, 2].map(|c| self.colour[c].sum(x, y, w, h) / n);
                    let mean = self.lum.sum(x, y, w, h) / n;
                    let spread = (self.lum2.sum(x, y, w, h) / n - mean * mean).max(0.0).sqrt();
                    found.push(Found { x, y, colour, spread });
                }
            }
            if let Some((x, y)) = pick(&found, (w, h)) {
                return Ok(Rect { x, y, w, h });
            }
        }
        Err("no plain rectangle on the shins".into())
    }

    fn mean(&self, r: Rect) -> [f64; 3] {
        let n = (r.w * r.h) as f64;
        [0, 1, 2].map(|c| self.colour[c].sum(r.x, r.y, r.w, r.h) / n)
    }

    fn stripe(&self, x: usize, y: usize, (w, h): (usize, usize)) -> f64 {
        let whole = self.mean(Rect { x, y, w, h });
        let columns = (x..x + w).map(|cx| [0, 1, 2].map(|c| (y..y + h).map(|ry| self.tex.at(cx, ry)[c]).sum::<f64>() / h as f64));
        let rows = (y..y + h).map(|ry| [0, 1, 2].map(|c| (x..x + w).map(|cx| self.tex.at(cx, ry)[c]).sum::<f64>() / w as f64));
        columns.chain(rows).map(|line| farthest(line, whole)).fold(0.0, f64::max)
    }
}

fn near(colour: [f64; 3]) -> impl Fn(&[Found], (usize, usize)) -> Option<(usize, usize)> {
    move |found, _| {
        let close: Vec<_> = found.iter().filter(|f| farthest(f.colour, colour) < 16.0).map(|f| (f.spread, f.x, f.y)).collect();
        match close.is_empty() {
            false => least(close.into_iter()),
            true => least(found.iter().map(|f| (farthest(f.colour, colour) + 3.0 * f.spread, f.x, f.y))),
        }
    }
}

fn darkest(found: &[Found], _: (usize, usize)) -> Option<(usize, usize)> {
    least(found.iter().filter(|f| f.spread < 3.0).map(|f| (f.colour.iter().sum::<f64>() / 3.0, f.x, f.y)))
}

struct Patch {
    rects: [(Material, Rect); 2],
    wide: usize,
    high: usize,
    density: f64,
}

struct Frame {
    on_leg: Vec<bool>,
    outline: Vec<V3>,
    cx: f64,
    cz: f64,
    half: f64,
    depth: f64,
    ankle: f64,
    height: f64,
    back: f64,
    front: f64,
}

struct Fit {
    cx: f64,
    cz: f64,
    along: Vec<f64>,
    sections: Vec<[f64; 5]>,
}

struct Foot {
    verts: Vec<V3>,
    normals: Vec<V3>,
    uvs: Vec<[f64; 2]>,
    tris: Vec<[usize; 3]>,
    contact: [V3; 3],
    fit: Fit,
}

struct Body<'a> {
    b: Blob<'a>,
    pos: Vec<V3>,
    uv: Option<Vec<[f64; 2]>>,
    weights: Vec<Vec<f64>>,
    joint: Vec<Vec<i64>>,
    main: Vec<i64>,
    names: Vec<(i64, String)>,
    records: HashMap<String, usize>,
    triangles: Vec<[usize; 3]>,
}

struct Edge {
    x0: i64,
    y0: i64,
    dx: f32,
    top: i64,
    bottom: i64,
}

impl Edge {
    fn x(&self, y: i64) -> f32 {
        (y - self.y0) as f32 * self.dx + self.x0 as f32
    }
}

fn fill(mask: &mut [bool], wide: usize, high: usize, points: [(f64, f64); 3]) {
    let round_up = |v: f32| {
        let v = v as f64;
        (if v >= 0.0 { (v + 0.5).floor() } else { -(v.abs() + 0.5).floor() }) as i64
    };
    let round_down = |v: f32| {
        let v = v as f64;
        (if v >= 0.0 { (v - 0.5).ceil() } else { -(v.abs() - 0.5).ceil() }) as i64
    };
    let (wide_i, high_i) = (wide as i64, high as i64);
    let mut hline = |x0: i64, y: i64, x1: i64| {
        if y < 0 || y >= high_i || x1 < 0 || x0 >= wide_i {
            return;
        }
        for x in x0.max(0)..=x1.min(wide_i - 1) {
            mask[y as usize * wide + x as usize] = true;
        }
    };
    let corners = points.map(|(x, y)| (x as i64, y as i64));
    let (mut top, mut bottom) = (high_i - 1, 0);
    let mut slanted = Vec::new();
    for i in 0..3 {
        let ((x0, y0), (x1, y1)) = (corners[i], corners[(i + 1) % 3]);
        if i == 2 && corners[2] == corners[0] {
            continue;
        }
        let (lo, hi) = (y0.min(y1), y0.max(y1));
        (top, bottom) = (top.min(lo), bottom.max(hi));
        if y0 == y1 {
            hline(x0.min(x1), y0, x0.max(x1));
        } else {
            slanted.push(Edge { x0, y0, dx: (x1 - x0) as f32 / (y1 - y0) as f32, top: lo, bottom: hi });
        }
    }
    let (top, bottom) = (top.max(0), bottom.min(high_i));
    for y in top..=bottom {
        let mut xs: Vec<f32> = Vec::with_capacity(2 * slanted.len());
        for (i, edge) in slanted.iter().enumerate() {
            if y < edge.top || y > edge.bottom {
                continue;
            }
            let x = edge.x(y);
            xs.push(x);
            if y == edge.bottom && y < bottom {
                xs.push(x);
            } else if edge.dx != 0.0 && x.round() == x {
                for (k, other) in slanted[..i].iter().enumerate() {
                    if (edge.dx > 0.0 && other.dx <= 0.0) || (edge.dx < 0.0 && other.dx >= 0.0) {
                        continue;
                    }
                    let meet = (y == edge.top && y == other.top) || (y == edge.bottom && y == other.bottom);
                    if meet && x == other.x(y) {
                        let next = if y == bottom { y - 1 } else { y + 1 };
                        let (here, there) = (edge.x(next), other.x(next));
                        xs[k] = match (y == edge.bottom, edge.dx > 0.0) {
                            (true, true) => here.max(there) + 1.0,
                            (true, false) => here.min(there) - 1.0,
                            (false, true) => here.min(there),
                            (false, false) => here.max(there) + 1.0,
                        };
                        break;
                    }
                }
            }
        }
        xs.sort_by(f32::total_cmp);
        for pair in xs.chunks_exact(2) {
            hline(round_up(pair[0]), y, round_down(pair[1]));
        }
    }
}

impl Body<'_> {
    fn joints(&self, wanted: &[String]) -> Vec<i64> {
        self.names.iter().filter(|(_, name)| wanted.contains(name)).map(|(joint, _)| *joint).collect()
    }

    fn knee(&self, leg: char) -> Result<f64, String> {
        let at = *self.records.get(&format!("{leg}_leg2")).ok_or("no shin")? + INVERSE_BIND;
        let m = |r: usize, c: usize| self.b.f32(at + 4 * (4 * r + c));
        Ok(-(0..3).map(|r| m(r, 1) * m(r, 3)).sum::<f64>())
    }

    fn frame(&self, leg: char) -> Result<Frame, String> {
        let bones = self.joints(&[format!("{leg}_leg2"), format!("{leg}_leg3")]);
        let n = self.pos.len();
        let mut on_leg: Vec<bool> = (0..n)
            .map(|i| {
                let own: f64 = self.joint[i].iter().zip(&self.weights[i]).filter(|(j, _)| bones.contains(j)).map(|(_, w)| w).sum();
                bones.contains(&self.main[i]) && own >= OWN * self.weights[i].iter().sum::<f64>()
            })
            .collect();
        let height = self.knee(leg)?;
        let (lo, hi) = (BAND.0 * height, BAND.1 * height);
        let end: Vec<usize> = (0..n).filter(|&i| on_leg[i] && self.pos[i][1] < END * height).collect();
        if end.len() >= 3 {
            let (first, last) = bounds(end.iter().map(|&i| self.pos[i]));
            let grown = mul(sub(last, first), 0.5);
            for (i, on) in on_leg.iter_mut().enumerate() {
                let p = self.pos[i];
                *on &= p[0] >= first[0] - grown[0] && p[0] <= last[0] + grown[0] && p[2] >= first[2] - grown[2] && p[2] <= last[2] + grown[2];
            }
        }
        let tris: Vec<[usize; 3]> = self.triangles.iter().copied().filter(|t| t.iter().all(|&i| on_leg[i])).collect();
        let crossings = |y: f64| {
            let mut found = Vec::new();
            for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                for t in &tris {
                    let (a, c) = (self.pos[t[i]], self.pos[t[j]]);
                    if (a[1] - y) * (c[1] - y) < 0.0 {
                        found.push(add(a, mul(sub(c, a), (y - a[1]) / (c[1] - a[1]))));
                    }
                }
            }
            found
        };
        let sections = |bottom: f64, top: f64| -> Vec<Vec<V3>> {
            (0..7).map(|k| if k == 6 { top } else { bottom + (top - bottom) / 6.0 * k as f64 }).map(crossings).filter(|cut| cut.len() >= 4).collect()
        };
        let (mut cuts, mut robed) = (sections(lo, hi), false);
        if cuts.len() < 2 {
            (cuts, robed) = (sections(END * height, lo), true);
        }
        if cuts.len() < 2 {
            return Err(format!("the {leg} shin has no shaft between {:.2} and {hi:.2}", END * height));
        }
        let extents: Vec<(V3, V3)> = cuts.iter().map(|cut| bounds(cut.iter().copied())).collect();
        let first: V3 = [0, 1, 2].map(|c| median(&mut extents.iter().map(|e| e.0[c]).collect::<Vec<_>>()));
        let last: V3 = [0, 1, 2].map(|c| median(&mut extents.iter().map(|e| e.1[c]).collect::<Vec<_>>()));
        let (cx, cz) = ((first[0] + last[0]) / 2.0, (first[2] + last[2]) / 2.0);
        let (half, depth) = ((last[0] - first[0]) / 2.0, last[2] - first[2]);
        let heights = cuts.iter().map(|cut| cut[0][1]);
        let ankle = 0.9 * if robed { heights.fold(f64::NEG_INFINITY, f64::max) } else { heights.fold(f64::INFINITY, f64::min) };
        let mut shown: Vec<f64> =
            (0..n).filter(|&i| on_leg[i] && self.pos[i][1] > SHOWN * ankle && self.pos[i][1] < hi).map(|i| self.pos[i][2]).collect();
        let (back, front) = match shown.len() >= 4 {
            true => (first[2].min(percentile(&mut shown, 5.0)), last[2].max(percentile(&mut shown, 95.0))),
            false => (first[2], last[2]),
        };
        let half = (half * 0.95).min(WIDEST * height / 2.0);
        Ok(Frame { on_leg, outline: cuts.swap_remove(0), cx, cz, half, depth, ankle, height, back, front })
    }

    fn texels<'t>(&self, chosen: impl Iterator<Item = &'t [usize; 3]>, wide: usize, high: usize) -> Vec<bool> {
        let uv = self.uv.as_ref().unwrap();
        let mut mask = vec![false; wide * high];
        for t in chosen {
            fill(&mut mask, wide, high, t.map(|i| (uv[i][0] * wide as f64, (1.0 - uv[i][1]) * high as f64)));
        }
        mask
    }

    fn materials(&self, tex: &Image, frames: &[Frame; 2], style: &Style) -> Result<Patch, String> {
        let uv = self.uv.as_ref().ok_or("no texture coordinates")?;
        let (wide, high) = (tex.width, tex.height);
        let shins = self.joints(&["l_leg2", "l_leg3", "r_leg2", "r_leg3"].map(String::from));
        let tris: Vec<[usize; 3]> = self.triangles.iter().copied().filter(|t| t.iter().all(|&i| shins.contains(&self.main[i]))).collect();
        let (mut area3, mut area2) = (0.0, 0.0);
        for t in &tris {
            let p = t.map(|i| self.pos[i]);
            area3 += norm(cross(sub(p[1], p[0]), sub(p[2], p[0])));
            let q = t.map(|i| [uv[i][0] * wide as f64, uv[i][1] * high as f64]);
            let (e1, e2) = ([q[1][0] - q[0][0], q[1][1] - q[0][1]], [q[2][0] - q[0][0], q[2][1] - q[0][1]]);
            area2 += (e1[0] * e2[1] - e1[1] * e2[0]).abs();
        }
        let density = (area2 / area3).sqrt();
        let mask = self.texels(tris.iter(), wide, high);
        let ankle = frames[0].ankle.min(frames[1].ankle);
        let middle = |t: &[usize; 3]| t.iter().map(|&i| self.pos[i][1]).sum::<f64>() / 3.0;
        let mut lowest = self.texels(tris.iter().filter(|t| middle(t) > 0.35 * ankle && middle(t) < 1.1 * ankle), wide, high);
        if !lowest.contains(&true) {
            lowest = self.texels(tris.iter().filter(|t| t.iter().any(|&i| self.pos[i][1] < ankle)), wide, high);
        }
        let low_texels: Vec<[f64; 3]> = match lowest.contains(&true) {
            true => (0..wide * high).filter(|&i| lowest[i]).map(|i| tex.rgb[i]).collect(),
            false => (0..wide * high).filter(|&i| mask[i]).map(|i| tex.rgb[i]).collect(),
        };
        if low_texels.is_empty() {
            return Err("no texels on the shins".into());
        }
        let usual = [0, 1, 2].map(|c| median(&mut low_texels.iter().map(|t| t[c]).collect::<Vec<_>>()));
        let texels = Texels::new(tex, mask);
        let large = [(32, 14), (28, 12), (24, 12), (20, 10), (16, 8)];
        let small = [(24, 10), (16, 8), (12, 6)];
        let more = |sizes: &[(usize, usize)]| [sizes, &[(12, 6), (8, 4)]].concat();
        let grained = |colour: [f64; 3]| {
            let texels = &texels;
            move |found: &[Found], size: (usize, usize)| {
                let mut close: Vec<(f64, usize, usize)> = found.iter().filter(|f| farthest(f.colour, colour) < 16.0).map(|f| (f.spread, f.x, f.y)).collect();
                close.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
                let flattest = close.first().map_or(0.0, |c| c.0);
                close.iter().rev().find(|c| c.0 <= flattest + 2.5 && texels.stripe(c.1, c.2, size) < 4.0).map(|c| (c.1, c.2))
            }
        };
        let upper = |colour: [f64; 3], sizes: &[(usize, usize)]| texels.plain(&more(sizes), &grained(colour)).or_else(|_| texels.plain(sizes, &near(colour)));
        let rects = if style.name == "sandal" {
            let bare = low_texels.iter().filter(|&&c| skin(c)).count() as f64 / low_texels.len() as f64;
            let bare_foot = |found: &[Found], _: (usize, usize)| {
                least(found.iter().filter(|f| skin(f.colour) && f.spread < 6.0).map(|f| (farthest(f.colour, usual) + f.spread, f.x, f.y)))
            };
            let foot = match bare >= 0.3 {
                true => texels.plain(&more(&large), &bare_foot),
                false => upper(usual, &large),
            };
            let foot = foot.or_else(|_| upper(usual, &large))?;
            [(Material::Upper, foot), (Material::Sole, texels.plain(&small, &near(usual))?)]
        } else {
            [(Material::Upper, upper(usual, &large)?), (Material::Sole, texels.plain(&small, &darkest)?)]
        };
        Ok(Patch { rects, wide, high, density })
    }

    fn foot(&self, frame: &Frame, style: &Style, patch: Option<&Patch>) -> Result<Foot, String> {
        let Frame { cx, cz, half, depth, ankle, height, back, .. } = *frame;
        let length = (LENGTH_PER_DEPTH * depth).clamp(LENGTH.0 * height, LENGTH.1 * height);
        let heel = back - cz - HEEL * length;
        let front = (frame.front - cz).max(heel + 0.2 * length);
        let toe = (heel + length).max(front + AHEAD_LEAST * length);
        let sections: Vec<[f64; 6]> = style.behind.iter().chain(style.ahead).copied().collect();
        let along: Vec<f64> =
            style.behind.iter().map(|s| heel + s[0] * length).chain(style.ahead.iter().map(|s| front + s[0] * (toe - front))).collect();
        let outline = |half: f64, top: f64, sole: f64, bottom: f64, shoulder: f64| {
            [(cx - half, bottom), (cx + half, bottom), (cx + half, sole), (cx + shoulder * half, top), (cx - shoulder * half, top), (cx - half, sole)]
        };
        let points: Vec<V3> = along
            .iter()
            .zip(&sections)
            .flat_map(|(&z, s)| outline(half * s[1], s[2] * ankle, s[3] * ankle, s[4] * ankle, s[5]).map(|(x, y)| [x, y, cz + z]))
            .collect();
        let rings = sections.len();
        let contact = [
            [cx, ankle, cz],
            [cx, sections[rings - 1][4] * ankle - LIFT, cz + along[rings - 1]],
            [cx, sections[0][4] * ankle - LIFT, cz + along[0]],
        ];
        let faces = foot_faces(&points, rings);
        let (tris, made_of, normal) = oriented_triangles(&points, &faces);
        let (verts, normals, materials, corners) = smoothed_vertices(&points, &tris, &made_of, &normal, style.smooth);
        let uvs = match patch {
            Some(patch) => {
                let bounds = FootBounds {
                    outward: [if cx >= 0.0 { 1.0 } else { -1.0 }, 1.0, 1.0],
                    length: along[rings - 1] - along[0],
                    width: 2.0 * half * sections.iter().map(|s| s[1]).fold(f64::NEG_INFINITY, f64::max),
                    tall: ankle * sections.iter().map(|s| s[2]).fold(f64::NEG_INFINITY, f64::max),
                };
                let middle = [cx, bounds.tall / 2.0, cz + (along[rings - 1] + along[0]) / 2.0];
                patch_uvs(patch, &verts, &normals, &materials, &bounds, middle)
            }
            None => self.boot_uvs(frame, &verts)?,
        };
        let fit = Fit { cx, cz, along, sections: sections.iter().map(|s| [half * s[1], s[2] * ankle, s[3] * ankle, s[4] * ankle, s[5]]).collect() };
        Ok(Foot { verts, normals, uvs, tris: corners, contact, fit })
    }

    fn boot_uvs(&self, frame: &Frame, verts: &[V3]) -> Result<Vec<[f64; 2]>, String> {
        let uv = self.uv.as_ref().ok_or("no texture coordinates")?;
        let boot: Vec<usize> = (0..self.pos.len()).filter(|&i| frame.on_leg[i] && self.pos[i][1] < 1.4).collect();
        if boot.is_empty() {
            return Err("no boot to take texture coordinates from".into());
        }
        let distance = |v: V3, i: usize| dot(sub(v, self.pos[i]), sub(v, self.pos[i]));
        Ok(verts.iter().map(|&v| uv[*boot.iter().min_by(|&&a, &&b| distance(v, a).total_cmp(&distance(v, b))).unwrap()]).collect())
    }
}

const RING_POINTS: usize = 6;

enum Outward {
    FromPoint(V3),
    AlongZ(f64),
}

struct FootBounds {
    outward: V3,
    length: f64,
    width: f64,
    tall: f64,
}

fn foot_faces(points: &[V3], rings: usize) -> Vec<([usize; 4], Material, Outward)> {
    let n = RING_POINTS;
    let mut faces: Vec<([usize; 4], Material, Outward)> = Vec::new();
    for r in 0..rings - 1 {
        let middle = mul(points[r * n..(r + 2) * n].iter().fold([0.0; 3], |sum, &p| add(sum, p)), 1.0 / (2 * n) as f64);
        for k in 0..n {
            let (a, b) = (r * n + k, r * n + (k + 1) % n);
            let material = if SOLE_EDGES.contains(&k) { Material::Sole } else { Material::Upper };
            faces.push(([a, b, b + n, a + n], material, Outward::FromPoint(middle)));
        }
    }
    for (base, out) in [(0, -1.0), ((rings - 1) * n, 1.0)] {
        for (quad, material) in [([0, 1, 2, 5], Material::Sole), ([5, 2, 3, 4], Material::Upper)] {
            faces.push((quad.map(|i| base + i), material, Outward::AlongZ(out)));
        }
    }
    faces
}

fn oriented_triangles(points: &[V3], faces: &[([usize; 4], Material, Outward)]) -> (Vec<[usize; 3]>, Vec<Material>, Vec<V3>) {
    let (mut tris, mut made_of, mut normal) = (Vec::new(), Vec::new(), Vec::new());
    for ([a, b, c, d], material, out) in faces {
        for tri in [[*a, *b, *c], [*a, *c, *d]] {
            let p = tri.map(|i| points[i]);
            let mut facing = cross(sub(p[1], p[0]), sub(p[2], p[0]));
            let outward = match out {
                Outward::FromPoint(middle) => sub(mul(add(add(p[0], p[1]), p[2]), 1.0 / 3.0), *middle),
                Outward::AlongZ(o) => [0.0, 0.0, *o],
            };
            let tri = if dot(facing, outward) < 0.0 {
                facing = mul(facing, -1.0);
                [tri[0], tri[2], tri[1]]
            } else {
                tri
            };
            tris.push(tri);
            made_of.push(*material);
            normal.push(mul(facing, 1.0 / norm(facing)));
        }
    }
    (tris, made_of, normal)
}

fn smoothed_vertices(points: &[V3], tris: &[[usize; 3]], made_of: &[Material], normal: &[V3], smooth_degrees: f64) -> (Vec<V3>, Vec<V3>, Vec<Material>, Vec<[usize; 3]>) {
    let least_cos = smooth_degrees.to_radians().cos();
    let (mut verts, mut normals, mut materials, mut corners) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut made: HashMap<(usize, Material, [i64; 3]), usize> = HashMap::new();
    for (t, tri) in tris.iter().enumerate() {
        let mut new = [0; 3];
        for (c, &point) in tri.iter().enumerate() {
            let sum = (0..tris.len()).filter(|&g| tris[g].contains(&point) && dot(normal[g], normal[t]) >= least_cos).fold([0.0; 3], |sum, g| add(sum, normal[g]));
            let vertex_normal = mul(sum, 1.0 / norm(sum));
            let key = (point, made_of[t], vertex_normal.map(|v| (v * 1e4).round_ties_even() as i64));
            new[c] = *made.entry(key).or_insert_with(|| {
                verts.push(points[point]);
                normals.push(vertex_normal);
                materials.push(made_of[t]);
                verts.len() - 1
            });
        }
        corners.push(new);
    }
    (verts, normals, materials, corners)
}

fn patch_uvs(patch: &Patch, verts: &[V3], normals: &[V3], materials: &[Material], bounds: &FootBounds, middle: V3) -> Vec<[f64; 2]> {
    let FootBounds { outward, length, width, tall } = *bounds;
    let mut noise = Noise::new();
    let jitter: Vec<[f64; 2]> = (0..verts.len()).map(|_| [noise.next(), noise.next()]).collect();
    let mut uvs = vec![[0.0; 2]; verts.len()];
    for &(material, Rect { x, y, w, h }) in &patch.rects {
        let (x0, y0, w, h) = (x as f64, y as f64, w as f64, h as f64);
        let scale = patch.density.min((w - 1.0) / length.max(width)).min((h - 1.0) / width.max(tall));
        for i in (0..verts.len()).filter(|&i| materials[i] == material) {
            let a = normals[i].map(f64::abs);
            let facing = if a[0] >= a[1] && a[0] >= a[2] { 0 } else if a[1] >= a[2] { 1 } else { 2 };
            let (u_axis, v_axis) = [(2, 1), (2, 0), (0, 1)][facing];
            let tx = x0 + w / 2.0 + (verts[i][u_axis] - middle[u_axis]) * outward[u_axis] * scale;
            let ty = y0 + h / 2.0 + (verts[i][v_axis] - middle[v_axis]) * outward[v_axis] * scale;
            let tx = (tx + jitter[i][0]).clamp(x0 + 1.0, x0 + w - 1.0);
            let ty = (ty + jitter[i][1]).clamp(y0 + 1.0, y0 + h - 1.0);
            uvs[i] = [tx / patch.wide as f64, 1.0 - ty / patch.high as f64];
        }
    }
    uvs
}

fn inside(q: V3, fit: &Fit, margin: f64) -> Option<V3> {
    let along = &fit.along;
    let z = (q[2] - fit.cz).clamp(along[0] + margin, along[along.len() - 1] - margin);
    let k = along.iter().position(|&a| a >= z).unwrap_or(along.len()).clamp(1, along.len() - 1);
    let t = (z - along[k - 1]) / (along[k] - along[k - 1]);
    let [w, top, sole, _, shoulder] = [0, 1, 2, 3, 4].map(|c| (1.0 - t) * fit.sections[k - 1][c] + t * fit.sections[k][c]);
    if q[1] >= top - margin {
        return None;
    }
    let y = q[1].max(sole + margin);
    let width = if y <= sole { w } else { w - (1.0 - shoulder) * w * (y - sole) / (top - sole) };
    Some([fit.cx + (q[0] - fit.cx).clamp(-0.9 * width, 0.9 * width), y, fit.cz + z])
}

fn put_u32(raw: &mut [u8], at: usize, value: u32) {
    raw[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_i32(raw: &mut [u8], at: usize, value: i64) {
    raw[at..at + 4].copy_from_slice(&(value as i32).to_le_bytes());
}

fn put_f32s(raw: &mut [u8], at: usize, values: &[f64]) {
    for (k, v) in values.iter().enumerate() {
        raw[at + 4 * k..at + 4 * k + 4].copy_from_slice(&(*v as f32).to_le_bytes());
    }
}

fn pad(raw: &mut Vec<u8>, n: usize) {
    raw.resize(raw.len().next_multiple_of(n), 0);
}

pub fn build(path: &str, model: &[u8], texture: Option<&Image>) -> Result<(Vec<u8>, &'static str), String> {
    let b = Blob(model);
    let (_, skeleton, shape) = model::parts(&b)?;
    let (body, v) = read_body(&b, model, skeleton, shape)?;
    let shins = shin_joints(&body)?;
    let style = style_of(path);
    let frames = [body.frame('l')?, body.frame('r')?];
    let patch = match texture {
        Some(texture) => Some(body.materials(texture, &frames, style)?),
        None => None,
    };
    let feet = [body.foot(&frames[0], style, patch.as_ref())?, body.foot(&frames[1], style, patch.as_ref())?];

    let mut moved = tucked_leg_ends(&body, &frames, v.count);
    let ends: BTreeSet<usize> = moved.keys().copied().collect();
    pull_inside_feet(&body, &frames, &feet, v.count, &mut moved);
    let mut after = body.pos.clone();
    for (&i, &q) in &moved {
        after[i] = q;
    }
    let attr = |usage: u32| -> Result<Attr, String> { v.attrs.get(&usage).copied().ok_or_else(|| format!("no vertex attribute {usage}")) };
    let (position, normal) = (attr(model::POSITION)?, attr(model::NORMAL)?);
    if position.format != FLOAT || normal.format != FLOAT {
        return Err("positions or normals that aren't floats".into());
    }
    let mut raw = model.to_vec();
    let touching = write_moved_vertices(&mut raw, &v, position, normal, &body.triangles, &after, &moved);
    if style.name != "sandal" {
        copy_end_uvs(&mut raw, &v, &frames, &ends, &touching, &after)?;
    }
    let foot_bones = add_foot_bones(&mut raw, &b, skeleton, &body, shins, &feet)?;
    let (uv_attr, bones_attr, weights_attr) = (attr(model::UV)?, attr(model::BONES)?, attr(model::WEIGHTS)?);
    if uv_attr.format != FLOAT {
        return Err("texture coordinates that aren't floats".into());
    }
    let (new_vertices, new_indices) = foot_geometry(&feet, &v, [position, normal, uv_attr], bones_attr, weights_attr);
    if new_indices.iter().any(|&i| i > u16::MAX as usize) {
        return Err("too many vertices for 16-bit indices".into());
    }
    append_feet_mesh(&mut raw, &b, &v, shape, foot_bones, &new_vertices, &new_indices)?;
    pad(&mut raw, 0x80);
    write_file_sizes(&mut raw, &b)?;
    Ok((raw, style.name))
}

fn read_body<'a>(b: &Blob<'a>, model: &'a [u8], skeleton: usize, shape: usize) -> Result<(Body<'a>, model::Vertices), String> {
    let mut names = Vec::new();
    let mut records = HashMap::new();
    for (name, at) in b.dict(b.rel(skeleton + 0x1C))? {
        names.push((b.i32(at + 8) as i64, name.clone()));
        records.insert(name, at);
    }
    let v = model::vertices(b, shape)?;
    let subs = model::submeshes(b, shape)?;
    let (Some(local), Some(weights)) = (&v.bones, &v.weights) else {
        return Err("its vertices are each one bone's, which this builder doesn't take".into());
    };
    let n = v.count;
    if subs.iter().flat_map(|sm| sm.triangles.iter().flatten()).any(|&i| i >= n) {
        return Err("a triangle outside the vertex buffer".into());
    }
    let mut joint = vec![vec![-1i64; local.first().map_or(0, Vec::len)]; n];
    for sm in &subs {
        let used: BTreeSet<usize> = sm.triangles.iter().flatten().copied().collect();
        for &i in &used {
            for (k, &slot) in local[i].iter().enumerate() {
                joint[i][k] = *sm.palette.get(slot as usize).ok_or("a bone outside its palette")? as i64;
            }
        }
    }
    let main = (0..n)
        .map(|i| {
            let w = &weights[i];
            let k = (0..w.len()).fold(0, |best, k| if w[k] > w[best] { k } else { best });
            joint[i].get(k).copied().unwrap_or(-1)
        })
        .collect();
    let body = Body {
        b: Blob(model),
        pos: v.position.clone(),
        uv: v.uv.clone(),
        weights: weights.clone(),
        joint,
        main,
        names,
        records,
        triangles: subs.iter().flat_map(|sm| sm.triangles.iter().copied()).collect(),
    };
    Ok((body, v))
}

fn shin_joints(body: &Body) -> Result<[i64; 2], String> {
    let shin = |leg: char| body.names.iter().find(|(_, name)| *name == format!("{leg}_leg2")).map(|(joint, _)| *joint);
    let (Some(left), Some(right)) = (shin('l'), shin('r')) else {
        return Err("no l_leg2 and r_leg2, the shins the feet hang on".into());
    };
    Ok([left, right])
}

fn tucked_leg_ends(body: &Body, frames: &[Frame; 2], vertex_count: usize) -> BTreeMap<usize, V3> {
    let pos = &body.pos;
    let mut moved: BTreeMap<usize, V3> = BTreeMap::new();
    for frame in frames {
        let low: Vec<usize> = (0..vertex_count).filter(|&i| frame.on_leg[i] && pos[i][1] < LOW * frame.ankle).collect();
        if low.is_empty() {
            continue;
        }
        let outline = &frame.outline;
        let (o_lo, o_hi) = bounds(outline.iter().copied());
        let o_mid = mul(add(o_lo, o_hi), 0.5);
        let o_size = sub(o_hi, o_lo).map(|d| (d / 2.0).max(1e-3));
        let (e_lo, e_hi) = bounds(low.iter().map(|&i| pos[i]));
        let e_mid = mul(add(e_lo, e_hi), 0.5);
        let e_size = [0, 1, 2].map(|c| ((e_hi[c] - e_lo[c]) / 2.0).max(0.3 * o_size[c]));
        let around: Vec<f64> = outline.iter().map(|o| ((o[2] - o_mid[2]) / o_size[2]).atan2((o[0] - o_mid[0]) / o_size[0])).collect();
        for &i in &low {
            let q = pos[i];
            let angle = ((q[2] - e_mid[2]) / e_size[2]).atan2((q[0] - e_mid[0]) / e_size[0]);
            let off = |a: f64| {
                let d = a - angle;
                d.sin().atan2(d.cos()).abs()
            };
            let nearest = (0..around.len()).fold(0, |best, k| if off(around[k]) < off(around[best]) { k } else { best });
            let target = outline[nearest];
            let x = o_mid[0] + (target[0] - o_mid[0]) * 0.95;
            let z = o_mid[2] + (target[2] - o_mid[2]) * 0.95;
            moved.insert(i, [x, TUCK * frame.ankle, z]);
        }
    }
    moved
}

fn pull_inside_feet(body: &Body, frames: &[Frame; 2], feet: &[Foot; 2], vertex_count: usize, moved: &mut BTreeMap<usize, V3>) {
    let pos = &body.pos;
    for (frame, foot) in frames.iter().zip(feet) {
        for i in (0..vertex_count).filter(|&i| frame.on_leg[i]) {
            let q = moved.get(&i).copied().unwrap_or(pos[i]);
            if let Some(put) = inside(q, &foot.fit, 0.02 * frame.height) {
                if (0..3).any(|c| (put[c] - q[c]).abs() > 1e-5 + 1e-5 * q[c].abs()) {
                    moved.insert(i, put);
                }
            }
        }
    }
}

fn write_moved_vertices(raw: &mut [u8], v: &model::Vertices, position: Attr, normal: Attr, triangles: &[[usize; 3]], after: &[V3], moved: &BTreeMap<usize, V3>) -> Vec<[usize; 3]> {
    let touching: Vec<[usize; 3]> = triangles.iter().copied().filter(|t| t.iter().any(|i| moved.contains_key(i))).collect();
    let mut sums: HashMap<usize, V3> = HashMap::new();
    for t in &touching {
        let p = t.map(|i| after[i]);
        let face = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let face = mul(face, 1.0 / (norm(face) + 1e-12));
        for i in t.iter().copied().collect::<BTreeSet<_>>() {
            let sum = sums.entry(i).or_insert([0.0; 3]);
            *sum = add(*sum, face);
        }
    }
    for (&i, &q) in moved {
        let sum = sums.get(&i).copied().unwrap_or([0.0; 3]);
        let vertex_normal = mul(sum, 1.0 / (norm(sum) + 1e-12));
        let at = v.data_at + i * v.stride;
        put_f32s(raw, at + position.offset, &q.map(|c| c / position.scale));
        put_f32s(raw, at + normal.offset, &vertex_normal.map(|c| c / normal.scale));
    }
    touching
}

fn copy_end_uvs(raw: &mut [u8], v: &model::Vertices, frames: &[Frame; 2], ends: &BTreeSet<usize>, touching: &[[usize; 3]], after: &[V3]) -> Result<(), String> {
    let Some(uv) = v.attrs.get(&model::UV) else {
        return Ok(());
    };
    let width = model::width(uv.format)? * uv.count;
    for frame in frames {
        let reach = 0.3 * frame.depth.max(2.0 * frame.half);
        for &i in ends.iter().filter(|&&i| frame.on_leg[i]) {
            let above: BTreeSet<usize> = touching.iter().filter(|t| t.contains(&i)).flatten().copied().filter(|j| !ends.contains(j)).collect();
            let flat = |j: usize| (after[j][0] - after[i][0]).powi(2) + (after[j][2] - after[i][2]).powi(2);
            let Some(j) = above.iter().copied().fold(None, |best: Option<usize>, j| match best {
                Some(best) if flat(best) <= flat(j) => Some(best),
                _ => Some(j),
            }) else {
                continue;
            };
            if flat(j).sqrt() > reach {
                continue;
            }
            let (from, to) = (v.data_at + j * v.stride + uv.offset, v.data_at + i * v.stride + uv.offset);
            raw.copy_within(from..from + width, to);
        }
    }
    Ok(())
}

fn add_foot_bones(raw: &mut Vec<u8>, b: &Blob, skeleton: usize, body: &Body, shins: [i64; 2], feet: &[Foot; 2]) -> Result<[u32; 2], String> {
    let old_dict = b.to(skeleton + 0x1C)?;
    let entries = b.dict(Some(old_dict))?;
    let count = b.u32(skeleton + 0x18) as usize;
    if count != entries.len() {
        return Err("a skeleton whose dictionary doesn't count its bones".into());
    }
    let mut name_at: HashMap<String, usize> = HashMap::new();
    for i in 1..=count {
        let at = b.to(old_dict + 0x0C + 0x10 * i + 8)?;
        name_at.insert(b.cstr(at), at);
    }
    let mut data_at: HashMap<String, usize> = entries.iter().cloned().collect();
    pad(raw, 4);
    for leg in ["l", "r"] {
        name_at.insert(format!("{leg}_shoe"), raw.len());
        raw.extend_from_slice(format!("{leg}_shoe\0").as_bytes());
    }
    pad(raw, 4);
    let mut foot_bones = [0u32; 2];
    for (k, leg) in ["l", "r"].into_iter().enumerate() {
        let shin_at = body.records[&format!("{leg}_leg2")];
        let at = raw.len();
        let mut record = raw[shin_at..shin_at + 0xE0].to_vec();
        put_i32(&mut record, 0, name_at[&format!("{leg}_shoe")] as i64 - at as i64);
        put_u32(&mut record, 4, b.u32(shin_at + 4) | SKINNED);
        put_u32(&mut record, 8, (count + k) as u32);
        put_u32(&mut record, 0x0C, shins[k] as u32);
        put_i32(&mut record, 0x10, shin_at as i64 - (at + 0x10) as i64);
        record[0x14..0x20].fill(0);
        put_f32s(&mut record, INVERSE_BIND, &feet[k].contact.concat());
        record[INVERSE_BIND + 36..INVERSE_BIND + 40].copy_from_slice(TAG);
        record[INVERSE_BIND + 40..INVERSE_BIND + 48].fill(0);
        record[0xD8..0xE0].fill(0);
        raw.extend(record);
        data_at.insert(format!("{leg}_shoe"), at);
        foot_bones[k] = (count + k) as u32;
    }
    pad(raw, 4);
    let dict_at = raw.len();
    let all: Vec<String> = entries.iter().map(|(name, _)| name.clone()).chain(["l_shoe".to_owned(), "r_shoe".to_owned()]).collect();
    let nodes = formats::dict_nodes(&all)?;
    raw.extend(formats::dict_bytes(&nodes, dict_at, &name_at, &data_at));
    put_u32(raw, skeleton + 0x18, (count + 2) as u32);
    put_i32(raw, skeleton + 0x1C, dict_at as i64 - (skeleton + 0x1C) as i64);
    Ok(foot_bones)
}

fn foot_geometry(feet: &[Foot; 2], v: &model::Vertices, [position, normal, uv]: [Attr; 3], bones: Attr, weights: Attr) -> (Vec<u8>, Vec<usize>) {
    let mut new_vertices = Vec::new();
    let mut new_indices: Vec<usize> = Vec::new();
    for (slot, foot) in feet.iter().enumerate() {
        let base = v.count + new_vertices.len() / v.stride;
        for ((p, nrm), t) in foot.verts.iter().zip(&foot.normals).zip(&foot.uvs) {
            let mut vertex = vec![0u8; v.stride];
            for (a, values) in [(position, &p[..]), (normal, &nrm[..]), (uv, &t[..])] {
                let values: Vec<f64> = values[..a.count.min(values.len())].iter().map(|x| x / a.scale).collect();
                put_f32s(&mut vertex, a.offset, &values);
            }
            vertex[bones.offset..bones.offset + bones.count].fill(0);
            vertex[bones.offset] = slot as u8;
            vertex[weights.offset..weights.offset + weights.count].fill(0);
            vertex[weights.offset] = 100;
            new_vertices.extend(vertex);
        }
        new_indices.extend(foot.tris.iter().flatten().map(|&i| i + base));
    }
    (new_vertices, new_indices)
}

fn append_feet_mesh(raw: &mut Vec<u8>, b: &Blob, v: &model::Vertices, shape: usize, foot_bones: [u32; 2], new_vertices: &[u8], new_indices: &[usize]) -> Result<(), String> {
    pad(raw, 0x80);
    let vertex_at = raw.len();
    raw.extend_from_within(v.data_at..v.data_at + v.size);
    raw.extend(new_vertices);
    pad(raw, 0x10);
    let index_at = raw.len();
    for &i in new_indices {
        raw.extend_from_slice(&(i as u16).to_le_bytes());
    }
    put_u32(raw, v.buffer + 0x14, (v.size + new_vertices.len()) as u32);
    put_i32(raw, v.buffer + 0x18, vertex_at as i64 - (v.buffer + 0x18) as i64);

    let sets = b.u32(shape + 0x2C) as usize;
    let old_list = b.to(shape + 0x30)?;
    let first = b.to(old_list)?;
    let first_primitive = b.to(b.to(first + 0x10)?)?;
    let first_stream = b.to(b.to(first_primitive + 4)?)?;
    if b.rel(first_primitive + 0x18) != Some(first_primitive + 0x20) {
        return Err("the first set's index stream isn't right after its primitive".into());
    }
    pad(raw, 4);
    let feet_set = raw.len();
    let mut set = vec![0u8; 0x14 + 4 * 2 + 4];
    for (k, value) in [2, 0x10, b.u32(first + 8), 1, 0x14 + 4 * 2 - 0x10].into_iter().enumerate() {
        put_u32(&mut set, 4 * k, value);
    }
    put_u32(&mut set, 0x14, foot_bones[0]);
    put_u32(&mut set, 0x18, foot_bones[1]);
    put_u32(&mut set, 0x1C, 4);
    set.extend_from_slice(&raw[first_primitive..first_primitive + 0x20]);
    let stream = set.len();
    set.extend_from_slice(&raw[first_stream..first_stream + 0x2C]);
    put_u32(&mut set, stream + 8, (2 * new_indices.len()) as u32);
    put_i32(&mut set, stream + 12, index_at as i64 - (feet_set + stream + 12) as i64);
    raw.extend(set);
    let new_list = raw.len();
    for i in 0..sets {
        let to = b.to(old_list + 4 * i)? as i64 - (new_list + 4 * i) as i64;
        raw.extend_from_slice(&(to as i32).to_le_bytes());
    }
    raw.extend_from_slice(&((feet_set as i64 - (new_list + 4 * sets) as i64) as i32).to_le_bytes());
    put_u32(raw, shape + 0x2C, (sets + 1) as u32);
    put_i32(raw, shape + 0x30, new_list as i64 - (shape + 0x30) as i64);
    Ok(())
}

fn write_file_sizes(raw: &mut [u8], b: &Blob) -> Result<(), String> {
    let data = b.u16(6) as usize;
    let imag = data + b.u32(data + 4) as usize;
    if raw.get(imag..imag + 4) != Some(b"IMAG") {
        return Err("no IMAG block after DATA".into());
    }
    let size = raw.len();
    put_u32(raw, imag + 4, (size - imag) as u32);
    put_u32(raw, 12, size as u32);
    Ok(())
}

pub fn colour_texture(file: &[u8]) -> Option<Image> {
    let b = Blob(file);
    if file.get(0..4) != Some(b"CGFX") {
        return None;
    }
    let data = b.u16(6) as usize;
    let textures = b.dict(b.rel(data + 8 + 8 + 4)).ok()?;
    let (_, txob) = textures.iter().find(|(name, _)| name.ends_with("_c0")).or(textures.first())?;
    let (height, width, format) = (b.u32(txob + 0x18) as usize, b.u32(txob + 0x1C) as usize, b.u32(txob + 0x34));
    let image = b.rel(txob + 0x38)?;
    let bits = b.rel(image + 0x0C)?;
    formats::decode_etc1(file.get(bits..)?, width, height, format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_noise_is_the_python_builders() {
        let mut noise = Noise::new();
        let first: Vec<f64> = (0..3).map(|_| noise.next()).collect();
        assert!(first.iter().all(|v| v.abs() <= NOISE));
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        assert_eq!(first[0], ((s >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0) * NOISE);
    }

    fn rows(points: [(f64, f64); 3]) -> Vec<(usize, usize, usize)> {
        let mut mask = vec![false; 32 * 32];
        fill(&mut mask, 32, 32, points);
        (0..32)
            .filter_map(|y| {
                let row: Vec<usize> = (0..32).filter(|&x| mask[y * 32 + x]).collect();
                Some((y, *row.first()?, *row.last()?))
            })
            .collect()
    }

    #[test]
    fn a_triangle_fills_what_pillow_fills() {
        let filled = [(2, 2, 2), (3, 2, 4), (4, 3, 7), (5, 3, 9), (6, 3, 8), (7, 3, 6), (8, 4, 5), (9, 4, 4)];
        assert_eq!(rows([(2.0, 2.0), (9.0, 5.0), (4.0, 9.0)]), filled);
        assert_eq!(rows([(2.6, 2.6), (9.6, 5.3), (4.9, 9.1)]), filled);
        assert_eq!(rows([(0.0, 7.0), (20.0, 20.0), (27.0, 10.0)])[0], (7, 0, 2));
    }

    #[test]
    fn classes_have_their_styles() {
        assert_eq!(style_of("b/uBody_M_kgre_9.m.lz").name, "sabaton");
        assert_eq!(style_of("b/uBody_F_sorc_0.m.lz").name, "sandal");
        assert_eq!(style_of("b/uBody_M_mymM_0.m.lz").name, "shoe");
    }

    #[test]
    fn percentiles_go_between_values() {
        assert_eq!(percentile(&mut [4.0, 1.0, 3.0, 2.0], 50.0), 2.5);
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), 2.0);
    }
}
