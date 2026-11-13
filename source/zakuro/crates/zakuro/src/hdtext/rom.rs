//! Reads the data files of the game.
//! It finds the textures in CTPK and CGFX files, decodes them, and reads the message tables of the game.

use std::collections::HashMap;

use zakuro_gpu::texture::{decode_level, TextureFormat};

use super::ops::{Plane, Rgba};

/// A texture of the game, decoded.
pub struct Texture {
    pub width: usize,
    pub height: usize,
    /// The number of the 3DS texture format of the texture.
    pub format: u32,
    /// The hash of the encoded bytes of the texture, which is how the emulator recognises it.
    pub hash: u64,
    /// The decoded pixels, as RGBA bytes.
    pub texels: Vec<[u8; 4]>,
}

impl Texture {
    /// The file name that the emulator looks a replacement up by.
    pub fn name(&self) -> String {
        format!("tex1_{}x{}_{:016X}_{}.png", self.width, self.height, self.hash, self.format)
    }

    /// Decodes the texture into a floating point picture.
    /// The formats that only hold alpha get white colour.
    pub fn rgba(&self) -> Rgba {
        let mut image = Rgba::from_texels(&self.texels, self.width, self.height);
        // Formats 8 and 11 are the alpha-only formats, whose colour channels are not meaningful.
        if self.format == 11 || self.format == 8 {
            for p in image.v.iter_mut() {
                p[0] = 1.0;
                p[1] = 1.0;
                p[2] = 1.0;
            }
        }
        image
    }

    /// Returns the alpha channel of the texture as a plane of values from 0 to 1.
    pub fn alpha(&self) -> Plane {
        Plane { w: self.width, h: self.height, v: self.texels.iter().map(|t| t[3] as f32 / 255.0).collect() }
    }
}

/// Returns the number of bits per pixel of a 3DS texture format, or `None` for an unknown format.
fn bits(format: u32) -> Option<usize> {
    Some(match format {
        0 => 32,
        1 => 24,
        2..=6 => 16,
        7..=9 => 8,
        10..=12 => 4,
        13 => 8,
        _ => return None,
    })
}

/// Decodes the texture whose encoded bytes start at `at` in the data.
/// Returns `None` when the size is not plausible or the bytes run past the end of the data.
pub fn texture(data: &[u8], at: usize, width: usize, height: usize, format: u32) -> Option<Texture> {
    // The 3DS GPU does not take textures larger than 1024 pixels on a side.
    if width == 0 || height == 0 || width > 1024 || height > 1024 {
        return None;
    }
    let size = width * height * bits(format)? / 8;
    let bytes = data.get(at..at + size)?;
    let mut texels = vec![[0u8; 4]; width * height];
    decode_level(bytes, TextureFormat::from_raw(format), width as u32, height as u32, &mut texels);
    Some(Texture { width, height, format, hash: cityhasher::hash(bytes), texels })
}

/// Reads a little-endian 16-bit number.
fn u16_at(d: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize)
}

/// Reads a little-endian 32-bit number.
fn u32_at(d: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?) as usize)
}

/// Follows a self-relative offset, as CGFX files store them.
/// The value is added to the position that it is stored at.
/// Zero means that there is no target.
fn relative(d: &[u8], at: usize) -> Option<usize> {
    let v = i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?);
    (v != 0).then(|| (at as i64 + v as i64) as usize)
}

/// Scans a file for textures and decodes them all, each with the offset of its encoded bytes.
/// It looks for CTPK texture packs and for CGFX texture objects, at any position that is a multiple of four.
pub fn textures_in(d: &[u8]) -> Vec<(usize, Texture)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= d.len() {
        match &d[i..i + 4] {
            b"CTPK" => {
                let count = u16_at(d, i + 6).unwrap_or(0);
                // The offset of the section that holds the texture data, from the start of the pack.
                let section = i + u32_at(d, i + 8).unwrap_or(0);
                for t in 0..count.min(4096) {
                    // Each entry of the pack is 0x20 bytes, after a header of 0x20 bytes.
                    // It holds the offset, the format, the width and the height of a texture.
                    let e = i + 0x20 + t * 0x20;
                    let (Some(off), Some(f), Some(w), Some(h)) = (u32_at(d, e + 8), u32_at(d, e + 12), u16_at(d, e + 16), u16_at(d, e + 18)) else { break };
                    if let Some(tex) = texture(d, section + off, w, h, f as u32) {
                        out.push((section + off, tex));
                    }
                }
            }
            // The TXOB magic sits four bytes after the start of the texture object, which holds the size, the format and a link to the pixels.
            b"TXOB" if i >= 4 => {
                let t = i - 4;
                if let (Some(h), Some(w), Some(f)) = (u32_at(d, t + 0x18), u32_at(d, t + 0x1C), u32_at(d, t + 0x34)) {
                    if let Some(bits_at) = relative(d, t + 0x38).and_then(|image| relative(d, image + 0x0C)) {
                        if let Some(tex) = texture(d, bits_at, w, h, f as u32) {
                            out.push((bits_at, tex));
                        }
                    }
                }
            }
            _ => {}
        }
        i += 4;
    }
    out
}

/// Reads a message table into a map from the key of a message to its text.
pub fn messages(d: &[u8]) -> HashMap<String, String> {
    labelled(d).into_iter().collect()
}

/// Reads a message table as a list of keys and texts, in file order.
/// The header gives the size of the text data, the number of pointers and the number of labels.
/// The texts are UTF-16 strings that end with a zero unit.
/// The keys are zero-terminated ASCII names.
pub fn labelled(d: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let (Some(data_size), Some(pointers), Some(labels)) = (u32_at(d, 4), u32_at(d, 8), u32_at(d, 12)) else { return out };
    // The data starts after a header of 0x20 bytes, then come the pointers, then the table of labels, then the names.
    let table = 0x20 + data_size + pointers * 4;
    let names = table + labels * 8;
    for i in 0..labels {
        let (Some(text_at), Some(name_at)) = (u32_at(d, table + i * 8), u32_at(d, table + i * 8 + 4)) else { break };
        let start_name = names + name_at;
        let Some(end) = d.get(start_name..).and_then(|rest| rest.iter().position(|&b| b == 0)) else { continue };
        let name: String = d[start_name..start_name + end].iter().map(|&b| if b.is_ascii() { b as char } else { '?' }).collect();
        let start = 0x20 + text_at;
        let mut stop = start;
        while stop + 1 < d.len() && !(d[stop] == 0 && d[stop + 1] == 0) {
            stop += 2;
        }
        // The text is read two bytes at a time, up to the first pair of zero bytes.
        let units: Vec<u16> = d[start..stop].as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
        out.push((name, String::from_utf16_lossy(&units)));
    }
    out
}
