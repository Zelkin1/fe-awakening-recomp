# Feet

In *Fire Emblem: Awakening*, the legs of the characters end in a point. This
mod gives every character, allies and enemies, a pair of simple feet. The feet
match what the character wears: shoes, armored boots or bare feet. When a
character stands, the feet tilt to rest flat on the ground.

The mod contains no game files. When you click Build, the launcher reads the
character models from your copy of the game and adds the feet. Then it writes
the models to `%APPDATA%\zakuro\mods\000400000009F100\feet\`. The game uses
them from its next start. If you delete that folder, the feet go away. The feet
touch the ground only with the original code of the game, revision 0.

## Files

- `zakuro-feet.patch` is the change that the mod makes to Zakuro. You apply it
  after the patch of the project. It adds the feet builder to the launcher
  (`crates/zakuro/src/feet/`). It also adds the hook that keeps the feet on
  the ground while the game runs (`crates/zakuro-core/src/feet.rs`).
