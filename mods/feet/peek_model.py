"""reads one RomFS file of the ROM by the offset romdump lists, undoes its LZ
compression and prints what it starts with and the names it holds. it keeps
nothing on disk but what it prints: peek_model.py rom list.txt path"""
import re
import struct
import sys

rom, listing, want = sys.argv[1], sys.argv[2], sys.argv[3]
for line in open(listing, encoding="utf-8"):
    parts = line.split()
    if len(parts) == 3 and parts[0] == want:
        size, at = int(parts[1]), int(parts[2].lstrip("@"), 16)
        break
else:
    sys.exit(f"{want} not listed")
with open(rom, "rb") as f:
    f.seek(at)
    data = f.read(size)


def lz11(src):
    kind = src[0]
    length = src[1] | src[2] << 8 | src[3] << 16
    at, out = 4, bytearray()
    if length == 0:
        length = struct.unpack_from("<I", src, 4)[0]
        at = 8
    while len(out) < length:
        flags = src[at]
        at += 1
        for bit in range(8):
            if len(out) >= length:
                break
            if not flags & (0x80 >> bit):
                out.append(src[at])
                at += 1
                continue
            if kind == 0x10:
                b0, b1 = src[at], src[at + 1]
                at += 2
                count, disp = (b0 >> 4) + 3, ((b0 & 0xF) << 8 | b1) + 1
            else:
                b0 = src[at]
                top = b0 >> 4
                if top == 0:
                    b1, b2 = src[at + 1], src[at + 2]
                    count = ((b0 & 0xF) << 4 | b1 >> 4) + 0x11
                    disp = ((b1 & 0xF) << 8 | b2) + 1
                    at += 3
                elif top == 1:
                    b1, b2, b3 = src[at + 1], src[at + 2], src[at + 3]
                    count = ((b0 & 0xF) << 12 | b1 << 4 | b2 >> 4) + 0x111
                    disp = ((b2 & 0xF) << 8 | b3) + 1
                    at += 4
                else:
                    b1 = src[at + 1]
                    count, disp = top + 1, ((b0 & 0xF) << 8 | b1) + 1
                    at += 2
            for _ in range(count):
                out.append(out[-disp])
    return bytes(out)


def unpack(data):
    # IntelligentSystems' LZ13 is a 4-byte header before an LZ11 stream
    if data[0] == 0x13:
        return lz11(data[4:])
    if data[0] in (0x10, 0x11):
        return lz11(data)
    return data


if __name__ == "__main__":
    print(f"{want}: {size} bytes packed, first byte 0x{data[0]:02X}")
    raw = unpack(data)
    print(f"unpacked {len(raw)} bytes, magic {raw[:4]!r}")
    # the names CGFX keeps, zero-ended identifiers, in their order
    seen = []
    for m in re.findall(rb"(?<=\x00)[A-Za-z_][A-Za-z0-9_.@-]{2,}(?=\x00)", raw):
        name = m.decode()
        if name not in seen:
            seen.append(name)
    print(len(seen), "names:")
    print(" ".join(seen))
