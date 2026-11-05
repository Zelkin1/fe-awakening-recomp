# Fire Emblem: Awakening for Windows

This project runs *Fire Emblem: Awakening* (Nintendo 3DS) natively on
Windows. [3dsrecomp](https://github.com/fearkov/3dsrecomp) converts the
game's code to native code ahead of time, and
[Zakuro](https://github.com/fearkov/zakuro) runs it and handles the
console's graphics, sound and system. The project's patches to both add the
features below.

The repository contains no game files. Everything is built from your own
copy of the game, on your computer.

## What you need

- The European release of the game (title ID `000400000009F100`), dumped
  from your cartridge as a decrypted `.3ds` or `.cci` file.
- The game's original code, revision 0. The widescreen mode, the other
  built-in tweaks and the feet are written for that code. With an update
  that changes the code, the game still runs, but without them.
- Optionally, the DLC, dumped as a decrypted `.cia` file.

## Playing

1. Download the launcher from the
   [releases page](https://github.com/Zelkin1/fe-awakening-recomp/releases),
   unzip it and run `zakuro.exe`. You can also build it yourself (see below).
2. Click Select game files and choose the game's `.3ds` or `.cci` file. If
   you have the DLC, choose its `.cia` file under Update in Settings, System.
3. Click Build. The launcher converts the game's code and gives the
   characters feet. This takes 10 to 20 minutes. If your computer has no C
   compiler, the launcher offers to download one (Zig, which needs no
   installation).
4. Click Play.

The keyboard, the mouse (as the touch screen) and most controllers work.
Esc opens the menu, with the settings, the cheats and save states.

## Features

- The 3D is drawn at up to 8 times the console's resolution, and frame
  interpolation can show battles and cutscenes at 60 frames per second.
- A 16:9 mode shows more of the battlefield.
- You can play on the top screen alone. E brings up the bottom screen in
  the middle, and Q moves from unit to unit while a unit's items are open.
- Battles can show the game's own panels, a display in the style of
  Fire Emblem: Three Houses, or nothing at all.
- On the world map, the name of the selected place stays on screen.
- Every character has feet that rest on the ground
  ([`mods/feet`](mods/feet/README.md)).
- The Cheats window has cheat codes and built-in tweaks such as faster
  battles.
- Original mode, at the top of Settings, Graphics, turns all of this off
  and plays the game as the console does.

## Building

The launcher is Zakuro with this project's patches, which apply to Zakuro
at commit `80d78be` and to 3dsrecomp at commit `9db0d8b`. You need Git,
Rust and MinGW-w64 gcc. In Git Bash:

```bash
git clone https://github.com/Zelkin1/fe-awakening-recomp
cd fe-awakening-recomp
git clone https://github.com/fearkov/3dsrecomp
git clone https://github.com/fearkov/zakuro
git -C 3dsrecomp checkout 9db0d8b
git -C zakuro checkout 80d78be
git -C 3dsrecomp apply ../patches/3dsrecomp-0001-port-toml-path.patch
git -C 3dsrecomp apply ../patches/3dsrecomp-0002-awakening-hints.patch
git -C zakuro apply ../patches/zakuro-local-changes.patch
git -C zakuro apply ../mods/feet/zakuro-feet.patch
(cd zakuro && cargo build --release -p zakuro)
```

The launcher is then `zakuro/target/release/zakuro.exe`. `tools/env.sh`
puts a Rust toolchain kept in `tools/rust` and a MinGW gcc on PATH, if
yours aren't there already.

A copy of the launcher given to others must include the licenses of the
code it is built from. `tools/licenses.py` collects them:

```bash
(cd zakuro && cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-gnu > ../metadata.json)
python tools/licenses.py metadata.json THIRD_PARTY_LICENSES.txt
```

3dsrecomp can also build the game into a program of its own,
`fe-awakening.exe`, which `Play-FireEmblemAwakening.bat` starts:

```bash
(cd 3dsrecomp && cargo build --release)
./3dsrecomp/target/release/3dsrecomp.exe port "<your game>.cci" FireEmblemAwakeningRecomp --zakuro zakuro --name fe-awakening
(cd FireEmblemAwakeningRecomp && cargo build --release)
```

## Legal

You need your own dump of the game. The project has no decryption and no
keys, and Zakuro refuses a dump that is still encrypted.

The repository and the launcher contain no code, data or assets from the
game. What they hold about it, such as addresses in its code and the names
of its files, is listed in [`LEGAL.md`](LEGAL.md). What the launcher builds
is made from your copy and stays on your computer: the converted code in
`%APPDATA%\3dsrecomp` and the feet in `%APPDATA%\zakuro\mods`. Don't share
either.

The project's own files are under the MIT license ([`LICENSE`](LICENSE)).
Zakuro and 3dsrecomp keep their own licenses
([`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)). Fire Emblem is a
trademark of Nintendo. This project is not affiliated with Nintendo or
Intelligent Systems.
