# Feet

In *Fire Emblem: Awakening*, the characters' legs end in a point instead of
feet. This mod gives every character, allies and enemies alike, a pair of
simple feet that match their outfit: shoes, armored boots or bare feet. The
feet tilt to rest flat on the ground when a character stands.

The mod contains no game files. The launcher reads the character models
from your own copy of the game and adds the feet on your computer.

## Installing

Click **Build** next to the game in Zakuro's launcher (see the main README).
The feet go into `%APPDATA%\zakuro\mods\000400000009F100\feet\`, and show
the next time the game starts. To remove the mod, delete that `feet` folder.

## Files

- `zakuro-feet.patch`: the change to Zakuro. It adds the feet builder to
  the launcher (`crates/zakuro/src/feet/`), which reads the game's model,
  texture and compression formats, and the hook that keeps the feet on the
  ground while the game runs (`crates/zakuro-core/src/feet.rs`).
- `*.py`: the same builder as Python scripts, for development. They also
  draw preview pictures of the feet. `build_all.py` builds every character,
  `foot_build.py` one.
