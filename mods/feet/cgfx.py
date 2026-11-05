"""a reader for the CGFX files of Awakening's battle models: cgfx.py rom
list.txt path. reads the ROM, keeps nothing on disk.

CGFX pointers are relative to where they are stored. a dictionary (DICT) is
a patricia tree whose nodes name an entry and point at it."""
import struct
import sys
import importlib.util

spec = importlib.util.spec_from_file_location("peek", __file__.replace("cgfx.py", "peek_model.py"))


def load(rom, listing, want):
    for line in open(listing, encoding="utf-8"):
        parts = line.split()
        if len(parts) == 3 and parts[0] == want:
            size, at = int(parts[1]), int(parts[2].lstrip("@"), 16)
            break
    else:
        raise SystemExit(f"{want} not listed")
    sys_argv = sys.argv
    sys.argv = ["peek", rom, listing, want]
    peek = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(peek)
    sys.argv = sys_argv
    with open(rom, "rb") as f:
        f.seek(at)
        return peek.unpack(f.read(size))


class Blob:
    def __init__(self, raw):
        self.raw = raw

    def u32(self, at):
        return struct.unpack_from("<I", self.raw, at)[0]

    def i32(self, at):
        return struct.unpack_from("<i", self.raw, at)[0]

    def u16(self, at):
        return struct.unpack_from("<H", self.raw, at)[0]

    def f32(self, at, n=1):
        v = struct.unpack_from(f"<{n}f", self.raw, at)
        return v if n > 1 else v[0]

    def rel(self, at):
        """where a self-relative pointer stored at at points, none for 0."""
        off = self.i32(at)
        return at + off if off else None

    def magic(self, at):
        return self.raw[at:at + 4].decode("latin-1")

    def cstr(self, at):
        if at is None:
            return None
        end = self.raw.index(b"\x00", at)
        return self.raw[at:end].decode("latin-1")

    def dict(self, at):
        """a DICT's entries, (name, where its data is), in order."""
        if at is None:
            return []
        assert self.magic(at) == "DICT", (hex(at), self.magic(at))
        count = self.u32(at + 8)
        out = []
        # the root node, then count nodes of 16 bytes each
        for i in range(count):
            node = at + 0x0C + 0x10 * (i + 1)
            name = self.cstr(self.rel(node + 8))
            data = self.rel(node + 12)
            out.append((name, data))
        return out


DATA_KINDS = ["models", "textures", "luts", "materials", "shaders", "cameras", "lights", "fogs", "environments",
              "skeletal anims", "material anims", "visibility anims", "camera anims", "light anims", "fog anims", "emitters"]

if __name__ == "__main__":
    raw = load(sys.argv[1], sys.argv[2], sys.argv[3])
    b = Blob(raw)
    assert b.magic(0) == "CGFX"
    header = b.u16(6)
    print("CGFX header", hex(header), "revision", hex(b.u32(8)), "size", hex(b.u32(12)), "blocks", b.u32(16))
    data = header
    print("block", b.magic(data), "size", hex(b.u32(data + 4)))
    for i, kind in enumerate(DATA_KINDS):
        count = b.u32(data + 8 + 8 * i)
        where = b.rel(data + 12 + 8 * i)
        if count:
            entries = b.dict(where)
            print(f"  {kind}: {count}", [(name, hex(at), b.magic(at)) for name, at in entries])
