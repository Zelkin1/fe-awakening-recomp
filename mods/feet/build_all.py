"""gives every body model in the player's ROM feet, with foot_build.py, into
one data folder's mods, several at a time, with a preview of each from the
front and the left leg's outer side and a contact sheet for each class. a body
the builder can't give feet is listed with why. writes only into the two
folders given.

    build_all.py rom list.txt data_dir sheet_dir [jobs] [body ...]"""
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

from PIL import Image, ImageDraw

here = os.path.dirname(os.path.abspath(__file__))
rom, listing, data_dir, sheet_dir = sys.argv[1:5]
jobs = int(sys.argv[5]) if len(sys.argv) > 5 else 8
chosen = sys.argv[6:]
bodies = sorted(set(re.findall(r"b/(uBody_[^ ]*)\.m\.lz", open(listing, encoding="utf-8").read())))
if chosen:
    bodies = [body for body in bodies if body.removeprefix("uBody_") in chosen]
os.makedirs(sheet_dir, exist_ok=True)


def build(body):
    preview = os.path.join(sheet_dir, f"{body}.png")
    done = subprocess.run([sys.executable, os.path.join(here, "foot_build.py"), rom, listing, f"b/{body}.m.lz", data_dir,
                           f"preview={preview}", "views=front,left"], capture_output=True, text=True)
    lines = (done.stdout + done.stderr).strip().splitlines()
    style = next((line.split(": a ")[-1] for line in lines if ": a " in line), "")
    return body, done.returncode == 0, style, lines[-1] if lines else ""


with ThreadPoolExecutor(jobs) as pool:
    results = list(pool.map(build, bodies))
with open(os.path.join(sheet_dir, "results.txt"), "w", encoding="utf-8") as f:
    for body, ok, style, last in results:
        f.write(f"{body}\t{'feet' if ok else 'none'}\t{style}\t{last}\n")
made = [(body, style) for body, ok, style, _ in results if ok]
print(f"{len(made)} of {len(results)} bodies have feet")
for body, ok, style, last in results:
    if not ok:
        print(f"  {body}: {last}")

# a sheet for each class, its bodies side by side, so that they can be
# compared: sheet-<class>.png, the class being the code in the bodies' names
classes = {}
for body, style in made:
    classes.setdefault(body.split("_")[2], []).append((body, style))
per_row = 3
for code, group in sorted(classes.items()):
    pictures = [Image.open(os.path.join(sheet_dir, f"{body}.png")) for body, _ in group]
    w, h = pictures[0].width // 2, pictures[0].height // 2
    across = min(per_row, len(group))
    sheet = Image.new("RGB", (across * w, ((len(group) + per_row - 1) // per_row) * (h + 16)), (255, 255, 255))
    draw = ImageDraw.Draw(sheet)
    for k, ((body, style), picture) in enumerate(zip(group, pictures)):
        x, y = (k % per_row) * w, (k // per_row) * (h + 16)
        sheet.paste(picture.resize((w, h), Image.LANCZOS), (x, y))
        draw.text((x + 4, y + h + 2), f"{body.removeprefix('uBody_')}: {style}", fill=(0, 0, 0))
    sheet.save(os.path.join(sheet_dir, f"sheet-{code}.png"))

# and pages to look at them on, six classes each, the pictures in the pages
# themselves, so that a viewer showing a page alone shows them: feet-<n>.html
import base64
import html
import io

per_page = 6
codes = sorted(classes)
for page in range(0, len(codes), per_page):
    parts = ['<!doctype html><meta charset="utf-8"><title>Feet by class</title>',
             '<style>body{font-family:sans-serif;margin:16px;background:#fff;color:#222}'
             'h2{font-size:15px;font-weight:500;margin:20px 0 6px}img{max-width:100%;border:1px solid #ddd}</style>',
             f'<p>Classes {page + 1} to {min(page + per_page, len(codes))} of {len(codes)}, each body from the front and '
             'the left leg\'s outer side, in the bind pose.</p>']
    for code in codes[page:page + per_page]:
        buffer = io.BytesIO()
        Image.open(os.path.join(sheet_dir, f"sheet-{code}.png")).convert("RGB").save(buffer, "JPEG", quality=82)
        data = base64.b64encode(buffer.getvalue()).decode()
        parts.append(f'<h2>{html.escape(code)}</h2><img src="data:image/jpeg;base64,{data}" alt="{html.escape(code)}">')
    with open(os.path.join(sheet_dir, f"feet-{page // per_page + 1}.html"), "w", encoding="utf-8") as f:
        f.write("\n".join(parts))
