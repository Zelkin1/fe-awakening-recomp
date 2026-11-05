"""writes the decompressed code.bin of a decrypted .3ds/.cci (or .cxi), its
segments one after another the way the ExeFS has them, and optionally the
same with an IPS patch applied over it.

    python tools/extract_code.py <rom> <code.bin> [<patch.ips> <patched.bin>]

the decompression mirrors 3dsrecomp's rom::decompress."""
import struct
import sys

MEDIA_UNIT = 0x200


def u32(data, at):
    return struct.unpack_from("<I", data, at)[0]


def read_code(path):
    with open(path, "rb") as rom:
        head = rom.read(0x200)
        ncch = u32(head, 0x120) * MEDIA_UNIT if head[0x100:0x104] == b"NCSD" else 0
        rom.seek(ncch)
        header = rom.read(0x200)
        assert header[0x100:0x104] == b"NCCH", "no NCCH header"
        assert header[0x18F] & 4, "the executable partition is encrypted"
        exheader = rom.read(0x400)
        compressed = exheader[0xD] & 1
        exefs = ncch + u32(header, 0x1A0) * MEDIA_UNIT
        rom.seek(exefs)
        files = rom.read(0x200)
        for i in range(10):
            name = files[i * 16:i * 16 + 8].rstrip(b"\0")
            if name == b".code":
                offset, size = u32(files, i * 16 + 8), u32(files, i * 16 + 12)
                rom.seek(exefs + 0x200 + offset)
                code = rom.read(size)
                return decompress(code) if compressed else code
    raise SystemExit("no .code in the ExeFS")


def decompress(compressed):
    size = len(compressed)
    out_size = size + u32(compressed, size - 4)
    footer = u32(compressed, size - 8)
    footer_size, span = footer >> 24, footer & 0x00FFFFFF
    out = bytearray(out_size)
    out[:size] = compressed
    index, stop, cursor = size - footer_size, size - span, out_size
    while index > stop:
        index -= 1
        control = compressed[index]
        for _ in range(8):
            if index <= stop or index == 0 or cursor == 0:
                break
            if control & 0x80:
                index -= 2
                pair = compressed[index] | compressed[index + 1] << 8
                length, distance = (pair >> 12) + 3, (pair & 0xFFF) + 2
                for _ in range(length):
                    byte = out[cursor + distance]
                    cursor -= 1
                    out[cursor] = byte
            else:
                index -= 1
                cursor -= 1
                out[cursor] = compressed[index]
            control = (control << 1) & 0xFF
    return bytes(out)


def ips(code, patch):
    """the records of an IPS patch, (offset, bytes), and code with them."""
    assert patch[:5] == b"PATCH", "not an IPS patch"
    out, at, records = bytearray(code), 5, []
    while patch[at:at + 3] != b"EOF":
        offset = int.from_bytes(patch[at:at + 3], "big")
        size = int.from_bytes(patch[at + 3:at + 5], "big")
        at += 5
        if size == 0:
            run, value = int.from_bytes(patch[at:at + 2], "big"), patch[at + 2]
            data, at = bytes([value]) * run, at + 3
        else:
            data, at = patch[at:at + size], at + size
        if offset + len(data) > len(out):
            out.extend(bytes(offset + len(data) - len(out)))
        out[offset:offset + len(data)] = data
        records.append((offset, data))
    return records, bytes(out)


if __name__ == "__main__":
    code = read_code(sys.argv[1])
    open(sys.argv[2], "wb").write(code)
    print(f"code.bin: {len(code)} bytes")
    if len(sys.argv) > 4:
        records, patched = ips(code, open(sys.argv[3], "rb").read())
        open(sys.argv[4], "wb").write(patched)
        print(f"patched: {len(records)} records, {len(patched)} bytes")
