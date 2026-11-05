# Feet

A mod for this recompilation of *Fire Emblem: Awakening* (EUR,
`000400000009F100`). The game's bodies end in a wedge under the shin. This mod
gives them feet of their own, built on your computer from your dump, and a
Zakuro hook keeps them on the floor.

Everything the mod is made of is in this folder:

| File | What it is |
|---|---|
| `build_all.py` | Builds every body, several at a time, with previews |
| `foot_build.py` | Builds one body's feet |
| `cgfx.py`, `cgfx_model.py`, `cgfx_dict.py` | Readers for the game's CGFX models, and their bone dictionaries |
| `textures.py` | Reads and decodes a body's colour texture (ETC1) from your dump |
| `lz11.py` | The LZ11 compressor, with the LZ13 header the models are stored in |
| `make_mod.py`, `peek_model.py` | A model rebuilt unchanged for a round-trip test, and a RomFS file unpacked with the names it holds printed |
| `zakuro-feet.patch` | The hook in Zakuro that keeps the feet on the floor |

## What it does

- **The shape:** a low-poly foot, 68 triangles each, about as coarse as the
  game's own boots: six cross-sections (the heel, the heel's corner, the
  instep, the ball, the toe box and a blunt, narrower toe), each a flat sole
  with straight sides under an upper that slopes in to a flat top. Faces
  meeting at a sharp angle keep a hard edge. There are three styles, chosen
  by class: a shoe for leather and cloth boots, a plate sabaton with a thin
  sole for armoured classes, and a low bare foot for bare or strapped
  legs.
- **Fitting it to the leg:** the leg is measured where its surface crosses
  seven heights just above the ankle, using the median extent, so a fin or
  spike at one height doesn't count. The foot is as wide as the leg, but
  no wider than 0.21 of the knee's height, so a massive greave doesn't make
  a wide foot, and 2.35 times as long as the leg is deep, kept between 0.42
  and 0.60 of the knee's height. The heel sits behind the leg's back by 3%
  of the foot's length, and the instep is at the leg's front, so the leg
  comes down over the back of the foot. At least 40% of the foot shows in
  front of the leg, even under a thick greave.
- **Colours:** all from the leg's own texels, read from your dump, at the
  shin's texel density, so that the foot reads as the same material as the
  leg right above it.
  - **Upper:** a stretch in the colour seen where leg and foot meet, with a
    little of the leg's grain, and no row or column of it standing out in
    any colour, which on the foot would be a stripe along it; a smaller
    stretch when no large one is free of them.
  - **Grain:** each vertex takes its texels a little off where the
    mapping puts them, up to 1.5 texels, the same on both feet, so the
    leg's grain comes out mottled across the foot rather than in bands.
  - **Sole:** the darkest even stretch.
  - **Mapping:** the top takes its texels from above, the sides from the
    side and the heel and toe from the front, so the grain isn't stretched.
    Across the foot they run outward on both feet, so the two feet are
    mirror images.
  - **The join:** on a booted leg, the leg's end takes the texels of the leg
    right above it, so the boot's texture goes straight on into the foot.
- **The boot's end:** the leg's end is made straight. Each of its points
  goes to the leg's outline just above the ankle, on the side it lies on
  from the middle of the end, and is tucked up inside the foot. A wedge
  pointing forward comes back under the leg with its back straight down into
  the heel, and a straight leg stays straight. Every other point of the leg
  under the foot's top is moved inside the foot.
- **On the floor:** a foot keeps its angle to the shin while it's off the
  floor. A toe that would go under the floor comes up just far enough to
  touch it. A heel that would lays the whole foot flat, so the toe comes
  down onto the floor too.

## Building it

Zakuro needs both patches, the project's and this mod's, in that order, in
`zakuro\`:

```bash
git apply ../patches/zakuro-local-changes.patch
```

```bash
git apply ../mods/feet/zakuro-feet.patch
```

Then build the port as the main README says. List your dump's RomFS files
with Zakuro's `romdump` (in `zakuro\`):

```bash
cargo run --release -p zakuro-fs --example romdump -- <rom> files > romfs-files.txt
```

Build every body, 8 at a time:

```bash
python mods/feet/build_all.py <rom> romfs-files.txt <data folder> <preview folder> 8
```

or one:

```bash
python mods/feet/foot_build.py <rom> romfs-files.txt b/uBody_M_tac1_0.m.lz <data folder> preview=<views.png>
```

- **Output:** the changed models go in the mod's own folder,
  `<data folder>/mods/000400000009F100/feet/romfs/`. For the game, the data
  folder is `%APPDATA%\zakuro`.
- **Switching it off:** Zakuro lays each folder in `mods\000400000009F100\`
  over the game as a mod of its own, so moving `feet\` out turns the feet off
  without touching other mods.
- **Previews:** `preview` draws the legs and the feet from the floor to above
  the knees, from the front, the back, both outer sides and above, with the
  texture across each face, brightened so dark leather's shape reads, light
  skin or plate no more than keeps it from going white.
  - `upto=<share of the knee's height>` gives a closer look.
  - `views=front,left,...` keeps only some views.
  - `pose=<file>`, a `ZAKURO_DUMP_UNIFORMS` file of the body's draw, poses
    the legs as that draw was, with the feet turned as the hook turns them.
- **What `build_all.py` adds:** a sheet of each class's bodies, pages of
  them, and the list of bodies it leaves alone: the beasts and dragons, the
  bosses without legs, and the dancer's and the zombies' bodies.

## The hook

`zakuro-feet.patch` adds `zakuro-core/src/feet.rs` and what it needs:

- **Hooks on recompiled calls** (`recompiled::Hook`, `Library::hook`):
  every call in the recompiled code finds its target through the host's
  `lookup`, which hands back host code for an address with a hook. The hook
  gets the registers and memory and either goes on to the game's code or
  returns to the caller in its place. Calls from the interpreter skip it.
- **Where the game skins a body:** its skeleton update (`f_002605E0`) calls
  the 3x4 product at `0x00306964` from `0x002607C8` for each bone whose
  flags (+4 in its record) have `0x240`: r0 the bone's skinning matrix, r1
  its world matrix, r2 its inverse bind matrix (+0xA4). Both matrix arrays
  go by bone, 48 bytes each; the world matrix of bone 0 is the model's
  place, whose origin and y axis give the floor.
- **The hook itself** (`feet::skinning`, on `0x00306964` for Awakening EUR
  rev0, `loader::hook`): it acts only on a call from the skinning loop for
  a bone whose inverse bind matrix has the tag `FEET` in its tenth word.
  - **What the mod gives it:** the builder adds a foot bone under each shin
    and puts the feet in a primitive set of their own on those bones. Each
    primitive set draws at most 17 bones, and the body's two are full. The
    builder sets `0x240` on the foot bones and stores each foot's ankle, toe
    and heel in place of the inverse bind matrix, which nothing else reads.
  - **What it does:** it writes the foot bone's skinning matrix, the shin's
    turned by the rule above, and skips the product.
  - **Without the mod** it changes nothing.
- **Tests:** the rule (`feet::tests`).

## Limits

- The hook's addresses are the EUR game's without an update.
- Like every mod, the models load only when the game starts, not from a
  save state.
- The models the builder writes are the game's own, changed. They stay on
  your computer; don't share them. The builder and the patch hold none of
  the game's data.
