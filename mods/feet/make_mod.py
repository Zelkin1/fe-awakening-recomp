"""builds one body model from the player's ROM into a harness data folder's
mods, for the round trip test: unpacked, changed or not, and packed again as
LZ13. the file lands only in that throwaway folder.

    make_mod.py rom list.txt path data_dir [change]

change is none (byte for byte), or tips (the lowest vertices of each leg
pushed forward, to see an edit reach the screen)."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np

import cgfx
import cgfx_model as cm

rom, listing, want, data_dir = sys.argv[1:5]
change = sys.argv[5] if len(sys.argv) > 5 else "none"
raw = bytearray(cgfx.load(rom, listing, want))


from lz11 import lz13


if change == "tips":
    b = cgfx.Blob(bytes(raw))
    model, skeleton, shape = cm.model_parts(b)
    v = cm.vertices(b, shape)
    pos = v["position"]
    low = np.nonzero(pos[:, 1] < 0.3)[0]
    vb = b.rel(b.rel(shape + 0x3C))
    data, stride = b.rel(vb + 0x18), b.u32(vb + 0x24)
    fmt, count, scale, offset = v["attrs"][0]
    for i in low:
        at = data + i * stride + offset + 8  # z
        z = np.frombuffer(bytes(raw[at:at + 4]), "<f4")[0]
        raw[at:at + 4] = np.float32(z + 1.5).tobytes()
    print(f"pushed {len(low)} vertices under y 0.3 forward by 1.5")

target = os.path.join(data_dir, "mods", "000400000009F100", "feet", "romfs", *want.split("/"))
os.makedirs(os.path.dirname(target), exist_ok=True)
open(target, "wb").write(lz13(bytes(raw)))
print("wrote", target, len(raw), "bytes unpacked")
