use std::collections::HashMap;

use zakuro_gpu::texture::{decode_level, TextureFormat};

use super::ops::{Plane, Rgba};

pub struct Texture {
    pub width: usize,
    pub height: usize,
    pub format: u32,
    pub hash: u64,
    pub texels: Vec<[u8; 4]>,
}

impl Texture {
    pub fn name(&self) -> String {
        format!("tex1_{}x{}_{:016X}_{}.png", self.width, self.height, self.hash, self.format)
    }

    pub fn rgba(&self) -> Rgba {
        let mut image = Rgba::from_texels(&self.texels, self.width, self.height);
        if self.format == 11 || self.format == 8 {
            for p in image.v.iter_mut() {
                p[0] = 1.0;
                p[1] = 1.0;
                p[2] = 1.0;
            }
        }
        image
    }

    pub fn alpha(&self) -> Plane {
        Plane { w: self.width, h: self.height, v: self.texels.iter().map(|t| t[3] as f32 / 255.0).collect() }
    }
}

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

pub fn texture(data: &[u8], at: usize, width: usize, height: usize, format: u32) -> Option<Texture> {
    if width == 0 || height == 0 || width > 1024 || height > 1024 {
        return None;
    }
    let size = width * height * bits(format)? / 8;
    let bytes = data.get(at..at + size)?;
    let mut texels = vec![[0u8; 4]; width * height];
    decode_level(bytes, TextureFormat::from_raw(format), width as u32, height as u32, &mut texels);
    Some(Texture { width, height, format, hash: cityhasher::hash(bytes), texels })
}

fn u16_at(d: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn u32_at(d: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn relative(d: &[u8], at: usize) -> Option<usize> {
    let v = i32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?);
    (v != 0).then(|| (at as i64 + v as i64) as usize)
}

pub fn textures_in(d: &[u8]) -> Vec<(usize, Texture)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= d.len() {
        match &d[i..i + 4] {
            b"CTPK" => {
                let count = u16_at(d, i + 6).unwrap_or(0);
                let section = i + u32_at(d, i + 8).unwrap_or(0);
                for t in 0..count.min(4096) {
                    let e = i + 0x20 + t * 0x20;
                    let (Some(off), Some(f), Some(w), Some(h)) = (u32_at(d, e + 8), u32_at(d, e + 12), u16_at(d, e + 16), u16_at(d, e + 18)) else { break };
                    if let Some(tex) = texture(d, section + off, w, h, f as u32) {
                        out.push((section + off, tex));
                    }
                }
            }
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

pub fn messages(d: &[u8]) -> HashMap<String, String> {
    labelled(d).into_iter().collect()
}

pub fn labelled(d: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let (Some(data_size), Some(pointers), Some(labels)) = (u32_at(d, 4), u32_at(d, 8), u32_at(d, 12)) else { return out };
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
        let units: Vec<u16> = d[start..stop].as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
        out.push((name, String::from_utf16_lossy(&units)));
    }
    out
}
