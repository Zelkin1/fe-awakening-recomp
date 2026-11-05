# Legal

This project recompiles *Fire Emblem: Awakening* (Nintendo 3DS, Europe) from
the player's own copy of the game and runs it in Zakuro. It is not
affiliated with, endorsed by or connected to Nintendo, Intelligent Systems or
any other company.

## The project's own files hold no game content

The project's own files, the only ones a repository of it holds, are
`.gitignore`, `README.md`, `LEGAL.md`, `LICENSE`, `THIRD_PARTY_NOTICES.md`,
the `.bat` launcher, the patches (`patches/*.patch`), the scripts
(`tools/*.py`, `tools/*.sh`) and the feet mod's patch, scripts and README
(`mods/feet/zakuro-feet.patch`, `mods/feet/*.py`, `mods/feet/README.md`).
`.gitignore` leaves everything else out and lets these in by name. They hold no code, data,
assets, text, shaders, keys or firmware from the game or the console, and no
decompiled or recompiled game code.

What they do hold about the game is what a patch or a test needs to find its
place:

- **Addresses and offsets** in the game's code: the functions 3dsrecomp
  should start at (the Awakening section of 3dsrecomp's `hints.txt`, in
  its patch), where the enhancements patch (`enhancements.txt` in the
  Zakuro patch) and where the feet hook steps in (`feet.rs`, in the feet
  mod's patch).
- **Names:** the game's file names (`b/uBody_M_tac1_0.m.lz`), its bones'
  names (`l_leg2`, `l_leg3`) and its title ID.
- **What the game draws:** the format, size and screen position of the
  textures that tell Zakuro which window is showing (the markers in
  `loader.rs`), the rectangles of the battle, title screen and world map
  panels it redraws (the pieces) and of the battle text it draws anew (the
  banners), the outline of the world map's name panel as a polygon of
  coordinates, and the colour rule for the dark ground around a place's
  icon. No pixels: what is shown is read from your running game's screen
  as it plays, and never stored.
- **The project's own instructions:** the enhancements' code lines are ARM
  instructions written for this project. What one overwrites is kept by
  Zakuro in memory while it runs, never in a file.

Where a hook changes what the game does, the project's own code does the
work: `feet.rs` computes the ankle's matrix itself, and the 16:9, Smooth map
sprites and Faster battles routines are the project's own instructions.

## You supply your own copy

You need your own dump of the game's cartridge, decrypted, and, if you want
it, your own dump of its DLC. How you make them is outside this project.

The project has no decryption and no keys. Zakuro refuses a dump that is
still encrypted, and DLC whose contents are.

## What is made from the game stays on your computer

Everything below is made on your computer from your copy. It contains the
game's code or data, changed or not. Don't share or upload any of it:

- `_work/`: the game's code extracted from your dump, and memory dumps.
- `FireEmblemAwakeningRecomp/`: the C that 3dsrecomp generates from the
  game's code, its objects, the library and `fe-awakening.exe`.
- `_runs/`: the test harness's screen dumps, memory, texture and uniform
  dumps, save states, saves, and the mods it builds.
- What the launcher's Build button makes:
  - The feet mod's models. The launcher (or `mods/feet/foot_build.py`)
    reads a body model from your dump and writes a changed copy into the
    mod's own folder, `%APPDATA%\zakuro\mods\000400000009F100\feet\` (or a
    folder in `_runs/` for the scripts). The builder is the project's; the
    model it writes is the game's, changed.
  - The game's code converted to native code, a library in
    `%APPDATA%\3dsrecomp\`.
- Zakuro's own folder, `%APPDATA%\zakuro`, and `userdata/`, where the
  launchers keep theirs: your saves, states, settings and mods.

Screenshots of the game are pictures of Nintendo's work; the harness keeps
them in `_runs/`.

The other work files stay on your computer too: the Zakuro and 3dsrecomp
clones the patches apply to, the development notes (`notes/`), the
interpreter hints Zakuro records (`hints/`), the rest of `mods/`, the
toolchain (`tools/rust/`, `tools/dl/`), logs and the recompiler's coverage report
(`analyze.txt`).

## Third-party code

Zakuro and 3dsrecomp are fearkov's, under the MIT license. The patches in
`patches/` change their code and stay under that license, with its notice;
see `THIRD_PARTY_NOTICES.md`. The Rust toolchain in `tools/rust/` and its
installer in `tools/dl/` are downloads under their own licenses, not part of
the project.

## Trademarks

Nintendo, Nintendo 3DS and Fire Emblem are trademarks of Nintendo.
*Fire Emblem: Awakening* is © Nintendo / Intelligent Systems. Other names
are their owners' trademarks.

## The project's own code

The project's own files are under the MIT license in `LICENSE`. The changes
the patches make to Zakuro and 3dsrecomp are under MIT too, beside their
own code's notice. The license covers none of the game, nor anything made
from it.
