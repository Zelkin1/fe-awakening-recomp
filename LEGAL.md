# Legal

This project converts *Fire Emblem: Awakening* (Nintendo 3DS, European
release) to native code from the own copy of the game of the player, and runs
it in Zakuro. The project is not affiliated with, endorsed by or connected to
Nintendo, Intelligent Systems or any other company.

## What the repository and the launcher contain

The repository holds only the files of the project:

- `.gitignore`.
- `.recomp.json`, the entry of the project for the recomp.fyi list.
- `README.md`, `LEGAL.md`, `LICENSE` and `THIRD_PARTY_NOTICES.md`.
- `PROJECT_CODE.md`, the list of the code of the project.
- The patches, `patches/*.patch`.
- The files of Zakuro and 3dsrecomp that the patches add or change, in their
  final form, in `source/`.
- Two build scripts, `tools/env.sh` and `tools/licenses.py`.
- The patch and the README of the feet mod, `mods/feet/zakuro-feet.patch` and
  `mods/feet/README.md`.

`.gitignore` leaves out everything else. The launcher download is Zakuro,
built with these patches, with the licenses of the code that it uses.

None of these files contain code, data, assets, text, shaders, keys or
firmware from the game or the console. They contain no decompiled or
recompiled game code. They hold only what the patches need to find their
place in the game:

- Addresses in the code of the game. These tell 3dsrecomp where functions
  start (the Awakening section of `hints.txt`, in the 3dsrecomp patch and in
  `source/3dsrecomp/`). They also tell where the built-in tweaks
  (`enhancements.txt`) and the feet hook (`feet.rs`) change the game.
- Names. These are the title ID of the game, the names of its files (such as
  `b/uBody_M_tac1_0.m.lz`) and the names of its bones (such as `l_leg2`).
  They also include the names of the two sounds that the bottom screen plays
  when it opens and closes (`SE_SYS_SELECT1` and `SE_SYS_CANCEL1`). The game
  plays these sounds when you choose a unit and when you let it go. The
  launcher reads them from the sound archive of your copy when the game
  starts, and keeps them in memory only.
- Words. These are the short words that the game paints on its phase,
  mission and result banners and on its inventory panels, in its five
  languages. They also include the chapter and place names that the title
  pictures of the game spell differently from its text files, and the words
  of its title screen and main menu (`hdtext/data.rs`). The HD text mod draws
  them again in an open font, over pictures that it reads from your copy.
- What the game draws. These are the format, the size and the position of
  the textures that tell Zakuro which screen shows, and the rectangles of the
  panels and the text that the project draws again. They also include the
  outline of the name panel on the world map, and the color rule for the dark
  area around the icon of a place. No pixels are stored. The launcher reads
  them from the screen of your running game.

The code that changes what the game does belongs to the project. `feet.rs`
calculates the matrices of the ankles itself. The 16:9, Smooth map sprites and
Faster battles tweaks are ARM instructions written for this project. Zakuro
keeps what a tweak overwrites in memory while the game runs, never in a file.

## You supply your own copy

You need your own decrypted dump of the cartridge of the game. If you want the
DLC, you also need your own dump of it. How you make the dumps is outside this
project. The project has no decryption and no keys, and Zakuro refuses a dump
that is still encrypted.

## What is made from the game stays on your computer

The launcher makes these files on your computer from your copy. They contain
the code or the data of the game, changed or not. Do not share or upload them:

- What the launcher builds:
  - The code of the game converted to native code, a library in
    `%APPDATA%\3dsrecomp\`.
  - The models of the feet mod in
    `%APPDATA%\zakuro\mods\000400000009F100\feet\`.
  - The fonts and the pictures of the HD text mod in
    `%APPDATA%\zakuro\mods\000400000009F100\hd\`.

  The builders belong to the project. What they write holds the models, the
  fonts and the pictures of the game, changed.
- The folder of Zakuro, `%APPDATA%\zakuro`, with your saves, states, settings
  and mods.
- In a development copy:
  - `FireEmblemAwakeningRecomp/`, with the C code that 3dsrecomp generates and
    the library that the test harness loads.
  - `_work/`, with the code of the game extracted from your dump, and memory
    dumps.
  - `_runs/`, with the screens, dumps, states and saves of the test harness,
    and the mods that it builds.
  - Screenshots of the game.

The other work files of a development copy stay offline too: the clones of
Zakuro and 3dsrecomp, the notes (`notes/`), the hints that Zakuro records
(`hints/`), the development scripts, the toolchain (`tools/rust/`) and the
logs.

## Licenses and trademarks

The files of the project are under the MIT license in `LICENSE`. The changes
that the patches make to Zakuro and 3dsrecomp are under the same license.
These changes keep the MIT notice of fearkov (`THIRD_PARTY_NOTICES.md`). The
files in `source/` hold those changes together with the code of fearkov around
them, under the same notices. The license covers no part of the game, and
nothing made from it.

The HD text mod draws with an open font under the SIL Open Font License. Its
notice is in `THIRD_PARTY_NOTICES.md`. The launcher download lists the
licenses of the crates that it is built from in `THIRD_PARTY_LICENSES.txt`.
The Rust toolchain in `tools/rust/` is a download under its own licenses.

Nintendo, Nintendo 3DS and Fire Emblem are trademarks of Nintendo.
*Fire Emblem: Awakening* is © Nintendo / Intelligent Systems. Other names are
trademarks of their owners.
