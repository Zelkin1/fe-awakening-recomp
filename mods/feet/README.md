# Feet

In *Fire Emblem: Awakening*, the characters' legs end in a point. This mod
gives every character, allies and enemies alike, a pair of simple feet that
match what they wear: shoes, armored boots or bare feet. When a character
stands, the feet tilt to rest flat on the ground.

The mod contains no game files. When you click Build, the launcher reads the
character models from your copy of the game, adds the feet and writes the
models to `%APPDATA%\zakuro\mods\000400000009F100\feet\`. The game uses them
from its next start, and deleting that folder removes them. The feet only
meet the ground on the game's original code, revision 0.

## Files

- `zakuro-feet.patch` is the mod's change to Zakuro, applied after the
  project's own patch. It adds the feet builder to the launcher
  (`crates/zakuro/src/feet/`) and the hook that keeps the feet on the
  ground while the game runs (`crates/zakuro-core/src/feet.rs`).
