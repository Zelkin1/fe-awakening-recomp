"""gives a body model feet of our own: a foot shape lofted from cross-sections
in front of each leg's end, fitted to the leg's shaft, and written into the
model's CGFX as a primitive set of its own, skinned to two bones added to the
skeleton for it, a foot bone under each shin: the vertices appended to the
shape's vertex buffer, the triangles, the set, the bones and the skeleton's
dictionary with them at the end of the file. each foot bone is marked for
Zakuro's feet hook (zakuro-core feet.rs), which turns it every frame so that
the foot meets the floor: its flags have the game compute it a skinning
matrix, and its inverse bind matrix, which only that would read, holds the
foot's points instead, the ankle it turns about, the toe and the heel, then
"FEET". reads the model from the player's ROM, writes the mod only into the
folder given, as the feet mod's own, <data_dir>/mods/000400000009F100/feet/.

    foot_build.py rom list.txt path data_dir [texture=body.png]
                  [pose=uniforms.txt] [preview=views.png] [views=front,left,...]
                  [upto=1.1] [style=shoe|sabaton|sandal]

the feet take the plainest rectangles of the shins' own texels, at the shins'
texel density, from the body's colour texture, which is read from the ROM
(textures.py); texture names a picture of it instead, as Zakuro dumps it
(ZAKURO_DUMP_TEXTURES), and without either the feet take the nearest boot
vertex's coordinates. preview renders the legs' ends and the feet; with pose,
a ZAKURO_DUMP_UNIFORMS file of a draw of the sub-mesh with the shins, posed as
that draw was and the feet turned as the hook turns them, over the floor."""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np

import cgfx
import cgfx_dict
import cgfx_model as cm
import textures
from lz11 import lz13

rom, listing, want, data_dir = sys.argv[1:5]
options = dict(arg.split("=", 1) for arg in sys.argv[5:])
raw = bytearray(cgfx.load(rom, listing, want))
b = cgfx.Blob(bytes(raw))
model, skeleton, shape = cm.model_parts(b)
names, records = {}, {}
for name, at in b.dict(b.rel(skeleton + 0x1C)):
    names[b.i32(at + 8)] = name
    records[name] = at
v = cm.vertices(b, shape)
subs = cm.submeshes(b, shape)
pos, uv, local, weights = v["position"], v["uv"], v["bones"], v["weights"]
if local is None or weights is None:
    raise SystemExit(f"{want}: its vertices are each one bone's, which this builder doesn't take")
joint = np.full(local.shape, -1)
for sm in subs:
    used = np.unique(sm["triangles"])
    joint[used] = np.array(sm["palette"])[local[used]]
main = joint[np.arange(len(joint)), weights.argmax(1)]

# the shins, and the sub-mesh with both of them, if one has, which a pose's
# matrices are for
shins = {leg: next((j for j, n in names.items() if n == f"{leg}_leg2"), None) for leg in "lr"}
if None in shins.values():
    raise SystemExit(f"{want}: no l_leg2 and r_leg2, the shins the feet hang on")
target = next((i for i, sm in enumerate(subs) if all(j in sm["palette"] for j in shins.values())), None)
sm_info = subs[target] if target is not None else None
# how far behind the leg's back the heel goes, as a share of the foot's length,
# a real heel's, and the foot's length: Robin's for the depth of his shaft,
# within shares of the knee's height, so that a thin leg doesn't get a doll's
# foot nor a thick greave a clown's
HEEL = 0.03
LENGTH_PER_DEPTH = 2.35
LENGTH = (0.42, 0.60)
AHEAD_LEAST = 0.40
# the shaft a foot is fitted to, as shares of the knee's height above the
# floor, where Robin's boots are straight; how much of a vertex's weight is
# the shin's for it to be the leg's own, which leaves out robes, whose
# vertices go with several bones
BAND = (0.234, 0.338)
# below this share of the knee's height a leg's vertices are its end, whose
# footprint tells the leg from a robe's hem hanging on the shin
END = 0.12
# above this share of the ankle's height the leg shows above the shoe
SHOWN = 0.85
OWN = 0.85
# the widest a foot's shaft is taken to be, as a share of the knee's height;
# Robin's is 0.19
WIDEST = 0.21
# how many texels each of a foot's vertices is nudged by at most, a noise
# that mottles the leg's grain across the foot
NOISE = 1.5


def knee(leg):
    """how high the shin's bone, at the knee, is in the bind pose."""
    m = np.array(b.f32(records[f"{leg}_leg2"] + 0xA4, 12)).reshape(3, 4)
    return float(-(m[:, :3].T @ m[:, 3])[1])


def frame(leg):
    """the leg's own vertices, its outline at the shaft's bottom, the centre
    a foot goes under, its half-width, the shaft's depth, the ankle's height,
    the shaft's bottom a little under it, the knee's, and how far back and
    forward the leg reaches there. the shaft is where the leg's surface
    crosses heights through BAND, or, under a robe, between its end and BAND;
    its extent is the median of those heights' own, so that a fin or a spike
    at one height doesn't count, and the centre is the middle of it."""
    bones = [j for j, n in names.items() if n in (f"{leg}_leg2", f"{leg}_leg3")]
    own = np.zeros(len(pos))
    for k in range(local.shape[1]):
        own += np.where(np.isin(joint[:, k], bones), weights[:, k], 0)
    on_leg = np.isin(main, bones) & (own >= OWN * weights.sum(1))
    height = knee(leg)
    lo, hi = BAND[0] * height, BAND[1] * height
    # and near the leg's end: within its footprint on the floor grown by half
    # its size each way, which leaves out a robe's hem hanging on the shin
    end = on_leg & (pos[:, 1] < END * height)
    if end.sum() >= 3:
        first, last = pos[end].min(0), pos[end].max(0)
        grown = (last - first) / 2
        near = ((pos[:, 0] >= first[0] - grown[0]) & (pos[:, 0] <= last[0] + grown[0])
                & (pos[:, 2] >= first[2] - grown[2]) & (pos[:, 2] <= last[2] + grown[2]))
        on_leg &= near
    p = pos[on_leg]
    tris = np.vstack([sm["triangles"] for sm in subs])
    tris = tris[on_leg[tris].all(1)]

    def crossings(y):
        found = []
        for i, j in ((0, 1), (1, 2), (2, 0)):
            a, c = pos[tris[:, i]], pos[tris[:, j]]
            cut = (a[:, 1] - y) * (c[:, 1] - y) < 0
            t = (y - a[cut, 1]) / (c[cut, 1] - a[cut, 1])
            found.append(a[cut] + t[:, None] * (c[cut] - a[cut]))
        return np.vstack(found)

    def sections(bottom, top):
        cuts = [crossings(y) for y in np.linspace(bottom, top, 7)]
        return [cut for cut in cuts if len(cut) >= 4]

    cuts, robed = sections(lo, hi), False
    if len(cuts) < 2:
        # nothing of the leg there, under a robe say: what of it there is
        # between its end and the band
        cuts, robed = sections(END * height, lo), True
    if len(cuts) < 2:
        raise SystemExit(f"{want}: the {leg} shin has no shaft between {END * height:.2f} and {hi:.2f}")
    first = np.median([cut.min(0) for cut in cuts], axis=0)
    last = np.median([cut.max(0) for cut in cuts], axis=0)
    cx, cz = (first[0] + last[0]) / 2, (first[2] + last[2]) / 2
    half, depth = (last[0] - first[0]) / 2, last[2] - first[2]
    # the back and the front are the shaft's, or the leg's under it where that
    # shows above the shoe, a boot's heel or a greave's flare say; lower, the
    # leg goes inside the shoe
    heights = [cut[0, 1] for cut in cuts]
    ankle = 0.9 * (max(heights) if robed else min(heights))
    shown = p[(p[:, 1] > SHOWN * ankle) & (p[:, 1] < hi)]
    back = min(first[2], np.percentile(shown[:, 2], 5)) if len(shown) >= 4 else first[2]
    front = max(last[2], np.percentile(shown[:, 2], 95)) if len(shown) >= 4 else last[2]
    # a massive greave doesn't make a wide foot: no wider than WIDEST of the
    # knee's height across
    half = min(half * 0.95, WIDEST * height / 2)
    return on_leg, cuts[0], cx, cz, half, depth, ankle, height, back, front

# the styles of foot, each fitting the legs it goes on: a shoe for boots of
# leather or cloth, a sabaton for plate, wider, its top in steps like plates
# over a thin sole, with harder edges, and a sandal for bare and strapped
# legs, a low bare foot on a thin sole. each is its cross-sections from heel
# to toe, few, as few as the game's own boots have, in two runs: behind the
# leg's front, where along the foot as a share of its length from the heel,
# and ahead of it, as a share of the way from the leg's front to the toe, so
# that the instep is always where the leg comes down into the foot. for each,
# the half-width as a share of the shaft's, as shares of the ankle's height
# the top of the upper, the top of the sole and its bottom, which rises at
# the toe, and where the upper's sides turn to its top, as a share of the
# half-width. the heel stands a little lower than the instep, the top drops
# from the instep to the ball and flattens over the toes, and the toe ends
# blunt, narrower and lower. then the angle in degrees under which faces
# share their normals, so that the foot is lit smoothly between its edges and
# its edges stay sharp, and the stretches between cross-sections, from the
# heel whose upper is in the trim's colour, none: the foot is one material,
# its detail a noise in where its texels come from, see NOISE
STYLES = {
    "shoe": ([(0.00, 0.86, 0.56, 0.18, 0.00, 0.85), (0.14, 1.00, 0.68, 0.16, 0.00, 0.85)],
             [(0.00, 1.04, 0.70, 0.14, 0.00, 0.78), (0.42, 1.06, 0.49, 0.13, 0.00, 0.66),
              (0.78, 0.98, 0.39, 0.12, 0.01, 0.62), (1.00, 0.76, 0.31, 0.12, 0.05, 0.62)], 30, set()),
    "sabaton": ([(0.00, 0.88, 0.60, 0.07, 0.00, 0.82), (0.14, 1.02, 0.74, 0.07, 0.00, 0.80)],
                [(0.00, 1.06, 0.78, 0.07, 0.00, 0.72), (0.40, 1.02, 0.54, 0.07, 0.00, 0.62),
                 (0.76, 0.86, 0.42, 0.07, 0.01, 0.56), (1.00, 0.58, 0.32, 0.07, 0.05, 0.56)], 20, set()),
    "sandal": ([(0.00, 0.80, 0.48, 0.06, 0.00, 0.82), (0.14, 0.95, 0.62, 0.06, 0.00, 0.80)],
               [(0.00, 1.00, 0.66, 0.06, 0.00, 0.70), (0.45, 0.98, 0.42, 0.06, 0.00, 0.62),
                (0.80, 0.86, 0.33, 0.06, 0.01, 0.62), (1.00, 0.64, 0.27, 0.06, 0.03, 0.64)], 30, set()),
}
# each class's style, by the code in its bodies' names, uBody_<sex>_<class>_<n>:
# plate for the armoured, bare feet for those whose legs end bare or strapped
CLASS_STYLES = {
    **{c: "sabaton" for c in ("karm", "gene", "kgre", "kgri", "kpal", "kdar", "kdr1", "kdr2", "lod2", "snyp", "war2", "ksoc", "merc")},
    **{c: "sandal" for c in ("dmag", "sorc", "lagz", "baba", "swd2")},
}
style = options.get("style") or CLASS_STYLES.get(want.split("_")[2], "shoe")
if style not in STYLES:
    raise SystemExit(f"{style}: no such style, there are {', '.join(STYLES)}")
BEHIND, AHEAD, SMOOTH, TRIMMED = STYLES[style]
SECTIONS = BEHIND + AHEAD
print(f"{want}: a {style}")
# what a face is made of: the dark sole, the leather upper or its trim, and
# which edges of a cross-section, from each point to the next, are the sole's
SOLE, UPPER, TRIM = 0, 1, 2
SOLE_EDGES = {0, 1, 5}
# how far above the floor a sole stands where it meets it, so that the
# floor doesn't flicker through it
LIFT = 0.01
# the bone flags the game computes a skinning matrix for, the inverse bind
# matrix in a bone's record, and the tag after the foot's points there
SKINNED, INVERSE_BIND, TAG = 0x240, 0xA4, b"FEET"
# the first float uniform of the bone matrices in Awakening's skinning
# shader, three rows a palette slot, the inverse bind folded in
POSE_BASE = 25


def pose_of(path):
    """the bone matrices of the last draw the file has, a 3x4 a slot."""
    if sm_info is None:
        raise SystemExit(f"{want}: no sub-mesh has both shins, so a pose can't be read")
    block = open(path).read().split("draw ")[-1].splitlines()[1:]
    u = np.array([[float(x) for x in line.split()[1:]] for line in block])
    return [u[POSE_BASE + 3 * k:POSE_BASE + 3 * k + 3] for k in range(len(sm_info["palette"]))]


def plain(tex, mask, sizes, pick):
    """the rectangle of the shins' own texels, a texel to spare all round for
    the filtering, that pick chooses from the candidates of the first size
    that has any: pick gets (x, y, mean colour, spread of brightness) of
    each and gives back the corner it wants. its corner and size."""
    lum = tex @ [0.299, 0.587, 0.114]
    rows, columns = np.nonzero(mask)
    for w, h in sizes:
        found = []
        for y in range(max(rows.min(), 1), rows.max() - h):
            for x in range(max(columns.min(), 1), columns.max() - w):
                if mask[y - 1:y + h + 1, x - 1:x + w + 1].all():
                    found.append((x, y, tex[y:y + h, x:x + w].reshape(-1, 3).mean(0), lum[y:y + h, x:x + w].std()))
        corner = pick(found, (w, h))
        if corner is not None:
            return (*corner, w, h)
    raise SystemExit("no plain rectangle on the shins")


def materials(tex):
    """where the upper and the sole take their texels in the texture tex, in
    the leg's own materials: the foot carries on the leg's faces right above
    it, the ones seen where they meet. for a shoe or a sabaton the upper is the
    plainest rectangle of the shins' texels near those faces' usual colour
    and the sole the darkest even one; for a sandal the foot is the plainest
    rectangle of the shins' skin, when they show some, and the sole the
    lowest faces' colour. with the texture's size and the shins' texels per
    unit."""
    from PIL import Image, ImageDraw
    tex = tex[:, :, :3].astype(float)
    high, wide = tex.shape[:2]
    on_shins = np.isin(main, [j for j, n in names.items() if n in ("l_leg2", "l_leg3", "r_leg2", "r_leg3")])
    tris = np.vstack([sm["triangles"] for sm in subs])
    tris = tris[on_shins[tris].all(1)]
    p, t = pos[tris], uv[tris] * [wide, high]
    area3 = np.linalg.norm(np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0]), axis=1).sum()
    e1, e2 = t[:, 1] - t[:, 0], t[:, 2] - t[:, 0]
    area2 = np.abs(e1[:, 0] * e2[:, 1] - e1[:, 1] * e2[:, 0]).sum()
    density = np.sqrt(area2 / area3)

    def texels(chosen):
        mask = Image.new("L", (wide, high), 0)
        draw = ImageDraw.Draw(mask)
        for tri in chosen:
            draw.polygon([(uv[i, 0] * wide, (1 - uv[i, 1]) * high) for i in tri], fill=255)
        return np.asarray(mask) > 0

    mask = texels(tris)
    ankle = min(frame(leg)[6] for leg in "lr")
    # the faces right above the foot, the ones seen where leg and foot meet
    middle = pos[tris][:, :, 1].mean(1)
    lowest = texels(tris[(middle > 0.35 * ankle) & (middle < 1.1 * ankle)])
    if not lowest.any():
        lowest = texels(tris[pos[tris][:, :, 1].min(1) < ankle])
    usual = np.median(tex[lowest if lowest.any() else mask], 0)

    def near(colour):
        def pick(found, size):
            close = [(spread, x, y) for x, y, c, spread in found if np.abs(c - colour).max() < 16]
            if not close:
                # none near enough: the nearest even one, evenness counting
                # thrice, so that a stretch across a pattern isn't taken
                close = [(np.abs(c - colour).max() + 3 * spread, x, y) for x, y, c, spread in found]
            return min(close)[1:] if close else None
        return pick

    def even_near(colour):
        # the even stretch nearest the colour, so that a trim is a colour
        # of its own and not a stretch across a pattern; none of this size
        # when there is no even one
        def pick(found, size):
            even = [(np.abs(c - colour).max(), x, y) for x, y, c, spread in found if spread < 5]
            return min(even)[1:] if even else None
        return pick

    def trim(colour, sizes, upper):
        # an even stretch of any size down to 8x4, else the nearest; the
        # upper itself when that is too far from it to read as the same
        # material
        try:
            found = plain(tex, mask, sizes + ((12, 6), (8, 4)), even_near(colour))
        except SystemExit:
            found = plain(tex, mask, sizes, near(colour))
        x, y, w, h = found
        return found if np.abs(tex[y:y + h, x:x + w].reshape(-1, 3).mean(0) - colour).max() < 28 else upper

    def stripe(x, y, size):
        # how far the row or the column of a stretch that stands out most
        # is from the stretch's own colour, in any channel, so that a purple
        # stripe on brown leather of the same brightness counts too
        w, h = size
        block = tex[y:y + h, x:x + w]
        whole = block.reshape(-1, 3).mean(0)
        return max(np.abs(block.mean(0) - whole).max(), np.abs(block.mean(1) - whole).max())

    def grained(colour):
        # of those near the colour, the most grained of those little more
        # grained than the flattest and with no row or column standing out,
        # which on the foot would be a stripe along it; none of this size
        # when no such one is, so that a smaller size is tried
        def pick(found, size):
            close = sorted((spread, x, y) for x, y, c, spread in found if np.abs(c - colour).max() < 16)
            mild = [entry for entry in close if entry[0] <= close[0][0] + 2.5 and stripe(entry[1], entry[2], size) < 4]
            return mild[-1][1:] if mild else None
        return pick

    def upper(colour, sizes):
        # a grained stretch without a stripe, of any size down to 8x4, else
        # the nearest
        try:
            return plain(tex, mask, sizes + ((12, 6), (8, 4)), grained(colour))
        except SystemExit:
            return plain(tex, mask, sizes, near(colour))

    def darkest(found, size):
        even = [(c.mean(), x, y) for x, y, c, spread in found if spread < 3]
        return min(even)[1:] if even else None

    def skin(found, size):
        # orange, from fair to dark, and even, the nearest to the leg's own
        # colour where it meets the foot
        bare = [(np.abs(c - usual).max() + spread, x, y) for x, y, c, spread in found
                if c[0] >= c[1] >= c[2] and 0.12 < (c[0] - c[2]) / (c[0] + 1) < 0.55 and c[0] > 70 and spread < 6]
        return min(bare)[1:] if bare else None

    large, small = ((32, 14), (28, 12), (24, 12), (20, 10), (16, 8)), ((24, 10), (16, 8), (12, 6))
    # a bare foot when the leg's lowest faces are mostly skin
    low_texels = tex[lowest] if lowest.any() else tex[mask]
    r, g, b_ = low_texels[:, 0], low_texels[:, 1], low_texels[:, 2]
    bare = ((r >= g) & (g >= b_) & ((r - b_) / (r + 1) > 0.12) & ((r - b_) / (r + 1) < 0.55) & (r > 70)).mean()
    if style == "sandal":
        try:
            foot_rect = plain(tex, mask, large + ((12, 6), (8, 4)), skin) if bare >= 0.3 else upper(usual, large)
        except SystemExit:
            foot_rect = upper(usual, large)
        # the strap in the colour of what the leg wears that isn't skin
        r, g, b_ = tex[mask][:, 0], tex[mask][:, 1], tex[mask][:, 2]
        worn = ~((r >= g) & (g >= b_) & ((r - b_) / (r + 1) > 0.12) & ((r - b_) / (r + 1) < 0.55) & (r > 70))
        strap = np.median(tex[mask][worn], 0) if worn.sum() > 20 else usual * 0.6
        rects = {UPPER: foot_rect, SOLE: plain(tex, mask, small, near(usual)), TRIM: trim(strap, small, foot_rect)}
    else:
        # the trim a shade darker than the upper, or lighter when that is
        # nearly black already, near enough to read as the same material
        lum = usual @ [0.299, 0.587, 0.114]
        shade = usual * 0.8 if lum >= 50 else np.minimum(usual * 1.25 + 4, 255)
        upper_rect = upper(usual, large)
        rects = {UPPER: upper_rect, SOLE: plain(tex, mask, small, darkest), TRIM: trim(shade, large, upper_rect)}
    for name, (x, y, w, h) in (("upper", rects[UPPER]), ("sole", rects[SOLE]), ("trim", rects[TRIM])):
        print(f"{name}: {w}x{h} at {x},{y}, colour {np.round(tex[y:y + h, x:x + w].reshape(-1, 3).mean(0))}")
    print(f"of {wide}x{high}, {density:.1f} texels a unit, the lowest faces' colour {np.round(usual)}, {bare:.0%} of them skin")
    return rects, (wide, high), density


if "texture" in options:
    from PIL import Image
    texture = np.asarray(Image.open(options["texture"]).convert("RGB"))
else:
    found = textures.colour_texture(rom, listing, want)
    texture = found[0] if found else None
patch = materials(texture) if texture is not None else None


def outline(cx, half, top, sole, bottom, shoulder):
    """a cross-section in x and y: the sole's bottom and its two sides, then
    the upper's sides in to its shoulders and its top."""
    return [(cx - half, bottom), (cx + half, bottom), (cx + half, sole),
            (cx + shoulder * half, top), (cx - shoulder * half, top), (cx - half, sole)]


def foot(leg):
    on_leg, _, cx, cz, half, depth, ankle, height, back, front = frame(leg)
    # the foot's length from the leg's depth, its heel a little behind the
    # leg's back, so that the leg comes down at the back of the foot as on a
    # real one, the instep at the leg's front, and the rest ahead of it, at
    # least AHEAD_LEAST of its length, however deep the leg
    length = float(np.clip(LENGTH_PER_DEPTH * depth, LENGTH[0] * height, LENGTH[1] * height))
    heel = back - cz - HEEL * length
    front = max(front - cz, heel + 0.2 * length)
    toe = max(heel + length, front + AHEAD_LEAST * length)
    along = [heel + z * length for z, *_ in BEHIND] + [front + z * (toe - front) for z, *_ in AHEAD]
    print(f"the {leg} foot {toe - heel:.2f} long, its heel {front - heel:.2f} behind the leg's front, "
          f"under a leg {depth:.2f} deep and {2 * half / 0.95:.2f} wide")
    rings = [[(x, y, cz + z) for x, y in outline(cx, half * w, top * ankle, sole * ankle, bottom * ankle, shoulder)]
             for z, (_, w, top, sole, bottom, shoulder) in zip(along, SECTIONS)]
    n = len(rings[0])
    points = np.array([point for ring in rings for point in ring])
    # where the hook turns the foot about, and the bottoms of the heel and
    # the toe, the points that meet the floor
    (heel_z, _, _, _, heel_y, _), (toe_z, _, _, _, toe_y, _) = SECTIONS[0], SECTIONS[-1]
    contact = [np.array([cx, ankle, cz]), np.array([cx, toe_y * ankle - LIFT, cz + along[-1]]),
               np.array([cx, heel_y * ankle - LIFT, cz + along[0]])]
    # the faces, each with what it's made of and which way is out of the
    # shoe: away from the middle of the two cross-sections a side joins, back
    # at the heel and forward at the toe
    faces = []
    upper = lambda r: TRIM if r in TRIMMED else UPPER
    for r in range(len(rings) - 1):
        middle = points[r * n:(r + 2) * n].mean(0)
        for k in range(n):
            a, b2 = r * n + k, r * n + (k + 1) % n
            faces.append(((a, b2, b2 + n, a + n), SOLE if k in SOLE_EDGES else upper(r), lambda c, m=middle: c - m))
    for base, out, r in ((0, -1.0, 0), ((len(rings) - 1) * n, 1.0, len(rings) - 2)):
        for quad, material in (((0, 1, 2, 5), SOLE), ((5, 2, 3, 4), upper(r))):
            faces.append((tuple(base + i for i in quad), material, lambda c, o=out: np.array([0.0, 0.0, o])))
    tris, made_of, normal = [], [], []
    for (a, b2, c, d), material, out in faces:
        for tri in ((a, b2, c), (a, c, d)):
            n_ = np.cross(points[tri[1]] - points[tri[0]], points[tri[2]] - points[tri[0]])
            if np.dot(n_, out(points[list(tri)].mean(0))) < 0:
                tri, n_ = (tri[0], tri[2], tri[1]), -n_
            tris.append(tri)
            made_of.append(material)
            normal.append(n_ / np.linalg.norm(n_))
    normal = np.array(normal)
    # a vertex for each point, material and normal: a corner's normal is the
    # mean of those of its point's faces that meet its own face at less than
    # SMOOTH degrees
    least = np.cos(np.radians(SMOOTH))
    verts, normals, materials, corners, made = [], [], [], [], {}
    for t, tri in enumerate(tris):
        new = []
        for point in tri:
            around = [g for g, other in enumerate(tris) if point in other and normal[g] @ normal[t] >= least]
            n_ = normal[around].sum(0)
            n_ /= np.linalg.norm(n_)
            key = (point, made_of[t], tuple(np.round(n_, 4)))
            if key not in made:
                made[key] = len(verts)
                verts.append(points[point])
                normals.append(n_)
                materials.append(made_of[t])
            new.append(made[key])
        corners.append(new)
    verts, normals, materials, tris = np.array(verts), np.array(normals), np.array(materials), np.array(corners)
    if patch is not None:
        # each material's rectangle laid on the foot from the way each
        # vertex faces: from above on the top and the sole, from the side on
        # the sides, from the front on the heel and the toe, at the shins'
        # density or tighter if it doesn't fit, so that the leg's grain goes
        # on across the foot. across the foot the texels go outward on both
        # feet, so that the two are mirror images
        rects, (wide, high), density = patch
        outward = np.array([1.0 if cx >= 0 else -1.0, 1.0, 1.0])
        length, width = along[-1] - along[0], 2 * half * max(s[1] for s in SECTIONS)
        tall = ankle * max(s[2] for s in SECTIONS)
        middle = np.array([cx, tall / 2, cz + (along[-1] + along[0]) / 2])
        facing = np.abs(normals).argmax(1)
        jitter = np.random.default_rng(7).uniform(-NOISE, NOISE, (len(verts), 2))
        # the axes along the rectangle's length and across it, by which way
        # a vertex faces: x, y or z
        axes = {0: (2, 1), 1: (2, 0), 2: (0, 1)}
        uvs = np.zeros((len(verts), 2))
        for material, (x0, y0, w, h) in rects.items():
            scale = min(density, (w - 1) / max(length, width), (h - 1) / max(width, tall))
            for way, (u_axis, v_axis) in axes.items():
                these = (materials == material) & (facing == way)
                tx = x0 + w / 2 + (verts[these, u_axis] - middle[u_axis]) * outward[u_axis] * scale
                ty = y0 + h / 2 + (verts[these, v_axis] - middle[v_axis]) * outward[v_axis] * scale
                # a little noise, each vertex nudged up to NOISE texels, the
                # same on both feet so that they stay mirror images, which
                # mottles the leg's grain across the foot
                tx = np.clip(tx + jitter[these, 0], x0 + 1, x0 + w - 1)
                ty = np.clip(ty + jitter[these, 1], y0 + 1, y0 + h - 1)
                uvs[these] = np.column_stack([tx / wide, 1 - ty / high])
    else:
        # the nearest boot vertex's texture coordinates
        boot = np.nonzero(on_leg & (pos[:, 1] < 1.4))[0]
        nearest = boot[np.argmin(((verts[:, None, :] - pos[boot][None, :, :]) ** 2).sum(-1), axis=1)]
        uvs = uv[nearest]
    fit = (cx, cz, along, [(half * w, top * ankle, sole * ankle, bottom * ankle, shoulder)
                           for _, w, top, sole, bottom, shoulder in SECTIONS])
    return verts, normals, uvs, tris, contact, fit


feet = {leg: foot(leg) for leg in "lr"}

# the boot's own end, below LOW of the ankle's height, made a straight boot
# inside the foot: each of its points goes to the leg's outline at the
# shaft's bottom, in the direction it lies in from the middle of the end, so
# that a wedge pointing forward comes back under the leg, its sides under the
# back of the leg, and the back goes straight down into the heel, while a leg
# already straight stays so; there it goes a little in, and up to TUCK of the
# ankle's height, inside the foot. the points moved take the mean of their
# faces' new normals
LOW, TUCK = 0.25, 0.40
moved = {}
for leg in "lr":
    on_leg, outline, cx, cz, half, depth, ankle, height, _, _ = frame(leg)
    low = np.nonzero(on_leg & (pos[:, 1] < LOW * ankle))[0]
    if not len(low):
        continue
    end = pos[low]
    o_mid = (outline.min(0) + outline.max(0)) / 2
    o_size = np.maximum((outline.max(0) - outline.min(0)) / 2, 1e-3)
    e_mid = (end.min(0) + end.max(0)) / 2
    # an end come to a point takes the outline's size, so that its few points
    # don't all go one way
    e_size = np.maximum((end.max(0) - end.min(0)) / 2, 0.3 * o_size)
    around = np.arctan2((outline[:, 2] - o_mid[2]) / o_size[2], (outline[:, 0] - o_mid[0]) / o_size[0])
    for i, q in zip(low, end):
        angle = np.arctan2((q[2] - e_mid[2]) / e_size[2], (q[0] - e_mid[0]) / e_size[0])
        target = outline[np.argmin(np.abs(np.angle(np.exp(1j * (around - angle)))))]
        q = q.copy()
        q[[0, 2]] = o_mid[[0, 2]] + (target[[0, 2]] - o_mid[[0, 2]]) * 0.95
        q[1] = TUCK * ankle
        moved[i] = q
straightened = len(moved)
ends = set(moved)


def inside(q, fit, margin):
    """the point q moved inside the shoe fit describes, margin in from its
    heel, toe and sole and a tenth in from its sides; None when it is above
    the shoe's top there, where the boot shows."""
    cx, cz, along, sections = fit
    z = float(np.clip(q[2] - cz, along[0] + margin, along[-1] - margin))
    k = int(np.clip(np.searchsorted(along, z), 1, len(along) - 1))
    t = (z - along[k - 1]) / (along[k] - along[k - 1])
    w, top, sole, bottom, shoulder = (1 - t) * np.array(sections[k - 1]) + t * np.array(sections[k])
    if q[1] >= top - margin:
        return None
    y = max(q[1], sole + margin)
    width = w if y <= sole else w - (1 - shoulder) * w * (y - sole) / (top - sole)
    x = cx + float(np.clip(q[0] - cx, -0.9 * width, 0.9 * width))
    return np.array([x, y, cz + z])


# then every point of the leg's own under the shoe's top goes inside it, so
# that no boot, whatever its shape, shows through or past the shoe
for leg in "lr":
    on_leg, height = frame(leg)[0], frame(leg)[7]
    for i in np.nonzero(on_leg)[0]:
        q = moved.get(i, pos[i])
        put = inside(q, feet[leg][5], 0.02 * height)
        if put is not None and not np.allclose(put, q, atol=1e-5):
            moved[i] = put
after = pos.copy()
for i, q in moved.items():
    after[i] = q
fmt, count, scale, offset = v["attrs"][0]
n_fmt, n_count, n_scale, n_offset = v["attrs"][1]
vb_at = b.rel(b.rel(shape + 0x3C))
data_at, stride_at = b.rel(vb_at + 0x18), b.u32(vb_at + 0x24)
every = np.vstack([sm["triangles"] for sm in subs])
touching = every[np.isin(every, list(moved)).any(1)]
face = np.cross(after[touching[:, 1]] - after[touching[:, 0]], after[touching[:, 2]] - after[touching[:, 0]])
face /= np.linalg.norm(face, axis=1, keepdims=True) + 1e-12
for i, q in moved.items():
    n_ = face[(touching == i).any(1)].sum(0)
    n_ /= np.linalg.norm(n_) + 1e-12
    at = data_at + i * stride_at
    raw[at + offset:at + offset + 12] = struct.pack("<3f", *(q / scale))
    raw[at + n_offset:at + n_offset + 12] = struct.pack("<3f", *(n_ / n_scale))
# the boot's end takes the texture coordinates of the leg's point right
# above each of its points, the nearest it shares a face with, so that the
# faces from there down into the foot carry the leg's own texels on instead
# of stretching the game's old sole up to the foot's top, a seam. a leg that
# leans has no point right above, and keeps its own, as does a bare leg,
# whose old foot is skin and straps that go on into the bare foot
carried = 0
if 4 in v["attrs"] and style != "sandal":
    _, u_count, _, u_offset = v["attrs"][4]
    u_width = {0x1406: 4, 0x1402: 2, 0x1401: 1, 0x1400: 1}[v["attrs"][4][0]] * u_count
    for leg in "lr":
        on_leg, _, _, _, half, depth = frame(leg)[:6]
        reach = 0.3 * max(depth, 2 * half)
        for i in ends & set(np.nonzero(on_leg)[0].tolist()):
            above = [j for j in np.unique(touching[(touching == i).any(1)]) if j not in ends]
            if not above:
                continue
            j = min(above, key=lambda j: float(np.sum((after[j, [0, 2]] - after[i, [0, 2]]) ** 2)))
            if np.linalg.norm(after[j, [0, 2]] - after[i, [0, 2]]) > reach:
                continue
            src, dst = data_at + j * stride_at + u_offset, data_at + i * stride_at + u_offset
            raw[dst:dst + u_width] = raw[src:src + u_width]
            carried += 1
print(f"moved {straightened} vertices of the boots' ends to make them straight boots, "
      f"{len(moved) - straightened} more inside the shoes, {carried} of the first given the leg's texels above them")



def pad(n):
    while len(raw) % n:
        raw.append(0)


# a foot bone for each leg after the skeleton's own: the shin's record copied,
# named for the foot, the shin its parent, no other bone linked to it, its
# flags having the game compute it a skinning matrix, and its foot's points
# and the tag in place of its inverse bind matrix; then the skeleton's
# dictionary built again with the two at its end
entries = b.dict(b.rel(skeleton + 0x1C))
count = b.u32(skeleton + 0x18)
assert count == len(entries)
old_dict = b.rel(skeleton + 0x1C)
name_at = {b.cstr(b.rel(old_dict + 0x0C + 0x10 * i + 8)): b.rel(old_dict + 0x0C + 0x10 * i + 8) for i in range(1, count + 1)}
data_at = {name: at for name, at in entries}
pad(4)
for leg in "lr":
    name_at[f"{leg}_shoe"] = len(raw)
    raw += f"{leg}_shoe".encode() + b"\0"
pad(4)
foot_bones = {}
for k, leg in enumerate("lr"):
    verts, normals, uvs, tris, points, _ = feet[leg]
    shin_at, at = records[f"{leg}_leg2"], len(raw)
    record = bytearray(raw[shin_at:shin_at + 0xE0])
    struct.pack_into("<iIIIiiii", record, 0, name_at[f"{leg}_shoe"] - at, b.u32(shin_at + 4) | SKINNED, count + k,
                     shins[leg], shin_at - (at + 0x10), 0, 0, 0)
    struct.pack_into("<9f", record, INVERSE_BIND, *np.concatenate(points))
    record[INVERSE_BIND + 36:INVERSE_BIND + 48] = TAG + bytes(8)
    record[0xD8:0xE0] = bytes(8)
    raw += record
    data_at[f"{leg}_shoe"] = at
    foot_bones[leg] = count + k
    print(f"{leg}_shoe, bone {count + k} under {names[shins[leg]]}: ankle {np.round(points[0], 3)}, toe {np.round(points[1], 3)}, heel {np.round(points[2], 3)}")
pad(4)
dict_at = len(raw)
nodes = cgfx_dict.build([name for name, _ in entries] + ["l_shoe", "r_shoe"])
raw += cgfx_dict.pack(nodes, dict_at, name_at, data_at)
struct.pack_into("<I", raw, skeleton + 0x18, count + 2)
struct.pack_into("<i", raw, skeleton + 0x1C, dict_at - (skeleton + 0x1C))

stride = v["stride"]
attrs = v["attrs"]
n0 = v["count"]
palette = [foot_bones["l"], foot_bones["r"]]
new_vertices = bytearray()
new_indices = []
for leg, (verts, normals, uvs, tris, points, _) in feet.items():
    base = n0 + len(new_vertices) // stride
    slot = palette.index(foot_bones[leg])
    for p_, n_, t_ in zip(verts, normals, uvs):
        vtx = bytearray(stride)
        for usage, values in ((0, p_), (1, n_), (4, t_)):
            fmt, count, scale, offset = attrs[usage]
            vtx[offset:offset + 4 * count] = struct.pack(f"<{count}f", *[float(x) / scale for x in values[:count]])
        fmt, count, scale, offset = attrs[7]
        vtx[offset:offset + count] = bytes([slot] + [0] * (count - 1))
        fmt, count, scale, offset = attrs[8]
        vtx[offset:offset + count] = bytes([100] + [0] * (count - 1))
        new_vertices += vtx
    new_indices += (tris + base).ravel().tolist()


# the grown vertex buffer and the feet's triangles at the end of IMAG, the
# last block
vb = b.rel(b.rel(shape + 0x3C))
old_size, old_data = b.u32(vb + 0x14), b.rel(vb + 0x18)
pad(0x80)
vertex_at = len(raw)
raw += raw[old_data:old_data + old_size] + new_vertices
pad(0x10)
index_at = len(raw)
raw += struct.pack(f"<{len(new_indices)}H", *new_indices)
struct.pack_into("<I", raw, vb + 0x14, old_size + len(new_vertices))
struct.pack_into("<i", raw, vb + 0x18, vertex_at - (vb + 0x18))

# the feet's primitive set after them, laid out as the first set is: its
# header, its palette, its list of primitives, the primitive and its index
# stream, those two copied from the first set's
sets, old_list = b.u32(shape + 0x2C), b.rel(shape + 0x30)
first = b.rel(old_list)
first_primitive = b.rel(b.rel(first + 0x10))
first_stream = b.rel(b.rel(first_primitive + 4))
pad(4)
feet_set = len(raw)
body = bytearray(0x14 + 4 * len(palette) + 4)
struct.pack_into("<5I", body, 0, len(palette), 0x10, b.u32(first + 8), 1, 0x14 + 4 * len(palette) - 0x10)
struct.pack_into(f"<{len(palette)}I", body, 0x14, *palette)
struct.pack_into("<I", body, len(body) - 4, 4)
primitive = len(body)
body += raw[first_primitive:first_primitive + 0x20]
assert b.rel(first_primitive + 0x18) == first_primitive + 0x20, "the first set's index stream right after its primitive"
stream = len(body)
body += raw[first_stream:first_stream + 0x2C]
struct.pack_into("<I", body, stream + 8, 2 * len(new_indices))
struct.pack_into("<i", body, stream + 12, index_at - (feet_set + stream + 12))
raw += body
# the shape's list of primitive sets, one longer
new_list = len(raw)
for i in range(sets):
    raw += struct.pack("<i", b.rel(old_list + 4 * i) - (new_list + 4 * i))
raw += struct.pack("<i", feet_set - (new_list + 4 * sets))
struct.pack_into("<I", raw, shape + 0x2C, sets + 1)
struct.pack_into("<i", raw, shape + 0x30, new_list - (shape + 0x30))
pad(0x80)
# the blocks' and the file's sizes
data = b.u16(6)
imag = data + b.u32(data + 4)
assert raw[imag:imag + 4] == b"IMAG"
struct.pack_into("<I", raw, imag + 4, len(raw) - imag)
struct.pack_into("<I", raw, 12, len(raw))
print(f"{want}: {len(new_vertices) // stride} foot vertices, {len(new_indices) // 3} triangles in primitive set {sets}, "
      f"{len(new_indices)} indices drawn, file {len(b.raw)} -> {len(raw)} bytes")

out = os.path.join(data_dir, "mods", "000400000009F100", "feet", "romfs", *want.split("/"))
os.makedirs(os.path.dirname(out), exist_ok=True)
open(out, "wb").write(lz13(bytes(raw)))
print("wrote", out)


def rotate(v, axis, angle):
    """v turned by angle about the unit axis, Rodrigues' way."""
    return v * np.cos(angle) + np.cross(axis, v) * np.sin(angle) + axis * (axis @ v) * (1 - np.cos(angle))


def turn(ankle, toe, heel, up=np.array([0.0, 1.0, 0.0]), floor=np.zeros(3)):
    """the axis and the angle the hook turns a foot by, feet.rs's turn."""
    height = lambda point: up @ (point - floor)
    if height(toe) >= 0 and height(heel) >= 0:
        return None
    sole = toe - heel
    across = np.cross(up, sole)
    if across @ across < 1e-12:
        return None
    axis = across / np.linalg.norm(across)
    rise = up @ sole
    flat = np.arcsin(np.clip(rise / np.linalg.norm(sole), -1, 1))
    # a heel under the floor lays the foot flat, a toe comes up to it
    if rise >= 0:
        return axis, flat
    lower_at = lambda angle: height(ankle + rotate(toe - ankle, axis, angle))
    if lower_at(flat) <= 0:
        return axis, flat
    under, over = 0.0, flat
    for _ in range(24):
        middle = (under + over) / 2
        under, over = (middle, over) if lower_at(middle) < 0 else (under, middle)
    return axis, over


def turned(m, axis, angle, about):
    """m followed by a turn of angle about axis through the point about."""
    out = np.zeros((3, 4))
    for column in range(3):
        out[:, column] = rotate(m[:, column], axis, angle)
    out[:, 3] = about + rotate(m[:, 3] - about, axis, angle)
    return out


if "preview" in options:
    # the legs' ends and the feet as the game draws them, from the rebuilt
    # file's own buffers with back faces culled and a depth buffer, over the
    # floor when posed: in the texture's colours when there is one, else
    # grey the body's own mesh and red the feet; from the
    # front, the back, both legs' outer sides and above the front
    from PIL import Image, ImageDraw
    nb = cgfx.Blob(bytes(raw))
    nshape = cm.model_parts(nb)[2]
    nv = cm.vertices(nb, nshape)
    npos = nv["position"]
    nsubs = cm.submeshes(nb, nshape)
    pose = pose_of(options["pose"]) if "pose" in options else None
    if pose is not None:
        # the sub-mesh with the shins skinned as the dumped draw was, and the
        # feet with the shins' matrices turned as the hook turns them, the
        # points it reads from the rebuilt foot bones
        feet_pose = []
        rebuilt = dict(nb.dict(nb.rel(cm.model_parts(nb)[1] + 0x1C)))
        for leg in "lr":
            shin = pose[sm_info["palette"].index(shins[leg])]
            at = nb.f32(rebuilt[f"{leg}_shoe"] + INVERSE_BIND, 9)
            ankle, toe, heel = [shin[:, :3] @ np.array(at[3 * k:3 * k + 3]) + shin[:, 3] for k in range(3)]
            turning = turn(ankle, toe, heel)
            print(f"{leg} foot in the pose: toe {toe[1]:+.3f}, heel {heel[1]:+.3f} over the floor, "
                  + (f"turned {np.degrees(turning[1]):+.1f} degrees" if turning else "not turned"))
            feet_pose.append(turned(shin, *turning, ankle) if turning else shin)
        w = nv["weights"] / np.maximum(nv["weights"].sum(1, keepdims=True), 1e-9)
        posed = np.zeros_like(npos)
        tri = []
        for palette_pose, sm in ((pose, nsubs[target]), (feet_pose, nsubs[-1])):
            used = np.unique(sm["triangles"])
            for k in range(nv["bones"].shape[1]):
                M = np.array(palette_pose)[np.minimum(nv["bones"][used, k], len(palette_pose) - 1)]
                posed[used] += w[used, k:k + 1] * (np.einsum("nij,nj->ni", M[:, :, :3], npos[used]) + M[:, :, 3])
            tri.append(sm["triangles"])
        npos, tri = posed, np.vstack(tri)
    else:
        tri = np.vstack([sm["triangles"] for sm in nsubs])
    low = npos[np.unique(tri), 1].min()
    # the legs from the floor to above the knees, or to upto shares of the
    # knee's height for a closer look
    tri = tri[(npos[tri][:, :, 1] < low + float(options.get("upto", 1.1)) * knee("l")).all(1)]
    ours = (tri >= n0).any(1)
    # the floor, a quad under the legs facing up
    lo, hi = npos[np.unique(tri)].min(0) - 0.8, npos[np.unique(tri)].max(0) + 0.8
    floor = len(npos)
    npos = np.vstack([npos, [[lo[0], 0.0, lo[2]], [hi[0], 0.0, lo[2]], [hi[0], 0.0, hi[2]], [lo[0], 0.0, hi[2]]]])
    shown_floor = pose is not None
    if shown_floor:
        tri = np.vstack([tri, [[floor, floor + 2, floor + 1], [floor, floor + 3, floor + 2]]])
        ours = np.append(ours, [False, False])
    is_floor = (tri >= floor).all(1)
    # the texture across each face, brightened so that dark leather's shape
    # reads, as the game's lighting does, and light skin or plate no more
    # than to keep it from going white
    tex = None
    if texture is not None:
        brightness = 2.4
        if patch is not None:
            x, y, w, h = patch[0][UPPER]
            upper_lum = texture[y:y + h, x:x + w, :3].reshape(-1, 3).mean(0) @ [0.299, 0.587, 0.114]
            brightness = float(np.clip(150 / max(upper_lum, 1), 1.0, 2.4))
        tex = texture[:, :, :3].astype(float) * brightness
    corner_uv = np.vstack([nv["uv"], np.zeros((4, 2))])[tri]
    views = {"front": (0, 12), "back": (180, 12), "left leg's outer side": (90, 0), "right leg's outer side": (-90, 0), "above the front": (35, 30)}
    # views=a,b keeps those, by the first word of their names
    if "views" in options:
        views = {name: view for name, view in views.items() if name.split()[0].strip("'s") in options["views"].split(",")}
    # the views turned as the body heads in the pose's world
    heading = 0.0
    if pose is not None:
        forward = pose[sm_info["palette"].index(next(j for j, n in names.items() if n == "hip"))][:, 2]
        heading = np.degrees(np.arctan2(forward[0], forward[2]))
    cell_w, cell_h = 420, 360
    img = Image.new("RGB", (cell_w * len(views), cell_h + 24), (245, 245, 245))
    pixels = np.asarray(img).copy()
    light = np.array([0.3, 0.8, 0.5]) / np.linalg.norm([0.3, 0.8, 0.5])
    p3 = npos[tri]
    n = np.cross(p3[:, 1] - p3[:, 0], p3[:, 2] - p3[:, 0])
    n /= np.linalg.norm(n, axis=1, keepdims=True) + 1e-12
    shade = 0.35 + 0.65 * np.clip(n @ light, 0, 1)
    for k, (label, (yaw, pitch)) in enumerate(views.items()):
        ya, pa = np.radians(yaw + heading), np.radians(pitch)
        # toward the camera; a face is drawn when it turns that way
        look = np.array([np.sin(ya) * np.cos(pa), np.sin(pa), np.cos(ya) * np.cos(pa)])
        right = np.cross([0, 1, 0], look)
        right /= np.linalg.norm(right)
        up = np.cross(look, right)
        sx, sy, sz = p3 @ right, p3 @ up, p3 @ look
        body = ~is_floor
        lo2, hi2 = np.array([sx[body].min(), sy[body].min()]), np.array([sx[body].max(), sy[body].max()])
        scale = min((cell_w - 20) / (hi2[0] - lo2[0]), (cell_h - 20) / (hi2[1] - lo2[1]))
        X = 10 + (sx - lo2[0]) * scale
        Y = cell_h - 10 - (sy - lo2[1]) * scale
        depth = np.full((cell_h, cell_w), -np.inf)
        cell = pixels[:cell_h, k * cell_w:(k + 1) * cell_w]
        for t in np.nonzero(n @ look > 0)[0]:
            xs, ys, zs = X[t], Y[t], sz[t]
            x0_, x1_ = max(int(xs.min()), 0), min(int(xs.max()) + 1, cell_w - 1)
            y0_, y1_ = max(int(ys.min()), 0), min(int(ys.max()) + 1, cell_h - 1)
            if x0_ > x1_ or y0_ > y1_:
                continue
            gx, gy = np.meshgrid(np.arange(x0_, x1_ + 1) + 0.5, np.arange(y0_, y1_ + 1) + 0.5)
            d = (ys[1] - ys[2]) * (xs[0] - xs[2]) + (xs[2] - xs[1]) * (ys[0] - ys[2])
            if abs(d) < 1e-9:
                continue
            l0 = ((ys[1] - ys[2]) * (gx - xs[2]) + (xs[2] - xs[1]) * (gy - ys[2])) / d
            l1 = ((ys[2] - ys[0]) * (gx - xs[2]) + (xs[0] - xs[2]) * (gy - ys[2])) / d
            l2 = 1 - l0 - l1
            inside = (l0 >= 0) & (l1 >= 0) & (l2 >= 0)
            z = l0 * zs[0] + l1 * zs[1] + l2 * zs[2]
            window = depth[y0_:y1_ + 1, x0_:x1_ + 1]
            nearer = inside & (z > window)
            window[nearer] = z[nearer]
            block = cell[y0_:y1_ + 1, x0_:x1_ + 1]
            if is_floor[t]:
                block[nearer] = (150, 185, 140)
            elif tex is not None:
                # the view is straight on, so texture coordinates go across the
                # face as the barycentric weights do
                u = l0 * corner_uv[t, 0, 0] + l1 * corner_uv[t, 1, 0] + l2 * corner_uv[t, 2, 0]
                w_ = l0 * corner_uv[t, 0, 1] + l1 * corner_uv[t, 1, 1] + l2 * corner_uv[t, 2, 1]
                tx = np.clip((u * tex.shape[1]).astype(int), 0, tex.shape[1] - 1)
                ty = np.clip(((1 - w_) * tex.shape[0]).astype(int), 0, tex.shape[0] - 1)
                block[nearer] = np.clip(tex[ty, tx] * shade[t], 0, 255)[nearer]
            else:
                colour = (220, 90, 70) if ours[t] else (170, 170, 180)
                block[nearer] = [int(c * shade[t]) for c in colour]
    img = Image.fromarray(pixels)
    d = ImageDraw.Draw(img)
    for k, label in enumerate(views):
        d.text((k * cell_w + 10, cell_h + 4), label + (", posed, the floor green" if shown_floor else ""), fill=(40, 40, 40))
    img.save(options["preview"])
