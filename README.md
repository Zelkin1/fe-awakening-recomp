# Fire Emblem: Awakening for Windows

This project runs *Fire Emblem: Awakening* (Nintendo 3DS) natively on
Windows. [3dsrecomp](https://github.com/fearkov/3dsrecomp) converts the
game's code to native code ahead of time, and
[Zakuro](https://github.com/fearkov/zakuro) runs it and handles the
console's graphics, sound and system. 

The repository contains no game files. Everything is built from your own
copy of the game, on your computer.

## What you need

- The European release of the game (title ID `000400000009F100` revision 0), dumped
  from your cartridge as a decrypted `.3ds` or `.cci` file.
- Optionally, the DLC, dumped as a decrypted `.cia` file.

## Playing

1. Download the launcher from the
   [releases page](https://github.com/Zelkin1/fe-awakening-recomp/releases),
   unzip it and run `zakuro.exe`. You can also build it yourself (see below).
2. Click Select game files and choose the game's `.3ds` or `.cci` file. If
   you have the DLC, choose its `.cia` file under Update in Settings, System.
3. Click Build. The launcher converts the game's code in 10 to 20 minutes. If your computer has no C
   compiler, the launcher offers to download one.
4. Click Play.

The keyboard, the mouse (as the touch screen) and most controllers work.
Esc opens the menu, with the settings, the cheats and save states.

## Features

- The 3D is drawn at up to 8 times the console's resolution, and frame
  interpolation can show battles at 60 frames per second.
- A 16:9 mode shows more of the battlefield, the talks and the unit views.
- You can play on the top screen alone. The bottom screen can be displayed as an overlay with the press of a button.
- Battles UI has 3 styles: vanilla style, FE3H style, or no UI. In each
  one, the six battle buttons show when you press any button and fade out
  3 seconds after your last press.
- A mod that adds feet to every characters in the game is implemented.
- The Cheats window has cheat codes and built-in tweaks such as faster
  battles, or "Glow to the edges", which keeps the glow of the 3D scenes
  from fading out along the edges of the screen.
- Original mode, at the top of Settings, Graphics, turns all of this off
  and plays the game as the console does.

## Building

You need Git, Rust and MinGW-w64 gcc. In Git Bash:

```bash
git clone https://github.com/Zelkin1/fe-awakening-recomp
cd fe-awakening-recomp
git clone https://github.com/fearkov/3dsrecomp
git clone https://github.com/fearkov/zakuro
git -C 3dsrecomp checkout 9db0d8b
git -C zakuro checkout 80d78be
git -C 3dsrecomp apply ../patches/3dsrecomp-0002-awakening-hints.patch
git -C zakuro apply ../patches/zakuro-local-changes.patch
git -C zakuro apply ../mods/feet/zakuro-feet.patch
(cd zakuro && cargo build --release -p zakuro)
```

The launcher is then `zakuro/target/release/zakuro.exe`. `tools/env.sh`
puts a Rust toolchain kept in `tools/rust` on PATH, and your MinGW gcc too
when `MINGW` names its folder.

The files the patches add or change are also in `source/`, in their final
form and at the same paths as in Zakuro and 3dsrecomp, for reading the
changes without applying the patches.

A copy of the launcher given to others must include the licenses of the
code it is built from. `tools/licenses.py` collects them:

```bash
(cd zakuro && cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-gnu > ../metadata.json)
python tools/licenses.py metadata.json THIRD_PARTY_LICENSES.txt
```

## Legal

You need your own dump of the game. The project has no decryption and no
keys, and Zakuro refuses a dump that is still encrypted.

The repository and the launcher contain no code, data or assets from the
game. What they hold about it, such as addresses in its code and the names
of its files, is listed in [`LEGAL.md`](LEGAL.md). What the launcher builds
is made from your copy and stays on your computer: the converted code in
`%APPDATA%\3dsrecomp` and the models with added feet in `%APPDATA%\zakuro\mods`. Don't share
either.

The project's own files are under the MIT license ([`LICENSE`](LICENSE)).
Zakuro and 3dsrecomp keep their own licenses
([`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)). Fire Emblem is a
trademark of Nintendo. This project is not affiliated with Nintendo or
Intelligent Systems.
