# Fire Emblem: Awakening — static recompilation

A native Windows build of *Fire Emblem: Awakening* (3DS, Europe, `CTR-P-AFEP`,
title ID `000400000009F100`), made with the
[3dsrecomp](https://github.com/fearkov/3dsrecomp) method. The game's ARM11 code
is translated ahead of time to C, compiled with gcc, and linked into
[Zakuro](https://github.com/fearkov/zakuro). Zakuro is the HLE runtime: kernel,
services, PICA200 GPU (Vulkan), and DSP audio. It interprets any code the
recompiler didn't reach.

You need your own decrypted dump of the game's cartridge, in the folder above
this one. The game reads its assets, its save, and everything except the code
from it. The project's own files hold no game code or data; what is built
from your dump (`_work/`, `FireEmblemAwakeningRecomp/`, `_runs/`) stays on
your computer. `LEGAL.md` lists which files are which and the trademarks,
`THIRD_PARTY_NOTICES.md` the licenses of Zakuro and 3dsrecomp. The project's
own files are under the MIT license, in `LICENSE`.

## Playing

Run `Play-FireEmblemAwakening.bat` for the console's two screens. For one
widescreen screen, run `Play-FireEmblemAwakening-16x9.bat`.

The widescreen launcher shows the top screen alone, at 16:9, and turns on
the game's **16:9** enhancement:

- **What it changes:** the 3D (maps, battles, the avatar editor) is drawn
  wider, so you see more at the sides at the right proportions.
- **What it doesn't:** the 2D on the top screen, such as menus and text
  boxes, is stretched by about 7%. The world map isn't widened.
- **The bottom screen over the top one:** with the top screen alone, the
  bottom screen, where the game keeps a unit's stats and items, shows only
  when you ask for it.
  - **The key:** **E** on the keyboard, or a click of the right stick,
    opens it while the map's cursor is on a unit, yours or an enemy, or a
    unit is chosen, and in the 3D view of a unit opened from its portrait,
    and again closes it. Elsewhere it does nothing. Both are set in
    Settings, in the Keybinds and Controller tabs ("Bottom screen over the
    top one"). Opening the 3D view closes it.
  - **Where:** in the middle of the top screen, 45% of its width, its own
    4:3 shape, with rounded corners and a soft dark shadow that follows
    them, as the game softens the unit window's edges.
  - **Inputs:** while it's open, the game gets no buttons and no sticks, so
    the map's cursor stays put, but for **L** (Q on the keyboard), which goes
    on to the next unit, so the bottom screen shows one unit after another.
    The mouse touches it.
  - **Where it doesn't open:** over a battle forecast (in detail or brief),
    over a battle, and while someone talks (the shade over the whole map, or
    the name plate over a dialogue box); there the key does nothing. A
    dialogue closes it.
- **The world map:** with the top screen alone, the place's name shows at
  the top middle in its black panel, at 70% of its size, in the panel's own
  outline and the place's own icon, with a short soft shadow; it fades in
  and out. The same key opens the whole bottom screen over the top one in
  the middle, rounded and shadowed the same way, the name taken away
  meanwhile, and closes it again. While it's open the game gets no buttons
  and no sticks, so the character stays put; the mouse touches it.
- **Battles:** with the top screen alone, what shows over a battle is
  chosen in Settings, Graphics, "Battles": the game's panels (below),
  banners, or nothing at all.
  - **Banners:** for each fighter, in the bottom corner on their side, the
    name on a banner with diamond ends, the HP number in a diamond by a long
    bar as full as the game's gauge, the weapon on a band, and Atk, Hit and
    Crit as tabs over boxes, in that order on both sides, each with a faint
    grain and a soft shadow. The text is the game's own, taken off the
    ground it lies on, and every shape is painted in that ground's colour.
    The game's six buttons stay small in the bottom middle. They show, fade
    and follow the game's shaking as the panels do.
- **The game's panels:** the bottom screen's panels show over the 3D along
  the bottom with the game's smooth edges. Each
  fighter's Atq, Prc and Crit table, with its own copy of the labels and a
  beige frame, a little dimmed, sits in the bottom corner on their side.
  Their name banner and HP panel touch it toward the middle, at the same
  size so their text is as sharp, as tall as the table so their tops and
  bottoms line up, with a rim on both sides, square where it meets the
  table. The portraits are left out. The six buttons, each on its own,
  are at 28% in the bottom middle, a little off the bottom, and a click
  presses them. Each panel has a short soft shadow around it; the buttons
  have none.
  They fade in as the battle starts, and fade out when the EXP bar comes
  up at its end, staying away until the map is back. When a fighter is
  hit, the game shakes their panel; the panel over the 3D follows it so it
  stays still and whole. The rest of the scene stays clear. They hide while
  someone in the battle talks. The panels are found by their place, not
  their text, so any language works.
- **The title screen:** with the top screen alone, its prompt and the
  copyright under it show at the bottom of the title, their light added
  onto it, without the bottom screen's dark ground.
- **Original mode** (Settings, System): plays the game as the console
  does: both screens, the top one 5:3, at the console's resolution with no
  smoothing and no frame interpolation, without mods, codes, enhancements
  or anything shown over the game's screens. Your own choices come back
  when you turn it off. Mods, the feet included, change the next time the
  game starts; everything else changes right away.
- **The other screen:** **F10** flips to the bottom screen and back. F9
  goes round all the layouts.
- **Where the settings live:** the layout and shape are per run. They're
  saved into Zakuro's settings only when something in the game saves
  them, F10 included.

Built-in mods are in Zakuro's Mods window (Esc), above your own codes, which
work as they do on the console. **16:9** is the
widescreen code above. Turning it on mid-scene takes effect from the next
scene load; the widescreen launcher turns it on from boot. **Smooth map
sprites** keeps the units' sprites and HP bars on the map where the camera
puts them: the game cuts their place on the screen to whole pixels, so at a
higher resolution they jitter against the map as it scrolls. Turn it on for
anything above 1×; at 1× the sprites can look a little soft. **Faster
battles** plays battle animations faster than usual, at the speed set by the
slider under it while it's on, from 100% to 200%, 130% to start with; a new
speed takes from the next battle. The battle's fast forward (▶▶) stays at
200% and its pause still stops them. Each of the three is its own toggle.

The resolution (Settings, Graphics) can be changed while you play. It takes
effect at once, after a short pause while the GPU redraws at the new size.
Frame interpolation (Settings, Graphics) draws a picture between each two the
game draws in scenes it draws at 30 a second, and can be switched on and off
while you play too. The frame rate in the corner (Settings, Graphics, "Show
the frame rate") is the pictures the game draws a second, 30 in battles and
cinematics, 60 on the map, as `60 FPS`. While interpolation draws pictures in
between it reads `30/60 FPS`, then the ones the top screen shows.

The game is chosen in Settings, System: its file, and its update's when
there is one. DLC beside the game's file goes with it.

- **DLC:** put your decrypted DLC dump (a CIA) beside the game's file;
  there's nothing to install. The DLC maps are at the Outrealm Gate on the
  world map, which opens after Chapter 4. A CIA whose contents are still
  encrypted inside is left out and listed as "Encrypted, Zakuro needs a
  decrypted dump".
- **Saves:** go to `userdata\`, next to the batch file.
- **Settings:** Zakuro always keeps window and controls settings in
  `%APPDATA%\zakuro\settings.toml`.
- **Keys:** Esc opens Zakuro's menu (settings, controls, cheats), F1 pauses,
  and F11 toggles fullscreen.

Default keys:

| 3DS | Keyboard |
|---|---|
| A / B / X / Y | X / Z / S / A |
| L / R | Q / W |
| Start / Select | Enter / Backspace |
| D-pad | Arrow keys |
| Circle pad | I / J / K / L |
| Touch screen | Mouse |

Controllers work too: Xbox, PlayStation, Switch Pro, and others.

## Feet

The feet mod, in `mods\feet\`, gives the game's bodies feet of their own,
built on your computer from your dump, and a Zakuro hook keeps them on the
floor. Its own README says how it works and how to build and install it.

## Layout

| Path | What it is |
|---|---|
| `3dsrecomp\` | Recompiler checkout (fearkov/3dsrecomp @ `9db0d8b`), with `patches\3dsrecomp-*.patch` applied |
| `zakuro\` | Runtime checkout (fearkov/zakuro @ `80d78be`, v0.2.30), with `patches\zakuro-local-changes.patch` and `mods\feet\zakuro-feet.patch` applied |
| `FireEmblemAwakeningRecomp\` | Made by `3dsrecomp port` from your dump: `code\` (generated C, `librecomp.a`), the Cargo project, and `target\release\fe-awakening.exe` |
| `patches\` | This project's changes to the two checkouts, as `git diff` files |
| `mods\feet\` | The feet mod: its builder, its Zakuro patch and its README |
| `tools\env.sh` | Puts the local Rust and MinGW gcc on PATH (Git Bash) |
| `tools\boot.sh` | Headless scripted runs in Zakuro's `boot` harness |
| `tools\port-headless.sh` | Headless smoke test of the built `.exe` |
| `tools\hostprof.sh`, `tools\sample_profile.py` | A sampling profile of the harness running a stretch of the game |
| `tools\extract_code.py`, `tools\ips_to_codes.py` | The game's `code.bin` from your dump, and a changed `code.bin` as code lines |
| `tools\memfind.py`, `tools\segment_rate.py`, `tools\ppm2png.py` | Searching a memory dump, instructions per frame, the harness's screens as PNG |
| `tools\rust\` | Local rustup install (Rust 1.99, `x86_64-pc-windows-gnu`) |
| `_work\` | The game's code extracted from your dump, and memory dumps |
| `_runs\` | Test runs: logs, PNG screens, throwaway saves, save states, memory, texture and uniform dumps, mod builds |
| `LEGAL.md`, `THIRD_PARTY_NOTICES.md` | Which files hold no game content and which are made from the game, trademarks, and the licenses of Zakuro and 3dsrecomp |
| `LICENSE` | The MIT license of the project's own files |
| `.gitignore` | Lets only the project's own files into a repository |

A repository of the project holds only the files `.gitignore` lets in: the
README, the legal files, the launchers, `patches\`, the scripts in
`tools\` and the feet mod in `mods\feet\`, as `LEGAL.md` lists them.
Everything else stays on your computer.

gcc 16 and binutils come from a portable MSYS2: `tools\env.sh` looks for
them in `..\..\FE7\CLAUDE\tools\msys64\mingw64\bin`, or wherever `MINGW=`
says.

## Changes to Zakuro

All of them are in one patch against upstream `80d78be`,
`patches/zakuro-local-changes.patch`. Apply it with
`git apply ../patches/zakuro-local-changes.patch` in `zakuro\`. The feet
mod's hook is a patch of its own over it, `mods/feet/zakuro-feet.patch`,
described in that mod's README.

- **JIT stack probes** (`zakuro-gpu/src/shader/jit.rs`): Zakuro compiles
  PICA200 vertex shaders to x86-64 with Cranelift, which emits no stack
  probes by default. Windows commits a thread's stack one page at a time
  behind a guard page, and some of Awakening's vertex shaders have a frame
  bigger than a page. Inline probes keep them from jumping over it.
- **A faster hasher on the write path** (`zakuro-common/src/hash.rs`,
  `zakuro-core/src/memory`): zakuro-gpu's FxHash-style hasher, moved to
  `zakuro-common`, hashes the idle-loop detector's write log, which keeps its
  allocation between watches.
- **Awakening's 16:9 enhancement** (`zakuro-core/src/enhancements.*`): the
  game's camera projection builder divides by the top screen's 5:3 at
  `0x002FE310`; the enhancement sends that to a routine at `0x0045AF00`
  that uses 16:9 when the aspect is 5:3, and sets the map camera's own 5:3
  at `0x003D57F4`. The enhancement list also takes an optional "where it
  came from" field.
- **Per-run command-line options** (`zakuro/src/cli.rs`): `--layout`,
  `--top-shape` and `--enhance`. The `--enhance` names are kept under
  `zakuro_core::ANY_GAME` for whatever game runs.
- **Resolution scaling for Awakening's post-process**
  (`zakuro-gpu/src/raster/hardware.rs`, `raster.rs`, `lib.rs`). The game
  copies its frame into a mipmapped texture and draws it back over the
  screen; every step stays at scale:
  - **TextureCopy:** whole tile rows of a GPU-held buffer are copied on the
    GPU at scale.
  - **Leading rows:** a drawn texture may start rows before its GPU
    surface, with those rows taken from memory.
  - **Mipmapped textures:** drawn textures keep their mip levels. Level 0
    comes from the GPU at scale, the smaller levels from memory.
- **Changing the resolution while a game runs:**
  - `Hardware::rescale` writes everything back, waits for the device, and
    drops what was made at the old scale.
  - It's exposed as `Gpu::set_resolution` and `System::set_resolution`.
  - The frontend applies the Resolution slider at once.
  - `ZAKURO_RESOLUTION_AT=frame:scale` in the boot harness tests it.
- **DLC still encrypted inside is refused** (`zakuro-fs/src/lib.rs`
  `Dlc::load`, `zakuro/src/library.rs` `addon`). Each content's RomFS must
  read, whatever the TMD's flags say; a dump that lacks the NCCH's
  "decrypted" flag but is decrypted is taken. The library lists a refused
  one with the reason, and the game starts with only the readable DLC.
- **The PICA's scissor test** (`zakuro-gpu/src/raster.rs` `scissor`,
  registers 0x065–0x067). Awakening clips its dialogue text with it while a
  line scrolls up. Mode 3, drawing only inside the rectangle, narrows the
  draw's bounds, which both renderers keep to. Mode 1, drawing only outside
  it, isn't followed and is logged once.
- **Pictures shown texel for pixel keep their edges when upscaled**
  (`zakuro-gpu/src/raster.rs` `blit_rect`, `extend_edges`). Awakening's
  videos are a 400×240 frame in a 512×256 texture, drawn 1:1; upscaled,
  the last rows and columns would blend in the padding past the frame. For
  draws of up to 6 vertices that map part of a texture 1:1 (on the CPU,
  checked against the shaded vertices), the texels just past that part
  become copies of its edges in the texture the GPU gets.
- **A frame rate of the pictures shown** (`zakuro-core/src/lib.rs`
  `System::pictures`, `zakuro/src/lib.rs`, `menus.rs` `frame_rate`). The
  corner counts the top screen's buffer swaps and interpolation's pictures
  in between, and the boot harness prints both. It reads `30/60`, the
  game's pictures, then the ones shown, while interpolation draws in
  between, and the game's alone otherwise.
- **Blinks in between** (`zakuro-gpu/src/interpolation.rs` `toggles`). For
  the one frame a blink lasts, the game steps a part of a 3D face a quarter
  out and back, a move too small to count as a jump. A component placing the
  vertices that steps between sixty-fourths from standing still, or straight
  back, stays as the frame has it, so the picture in between doesn't draw
  it half out.
- **Smooth map sprites** (`zakuro-core/src/enhancements.txt`). The game
  projects a unit to the screen, cuts the place to whole pixels
  (`0x003D1AFC`–`0x003D1B24`, a `vcvt` round trip each for x and y), and
  builds the sprite's matrix from that, while the map moves in fractions.
  The enhancement makes the two `vstr`s that write the cut place over the
  exact one (`0x003D1B18`, `0x003D1B24`) no-ops.
- **Faster battles** (`zakuro-core/src/enhancements.txt`).
  - **The game's speed control:** a battle keeps a speed object: its current
    playback speed at +0x10, which the animations read (`0x002FB634` among
    others) and copy, its target speed at +0x20 and its mode at +0x24, 0
    usual, 1 slow, 2 paused, 3 fast. The function at `0x001D612C`, run every
    frame, steps from the old mode to the new: leaving a mode undoes it,
    entering one applies it, through two five-entry jump tables
    (`0x001D6164` for leaving, `0x001D61B0` for entering, by the mode plus
    one). Slow multiplies the target by the literal 0.5 at `0x001D637C` and
    fast by 2.0 at `0x001D6378`; the pause adds 1000 to a count at +0x1C
    that holds the current speed at 0. The usual mode has no factor of its
    own.
  - **The patch:** points the usual mode's two entries (`0x001D6168`,
    `0x001D61B4`) at two routines past the end of the text, at
    `0x0045AF40` and `0x0045AF54`, after the widescreen one: load the
    target, multiply it by the inverse of the speed on leaving or the speed
    on entering, store it, branch back. Fast forward reaches 1.0 × 2 = 2.0
    and slow 0.5 as usual. A battle's speed object starts by entering the
    usual mode, so it starts at the set speed.
  - **The setting:** an enhancement can declare one on a line after its
    own, `~ Speed 100..200 = 130 %` (`enhancements::Setting`); its code
    says `{x/100}` and `{100/x}` where the value over 100 and 100 over it go,
    as 32-bit floats (`code_at`), here the two literals at `0x0045AF70` and
    `0x0045AF74`. The Mods window shows a slider under it while it's on;
    moving it makes the code again at the new value (`Cheat::set_lines`,
    the addresses the same, so turning it off still puts back what it
    first overwrote) and saves it with the settings
    (`enhancement_values`, by title and name, passed to the core as
    `Config::enhancement_values`).
- **Mods in folders of their own** (`zakuro-core/src/mods.rs` `folders`,
  `zakuro-fs` `Layered::stacked`): besides the title's folder itself,
  `mods\<title id>\`, each folder in it other than a mod's parts (`romfs`,
  `romfs_ext`, `exefs`, `textures`) is a mod of its own, laid over the game
  after it in the order of their names. A later mod's file takes the place
  of an earlier one's and its patch goes over it; the last `code.bin` and
  `exheader.bin` win, and every mod's `code.ips` and `code.bps` apply in
  turn. The texture pack stays the title folder's. Taking a mod's folder out
  takes it out alone.
- **The bottom screen over the top one** (`zakuro-gpu/src/backend/mod.rs`
  `ScreenLayout::BottomOverTop`, `zakuro/src/lib.rs` `place_bottom_over`,
  `toggle_bottom_over`, `menus.rs` `bottom_over`). It's the top-only layout
  with the bottom screen drawn after the top one by the frontend's overlay,
  so the mouse touches it the same way.
  - **When:** while the key (E, or the right stick's click, rebindable)
    has it open, in the frontend's `Scene::Plain`, `Scene::WorldMap` or
    `Scene::UnitView`, never in original mode. On a map the key opens it
    only while `System::cursor_on_unit` or `unit_selected` says so
    (`toggle_bottom_over`). `Scene` (`Plain`, `WorldMap`, `UnitView`,
    `Forecast`, `Pieces`) comes from the markers each frame; a change of
    scene, or dialogue (`System::talking`), undoes the key, so over a
    forecast, a battle or dialogue the key does nothing, and the bottom
    screen never comes back by itself. While it shows, `emulate` sends the
    game only the mouse's touches and, on a map or in a unit's 3D view, L,
    which moves the cursor to the next unit.
  - **A unit's 3D view:** the `UNIT_VIEW` marker is the portrait of the
    unit's profile on the bottom screen, a 256×256 ETC1A4 quad from x -0.15
    and 0.8 of the screen across, 1.07 of it down, which no other screen
    draws (`System::unit_view`).
  - **Where:** `width` ten-thousandths of the top screen's width across,
    4:3, no taller than the top screen, at `at`: `Over::Corner`, against its
    right edge `margin` ten-thousandths of its height off its bottom;
    `Over::Side`, `margin` off its right edge and halfway down;
    `Over::Middle`, in its middle. The margins are the game's,
    `System::bottom_over_place` from `BOTTOM_OVER` in `loader.rs`. The
    key's bottom screen goes in the middle everywhere. The two, with their
    shadow, take `BOTTOM_OVER_WIDTH`, 45% of the top screen's width.
  - **How it's drawn:** the presenters leave the bottom screen out of
    `BottomOverTop` (`place`), and `menus::bottom_over` paints it as an
    egui `RectShape` textured with the bottom screen (see the battle's
    panels below), corners 6 of its pixels round (`BOTTOM_OVER_CORNER`),
    over an egui `Shadow` of the same rounding, blurred across 1.5 shadow
    widths from half a width out, so it is dark at the screen's edge and
    gone a width past it, round at the corners.
  - **The window's edge:** screens that reach it touch it
    (`Viewport::pixels`, `vulkan.rs`, `gl.rs`): each edge of a screen is
    rounded to whole pixels on its own.
- **The world map's place name** (`loader.rs` `WORLD_PIECES`,
  `System::world_pieces`, `zakuro-gpu` `Rounded::outline`, `Rounded::cut`,
  `cut_out`, `menus::pieces`). While the world map shows and nobody talks,
  the top-only layout is `TopWithPieces` with the name's panel, at 70%, 2
  pixels off the top, in the middle.
  - **The frame** is drawn within its outline, a polygon traced from the
    game's frame in tenths of a pixel (`PLACE_FRAME`), as an egui mesh fanned
    from its middle with a ring a pixel wide fading out past its edge
    (`outline_mesh`). Its shadow comes from rounded rectangles of its body
    and caps, the shapes sharing the outline filling it once.
  - **The icon** above the frame differs from place to place, so that part
    is cut out of the dark reddish ground around it (`Ground`: red over
    green by more than 3, red, green and blue under 90 together): a fill
    from its top and sides through ground pixels of the bottom screen's
    picture at the console's resolution, what is left drawn as one quad a
    run of pixels, its shadow its runs' box.
  - **That picture** is the bottom screen's capture at 1× each display
    transfer already makes, kept as its batch finishes once it was asked
    for (`Hardware::native_picture`, `System::bottom_picture`), so a frame
    or two behind and never waited for; memory's in software.
- **Battle banners** (`zakuro/src/banners.rs`, `menus::banners`,
  `loader.rs` `BANNERS`, `BATTLE_BUTTONS`, `System::battle_banners`,
  `settings::BattleUi`). `Scene::Battle` shows the game's pieces, the
  buttons alone with the banners drawn by the frontend, or nothing.
  - **What they show** comes from the bottom screen's picture at the
    console's resolution (`System::bottom_picture`, see the place name):
    each fighter's name, HP number, weapon and values, and the labels, are
    rectangles moved as the game shakes the panels (the banner markers'
    spans), each cut off its ground: the ground is the commonest colour of
    the rectangle, a pixel's opacity how far it is from it against the
    farthest one, its colour what is left once the ground's share is taken
    out, trimmed to what shows. Drawn over the ground's colour that gives
    back the game's pixel; the GPU smooths it bigger as the game's text is.
  - **The HP bar** is as full as the gauge's lit 3-pixel segments in its
    brightest row against all segments within 6 rows of it.
  - **Drawing:** egui convex polygons in the mockup's measures of a
    1080-high window at 85%, the text as textures given new pixels only
    when it changes (`BannerTextures`), a 160×24 noise texture of the
    frontend's own stretched over each shape.
- **Original mode** (`settings.rs` `Settings::set_original`, `Kept`,
  `zakuro-core` `Config::original`, `System::set_original`,
  `System::cheats_held`): the settings put the screens, the top screen's
  shape, frame interpolation, the resolution and the filter aside and use
  the console's, given back when it's off; the core lays no mods, no
  texture pack and no code changes over the game at its start, and holds
  every cheat and enhancement back (`cheats::run` runs none and puts back
  what enhancements overwrote) while keeping whether each is on. The
  frontend shows nothing over the screens and the key does nothing.
- **Following a game's windows** (`zakuro-gpu/src/raster.rs` `Marker`,
  `Place`, `marker_drawn`, `zakuro-core/src/loader.rs` `MARKERS`,
  `System::cursor_on_unit`).
  - **What's followed:** each marker has an id. `ON_UNIT` is the window
    drawn only while the cursor is on a unit.
  - **How it's found:** a draw on the top screen's buffer is known by
    texture unit 0's format and size, the FNV-1a hash of the texture's
    bytes when it's the same in every language, and the place the draw
    lands on the screen when that's what tells it apart. Draws of up to 64
    vertices with a marker's kind of texture have their vertices shaded on
    the CPU for this.
  - **On a unit:** Awakening's unit window is 30 vertices from a 256×256
    ETC1A4 atlas of the interface, at x 256–400, y 4–44 of 400×240. The
    atlas holds the language's text, and the terrain window at the top left
    is drawn from it too, so the place tells them apart: left edge 55–98% of
    the width, top within 6%, 30–42% of the width across, 12–21% of the
    height tall.
  - **The harness:** it prints "the cursor on a unit" at each dump while
    the window is up, and "a unit chosen", "a dialogue", "the world map",
    "a forecast", "a brief forecast", "a battle" and "the title screen" for
    the markers below.
  - **The other markers:** `Marker::bottom` puts a marker on the bottom
    screen's buffer (240×320) instead of the top one's.
    - `TALKING`: the untextured shade over the whole map, and the dialogue
      box's name plate (256×512 ETC1A4, 18 vertices, 30–35% of the width
      across, 12–16% tall, anywhere). The shade is exactly the screen, its
      edges within 0.005; the tint the map lays over itself while it shows
      the enemies' range is untextured too but runs past the screen's edges,
      so it doesn't count.
    - `WORLD_MAP`: the help bar along the world map's bottom (256×512 RGBA4).
    - `FORECAST`: the detailed forecast window's background, 256×512 ETC1A4
      and 48 vertices across the whole top screen, 70–85% of its height.
    - `BRIEF_FORECAST`: the brief forecast's panel along the top screen's
      bottom, 256×512 ETC1A4 and 30 vertices, tops of 67–69.5% and heights
      of 31–33.5%, which leaves out the map's dialogue box from the same
      atlas.
    - `BATTLE`: the panel behind the fighters' HP and stats on the bottom
      screen, 256×512 ETC1A4, all across it, 50–65% of its height, drawn
      only in a battle. Its place takes in the hit shake (left within 10%,
      top 0–14%).
    - `TITLE`: on the title screen's bottom screen, the prompt to press a
      button (IA8 256×32, x 10–90%, y 43–57%), or the copyright under it
      (L8 512×32, wider than the screen, from y 87%), whichever shows.
    - `EXP`: the bar a battle's experience fills at its end, a 256×512
      ETC1A4 pill of 18 vertices at x 22–78%, y 43–57% of the top screen,
      matched once it has opened to four tenths across.
    - `LEFT_BANNER`, `RIGHT_BANNER`: the fighters' name banners on a
      battle's bottom screen, 256×512 ETC1A4 and 18 vertices, at rest x 24
      and 184, y 4. Markers with a place report where they landed
      (`take_spans`, `System::marker_spans`). In Awakening's battles a draw
      shows on the screen three frames after it was made, so `marker_spans`
      gives the places drawn `SPAN_LAG` frames ago, those of the picture on
      screen.
- **A battle's panels over the 3D, the title screen's texts**
  (`zakuro-gpu/src/backend/mod.rs` `ScreenLayout::TopWithPieces`, `Piece`,
  `Rounded`, `piece_places`, `zakuro-core/src/loader.rs` `PIECES`,
  `TITLE_PIECES`, `System::battle_pieces`, `title_pieces`).
  - **A piece:** a box of the bottom screen, the anchor it goes to across
    and down the top screen, its size in hundredths, and the rounded
    rectangles of the bottom screen in it that show (`Rounded`: a source
    rectangle, a `shift` from where it is drawn and whether `mirror`ed left
    to right, its four corners' radii, how soft its edge is, `light`, see
    below, and a `shade` it is drawn at, out of 255). Each side of a
    battle is two pieces at 50% against the bottom edge:
    - **The table:** in the corner, values x 72–144 or 175–247 and the
      labels between at 145–174, y 91–137, with 6 rows of the stats
      panel's beige above and below and 4 columns on its outer side, short
      of the portraits, so it is 59 rows like the HP body; at shade 205.
      It's drawn at the HP panel's size, so its text is as sharp.
    - **The HP panel:** touching it, 53 top-screen pixels in from the side,
      half a pixel over the table so no seam shows. Its body is cut at y 76:
      below that the stats panel's notch and the portraits reach into it.
      The game draws the body's rim (a light line over a dark one) only on
      its inner side, the other being the screen's edge, so that side gets
      a mirrored copy of the inner one: the 5 columns past the HP gauge,
      drawn over the body's first column, where both are plain fill. Its
      top 7 rows are the straight rim from y 24 again, so the corner is
      square, and the body's top highlight is drawn over that corner.
    - **The shadow:** the bodies, banners and tables each cast a short
      shadow (`Rounded::shadow`), black at 30% over 1.5 top-screen pixels
      (`PIECE_SHADOW`), with the same falloff as the bottom screen's brown
      one over the top one (`menus::shadow`). All the shadows are drawn
      before any part, so none darkens a part next to it; the rim and
      highlight pieces cast none, so the joins aren't darkened twice.
    The six buttons are one piece at 28%, 4 pixels off the bottom, each
    button its own. On the 1× bottom screen: bodies x 0–158 and 162–320, y
    17–76, corners 6; banners 112×22 from y 6, corners 8; buttons 42–43 px
    from y 191, corners 4.
  - **The title screen:** two pieces at full size in the bottom middle, the
    prompt (its ornaments x 43–275, y 113–126) and the copyright (x
    20–297, y 218–230), each `light` with a soft edge: drawn white with no
    alpha, premultiplied, which the overlay's blending adds onto the top
    screen, so the bottom screen's nearly black ground adds next to nothing
    and no shadow shows around the text.
  - **How they're drawn:** by the frontend's overlay, not the presenter
    (`menus::pieces`): each is an egui `RectShape` with the bottom screen as
    its texture, `gui::BOTTOM_SCREEN` (egui's `TextureId::User(0)`, the
    overlay's `BOTTOM_SCREEN_TEXTURE`), so egui's tessellator rounds the
    corners and feathers the edges by a pixel. The presenters bind their
    own bottom-screen picture for that texture: Vulkan its screen
    descriptor set, whose layout matches the overlay's, with the
    coordinates moved to where the screen lies in its image (`upload`'s
    `screen`); OpenGL its bottom texture. `TopWithPieces` draws only the
    top screen in the presenters. The overlay's shader multiplies by
    white, so the colours match the screen's on a UNORM swapchain (the one
    Zakuro picks); on an sRGB-only surface they would come out darker.
  - **When:** `battle_pieces` while the `BATTLE` marker is recent and
    nobody talks, and not from the `EXP` marker on until `BATTLE` is no
    longer recent (`System::battle_over`); `title_pieces` while the `TITLE`
    one is. The frontend fades them in and out over 0.2 s
    (`PIECE_FADE`, egui's `animate_bool_with_time`, asked every frame so
    the first appearance fades too), drawing the last ones shown
    (`last_pieces`) while they fade out; `menus::pieces` scales every
    part's and shadow's colour by the opacity.
  - **Shaken panels:** a piece can follow a marker (`Piece::follows`: its
    id and its draw's top left at rest). `piece_places` cuts the piece
    where the draw is now, so the panel the game shakes stays still over
    the top screen. The HP pieces follow the banners. The portrait, which
    the game draws over the HP panel, isn't followed: shaken down, the
    panel's bottom slides under it for a frame. The tables' top margin is
    copied from the beige under them, where a shaken HP body doesn't reach.
- **The menus in the game's look** (`zakuro/src/theme.rs`, `gui.rs`
  `context`, `menus.rs`).
  - **The look:** parchment windows in a bronze frame, a dark band with
    the title in gold, and ivory command bars that turn coral with a white
    glove pointing at the one chosen, in the colours of the game's map
    menu, unit list and tutorial windows.
  - **The text:** Cambria Bold, read from Windows' fonts; without it,
    egui's own.
  - **Settings:** a tab for each kind of setting (System, Graphics,
    Emulation, Sound, Interface, Keybinds, Controller) under the header.
    Each tab keeps its own scroll, and the window keeps its height from tab
    to tab where the screen has room.
  - **Looking at them without a window:** `snapshot_the_menus` draws the
    Paused menu and some of the Settings tabs to PNGs with a small software
    rasterizer. It's ignored by default:
    `ZAKURO_SNAPSHOT=<folder> cargo test --release -p zakuro --lib snapshot -- --ignored`.
- **Memory tools in the boot harness:** `ZAKURO_FIND` and
  `ZAKURO_DUMP_MEM` list or dump the game's memory at a frame
  (`tools/memfind.py`).
- **A draw's uniforms** (`zakuro-gpu/src/lib.rs` `dump_uniforms`):
  `ZAKURO_DUMP_UNIFORMS=<count>:<file>` adds the vertex shader's float
  uniforms at each draw of that many vertices to the file. Awakening's
  skinning shader keeps a body's bone matrices from c25, three rows a
  palette slot, the view at c90 to c92 and the projection at c86 to c89.
  Robin's body draws 3102 indices in its first primitive set, one draw an
  eye with the 3D on.
- **Tests:**
  - GPU: a texture starting before a drawn buffer, a GPU TextureCopy, the
    scissor (`the_scissor_keeps_a_draw_inside_its_rectangle`), a picture
    found texel for pixel and its edges carried past it, a marker found by
    its texture and place, a part cut out of its ground
    (`a_part_is_cut_out_of_its_ground`), the bottom screen's place over the top one and
    its pixels at the window's edge.
  - Core: the 16:9 enhancement and `ANY_GAME`, an enhancement's setting,
    mods in folders of their own and their order
    (`mods_in_folders_of_their_own_follow_the_titles`, `mods_stack_in_order`).
  - DLC encrypted inside is refused (`zakuro-fs`), and the library lists
    it as such (`updates_and_dlc_go_with_their_game`).
  - Frontend: the frame rate's text, and the bottom screen's bindings in the
    settings file.
  - zakuro-gpu's Vulkan tests run only with `--features vulkan`. Four of
    them count shaders translated for the GPU, which Zakuro leaves to the
    CPU on an integrated GPU; there, run them with `ZAKURO_GPU_SHADERS=1`.

## Changes to 3dsrecomp

- **`3dsrecomp-0001-port-toml-path.patch`:** on Windows, `port --zakuro
  <dir>` writes the canonical `\\?\C:\...` path into `Cargo.toml`; the patch
  escapes it so Cargo can parse the project, with a unit test.
- **`3dsrecomp-0002-awakening-hints.patch`:** Awakening's section of the
  bundled `hints.txt`: 37 function starts the static analysis misses, which
  Zakuro records when it interprets them. Every build takes them in.

## Building

From Git Bash, in this folder (`env.sh` sets `W` to it):

```bash
source tools/env.sh
cd "$W"
```

First build or update the recompiler:

```bash
(cd 3dsrecomp && cargo build --release)
```

Then generate and compile the C. This takes about 20 minutes:

```bash
./3dsrecomp/target/release/3dsrecomp.exe port "../Fire Emblem - Awakening (Europe) (En,Fr,De,Es,It).cci" FireEmblemAwakeningRecomp --zakuro "$W/zakuro" --name fe-awakening
```

Finally, link Zakuro and the code into the program:

```bash
(cd FireEmblemAwakeningRecomp && cargo build --release)
```

The port's release profile is incremental, so after a change to one of
Zakuro's crates only what it touches is compiled again (about a minute; the
link alone is about 15 seconds). It keeps 16 codegen units, the release
default, so the game runs as fast. `3dsrecomp port` writes a fresh
`Cargo.toml`, so add the profile back after it:

```toml
[profile.release]
incremental = true
codegen-units = 16
```

The link uses GNU ld: MSYS2's `ld.lld` splits the paths in gcc's response
files at their spaces.

`3dsrecomp analyze <rom>` prints the coverage report without building anything.

### Feeding back interpreter hints

Zakuro writes the places it had to interpret next to the library it ran:
`FireEmblemAwakeningRecomp\code\000400000009F100.hints`.

To fold them into the next build:

1. Run `3dsrecomp hints <rom> <that file>`. It prints a `hints.txt`
   section with only the addresses that start new functions.
2. Replace the `= 000400000009F100 …` section in `3dsrecomp\hints.txt`
   with it.
3. Rebuild 3dsrecomp, then run `port` again.

## Test harness

`tools\boot.sh` runs Zakuro's `zakuro-core` `boot` example. Build it once
with `cargo build --release -p zakuro-core --example boot` in `zakuro\`.

The harness loads the recompiled code from
`FireEmblemAwakeningRecomp\code\000400000009F100.dll`. That DLL is the
port's objects linked with
`gcc -static-libgcc -shared -o 000400000009F100.dll entries.o code*.o`.

Usage:

```bash
INPUT="650:A,740:DOWN+A" DUMP_AT=700,900 bash tools/boot.sh 1200 myrun
```

- **Button presses:** each one is held for 6 frames; `500:@160x180`
  touches the bottom screen there.
- **Screens:** written as `_runs\myrun\*.png`.
- **Saves:** go to `_runs\myrun\data` unless `DATA=` names a folder. Mods go
  in that folder's `mods\` and load only on a fresh boot, not from a state.
- **Interpreter only:** `INTERP=1` runs without the recompiled code.
- **Other options:** any `ZAKURO_*` variable passes through, for example
  `ZAKURO_RASTERIZER=hardware`, `ZAKURO_RESOLUTION=3`, `ZAKURO_LANGUAGE=2`
  (French), `ZAKURO_SAVE_AT=frame:C:/path.state`, and
  `ZAKURO_LOAD=C:/path.state`. State paths must be Windows-form
  (`C:/...`), and a state wants the save folder it was made with, as
  `DATA=`.

Dialogs that ask Yes/No (save, StreetPass, avatar confirm) default to No, so
press LEFT before A.
