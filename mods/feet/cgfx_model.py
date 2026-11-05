"""a body model's skeleton and skinned mesh read from its CGFX: bones with
their world matrices, and per sub-mesh its bone list and triangles over the
one interleaved vertex buffer. reads only."""
import struct

import numpy as np


def bones(b, skeleton):
    """name, joint index, parent index and world matrix (3x4) of each."""
    out = {}
    for name, at in b.dict(b.rel(skeleton + 0x1C)):
        joint, parent = b.i32(at + 0x08), b.i32(at + 0x0C)
        world = np.array(b.f32(at + 0x74, 12)).reshape(3, 4)
        out[joint] = (name, parent, world)
    return out


def model_parts(b):
    """the model's skeleton and its first shape, from the DATA block."""
    models = b.dict(b.rel(b.u16(6) + 12))
    model = models[0][1]
    skeleton = b.rel(model + 0xE0)
    shape = b.rel(b.rel(model + 0xC8))
    return model, skeleton, shape


def vertices(b, shape):
    """positions, uvs, bone indices and weights of the shape's buffer."""
    vb = b.rel(b.rel(shape + 0x3C))
    size, data, stride = b.u32(vb + 0x14), b.rel(vb + 0x18), b.u32(vb + 0x24)
    attrs = {}
    lst = b.rel(vb + 0x2C)
    for i in range(b.u32(vb + 0x28)):
        at = b.rel(lst + 4 * i)
        usage, fmt, count, scale, offset = b.u32(at + 4), b.u32(at + 36), b.u32(at + 40), b.f32(at + 44), b.u32(at + 48)
        attrs[usage] = (fmt, count, scale, offset)
    n = size // stride
    buf = np.frombuffer(b.raw, dtype=np.uint8, count=size, offset=data).reshape(n, stride)

    def take(usage):
        fmt, count, scale, offset = attrs[usage]
        width = {0x1406: 4, 0x1402: 2, 0x1401: 1, 0x1400: 1}[fmt]
        kind = {0x1406: "<f4", 0x1402: "<i2", 0x1401: "u1", 0x1400: "i1"}[fmt]
        cols = buf[:, offset:offset + width * count].copy().view(kind).reshape(n, count)
        return cols.astype(np.float64) * scale
    return {"position": take(0), "uv": take(4) if 4 in attrs else None, "bones": take(7).astype(int) if 7 in attrs else None,
            "weights": take(8) if 8 in attrs else None, "stride": stride, "attrs": attrs, "count": n}


def submeshes(b, shape):
    """each sub-mesh's bone list, skinning and triangle indices."""
    out = []
    lst = b.rel(shape + 0x30)
    for i in range(b.u32(shape + 0x2C)):
        sm = b.rel(lst + 4 * i)
        palette = [b.u32(b.rel(sm + 4) + 4 * k) for k in range(b.u32(sm))]
        skinning = b.u32(sm + 8)
        tris = []
        faces = b.rel(sm + 0x10)
        for f in range(b.u32(sm + 0x0C)):
            face = b.rel(faces + 4 * f)
            descs = b.rel(face + 4)
            for d in range(b.u32(face)):
                desc = b.rel(descs + 4 * d)
                fmt, size, data = b.u32(desc), b.u32(desc + 8), b.rel(desc + 12)
                kind = "<u2" if fmt == 0x1403 else "u1"
                idx = np.frombuffer(b.raw, dtype=kind, count=size // (2 if fmt == 0x1403 else 1), offset=data)
                tris.append(idx.reshape(-1, 3))
        out.append({"palette": palette, "skinning": skinning, "triangles": np.concatenate(tris) if tris else np.zeros((0, 3), int)})
    return out
