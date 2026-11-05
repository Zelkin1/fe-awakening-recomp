"""a body's colour texture read from the player's ROM: the CGFX texture file
beside the model (b/uBody_X_c0.t.lz for b/uBody_X.m.lz), its first image
decoded from the PICA's ETC1 or ETC1A4, its rows in the PICA's order, which
is how Zakuro's texture dumps have them and how texture coordinates find
them, v up from the bottom row. reads only."""
import numpy as np

import cgfx

# the PICA's texture formats this reads, and ETC1's intensity modifiers
ETC1, ETC1A4 = 0xC, 0xD
MODIFIERS = np.array([[2, 8], [5, 17], [9, 29], [13, 42], [18, 60], [24, 80], [33, 106], [47, 183]])


def colour_texture(rom, listing, model_path):
    """the RGB(A) image of the model's colour texture, as an array of rows,
    and the texture's name, or None when it has none this reads."""
    path = model_path.replace(".m.lz", "_c0.t.lz")
    try:
        b = cgfx.Blob(cgfx.load(rom, listing, path))
    except SystemExit:
        return None
    data = b.u16(6)
    textures = b.dict(b.rel(data + 8 + 8 * 1 + 4))
    name, txob = next(((n, t) for n, t in textures if n.endswith("_c0")), textures[0])
    height, width, fmt = b.u32(txob + 0x18), b.u32(txob + 0x1C), b.u32(txob + 0x34)
    image = b.rel(txob + 0x38)
    bits = b.raw[b.rel(image + 0x0C):]
    if fmt == ETC1:
        return decode(bits, width, height, alpha=False), name
    if fmt == ETC1A4:
        return decode(bits, width, height, alpha=True), name
    return None


def decode(data, width, height, alpha):
    """the image from its bytes: 8x8 tiles in rows, each four 4x4 blocks, left
    to right then top to bottom, each block 8 bytes of ETC1 read as a
    little-endian 64-bit word, after 8 bytes of 4-bit alpha for ETC1A4."""
    step = 16 if alpha else 8
    blocks = (width // 4) * (height // 4)
    raw = np.frombuffer(data, np.uint8, blocks * step).reshape(blocks, step)
    word = raw[:, step - 8:].copy().view("<u8")[:, 0]
    rgb = etc1(word)  # blocks x 16 pixels (x * 4 + y) x 3
    if alpha:
        a = raw[:, :8].copy().view("<u8")[:, 0]
        # 4 bits a pixel, pixel (x * 4 + y)
        shifts = 4 * np.arange(16, dtype=np.uint64)
        a_px = ((a[:, None] >> shifts) & np.uint64(0xF)).astype(np.uint8) * 17
        rgb = np.concatenate([rgb, a_px[:, :, None]], axis=2)
    channels = rgb.shape[2]
    # pixels (x * 4 + y) into 4x4 blocks
    pixels = rgb.reshape(blocks, 4, 4, channels).transpose(0, 2, 1, 3)  # block, y, x
    tiles = pixels.reshape(-1, 2, 2, 4, 4, channels)  # tile, block row, block column, y, x
    tiles = tiles.transpose(0, 1, 3, 2, 4, 5).reshape(-1, 8, 8, channels)
    columns = width // 8
    image = tiles.reshape(height // 8, columns, 8, 8, channels).transpose(0, 2, 1, 3, 4).reshape(height, width, channels)
    return image


def etc1(word):
    """ETC1 blocks, each a 64-bit word, as 16 RGB pixels each, pixel x * 4 + y."""
    w = word.astype(np.uint64)

    def field(shift, size):
        return ((w >> np.uint64(shift)) & np.uint64((1 << size) - 1)).astype(np.int64)

    differential, flip = field(33, 1).astype(bool), field(32, 1).astype(bool)
    base = np.zeros((len(w), 2, 3), np.int64)
    for c, (s5, s3, s4a, s4b) in enumerate(((59, 56, 60, 56), (51, 48, 52, 48), (43, 40, 44, 40))):
        first5, delta = field(s5, 5), field(s3, 3)
        delta = np.where(delta >= 4, delta - 8, delta)
        second5 = (first5 + delta) & 31
        diff_first, diff_second = (first5 << 3) | (first5 >> 2), (second5 << 3) | (second5 >> 2)
        ind_first, ind_second = field(s4a, 4) * 17, field(s4b, 4) * 17
        base[:, 0, c] = np.where(differential, diff_first, ind_first)
        base[:, 1, c] = np.where(differential, diff_second, ind_second)
    tables = np.stack([field(37, 3), field(34, 3)], axis=1)
    index = np.arange(16)
    x, y = index // 4, index % 4
    second = np.where(flip[:, None], y[None, :] >= 2, x[None, :] >= 2)  # blocks x 16
    msb = ((w[:, None] >> (np.uint64(16) + index.astype(np.uint64))) & np.uint64(1)).astype(np.int64)
    lsb = ((w[:, None] >> index.astype(np.uint64)) & np.uint64(1)).astype(np.int64)
    table = np.take_along_axis(tables, second.astype(np.int64), axis=1)
    small, large = MODIFIERS[table, 0], MODIFIERS[table, 1]
    selector = msb * 2 + lsb
    modifier = np.select([selector == 0, selector == 1, selector == 2], [small, large, -small], -large)
    colours = np.take_along_axis(base, second.astype(np.int64)[:, :, None].repeat(3, 2), axis=1)
    return np.clip(colours + modifier[:, :, None], 0, 255).astype(np.uint8)
