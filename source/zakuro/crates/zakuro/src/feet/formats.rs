
//! Readers and writers for the binary formats of the 3DS that the feet need.
//! These are the little endian data of CGFX files, the dictionaries in them, and the ETC1 textures.

use std::collections::HashMap;

/// A file of bytes with readers for little endian values and for the relative pointers of CGFX files.
pub struct Blob<'a>(pub &'a [u8]);

impl Blob<'_> {
    /// Reads a little endian u32.
    pub fn u32(&self, at: usize) -> u32 {
        u32::from_le_bytes(self.0[at..at + 4].try_into().unwrap())
    }

    /// Reads a little endian i32.
    pub fn i32(&self, at: usize) -> i32 {
        self.u32(at) as i32
    }

    /// Reads a little endian u16.
    pub fn u16(&self, at: usize) -> u16 {
        u16::from_le_bytes(self.0[at..at + 2].try_into().unwrap())
    }

    /// Reads a little endian f32 and widens it to f64.
    pub fn f32(&self, at: usize) -> f64 {
        f32::from_bits(self.u32(at)) as f64
    }

    /// Follows a relative pointer.
    /// The value is an offset from the position of the pointer itself.
    /// Zero means a null pointer.
    pub fn rel(&self, at: usize) -> Option<usize> {
        match self.i32(at) {
            0 => None,
            offset => Some((at as i64 + offset as i64) as usize),
        }
    }

    /// Follows a relative pointer that must not be null.
    pub fn to(&self, at: usize) -> Result<usize, String> {
        self.rel(at).ok_or_else(|| format!("a null pointer at 0x{at:X}"))
    }

    /// Reads a string that ends with a zero byte, one byte per character.
    pub fn cstr(&self, at: usize) -> String {
        let end = self.0[at..].iter().position(|&b| b == 0).map_or(self.0.len(), |n| at + n);
        self.0[at..end].iter().map(|&b| b as char).collect()
    }

    /// Reads a dictionary of CGFX and returns its names with the positions of their data.
    /// A missing dictionary gives an empty list.
    pub fn dict(&self, at: Option<usize>) -> Result<Vec<(String, usize)>, String> {
        let Some(at) = at else { return Ok(Vec::new()) };
        if &self.0[at..at + 4] != b"DICT" {
            return Err(format!("no dictionary at 0x{at:X}"));
        }
        let count = self.u32(at + 8) as usize;
        // The first node is the root of the tree and holds no entry, so the entries are the nodes from 1.
        // Each node is 0x10 bytes, with the pointer to the name at 8 and the pointer to the data at 12.
        (1..=count)
            .map(|i| {
                let node = at + 0x0C + 0x10 * i;
                Ok((self.cstr(self.to(node + 8)?), self.to(node + 12)?))
            })
            .collect()
    }
}

/// The bit index of the root node of a dictionary.
const ROOT_BIT: u32 = u32::MAX;

/// Returns the bit `n` of a name, where bit 0 is the lowest bit of the first byte.
/// Bits past the end of the name are zero.
fn bit(name: &[u8], n: u32) -> bool {
    name.get((n >> 3) as usize).is_some_and(|&c| (c >> (n & 7)) & 1 == 1)
}

/// Returns the highest bit in which two names differ, or nothing when they are equal.
fn highest_difference(a: &[u8], b: &[u8]) -> Option<u32> {
    (0..8 * a.len().max(b.len()) as u32).rev().find(|&n| bit(a, n) != bit(b, n))
}

/// A node of a dictionary.
/// A dictionary is a radix tree over the bits of the names.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The index of the bit of the name that this node tests.
    pub bit: u32,
    /// The index of the node to visit when the bit is clear.
    pub left: u16,
    /// The index of the node to visit when the bit is set.
    pub right: u16,
    /// The name of the entry.
    /// It is empty for the root.
    pub name: String,
}

/// Walks the tree with a name and returns the index of the node where the walk ends.
/// The walk ends when a link goes back up to a node with a lower or equal bit index.
fn search(nodes: &[Node], name: &[u8]) -> usize {
    let (mut previous, mut current) = (0, nodes[0].left as usize);
    while nodes[current].bit < nodes[previous].bit {
        previous = current;
        current = if bit(name, nodes[current].bit) { nodes[current].right } else { nodes[current].left } as usize;
    }
    current
}

/// Builds the nodes of a dictionary for a list of names.
/// It inserts the names one after the other.
/// The root is the first node.
/// The node of each name has the index of the name plus one.
/// A name that appears twice is an error.
pub fn dict_nodes(names: &[String]) -> Result<Vec<Node>, String> {
    let mut nodes = vec![Node { bit: ROOT_BIT, left: 0, right: 0, name: String::new() }];
    for name in names {
        let key = name.as_bytes();
        // The new node tests the highest bit in which the name differs from the name where the walk ends.
        let ending = search(&nodes, key);
        let difference = highest_difference(key, nodes[ending].name.as_bytes()).ok_or_else(|| format!("{name} twice in a dictionary"))?;
        // The walk goes down again and stops in front of the first node that tests a lower bit than the new one.
        let (mut previous, mut current, mut went_right) = (0usize, nodes[0].left as usize, false);
        while nodes[current].bit < nodes[previous].bit && nodes[current].bit > difference {
            previous = current;
            went_right = bit(key, nodes[current].bit);
            current = if went_right { nodes[current].right } else { nodes[current].left } as usize;
        }
        // The new node links to itself on the side of its own bit, and to the node that it replaces on the other side.
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

/// Writes the nodes of a dictionary as bytes, for a dictionary that is placed at position `at` in its file.
/// The maps give the positions of the name and the data of each entry.
/// These positions are stored as pointers relative to the fields.
pub fn dict_bytes(nodes: &[Node], at: usize, name_at: &HashMap<String, usize>, data_at: &HashMap<String, usize>) -> Vec<u8> {
    // The header is the magic, the size of the dictionary, and the number of entries without the root.
    let mut out = b"DICT".to_vec();
    out.extend_from_slice(&((0x0C + 0x10 * nodes.len()) as u32).to_le_bytes());
    out.extend_from_slice(&((nodes.len() - 1) as u32).to_le_bytes());
    for (i, node) in nodes.iter().enumerate() {
        // The root has no name and no data, so both of its pointers are null.
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

/// A decoded picture.
pub struct Image {
    /// The width in pixels.
    pub width: usize,
    /// The height in pixels.
    pub height: usize,
    /// The colors of the pixels, row by row, as red, green and blue from 0 to 255.
    pub rgb: Vec<[f64; 3]>,
}

impl Image {
    /// Returns the color of the pixel at the given column and row.
    pub fn at(&self, x: usize, y: usize) -> [f64; 3] {
        self.rgb[y * self.width + x]
    }
}

/// The brightness modifiers of the eight ETC1 tables, as a small and a large step.
const MODIFIERS: [[i32; 2]; 8] = [[2, 8], [5, 17], [9, 29], [13, 42], [18, 60], [24, 80], [33, 106], [47, 183]];

/// Decodes an ETC1 texture of the 3DS, or an ETC1A4 texture.
/// An ETC1A4 texture has four bits of alpha in front of each block.
/// The alpha is dropped.
/// It returns nothing for other formats or when the data is too short.
pub fn decode_etc1(data: &[u8], width: usize, height: usize, format: u32) -> Option<Image> {
    // Format 12 is ETC1 with 8 bytes per block.
    // Format 13 is ETC1A4 with 16 bytes per block.
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
        // The color data is the last eight bytes of each block.
        let bytes = &data[block * step + step - 8..block * step + step];
        let word = u64::from_le_bytes(bytes.try_into().unwrap());
        // The blocks of 4 by 4 pixels are stored in tiles of 8 by 8 pixels, with four blocks per tile in rows of two.
        let (tile, inner) = (block / 4, block % 4);
        let columns = width / 8;
        let (tile_x, tile_y) = (tile % columns, tile / columns);
        let (bx, by) = (tile_x * 8 + (inner % 2) * 4, tile_y * 8 + (inner / 2) * 4);
        // The pixels of a block run down the columns, so the index gives the column first.
        for (pixel, colour) in etc1_block(word).into_iter().enumerate() {
            let (x, y) = (pixel / 4, pixel % 4);
            rgb[(by + y) * width + bx + x] = colour.map(f64::from);
        }
    }
    Some(Image { width, height, rgb })
}

/// Decodes one ETC1 block of 4 by 4 pixels from its 64 bit word.
/// The block has two sub blocks with a base color each.
/// Every pixel adds a modifier to the base color of its sub block.
fn etc1_block(w: u64) -> [[u8; 3]; 16] {
    let field = |shift: u32, size: u32| ((w >> shift) & ((1 << size) - 1)) as i32;
    // The differential bit picks the color mode.
    // The flip bit splits the block into top and bottom instead of left and right.
    let (differential, flip) = (field(33, 1) == 1, field(32, 1) == 1);
    let mut base = [[0i32; 3]; 2];
    // These are the shifts per channel of the first five bit color, the three bit delta, and the two four bit colors.
    for (c, (s5, s3, s4a, s4b)) in [(59, 56, 60, 56), (51, 48, 52, 48), (43, 40, 44, 40)].into_iter().enumerate() {
        let first5 = field(s5, 5);
        // The delta is a signed three bit number.
        let mut delta = field(s3, 3);
        if delta >= 4 {
            delta -= 8;
        }
        let second5 = (first5 + delta) & 31;
        // Five bit colors are widened to eight bits by repeating the top bits.
        // Four bit colors are widened to eight bits by multiplying with 17.
        if differential {
            base[0][c] = (first5 << 3) | (first5 >> 2);
            base[1][c] = (second5 << 3) | (second5 >> 2);
        } else {
            base[0][c] = field(s4a, 4) * 17;
            base[1][c] = field(s4b, 4) * 17;
        }
    }
    // Each sub block has its own table of modifiers.
    let tables = [field(37, 3) as usize, field(34, 3) as usize];
    let mut out = [[0u8; 3]; 16];
    for index in 0..16u32 {
        let (x, y) = (index / 4, index % 4);
        let second = if flip { y >= 2 } else { x >= 2 } as usize;
        // Two bits per pixel pick the modifier. The high bits are stored in bits 16 to 31, and the low bits in bits 0 to 15.
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

    /// Every name is found at its own node.
    /// A name that appears twice is refused.
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
