
use std::collections::HashMap;

pub struct Blob<'a>(pub &'a [u8]);

impl Blob<'_> {
    pub fn u32(&self, at: usize) -> u32 {
        u32::from_le_bytes(self.0[at..at + 4].try_into().unwrap())
    }

    pub fn i32(&self, at: usize) -> i32 {
        self.u32(at) as i32
    }

    pub fn u16(&self, at: usize) -> u16 {
        u16::from_le_bytes(self.0[at..at + 2].try_into().unwrap())
    }

    pub fn f32(&self, at: usize) -> f64 {
        f32::from_bits(self.u32(at)) as f64
    }

    pub fn rel(&self, at: usize) -> Option<usize> {
        match self.i32(at) {
            0 => None,
            offset => Some((at as i64 + offset as i64) as usize),
        }
    }

    pub fn to(&self, at: usize) -> Result<usize, String> {
        self.rel(at).ok_or_else(|| format!("a null pointer at 0x{at:X}"))
    }

    pub fn cstr(&self, at: usize) -> String {
        let end = self.0[at..].iter().position(|&b| b == 0).map_or(self.0.len(), |n| at + n);
        self.0[at..end].iter().map(|&b| b as char).collect()
    }

    pub fn dict(&self, at: Option<usize>) -> Result<Vec<(String, usize)>, String> {
        let Some(at) = at else { return Ok(Vec::new()) };
        if &self.0[at..at + 4] != b"DICT" {
            return Err(format!("no dictionary at 0x{at:X}"));
        }
        let count = self.u32(at + 8) as usize;
        (1..=count)
            .map(|i| {
                let node = at + 0x0C + 0x10 * i;
                Ok((self.cstr(self.to(node + 8)?), self.to(node + 12)?))
            })
            .collect()
    }
}

const ROOT_BIT: u32 = u32::MAX;

fn bit(name: &[u8], n: u32) -> bool {
    name.get((n >> 3) as usize).is_some_and(|&c| (c >> (n & 7)) & 1 == 1)
}

fn highest_difference(a: &[u8], b: &[u8]) -> Option<u32> {
    (0..8 * a.len().max(b.len()) as u32).rev().find(|&n| bit(a, n) != bit(b, n))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub bit: u32,
    pub left: u16,
    pub right: u16,
    pub name: String,
}

fn search(nodes: &[Node], name: &[u8]) -> usize {
    let (mut previous, mut current) = (0, nodes[0].left as usize);
    while nodes[current].bit < nodes[previous].bit {
        previous = current;
        current = if bit(name, nodes[current].bit) { nodes[current].right } else { nodes[current].left } as usize;
    }
    current
}

pub fn dict_nodes(names: &[String]) -> Result<Vec<Node>, String> {
    let mut nodes = vec![Node { bit: ROOT_BIT, left: 0, right: 0, name: String::new() }];
    for name in names {
        let key = name.as_bytes();
        let ending = search(&nodes, key);
        let difference = highest_difference(key, nodes[ending].name.as_bytes()).ok_or_else(|| format!("{name} twice in a dictionary"))?;
        let (mut previous, mut current, mut went_right) = (0usize, nodes[0].left as usize, false);
        while nodes[current].bit < nodes[previous].bit && nodes[current].bit > difference {
            previous = current;
            went_right = bit(key, nodes[current].bit);
            current = if went_right { nodes[current].right } else { nodes[current].left } as usize;
        }
        let index = nodes.len() as u16;
        let (left, right) = if bit(key, difference) { (current as u16, index) } else { (index, current as u16) };
        nodes.push(Node { bit: difference, left, right, name: name.clone() });
        if previous == 0 {
            nodes[0].left = index;
        } else if went_right {
            nodes[previous].right = index;
        } else {
            nodes[previous].left = index;
        }
    }
    Ok(nodes)
}

pub fn dict_bytes(nodes: &[Node], at: usize, name_at: &HashMap<String, usize>, data_at: &HashMap<String, usize>) -> Vec<u8> {
    let mut out = b"DICT".to_vec();
    out.extend_from_slice(&((0x0C + 0x10 * nodes.len()) as u32).to_le_bytes());
    out.extend_from_slice(&((nodes.len() - 1) as u32).to_le_bytes());
    for (i, node) in nodes.iter().enumerate() {
        let here = at + 0x0C + 0x10 * i;
        let (names, data) = if node.name.is_empty() {
            (0, 0)
        } else {
            (name_at[&node.name] as i64 - (here + 8) as i64, data_at[&node.name] as i64 - (here + 12) as i64)
        };
        out.extend_from_slice(&node.bit.to_le_bytes());
        out.extend_from_slice(&node.left.to_le_bytes());
        out.extend_from_slice(&node.right.to_le_bytes());
        out.extend_from_slice(&(names as i32).to_le_bytes());
        out.extend_from_slice(&(data as i32).to_le_bytes());
    }
    out
}

pub fn unpack(data: &[u8]) -> Result<Vec<u8>, String> {
    match data.first() {
        Some(0x13) => lz11_unpack(&data[4..]),
        Some(0x10 | 0x11) => lz11_unpack(data),
        _ => Ok(data.to_vec()),
    }
}

fn lz11_unpack(src: &[u8]) -> Result<Vec<u8>, String> {
    let short = || "an LZ stream ends early".to_owned();
    let kind = src[0];
    let (mut length, mut at) = (src[1] as usize | (src[2] as usize) << 8 | (src[3] as usize) << 16, 4);
    if length == 0 {
        length = u32::from_le_bytes(src.get(4..8).ok_or_else(short)?.try_into().unwrap()) as usize;
        at = 8;
    }
    let mut out = Vec::with_capacity(length);
    let byte = |at: usize| src.get(at).copied().ok_or_else(short);
    while out.len() < length {
        let flags = byte(at)?;
        at += 1;
        for bit in 0..8 {
            if out.len() >= length {
                break;
            }
            if flags & (0x80 >> bit) == 0 {
                out.push(byte(at)?);
                at += 1;
                continue;
            }
            let (count, disp);
            let b0 = byte(at)? as usize;
            if kind == 0x10 {
                (count, disp) = ((b0 >> 4) + 3, ((b0 & 0xF) << 8 | byte(at + 1)? as usize) + 1);
                at += 2;
            } else {
                match b0 >> 4 {
                    0 => {
                        let (b1, b2) = (byte(at + 1)? as usize, byte(at + 2)? as usize);
                        (count, disp) = (((b0 & 0xF) << 4 | b1 >> 4) + 0x11, ((b1 & 0xF) << 8 | b2) + 1);
                        at += 3;
                    }
                    1 => {
                        let (b1, b2, b3) = (byte(at + 1)? as usize, byte(at + 2)? as usize, byte(at + 3)? as usize);
                        (count, disp) = (((b0 & 0xF) << 12 | b1 << 4 | b2 >> 4) + 0x111, ((b2 & 0xF) << 8 | b3) + 1);
                        at += 4;
                    }
                    top => {
                        (count, disp) = (top + 1, ((b0 & 0xF) << 8 | byte(at + 1)? as usize) + 1);
                        at += 2;
                    }
                }
            }
            if disp > out.len() {
                return Err("an LZ stream reaches before its start".into());
            }
            for _ in 0..count {
                out.push(out[out.len() - disp]);
            }
        }
    }
    Ok(out)
}

pub fn lz11(data: &[u8], chain: usize) -> Vec<u8> {
    let n = data.len();
    let mut out = vec![0x11, n as u8, (n >> 8) as u8, (n >> 16) as u8];
    let mut heads: HashMap<[u8; 3], i64> = HashMap::new();
    let mut previous = vec![-1i64; n];
    let remember = |i: usize, heads: &mut HashMap<[u8; 3], i64>, previous: &mut Vec<i64>| {
        if i + 3 <= n {
            let key = [data[i], data[i + 1], data[i + 2]];
            previous[i] = heads.get(&key).copied().unwrap_or(-1);
            heads.insert(key, i as i64);
        }
    };
    let mut at = 0;
    while at < n {
        let flag_at = out.len();
        out.push(0);
        let mut flags = 0u8;
        for bit in 0..8 {
            if at >= n {
                break;
            }
            let (mut best_len, mut best_disp) = (0usize, 0usize);
            if at + 3 <= n {
                let mut cand = heads.get(&[data[at], data[at + 1], data[at + 2]]).copied().unwrap_or(-1);
                let limit = (n - at).min(0x10110);
                let mut tries = 0;
                while cand >= 0 && at - cand as usize <= 0x1000 && tries < chain {
                    let c = cand as usize;
                    let mut length = 3;
                    while length < limit && data[c + length] == data[at + length] {
                        length += 1;
                    }
                    if length > best_len {
                        (best_len, best_disp) = (length, at - c);
                        if length == limit {
                            break;
                        }
                    }
                    cand = previous[c];
                    tries += 1;
                }
            }
            if best_len >= 3 {
                flags |= 0x80 >> bit;
                let d = best_disp - 1;
                if best_len <= 16 {
                    out.extend_from_slice(&[((best_len - 1) << 4 | d >> 8) as u8, d as u8]);
                } else if best_len <= 0x110 {
                    let m = best_len - 0x11;
                    out.extend_from_slice(&[(m >> 4) as u8, ((m & 0xF) << 4 | d >> 8) as u8, d as u8]);
                } else {
                    let m = best_len - 0x111;
                    out.extend_from_slice(&[(0x10 | m >> 12) as u8, (m >> 4) as u8, ((m & 0xF) << 4 | d >> 8) as u8, d as u8]);
                }
                for i in at..at + best_len {
                    remember(i, &mut heads, &mut previous);
                }
                at += best_len;
            } else {
                out.push(data[at]);
                remember(at, &mut heads, &mut previous);
                at += 1;
            }
        }
        out[flag_at] = flags;
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}

pub fn lz13(data: &[u8]) -> Vec<u8> {
    let n = data.len();
    let mut out = vec![0x13, n as u8, (n >> 8) as u8, (n >> 16) as u8];
    out.extend(lz11(data, 64));
    out
}

pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<[f64; 3]>,
}

impl Image {
    pub fn at(&self, x: usize, y: usize) -> [f64; 3] {
        self.rgb[y * self.width + x]
    }
}

const MODIFIERS: [[i32; 2]; 8] = [[2, 8], [5, 17], [9, 29], [13, 42], [18, 60], [24, 80], [33, 106], [47, 183]];

pub fn decode_etc1(data: &[u8], width: usize, height: usize, format: u32) -> Option<Image> {
    let step = match format {
        12 => 8,
        13 => 16,
        _ => return None,
    };
    let blocks = (width / 4) * (height / 4);
    if data.len() < blocks * step {
        return None;
    }
    let mut rgb = vec![[0.0; 3]; width * height];
    for block in 0..blocks {
        let bytes = &data[block * step + step - 8..block * step + step];
        let word = u64::from_le_bytes(bytes.try_into().unwrap());
        let (tile, inner) = (block / 4, block % 4);
        let columns = width / 8;
        let (tile_x, tile_y) = (tile % columns, tile / columns);
        let (bx, by) = (tile_x * 8 + (inner % 2) * 4, tile_y * 8 + (inner / 2) * 4);
        for (pixel, colour) in etc1_block(word).into_iter().enumerate() {
            let (x, y) = (pixel / 4, pixel % 4);
            rgb[(by + y) * width + bx + x] = colour.map(f64::from);
        }
    }
    Some(Image { width, height, rgb })
}

fn etc1_block(w: u64) -> [[u8; 3]; 16] {
    let field = |shift: u32, size: u32| ((w >> shift) & ((1 << size) - 1)) as i32;
    let (differential, flip) = (field(33, 1) == 1, field(32, 1) == 1);
    let mut base = [[0i32; 3]; 2];
    for (c, (s5, s3, s4a, s4b)) in [(59, 56, 60, 56), (51, 48, 52, 48), (43, 40, 44, 40)].into_iter().enumerate() {
        let first5 = field(s5, 5);
        let mut delta = field(s3, 3);
        if delta >= 4 {
            delta -= 8;
        }
        let second5 = (first5 + delta) & 31;
        if differential {
            base[0][c] = (first5 << 3) | (first5 >> 2);
            base[1][c] = (second5 << 3) | (second5 >> 2);
        } else {
            base[0][c] = field(s4a, 4) * 17;
            base[1][c] = field(s4b, 4) * 17;
        }
    }
    let tables = [field(37, 3) as usize, field(34, 3) as usize];
    let mut out = [[0u8; 3]; 16];
    for index in 0..16u32 {
        let (x, y) = (index / 4, index % 4);
        let second = if flip { y >= 2 } else { x >= 2 } as usize;
        let msb = ((w >> (16 + index)) & 1) as usize;
        let lsb = ((w >> index) & 1) as usize;
        let [small, large] = MODIFIERS[tables[second]];
        let modifier = match msb * 2 + lsb {
            0 => small,
            1 => large,
            2 => -small,
            _ => -large,
        };
        out[index as usize] = base[second].map(|c| (c + modifier).clamp(0, 255) as u8);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lz11_unpacks_to_what_it_packed() {
        let data: Vec<u8> = (0..5000u32).map(|i| ((i * 7) % 13) as u8).chain(std::iter::repeat_n(9, 300)).collect();
        let packed = lz13(&data);
        assert!(packed.len() < data.len() / 4);
        assert_eq!(unpack(&packed).unwrap(), data);
    }

    #[test]
    fn a_dictionary_finds_its_names() {
        let names: Vec<String> = ["hip", "l_leg1", "l_leg2", "l_leg3", "r_leg2", "l_shoe", "r_shoe"].map(String::from).to_vec();
        let nodes = dict_nodes(&names).unwrap();
        for (i, name) in names.iter().enumerate() {
            assert_eq!(search(&nodes, name.as_bytes()), i + 1, "{name}");
        }
        assert!(dict_nodes(&["a".to_owned(), "a".to_owned()]).is_err());
    }
}
