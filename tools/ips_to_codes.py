"""turns a code patch into cheat lines Zakuro's cheat engine runs, a 32-bit
write (type 0) for every aligned word the patched code.bin differs in.

    python tools/ips_to_codes.py <code.bin> <patched code.bin> [base]

base is where code.bin loads, 0x00100000 for the executable."""
import sys

original = open(sys.argv[1], "rb").read()
patched = open(sys.argv[2], "rb").read()
base = int(sys.argv[3], 16) if len(sys.argv) > 3 else 0x0010_0000
assert len(original) == len(patched), "the patch changes the code's size, which a cheat can't"
for offset in range(0, len(original), 4):
    old, new = original[offset:offset + 4], patched[offset:offset + 4]
    if old != new:
        address = base + offset
        assert address < 0x1000_0000, "a type 0 line holds 28 bits of address"
        print(f"{address:08X} {int.from_bytes(new, 'little'):08X}")
