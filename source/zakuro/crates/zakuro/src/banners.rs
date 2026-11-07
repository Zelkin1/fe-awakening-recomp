use zakuro_gpu::{BannerSide, Banners};

const WIDTH: usize = 320;
const HEIGHT: usize = 240;
const GROUND_TOLERANCE: f32 = 12.0;
const FAINTEST_ALPHA: f32 = 0.1;
const WEAKEST_TEXT: f32 = 40.0;
const GAUGE_SEEN: f32 = 50.0;
const TICK_LIT: f32 = 100.0;
const MOST_MOVE: [f32; 2] = [14.0, 9.0];

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
    pub ground: [u8; 3],
    pub down: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Side {
    pub name: Text,
    pub hp: Text,
    pub weapon: Text,
    pub values: [Text; 3],
    pub ticks: Vec<[u32; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Read {
    pub sides: [Side; 2],
    pub labels: [Text; 3],
}

fn pixel(picture: &[u8], x: usize, y: usize) -> [u8; 3] {
    let at = (y * WIDTH + x) * 4;
    [picture[at], picture[at + 1], picture[at + 2]]
}

fn clamped_ranges([left, top, width, height]: [u16; 4], [moved_x, moved_y]: [i32; 2]) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
    let clamp = |value: i32, most: usize| value.clamp(0, most as i32) as usize;
    let (x0, y0) = (clamp(left as i32 + moved_x, WIDTH), clamp(top as i32 + moved_y, HEIGHT));
    let (x1, y1) = (clamp(left as i32 + width as i32 + moved_x, WIDTH), clamp(top as i32 + height as i32 + moved_y, HEIGHT));
    (x0..x1, y0..y1)
}

pub fn ground(picture: &[u8], rect: [u16; 4], moved: [i32; 2]) -> [u8; 3] {
    let (xs, ys) = clamped_ranges(rect, moved);
    let mut buckets: std::collections::HashMap<[u8; 3], (u32, [u32; 3])> = std::collections::HashMap::new();
    for y in ys {
        for x in xs.clone() {
            let color = pixel(picture, x, y);
            let (count, sum) = buckets.entry(color.map(|channel| channel / 32)).or_default();
            *count += 1;
            for i in 0..3 {
                sum[i] += color[i] as u32;
            }
        }
    }
    buckets.values().max_by_key(|(count, _)| *count).map_or([0; 3], |&(count, sum)| sum.map(|channel| (channel / count.max(1)) as u8))
}

pub fn text(picture: &[u8], rect: [u16; 4], moved: [i32; 2], ground: [u8; 3]) -> Text {
    let (xs, ys) = clamped_ranges(rect, moved);
    let size = [xs.len(), ys.len()];
    let ground_color = ground.map(f32::from);
    let distance_from_ground = |x: usize, y: usize| {
        let color = pixel(picture, x, y).map(f32::from);
        (0..3).map(|i| (color[i] - ground_color[i]).abs()).fold(0.0, f32::max)
    };
    let strongest = ys.clone().flat_map(|y| xs.clone().map(move |x| (x, y))).map(|(x, y)| distance_from_ground(x, y)).fold(0.0, f32::max);
    let mut rgba = Vec::with_capacity(size[0] * size[1] * 4);
    for y in ys {
        for x in xs.clone() {
            let color = pixel(picture, x, y).map(f32::from);
            let alpha = if strongest < WEAKEST_TEXT { 0.0 } else { ((distance_from_ground(x, y) - GROUND_TOLERANCE) / (strongest - GROUND_TOLERANCE)).clamp(0.0, 1.0) };
            let alpha = if alpha < FAINTEST_ALPHA { 0.0 } else { alpha };
            let text_color = if alpha > 0.0 { [0, 1, 2].map(|i| ((color[i] - ground_color[i] * (1.0 - alpha)) / alpha).clamp(0.0, 255.0) as u8) } else { [0; 3] };
            rgba.extend_from_slice(&[text_color[0], text_color[1], text_color[2], (alpha * 255.0).round() as u8]);
        }
    }
    trimmed(Text { size, rgba, ground, down: 0.0 })
}

fn trimmed(text: Text) -> Text {
    let [width, height] = text.size;
    let seen = |x: usize, y: usize| text.rgba[(y * width + x) * 4 + 3] > 0;
    let columns: Vec<usize> = (0..width).filter(|&x| (0..height).any(|y| seen(x, y))).collect();
    let rows: Vec<usize> = (0..height).filter(|&y| (0..width).any(|x| seen(x, y))).collect();
    let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) = (columns.first(), columns.last(), rows.first(), rows.last()) else {
        return Text { size: [0, 0], rgba: Vec::new(), ground: text.ground, down: 0.0 };
    };
    let mut rgba = Vec::with_capacity((x1 - x0 + 1) * (y1 - y0 + 1) * 4);
    for y in y0..=y1 {
        rgba.extend_from_slice(&text.rgba[(y * width + x0) * 4..(y * width + x1 + 1) * 4]);
    }
    let down = (y0 + y1 + 1) as f32 / 2.0 - height as f32 / 2.0;
    Text { size: [x1 - x0 + 1, y1 - y0 + 1], rgba, ground: text.ground, down }
}

pub fn ticks(picture: &[u8], rect: [u16; 4], moved: [i32; 2]) -> Vec<[u32; 2]> {
    let (xs, ys) = clamped_ranges(rect, moved);
    let light = |x: usize, y: usize| {
        let [r, g, b] = pixel(picture, x, y).map(f32::from);
        0.299 * r + 0.587 * g + 0.114 * b
    };
    let runs = |y: usize, level: f32| {
        let mut count = 0;
        let mut inside = false;
        for x in xs.clone() {
            let on = light(x, y) > level;
            count += (on && !inside) as u32;
            inside = on;
        }
        count
    };
    let mut rows = Vec::new();
    let mut row: Option<[u32; 2]> = None;
    for y in ys {
        match runs(y, GAUGE_SEEN) {
            0 => rows.extend(row.take()),
            all => {
                let [lit, most] = row.get_or_insert([0, 0]);
                *lit = (*lit).max(runs(y, TICK_LIT));
                *most = (*most).max(all);
            }
        }
    }
    rows.extend(row);
    rows
}

fn moved(side: &BannerSide, spans: &[(u32, [f32; 4])]) -> [i32; 2] {
    let Some((id, [rest_x, rest_y])) = side.follows else { return [0, 0] };
    let Some(&(_, [x, y, ..])) = spans.iter().find(|&&(marked, _)| marked == id) else { return [0, 0] };
    let [most_x, most_y] = MOST_MOVE;
    [((x * WIDTH as f32) - rest_x as f32).round().clamp(-most_x, most_x) as i32, ((y * HEIGHT as f32) - rest_y as f32).round().clamp(-most_y, most_y) as i32]
}

pub fn read(picture: &[u8], banners: &Banners, spans: &[(u32, [f32; 4])]) -> Option<Read> {
    if picture.len() < WIDTH * HEIGHT * 4 {
        return None;
    }
    let side = |side: &BannerSide| {
        let moved = moved(side, spans);
        let taken = |rect: [u16; 4]| text(picture, rect, moved, ground(picture, rect, moved));
        Side {
            name: taken(side.name),
            hp: taken(side.hp),
            weapon: taken(side.weapon),
            values: side.values.map(taken),
            ticks: ticks(picture, side.gauge, moved),
        }
    };
    let labels = banners.labels.map(|rect| text(picture, rect, [0, 0], ground(picture, rect, [0, 0])));
    Some(Read { sides: [side(&banners.sides[0]), side(&banners.sides[1])], labels })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(ground: [u8; 3]) -> Vec<u8> {
        let mut picture = vec![255u8; WIDTH * HEIGHT * 4];
        for chunk in picture.chunks_mut(4) {
            chunk[..3].copy_from_slice(&ground);
        }
        picture
    }

    fn put(picture: &mut [u8], x: usize, y: usize, rgb: [u8; 3]) {
        picture[(y * WIDTH + x) * 4..][..3].copy_from_slice(&rgb);
    }

    #[test]
    fn text_comes_off_its_ground() {
        let navy = [10, 18, 34];
        let mut picture = picture(navy);
        put(&mut picture, 11, 21, [230, 220, 180]);
        put(&mut picture, 12, 21, [120, 119, 107]);
        let found = ground(&picture, [10, 20, 4, 3], [0, 0]);
        assert_eq!(found, navy);
        let text = text(&picture, [10, 20, 4, 3], [0, 0], found);
        assert_eq!(text.size, [2, 1], "trimmed to what shows");
        let at = |x: usize, y: usize| &text.rgba[(y * 2 + x) * 4..][..4];
        assert_eq!(at(0, 0), [230, 220, 180, 255]);
        let half = at(1, 0);
        assert!((half[3] as i32 - 120).abs() <= 2, "{half:?}");
        assert!(half[0] > 200, "its own colour again, {half:?}");
    }

    fn gauge(rows: &[(usize, usize)], [dx, dy]: [i32; 2]) -> Vec<u8> {
        let mut picture = picture([10, 18, 34]);
        for (row, &(count, lit)) in rows.iter().enumerate() {
            let top = (31 + row as i32 * 10 + dy) as usize;
            for segment in 0..count {
                for column in 0..2 {
                    let x = (40 + dx) as usize + segment * 3 + column;
                    for y in top..top + 4 {
                        put(&mut picture, x, y, if segment < lit { [217, 217, 81] } else { [40, 40, 40] });
                    }
                    for y in top + 4..top + 8 {
                        put(&mut picture, x, y, [78, 78, 10]);
                    }
                }
            }
        }
        picture
    }

    #[test]
    fn a_gauge_comes_in_rows_of_ticks() {
        let rect = [38, 28, 60, 20];
        assert_eq!(ticks(&gauge(&[(12, 12), (3, 1)], [0, 0]), rect, [0, 0]), vec![[12, 12], [1, 3]]);
        assert_eq!(ticks(&gauge(&[(12, 7), (3, 0)], [0, 0]), rect, [0, 0]), vec![[7, 12], [0, 3]]);
        assert_eq!(ticks(&gauge(&[(9, 4)], [0, 0]), rect, [0, 0]), vec![[4, 9]]);
        for moved in [[6, -5], [-4, 7]] {
            assert_eq!(ticks(&gauge(&[(12, 12), (3, 2)], moved), rect, moved), vec![[12, 12], [2, 3]], "{moved:?}");
        }
    }
}
