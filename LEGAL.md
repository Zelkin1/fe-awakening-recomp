# Legal

This project converts *Fire Emblem: Awakening* (Nintendo 3DS, European
release) to native code from the player's own copy of the game, and runs it
in Zakuro. It is not affiliated with, endorsed by or connected to Nintendo,
Intelligent Systems or any other company.

## What the repository and the launcher contain

The repository holds only the project's own files: `.gitignore`,
`.recomp.json` (the project's entry for the recomp.fyi list),
`README.md`, `LEGAL.md`, `LICENSE`, `THIRD_PARTY_NOTICES.md`, `PROJECT_CODE.md`
(the list of the project's code), the patches
(`patches/*.patch`), the Zakuro and 3dsrecomp files the patches add or
change in their final form (`source/`), two build scripts (`tools/env.sh` and
`tools/licenses.py`) and the feet mod's patch and README
(`mods/feet/zakuro-feet.patch` and `mods/feet/README.md`).
`.gitignore` leaves out everything else. The launcher download is Zakuro
built with these patches, with the licenses of the code it uses.

None of this contains code, data, assets, text, shaders, keys or firmware
from the game or the console, or any decompiled or recompiled game code.
What it does hold about the game is what the patches need to find their
place:

- addresses in the game's code: where 3dsrecomp should start functions (the
  Awakening section of `hints.txt`, in the 3dsrecomp patch and in
  `source/3dsrecomp/`), and where the
  built-in tweaks (`enhancements.txt`) and the feet hook (`feet.rs`) step in;
- names: the game's title ID, its file names (such as
  `b/uBody_M_tac1_0.m.lz`), its bones' names (such as `l_leg2`) and the
  names of the sounds the bottom screen plays as it comes up and goes away
  (`SE_SYS_SELECT1` and `SE_SYS_CANCEL1`, the game's sounds for choosing a
  unit and letting it go). The launcher reads them from your copy's sound
  archive when the game starts, and keeps them in memory only;
- what the game draws: the format, size and position of the textures that
  tell Zakuro which screen is showing, the rectangles of the panels and text
  the project redraws, the outline of the world map's name panel, and the
  colour rule for the dark area around a place's icon. No pixels are
  stored: the launcher reads them from your running game's screen.

The code that changes what the game does is the project's own. `feet.rs`
computes the ankles' matrices itself, and the 16:9, Smooth map sprites and
Faster battles tweaks are ARM instructions written for this project. Zakuro
keeps what a tweak overwrites in memory while the game runs, never in a
file.

## You supply your own copy

You need your own decrypted dump of the game's cartridge and, if you want
it, of its DLC. How you make them is outside this project. The project has
no decryption and no keys, and Zakuro refuses a dump that is still
encrypted.

## What is made from the game stays on your computer

These are made on your computer from your copy, and contain the game's code
or data, changed or not. Don't share or upload them:

- what the launcher's Build button makes: the game's code converted to
  native code, a library in `%APPDATA%\3dsrecomp\`, and the feet mod's
  models in `%APPDATA%\zakuro\mods\000400000009F100\feet\`. The builder is
  the project's; the models it writes are the game's, changed;
- Zakuro's folder, `%APPDATA%\zakuro`, with your saves, states, settings
  and mods;
- in a development copy, `FireEmblemAwakeningRecomp/` (the C that 3dsrecomp
  generates and the library the test harness loads), `_work/` (the game's code
  extracted from your dump, and memory dumps), `_runs/` (the test harness's
  screens, dumps, states, saves and the mods it builds) and screenshots of
  the game.

A development copy's other work files stay offline too: the Zakuro and
3dsrecomp clones, the notes (`notes/`), the hints Zakuro records (`hints/`),
the development scripts, the toolchain (`tools/rust/`) and logs.

## Licenses and trademarks

The project's own files are under the MIT license in `LICENSE`. So are the
changes the patches make to Zakuro and 3dsrecomp, which keep fearkov's MIT
notice (`THIRD_PARTY_NOTICES.md`). The files in `source/` hold those
changes together with fearkov's code around them, under the same notices.
The license covers none of the game, nor anything made from it. The
launcher download lists the licenses of the
crates it is built from in `THIRD_PARTY_LICENSES.txt`. The Rust toolchain in
`tools/rust/` is a download under its own licenses.

Nintendo, Nintendo 3DS and Fire Emblem are trademarks of Nintendo.
*Fire Emblem: Awakening* is © Nintendo / Intelligent Systems. Other names
are their owners' trademarks.
