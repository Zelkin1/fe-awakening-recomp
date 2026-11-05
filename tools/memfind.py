"""searches a memory dump from the boot harness's ZAKURO_DUMP_MEM for floats.

    python tools/memfind.py <dump> near <a> <b> [tolerance]   a float a with b within 8 words
    python tools/memfind.py <dump> value <a> [tolerance]       every float within tolerance of a
"""
import struct
import sys

import numpy as np


def regions(path):
    data = open(path, "rb").read()
    at = 0
    while at < len(data):
        base, size = struct.unpack_from("<II", data, at)
        at += 8
        yield base, np.frombuffer(data, dtype="<f4", count=size // 4, offset=at)
        at += size


def main():
    path, mode = sys.argv[1], sys.argv[2]
    if mode == "near":
        a, b = float(sys.argv[3]), float(sys.argv[4])
        tol = float(sys.argv[5]) if len(sys.argv) > 5 else 2e-4
    else:
        a, b = float(sys.argv[3]), None
        tol = float(sys.argv[4]) if len(sys.argv) > 4 else 2e-4
    for base, words in regions(path):
        with np.errstate(all="ignore"):
            f = np.where(np.isfinite(words), np.abs(words.astype(np.float64)), 0.0)
        hits = np.nonzero(np.abs(f - abs(a)) < tol)[0]
        partners = set(np.nonzero(np.abs(f - abs(b)) < tol)[0].tolist()) if b is not None else None
        for i in hits:
            if partners is not None and not any(j in partners for j in range(i - 8, i + 9)):
                continue
            row = " ".join(f"{x:.5f}" for x in words[max(i - 4, 0):i + 12])
            print(f"{base + i * 4:08X}: {words[i]:.6f} | {row}")


if __name__ == "__main__":
    main()
