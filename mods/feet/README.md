# Feet

In *Fire Emblem: Awakening*, the characters' legs end in a point instead of
feet. This mod gives every character, allies and enemies alike, a pair of
simple feet that match their outfit: shoes, armored boots or bare feet. The
feet tilt to rest flat on the ground when a character stands.

The mod contains no game files. A script reads the character models from
your own copy of the game and adds the feet on your computer.

## Installing

1. Apply `zakuro-feet.patch` to Zakuro (after the project's own patch) and
   build the game, as described in the main README.
2. List your game's files with Zakuro's `romdump` (run in the `zakuro`
   folder):

```bash
cargo run --release -p zakuro-fs --example romdump -- <your game>.cci files > romfs-files.txt
```

3. Build the feet for every character:

```bash
python mods/feet/build_all.py <your game>.cci romfs-files.txt %APPDATA%/zakuro <preview folder>
```

The new models go into `%APPDATA%\zakuro\mods\000400000009F100\feet\`. Start
the game again to see them. To remove the mod, delete that `feet` folder.

## Files

- `build_all.py`: builds the feet for every character, with preview pictures.
- `foot_build.py`: builds the feet for one character.
- `cgfx.py`, `cgfx_model.py`, `cgfx_dict.py`, `textures.py`, `lz11.py`: read
  and write the game's model, texture and compression formats.
- `make_mod.py`, `peek_model.py`: small tools for testing.
- `zakuro-feet.patch`: the change to Zakuro that keeps the feet on the ground.
