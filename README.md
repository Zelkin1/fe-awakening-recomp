# Fire Emblem: Awakening for Windows

A native Windows version of *Fire Emblem: Awakening* (3DS, European release).
The game's code is converted ahead of time with
[3dsrecomp](https://github.com/fearkov/3dsrecomp) and runs on
[Zakuro](https://github.com/fearkov/zakuro), which handles the console's
graphics, sound and system.

This repository contains no game files. You need your own copy of the game,
dumped from your cartridge; everything is built from it on your computer.
See `LEGAL.md` for details.

## Features

- **High resolution:** the 3D is drawn at up to 8 times the console's
  resolution, with optional smooth 60 FPS battles and cutscenes.
- **Widescreen:** a 16:9 mode shows more of the battlefield.
- **One screen:** play on the top screen alone. Press **E** to bring up the
  bottom screen (a unit's stats and items, the world map) in the middle of
  the screen, and **Q** to move from unit to unit while it is open.
- **Battle display:** choose the game's own panels, a cleaner banner style,
  or nothing at all over the fight.
- **World map:** the name of the selected place stays at the top of the
  screen.
- **Feet:** an optional mod gives every character proper feet that rest on
  the ground. See `mods/feet/README.md`.
- **Mods and codes:** game mods each in their own folder, cheat codes, and
  built-in tweaks such as faster battles.
- **Original mode:** one switch in the settings restores the game exactly
  as it plays on the console.

## Controls

The keyboard and most controllers (Xbox, PlayStation, Switch Pro) work. The
mouse acts as the touch screen. Keys can be changed in the settings.

## Playing

You need your own copy of the game: the European cartridge dumped as a
decrypted `.3ds` or `.cci` file, and if you have them, its update and DLC as
decrypted `.cia` files.

1. Get Zakuro's launcher, `zakuro.exe` (see "Building the launcher").
2. Open it and choose the folder that holds your game files.
3. Click **Build** next to the game. The launcher builds the feet mod from
   your copy of the game, then converts the game's code to native code.
   This takes 10 to 20 minutes. If your computer has no C compiler, the
   launcher offers to download one (Zig, which needs no installation).
4. Click **Play**.

What the launcher builds stays on your computer: the feet mod in
`%APPDATA%\zakuro\mods`, and the converted code in `%APPDATA%\3dsrecomp`.

## Building the launcher

You need Git Bash, Rust and a MinGW gcc (`tools/env.sh` sets them up).

1. Clone [3dsrecomp](https://github.com/fearkov/3dsrecomp) and
   [Zakuro](https://github.com/fearkov/zakuro) into this folder, then apply
   the patches: `patches/3dsrecomp-*.patch` to 3dsrecomp, and
   `patches/zakuro-local-changes.patch` then `mods/feet/zakuro-feet.patch`
   to Zakuro.
2. Build it:

```bash
source tools/env.sh
(cd zakuro && cargo build --release -p zakuro)
```

The launcher is `zakuro/target/release/zakuro.exe`. A copy given to others
must come with the licenses of the code it is built from, which
`tools/licenses.py` collects:

```bash
(cd zakuro && cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-gnu > ../metadata.json)
python tools/licenses.py metadata.json THIRD_PARTY_LICENSES.txt
```

### A standalone program

The game can also be built into a program of its own, `fe-awakening.exe`,
which `Play-FireEmblemAwakening.bat` starts:

```bash
source tools/env.sh
(cd 3dsrecomp && cargo build --release)
./3dsrecomp/target/release/3dsrecomp.exe port "<your game>.cci" FireEmblemAwakeningRecomp --zakuro "$W/zakuro" --name fe-awakening
(cd FireEmblemAwakeningRecomp && cargo build --release)
```

## License

The project's own files are under the MIT license (`LICENSE`). Zakuro and
3dsrecomp keep their own licenses (`THIRD_PARTY_NOTICES.md`). Fire Emblem is
a trademark of Nintendo; this project is not affiliated with Nintendo or
Intelligent Systems.
