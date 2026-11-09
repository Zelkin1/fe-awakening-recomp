#[derive(Clone)]
pub struct Plane {
    pub w: usize,
    pub h: usize,
    pub v: Vec<f32>,
}

impl Plane {
    pub fn new(w: usize, h: usize) -> Plane {
        Plane { w, h, v: vec![0.0; w * h] }
    }

    pub fn at(&self, x: usize, y: usize) -> f32 {
        self.v[y * self.w + x]
    }

    pub fn set(&mut self, x: usize, y: usize, value: f32) {
        self.v[y * self.w + x] = value;
    }

    pub fn map(&self, f: impl Fn(f32) -> f32) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().map(|&x| f(x)).collect() }
    }

    pub fn zip(&self, other: &Plane, f: impl Fn(f32, f32) -> f32) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().zip(&other.v).map(|(&a, &b)| f(a, b)).collect() }
    }

    pub fn crop(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> Plane {
        let mut out = Plane::new(x1 - x0, y1 - y0);
        for y in y0..y1 {
            out.v[(y - y0) * out.w..(y - y0 + 1) * out.w].copy_from_slice(&self.v[y * self.w + x0..y * self.w + x1]);
        }
        out
    }

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

    pub fn max(&self) -> f32 {
        self.v.iter().cloned().fold(0.0, f32::max)
    }

    pub fn sum(&self) -> f32 {
        self.v.iter().sum()
    }

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

    pub fn row_max(&self, y: usize) -> f32 {
        self.v[y * self.w..(y + 1) * self.w].iter().cloned().fold(0.0, f32::max)
    }

    pub fn col_max(&self, x: usize) -> f32 {
        (0..self.h).map(|y| self.v[y * self.w + x]).fold(0.0, f32::max)
    }

    pub fn rows_above(&self, threshold: f32) -> Option<(usize, usize)> {
        let rows: Vec<usize> = (0..self.h).filter(|&y| self.row_max(y) > threshold).collect();
        Some((*rows.first()?, *rows.last()?))
    }

    pub fn cols_above(&self, threshold: f32) -> Option<(usize, usize)> {
        let cols: Vec<usize> = (0..self.w).filter(|&x| self.col_max(x) > threshold).collect();
        Some((*cols.first()?, *cols.last()?))
    }

    pub fn bbox(&self, threshold: f32) -> Option<(usize, usize, usize, usize)> {
        let (y0, y1) = self.rows_above(threshold)?;
        let (x0, x1) = self.cols_above(threshold)?;
        Some((x0, x1 + 1, y0, y1 + 1))
    }
}

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

pub fn gaussian_kernel(sigma: f32) -> Vec<f32> {
    let r = (4.0 * sigma + 0.5) as isize;
    let mut k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let s: f32 = k.iter().sum();
    k.iter_mut().for_each(|v| *v /= s);
    k
}

pub fn gaussian(p: &Plane, sigma: f32) -> Plane {
    if sigma <= 0.0 {
        return p.clone();
    }
    let k = gaussian_kernel(sigma);
    convolve_cols(&convolve_rows(p, &k), &k)
}

pub fn uniform(p: &Plane, size: usize) -> Plane {
    let k = vec![1.0 / size as f32; size];
    convolve_cols(&convolve_rows(p, &k), &k)
}

#[derive(Clone)]
pub struct Mask {
    pub w: usize,
    pub h: usize,
    pub v: Vec<bool>,
}

impl Mask {
    pub fn new(w: usize, h: usize) -> Mask {
        Mask { w, h, v: vec![false; w * h] }
    }

    pub fn from(p: &Plane, f: impl Fn(f32) -> bool) -> Mask {
        Mask { w: p.w, h: p.h, v: p.v.iter().map(|&x| f(x)).collect() }
    }

    pub fn at(&self, x: usize, y: usize) -> bool {
        self.v[y * self.w + x]
    }

    pub fn set(&mut self, x: usize, y: usize, value: bool) {
        self.v[y * self.w + x] = value;
    }

    pub fn and(&self, o: &Mask) -> Mask {
        Mask { w: self.w, h: self.h, v: self.v.iter().zip(&o.v).map(|(&a, &b)| a && b).collect() }
    }

    pub fn or(&self, o: &Mask) -> Mask {
        Mask { w: self.w, h: self.h, v: self.v.iter().zip(&o.v).map(|(&a, &b)| a || b).collect() }
    }

    pub fn not(&self) -> Mask {
        Mask { w: self.w, h: self.h, v: self.v.iter().map(|&a| !a).collect() }
    }

    pub fn count(&self) -> usize {
        self.v.iter().filter(|&&a| a).count()
    }

    pub fn any(&self) -> bool {
        self.v.iter().any(|&a| a)
    }

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

    pub fn scale_up(&self, factor: usize) -> Mask {
        let mut out = Mask::new(self.w * factor, self.h * factor);
        for y in 0..out.h {
            for x in 0..out.w {
                out.v[y * out.w + x] = self.v[(y / factor) * self.w + x / factor];
            }
        }
        out
    }

    pub fn as_plane(&self) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().map(|&a| if a { 1.0 } else { 0.0 }).collect() }
    }
}

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

pub fn fill_holes(m: &Mask) -> Mask {
    let mut outside = Mask::new(m.w, m.h);
    let mut stack = Vec::new();
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

pub fn distance(m: &Mask) -> (Plane, Vec<(usize, usize)>) {
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

#[derive(Clone, Copy)]
pub enum Filter {
    Nearest,
    Bilinear,
    Bicubic,
    Lanczos,
}

fn filter_weight(filter: Filter, x: f32) -> f32 {
    let x = x.abs();
    match filter {
        Filter::Nearest => {
            if x < 0.5 { 1.0 } else { 0.0 }
        }
        Filter::Bilinear => (1.0 - x).max(0.0),
        Filter::Bicubic => {
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

fn filter_support(filter: Filter) -> f32 {
    match filter {
        Filter::Nearest => 0.5,
        Filter::Bilinear => 1.0,
        Filter::Bicubic => 2.0,
        Filter::Lanczos => 3.0,
    }
}

fn resample_axis(src_len: usize, dst_len: usize, filter: Filter) -> Vec<(usize, Vec<f32>)> {
    let scale = src_len as f32 / dst_len as f32;
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

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn uniform(&mut self) -> f32 {
        ((self.next() >> 40) as f32 + 0.5) / (1u64 << 24) as f32
    }

    pub fn normal(&mut self) -> f32 {
        let (a, b) = (self.uniform(), self.uniform());
        (-2.0 * a.ln()).sqrt() * (2.0 * std::f32::consts::PI * b).cos()
    }
}

pub fn luminance(rgb: [f32; 3]) -> f32 {
    rgb[0] * 0.299 + rgb[1] * 0.587 + rgb[2] * 0.114
}

#[derive(Clone)]
pub struct Rgba {
    pub w: usize,
    pub h: usize,
    pub v: Vec<[f32; 4]>,
}

impl Rgba {
    pub fn new(w: usize, h: usize) -> Rgba {
        Rgba { w, h, v: vec![[0.0; 4]; w * h] }
    }

    pub fn from_texels(texels: &[[u8; 4]], w: usize, h: usize) -> Rgba {
        Rgba { w, h, v: texels.iter().map(|t| [t[0] as f32 / 255.0, t[1] as f32 / 255.0, t[2] as f32 / 255.0, t[3] as f32 / 255.0]).collect() }
    }

    pub fn at(&self, x: usize, y: usize) -> [f32; 4] {
        self.v[y * self.w + x]
    }

    pub fn channel(&self, c: usize) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().map(|p| p[c]).collect() }
    }

    pub fn lum(&self) -> Plane {
        Plane { w: self.w, h: self.h, v: self.v.iter().map(|p| luminance([p[0], p[1], p[2]])).collect() }
    }

    pub fn crop(&self, x0: usize, y0: usize, x1: usize, y1: usize) -> Rgba {
        let mut out = Rgba::new(x1 - x0, y1 - y0);
        for y in y0..y1 {
            out.v[(y - y0) * out.w..(y - y0 + 1) * out.w].copy_from_slice(&self.v[y * self.w + x0..y * self.w + x1]);
        }
        out
    }

    pub fn paste(&mut self, other: &Rgba, x0: usize, y0: usize) {
        for y in 0..other.h {
            self.v[(y + y0) * self.w + x0..(y + y0) * self.w + x0 + other.w].copy_from_slice(&other.v[y * other.w..(y + 1) * other.w]);
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.v.iter().flat_map(|p| p.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)).collect()
    }
}

pub fn median_of(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = values.len();
    if n % 2 == 1 { values[n / 2] } else { (values[n / 2 - 1] + values[n / 2]) / 2.0 }
}

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
