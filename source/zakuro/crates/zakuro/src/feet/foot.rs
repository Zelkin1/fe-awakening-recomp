
//! Builds the feet for one body model.
//! This module measures the legs of the body.
//! It shapes a foot for each leg.
//! It takes the texture coordinates of the feet from the texture of the body.
//! Then it writes the bones, the vertices and the triangles of the feet into a copy of the CGFX file of the model.
//! In the coordinates of the model, y is up, z runs from the back to the front, and x is sideways.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::formats::{self, Blob, Image};
use super::model::{self, Attr, FLOAT};

/// A point or a vector with the three coordinates x, y and z.
type V3 = [f64; 3];

/// Adds two vectors.
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Subtracts the second vector from the first.
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Multiplies a vector by a number.
fn mul(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// Returns the dot product of two vectors.
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Returns the cross product of two vectors.
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Returns the length of a vector.
fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// Returns the lowest and the highest corner of the box around a set of points.
fn bounds(points: impl Iterator<Item = V3>) -> (V3, V3) {
    points.fold(([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]), |(lo, hi), p| {
        ([lo[0].min(p[0]), lo[1].min(p[1]), lo[2].min(p[2])], [hi[0].max(p[0]), hi[1].max(p[1]), hi[2].max(p[2])])
    })
}

/// Returns the median of the values.
/// It sorts the values in place.
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    if n % 2 == 1 { values[n / 2] } else { (values[n / 2 - 1] + values[n / 2]) / 2.0 }
}

/// Returns a percentile of the values, from 0 to 100, with a linear blend between the two nearest values.
/// It sorts the values in place.
fn percentile(values: &mut [f64], q: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    let at = (values.len() - 1) as f64 * q / 100.0;
    let (low, share) = (at.floor() as usize, at - at.floor());
    match values.get(low + 1) {
        Some(&next) => values[low] + (next - values[low]) * share,
        None => values[low],
    }
}

/// How far the heel sits behind the back of the shin, as a share of the length of the foot.
const HEEL: f64 = 0.03;
/// The length of the foot for each unit of depth of the shin.
const LENGTH_PER_DEPTH: f64 = 2.35;
/// The least and the most length of the foot, as shares of the height of the knee.
const LENGTH: (f64, f64) = (0.42, 0.60);
/// The least length of the part of the foot in front of the shin, as a share of the length of the foot.
const AHEAD_LEAST: f64 = 0.40;
/// The band of the shin that is cut to find its shaft, as shares of the height of the knee.
const BAND: (f64, f64) = (0.234, 0.338);
/// The share of the weight of a vertex that the bones of the shin must hold for the vertex to belong to the leg.
const OWN: f64 = 0.85;
/// The height that counts as the end of the leg, as a share of the height of the knee.
const END: f64 = 0.12;
/// The share of the height of the ankle above which vertices of the leg count for the front and the back of the shin.
const SHOWN: f64 = 0.85;
/// The widest that the foot can be, as a share of the height of the knee.
const WIDEST: f64 = 0.21;
/// The most that a texture coordinate is moved by the random jitter, in texels.
const NOISE: f64 = 1.5;
/// The share of the height of the ankle below which vertices of the leg are the end of the leg.
const LOW: f64 = 0.25;
/// The height that the end of the leg is tucked to, as a share of the height of the ankle.
const TUCK: f64 = 0.40;
/// The distance that the contact points of a foot sit below the bottom of the foot.
const LIFT: f64 = 0.01;
/// The flag bits that are set in the new bones of the feet.
const SKINNED: u32 = 0x240;
/// The position of the inverse bind matrix inside the record of a bone.
const INVERSE_BIND: usize = 0xA4;
/// The four bytes that are written into the inverse bind matrix of a new foot bone.
const TAG: &[u8; 4] = b"FEET";

/// The shape of a foot, as cross sections along the foot.
pub struct Style {
    /// The name of the style.
    pub name: &'static str,
    /// The sections from the heel to the front of the shin.
    /// Each section is the place along the part, the width, the top, the sole, the bottom and the shoulder.
    /// The place is a share of the length of the part.
    /// The width is a share of the half width of the shin.
    /// The three heights are shares of the height of the ankle.
    /// The shoulder is the share of the width that is left at the top.
    behind: &'static [[f64; 6]],
    /// The sections from the front of the shin to the toe, in the same form as the sections behind.
    ahead: &'static [[f64; 6]],
    /// The angle in degrees up to which neighboring faces are shaded as one smooth surface.
    smooth: f64,
}

/// The three styles of feet.
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

/// The class names of the bodies that get sabatons, which are the armored feet.
const SABATONS: [&str; 13] = ["karm", "gene", "kgre", "kgri", "kpal", "kdar", "kdr1", "kdr2", "lod2", "snyp", "war2", "ksoc", "merc"];
/// The class names of the bodies that get sandals.
const SANDALS: [&str; 5] = ["dmag", "sorc", "lagz", "baba", "swd2"];

/// Picks the style of the feet from the path of a body model.
/// The class of the body is the third part of the name, as in `b/uBody_M_kgre_9.m.lz`.
/// Every other class gets shoes.
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

/// The two materials of a foot, which take their colors from two different places of the texture.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
enum Material {
    /// The bottom of the foot.
    Sole,
    /// The rest of the foot.
    Upper,
}

/// The edges of a ring of the foot that belong to the sole, as indices of the points of the ring.
const SOLE_EDGES: [usize; 3] = [0, 1, 5];

/// A small random number generator with a fixed start, so that the same body always gets the same jitter.
struct Noise(u64);

impl Noise {
    /// Starts the generator at its fixed state.
    fn new() -> Noise {
        Noise(0x9E37_79B9_7F4A_7C15)
    }

    /// Returns the next number, from minus `NOISE` to `NOISE`.
    fn next(&mut self) -> f64 {
        // This is one step of the xorshift64 generator.
        let mut s = self.0;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.0 = s;
        // The top 53 bits make a number from 0 to 1, which is then stretched to the range of minus one to one.
        ((s >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0) * NOISE
    }
}

/// A rectangle of texels in the texture.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    /// The column of the left edge.
    x: usize,
    /// The row of the top edge.
    y: usize,
    /// The width in texels.
    w: usize,
    /// The height in texels.
    h: usize,
}

/// A candidate rectangle of the texture with its measured colors.
struct Found {
    /// The column of the left edge.
    x: usize,
    /// The row of the top edge.
    y: usize,
    /// The mean color of the rectangle.
    colour: [f64; 3],
    /// The standard deviation of the brightness inside the rectangle, which is low for a plain rectangle.
    spread: f64,
}

/// Returns the largest difference between two colors in any one channel.
fn farthest(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs()).max((a[2] - b[2]).abs())
}

/// Tells whether a color looks like skin, which is warm with red above green above blue.
fn skin(c: [f64; 3]) -> bool {
    let warm = (c[0] - c[2]) / (c[0] + 1.0);
    c[0] >= c[1] && c[1] >= c[2] && warm > 0.12 && warm < 0.55 && c[0] > 70.0
}

/// Returns the position of the candidate with the lowest key.
/// Each item is a key with a column and a row.
/// Ties are broken by the column and then by the row, so that the choice is always the same.
fn least(keys: impl Iterator<Item = (f64, usize, usize)>) -> Option<(usize, usize)> {
    keys.min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2))).map(|(_, x, y)| (x, y))
}

/// A summed area table, which gives the sum of the values in any rectangle of a grid with four reads.
struct Sums {
    /// The width of the grid.
    wide: usize,
    /// The running sums, with one extra row and column of zeros at the top and the left.
    table: Vec<f64>,
}

impl Sums {
    /// Builds the table for a grid of the given size, where `value` gives the value of each cell.
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

    /// Returns the sum of the values in a rectangle.
    fn sum(&self, x: usize, y: usize, w: usize, h: usize) -> f64 {
        let at = |x: usize, y: usize| self.table[y * (self.wide + 1) + x];
        at(x + w, y + h) - at(x, y + h) - at(x + w, y) + at(x, y)
    }
}

/// The texture of a body with a mask of the texels that the shins use, and tables for measuring rectangles of it.
struct Texels<'a> {
    /// The texture.
    tex: &'a Image,
    /// Tells for each texel whether the shins use it.
    mask: Vec<bool>,
    /// The number of masked texels.
    inside: Sums,
    /// The values of the red, green and blue channels.
    colour: [Sums; 3],
    /// The brightness.
    lum: Sums,
    /// The square of the brightness, which gives the spread together with the brightness.
    lum2: Sums,
}

/// Returns the brightness of a color with the usual luma weights.
fn lum(c: [f64; 3]) -> f64 {
    c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114
}

impl<'a> Texels<'a> {
    /// Builds the tables for a texture and the mask of its texels that the shins use.
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

    /// Finds a plain rectangle of the texture that lies fully inside the mask.
    /// It tries the sizes in order.
    /// For each size, it measures every position, and `pick` chooses one of them.
    /// The first size with a pick wins.
    fn plain(&self, sizes: &[(usize, usize)], pick: &dyn Fn(&[Found], (usize, usize)) -> Option<(usize, usize)>) -> Result<Rect, String> {
        let wide = self.tex.width;
        // These are the rows and the columns that the mask covers, as a first and a last.
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
                    // The rectangle and a border of one texel around it must be inside the mask.
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

    /// Returns the mean color of a rectangle.
    fn mean(&self, r: Rect) -> [f64; 3] {
        let n = (r.w * r.h) as f64;
        [0, 1, 2].map(|c| self.colour[c].sum(r.x, r.y, r.w, r.h) / n)
    }

    /// Measures how much a rectangle is striped.
    /// It returns the largest distance from the mean color of the rectangle to the mean color of any single column or row.
    fn stripe(&self, x: usize, y: usize, (w, h): (usize, usize)) -> f64 {
        let whole = self.mean(Rect { x, y, w, h });
        let columns = (x..x + w).map(|cx| [0, 1, 2].map(|c| (y..y + h).map(|ry| self.tex.at(cx, ry)[c]).sum::<f64>() / h as f64));
        let rows = (y..y + h).map(|ry| [0, 1, 2].map(|c| (x..x + w).map(|cx| self.tex.at(cx, ry)[c]).sum::<f64>() / w as f64));
        columns.chain(rows).map(|line| farthest(line, whole)).fold(0.0, f64::max)
    }
}

/// Makes a pick for `Texels::plain` that looks for a rectangle of the given color.
/// Among the rectangles within 16 of the color, it takes the plainest.
/// When there is none, it takes the one with the lowest mix of distance and spread.
fn near(colour: [f64; 3]) -> impl Fn(&[Found], (usize, usize)) -> Option<(usize, usize)> {
    move |found, _| {
        let close: Vec<_> = found.iter().filter(|f| farthest(f.colour, colour) < 16.0).map(|f| (f.spread, f.x, f.y)).collect();
        match close.is_empty() {
            false => least(close.into_iter()),
            true => least(found.iter().map(|f| (farthest(f.colour, colour) + 3.0 * f.spread, f.x, f.y))),
        }
    }
}

/// A pick for `Texels::plain` that takes the darkest rectangle among the plain ones.
fn darkest(found: &[Found], _: (usize, usize)) -> Option<(usize, usize)> {
    least(found.iter().filter(|f| f.spread < 3.0).map(|f| (f.colour.iter().sum::<f64>() / 3.0, f.x, f.y)))
}

/// The places of the texture that the materials of the feet take their colors from.
struct Patch {
    /// The rectangle of each material.
    rects: [(Material, Rect); 2],
    /// The width of the texture in texels.
    wide: usize,
    /// The height of the texture in texels.
    high: usize,
    /// The number of texels for each unit of length of the shins.
    density: f64,
}

/// The measurements of one leg.
struct Frame {
    /// Tells for each vertex of the body whether it belongs to the leg.
    on_leg: Vec<bool>,
    /// The points where one cut through the shin meets its triangles.
    outline: Vec<V3>,
    /// The x of the center of the shin.
    cx: f64,
    /// The z of the center of the shin.
    cz: f64,
    /// The half width of the shin.
    half: f64,
    /// The depth of the shin from the back to the front.
    depth: f64,
    /// The height of the ankle.
    ankle: f64,
    /// The height of the knee.
    height: f64,
    /// The z of the back of the shin.
    back: f64,
    /// The z of the front of the shin.
    front: f64,
}

/// The volume of a foot, which is used to pull the vertices of the leg inside it.
struct Fit {
    /// The x of the center of the foot.
    cx: f64,
    /// The z of the center of the shin, which is the origin of the places along the foot.
    cz: f64,
    /// The place of each section along the foot, from the back to the front.
    along: Vec<f64>,
    /// The width, the top, the sole, the bottom and the shoulder of each section.
    sections: Vec<[f64; 5]>,
}

/// A foot that is built for one leg.
struct Foot {
    /// The positions of the vertices.
    verts: Vec<V3>,
    /// The normals of the vertices.
    normals: Vec<V3>,
    /// The texture coordinates of the vertices.
    uvs: Vec<[f64; 2]>,
    /// The triangles as indices into the vertices.
    tris: Vec<[usize; 3]>,
    /// The three points that are written into the new foot bone, which are the ankle, the toe and the heel.
    contact: [V3; 3],
    /// The volume of the foot.
    fit: Fit,
}

/// The model of a body as it is read from its file.
struct Body<'a> {
    /// The bytes of the file.
    b: Blob<'a>,
    /// The positions of the vertices.
    pos: Vec<V3>,
    /// The texture coordinates of the vertices, when there are any.
    uv: Option<Vec<[f64; 2]>>,
    /// The weights of the bones of each vertex.
    weights: Vec<Vec<f64>>,
    /// The bones of each vertex as indices of the skeleton, or minus one for none.
    joint: Vec<Vec<i64>>,
    /// The bone with the highest weight of each vertex.
    main: Vec<i64>,
    /// The bones of the skeleton as an index and a name.
    names: Vec<(i64, String)>,
    /// The position of the record of each bone by its name.
    records: HashMap<String, usize>,
    /// All triangles of the body as indices into the vertices.
    triangles: Vec<[usize; 3]>,
}

/// A slanted edge of a triangle, for filling triangles into a mask.
struct Edge {
    /// The x of the start point.
    x0: i64,
    /// The y of the start point.
    y0: i64,
    /// The change of x for each step of y.
    dx: f32,
    /// The lowest y of the edge.
    top: i64,
    /// The highest y of the edge.
    bottom: i64,
}

impl Edge {
    /// Returns the x of the edge at the given y.
    fn x(&self, y: i64) -> f32 {
        (y - self.y0) as f32 * self.dx + self.x0 as f32
    }
}

/// Fills a triangle into a mask of texels, with the same rounding rules as the polygon fill of the Pillow library.
/// The points are in texels, and the parts of the triangle outside the mask are cut off.
fn fill(mask: &mut [bool], wide: usize, high: usize, points: [(f64, f64); 3]) {
    // The rounding of the left and the right end of a span differs, so that the edges of neighboring triangles do not overlap.
    let round_up = |v: f32| {
        let v = v as f64;
        (if v >= 0.0 { (v + 0.5).floor() } else { -(v.abs() + 0.5).floor() }) as i64
    };
    let round_down = |v: f32| {
        let v = v as f64;
        (if v >= 0.0 { (v - 0.5).ceil() } else { -(v.abs() - 0.5).ceil() }) as i64
    };
    let (wide_i, high_i) = (wide as i64, high as i64);
    // Sets the texels of one horizontal span, from x0 to x1 inclusive.
    let mut hline = |x0: i64, y: i64, x1: i64| {
        if y < 0 || y >= high_i || x1 < 0 || x0 >= wide_i {
            return;
        }
        for x in x0.max(0)..=x1.min(wide_i - 1) {
            mask[y as usize * wide + x as usize] = true;
        }
    };
    // The corners are cut to whole texels. Flat edges are drawn at once, and the slanted edges are kept for the scan below.
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
    // Each row is scanned from the top to the bottom. The crossings of the slanted edges with the row are collected, and every pair of them is a span.
    let (top, bottom) = (top.max(0), bottom.min(high_i));
    for y in top..=bottom {
        let mut xs: Vec<f32> = Vec::with_capacity(2 * slanted.len());
        for (i, edge) in slanted.iter().enumerate() {
            if y < edge.top || y > edge.bottom {
                continue;
            }
            let x = edge.x(y);
            xs.push(x);
            // The lower end of an edge is counted twice, except on the last row, so that a corner is handled like a crossing.
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
    /// Returns the indices of the bones with the wanted names.
    fn joints(&self, wanted: &[String]) -> Vec<i64> {
        self.names.iter().filter(|(_, name)| wanted.contains(name)).map(|(joint, _)| *joint).collect()
    }

    /// Returns the height of the knee of a leg, `l` or `r`.
    /// It is worked out from the inverse bind matrix of the shin bone, which is a matrix of three rows and four columns.
    fn knee(&self, leg: char) -> Result<f64, String> {
        let at = *self.records.get(&format!("{leg}_leg2")).ok_or("no shin")? + INVERSE_BIND;
        let m = |r: usize, c: usize| self.b.f32(at + 4 * (4 * r + c));
        Ok(-(0..3).map(|r| m(r, 1) * m(r, 3)).sum::<f64>())
    }

    /// Measures one leg of the body.
    /// It cuts the shin at several heights to find its center, its width and its depth.
    /// It places the ankle near the lowest cut.
    fn frame(&self, leg: char) -> Result<Frame, String> {
        let bones = self.joints(&[format!("{leg}_leg2"), format!("{leg}_leg3")]);
        let n = self.pos.len();
        // A vertex belongs to the leg when the bones of the shin are its main bone and hold most of its weight.
        let mut on_leg: Vec<bool> = (0..n)
            .map(|i| {
                let own: f64 = self.joint[i].iter().zip(&self.weights[i]).filter(|(j, _)| bones.contains(j)).map(|(_, w)| w).sum();
                bones.contains(&self.main[i]) && own >= OWN * self.weights[i].iter().sum::<f64>()
            })
            .collect();
        let height = self.knee(leg)?;
        let (lo, hi) = (BAND.0 * height, BAND.1 * height);
        // The vertices at the end of the leg give a box in x and z. Vertices outside that box, grown by half of its size, are dropped from the leg.
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
        // These are the points where the edges of the triangles of the leg cross a horizontal plane at height y.
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
        // Seven cuts are evenly spaced from the bottom to the top. Cuts with fewer than four crossings are dropped.
        let sections = |bottom: f64, top: f64| -> Vec<Vec<V3>> {
            (0..7).map(|k| if k == 6 { top } else { bottom + (top - bottom) / 6.0 * k as f64 }).map(crossings).filter(|cut| cut.len() >= 4).collect()
        };
        // When the band holds no shaft, as with a robe, the range below the band is cut instead.
        let (mut cuts, mut robed) = (sections(lo, hi), false);
        if cuts.len() < 2 {
            (cuts, robed) = (sections(END * height, lo), true);
        }
        if cuts.len() < 2 {
            return Err(format!("the {leg} shin has no shaft between {:.2} and {hi:.2}", END * height));
        }
        // The box of the shin is the median of the boxes of the cuts, which keeps single odd cuts from skewing it.
        let extents: Vec<(V3, V3)> = cuts.iter().map(|cut| bounds(cut.iter().copied())).collect();
        let first: V3 = [0, 1, 2].map(|c| median(&mut extents.iter().map(|e| e.0[c]).collect::<Vec<_>>()));
        let last: V3 = [0, 1, 2].map(|c| median(&mut extents.iter().map(|e| e.1[c]).collect::<Vec<_>>()));
        let (cx, cz) = ((first[0] + last[0]) / 2.0, (first[2] + last[2]) / 2.0);
        let (half, depth) = ((last[0] - first[0]) / 2.0, last[2] - first[2]);
        // The ankle sits at 90 percent of the height of the lowest cut, or of the highest cut for a robe.
        let heights = cuts.iter().map(|cut| cut[0][1]);
        let ankle = 0.9 * if robed { heights.fold(f64::NEG_INFINITY, f64::max) } else { heights.fold(f64::INFINITY, f64::min) };
        // When the cuts are narrower, the back and the front of the shin reach to the 5th and the 95th percentile of the z of the leg vertices above the ankle.
        let mut shown: Vec<f64> =
            (0..n).filter(|&i| on_leg[i] && self.pos[i][1] > SHOWN * ankle && self.pos[i][1] < hi).map(|i| self.pos[i][2]).collect();
        let (back, front) = match shown.len() >= 4 {
            true => (first[2].min(percentile(&mut shown, 5.0)), last[2].max(percentile(&mut shown, 95.0))),
            false => (first[2], last[2]),
        };
        // The foot is a little narrower than the shin, and never wider than `WIDEST`.
        let half = (half * 0.95).min(WIDEST * height / 2.0);
        Ok(Frame { on_leg, outline: cuts.swap_remove(0), cx, cz, half, depth, ankle, height, back, front })
    }

    /// Makes a mask of the texels that the chosen triangles cover in the texture.
    /// The v axis of the texture coordinates runs upward, so it is flipped to rows.
    fn texels<'t>(&self, chosen: impl Iterator<Item = &'t [usize; 3]>, wide: usize, high: usize) -> Vec<bool> {
        let uv = self.uv.as_ref().unwrap();
        let mut mask = vec![false; wide * high];
        for t in chosen {
            fill(&mut mask, wide, high, t.map(|i| (uv[i][0] * wide as f64, (1.0 - uv[i][1]) * high as f64)));
        }
        mask
    }

    /// Chooses the rectangles of the texture that give the feet their colors.
    /// One rectangle is for the upper of the foot, and one is for the sole.
    /// It looks for both among the texels of the shins.
    fn materials(&self, tex: &Image, frames: &[Frame; 2], style: &Style) -> Result<Patch, String> {
        let uv = self.uv.as_ref().ok_or("no texture coordinates")?;
        let (wide, high) = (tex.width, tex.height);
        let shins = self.joints(&["l_leg2", "l_leg3", "r_leg2", "r_leg3"].map(String::from));
        let tris: Vec<[usize; 3]> = self.triangles.iter().copied().filter(|t| t.iter().all(|&i| shins.contains(&self.main[i]))).collect();
        // The density is the number of texels for each unit of length, from the ratio of the area of the triangles in the texture to their area in space.
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
        // The texels of the lowest part of the shins show what the body wears on its feet. They are taken from the triangles around the ankle, or from any triangle below it.
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
        // The usual color is the median color of those texels.
        let usual = [0, 1, 2].map(|c| median(&mut low_texels.iter().map(|t| t[c]).collect::<Vec<_>>()));
        let texels = Texels::new(tex, mask);
        // The sizes of the rectangles in texels, from the largest to the smallest. The small ones are for the sole.
        let large = [(32, 14), (28, 12), (24, 12), (20, 10), (16, 8)];
        let small = [(24, 10), (16, 8), (12, 6)];
        let more = |sizes: &[(usize, usize)]| [sizes, &[(12, 6), (8, 4)]].concat();
        // A pick that looks for a flat rectangle of the color without stripes. It takes the last of the flattest candidates in the order of the sort.
        let grained = |colour: [f64; 3]| {
            let texels = &texels;
            move |found: &[Found], size: (usize, usize)| {
                let mut close: Vec<(f64, usize, usize)> = found.iter().filter(|f| farthest(f.colour, colour) < 16.0).map(|f| (f.spread, f.x, f.y)).collect();
                close.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
                let flattest = close.first().map_or(0.0, |c| c.0);
                close.iter().rev().find(|c| c.0 <= flattest + 2.5 && texels.stripe(c.1, c.2, size) < 4.0).map(|c| (c.1, c.2))
            }
        };
        // The upper of the foot first tries the grained pick with extra small sizes, and then falls back to the nearest color.
        let upper = |colour: [f64; 3], sizes: &[(usize, usize)]| texels.plain(&more(sizes), &grained(colour)).or_else(|_| texels.plain(sizes, &near(colour)));
        let rects = if style.name == "sandal" {
            // A sandal shows the bare foot when at least 30 percent of the low texels look like skin.
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

    /// Builds the foot for one leg.
    /// The foot is a row of rings that are placed along z.
    /// Quads join the rings.
    /// The texture coordinates come from the patch.
    /// When there is no texture, they come from the boot of the body.
    fn foot(&self, frame: &Frame, style: &Style, patch: Option<&Patch>) -> Result<Foot, String> {
        let Frame { cx, cz, half, depth, ankle, height, back, .. } = *frame;
        // The places of the heel, the front of the shin and the toe, relative to the center of the shin.
        let length = (LENGTH_PER_DEPTH * depth).clamp(LENGTH.0 * height, LENGTH.1 * height);
        let heel = back - cz - HEEL * length;
        let front = (frame.front - cz).max(heel + 0.2 * length);
        let toe = (heel + length).max(front + AHEAD_LEAST * length);
        let sections: Vec<[f64; 6]> = style.behind.iter().chain(style.ahead).copied().collect();
        let along: Vec<f64> =
            style.behind.iter().map(|s| heel + s[0] * length).chain(style.ahead.iter().map(|s| front + s[0] * (toe - front))).collect();
        // The six points of one ring in x and y. They run from the bottom left, over the right side and the top, to the left side.
        let outline = |half: f64, top: f64, sole: f64, bottom: f64, shoulder: f64| {
            [(cx - half, bottom), (cx + half, bottom), (cx + half, sole), (cx + shoulder * half, top), (cx - shoulder * half, top), (cx - half, sole)]
        };
        let points: Vec<V3> = along
            .iter()
            .zip(&sections)
            .flat_map(|(&z, s)| outline(half * s[1], s[2] * ankle, s[3] * ankle, s[4] * ankle, s[5]).map(|(x, y)| [x, y, cz + z]))
            .collect();
        // The contact points are the ankle, the bottom at the toe and the bottom at the heel.
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
                // The outward direction in x depends on which side of the body the foot is on.
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

    /// Takes texture coordinates for the vertices of a foot from the nearest vertex of the boot of the body.
    /// The boot is the vertices of the leg below a height of 1.4.
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

/// The number of points in one ring of a foot.
const RING_POINTS: usize = 6;

/// How the outward side of a face is found.
enum Outward {
    /// Outward is away from this point.
    FromPoint(V3),
    /// Outward is along the z axis, in the direction of the sign of the number.
    AlongZ(f64),
}

/// The size of a foot and the direction of its sides, for placing the texture on it.
struct FootBounds {
    /// The sign of each axis that points away from the body, which mirrors the foot of one side.
    outward: V3,
    /// The length of the foot along z.
    length: f64,
    /// The width of the foot along x.
    width: f64,
    /// The height of the foot along y.
    tall: f64,
}

/// Builds the faces of a foot from its rings.
/// Each face is a quad of four point indices with a material and a way to find its outward side.
/// Six quads join neighboring rings.
/// Two quads close each of the two end rings.
fn foot_faces(points: &[V3], rings: usize) -> Vec<([usize; 4], Material, Outward)> {
    let n = RING_POINTS;
    let mut faces: Vec<([usize; 4], Material, Outward)> = Vec::new();
    for r in 0..rings - 1 {
        // The middle of the two rings, which is inside the foot, tells which side of a quad is outward.
        let middle = mul(points[r * n..(r + 2) * n].iter().fold([0.0; 3], |sum, &p| add(sum, p)), 1.0 / (2 * n) as f64);
        for k in 0..n {
            let (a, b) = (r * n + k, r * n + (k + 1) % n);
            let material = if SOLE_EDGES.contains(&k) { Material::Sole } else { Material::Upper };
            faces.push(([a, b, b + n, a + n], material, Outward::FromPoint(middle)));
        }
    }
    // The first ring is closed facing backward, and the last ring is closed facing forward.
    for (base, out) in [(0, -1.0), ((rings - 1) * n, 1.0)] {
        for (quad, material) in [([0, 1, 2, 5], Material::Sole), ([5, 2, 3, 4], Material::Upper)] {
            faces.push((quad.map(|i| base + i), material, Outward::AlongZ(out)));
        }
    }
    faces
}

/// Splits the quads into triangles and turns each triangle so that it faces outward.
/// It returns the triangles, the material of each triangle and the unit normal of each triangle.
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
            // A triangle that faces inward gets two of its corners swapped, which flips its winding and its normal.
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

/// Makes the final vertices of a foot with smooth normals.
/// A corner of a triangle takes the sum of the normals of the triangles at the same point that bend away by no more than the smooth angle.
/// Corners at one point share a vertex only when their material and normal match, so sharp edges stay sharp.
/// It returns the positions, the normals, the materials and the triangles as indices of the new vertices.
fn smoothed_vertices(points: &[V3], tris: &[[usize; 3]], made_of: &[Material], normal: &[V3], smooth_degrees: f64) -> (Vec<V3>, Vec<V3>, Vec<Material>, Vec<[usize; 3]>) {
    let least_cos = smooth_degrees.to_radians().cos();
    let (mut verts, mut normals, mut materials, mut corners) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut made: HashMap<(usize, Material, [i64; 3]), usize> = HashMap::new();
    for (t, tri) in tris.iter().enumerate() {
        let mut new = [0; 3];
        for (c, &point) in tri.iter().enumerate() {
            let sum = (0..tris.len()).filter(|&g| tris[g].contains(&point) && dot(normal[g], normal[t]) >= least_cos).fold([0.0; 3], |sum, g| add(sum, normal[g]));
            let vertex_normal = mul(sum, 1.0 / norm(sum));
            // The normal is rounded to four decimals for the key, so that equal normals match despite tiny float differences.
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

/// Gives the vertices of a foot texture coordinates inside the rectangle of their material.
/// It projects each vertex flat along the axis that its normal points to most.
/// It jitters the result a little so that the plain rectangle does not look flat.
fn patch_uvs(patch: &Patch, verts: &[V3], normals: &[V3], materials: &[Material], bounds: &FootBounds, middle: V3) -> Vec<[f64; 2]> {
    let FootBounds { outward, length, width, tall } = *bounds;
    let mut noise = Noise::new();
    let jitter: Vec<[f64; 2]> = (0..verts.len()).map(|_| [noise.next(), noise.next()]).collect();
    let mut uvs = vec![[0.0; 2]; verts.len()];
    for &(material, Rect { x, y, w, h }) in &patch.rects {
        let (x0, y0, w, h) = (x as f64, y as f64, w as f64, h as f64);
        // The scale in texels per unit is the density of the body, but small enough that the foot fits into the rectangle.
        let scale = patch.density.min((w - 1.0) / length.max(width)).min((h - 1.0) / width.max(tall));
        for i in (0..verts.len()).filter(|&i| materials[i] == material) {
            // The facing is the axis of the largest part of the normal. It picks which two axes of the vertex become u and v.
            let a = normals[i].map(f64::abs);
            let facing = if a[0] >= a[1] && a[0] >= a[2] { 0 } else if a[1] >= a[2] { 1 } else { 2 };
            let (u_axis, v_axis) = [(2, 1), (2, 0), (0, 1)][facing];
            let tx = x0 + w / 2.0 + (verts[i][u_axis] - middle[u_axis]) * outward[u_axis] * scale;
            let ty = y0 + h / 2.0 + (verts[i][v_axis] - middle[v_axis]) * outward[v_axis] * scale;
            // The jittered point stays one texel inside the rectangle, so that it does not blend with the colors around the rectangle.
            let tx = (tx + jitter[i][0]).clamp(x0 + 1.0, x0 + w - 1.0);
            let ty = (ty + jitter[i][1]).clamp(y0 + 1.0, y0 + h - 1.0);
            uvs[i] = [tx / patch.wide as f64, 1.0 - ty / patch.high as f64];
        }
    }
    uvs
}

/// Moves a point into the volume of a foot, with a margin to the surface.
/// It returns nothing for a point that lies above the top of the foot there.
fn inside(q: V3, fit: &Fit, margin: f64) -> Option<V3> {
    let along = &fit.along;
    // The sections around the place of the point are blended into one section.
    let z = (q[2] - fit.cz).clamp(along[0] + margin, along[along.len() - 1] - margin);
    let k = along.iter().position(|&a| a >= z).unwrap_or(along.len()).clamp(1, along.len() - 1);
    let t = (z - along[k - 1]) / (along[k] - along[k - 1]);
    let [w, top, sole, _, shoulder] = [0, 1, 2, 3, 4].map(|c| (1.0 - t) * fit.sections[k - 1][c] + t * fit.sections[k][c]);
    if q[1] >= top - margin {
        return None;
    }
    let y = q[1].max(sole + margin);
    // The width shrinks from the sole to the top, down to the share of the shoulder, and the point is kept within 90 percent of it.
    let width = if y <= sole { w } else { w - (1.0 - shoulder) * w * (y - sole) / (top - sole) };
    Some([fit.cx + (q[0] - fit.cx).clamp(-0.9 * width, 0.9 * width), y, fit.cz + z])
}

/// Writes a little endian u32.
fn put_u32(raw: &mut [u8], at: usize, value: u32) {
    raw[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// Writes a little endian i32 from a wider number, which is used for relative pointers.
fn put_i32(raw: &mut [u8], at: usize, value: i64) {
    raw[at..at + 4].copy_from_slice(&(value as i32).to_le_bytes());
}

/// Writes a row of values as little endian f32.
fn put_f32s(raw: &mut [u8], at: usize, values: &[f64]) {
    for (k, v) in values.iter().enumerate() {
        raw[at + 4 * k..at + 4 * k + 4].copy_from_slice(&(*v as f32).to_le_bytes());
    }
}

/// Pads the file with zero bytes up to a multiple of `n`.
fn pad(raw: &mut Vec<u8>, n: usize) {
    raw.resize(raw.len().next_multiple_of(n), 0);
}

/// Builds a model with feet from a model of a body, which is the unpacked CGFX file.
/// The path of the model picks the style of the feet.
/// When the texture of the body is given, the feet take their colors from it.
/// It returns the bytes of the new file and the name of the style.
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

    // First the ends of the legs are tucked in, and then the leg vertices that reach into the range of a foot are pulled inside it.
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
    // The new file starts as a copy of the old one. Everything that is added goes to its end, and the pointers are patched to reach it.
    let mut raw = model.to_vec();
    let touching = write_moved_vertices(&mut raw, &v, position, normal, &body.triangles, &after, &moved);
    // The texture coordinates of the tucked ends are copied for the styles other than the sandal.
    if style.name != "sandal" {
        copy_end_uvs(&mut raw, &v, &frames, &ends, &touching, &after)?;
    }
    let foot_bones = add_foot_bones(&mut raw, &b, skeleton, &body, shins, &feet)?;
    let (uv_attr, bones_attr, weights_attr) = (attr(model::UV)?, attr(model::BONES)?, attr(model::WEIGHTS)?);
    if uv_attr.format != FLOAT {
        return Err("texture coordinates that aren't floats".into());
    }
    let (new_vertices, new_indices) = foot_geometry(&feet, &v, [position, normal, uv_attr], bones_attr, weights_attr);
    // The index streams of the game are 16 bit.
    if new_indices.iter().any(|&i| i > u16::MAX as usize) {
        return Err("too many vertices for 16-bit indices".into());
    }
    append_feet_mesh(&mut raw, &b, &v, shape, foot_bones, &new_vertices, &new_indices)?;
    // The file is padded to a multiple of 0x80 bytes.
    pad(&mut raw, 0x80);
    write_file_sizes(&mut raw, &b)?;
    Ok((raw, style.name))
}

/// Reads the skeleton, the vertices and the triangles of a body.
/// It fails for a body whose vertices have only one bone each, because the builder needs the weights of several bones.
fn read_body<'a>(b: &Blob<'a>, model: &'a [u8], skeleton: usize, shape: usize) -> Result<(Body<'a>, model::Vertices), String> {
    let mut names = Vec::new();
    let mut records = HashMap::new();
    // The dictionary of the bones is at 0x1C of the skeleton, and the index of a bone is at 8 of its record.
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
    // The bone indices of a vertex are slots in the palette of the part that uses the vertex. They are turned into indices of the skeleton.
    let mut joint = vec![vec![-1i64; local.first().map_or(0, Vec::len)]; n];
    for sm in &subs {
        let used: BTreeSet<usize> = sm.triangles.iter().flatten().copied().collect();
        for &i in &used {
            for (k, &slot) in local[i].iter().enumerate() {
                joint[i][k] = *sm.palette.get(slot as usize).ok_or("a bone outside its palette")? as i64;
            }
        }
    }
    // The main bone of a vertex is the bone with the highest weight.
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

/// Returns the indices of the left and the right shin bone, which the feet are attached to.
fn shin_joints(body: &Body) -> Result<[i64; 2], String> {
    let shin = |leg: char| body.names.iter().find(|(_, name)| *name == format!("{leg}_leg2")).map(|(joint, _)| *joint);
    let (Some(left), Some(right)) = (shin('l'), shin('r')) else {
        return Err("no l_leg2 and r_leg2, the shins the feet hang on".into());
    };
    Ok([left, right])
}

/// Moves the lowest vertices of each leg up into the end of the shin, so that the end of the leg is hidden inside the foot.
/// Each vertex goes to the point of the outline of the shin that lies at the nearest angle around the center, pulled in by 5 percent.
/// It returns the new position of every vertex that moved.
fn tucked_leg_ends(body: &Body, frames: &[Frame; 2], vertex_count: usize) -> BTreeMap<usize, V3> {
    let pos = &body.pos;
    let mut moved: BTreeMap<usize, V3> = BTreeMap::new();
    for frame in frames {
        let low: Vec<usize> = (0..vertex_count).filter(|&i| frame.on_leg[i] && pos[i][1] < LOW * frame.ankle).collect();
        if low.is_empty() {
            continue;
        }
        // The outline and the low vertices are both measured by their box. The angle around the center of a box is taken in units of its half sizes, so that a box that is not square still gives even angles.
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
            // This is the distance between two angles, which wraps around at a full turn.
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

/// Pulls the vertices of each leg into the volume of its foot, where they lie in the range of the foot.
/// A vertex is only recorded as moved when the new place differs from the old one by more than a tiny amount.
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

/// Writes the new positions of the moved vertices into the file, with new normals.
/// A new normal is the average of the normals of the triangles around the vertex in the new shape.
/// It returns the triangles that touch a moved vertex.
fn write_moved_vertices(raw: &mut [u8], v: &model::Vertices, position: Attr, normal: Attr, triangles: &[[usize; 3]], after: &[V3], moved: &BTreeMap<usize, V3>) -> Vec<[usize; 3]> {
    let touching: Vec<[usize; 3]> = triangles.iter().copied().filter(|t| t.iter().any(|i| moved.contains_key(i))).collect();
    let mut sums: HashMap<usize, V3> = HashMap::new();
    for t in &touching {
        let p = t.map(|i| after[i]);
        let face = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        // The tiny number keeps a triangle without area from dividing by zero.
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
        // The values are divided by the scale of the attribute, because the scale is multiplied in again when the values are read.
        put_f32s(raw, at + position.offset, &q.map(|c| c / position.scale));
        put_f32s(raw, at + normal.offset, &vertex_normal.map(|c| c / normal.scale));
    }
    touching
}

/// Gives each tucked vertex the texture coordinates of its nearest neighbor that was not tucked.
/// This keeps the texture of the leg from stretching over the moved vertices.
/// A neighbor that is too far away is not used.
fn copy_end_uvs(raw: &mut [u8], v: &model::Vertices, frames: &[Frame; 2], ends: &BTreeSet<usize>, touching: &[[usize; 3]], after: &[V3]) -> Result<(), String> {
    let Some(uv) = v.attrs.get(&model::UV) else {
        return Ok(());
    };
    let width = model::width(uv.format)? * uv.count;
    for frame in frames {
        let reach = 0.3 * frame.depth.max(2.0 * frame.half);
        for &i in ends.iter().filter(|&&i| frame.on_leg[i]) {
            // The neighbors are the other corners of the triangles at the vertex. The nearest one in x and z is found, and the first of equal ones is kept.
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

/// Adds a bone for each foot to the skeleton, named `l_shoe` and `r_shoe`.
/// Each record is a copy of the record of the shin, and the new bone is attached to that shin.
/// It appends the names, the records and a new dictionary to the file.
/// It patches the pointers of the skeleton.
/// It returns the indices of the two new bones.
fn add_foot_bones(raw: &mut Vec<u8>, b: &Blob, skeleton: usize, body: &Body, shins: [i64; 2], feet: &[Foot; 2]) -> Result<[u32; 2], String> {
    let old_dict = b.to(skeleton + 0x1C)?;
    let entries = b.dict(Some(old_dict))?;
    let count = b.u32(skeleton + 0x18) as usize;
    if count != entries.len() {
        return Err("a skeleton whose dictionary doesn't count its bones".into());
    }
    // The positions of the names and the data of the old bones are kept, because the new dictionary points to them again.
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
        // A bone record is 0xE0 bytes. The fields that are set here are the pointer to the name at 0, the flags at 4, the index at 8, the index of the parent at 0x0C and the pointer to the parent record at 0x10.
        let mut record = raw[shin_at..shin_at + 0xE0].to_vec();
        put_i32(&mut record, 0, name_at[&format!("{leg}_shoe")] as i64 - at as i64);
        put_u32(&mut record, 4, b.u32(shin_at + 4) | SKINNED);
        put_u32(&mut record, 8, (count + k) as u32);
        put_u32(&mut record, 0x0C, shins[k] as u32);
        put_i32(&mut record, 0x10, shin_at as i64 - (at + 0x10) as i64);
        record[0x14..0x20].fill(0);
        // The three contact points replace the first nine floats of the inverse bind matrix, and the tag is written after them.
        put_f32s(&mut record, INVERSE_BIND, &feet[k].contact.concat());
        record[INVERSE_BIND + 36..INVERSE_BIND + 40].copy_from_slice(TAG);
        record[INVERSE_BIND + 40..INVERSE_BIND + 48].fill(0);
        record[0xD8..0xE0].fill(0);
        raw.extend(record);
        data_at.insert(format!("{leg}_shoe"), at);
        foot_bones[k] = (count + k) as u32;
    }
    pad(raw, 4);
    // The dictionary is rebuilt with the two new names, and the skeleton is changed to count the new bones and to point to the new dictionary.
    let dict_at = raw.len();
    let all: Vec<String> = entries.iter().map(|(name, _)| name.clone()).chain(["l_shoe".to_owned(), "r_shoe".to_owned()]).collect();
    let nodes = formats::dict_nodes(&all)?;
    raw.extend(formats::dict_bytes(&nodes, dict_at, &name_at, &data_at));
    put_u32(raw, skeleton + 0x18, (count + 2) as u32);
    put_i32(raw, skeleton + 0x1C, dict_at as i64 - (skeleton + 0x1C) as i64);
    Ok(foot_bones)
}

/// Encodes the vertices and the indices of both feet in the vertex layout of the body.
/// The first foot is bound to the first bone of the palette of the new part and the second foot to the second bone.
/// It returns the bytes of the new vertices and the indices, which count from the end of the vertices of the body.
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
            // Every vertex of a foot has one bone with the weight 100, and the other slots stay zero.
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

/// Appends the feet to the shape as a new part.
/// The vertex buffer is copied to the end of the file with the new vertices behind the old ones, and the indices of the feet are written after it.
/// The new part is a copy of the first part with a palette of the two foot bones.
/// The list of parts of the shape is rewritten with the new part last.
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

    // The first part is the template for the new one. Its index stream must follow its primitive directly, so that both can be copied as one block.
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
    // The new part is a header of 0x14 bytes, the palette of two bones, one pointer to its primitive, and then a copy of the primitive and of the index stream.
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
    // The stream gets the size of the new indices in bytes, which is two bytes for each, and a pointer to them.
    put_u32(&mut set, stream + 8, (2 * new_indices.len()) as u32);
    put_i32(&mut set, stream + 12, index_at as i64 - (feet_set + stream + 12) as i64);
    raw.extend(set);
    // The new list holds the old pointers, which are made relative to the new place, and a pointer to the new part.
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

/// Updates the sizes in the headers of the file after data was appended.
/// The size of the IMAG block, which is the last block, and the size of the whole file are written.
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

/// Decodes the color texture of a body from its unpacked texture file.
/// The texture that is named with the ending `_c0` is taken, or the first texture when there is none.
/// It returns nothing when the file cannot be read or its format is not ETC1.
pub fn colour_texture(file: &[u8]) -> Option<Image> {
    let b = Blob(file);
    if file.get(0..4) != Some(b"CGFX") {
        return None;
    }
    let data = b.u16(6) as usize;
    let textures = b.dict(b.rel(data + 8 + 8 + 4)).ok()?;
    let (_, txob) = textures.iter().find(|(name, _)| name.ends_with("_c0")).or(textures.first())?;
    // The height, the width and the format are in the header of the texture, and the pixel data is behind two more pointers.
    let (height, width, format) = (b.u32(txob + 0x18) as usize, b.u32(txob + 0x1C) as usize, b.u32(txob + 0x34));
    let image = b.rel(txob + 0x38)?;
    let bits = b.rel(image + 0x0C)?;
    formats::decode_etc1(file.get(bits..)?, width, height, format)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The noise stays in its range and follows the xorshift steps from the fixed start.
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

    /// Fills a triangle into a mask of 32 by 32 texels and lists each row that it touches with its first and last texel.
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

    /// The fill gives the same rows as the polygon fill of the Pillow library, also for triangles that reach past the mask.
    #[test]
    fn a_triangle_fills_what_pillow_fills() {
        let filled = [(2, 2, 2), (3, 2, 4), (4, 3, 7), (5, 3, 9), (6, 3, 8), (7, 3, 6), (8, 4, 5), (9, 4, 4)];
        assert_eq!(rows([(2.0, 2.0), (9.0, 5.0), (4.0, 9.0)]), filled);
        assert_eq!(rows([(2.6, 2.6), (9.6, 5.3), (4.9, 9.1)]), filled);
        assert_eq!(rows([(0.0, 7.0), (20.0, 20.0), (27.0, 10.0)])[0], (7, 0, 2));
    }

    /// The class in the name of a body picks the style.
    #[test]
    fn classes_have_their_styles() {
        assert_eq!(style_of("b/uBody_M_kgre_9.m.lz").name, "sabaton");
        assert_eq!(style_of("b/uBody_F_sorc_0.m.lz").name, "sandal");
        assert_eq!(style_of("b/uBody_M_mymM_0.m.lz").name, "shoe");
    }

    /// The percentile blends between neighboring values, and the median of an odd count is the middle value.
    #[test]
    fn percentiles_go_between_values() {
        assert_eq!(percentile(&mut [4.0, 1.0, 3.0, 2.0], 50.0), 2.5);
        assert_eq!(median(&mut [3.0, 1.0, 2.0]), 2.0);
    }
}
