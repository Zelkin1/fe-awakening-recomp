# What this project adds to Zakuro and 3dsrecomp

This list separates the project's code from Zakuro's and 3dsrecomp's. Paths start at the two folders the README's build steps make: `zakuro/` (Zakuro at `80d78be`) and `3dsrecomp/` (3dsrecomp at `9db0d8b`), both with the project's patches applied. The parts marked as the feet mod come from `mods/feet/zakuro-feet.patch`; everything else comes from `patches/`. `source/` holds the same files.

## Files the project adds

| File | Lines | What it holds |
|---|---|---|
| `zakuro/crates/zakuro-core/src/game_ui.rs` | 162 | Awakening's screen data: the markers that tell its scenes apart, and the rectangles of the battle panels, battle buttons, FE3H banners, title and world-map pieces. |
| `zakuro/crates/zakuro-core/src/scenes.rs` | 146 | Scene detection from the markers the GPU sees: which scene shows, the end of a battle, marker positions, and the unit selection. |
| `zakuro/crates/zakuro/src/banners.rs` | 221 | Reads the text of the FE3H-style banners (names, HP, weapon, stats) off the bottom screen. |
| `zakuro/crates/zakuro/src/console.rs` | 81 | The optional console window with Zakuro's log (Settings, System). |
| `zakuro/crates/zakuro/src/overlays.rs` | 603 | The overlays over the top screen: scene state, the battle buttons, the bottom screen over the top one, the battle panels and the FE3H banners. |
| `zakuro/crates/zakuro/src/sounds.rs` | 301 | Reads the bottom screen's open and close sounds from the game's sound archive. |
| `zakuro/crates/zakuro/src/theme.rs` | 376 | The launcher's look: colours, fonts, window frames, tabs and buttons. |
| `zakuro/crates/zakuro/src/update.rs` | 161 | Update Launcher: finds the latest release on GitHub, downloads it and puts it in place of the running launcher. |
| `zakuro/crates/zakuro-core/src/feet.rs` | 212 | The feet mod's per-frame hook: turns each foot so that it rests flat on the ground. (feet mod) |
| `zakuro/crates/zakuro/src/feet/foot.rs` | 1123 | Builds the feet for one body model and writes the changed model. (feet mod) |
| `zakuro/crates/zakuro/src/feet/formats.rs` | 369 | The game's formats the builder reads and writes: LZ compression, CGFX dictionaries and ETC1 textures. (feet mod) |
| `zakuro/crates/zakuro/src/feet/mod.rs` | 108 | The feet mod's build step: runs the builder over every body model of the game. (feet mod) |
| `zakuro/crates/zakuro/src/feet/model.rs` | 133 | Reads a CGFX model's skeleton, vertices and submeshes. (feet mod) |

## Changes inside Zakuro's files

### `zakuro/crates/zakuro-common/src/hash.rs`

Moved from `zakuro/crates/zakuro-gpu/src/raster/hardware/hasher.rs`, so that zakuro-core can use it too.

Added:

- `const MULTIPLIER`
- `struct QuickHasher`
- `impl QuickHasher`
- `fn QuickHasher::add`
- `impl Hasher for QuickHasher`
- `fn <QuickHasher as Hasher>::finish`
- `fn tests::consecutive_addresses_spread_over_the_buckets`

Changed:

- `code outside any item`
- `type QuickMap`
- `type QuickSet`
- `mod tests`
- `fn tests::hash`

Removed:

- `const K`
- `fn <Quick as Hasher>::finish`
- `fn <Quick as Hasher>::write_u16`
- `fn <Quick as Hasher>::write_u32`
- `fn <Quick as Hasher>::write_u64`
- `fn <Quick as Hasher>::write_u8`
- `fn <Quick as Hasher>::write_usize`
- `fn <Quick as Hasher>::write`
- `fn Quick::add`
- `impl Hasher for Quick`
- `impl Quick`
- `struct Quick`

### `zakuro/crates/zakuro-common/src/lib.rs`

Added:

- `mod hash`

### `zakuro/crates/zakuro-core/examples/boot.rs`

Added:

- `fn frame_and_value`
- `fn mapped_memory`
- `fn print_scenes`

Changed:

- `fn main`

### `zakuro/crates/zakuro-core/src/cheats.rs`

Added:

- `struct CheatSetting`
- `impl CheatSetting`
- `fn CheatSetting::new`
- `fn Cheat::set_value`

Changed:

- `struct Cheat`
- `fn Cheat::new`
- `fn run`

### `zakuro/crates/zakuro-core/src/enhancements.rs`

Added:

- `const DEFAULT_SOURCE`
- `struct Setting`
- `impl Setting`
- `fn Setting::parse`
- `fn code_with_value`
- `fn cheat_for`
- `fn tests::a_code_says_where_it_came_from`
- `fn tests::a_new_value_rebuilds_the_code`

Changed:

- `code outside any item`
- `struct Enhancement`
- `fn for_title`
- `fn cheats`
- `fn tests::a_game_gets_the_enhancements_for_its_build`
- `fn tests::every_listed_code_is_code`

Its imports change too.

### `zakuro/crates/zakuro-core/src/lib.rs`

Added:

- `mod feet` (feet mod)
- `mod game_ui`
- `mod scenes`
- `const ANY_GAME`
- `impl Config`
- `fn Config::mods_dir`
- `fn Config::enhancements_on`
- `struct Pictures`
- `fn System::set_original`
- `fn System::cursor_on_unit`
- `fn System::talking`
- `fn System::world_map`
- `fn System::forecast`
- `fn System::brief_forecast`
- `fn System::unit_view`
- `fn System::unit_selected`
- `fn System::battle_pieces`
- `fn System::battle_banners`
- `fn System::battle_buttons`
- `fn System::battle_ui`
- `fn System::title_pieces`
- `fn System::world_pieces`
- `fn System::marker_spans`
- `fn System::follows_cursor`
- `fn System::pictures`
- `fn System::set_resolution`
- `fn System::bottom_picture`

Changed:

- `struct Config`
- `fn <Config as Default>::default`
- `struct Pacing`
- `struct System`
- `fn System::new`
- `fn System::end_frame`
- `fn System::pace`
- `fn System::set_framebuffer`

Its imports change too.

### `zakuro/crates/zakuro-core/src/loader.rs`

Added:

- `fn add_game_hooks` (feet mod)

Changed:

- `fn load` (partly the feet mod)
- `fn swap_recompiled` (partly the feet mod)
- `fn map_code`
- `fn tests::a_game_has_the_enhancements_for_its_build`

### `zakuro/crates/zakuro-core/src/memory/mod.rs`

Changed:

- `struct Memory`
- `struct WriteLog`
- `fn Memory::new`
- `fn Memory::log_writes`
- `fn Memory::stop_logging_writes`

Its imports change too.

### `zakuro/crates/zakuro-core/src/mods.rs`

Added:

- `const MOD_PARTS`
- `fn folders`
- `fn is_mod_folder`
- `fn describe`
- `mod tests`
- `fn tests::mods_in_folders_of_their_own_follow_the_titles`

Changed:

- `code outside any item`
- `fn present`
- `fn exheader`
- `fn code`
- `fn lay`

Its imports change too.

### `zakuro/crates/zakuro-core/src/recompiled.rs`

Added:

- `type Hook` (feet mod)
- `enum AfterHook` (feet mod)
- `fn hooked` (feet mod)
- `fn return_to_caller` (feet mod)
- `fn Library::hook` (feet mod)

Changed:

- `struct Library` (feet mod)
- `fn Library::new` (feet mod)
- `fn Library::lookup` (feet mod)

### `zakuro/crates/zakuro-fs/src/layered.rs`

Added:

- `fn Layered::stacked`
- `fn tests::mods_stack_in_order`

Changed:

- `code outside any item`
- `fn Layered::new`

### `zakuro/crates/zakuro-fs/src/lib.rs`

Added:

- `fn Dlc::check_readable`
- `fn addon_tests::dlc_encrypted_inside_is_refused`

Changed:

- `fn Dlc::load`
- `fn Title::lay_mods`

### `zakuro/crates/zakuro-gpu/src/backend/gl.rs`

Changed:

- `fn GlPresenter::set_viewport`
- `fn <GlPresenter as Presenter>::present`
- `fn GlOverlay::draw`

Its imports change too.

### `zakuro/crates/zakuro-gpu/src/backend/mod.rs`

Added:

- `const BOTTOM_SCREEN_TEXTURE`
- `impl Viewport`
- `fn Viewport::pixels`
- `struct Piece`
- `struct Rounded`
- `struct Banners`
- `struct BannerSide`
- `struct Ground`
- `fn cut_out`
- `struct PiecePlace`
- `enum Anchor`
- `fn piece_places`
- `fn tests::a_part_is_cut_out_of_its_ground`
- `fn tests::a_piece_is_its_rounded_rectangles`
- `fn tests::the_bottom_screen_goes_over_the_middle_of_the_top_one`
- `fn tests::a_viewport_keeps_its_last_column`

Changed:

- `enum ScreenLayout`
- `fn ScreenLayout::size_f`
- `fn layout`
- `fn place`

### `zakuro/crates/zakuro-gpu/src/backend/vulkan.rs`

Changed:

- `fn VulkanPresenter::record`
- `fn set_viewport`

### `zakuro/crates/zakuro-gpu/src/backend/vulkan_overlay.rs`

Changed:

- `impl OverlayPainter`
- `fn OverlayPainter::upload`
- `fn OverlayPainter::draw`

Its imports change too.

### `zakuro/crates/zakuro-gpu/src/interpolation.rs`

Added:

- `const TURN`
- `fn toggles`
- `fn tests::a_round_step_from_standing_still_and_back_toggles`

Changed:

- `const JUMP`
- `fn Interpolation::draw`
- `mod tests`
- `fn tests::draws_match_the_frame_before_in_order`
- `fn tests::going_on_as_before_moves_however_fast`
- `fn tests::draws_that_keep_jumping_count_apart`
- `fn tests::jumps_are_not_drawn_in_between`

Removed:

- `const JUMP_PART`

### `zakuro/crates/zakuro-gpu/src/lib.rs`

Added:

- `fn Gpu::set_markers`
- `fn Gpu::follows_markers`
- `fn Gpu::take_marked`
- `fn Gpu::take_spans`
- `fn Gpu::set_resolution`
- `fn Gpu::native_picture`

Changed:

- `fn Gpu::texture_copy`
- `fn Gpu::draw`
- `code outside any item`

Its imports change too.

### `zakuro/crates/zakuro-gpu/src/raster.rs`

Added:

- `fn scissor`
- `const BLIT_VERTICES`
- `const MARKER_VERTICES`
- `struct Marker`
- `impl Marker`
- `const Marker::UNTEXTURED`
- `fn Marker::matches_texture`
- `fn Marker::targets`
- `struct Place`
- `impl Place`
- `fn Place::contains`
- `struct MarkerWatch`
- `impl MarkerWatch`
- `fn MarkerWatch::may_match`
- `fn MarkerWatch::record`
- `fn texture0_size`
- `fn screen_span`
- `fn marker_drawn`
- `const SNAP`
- `fn blit_rect`
- `fn extend_edges`
- `struct ExtendedPicture`
- `struct EdgeCache`
- `impl EdgeCache`
- `const EdgeCache::CAPACITY`
- `fn EdgeCache::extended`
- `fn cpu_shaded_vertices`
- `fn tests::placed_at`
- `fn tests::a_picture_shown_texel_for_pixel_is_found`
- `fn tests::a_marker_is_found_by_its_texture_and_place`
- `fn tests::the_edges_of_a_picture_are_carried_past_it`
- `fn tests::the_scissor_keeps_a_draw_inside_its_rectangle`
- `fn tests::a_texture_starting_before_a_drawn_buffer_has_its_first_rows_from_memory`
- `fn tests::a_texture_copy_of_drawn_rows_of_tiles_is_done_on_the_gpu`
- `fn tests::a_texture_copy_into_the_first_rows_of_a_held_buffer_keeps_the_rest`

Changed:

- `struct Resources`
- `fn TextureCache::decoded`
- `fn bind_texture`
- `fn draw`
- `fn rasterize`
- `mod tests`

### `zakuro/crates/zakuro-gpu/src/raster/hardware.rs`

Added:

- `struct Slot`
- `const POOL_BLOCK`
- `const POOL_LARGEST`
- `struct Pool`
- `struct PoolBlock`
- `impl Pool`
- `fn Pool::take`
- `fn Pool::give_back`
- `fn Hardware::rescale`
- `fn Hardware::from_pool`
- `fn Hardware::release`
- `fn Hardware::destroy_copy`
- `fn Hardware::first_level`
- `fn Hardware::level_view`
- `fn Hardware::blit_with`
- `fn Hardware::texture_copy`
- `fn Hardware::texture_copy_columns`
- `fn Hardware::transfer`
- `fn Hardware::native_transfer`
- `fn Hardware::level_source`
- `fn Hardware::copy_level`
- `fn Hardware::native_picture`

Changed:

- `struct Image`
- `struct Copied`
- `struct Hardware`
- `fn Hardware::with_device`
- `fn Hardware::image_levels`
- `fn Hardware::destroy_image`
- `fn Hardware::surface_rows`
- `fn Hardware::reads_twin`
- `fn Hardware::blit`
- `fn Hardware::pipeline_for`
- `fn Hardware::display_transfer`
- `fn Hardware::texture_source`
- `fn Hardware::covers`
- `fn Hardware::holds`
- `fn Hardware::holds_unchanged`
- `fn Hardware::copy_texture`
- `fn Hardware::copies`
- `fn Hardware::picture`
- `fn Hardware::read_picture`
- `fn Hardware::submit`
- `fn Hardware::retire_oldest`
- `fn <Hardware as Drop>::drop`
- `fn tests::a_pixel_standing_in_for_the_shadow_keeps_what_the_bytes_would`

Removed:

- `mod hasher`

Its imports change too.

### `zakuro/crates/zakuro-gpu/src/registers.rs`

Added:

- `const REG_SCISSOR_MODE`
- `const REG_SCISSOR_FROM`
- `const REG_SCISSOR_TO`

### `zakuro/crates/zakuro-gpu/src/shader/jit.rs`

Changed:

- `fn compile`

### `zakuro/crates/zakuro-gpu/src/texture.rs`

Added:

- `const TILE_ORDER`
- `const PARALLEL_TEXELS`
- `fn decode_level`
- `fn decode_levels`
- `fn decoders`
- `fn decode_tile`
- `fn etc1_block`
- `fn etc1_palettes`

Changed:

- `code outside any item`
- `fn decode_etc1_block`

Its imports change too.

### `zakuro/crates/zakuro/src/audio.rs`

Added:

- `struct PlayingSound`
- `impl PlayingSound`
- `fn PlayingSound::next_frame`
- `fn Queue::next_console_sample`
- `fn Audio::play`
- `fn tests::a_frontend_sound_plays_over_the_consoles`

Changed:

- `struct Queue`
- `impl Queue`
- `fn Queue::next`
- `struct Audio`
- `fn Audio::open`
- `fn tests::queue`

### `zakuro/crates/zakuro/src/cli.rs`

Changed:

- `struct Options`
- `const USAGE`
- `fn parse`

Its imports change too.

### `zakuro/crates/zakuro/src/gamepad.rs`

Added:

- `fn Gamepads::lean`

Changed:

- `const DEADZONE`

### `zakuro/crates/zakuro/src/gui.rs`

Added:

- `const BOTTOM_SCREEN`
- `fn context`
- `fn tests::the_bottom_screen_is_the_presenters_texture`
- `fn tests::paint`
- `fn tests::paint_with`
- `fn tests::snapshot_the_menus`
- `fn tests::snapshot_the_library`
- `fn tests::snapshot_the_battle_panels`

Changed:

- `fn Gui::new`

### `zakuro/crates/zakuro/src/lib.rs`

Added:

- `mod banners`
- `mod console`
- `mod feet` (feet mod)
- `mod overlays`
- `mod sounds`
- `mod theme`
- `mod update`
- `const CURSOR_HIDES_AFTER`
- `fn App::touch_on_pieces`
- `fn App::marker_spans`
- `fn App::library_source`
- `fn App::save_and_rescan`
- `fn App::shown_layout`
- `fn App::bottom_over_layout`
- `fn App::current_scene`
- `fn App::showing_level_up`
- `fn App::bottom_map_screen`
- `fn App::place_bottom_over`
- `fn App::close_bottom_over_with_cancel`
- `fn App::map_overview`
- `fn App::toggle_bottom_over`
- `fn App::build_feet`
- `fn App::poll_update`
- `fn App::window_layout`
- `fn App::bottom_over_frame`
- `fn App::placed_pieces`
- `fn App::bottom_screen_cuts`
- `fn App::banners_on`
- `fn App::read_banners`
- `fn App::button_places`
- `fn mod_setting`

Changed:

- `fn run_with`
- `struct Running`
- `fn Running::new`
- `fn Running::count_frame`
- `struct App`
- `fn <App as ApplicationHandler>::resumed`
- `fn <App as ApplicationHandler>::window_event`
- `fn App::config`
- `fn App::play`
- `fn App::key`
- `fn App::poll_gamepads`
- `fn App::touch`
- `fn App::apply_settings`
- `fn App::apply_layout`
- `fn App::poll_jobs`
- `fn App::act`
- `fn App::step`
- `fn App::emulate`
- `fn App::interface`
- `fn App::present`

Its imports change too.

### `zakuro/crates/zakuro/src/library.rs`

Added:

- `enum Source`
- `fn read_folder`
- `fn read_chosen_files`
- `fn chosen_addon`

Changed:

- `struct Game`
- `struct Library`
- `fn Library::scan`
- `fn scan`
- `fn addon`
- `fn read_or_list_unreadable`
- `fn unplayable`
- `fn read_game`
- `struct Smdh`
- `fn Smdh::parse`
- `fn tests::titles_are_read_in_english_first`
- `fn tests::updates_and_dlc_go_with_their_game`

Removed:

- `fn Library::new`
- `fn addons_beside`
- `fn folder_of`
- `fn tests::a_bare_file_name_is_in_the_current_folder`

### `zakuro/crates/zakuro/src/main.rs`

Changed:

- `fn main`

### `zakuro/crates/zakuro/src/menus.rs`

Added:

- `const STATE_SLOTS`
- `enum Confirm`
- `enum SettingsTab`
- `const SETTINGS_TABS`
- `fn Menus::game_card`
- `fn Menus::keybinds_tab`
- `fn Menus::controller_tab`
- `fn system_tab`
- `fn graphics_tab`
- `fn sound_tab`
- `const ICON`
- `const CARD_WIDTH`
- `fn card_height`
- `const BOTTOM_OVER`
- `enum UpdateButton`
- `fn frame_rate`
- `struct ModSetting`
- `fn tests::the_frame_rate_counts_the_pictures_shown`

Changed:

- `enum Action`
- `struct Menus`
- `impl Menus`
- `fn Menus::library`
- `fn Menus::game`
- `fn Menus::cheats`
- `fn Menus::settings`
- `fn Menus::message`
- `fn progress`
- `fn tests::the_settings_fit_a_short_screen`

Its imports change too.

### `zakuro/crates/zakuro/src/recompile.rs`

Added:

- `type ModBuilder`
- `struct ModStep`
- `const MOD_STEPS` (feet mod)
- `fn mod_steps`
- `fn has_mod_steps`
- `fn Job::build_mods_only`
- `fn build_mods`
- `fn tests::a_whole_build`

Changed:

- `enum Stage`
- `fn Stage::fraction`
- `struct Job`
- `impl Job`
- `fn Job::start`

### `zakuro/crates/zakuro/src/settings.rs`

Added:

- `enum BattleUi`
- `impl BattleUi`
- `const BattleUi::ALL`
- `fn BattleUi::name`
- `struct Kept`
- `const ORIGINAL_DISPLAY`
- `impl Kept`
- `fn Kept::of`
- `fn Kept::apply_to`
- `const BOTTOM_OVER_KEY`
- `const BOTTOM_OVER_BUTTON`
- `fn Settings::set_original`
- `fn all_enhancements_on`
- `fn tests::a_new_player_starts_with_every_enhancement_on_and_the_controller_by_its_labels`
- `fn tests::the_bottom_screen_over_the_top_one_keeps_its_binding`

Changed:

- `fn TopShape::name`
- `code outside any item`
- `fn <PadButtons as Default>::default`
- `struct Settings`
- `impl Settings`
- `fn <Settings as Default>::default`
- `fn Settings::load`
- `fn tests::missing_fields_take_their_defaults`

Removed:

- `const CameraSource::ALL`
- `fn CameraSource::name`
- `impl CameraSource`

### `zakuro/crates/zakuro/src/zig.rs`

Added:

- `fn windowless`

Changed:

- `enum Fetcher`
- `fn Fetcher::find`
- `fn Fetcher::command`
- `fn fetch`
- `fn unpack`

## Changes inside 3dsrecomp

### `3dsrecomp/src/build.rs`

From `patches/3dsrecomp-0003-build-fixes.patch`.

Changed:

- `fn build`

### `3dsrecomp/src/codegen/mod.rs`

From `patches/3dsrecomp-0003-build-fixes.patch`.

Changed:

- `fn tests::compiles`

### `3dsrecomp/src/compile.rs`

From `patches/3dsrecomp-0003-build-fixes.patch`.

Added:

- `type Cancelled`
- `const CREATE_NO_WINDOW`
- `fn windowless`
- `fn run_unless_cancelled`

Changed:

- `fn Compiler::command`
- `fn lower`
- `code outside any item`
- `fn compile`
- `fn objects`
- `fn archive`
- `fn tests::objects_link_however_long_their_paths_are`

## Data and build files

- `zakuro/Cargo.toml`: 3dsrecomp's ABI crate taken from the 3dsrecomp folder beside Zakuro.
- `zakuro/crates/zakuro-core/src/enhancements.txt`: The section for Fire Emblem: Awakening (EUR): 16:9, Smooth map sprites and Faster battles, with the Speed setting.
- `zakuro/crates/zakuro/Cargo.toml`: 3dsrecomp taken from the folder beside Zakuro; serde_json for Update Launcher.
- `3dsrecomp/hints.txt`: The section for Fire Emblem: Awakening: where 3dsrecomp starts functions.
