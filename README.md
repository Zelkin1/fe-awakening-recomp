# Fire Emblem: Awakening Recompilation

This project runs *Fire Emblem: Awakening* (Nintendo 3DS) natively on
Windows and Linux. [3dsrecomp](https://github.com/fearkov/3dsrecomp) converts
the code of the game to native code before the game runs.
[Zakuro](https://github.com/fearkov/zakuro) runs that code and does the work
of the graphics, the sound and the system of the console.

The repository contains no game files. You build everything from your own
copy of the game, on your computer.

## What you need

- The European release of the game (title ID `000400000009F100`, revision 0),
  dumped from your cartridge as a decrypted `.3ds` or `.cci` file.
- The DLC, dumped as a decrypted `.cia` file. The DLC is optional.

## Playing

1. Download the launcher from the
   [releases page](https://github.com/Zelkin1/fe-awakening-recomp/releases).
   On Windows, unzip it and run `zakuro.exe`. On Linux, make the AppImage
   executable and run it. The AppImage needs the ALSA sound library
   (`libasound2` on Debian and Ubuntu, `alsa-lib` on Arch). The Linux version
   is new and needs more testing. You can also build the launcher yourself, as
   the section Building tells.
2. Click Select game files and choose the `.3ds` or `.cci` file of the game.
   If you have the DLC, choose its `.cia` file under DLC in Settings, System.
3. Click Recompile. The launcher converts the code of the game in a few
   minutes to 20 minutes. If your computer has no C compiler, the launcher
   offers to download one.
4. Click Play.

You can install the feet and the HD text from Settings, Mods.

The keyboard, the mouse and most controllers work. The mouse acts as the
touch screen. Esc opens the menu, with the settings, the cheats and the save
states.

## Features

- The launcher draws the 3D at up to 8 times the resolution of the console.
  Frame interpolation can show the battles at 60 frames per second.
- You can play on the top screen alone. A button shows the bottom screen over
  the top screen: the right trigger, or E on the keyboard.
- The battle UI has 3 styles: the style of the original game, the style of
  FE3H, or no UI.
- The HD fonts work in all languages.
- A mod adds feet to every character in the game.
- Cheat codes and built-in tweaks, such as a 16:9 top screen and faster
  battles.
- Original mode plays the game as the console does.

## Recommended texture pack

For HD art across the whole game, install
[Amateur Awakening HD](https://gamebanana.com/mods/400809) by L498. This
texture pack draws the portraits, the cutscenes, the battle maps and the menus
again at 4 times the resolution.

1. Download its three main parts, P1, P2 and P3, from the links under
   Alternate Sources on its page.
2. Unzip all three parts into one folder of its own in the mods folder of the
   game: `%APPDATA%\zakuro\mods\000400000009F100\aahd\textures` on Windows, or
   `~/.local/share/zakuro/mods/000400000009F100/aahd/textures` on Linux.

The pack is for the North American release. Some textures are different in
the European release, and those textures stay as they are.

## Building

On Windows, you need Git, Rust and MinGW-w64 gcc, and you run the commands in
Git Bash. On Linux, you need Git, Rust, gcc or clang, `pkg-config` and the
headers of ALSA and udev (`libasound2-dev` and `libudev-dev` on Debian and
Ubuntu). The commands are the same on both systems:

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
`zakuro/target/release/zakuro` on Linux. On Windows, `tools/env.sh` puts the
Rust toolchain in `tools/rust` on PATH. It also puts your MinGW gcc on PATH
when `MINGW` names its folder.

The folder `source/` holds the files that the patches add or change, in their
final form. They are at the same paths as in Zakuro and 3dsrecomp, so you can
read the changes without the patches. [`PROJECT_CODE.md`](PROJECT_CODE.md)
lists the files that the project adds. For each file of Zakuro and 3dsrecomp
that the project changes, it lists the functions and the types that the
project adds, changes or removes.

If you give a copy of the launcher to other people, the copy must include the
licenses of the code that it is built from. `tools/licenses.py` collects them.
On Linux, the platform is `x86_64-unknown-linux-gnu`:

```bash
(cd zakuro && cargo metadata --format-version 1 --filter-platform x86_64-pc-windows-gnu > ../metadata.json)
python tools/licenses.py metadata.json THIRD_PARTY_LICENSES.txt
```

## Legal

You need your own dump of the game. The project has no decryption and no
keys, and Zakuro refuses a dump that is still encrypted.

The repository and the launcher contain no code, data or assets from the
game. [`LEGAL.md`](LEGAL.md) lists what they hold about the game, such as
addresses in its code and the names of its files. The launcher builds its
files from your copy, and they stay on your computer. The converted code goes
in `%APPDATA%\3dsrecomp` and the mods go in `%APPDATA%\zakuro\mods` on
Windows. On Linux, they go in `~/.local/share/3dsrecomp` and
`~/.local/share/zakuro/mods`. Do not share these files.

The files of the project are under the MIT license ([`LICENSE`](LICENSE)).
Zakuro and 3dsrecomp keep their own licenses
([`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md)). Fire Emblem is a
trademark of Nintendo. This project is not affiliated with Nintendo or
Intelligent Systems.
