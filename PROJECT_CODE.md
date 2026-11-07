# What this project adds to Zakuro and 3dsrecomp

This list separates the project's code from Zakuro's and 3dsrecomp's. Paths start at the two folders the README's build steps make: `zakuro/` (Zakuro at `80d78be`) and `3dsrecomp/` (3dsrecomp at `9db0d8b`), both with the project's patches applied. Line numbers point at that patched code. The parts marked as the feet mod come from `mods/feet/zakuro-feet.patch`; everything else comes from `patches/`. `source/` holds the same files.

## Files the project adds

| File | Lines | What it holds |
|---|---|---|
| `zakuro/crates/zakuro-core/src/game_ui.rs` | 144 | Awakening's screen data: the markers that tell its scenes apart, and the rectangles of the battle panels, battle buttons, FE3H banners, title and world-map pieces. |
| `zakuro/crates/zakuro-core/src/scenes.rs` | 146 | Scene detection from the markers the GPU sees: which scene shows, the end of a battle, marker positions, and the unit selection. |
| `zakuro/crates/zakuro/src/banners.rs` | 221 | Reads the text of the FE3H-style banners (names, HP, weapon, stats) off the bottom screen. |
| `zakuro/crates/zakuro/src/console.rs` | 81 | The optional console window with Zakuro's log (Settings, System). |
| `zakuro/crates/zakuro/src/overlays.rs` | 407 | The overlays over the top screen: scene state, the battle buttons, the bottom screen over the top one, the battle panels and the FE3H banners. |
| `zakuro/crates/zakuro/src/sounds.rs` | 301 | Reads the bottom screen's open and close sounds from the game's sound archive. |
| `zakuro/crates/zakuro/src/theme.rs` | 250 | The launcher's look: colours, fonts, window frames, tabs and buttons. |
| `zakuro/crates/zakuro/src/update.rs` | 161 | Update Launcher: finds the latest release on GitHub, downloads it and puts it in place of the running launcher. |
| `zakuro/crates/zakuro-core/src/feet.rs` | 212 | The feet mod's per-frame hook: turns each foot so that it rests flat on the ground. (feet mod) |
| `zakuro/crates/zakuro/src/feet/foot.rs` | 1123 | Builds the feet for one body model and writes the changed model. (feet mod) |
| `zakuro/crates/zakuro/src/feet/formats.rs` | 369 | The game's formats the builder reads and writes: LZ compression, CGFX dictionaries and ETC1 textures. (feet mod) |
| `zakuro/crates/zakuro/src/feet/mod.rs` | 108 | The feet mod's build step: runs the builder over every body model of the game. (feet mod) |
| `zakuro/crates/zakuro/src/feet/model.rs` | 133 | Reads a CGFX model's skeleton, vertices and submeshes. (feet mod) |

## Changes inside Zakuro's files

### `zakuro/crates/zakuro-common/src/hash.rs`

Moved from `zakuro/crates/zakuro-gpu/src/raster/hardware/hasher.rs`, so that zakuro-core can use it too.

- Added: `const MULTIPLIER`, line 8; `struct QuickHasher`, lines 10–13; `impl QuickHasher`, lines 15–19; `fn QuickHasher::add`, lines 16–18; `impl Hasher for QuickHasher`, lines 21–58; `fn <QuickHasher as Hasher>::finish`, lines 54–57; `fn tests::consecutive_addresses_spread_over_the_buckets`, lines 91–97.
- Changed: `lines outside any item`, line 1; `type QuickMap`, line 5; `type QuickSet`, line 6; `lines outside any item`, line 9; `mod tests`, lines 60–98; `fn tests::hash`, lines 65–67.
- Removed: `const K`; `fn <Quick as Hasher>::finish`; `fn <Quick as Hasher>::write_u16`; `fn <Quick as Hasher>::write_u32`; `fn <Quick as Hasher>::write_u64`; `fn <Quick as Hasher>::write_u8`; `fn <Quick as Hasher>::write_usize`; `fn <Quick as Hasher>::write`; `fn Quick::add`; `impl Hasher for Quick`; `impl Quick`; `struct Quick`.

### `zakuro/crates/zakuro-common/src/lib.rs`

- Added: `mod hash`, line 4.

### `zakuro/crates/zakuro-core/examples/boot.rs`

- Added: `fn frame_and_value`, lines 633–637; `fn mapped_memory`, lines 639–649; `fn print_scenes`, lines 651–674.
- Changed: `fn main`, lines 8–626.

### `zakuro/crates/zakuro-core/src/cheats.rs`

- Added: `struct CheatSetting`, lines 35–40; `impl CheatSetting`, lines 42–46; `fn CheatSetting::new`, lines 43–45; `fn Cheat::set_value`, lines 103–111.
- Changed: `struct Cheat`, lines 16–33; `fn Cheat::new`, lines 94–97; `fn run`, lines 236–274.

### `zakuro/crates/zakuro-core/src/enhancements.rs`

- Added: `const DEFAULT_SOURCE`, line 11; `struct Setting`, lines 25–32; `impl Setting`, lines 34–44; `fn Setting::parse`, lines 35–43; `fn code_with_value`, lines 46–50; `fn cheat_for`, lines 101–117; `fn tests::a_code_says_where_it_came_from`, lines 142–184; `fn tests::a_new_value_rebuilds_the_code`, lines 186–194.
- Changed: `lines outside any item`, line 8; `struct Enhancement`, lines 14–23; `fn for_title`, lines 62–95; `fn cheats`, lines 97–99; `fn tests::a_game_gets_the_enhancements_for_its_build`, lines 125–140; `fn tests::every_listed_code_is_code`, lines 196–202.
- Its imports change too.

### `zakuro/crates/zakuro-core/src/lib.rs`

- Added: `mod feet`, line 7 (feet mod); `mod game_ui`, line 8; `mod scenes`, line 20; `const ANY_GAME`, line 38; `impl Config`, lines 134–148; `fn Config::mods_dir`, lines 135–137; `fn Config::enhancements_on`, lines 139–147; `struct Pictures`, lines 183–187; `fn System::set_original`, lines 651–653; `fn System::cursor_on_unit`, lines 833–835; `fn System::talking`, lines 837–839; `fn System::world_map`, lines 841–843; `fn System::forecast`, lines 845–847; `fn System::brief_forecast`, lines 849–851; `fn System::unit_view`, lines 853–855; `fn System::unit_selected`, lines 857–859; `fn System::battle_pieces`, lines 861–863; `fn System::battle_banners`, lines 865–867; `fn System::battle_buttons`, lines 869–871; `fn System::battle_ui`, lines 873–875; `fn System::title_pieces`, lines 877–879; `fn System::world_pieces`, lines 881–883; `fn System::marker_spans`, lines 885–887; `fn System::follows_cursor`, lines 889–891; `fn System::pictures`, lines 893–895; `fn System::set_resolution`, lines 1112–1117; `fn System::bottom_picture`, lines 1212–1218.
- Changed: `struct Config`, lines 41–100; `fn <Config as Default>::default`, lines 103–131; `struct System`, lines 189–261; `fn System::new`, lines 290–341; `fn System::end_frame`, lines 738–777; `fn System::pace`, lines 786–812; `fn System::set_framebuffer`, lines 948–962.
- Its imports change too.

### `zakuro/crates/zakuro-core/src/loader.rs`

- Added: `fn add_game_hooks`, lines 281–285 (feet mod).
- Changed: `fn load`, lines 65–240 (partly the feet mod); `fn swap_recompiled`, lines 248–279 (partly the feet mod); `fn map_code`, lines 367–461; `fn tests::a_game_has_the_enhancements_for_its_build`, lines 573–594.

### `zakuro/crates/zakuro-core/src/memory/mod.rs`

- Changed: `struct Memory`, lines 67–112; `struct WriteLog`, lines 115–120; `fn Memory::new`, lines 214–231; `fn Memory::log_writes`, lines 297–302; `fn Memory::stop_logging_writes`, lines 304–308.
- Its imports change too.

### `zakuro/crates/zakuro-core/src/mods.rs`

- Added: `const MOD_PARTS`, line 31; `fn folders`, lines 33–38; `fn is_mod_folder`, lines 40–45; `fn describe`, lines 173–182; `mod tests`, lines 184–204; `fn tests::mods_in_folders_of_their_own_follow_the_titles`, lines 188–203.
- Changed: `lines outside any item`, line 1; `fn present`, lines 48–53; `lines outside any item`, line 54; `fn exheader`, lines 55–78; `lines outside any item`, line 86; `fn code`, lines 87–118; `lines outside any item`, line 130; `fn lay`, lines 131–171.
- Its imports change too.

### `zakuro/crates/zakuro-core/src/recompiled.rs`

- Added: `type Hook`, line 35 (feet mod); `enum AfterHook`, lines 37–41 (feet mod); `fn hooked`, lines 131–145 (feet mod); `fn return_to_caller`, lines 147–152 (feet mod); `fn Library::hook`, lines 197–201 (feet mod).
- Changed: `struct Library`, lines 24–33 (feet mod); `fn Library::new`, lines 193–195 (feet mod); `fn Library::lookup`, lines 232–245 (feet mod).

### `zakuro/crates/zakuro-fs/src/layered.rs`

- Added: `fn Layered::stacked`, lines 64–82; `fn tests::mods_stack_in_order`, lines 566–586.
- Changed: `lines outside any item`, line 1; `fn Layered::new`, lines 60–62.

### `zakuro/crates/zakuro-fs/src/lib.rs`

- Added: `fn Dlc::check_readable`, lines 199–212; `fn addon_tests::dlc_encrypted_inside_is_refused`, lines 447–458.
- Changed: `fn Dlc::load`, lines 179–197; `fn Title::lay_mods`, lines 327–335.

### `zakuro/crates/zakuro-gpu/src/backend/gl.rs`

- Changed: `fn GlPresenter::set_viewport`, lines 264–268; `fn <GlPresenter as Presenter>::present`, lines 313–355; `fn GlOverlay::draw`, lines 427–527.
- Its imports change too.

### `zakuro/crates/zakuro-gpu/src/backend/mod.rs`

- Added: `const BOTTOM_SCREEN_TEXTURE`, line 50; `impl Viewport`, lines 94–99; `fn Viewport::pixels`, lines 95–98; `struct Piece`, lines 119–127; `struct Rounded`, lines 129–141; `struct Banners`, lines 143–147; `struct BannerSide`, lines 149–157; `struct Ground`, lines 159–163; `fn cut_out`, lines 165–211; `struct PiecePlace`, lines 213–224; `enum Anchor`, lines 226–231; `fn piece_places`, lines 233–266; `fn tests::a_part_is_cut_out_of_its_ground`, lines 510–534; `fn tests::a_piece_is_its_rounded_rectangles`, lines 536–563; `fn tests::the_bottom_screen_goes_over_the_middle_of_the_top_one`, lines 640–655; `fn tests::a_viewport_keeps_its_last_column`, lines 657–660.
- Changed: `enum ScreenLayout`, lines 102–117; `fn ScreenLayout::size_f`, lines 300–308; `fn layout`, lines 323–388; `fn place`, lines 432–465.

### `zakuro/crates/zakuro-gpu/src/backend/vulkan.rs`

- Changed: `fn VulkanPresenter::record`, lines 669–813; `fn set_viewport`, lines 1333–1367.

### `zakuro/crates/zakuro-gpu/src/backend/vulkan_overlay.rs`

- Changed: `impl OverlayPainter`, lines 68–425; `fn OverlayPainter::upload`, lines 153–253; `fn OverlayPainter::draw`, lines 256–320.
- Its imports change too.

### `zakuro/crates/zakuro-gpu/src/interpolation.rs`

- Added: `fn toggles`, lines 39–41; `fn tests::a_round_step_from_standing_still_and_back_toggles`, lines 392–408.
- Changed: `fn Interpolation::draw`, lines 150–235; `fn tests::draws_match_the_frame_before_in_order`, lines 312–334; `fn tests::jumps_are_not_drawn_in_between`, lines 466–480.

### `zakuro/crates/zakuro-gpu/src/lib.rs`

- Added: `fn Gpu::set_markers`, lines 763–765; `fn Gpu::follows_markers`, lines 767–769; `fn Gpu::take_marked`, lines 771–773; `fn Gpu::take_spans`, lines 775–777; `fn Gpu::set_resolution`, lines 839–853; `fn Gpu::native_picture`, lines 869–877.
- Changed: `fn Gpu::texture_copy`, lines 564–626; `fn Gpu::draw`, lines 1176–1238; `lines outside any item`, line 1539; `lines outside any item`, line 1540; `lines outside any item`, line 1541; `lines outside any item`, line 1542; `lines outside any item`, line 1543; `lines outside any item`, line 1544; `lines outside any item`, line 1545; `lines outside any item`, line 1546; `lines outside any item`, line 1547; `lines outside any item`, line 1548; `lines outside any item`, line 1549; `lines outside any item`, line 1550; `lines outside any item`, line 1552; `lines outside any item`, line 1553; `lines outside any item`, line 1554; `lines outside any item`, line 1555.
- Its imports change too.

### `zakuro/crates/zakuro-gpu/src/raster.rs`

- Added: `fn scissor`, lines 1113–1126; `const BLIT_VERTICES`, line 1362; `const MARKER_VERTICES`, line 1364; `struct Marker`, lines 1366–1374; `impl Marker`, lines 1376–1391; `const Marker::UNTEXTURED`, line 1377; `fn Marker::matches_texture`, lines 1379–1385; `fn Marker::targets`, lines 1387–1390; `struct Place`, lines 1393–1399; `impl Place`, lines 1401–1406; `fn Place::contains`, lines 1402–1405; `struct MarkerWatch`, lines 1408–1413; `impl MarkerWatch`, lines 1415–1428; `fn MarkerWatch::may_match`, lines 1416–1418; `fn MarkerWatch::record`, lines 1420–1427; `fn texture0_size`, lines 1430–1433; `fn screen_span`, lines 1435–1442; `fn marker_drawn`, lines 1444–1449; `const SNAP`, line 1451; `fn blit_rect`, lines 1453–1483; `fn extend_edges`, lines 1485–1517; `struct ExtendedPicture`, lines 1519–1523; `struct EdgeCache`, lines 1525–1526; `impl EdgeCache`, lines 1528–1544; `const EdgeCache::CAPACITY`, line 1529; `fn EdgeCache::extended`, lines 1531–1543; `fn cpu_shaded_vertices`, lines 3318–3327; `fn tests::placed_at`, lines 3665–3668; `fn tests::a_picture_shown_texel_for_pixel_is_found`, lines 3670–3688; `fn tests::a_marker_is_found_by_its_texture_and_place`, lines 3690–3712; `fn tests::the_edges_of_a_picture_are_carried_past_it`, lines 3714–3724; `fn tests::the_scissor_keeps_a_draw_inside_its_rectangle`, lines 3974–3995; `fn tests::a_texture_starting_before_a_drawn_buffer_has_its_first_rows_from_memory`, lines 4964–5008; `fn tests::a_texture_copy_of_drawn_rows_of_tiles_is_done_on_the_gpu`, lines 5010–5029; `fn tests::a_texture_copy_into_the_first_rows_of_a_held_buffer_keeps_the_rest`, lines 5031–5057.
- Changed: `struct Resources`, lines 1718–1745; `fn bind_texture`, lines 2186–2259; `fn draw`, lines 3045–3142; `fn rasterize`, lines 3333–3638; `mod tests`, lines 3640–7071.

### `zakuro/crates/zakuro-gpu/src/raster/hardware.rs`

- Added: `fn Hardware::rescale`, lines 1857–1878; `fn Hardware::destroy_copy`, lines 2026–2031; `fn Hardware::first_level`, lines 2033–2043; `fn Hardware::texture_copy`, lines 4125–4162; `fn Hardware::native_picture`, lines 4933–4953.
- Changed: `struct Copied`, lines 790–800; `struct Hardware`, lines 1410–1564; `fn Hardware::reads_twin`, lines 2555–2557; `fn Hardware::texture_source`, lines 4317–4346; `fn Hardware::covers`, lines 4349–4351; `fn Hardware::holds`, lines 4358–4373; `fn Hardware::holds_unchanged`, lines 4382–4402; `fn Hardware::copy_texture`, lines 4408–4521; `fn Hardware::copies`, lines 4524–4529; `fn Hardware::picture`, lines 4899–4931; `fn Hardware::read_picture`, lines 4955–4972; `fn Hardware::submit`, lines 5106–5166; `fn Hardware::retire_oldest`, lines 5255–5288; `fn <Hardware as Drop>::drop`, lines 5545–5614.
- Removed: `mod hasher`.
- Its imports change too.

### `zakuro/crates/zakuro-gpu/src/registers.rs`

- Added: `const REG_SCISSOR_MODE`, line 37; `const REG_SCISSOR_FROM`, line 38; `const REG_SCISSOR_TO`, line 39.

### `zakuro/crates/zakuro-gpu/src/shader/jit.rs`

- Changed: `fn compile`, lines 699–791.

### `zakuro/crates/zakuro/src/audio.rs`

- Added: `struct PlayingSound`, lines 40–45; `impl PlayingSound`, lines 47–58; `fn PlayingSound::next_frame`, lines 48–57; `fn Queue::next_console_sample`, lines 74–99; `fn Audio::play`, lines 290–299; `fn tests::a_frontend_sound_plays_over_the_consoles`, lines 373–382.
- Changed: `struct Queue`, lines 22–38; `impl Queue`, lines 60–117; `fn Queue::next`, lines 61–72; `struct Audio`, lines 226–233; `fn Audio::open`, lines 237–263; `fn tests::queue`, lines 360–371.

### `zakuro/crates/zakuro/src/cli.rs`

- Changed: `struct Options`, lines 7–36; `const USAGE`, lines 38–82; `fn parse`, lines 84–185.
- Its imports change too.

### `zakuro/crates/zakuro/src/gui.rs`

- Added: `const BOTTOM_SCREEN`, line 102; `fn context`, lines 112–119; `fn tests::the_bottom_screen_is_the_presenters_texture`, lines 200–203; `fn tests::paint`, lines 205–207; `fn tests::paint_with`, lines 209–305; `fn tests::snapshot_the_menus`, lines 307–345; `fn tests::snapshot_the_library`, lines 347–372; `fn tests::snapshot_the_battle_panels`, lines 374–449.
- Changed: `fn Gui::new`, lines 17–28.

### `zakuro/crates/zakuro/src/lib.rs`

- Added: `mod banners`, line 6; `mod console`, line 9; `mod feet`, line 10 (feet mod); `mod overlays`, line 16; `mod sounds`, line 21; `mod theme`, line 22; `mod update`, line 23; `fn App::touch_on_pieces`, lines 924–940; `fn App::marker_spans`, lines 942–944; `fn App::library_source`, lines 1078–1084; `fn App::save_and_rescan`, lines 1086–1091; `fn App::shown_layout`, lines 1093–1112; `fn App::bottom_over_layout`, lines 1114–1119; `fn App::current_scene`, lines 1121–1132; `fn App::place_bottom_over`, lines 1134–1160; `fn App::close_bottom_over_with_cancel`, lines 1162–1165; `fn App::toggle_bottom_over`, lines 1167–1178; `fn App::build_feet`, lines 1229–1240; `fn App::poll_update`, lines 1242–1258; `fn App::window_layout`, lines 1908–1911; `fn App::bottom_over_frame`, lines 1913–1921; `fn App::placed_pieces`, lines 1923–1926; `fn App::bottom_screen_cuts`, lines 1928–1947; `fn App::banners_on`, lines 1949–1954; `fn App::read_banners`, lines 1956–1964; `fn App::button_places`, lines 1966–1970; `fn mod_setting`, lines 2074–2077.
- Changed: `fn run_with`, lines 76–199; `struct Running`, lines 357–373; `fn Running::new`, lines 376–390; `fn Running::count_frame`, lines 392–405; `struct App`, lines 408–500; `fn <App as ApplicationHandler>::resumed`, lines 553–592; `fn App::config`, lines 654–682; `fn App::play`, lines 706–790; `fn App::key`, lines 805–872; `fn App::poll_gamepads`, lines 876–895; `fn App::touch`, lines 903–922; `fn App::apply_settings`, lines 989–1016; `fn App::apply_layout`, lines 1045–1076; `fn App::poll_jobs`, lines 1261–1298; `fn App::act`, lines 1300–1446; `fn App::step`, lines 1448–1557; `fn App::emulate`, lines 1560–1620; `fn App::interface`, lines 1809–1906; `fn App::present`, lines 1972–2003.
- Its imports change too.

### `zakuro/crates/zakuro/src/library.rs`

- Added: `enum Source`, lines 37–41; `fn read_folder`, lines 126–142; `fn read_chosen_files`, lines 144–160; `fn chosen_addon`, lines 162–174.
- Changed: `struct Game`, lines 11–27; `struct Library`, lines 44–53; `fn Library::scan`, lines 56–62; `fn scan`, lines 91–124; `fn addon`, lines 178–196; `fn read_or_list_unreadable`, lines 200–205; `fn unplayable`, lines 208–220; `fn read_game`, lines 222–250; `struct Smdh`, lines 270–274; `fn Smdh::parse`, lines 285–296; `fn tests::titles_are_read_in_english_first`, lines 348–363; `fn tests::updates_and_dlc_go_with_their_game`, lines 382–442.
- Removed: `fn Library::new`; `fn addons_beside`; `fn folder_of`; `fn tests::a_bare_file_name_is_in_the_current_folder`.

### `zakuro/crates/zakuro/src/main.rs`

- Changed: `fn main`, lines 1–5.

### `zakuro/crates/zakuro/src/menus.rs`

- Added: `enum Confirm`, lines 108–112; `enum SettingsTab`, lines 114–122; `const SETTINGS_TABS`, lines 124–130; `fn Menus::game_card`, lines 327–438; `fn Menus::keybinds_tab`, lines 806–833; `fn Menus::controller_tab`, lines 835–863; `fn system_tab`, lines 959–1001; `fn graphics_tab`, lines 1003–1081; `fn sound_tab`, lines 1083–1086; `const ICON`, line 1100; `const LONE_CARD`, line 1102; `const CARD_WIDTH`, line 1104; `fn card_height`, lines 1106–1113; `const BOTTOM_OVER`, line 1132; `enum UpdateButton`, lines 1134–1138; `fn frame_rate`, lines 1140–1146; `struct ModSetting`, lines 1148–1154; `fn tests::the_frame_rate_counts_the_pictures_shown`, lines 1214–1218.
- Changed: `enum Action`, lines 18–65; `struct Menus`, lines 67–106; `fn Menus::library`, lines 233–325; `fn Menus::game`, lines 442–582; `fn Menus::cheats`, lines 585–649; `fn Menus::settings`, lines 767–804; `fn progress`, lines 1115–1130; `fn tests::the_settings_fit_a_short_screen`, lines 1197–1212.
- Its imports change too.

### `zakuro/crates/zakuro/src/recompile.rs`

- Added: `type ModBuilder`, line 20; `struct ModStep`, lines 22–26; `const MOD_STEPS`, line 28 (feet mod); `fn mod_steps`, lines 30–32; `fn has_mod_steps`, lines 34–36; `fn Job::build_mods_only`, lines 161–190; `fn build_mods`, lines 209–219; `fn tests::a_whole_build`, lines 464–492.
- Changed: `enum Stage`, lines 38–50; `fn Stage::fraction`, lines 58–67; `struct Job`, lines 86–95; `impl Job`, lines 97–207; `fn Job::start`, lines 98–159.

### `zakuro/crates/zakuro/src/settings.rs`

- Added: `enum BattleUi`, lines 508–515; `impl BattleUi`, lines 517–527; `const BattleUi::ALL`, line 518; `fn BattleUi::name`, lines 520–526; `struct Kept`, lines 529–536; `const ORIGINAL_DISPLAY`, line 538; `impl Kept`, lines 540–558; `fn Kept::of`, lines 541–549; `fn Kept::apply_to`, lines 551–557; `const BOTTOM_OVER_KEY`, line 560; `const BOTTOM_OVER_BUTTON`, line 561; `fn Settings::set_original`, lines 564–575; `fn all_enhancements_on`, lines 578–582; `fn tests::a_new_player_starts_with_every_enhancement_on_and_the_controller_by_its_labels`, lines 742–748; `fn tests::the_bottom_screen_over_the_top_one_keeps_its_binding`, lines 750–757.
- Changed: `fn TopShape::name`, lines 116–122; `lines outside any item`, line 254; `fn <PadButtons as Default>::default`, lines 370–387; `struct Settings`, lines 432–506; `impl Settings`, lines 563–576; `fn <Settings as Default>::default`, lines 585–628; `fn Settings::load`, lines 660–676; `fn tests::missing_fields_take_their_defaults`, lines 731–740.
- Removed: `const CameraSource::ALL`; `fn CameraSource::name`; `impl CameraSource`.

### `zakuro/crates/zakuro/src/zig.rs`

- Added: `fn windowless`, lines 236–244.
- Changed: `enum Fetcher`, lines 72–76; `fn Fetcher::find`, lines 79–84; `fn Fetcher::command`, lines 95–116; `fn fetch`, lines 187–216; `fn unpack`, lines 249–275.

## Changes inside 3dsrecomp

### `3dsrecomp/src/build.rs`

From `patches/3dsrecomp-0003-build-fixes.patch`.

- Changed: `fn build`, lines 129–152.

### `3dsrecomp/src/codegen/mod.rs`

From `patches/3dsrecomp-0003-build-fixes.patch`.

- Changed: `fn tests::compiles`, lines 584–595.

### `3dsrecomp/src/compile.rs`

From `patches/3dsrecomp-0003-build-fixes.patch`.

- Added: `type Cancelled`, line 34; `const CREATE_NO_WINDOW`, lines 117–118; `fn windowless`, lines 120–124; `fn windowless`, lines 126–127; `fn run_unless_cancelled`, lines 294–311.
- Changed: `fn Compiler::command`, lines 77–82; `fn lower`, lines 110–115; `lines outside any item`, line 188; `fn compile`, lines 189–199; `fn objects`, lines 206–292; `fn archive`, lines 350–365; `fn tests::objects_link_however_long_their_paths_are`, lines 386–402.

## Data and build files

- `zakuro/Cargo.toml`: 3dsrecomp's ABI crate taken from the 3dsrecomp folder beside Zakuro.
- `zakuro/crates/zakuro-core/src/enhancements.txt`: The section for Fire Emblem: Awakening (EUR): 16:9, Smooth map sprites and Faster battles, with the Speed setting.
- `zakuro/crates/zakuro/Cargo.toml`: 3dsrecomp taken from the folder beside Zakuro; serde_json for Update Launcher.
- `3dsrecomp/hints.txt`: The section for Fire Emblem: Awakening: where 3dsrecomp starts functions.
