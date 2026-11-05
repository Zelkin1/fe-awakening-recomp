"""executed instructions per frame and per second between two boot harness
logs of the same state, one stopped a frame in and one at the stretch's end.
    python tools/segment_rate.py <first log> <second log> <frames> <name>"""
import re
import sys


def read(path):
    text = open(path, encoding="utf-8", errors="replace").read()
    counted = int(re.search(r"instructions:\s+(\d+)", text).group(1))
    skipped = re.search(r"idle skipped:\s+(\d+)", text)
    value, unit = re.search(r"frames in ([\d.]+)(ms|s)\b", text).groups()
    return counted, int(skipped.group(1)) if skipped else 0, float(value) / (1000 if unit == "ms" else 1)


a, b = read(sys.argv[1]), read(sys.argv[2])
frames, name = int(sys.argv[3]), sys.argv[4]
counted, skipped, wall = b[0] - a[0], b[1] - a[1], b[2] - a[2]
executed = counted - skipped
print(f"{name:<8} per frame: {counted / frames / 1e6:5.2f}M counted, {skipped / frames / 1e6:5.2f}M idle-skipped, "
      f"{executed / frames / 1e6:5.2f}M executed | {wall / frames * 1000:5.1f} ms/frame | {executed / wall / 1e6:6.0f} MIPS executed")
