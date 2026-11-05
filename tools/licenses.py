"""writes the licenses of everything built into Zakuro's launcher, for a
package of the launcher: each crate with its version and license, then the
license and notice files the crates carry, each text once.

    python tools/licenses.py <cargo metadata json> <out.txt>

the json is `cargo metadata --format-version 1 --filter-platform
x86_64-pc-windows-gnu`, run in zakuro/."""
import json
import os
import sys

meta_path, out_path = sys.argv[1:3]
meta = json.load(open(meta_path, encoding="utf-8"))
packages = {p["id"]: p for p in meta["packages"]}
nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
root = next(i for i, p in packages.items() if p["name"] == "zakuro")

# what the launcher links: its dependencies, normal and build, not dev
linked, todo = set(), [root]
while todo:
    i = todo.pop()
    if i in linked:
        continue
    linked.add(i)
    for dep in nodes[i]["deps"]:
        if any(kind["kind"] in (None, "build") for kind in dep["dep_kinds"]):
            todo.append(dep["pkg"])

NAMES = ("license", "licence", "copying", "notice", "copyright", "unlicense")
texts = {}
rows = []
for i in sorted(linked, key=lambda i: (packages[i]["name"], packages[i]["version"])):
    p = packages[i]
    rows.append(f"{p['name']} {p['version']}: {p.get('license') or 'see its license file'}")
    folder = os.path.dirname(p["manifest_path"])
    # in the crate's folder and the folders right in it, where fonts and
    # bundled C code keep theirs
    files = []
    for sub in [""] + sorted(d for d in os.listdir(folder) if os.path.isdir(os.path.join(folder, d)) and d not in ("target", ".git")):
        here = os.path.join(folder, sub)
        for f in sorted(os.listdir(here)):
            name = f.lower()
            # a font's license is the text file beside it, OFL.txt say
            font = sub.lower() == "fonts" and name.endswith(".txt")
            if (name.startswith(NAMES) or "license" in name or font) and os.path.isfile(os.path.join(here, f)):
                files.append(os.path.join(sub, f))
    if p.get("license_file"):
        files.append(p["license_file"])
    for name in dict.fromkeys(files):
        try:
            text = open(os.path.join(folder, name), encoding="utf-8", errors="replace").read().strip()
        except OSError:
            continue
        texts.setdefault(text, []).append(f"{p['name']} {p['version']}")

with open(out_path, "w", encoding="utf-8", newline="\n") as out:
    out.write("The launcher is built from these crates, under these licenses:\n\n")
    out.write("\n".join(rows))
    out.write("\n\nTheir license and notice files follow, each once, with the crates that carry it.\n")
    for text, owners in texts.items():
        out.write("\n" + "=" * 78 + "\n")
        out.write(", ".join(owners) + "\n" + "-" * 78 + "\n")
        out.write(text + "\n")
print(f"{len(rows)} crates, {len(texts)} license texts")
