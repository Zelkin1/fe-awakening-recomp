//! Image operations for the HD text mod.
//!
//! This file holds single-channel planes, boolean masks, blurs, morphology, labelling, distance transforms and resampling filters.

/// A single channel of floating point values, stored row by row.
#[derive(Clone)]
pub struct Plane {
    /// The width in pixels.
    pub w: usize,
    /// The height in pixels.
    pub h: usize,
    /// The values, `w * h` of them, row by row.
    pub v: Vec<f32>,
}

impl Plane {
    /// Creates a plane of the given size filled with zeros.
    pub fn new(w: usize, h: usize) -> Plane {
        Plane { w, h, v: vec![0.0; w * h] }
    }

    /// Returns the value at the given position.
    pub fn at(&self, x: usize, y: usize) -> f32 {
        self.v[y * self.w + x]
    }

    /// Sets the value at the given position.
    pub fn set(&mut self, x: usize, y: usize, value: f32) {
        self.v[y * self.w + x] = value;
    }

    /// Returns a new plane with the function applied to every value.
    pub fn map(&self, f: impl Fn(f32) -> f32) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().map(|&x| f(x)).collect() }
    }

    /// Combines two planes of the same size value by value with the function.
    pub fn zip(&self, other: &Plane, f: impl Fn(f32, f32) -> f32) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().zip(&other.v).map(|(&a, &b)| f(a, b)).collect() }
    }

    /// Copies the rectangle from `x0, y0` up to but not including `x1, y1` into a new plane.
    pub fn crop(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> Plane {
        let mut out = Plane::new(x1 - x0, y1 - y0);
        for y in y0..y1 {
            out.v[(y - y0) * out.w..(y - y0 + 1) * out.w].copy_from_slice(&self.v[y * self.w + x0..y * self.w + x1]);
        }
        out
    }

    /// Copies another plane into this one at the given offset, which can be negative. The parts outside this plane are dropped.
    pub fn paste(&mut self, other: &Plane, x0: isize, y0: isize) {
        for y in 0..other.h {
            let ty = y as isize + y0;
            if ty < 0 || ty >= self.h as isize {
                continue;
            }
            for x in 0..other.w {
                let tx = x as isize + x0;
                if tx < 0 || tx >= self.w as isize {
                    continue;
                }
                self.v[ty as usize * self.w + tx as usize] = other.v[y * other.w + x];
            }
        }
    }

    /// Returns the largest value, or zero when no value is above zero.
    pub fn max(&self) -> f32 {
        self.v.iter().cloned().fold(0.0, f32::max)
    }

    /// Returns the sum of all values.
    pub fn sum(&self) -> f32 {
        self.v.iter().sum()
    }

    /// Shrinks the plane by an integer factor. Each block of `factor * factor` pixels becomes one pixel with their average value. A remainder at the right or bottom edge is dropped.
    pub fn down(&self, factor: usize) -> Plane {
        let (w, h) = (self.w / factor, self.h / factor);
        let mut out = Plane::new(w, h);
        let scale = 1.0 / (factor * factor) as f32;
        for y in 0..h {
            for dy in 0..factor {
                let row = &self.v[(y * factor + dy) * self.w..];
                for x in 0..w {
                    let mut s = 0.0;
                    for dx in 0..factor {
                        s += row[x * factor + dx];
                    }
                    out.v[y * w + x] += s * scale;
                }
            }
        }
        out
    }

    /// Returns the largest value in the row.
    pub fn row_max(&self, y: usize) -> f32 {
        self.v[y * self.w..(y + 1) * self.w].iter().cloned().fold(0.0, f32::max)
    }

    /// Returns the largest value in the column.
    pub fn col_max(&self, x: usize) -> f32 {
        (0..self.h).map(|y| self.v[y * self.w + x]).fold(0.0, f32::max)
    }

    /// Returns the first and last row that holds a value above the threshold, both inclusive.
    pub fn rows_above(&self, threshold: f32) -> Option<(usize, usize)> {
        let rows: Vec<usize> = (0..self.h).filter(|&y| self.row_max(y) > threshold).collect();
        Some((*rows.first()?, *rows.last()?))
    }

    /// Returns the first and last column that holds a value above the threshold, both inclusive.
    pub fn cols_above(&self, threshold: f32) -> Option<(usize, usize)> {
        let cols: Vec<usize> = (0..self.w).filter(|&x| self.col_max(x) > threshold).collect();
        Some((*cols.first()?, *cols.last()?))
    }

    /// Returns the bounding box of the values above the threshold as `x0, x1, y0, y1`, with the upper ends exclusive.
    pub fn bbox(&self, threshold: f32) -> Option<(usize, usize, usize, usize)> {
        let (y0, y1) = self.rows_above(threshold)?;
        let (x0, x1) = self.cols_above(threshold)?;
        Some((x0, x1 + 1, y0, y1 + 1))
    }
}

/// Maps an index outside `0..n` back inside by mirroring at the edges, so that the edge pixel is repeated once.
fn reflect(i: isize, n: usize) -> usize {
    let n = n as isize;
    let mut i = i;
    loop {
        if i < 0 {
            i = -i - 1;
        } else if i >= n {
            i = 2 * n - i - 1;
        } else {
            return i as usize;
        }
    }
}

/// Convolves every row with a one-dimensional kernel. The edges are mirrored.
fn convolve_rows(p: &Plane, kernel: &[f32]) -> Plane {
    let r = (kernel.len() / 2) as isize;
    let mut out = Plane::new(p.w, p.h);
    for y in 0..p.h {
        let row = &p.v[y * p.w..(y + 1) * p.w];
        for x in 0..p.w {
            let mut s = 0.0;
            for (k, &kv) in kernel.iter().enumerate() {
                s += kv * row[reflect(x as isize + k as isize - r, p.w)];
            }
            out.v[y * p.w + x] = s;
        }
    }
    out
}

/// Convolves every column with a one-dimensional kernel. The edges are mirrored.
fn convolve_cols(p: &Plane, kernel: &[f32]) -> Plane {
    let r = (kernel.len() / 2) as isize;
    let mut out = Plane::new(p.w, p.h);
    for y in 0..p.h {
        for (k, &kv) in kernel.iter().enumerate() {
            let sy = reflect(y as isize + k as isize - r, p.h);
            let src = &p.v[sy * p.w..(sy + 1) * p.w];
            let dst = &mut out.v[y * p.w..(y + 1) * p.w];
            for x in 0..p.w {
                dst[x] += kv * src[x];
            }
        }
    }
    out
}

/// Builds a normalised Gaussian kernel.
pub fn gaussian_kernel(sigma: f32) -> Vec<f32> {
    // The kernel reaches four sigmas to each side, rounded to the nearest pixel.
    let r = (4.0 * sigma + 0.5) as isize;
    let mut k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let s: f32 = k.iter().sum();
    k.iter_mut().for_each(|v| *v /= s);
    k
}

/// Blurs the plane with a separable Gaussian. A sigma of zero or less returns a copy.
pub fn gaussian(p: &Plane, sigma: f32) -> Plane {
    if sigma <= 0.0 {
        return p.clone();
    }
    let k = gaussian_kernel(sigma);
    convolve_cols(&convolve_rows(p, &k), &k)
}

/// Blurs the plane with a box filter of the given size.
pub fn uniform(p: &Plane, size: usize) -> Plane {
    let k = vec![1.0 / size as f32; size];
    convolve_cols(&convolve_rows(p, &k), &k)
}

/// A boolean image, stored row by row.
#[derive(Clone)]
pub struct Mask {
    /// The width in pixels.
    pub w: usize,
    /// The height in pixels.
    pub h: usize,
    /// The flags, `w * h` of them, row by row.
    pub v: Vec<bool>,
}

impl Mask {
    /// Creates a mask of the given size with every flag cleared.
    pub fn new(w: usize, h: usize) -> Mask {
        Mask { w, h, v: vec![false; w * h] }
    }

    /// Builds a mask from a plane by testing every value with the function.
    pub fn from(p: &Plane, f: impl Fn(f32) -> bool) -> Mask {
        Mask { w: p.w, h: p.h, v: p.v.iter().map(|&x| f(x)).collect() }
    }

    /// Returns the flag at the given position.
    pub fn at(&self, x: usize, y: usize) -> bool {
        self.v[y * self.w + x]
    }

    /// Sets the flag at the given position.
    pub fn set(&mut self, x: usize, y: usize, value: bool) {
        self.v[y * self.w + x] = value;
    }

    /// Returns the pixels set in both masks.
    pub fn and(&self, o: &Mask) -> Mask {
        Mask { w: self.w, h: self.h, v: self.v.iter().zip(&o.v).map(|(&a, &b)| a && b).collect() }
    }

    /// Returns the pixels set in either mask.
    pub fn or(&self, o: &Mask) -> Mask {
        Mask { w: self.w, h: self.h, v: self.v.iter().zip(&o.v).map(|(&a, &b)| a || b).collect() }
    }

    /// Returns the inverted mask.
    pub fn not(&self) -> Mask {
        Mask { w: self.w, h: self.h, v: self.v.iter().map(|&a| !a).collect() }
    }

    /// Returns the number of set pixels.
    pub fn count(&self) -> usize {
        self.v.iter().filter(|&&a| a).count()
    }

    /// Returns whether any pixel is set.
    pub fn any(&self) -> bool {
        self.v.iter().any(|&a| a)
    }

    /// Returns the bounding box of the set pixels as `x0, y0, x1, y1`, with the upper ends exclusive. Returns `None` for an empty mask.
    pub fn bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for y in 0..self.h {
            for x in 0..self.w {
                if self.at(x, y) {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x + 1);
                    y1 = y1.max(y + 1);
                }
            }
        }
        (x0 != usize::MAX).then_some((x0, y0, x1, y1))
    }

    /// Enlarges the mask by an integer factor. Every pixel is repeated.
    pub fn scale_up(&self, factor: usize) -> Mask {
        let mut out = Mask::new(self.w * factor, self.h * factor);
        for y in 0..out.h {
            for x in 0..out.w {
                out.v[y * out.w + x] = self.v[(y / factor) * self.w + x / factor];
            }
        }
        out
    }

    /// Converts the mask to a plane of zeros and ones.
    pub fn as_plane(&self) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().map(|&a| if a { 1.0 } else { 0.0 }).collect() }
    }
}

/// Grows the mask by one pixel per iteration. It uses the four direct neighbours.
pub fn dilate(m: &Mask, iterations: usize) -> Mask {
    let mut cur = m.clone();
    for _ in 0..iterations {
        let mut out = cur.clone();
        for y in 0..m.h {
            for x in 0..m.w {
                if cur.at(x, y) {
                    continue;
                }
                let hit = (x > 0 && cur.at(x - 1, y)) || (x + 1 < m.w && cur.at(x + 1, y)) || (y > 0 && cur.at(x, y - 1)) || (y + 1 < m.h && cur.at(x, y + 1));
                if hit {
                    out.set(x, y, true);
                }
            }
        }
        cur = out;
    }
    cur
}

/// Shrinks the mask by one pixel per iteration. A pixel stays only when all four direct neighbours are set, so pixels at the border of the image are removed.
pub fn erode(m: &Mask, iterations: usize) -> Mask {
    let mut cur = m.clone();
    for _ in 0..iterations {
        let mut out = cur.clone();
        for y in 0..m.h {
            for x in 0..m.w {
                if !cur.at(x, y) {
                    continue;
                }
                let keep = x > 0 && cur.at(x - 1, y) && x + 1 < m.w && cur.at(x + 1, y) && y > 0 && cur.at(x, y - 1) && y + 1 < m.h && cur.at(x, y + 1);
                if !keep {
                    out.set(x, y, false);
                }
            }
        }
        cur = out;
    }
    cur
}

/// Sets every pixel that cannot be reached from the border without crossing the mask.
pub fn fill_holes(m: &Mask) -> Mask {
    let mut outside = Mask::new(m.w, m.h);
    let mut stack = Vec::new();
    // The flood fill starts from every pixel on the border.
    for x in 0..m.w {
        stack.push((x, 0));
        stack.push((x, m.h - 1));
    }
    for y in 0..m.h {
        stack.push((0, y));
        stack.push((m.w - 1, y));
    }
    while let Some((x, y)) = stack.pop() {
        if m.at(x, y) || outside.at(x, y) {
            continue;
        }
        outside.set(x, y, true);
        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < m.w {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < m.h {
            stack.push((x, y + 1));
        }
    }
    outside.not()
}

/// Labels the connected components of the mask. Returns one label per pixel, where zero is the background, and the number of components. With `eight` set, diagonal neighbours connect as well.
pub fn label(m: &Mask, eight: bool) -> (Vec<u32>, usize) {
    let mut labels = vec![0u32; m.w * m.h];
    let mut count = 0usize;
    let mut stack = Vec::new();
    for start in 0..m.v.len() {
        if !m.v[start] || labels[start] != 0 {
            continue;
        }
        count += 1;
        labels[start] = count as u32;
        stack.push(start);
        while let Some(i) = stack.pop() {
            let (x, y) = ((i % m.w) as isize, (i / m.w) as isize);
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)].iter().take(if eight { 8 } else { 4 }) {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= m.w as isize || ny >= m.h as isize {
                    continue;
                }
                let j = ny as usize * m.w + nx as usize;
                if m.v[j] && labels[j] == 0 {
                    labels[j] = count as u32;
                    stack.push(j);
                }
            }
        }
    }
    (labels, count)
}

/// Returns the largest four-connected component of the mask, or `None` when the mask is empty.
pub fn largest(m: &Mask) -> Option<Mask> {
    let (labels, count) = label(m, false);
    if count == 0 {
        return None;
    }
    let mut sizes = vec![0usize; count + 1];
    for &l in &labels {
        sizes[l as usize] += 1;
    }
    let best = (1..=count).fold(1, |best, i| if sizes[i] > sizes[best] { i } else { best }) as u32;
    Some(Mask { w: m.w, h: m.h, v: labels.iter().map(|&l| l == best).collect() })
}

/// Computes the one-dimensional squared distance transform of the function `f` as the lower envelope of parabolas. Writes the squared distance to `d` and the index of the nearest source to `idx`.
fn edt_1d(f: &[f32], d: &mut [f32], idx: &mut [usize]) {
    let n = f.len();
    let mut v = vec![0usize; n];
    let mut z = vec![0f32; n + 1];
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    for q in 1..n {
        loop {
            let p = v[k];
            let s = ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32)) / (2.0 * q as f32 - 2.0 * p as f32);
            if s <= z[k] && k > 0 {
                k -= 1;
                continue;
            }
            if s <= z[k] {
                v[k] = q;
                z[k + 1] = f32::INFINITY;
                break;
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f32::INFINITY;
            break;
        }
    }
    k = 0;
    for q in 0..n {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let p = v[k];
        d[q] = (q as f32 - p as f32).powi(2) + f[p];
        idx[q] = p;
    }
}

/// Computes the Euclidean distance from every pixel to the nearest pixel that is not set in the mask. Also returns the position of that nearest pixel for every pixel.
pub fn distance(m: &Mask) -> (Plane, Vec<(usize, usize)>) {
    // A stand-in for infinity that still works in the squared distance arithmetic.
    let big = 1e20f32;
    let (w, h) = (m.w, m.h);
    let mut col_d = vec![0f32; w * h];
    let mut col_i = vec![0usize; w * h];
    let mut f = vec![0f32; h];
    let mut d = vec![0f32; h];
    let mut ix = vec![0usize; h];
    for x in 0..w {
        for (y, v) in f.iter_mut().enumerate() {
            *v = if m.at(x, y) { big } else { 0.0 };
        }
        if f.iter().all(|&v| v >= big) {
            for y in 0..h {
                col_d[y * w + x] = big;
                col_i[y * w + x] = y;
            }
            continue;
        }
        edt_1d(&f, &mut d, &mut ix);
        for y in 0..h {
            col_d[y * w + x] = d[y];
            col_i[y * w + x] = ix[y];
        }
    }
    // The second pass runs along the rows over the results of the column pass, which gives the full two-dimensional distance.
    let mut out = Plane::new(w, h);
    let mut nearest = vec![(0usize, 0usize); w * h];
    let mut f = vec![0f32; w];
    let mut d = vec![0f32; w];
    let mut ix = vec![0usize; w];
    for y in 0..h {
        for x in 0..w {
            f[x] = col_d[y * w + x];
        }
        edt_1d(&f, &mut d, &mut ix);
        for x in 0..w {
            out.v[y * w + x] = d[x].min(big).sqrt();
            nearest[y * w + x] = (ix[x], col_i[y * w + ix[x]]);
        }
    }
    (out, nearest)
}

/// Applies a square median filter of the given odd size. Pixels outside the plane repeat the nearest edge pixel.
pub fn median(p: &Plane, size: usize) -> Plane {
    let r = (size / 2) as isize;
    let mut out = Plane::new(p.w, p.h);
    let mut window = Vec::with_capacity(size * size);
    for y in 0..p.h {
        for x in 0..p.w {
            window.clear();
            for dy in -r..=r {
                for dx in -r..=r {
                    let xx = (x as isize + dx).clamp(0, p.w as isize - 1) as usize;
                    let yy = (y as isize + dy).clamp(0, p.h as isize - 1) as usize;
                    window.push(p.v[yy * p.w + xx]);
                }
            }
            let mid = window.len() / 2;
            window.select_nth_unstable_by(mid, |a, b| a.partial_cmp(b).unwrap());
            out.v[y * p.w + x] = window[mid];
        }
    }
    out
}

/// Returns the gradient magnitude of the plane using the Sobel operator.
pub fn sobel_magnitude(p: &Plane) -> Plane {
    let mut out = Plane::new(p.w, p.h);
    let g = |x: isize, y: isize| p.v[reflect(y, p.h) * p.w + reflect(x, p.w)];
    for y in 0..p.h as isize {
        for x in 0..p.w as isize {
            let gx = (g(x + 1, y - 1) + 2.0 * g(x + 1, y) + g(x + 1, y + 1)) - (g(x - 1, y - 1) + 2.0 * g(x - 1, y) + g(x - 1, y + 1));
            let gy = (g(x - 1, y + 1) + 2.0 * g(x, y + 1) + g(x + 1, y + 1)) - (g(x - 1, y - 1) + 2.0 * g(x, y - 1) + g(x + 1, y - 1));
            out.v[y as usize * p.w + x as usize] = (gx * gx + gy * gy).sqrt();
        }
    }
    out
}

/// The resampling filters that `resize` supports.
#[derive(Clone, Copy)]
pub enum Filter {
    Nearest,
    Bilinear,
    Bicubic,
    /// A Lanczos filter with three lobes.
    Lanczos,
}

/// Returns the weight of the filter at the given distance from the centre.
fn filter_weight(filter: Filter, x: f32) -> f32 {
    let x = x.abs();
    match filter {
        Filter::Nearest => {
            if x < 0.5 { 1.0 } else { 0.0 }
        }
        Filter::Bilinear => (1.0 - x).max(0.0),
        Filter::Bicubic => {
            // The Catmull-Rom spline.
            let a = -0.5;
            if x < 1.0 {
                ((a + 2.0) * x - (a + 3.0)) * x * x + 1.0
            } else if x < 2.0 {
                (((x - 5.0) * x + 8.0) * x - 4.0) * a
            } else {
                0.0
            }
        }
        Filter::Lanczos => {
            if x < 3.0 {
                let s = |t: f32| if t == 0.0 { 1.0 } else { (std::f32::consts::PI * t).sin() / (std::f32::consts::PI * t) };
                s(x) * s(x / 3.0)
            } else {
                0.0
            }
        }
    }
}

/// Returns the radius in source pixels beyond which the filter has no weight.
fn filter_support(filter: Filter) -> f32 {
    match filter {
        Filter::Nearest => 0.5,
        Filter::Bilinear => 1.0,
        Filter::Bicubic => 2.0,
        Filter::Lanczos => 3.0,
    }
}

/// Computes, for every output pixel along one axis, the first source pixel and the normalised weights of the source pixels that follow.
fn resample_axis(src_len: usize, dst_len: usize, filter: Filter) -> Vec<(usize, Vec<f32>)> {
    let scale = src_len as f32 / dst_len as f32;
    // When the size goes down, the filter is stretched so that it covers all source pixels and avoids aliasing.
    let filterscale = scale.max(1.0);
    let support = filter_support(filter) * filterscale;
    (0..dst_len)
        .map(|i| {
            let center = (i as f32 + 0.5) * scale;
            let start = ((center - support + 0.5).floor().max(0.0)) as usize;
            let end = (((center + support + 0.5).floor()) as usize).min(src_len);
            let mut weights: Vec<f32> = (start..end).map(|j| filter_weight(filter, (j as f32 + 0.5 - center) / filterscale)).collect();
            let total: f32 = weights.iter().sum();
            if total != 0.0 {
                weights.iter_mut().for_each(|w| *w /= total);
            }
            (start, weights)
        })
        .collect()
}

/// Resizes the plane to the given size with the filter. The horizontal pass runs first and the vertical pass second.
pub fn resize(p: &Plane, w: usize, h: usize, filter: Filter) -> Plane {
    if let Filter::Nearest = filter {
        let mut out = Plane::new(w, h);
        for y in 0..h {
            let sy = ((y as f32 + 0.5) * p.h as f32 / h as f32) as usize;
            for x in 0..w {
                let sx = ((x as f32 + 0.5) * p.w as f32 / w as f32) as usize;
                out.v[y * w + x] = p.v[sy.min(p.h - 1) * p.w + sx.min(p.w - 1)];
            }
        }
        return out;
    }
    let horizontal = resample_axis(p.w, w, filter);
    let mut mid = Plane::new(w, p.h);
    for y in 0..p.h {
        let row = &p.v[y * p.w..(y + 1) * p.w];
        for (x, (start, weights)) in horizontal.iter().enumerate() {
            mid.v[y * w + x] = weights.iter().enumerate().map(|(k, wt)| wt * row[start + k]).sum();
        }
    }
    let vertical = resample_axis(p.h, h, filter);
    let mut out = Plane::new(w, h);
    for (y, (start, weights)) in vertical.iter().enumerate() {
        let dst = &mut out.v[y * w..(y + 1) * w];
        for (k, wt) in weights.iter().enumerate() {
            let src = &mid.v[(start + k) * w..(start + k + 1) * w];
            for x in 0..w {
                dst[x] += wt * src[x];
            }
        }
    }
    out
}

/// A small deterministic random number generator of the SplitMix64 kind. The same seed always gives the same pictures.
pub struct Rng(u64);

impl Rng {
    /// Creates a generator from a seed. The seed is scrambled so that nearby seeds give unrelated sequences.
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }

    /// Returns the next 64 bit value.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns a value between zero and one, never exactly zero or one.
    pub fn uniform(&mut self) -> f32 {
        // The top 24 bits fit exactly in the mantissa of an f32.
        ((self.next() >> 40) as f32 + 0.5) / (1u64 << 24) as f32
    }

    /// Returns a normally distributed value with mean zero and deviation one. It uses the Box-Muller transform.
    pub fn normal(&mut self) -> f32 {
        let (a, b) = (self.uniform(), self.uniform());
        (-2.0 * a.ln()).sqrt() * (2.0 * std::f32::consts::PI * b).cos()
    }
}

/// Returns the brightness of a colour using the Rec. 601 weights.
pub fn luminance(rgb: [f32; 3]) -> f32 {
    rgb[0] * 0.299 + rgb[1] * 0.587 + rgb[2] * 0.114
}

/// A colour image with four floating point channels per pixel, from zero to one.
#[derive(Clone)]
pub struct Rgba {
    /// The width in pixels.
    pub w: usize,
    /// The height in pixels.
    pub h: usize,
    /// The pixels as red, green, blue and alpha, row by row.
    pub v: Vec<[f32; 4]>,
}

impl Rgba {
    /// Creates a fully transparent black image of the given size.
    pub fn new(w: usize, h: usize) -> Rgba {
        Rgba { w, h, v: vec![[0.0; 4]; w * h] }
    }

    /// Converts 8 bit texels to floating point values between zero and one.
    pub fn from_texels(texels: &[[u8; 4]], w: usize, h: usize) -> Rgba {
        Rgba { w, h, v: texels.iter().map(|t| [t[0] as f32 / 255.0, t[1] as f32 / 255.0, t[2] as f32 / 255.0, t[3] as f32 / 255.0]).collect() }
    }

    /// Returns the pixel at the given position.
    pub fn at(&self, x: usize, y: usize) -> [f32; 4] {
        self.v[y * self.w + x]
    }

    /// Extracts one channel as a plane, where 0 to 3 are red, green, blue and alpha.
    pub fn channel(&self, c: usize) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().map(|p| p[c]).collect() }
    }

    /// Returns the brightness of every pixel as a plane. The alpha channel is ignored.
    pub fn lum(&self) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().map(|p| luminance([p[0], p[1], p[2]])).collect() }
    }

    /// Copies the rectangle from `x0, y0` up to but not including `x1, y1` into a new image.
    pub fn crop(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> Rgba {
        let mut out = Rgba::new(x1 - x0, y1 - y0);
        for y in y0..y1 {
            out.v[(y - y0) * out.w..(y - y0 + 1) * out.w].copy_from_slice(&self.v[y * self.w + x0..y * self.w + x1]);
        }
        out
    }

    /// Copies another image into this one at the given offset. The other image must fit inside this one.
    pub fn paste(&mut self, other: &Rgba, x0: usize, y0: usize) {
        for y in 0..other.h {
            self.v[(y + y0) * self.w + x0..(y + y0) * self.w + x0 + other.w].copy_from_slice(&other.v[y * other.w..(y + 1) * other.w]);
        }
    }

    /// Converts the image to 8 bit RGBA bytes. Every channel is clamped to the range from zero to one.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.v.iter().flat_map(|p| p.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)).collect()
    }
}

/// Returns the median of the values. It sorts the values in place. An empty slice gives zero.
pub fn median_of(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = values.len();
    if n % 2 == 1 { values[n / 2] } else { (values[n / 2 - 1] + values[n / 2]) / 2.0 }
}

/// Returns the median of every channel over the pixels selected by the mask, or `None` when the mask selects nothing.
pub fn median_colour(image: &Rgba, mask: &Mask) -> Option<[f32; 4]> {
    let picked: Vec<[f32; 4]> = image.v.iter().zip(&mask.v).filter(|(_, &m)| m).map(|(p, _)| *p).collect();
    if picked.is_empty() {
        return None;
    }
    let mut out = [0.0; 4];
    for c in 0..4 {
        let mut column: Vec<f32> = picked.iter().map(|p| p[c]).collect();
        out[c] = median_of(&mut column);
    }
    Some(out)
}

/// Returns the percentile `q`, from 0 to 100, of the values. It interpolates linearly between neighbours. An empty slice gives zero.
pub fn percentile(values: &[f32], q: f32) -> f32 {
    let mut v: Vec<f32> = values.to_vec();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pos = (q / 100.0) * (v.len() - 1) as f32;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    v[lo] + (v[hi] - v[lo]) * (pos - lo as f32)
}
