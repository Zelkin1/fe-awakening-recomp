
//! Reads the model of a body from a CGFX file.
//! It finds the vertices, the attributes of the vertices and the triangles of the meshes.

use std::collections::HashMap;

use super::formats::Blob;

/// The OpenGL code of a 32 bit float.
pub const FLOAT: u32 = 0x1406;
/// The OpenGL code of a signed 16 bit integer.
const SHORT: u32 = 0x1402;
/// The OpenGL code of an unsigned 8 bit integer.
const UBYTE: u32 = 0x1401;
/// The OpenGL code of a signed 8 bit integer.
const BYTE: u32 = 0x1400;
/// The OpenGL code of an unsigned 16 bit integer.
/// The indices of triangles use this code.
const SHORT_INDICES: u32 = 0x1403;
/// The usage of an attribute that holds the position of a vertex.
pub const POSITION: u32 = 0;
/// The usage of an attribute that holds the normal of a vertex.
pub const NORMAL: u32 = 1;
/// The usage of an attribute that holds the texture coordinates of a vertex.
pub const UV: u32 = 4;
/// The usage of an attribute that holds the indices of the bones that move a vertex.
pub const BONES: u32 = 7;
/// The usage of an attribute that holds the weights of those bones.
pub const WEIGHTS: u32 = 8;

/// Returns the size in bytes of one value of a vertex format.
/// A format that is not read is an error.
pub fn width(format: u32) -> Result<usize, String> {
    match format {
        FLOAT => Ok(4),
        SHORT => Ok(2),
        UBYTE | BYTE => Ok(1),
        _ => Err(format!("a vertex format 0x{format:X} this doesn't read")),
    }
}

/// One attribute of a vertex, such as its position.
#[derive(Debug, Clone, Copy)]
pub struct Attr {
    /// The OpenGL code of the format of each value.
    pub format: u32,
    /// The number of values, for example three for a position.
    pub count: usize,
    /// The factor that is applied to each value after it is read.
    pub scale: f64,
    /// The position of the attribute inside one vertex, in bytes.
    pub offset: usize,
}

/// The vertices of one shape, as they are read from its vertex buffer.
pub struct Vertices {
    /// The positions of the vertices.
    pub position: Vec<[f64; 3]>,
    /// The texture coordinates, when the shape has them.
    pub uv: Option<Vec<[f64; 2]>>,
    /// The indices of the bones of each vertex, when the shape has them.
    pub bones: Option<Vec<Vec<i64>>>,
    /// The weights of the bones of each vertex, when the shape has them.
    pub weights: Option<Vec<Vec<f64>>>,
    /// The size of one vertex in bytes.
    pub stride: usize,
    /// The attributes of the vertex by their usage.
    pub attrs: HashMap<u32, Attr>,
    /// The number of vertices.
    pub count: usize,
    /// The position of the vertex data in the file.
    pub data_at: usize,
    /// The size of the vertex data in bytes.
    pub size: usize,
    /// The position of the description of the vertex buffer in the file.
    pub buffer: usize,
}

/// A part of a shape with its own triangles.
pub struct SubMesh {
    /// The indices of the bones that the part uses.
    pub palette: Vec<u32>,
    /// The triangles of the part as three indices of vertices each.
    pub triangles: Vec<[usize; 3]>,
}

/// Finds the main parts of the first model of a CGFX file.
/// It returns the position of the model, and the positions of two lists that the model points to at 0xE0 and 0xC8.
pub fn parts(b: &Blob) -> Result<(usize, usize, usize), String> {
    if b.0.get(0..4) != Some(b"CGFX") {
        return Err("not a CGFX file".into());
    }
    // The header size is at 6.
    // The dictionary of the models is the first pointer after the header.
    let models = b.dict(b.rel(b.u16(6) as usize + 12))?;
    let model = models.first().ok_or("no model")?.1;
    Ok((model, b.to(model + 0xE0)?, b.to(b.to(model + 0xC8)?)?))
}

/// Reads the vertices of a shape from its vertex buffer.
/// A shape without positions, or a buffer outside the file, is an error.
pub fn vertices(b: &Blob, shape: usize) -> Result<Vertices, String> {
    // The shape points to a vertex buffer through one more pointer. The buffer holds the size, the data pointer, the stride and the attributes.
    let buffer = b.to(b.to(shape + 0x3C)?)?;
    let (size, data_at, stride) = (b.u32(buffer + 0x14) as usize, b.to(buffer + 0x18)?, b.u32(buffer + 0x24) as usize);
    let mut attrs = HashMap::new();
    let list = b.to(buffer + 0x2C)?;
    for i in 0..b.u32(buffer + 0x28) as usize {
        let at = b.to(list + 4 * i)?;
        // The usage is at 4 in an attribute.
        // The format, the value count, the scale and the offset are at 36, 40, 44 and 48.
        let attr = Attr { format: b.u32(at + 36), count: b.u32(at + 40) as usize, scale: b.f32(at + 44), offset: b.u32(at + 48) as usize };
        attrs.insert(b.u32(at + 4), attr);
    }
    if stride == 0 || data_at + size > b.0.len() {
        return Err("a vertex buffer outside its file".into());
    }
    let count = size / stride;
    // Reads all values of one attribute for every vertex, with the scale applied. It gives nothing when the shape has no such attribute.
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

/// Reads the parts of a shape with their bone palettes and triangles.
pub fn submeshes(b: &Blob, shape: usize) -> Result<Vec<SubMesh>, String> {
    let list = b.to(shape + 0x30)?;
    let mut out = Vec::new();
    for i in 0..b.u32(shape + 0x2C) as usize {
        let sm = b.to(list + 4 * i)?;
        // The part starts with the number of bones in its palette. A part with none has no palette pointer.
        let count = b.u32(sm) as usize;
        let palette = match count {
            0 => Vec::new(),
            _ => {
                let at = b.to(sm + 4)?;
                (0..count).map(|k| b.u32(at + 4 * k)).collect()
            }
        };
        // The faces of the part hold streams of indices.
        // Every three indices of a stream are one triangle.
        let mut triangles = Vec::new();
        let faces = b.rel(sm + 0x10);
        for f in 0..b.u32(sm + 0x0C) as usize {
            let face = b.to(faces.ok_or("no faces")? + 4 * f)?;
            let descs = b.rel(face + 4);
            for d in 0..b.u32(face) as usize {
                let desc = b.to(descs.ok_or("no index streams")? + 4 * d)?;
                let (format, size, data) = (b.u32(desc), b.u32(desc + 8) as usize, b.to(desc + 12)?);
                // The indices are 16 bit when the format says so, and 8 bit otherwise.
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
