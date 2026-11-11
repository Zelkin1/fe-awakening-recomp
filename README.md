# Fire Emblem: Awakening Recompilation

This project runs *Fire Emblem: Awakening* (Nintendo 3DS) natively on
Windows and Linux. [3dsrecomp](https://github.com/fearkov/3dsrecomp) converts the
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
   [releases page](https://github.com/Zelkin1/fe-awakening-recomp/releases).
   On Windows, unzip it and run `zakuro.exe`. On Linux, make the AppImage
   executable and run it; it needs the ALSA sound library (`libasound2` on
   Debian and Ubuntu, `alsa-lib` on Arch). The Linux version is new and still
   needs testing. You can also build it yourself (see below).
2. Click Select game files and choose the game's `.3ds` or `.cci` file. If
   you have the DLC, choose its `.cia` file under DLC in Settings, System.
3. Click Recompile. The launcher converts the game's code in a few minutes to 20 minutes. If your computer has no C
   compiler, the launcher offers to download one.
4. Click Play.

The feet and the HD text are installed from Settings, Mods.

The keyboard, the mouse (as the touch screen) and most controllers work.
Esc opens the menu, with the settings, the cheats and save states.

## Recommended texture pack

For HD art across the whole game, install
[Amateur Awakening HD](https://gamebanana.com/mods/400809) by L498, a
texture pack that redraws the portraits, cutscenes, battle maps and menus at
4 times the resolution.

1. Download its three main parts, P1, P2 and P3, from the links under
   Alternate Sources on its page.
2. Unzip all three into a folder of its own in the game's mods folder:
   `%APPDATA%\zakuro\mods\000400000009F100\aahd\textures` on Windows, or
   `~/.local/share/zakuro/mods/000400000009F100/aahd/textures` on Linux.
3. Keep Texture packs on in Settings, Graphics, and play at 4x or above.

The pack was made for the North American release, so a few textures with
regional differences stay as they are on the European one.

## Features

- The 3D is drawn at up to 8 times the console's resolution, and frame
  interpolation can show battles at 60 frames per second.
- You can play on the top screen alone. The bottom screen can be displayed as an overlay with the press of a button
  (the right trigger, or E on the keyboard).
- Battles UI has 3 styles: vanilla style, FE3H style, or no UI.
- HD fonts in all languages (still WIP)
- A mod that adds feet to every characters in the game is implemented.
- Cheat codes and built-in tweaks such as top screen 16:9 aspect ratio and faster
  battles.
- Original mode plays the game as the console does.

## Building

On Windows you need Git, Rust and MinGW-w64 gcc, and the commands run in Git
Bash. On Linux you need Git, Rust, gcc or clang, `pkg-config` and the ALSA and
udev headers (`libasound2-dev` and `libudev-dev` on Debian and Ubuntu). The
commands are the same:

```bash
git clone https://github.com/Zelkin1/fe-awakening-recomp
cd fe-awakening-recomp
git clone https://github.com/fearkov/3dsrecomp
git clone https://github.com/fearkov/zakuro
git -C 3dsrecomp checkout 9db0d8b
git -C zakuro checkout 80d78be
git -C 3dsrecomp apply ../patches/3dsrecomp-0002-awakening-hints.patch
git -C 3dsrecomp apply ../patches/3dsrecomp-0003-build-fixes.patch
git -C zakuro apply ../patches/zakuro-local-changes.patch
git -C zakuro apply ../mods/feet/zakuro-feet.patch
(cd zakuro && cargo build --release -p zakuro)
```

The launcher is then `zakuro/target/release/zakuro.exe` on Windows and
`zakuro/target/release/zakuro` on Linux. On Windows, `tools/env.sh` puts a
Rust toolchain kept in `tools/rust` on PATH, and your MinGW gcc too when
`MINGW` names its folder.

The files the patches add or change are also in `source/`, in their final
form and at the same paths as in Zakuro and 3dsrecomp, for reading the
changes without applying the patches. [`PROJECT_CODE.md`](PROJECT_CODE.md)
lists the files the project adds and, for each Zakuro and 3dsrecomp file it
changes, the functions and types it adds, changes or removes.

A copy of the launcher given to others must include the licenses of the
code it is built from. `tools/licenses.py` collects them (on Linux, the
platform is `x86_64-unknown-linux-gnu`):

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
`%APPDATA%\3dsrecomp` and the mods in `%APPDATA%\zakuro\mods` on Windows, or
`~/.local/share/3dsrecomp` and `~/.local/share/zakuro/mods` on Linux. Don't
share either.

The project's own files are under the MIT license ([`LICENSE`](LICENSE)).
Zakuro and 3dsrecomp keep their own licenses
([`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)). Fire Emblem is a
trademark of Nintendo. This project is not affiliated with Nintendo or
Intelligent Systems.
