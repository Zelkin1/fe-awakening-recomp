
use std::collections::HashMap;

use super::formats::Blob;

pub const FLOAT: u32 = 0x1406;
const SHORT: u32 = 0x1402;
const UBYTE: u32 = 0x1401;
const BYTE: u32 = 0x1400;
const SHORT_INDICES: u32 = 0x1403;
pub const POSITION: u32 = 0;
pub const NORMAL: u32 = 1;
pub const UV: u32 = 4;
pub const BONES: u32 = 7;
pub const WEIGHTS: u32 = 8;

pub fn width(format: u32) -> Result<usize, String> {
    match format {
        FLOAT => Ok(4),
        SHORT => Ok(2),
        UBYTE | BYTE => Ok(1),
        _ => Err(format!("a vertex format 0x{format:X} this doesn't read")),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Attr {
    pub format: u32,
    pub count: usize,
    pub scale: f64,
    pub offset: usize,
}

pub struct Vertices {
    pub position: Vec<[f64; 3]>,
    pub uv: Option<Vec<[f64; 2]>>,
    pub bones: Option<Vec<Vec<i64>>>,
    pub weights: Option<Vec<Vec<f64>>>,
    pub stride: usize,
    pub attrs: HashMap<u32, Attr>,
    pub count: usize,
    pub data_at: usize,
    pub size: usize,
    pub buffer: usize,
}

pub struct SubMesh {
    pub palette: Vec<u32>,
    pub triangles: Vec<[usize; 3]>,
}

pub fn parts(b: &Blob) -> Result<(usize, usize, usize), String> {
    if b.0.get(0..4) != Some(b"CGFX") {
        return Err("not a CGFX file".into());
    }
    let models = b.dict(b.rel(b.u16(6) as usize + 12))?;
    let model = models.first().ok_or("no model")?.1;
    Ok((model, b.to(model + 0xE0)?, b.to(b.to(model + 0xC8)?)?))
}

pub fn vertices(b: &Blob, shape: usize) -> Result<Vertices, String> {
    let buffer = b.to(b.to(shape + 0x3C)?)?;
    let (size, data_at, stride) = (b.u32(buffer + 0x14) as usize, b.to(buffer + 0x18)?, b.u32(buffer + 0x24) as usize);
    let mut attrs = HashMap::new();
    let list = b.to(buffer + 0x2C)?;
    for i in 0..b.u32(buffer + 0x28) as usize {
        let at = b.to(list + 4 * i)?;
        let attr = Attr { format: b.u32(at + 36), count: b.u32(at + 40) as usize, scale: b.f32(at + 44), offset: b.u32(at + 48) as usize };
        attrs.insert(b.u32(at + 4), attr);
    }
    if stride == 0 || data_at + size > b.0.len() {
        return Err("a vertex buffer outside its file".into());
    }
    let count = size / stride;
    let take = |usage: u32| -> Result<Option<Vec<Vec<f64>>>, String> {
        let Some(attr) = attrs.get(&usage) else { return Ok(None) };
        let w = width(attr.format)?;
        let values = (0..count)
            .map(|i| {
                (0..attr.count)
                    .map(|c| {
                        let at = data_at + i * stride + attr.offset + w * c;
                        let value = match attr.format {
                            FLOAT => b.f32(at),
                            SHORT => b.u16(at) as i16 as f64,
                            UBYTE => b.0[at] as f64,
                            _ => b.0[at] as i8 as f64,
                        };
                        value * attr.scale
                    })
                    .collect()
            })
            .collect();
        Ok(Some(values))
    };
    let position = take(POSITION)?.ok_or("no positions")?.into_iter().map(|p| [p[0], p[1], p[2]]).collect();
    let uv = take(UV)?.map(|uv| uv.into_iter().map(|t| [t[0], t[1]]).collect());
    let bones = take(BONES)?.map(|bones| bones.into_iter().map(|row| row.into_iter().map(|v| v as i64).collect()).collect());
    Ok(Vertices { position, uv, bones, weights: take(WEIGHTS)?, stride, attrs, count, data_at, size, buffer })
}

pub fn submeshes(b: &Blob, shape: usize) -> Result<Vec<SubMesh>, String> {
    let list = b.to(shape + 0x30)?;
    let mut out = Vec::new();
    for i in 0..b.u32(shape + 0x2C) as usize {
        let sm = b.to(list + 4 * i)?;
        let count = b.u32(sm) as usize;
        let palette = match count {
            0 => Vec::new(),
            _ => {
                let at = b.to(sm + 4)?;
                (0..count).map(|k| b.u32(at + 4 * k)).collect()
            }
        };
        let mut triangles = Vec::new();
        let faces = b.rel(sm + 0x10);
        for f in 0..b.u32(sm + 0x0C) as usize {
            let face = b.to(faces.ok_or("no faces")? + 4 * f)?;
            let descs = b.rel(face + 4);
            for d in 0..b.u32(face) as usize {
                let desc = b.to(descs.ok_or("no index streams")? + 4 * d)?;
                let (format, size, data) = (b.u32(desc), b.u32(desc + 8) as usize, b.to(desc + 12)?);
                let indices: Vec<usize> = match format {
                    SHORT_INDICES => (0..size / 2).map(|k| b.u16(data + 2 * k) as usize).collect(),
                    _ => b.0[data..data + size].iter().map(|&i| i as usize).collect(),
                };
                triangles.extend(indices.chunks_exact(3).map(|t| [t[0], t[1], t[2]]));
            }
        }
        out.push(SubMesh { palette, triangles });
    }
    Ok(out)
}
